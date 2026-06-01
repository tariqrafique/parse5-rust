use crate::common::entities::{escape_attribute, escape_text};
use crate::common::html::{
    has_unescaped_text, is_void_element, NS_HTML, NS_XLINK, NS_XML, NS_XMLNS,
};
use crate::common::token::Attribute;
use crate::tree_adapters::default::{get_template_content, NodeData, NodeRef};

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

pub fn serialize(node: &NodeRef, options: SerializerOptions) -> String {
    if is_html_void_element_node(node) {
        return String::new();
    }

    serialize_child_nodes(node, options)
}

pub fn serialize_outer(node: &NodeRef, options: SerializerOptions) -> String {
    serialize_node(node, options)
}

fn serialize_child_nodes(node: &NodeRef, options: SerializerOptions) -> String {
    let container = if is_html_template_node(node) {
        get_template_content(node).unwrap_or_else(|| node.clone())
    } else {
        node.clone()
    };
    let children = container
        .borrow()
        .child_nodes()
        .cloned()
        .unwrap_or_default();

    children
        .iter()
        .map(|child| serialize_node(child, options))
        .collect()
}

fn serialize_node(node: &NodeRef, options: SerializerOptions) -> String {
    enum Snapshot {
        Document,
        DocumentFragment,
        Element {
            tag_name: String,
            attrs: Vec<Attribute>,
            namespace_uri: String,
            child_nodes: Vec<NodeRef>,
        },
        Comment(String),
        Text(String),
        DocumentType {
            name: String,
        },
    }

    let snapshot = {
        let node_ref = node.borrow();
        match &node_ref.data {
            NodeData::Document { .. } => Snapshot::Document,
            NodeData::DocumentFragment { .. } => Snapshot::DocumentFragment,
            NodeData::Element {
                tag_name,
                attrs,
                namespace_uri,
                child_nodes,
                ..
            } => Snapshot::Element {
                tag_name: tag_name.clone(),
                attrs: attrs.clone(),
                namespace_uri: namespace_uri.clone(),
                child_nodes: child_nodes.clone(),
            },
            NodeData::Comment { data } => Snapshot::Comment(data.clone()),
            NodeData::Text { value } => Snapshot::Text(value.clone()),
            NodeData::DocumentType {
                name,
                public_id: _,
                system_id: _,
            } => Snapshot::DocumentType { name: name.clone() },
        }
    };

    match snapshot {
        Snapshot::Document | Snapshot::DocumentFragment => serialize_child_nodes(node, options),
        Snapshot::Element {
            tag_name,
            attrs,
            namespace_uri,
            child_nodes,
        } => {
            let mut html = String::new();
            html.push('<');
            html.push_str(&tag_name);
            html.push_str(&serialize_attrs(&attrs));
            html.push('>');

            if namespace_uri == NS_HTML && is_void_element(&tag_name) {
                return html;
            }

            let content_root = if namespace_uri == NS_HTML && tag_name == "template" {
                get_template_content(node)
            } else {
                None
            };

            if let Some(content_root) = content_root {
                html.push_str(&serialize_child_nodes(&content_root, options));
            } else {
                for child in child_nodes {
                    html.push_str(&serialize_node(&child, options));
                }
            }

            html.push_str("</");
            html.push_str(&tag_name);
            html.push('>');
            html
        }
        Snapshot::Comment(data) => format!("<!--{data}-->"),
        Snapshot::Text(value) => {
            if has_unescaped_text_parent(node, options.scripting_enabled) {
                value
            } else {
                escape_text(&value)
            }
        }
        Snapshot::DocumentType { name } => serialize_doctype(&name),
    }
}

fn serialize_attrs(attrs: &[Attribute]) -> String {
    let mut html = String::new();

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
        html.push_str(&escape_attribute(&attr.value));
        html.push('"');
    }

    html
}

fn serialize_doctype(name: &str) -> String {
    format!("<!DOCTYPE {name}>")
}

fn is_html_void_element_node(node: &NodeRef) -> bool {
    matches!(
        &node.borrow().data,
        NodeData::Element {
            tag_name,
            namespace_uri,
            ..
        } if namespace_uri == NS_HTML && is_void_element(tag_name)
    )
}

fn is_html_template_node(node: &NodeRef) -> bool {
    matches!(
        &node.borrow().data,
        NodeData::Element {
            tag_name,
            namespace_uri,
            ..
        } if namespace_uri == NS_HTML && tag_name == "template"
    )
}

fn has_unescaped_text_parent(node: &NodeRef, scripting_enabled: bool) -> bool {
    let Some(parent) = node.borrow().parent() else {
        return false;
    };

    let parent = parent.borrow();
    matches!(
        &parent.data,
        NodeData::Element {
            tag_name,
            namespace_uri,
            ..
        } if namespace_uri == NS_HTML && has_unescaped_text(tag_name, scripting_enabled)
    )
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
