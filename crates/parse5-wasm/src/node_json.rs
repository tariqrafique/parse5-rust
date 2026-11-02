//! Streaming JSON views of the default tree, in the shape of parse5's default
//! tree adapter.
//!
//! Serializing straight from the Rust tree avoids building an intermediate
//! `serde_json::Value` (one heap-allocated map per node and location), which
//! used to dominate the cost of handing a parsed document to JS. Keys are
//! emitted in the order upstream parse5 creates them, so `Object.keys` and
//! `JSON.stringify` of a node match the reference implementation.

use parse5::common::html::{DocumentMode, NS_HTML};
use parse5::common::token::{ElementLocation, Location};
use parse5::tree_adapters::default::{Node, NodeData, NodeRef};
#[cfg(test)]
use serde::ser::SerializeSeq;
use serde::ser::{SerializeMap, Serializer};
use serde::Serialize;
use std::collections::BTreeMap;

/// A node and its subtree. Kept for the serde comparison test; production
/// encoding uses [`write_node_json`] so deep trees do not recurse.
#[cfg(test)]
pub struct NodeJson<'a>(pub &'a NodeRef);

#[cfg(test)]
struct ChildNodesJson<'a>(&'a [NodeRef]);

/// A plain `{ startLine, …, endOffset }` location.
pub struct LocationJson<'a>(pub &'a Location);

/// A location with optional per-attribute locations, as used for start tags.
struct LocationWithAttrsJson<'a> {
    location: &'a Location,
    attrs: &'a BTreeMap<String, Location>,
}

struct AttrLocationsJson<'a>(&'a BTreeMap<String, Location>);

struct ElementLocationJson<'a>(&'a ElementLocation);

#[cfg(test)]
struct EmptyFragmentJson;

pub fn document_mode_value(mode: DocumentMode) -> &'static str {
    match mode {
        DocumentMode::NoQuirks => "no-quirks",
        DocumentMode::Quirks => "quirks",
        DocumentMode::LimitedQuirks => "limited-quirks",
    }
}

#[cfg(test)]
impl Serialize for NodeJson<'_> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let node = self.0.borrow();
        let mut map = serializer.serialize_map(None)?;

        match &node.data {
            NodeData::Document { mode, child_nodes } => {
                map.serialize_entry("nodeName", "#document")?;
                map.serialize_entry("mode", document_mode_value(*mode))?;
                map.serialize_entry("childNodes", &ChildNodesJson(child_nodes))?;
                serialize_node_location(&mut map, &node)?;
            }
            NodeData::DocumentFragment { child_nodes } => {
                map.serialize_entry("nodeName", "#document-fragment")?;
                map.serialize_entry("childNodes", &ChildNodesJson(child_nodes))?;
                serialize_node_location(&mut map, &node)?;
            }
            NodeData::Element {
                tag_name,
                attrs,
                namespace_uri,
                child_nodes,
                template_content,
            } => {
                map.serialize_entry("nodeName", tag_name)?;
                map.serialize_entry("tagName", tag_name)?;
                map.serialize_entry("attrs", attrs)?;
                map.serialize_entry("namespaceURI", namespace_uri)?;
                map.serialize_entry("childNodes", &ChildNodesJson(child_nodes))?;
                map.serialize_entry("parentNode", &())?;

                if tag_name == "template" && namespace_uri == NS_HTML {
                    match template_content {
                        Some(content) => map.serialize_entry("content", &NodeJson(content))?,
                        None => map.serialize_entry("content", &EmptyFragmentJson)?,
                    }
                }

                match node.source_code_location_ref() {
                    Some(location) => {
                        map.serialize_entry("sourceCodeLocation", &ElementLocationJson(location))?
                    }
                    None if node.source_code_location_is_set() => {
                        map.serialize_entry("sourceCodeLocation", &())?
                    }
                    None => {}
                }
            }
            NodeData::Comment { data } => {
                map.serialize_entry("nodeName", "#comment")?;
                map.serialize_entry("data", data)?;
                map.serialize_entry("parentNode", &())?;
                serialize_node_location(&mut map, &node)?;
            }
            NodeData::Text { value } => {
                map.serialize_entry("nodeName", "#text")?;
                map.serialize_entry("value", value)?;
                map.serialize_entry("parentNode", &())?;
                serialize_node_location(&mut map, &node)?;
            }
            NodeData::DocumentType {
                name,
                public_id,
                system_id,
            } => {
                map.serialize_entry("nodeName", "#documentType")?;
                map.serialize_entry("name", name)?;
                map.serialize_entry("publicId", public_id)?;
                map.serialize_entry("systemId", system_id)?;
                map.serialize_entry("parentNode", &())?;
                serialize_node_location(&mut map, &node)?;
            }
        }

        map.end()
    }
}

/// Non-element nodes only expose their plain location.
#[cfg(test)]
fn serialize_node_location<M: SerializeMap>(map: &mut M, node: &Node) -> Result<(), M::Error> {
    match node.source_code_location_ref() {
        Some(location) => {
            map.serialize_entry("sourceCodeLocation", &LocationJson(&location.location))
        }
        None if node.source_code_location_is_set() => {
            map.serialize_entry("sourceCodeLocation", &())
        }
        None => Ok(()),
    }
}

#[cfg(test)]
impl Serialize for ChildNodesJson<'_> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut seq = serializer.serialize_seq(Some(self.0.len()))?;

        for child in self.0 {
            seq.serialize_element(&NodeJson(child))?;
        }

        seq.end()
    }
}

#[cfg(test)]
impl Serialize for EmptyFragmentJson {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut map = serializer.serialize_map(Some(2))?;
        map.serialize_entry("nodeName", "#document-fragment")?;
        map.serialize_entry("childNodes", &[(); 0])?;
        map.end()
    }
}

fn serialize_location_fields<M: SerializeMap>(
    map: &mut M,
    location: &Location,
) -> Result<(), M::Error> {
    map.serialize_entry("startLine", &location.start_line)?;
    map.serialize_entry("startCol", &location.start_col)?;
    map.serialize_entry("startOffset", &location.start_offset)?;
    map.serialize_entry("endLine", &location.end_line)?;
    map.serialize_entry("endCol", &location.end_col)?;
    map.serialize_entry("endOffset", &location.end_offset)
}

impl Serialize for LocationJson<'_> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut map = serializer.serialize_map(Some(6))?;
        serialize_location_fields(&mut map, self.0)?;
        map.end()
    }
}

impl Serialize for LocationWithAttrsJson<'_> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut map = serializer.serialize_map(None)?;
        serialize_location_fields(&mut map, self.location)?;

        if !self.attrs.is_empty() {
            map.serialize_entry("attrs", &AttrLocationsJson(self.attrs))?;
        }

        map.end()
    }
}

impl Serialize for AttrLocationsJson<'_> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut map = serializer.serialize_map(Some(self.0.len()))?;

        for (name, location) in self.0 {
            map.serialize_entry(name, &LocationJson(location))?;
        }

        map.end()
    }
}

impl Serialize for ElementLocationJson<'_> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let location = self.0;
        let mut map = serializer.serialize_map(None)?;
        serialize_location_fields(&mut map, &location.location)?;

        if !location.attrs.is_empty() {
            map.serialize_entry("attrs", &AttrLocationsJson(&location.attrs))?;
        }

        if let Some(start_tag) = &location.start_tag {
            map.serialize_entry(
                "startTag",
                &LocationWithAttrsJson {
                    location: start_tag,
                    attrs: &location.attrs,
                },
            )?;
        }

        if let Some(end_tag) = &location.end_tag {
            map.serialize_entry("endTag", &LocationJson(end_tag))?;
        }

        map.end()
    }
}

enum Phase {
    Open,
    Children,
    AfterContent,
}

struct Frame {
    node: NodeRef,
    children: Vec<NodeRef>,
    child_index: usize,
    phase: Phase,
}

enum TemplateContent {
    None,
    Empty,
    Node(NodeRef),
}

/// JSON for `node` and its subtree, written with an explicit heap stack.
///
/// `serde_json` walks the tree by recursion, which overflows the wasm stack
/// between 5k and 10k nested elements. The bytes match `NodeJson`'s `Serialize`
/// impl for trees that fit on the stack.
pub fn write_node_json(out: &mut String, root: &NodeRef) {
    let mut stack = vec![Frame {
        node: root.clone(),
        children: Vec::new(),
        child_index: 0,
        phase: Phase::Open,
    }];

    while let Some(index) = stack.len().checked_sub(1) {
        match stack[index].phase {
            Phase::Open => {
                let node = stack[index].node.clone();
                if !has_child_list(&node) {
                    write_leaf(out, &node);
                    stack.pop();
                    continue;
                }
                write_open(out, &node);
                stack[index].children = children_of(&node);
                stack[index].phase = Phase::Children;
                stack[index].child_index = 0;
            }
            Phase::Children => {
                let child_index = stack[index].child_index;
                if child_index < stack[index].children.len() {
                    if child_index > 0 {
                        out.push(',');
                    }
                    let child = stack[index].children[child_index].clone();
                    stack[index].child_index += 1;
                    stack.push(Frame {
                        node: child,
                        children: Vec::new(),
                        child_index: 0,
                        phase: Phase::Open,
                    });
                    continue;
                }
                out.push(']');
                let node = stack[index].node.clone();
                match template_content(&node) {
                    TemplateContent::Node(content) => {
                        out.push_str(",\"parentNode\":null,\"content\":");
                        stack[index].phase = Phase::AfterContent;
                        stack.push(Frame {
                            node: content,
                            children: Vec::new(),
                            child_index: 0,
                            phase: Phase::Open,
                        });
                    }
                    TemplateContent::Empty => {
                        out.push_str(",\"parentNode\":null,\"content\":");
                        write_empty_fragment(out);
                        write_element_location(out, &node);
                        out.push('}');
                        stack.pop();
                    }
                    TemplateContent::None => {
                        write_tail(out, &node);
                        stack.pop();
                    }
                }
            }
            Phase::AfterContent => {
                let node = stack[index].node.clone();
                write_element_location(out, &node);
                out.push('}');
                stack.pop();
            }
        }
    }
}

fn has_child_list(node: &NodeRef) -> bool {
    node.borrow().child_nodes().is_some()
}

fn children_of(node: &NodeRef) -> Vec<NodeRef> {
    node.borrow().child_nodes().cloned().unwrap_or_default()
}

fn template_content(node: &NodeRef) -> TemplateContent {
    let borrowed = node.borrow();
    match &borrowed.data {
        NodeData::Element {
            tag_name,
            namespace_uri,
            template_content,
            ..
        } if tag_name == "template" && namespace_uri == NS_HTML => match template_content {
            Some(content) => TemplateContent::Node(content.clone()),
            None => TemplateContent::Empty,
        },
        _ => TemplateContent::None,
    }
}

fn push_json_str(out: &mut String, value: &str) {
    out.push_str(&serde_json::to_string(value).expect("string json"));
}

fn write_open(out: &mut String, node: &NodeRef) {
    let borrowed = node.borrow();
    out.push('{');
    match &borrowed.data {
        NodeData::Document { mode, .. } => {
            out.push_str("\"nodeName\":\"#document\",\"mode\":");
            push_json_str(out, document_mode_value(*mode));
        }
        NodeData::DocumentFragment { .. } => {
            out.push_str("\"nodeName\":\"#document-fragment\"");
        }
        NodeData::Element {
            tag_name,
            attrs,
            namespace_uri,
            ..
        } => {
            out.push_str("\"nodeName\":");
            push_json_str(out, tag_name);
            out.push_str(",\"tagName\":");
            push_json_str(out, tag_name);
            out.push_str(",\"attrs\":");
            out.push_str(&serde_json::to_string(attrs).expect("attrs json"));
            out.push_str(",\"namespaceURI\":");
            push_json_str(out, namespace_uri);
        }
        NodeData::Comment { .. } | NodeData::Text { .. } | NodeData::DocumentType { .. } => {
            unreachable!("leaves are written by write_leaf")
        }
    }
    out.push_str(",\"childNodes\":[");
}

fn write_tail(out: &mut String, node: &NodeRef) {
    let borrowed = node.borrow();
    match &borrowed.data {
        NodeData::Document { .. } | NodeData::DocumentFragment { .. } => {
            write_plain_location(out, &borrowed);
            out.push('}');
        }
        NodeData::Element { .. } => {
            out.push_str(",\"parentNode\":null");
            drop(borrowed);
            write_element_location(out, node);
            out.push('}');
        }
        NodeData::Comment { .. } | NodeData::Text { .. } | NodeData::DocumentType { .. } => {
            unreachable!("leaves are written by write_leaf")
        }
    }
}

fn write_leaf(out: &mut String, node: &NodeRef) {
    let borrowed = node.borrow();
    out.push('{');
    match &borrowed.data {
        NodeData::Comment { data } => {
            out.push_str("\"nodeName\":\"#comment\",\"data\":");
            push_json_str(out, data);
        }
        NodeData::Text { value } => {
            out.push_str("\"nodeName\":\"#text\",\"value\":");
            push_json_str(out, value);
        }
        NodeData::DocumentType {
            name,
            public_id,
            system_id,
        } => {
            out.push_str("\"nodeName\":\"#documentType\",\"name\":");
            push_json_str(out, name);
            out.push_str(",\"publicId\":");
            push_json_str(out, public_id);
            out.push_str(",\"systemId\":");
            push_json_str(out, system_id);
        }
        NodeData::Document { .. }
        | NodeData::DocumentFragment { .. }
        | NodeData::Element { .. } => unreachable!("parent nodes are written by write_open"),
    }
    out.push_str(",\"parentNode\":null");
    write_plain_location(out, &borrowed);
    out.push('}');
}

fn write_plain_location(out: &mut String, node: &Node) {
    match node.source_code_location_ref() {
        Some(location) => {
            out.push_str(",\"sourceCodeLocation\":");
            out.push_str(
                &serde_json::to_string(&LocationJson(&location.location)).expect("location json"),
            );
        }
        None if node.source_code_location_is_set() => {
            out.push_str(",\"sourceCodeLocation\":null");
        }
        None => {}
    }
}

fn write_element_location(out: &mut String, node: &NodeRef) {
    let borrowed = node.borrow();
    match borrowed.source_code_location_ref() {
        Some(location) => {
            out.push_str(",\"sourceCodeLocation\":");
            out.push_str(
                &serde_json::to_string(&ElementLocationJson(location)).expect("element location"),
            );
        }
        None if borrowed.source_code_location_is_set() => {
            out.push_str(",\"sourceCodeLocation\":null");
        }
        None => {}
    }
}

fn write_empty_fragment(out: &mut String) {
    out.push_str("{\"nodeName\":\"#document-fragment\",\"childNodes\":[]}");
}

#[cfg(test)]
mod tests {
    use super::{write_node_json, NodeJson};
    use parse5::{parse, parse_fragment, ParserOptions};

    fn assert_matches_serde(html: &str, fragment: bool) {
        let options = ParserOptions {
            source_code_location_info: true,
            ..ParserOptions::default()
        };
        let node = if fragment {
            parse_fragment(html, options)
        } else {
            parse(html, options)
        };
        let via_serde = serde_json::to_string(&NodeJson(&node)).expect("serde");
        let mut via_stack = String::new();
        write_node_json(&mut via_stack, &node);
        assert_eq!(via_stack, via_serde);
    }

    #[test]
    fn iterative_json_matches_serde_for_documents_and_fragments() {
        assert_matches_serde(
            "<!DOCTYPE html><html><body><div id=\"a\"><template><p>hi &amp; yo</p></template><!--c--><svg viewBox=\"0 0 1 1\"><g></g></svg></div></body></html>",
            false,
        );
        assert_matches_serde("<div a=\"x\" /><span b>text</span>", true);
        assert_matches_serde("<template></template>", true);
    }

    #[test]
    fn iterative_json_writes_a_deeply_nested_tree() {
        let html = "<div>".repeat(5_000);
        let document = parse(&html, ParserOptions::default());
        let mut json = String::new();
        write_node_json(&mut json, &document);
        assert!(json.matches("\"nodeName\":\"div\"").count() >= 5_000);
    }
}
