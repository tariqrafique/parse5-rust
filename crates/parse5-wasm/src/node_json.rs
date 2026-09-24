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
use serde::ser::{SerializeMap, SerializeSeq, Serializer};
use serde::Serialize;
use std::collections::BTreeMap;

/// A node and its subtree.
pub struct NodeJson<'a>(pub &'a NodeRef);

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

struct EmptyFragmentJson;

pub fn document_mode_value(mode: DocumentMode) -> &'static str {
    match mode {
        DocumentMode::NoQuirks => "no-quirks",
        DocumentMode::Quirks => "quirks",
        DocumentMode::LimitedQuirks => "limited-quirks",
    }
}

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

impl Serialize for ChildNodesJson<'_> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut seq = serializer.serialize_seq(Some(self.0.len()))?;

        for child in self.0 {
            seq.serialize_element(&NodeJson(child))?;
        }

        seq.end()
    }
}

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
