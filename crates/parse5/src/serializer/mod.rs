use crate::common::entities::{push_escaped_attribute, push_escaped_text};
use crate::common::html::{
    has_unescaped_text, is_void_element, NS_HTML, NS_XLINK, NS_XML, NS_XMLNS,
};
use crate::common::token::Attribute;
use crate::tree_adapters::default::{get_template_content, Node, NodeData, NodeRef};

#[derive(Debug, Clone, Copy)]
pub struct SerializerOptions {
    pub scripting_enabled: bool,
}

impl Default for SerializerOptions {
    fn default() -> Self {
        Self {
            scripting_enabled: true,
        }
    }
}

/// Serializes the children of `node` (the equivalent of `innerHTML`).
pub fn serialize(node: &NodeRef, options: SerializerOptions) -> String {
    let mut serializer = Serializer::new(options);

    if !is_html_void_element(&node.borrow()) {
        serializer.push_child_nodes(node);
        serializer.run();
    }

    serializer.html
}

/// Serializes `node` itself, including its children (the equivalent of
/// `outerHTML`).
pub fn serialize_outer(node: &NodeRef, options: SerializerOptions) -> String {
    let mut serializer = Serializer::new(options);

    serializer.stack.push(Work::Node(node.clone()));
    serializer.run();
    serializer.html
}

enum Work {
    Node(NodeRef),
    EndTag(NodeRef),
}

/// Iterative serializer: an explicit work stack keeps deeply nested trees from
/// overflowing the call stack, which is much smaller on wasm.
struct Serializer {
    html: String,
    options: SerializerOptions,
    stack: Vec<Work>,
}

impl Serializer {
    fn new(options: SerializerOptions) -> Self {
        Self {
            html: String::new(),
            options,
            stack: Vec::new(),
        }
    }

    fn run(&mut self) {
        while let Some(work) = self.stack.pop() {
            match work {
                Work::Node(node) => self.serialize_node(&node),
                Work::EndTag(node) => {
                    if let Some(tag_name) = node.borrow().tag_name() {
                        self.html.push_str("</");
                        self.html.push_str(tag_name);
                        self.html.push('>');
                    }
                }
            }
        }
    }

    /// Schedules the children of `node` (or of its template content) so that
    /// they are serialized next, in document order.
    fn push_child_nodes(&mut self, node: &NodeRef) {
        let template_content = if is_html_template(&node.borrow()) {
            get_template_content(node)
        } else {
            None
        };
        let container = template_content.as_ref().unwrap_or(node).borrow();

        if let Some(child_nodes) = container.child_nodes() {
            self.stack
                .extend(child_nodes.iter().rev().cloned().map(Work::Node));
        }
    }

    fn serialize_node(&mut self, node: &NodeRef) {
        let node_ref = node.borrow();

        match &node_ref.data {
            NodeData::Document { .. } | NodeData::DocumentFragment { .. } => {
                drop(node_ref);
                self.push_child_nodes(node);
            }
            NodeData::Element {
                tag_name,
                attrs,
                namespace_uri,
                ..
            } => {
                self.html.push('<');
                self.html.push_str(tag_name);
                self.serialize_attrs(attrs);
                self.html.push('>');

                if namespace_uri == NS_HTML && is_void_element(tag_name) {
                    return;
                }

                drop(node_ref);
                self.stack.push(Work::EndTag(node.clone()));
                self.push_child_nodes(node);
            }
            NodeData::Comment { data } => {
                self.html.push_str("<!--");
                self.html.push_str(data);
                self.html.push_str("-->");
            }
            NodeData::Text { value } => {
                if has_unescaped_text_parent(&node_ref, self.options.scripting_enabled) {
                    self.html.push_str(value);
                } else {
                    push_escaped_text(&mut self.html, value);
                }
            }
            NodeData::DocumentType { name, .. } => {
                self.html.push_str("<!DOCTYPE ");
                self.html.push_str(name);
                self.html.push('>');
            }
        }
    }

    fn serialize_attrs(&mut self, attrs: &[Attribute]) {
        let html = &mut self.html;

        for attr in attrs {
            html.push(' ');

            match attr.namespace.as_deref() {
                Some(NS_XML) => {
                    html.push_str("xml:");
                    html.push_str(&attr.name);
                }
                Some(NS_XMLNS) => {
                    if attr.name != "xmlns" {
                        html.push_str("xmlns:");
                    }
                    html.push_str(&attr.name);
                }
                Some(NS_XLINK) => {
                    html.push_str("xlink:");
                    html.push_str(&attr.name);
                }
                Some(_) => {
                    if let Some(prefix) = &attr.prefix {
                        html.push_str(prefix);
                        html.push(':');
                    }
                    html.push_str(&attr.name);
                }
                None => html.push_str(&attr.name),
            }

            html.push_str("=\"");
            push_escaped_attribute(html, &attr.value);
            html.push('"');
        }
    }
}

fn is_html_void_element(node: &Node) -> bool {
    node.namespace_uri() == Some(NS_HTML) && node.tag_name().is_some_and(is_void_element)
}

fn is_html_template(node: &Node) -> bool {
    node.namespace_uri() == Some(NS_HTML) && node.tag_name() == Some("template")
}

fn has_unescaped_text_parent(node: &Node, scripting_enabled: bool) -> bool {
    let Some(parent) = node.parent() else {
        return false;
    };
    let parent = parent.borrow();

    parent.namespace_uri() == Some(NS_HTML)
        && parent
            .tag_name()
            .is_some_and(|tag_name| has_unescaped_text(tag_name, scripting_enabled))
}

#[cfg(test)]
mod tests {
    use super::{serialize, serialize_outer, SerializerOptions};
    use crate::common::html::{NS_HTML, NS_SVG, NS_XLINK, NS_XML, NS_XMLNS};
    use crate::common::token::Attribute;
    use crate::tree_adapters::default::{
        append_child, create_document_fragment, create_element, create_text_node,
        set_template_content, Node,
    };

    #[test]
    fn serializes_basic_fragment() {
        let fragment = create_document_fragment();
        let div = create_element("div", NS_HTML, vec![Attribute::new("title", "\"<&")]);
        append_child(&div, create_text_node("a&b"));
        append_child(&fragment, div);

        assert_eq!(
            serialize(&fragment, SerializerOptions::default()),
            "<div title=\"&quot;<&amp;\">a&amp;b</div>"
        );
    }

    #[test]
    fn skips_void_element_children_in_html_namespace() {
        let fragment = create_document_fragment();
        let input = create_element("input", NS_HTML, Vec::new());
        append_child(&input, create_text_node("ignored"));
        append_child(&fragment, input);

        assert_eq!(
            serialize(&fragment, SerializerOptions::default()),
            "<input>"
        );
    }

    #[test]
    fn serialize_returns_empty_string_for_void_element_node() {
        let input = create_element("input", NS_HTML, Vec::new());
        append_child(&input, create_text_node("ignored"));

        assert_eq!(serialize(&input, SerializerOptions::default()), "");
        assert_eq!(
            serialize_outer(&input, SerializerOptions::default()),
            "<input>"
        );
    }

    #[test]
    fn text_escaping_for_unescaped_text_elements_is_namespace_sensitive() {
        let fragment = create_document_fragment();
        let html_style = create_element("style", NS_HTML, Vec::new());
        let svg_style = create_element("style", NS_SVG, Vec::new());
        append_child(&html_style, create_text_node("x < y && z"));
        append_child(&svg_style, create_text_node("x < y && z"));
        append_child(&fragment, html_style);
        append_child(&fragment, svg_style);

        assert_eq!(
            serialize(&fragment, SerializerOptions::default()),
            "<style>x < y && z</style><style>x &lt; y &amp;&amp; z</style>"
        );
    }

    #[test]
    fn serializes_namespaced_attributes_like_parse5() {
        let fragment = create_document_fragment();
        let node = create_element(
            "svg",
            NS_SVG,
            vec![
                Attribute {
                    name: "lang".to_string(),
                    namespace: Some(NS_XML.to_string()),
                    prefix: None,
                    value: "en".to_string(),
                },
                Attribute {
                    name: "xmlns".to_string(),
                    namespace: Some(NS_XMLNS.to_string()),
                    prefix: None,
                    value: "http://www.w3.org/2000/svg".to_string(),
                },
                Attribute {
                    name: "xlink".to_string(),
                    namespace: Some(NS_XMLNS.to_string()),
                    prefix: Some("xmlns".to_string()),
                    value: NS_XLINK.to_string(),
                },
                Attribute {
                    name: "href".to_string(),
                    namespace: Some(NS_XLINK.to_string()),
                    prefix: Some("custom".to_string()),
                    value: "#icon".to_string(),
                },
                Attribute {
                    name: "name".to_string(),
                    namespace: Some("urn:custom".to_string()),
                    prefix: Some("p".to_string()),
                    value: "\"<&".to_string(),
                },
            ],
        );
        append_child(&fragment, node);

        assert_eq!(
            serialize(&fragment, SerializerOptions::default()),
            "<svg xml:lang=\"en\" xmlns=\"http://www.w3.org/2000/svg\" xmlns:xlink=\"http://www.w3.org/1999/xlink\" xlink:href=\"#icon\" p:name=\"&quot;<&amp;\"></svg>"
        );
    }

    #[test]
    fn serializes_document_type_name_only() {
        let fragment = create_document_fragment();
        append_child(
            &fragment,
            Node::document_type(
                "html",
                "-//W3C//DTD XHTML 1.0 Transitional//EN",
                "http://www.w3.org/TR/xhtml1/DTD/xhtml1-transitional.dtd",
            ),
        );

        assert_eq!(
            serialize(&fragment, SerializerOptions::default()),
            "<!DOCTYPE html>"
        );
    }

    #[test]
    fn html_template_serializes_template_content() {
        let fragment = create_document_fragment();
        let template = create_element("template", NS_HTML, Vec::new());
        let ignored_child = create_element("span", NS_HTML, Vec::new());
        append_child(&ignored_child, create_text_node("ignored"));
        append_child(&template, ignored_child);

        let content = create_document_fragment();
        let div = create_element("div", NS_HTML, Vec::new());
        append_child(&div, create_text_node("content"));
        append_child(&content, div);
        set_template_content(&template, content);
        append_child(&fragment, template);

        assert_eq!(
            serialize(&fragment, SerializerOptions::default()),
            "<template><div>content</div></template>"
        );
    }

    #[test]
    fn non_html_template_serializes_child_nodes() {
        let fragment = create_document_fragment();
        let template = create_element("template", NS_SVG, Vec::new());
        let child = create_element("span", NS_HTML, Vec::new());
        append_child(&child, create_text_node("child"));
        append_child(&template, child);

        let content = create_document_fragment();
        append_child(&content, create_text_node("ignored content"));
        set_template_content(&template, content);
        append_child(&fragment, template);

        assert_eq!(
            serialize(&fragment, SerializerOptions::default()),
            "<template><span>child</span></template>"
        );
    }
}
