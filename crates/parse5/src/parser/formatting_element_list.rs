use crate::common::token::{Attribute, TagToken};
use crate::tree_adapters::default::{NodeData, NodeRef};
use std::collections::HashMap;
use std::rc::Rc;

const NOAH_ARK_CAPACITY: usize = 3;
const MARKER_ID: u64 = 0;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EntryType {
    Marker,
    Element,
}

#[derive(Debug, Clone)]
pub struct Entry {
    pub entry_type: EntryType,
    pub element: Option<NodeRef>,
    pub token: Option<TagToken>,
    id: u64,
}

impl Entry {
    fn marker() -> Self {
        Self {
            entry_type: EntryType::Marker,
            element: None,
            token: None,
            id: MARKER_ID,
        }
    }

    fn element(id: u64, element: NodeRef, token: TagToken) -> Self {
        Self {
            entry_type: EntryType::Element,
            element: Some(element),
            token: Some(token),
            id,
        }
    }

    pub fn is_marker(&self) -> bool {
        self.entry_type == EntryType::Marker
    }

    pub fn element_ref(&self) -> Option<NodeRef> {
        self.element.clone()
    }
}

#[derive(Debug)]
pub struct FormattingElementList {
    pub entries: Vec<Entry>,
    pub bookmark: Option<u64>,
    next_entry_id: u64,
}

impl Default for FormattingElementList {
    fn default() -> Self {
        Self {
            entries: Vec::new(),
            bookmark: None,
            next_entry_id: 1,
        }
    }
}

impl FormattingElementList {
    pub fn new() -> Self {
        Self::default()
    }

    fn next_id(&mut self) -> u64 {
        let id = self.next_entry_id;
        self.next_entry_id += 1;
        id
    }

    fn get_noah_ark_condition_candidates(
        &self,
        new_element: &NodeRef,
        new_attrs: &[Attribute],
    ) -> Vec<(usize, Vec<Attribute>)> {
        let mut candidates = Vec::new();
        let Some((new_tag_name, new_namespace_uri)) = element_name_and_namespace(new_element)
        else {
            return candidates;
        };

        for (idx, entry) in self.entries.iter().enumerate() {
            if entry.entry_type == EntryType::Marker {
                break;
            }

            let Some(element) = &entry.element else {
                continue;
            };
            let Some((tag_name, namespace_uri)) = element_name_and_namespace(element) else {
                continue;
            };

            if tag_name == new_tag_name && namespace_uri == new_namespace_uri {
                let attrs = element_attrs(element);

                if attrs.len() == new_attrs.len() {
                    candidates.push((idx, attrs));
                }
            }
        }

        candidates
    }

    fn ensure_noah_ark_condition(&mut self, new_element: &NodeRef) {
        if self.entries.len() < NOAH_ARK_CAPACITY {
            return;
        }

        let new_attrs = element_attrs(new_element);
        let candidates = self.get_noah_ark_condition_candidates(new_element, &new_attrs);

        if candidates.len() < NOAH_ARK_CAPACITY {
            return;
        }

        let new_attrs_by_name: HashMap<&str, &str> = new_attrs
            .iter()
            .map(|attr| (attr.name.as_str(), attr.value.as_str()))
            .collect();
        let mut valid_candidates = 0;
        let mut removed_before_current = 0usize;

        for (candidate_idx, candidate_attrs) in candidates {
            if candidate_attrs
                .iter()
                .all(|attr| new_attrs_by_name.get(attr.name.as_str()) == Some(&attr.value.as_str()))
            {
                valid_candidates += 1;

                if valid_candidates >= NOAH_ARK_CAPACITY {
                    let idx = candidate_idx.saturating_sub(removed_before_current);
                    if idx < self.entries.len() {
                        self.entries.remove(idx);
                        removed_before_current += 1;
                    }
                }
            }
        }
    }

    pub fn insert_marker(&mut self) {
        self.entries.insert(0, Entry::marker());
    }

    pub fn push_element(&mut self, element: NodeRef, token: TagToken) {
        self.ensure_noah_ark_condition(&element);
        let id = self.next_id();
        self.entries.insert(0, Entry::element(id, element, token));
    }

    pub fn set_bookmark(&mut self, entry: &Entry) {
        self.bookmark = Some(entry.id);
    }

    pub fn set_bookmark_to_entry_index(&mut self, idx: usize) {
        self.bookmark = self.entries.get(idx).map(|entry| entry.id);
    }

    pub fn insert_element_after_bookmark(&mut self, element: NodeRef, token: TagToken) {
        let bookmark_idx = self
            .bookmark
            .and_then(|id| self.entries.iter().position(|entry| entry.id == id))
            .unwrap_or(0);
        let id = self.next_id();

        self.entries
            .insert(bookmark_idx, Entry::element(id, element, token));
    }

    pub fn remove_entry(&mut self, entry: &Entry) {
        if let Some(entry_idx) = self.entries.iter().position(|item| same_entry(item, entry)) {
            self.entries.remove(entry_idx);
        }
    }

    pub fn replace_entry_element(&mut self, entry: &Entry, element: NodeRef) {
        if let Some(entry) = self.entries.iter_mut().find(|item| same_entry(item, entry)) {
            entry.element = Some(element);
        }
    }

    pub fn clear_to_last_marker(&mut self) {
        if let Some(marker_idx) = self
            .entries
            .iter()
            .position(|entry| entry.entry_type == EntryType::Marker)
        {
            self.entries.drain(..=marker_idx);
        } else {
            self.entries.clear();
        }
    }

    pub fn get_element_entry_in_scope_with_tag_name(&self, tag_name: &str) -> Option<Entry> {
        let entry = self.entries.iter().find(|entry| {
            entry.entry_type == EntryType::Marker
                || entry.element.as_ref().and_then(element_tag_name).as_deref() == Some(tag_name)
        })?;

        (entry.entry_type == EntryType::Element).then(|| entry.clone())
    }

    pub fn get_element_entry(&self, element: &NodeRef) -> Option<Entry> {
        self.entries
            .iter()
            .find(|entry| {
                entry.entry_type == EntryType::Element
                    && entry
                        .element
                        .as_ref()
                        .is_some_and(|entry_element| Rc::ptr_eq(entry_element, element))
            })
            .cloned()
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

fn same_entry(left: &Entry, right: &Entry) -> bool {
    if left.id == MARKER_ID || right.id == MARKER_ID {
        left.entry_type == right.entry_type
    } else {
        left.id == right.id
    }
}

fn element_tag_name(element: &NodeRef) -> Option<String> {
    match &element.borrow().data {
        NodeData::Element { tag_name, .. } => Some(tag_name.clone()),
        _ => None,
    }
}

fn element_name_and_namespace(element: &NodeRef) -> Option<(String, String)> {
    match &element.borrow().data {
        NodeData::Element {
            tag_name,
            namespace_uri,
            ..
        } => Some((tag_name.clone(), namespace_uri.clone())),
        _ => None,
    }
}

fn element_attrs(element: &NodeRef) -> Vec<Attribute> {
    match &element.borrow().data {
        NodeData::Element { attrs, .. } => attrs.clone(),
        _ => Vec::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::{EntryType, FormattingElementList};
    use crate::common::html::{get_tag_id, NS_HTML};
    use crate::common::token::{Attribute, TagToken};
    use crate::tree_adapters::default::create_element;
    use std::rc::Rc;

    fn token(name: &str) -> TagToken {
        TagToken::new_start_tag(name, get_tag_id(name), Vec::new())
    }

    fn attr(name: &str, value: &str) -> Attribute {
        Attribute::new(name, value)
    }

    #[test]
    fn inserts_markers_at_the_front_like_parse5() {
        let mut list = FormattingElementList::new();

        list.insert_marker();
        list.insert_marker();

        assert_eq!(list.entries.len(), 2);
        assert_eq!(list.entries[0].entry_type, EntryType::Marker);
        assert_eq!(list.entries[1].entry_type, EntryType::Marker);
    }

    #[test]
    fn pushes_elements_at_the_front_with_tokens() {
        let mut list = FormattingElementList::new();
        let div = create_element("div", NS_HTML, Vec::new());
        let p = create_element("p", NS_HTML, Vec::new());
        let div_token = token("div");
        let p_token = token("p");

        list.push_element(div.clone(), div_token.clone());
        list.push_element(p.clone(), p_token.clone());

        assert_eq!(list.entries.len(), 2);
        assert_eq!(list.entries[0].entry_type, EntryType::Element);
        assert!(Rc::ptr_eq(list.entries[0].element.as_ref().unwrap(), &p));
        assert_eq!(list.entries[0].token.as_ref(), Some(&p_token));
        assert!(Rc::ptr_eq(list.entries[1].element.as_ref().unwrap(), &div));
        assert_eq!(list.entries[1].token.as_ref(), Some(&div_token));
    }

    #[test]
    fn inserts_element_after_bookmark() {
        let mut list = FormattingElementList::new();
        let div = create_element("div", NS_HTML, Vec::new());
        let p = create_element("p", NS_HTML, Vec::new());
        let span = create_element("span", NS_HTML, Vec::new());
        let title = create_element("title", NS_HTML, Vec::new());

        list.push_element(div, token("div"));
        list.set_bookmark_to_entry_index(0);
        list.push_element(p, token("p"));
        list.push_element(span, token("span"));
        list.insert_element_after_bookmark(title.clone(), token("title"));

        assert_eq!(list.entries.len(), 4);
        assert!(Rc::ptr_eq(
            list.entries[2].element.as_ref().unwrap(),
            &title
        ));
    }

    #[test]
    fn enforces_noah_ark_condition_before_the_last_marker() {
        let mut list = FormattingElementList::new();
        let element1 = create_element(
            "div",
            NS_HTML,
            vec![attr("attr1", "val1"), attr("attr2", "val2")],
        );
        let element2 = create_element(
            "div",
            NS_HTML,
            vec![attr("attr1", "val1"), attr("attr2", "someOtherValue")],
        );
        let token1 = token("div");
        let token2 = token("div");
        let token3 = token("div");
        let token4 = token("div");
        let token5 = token("div");
        let token6 = token("div");

        list.push_element(element1.clone(), token1);
        list.push_element(element1.clone(), token2.clone());
        list.push_element(element2, token3.clone());
        list.push_element(element1.clone(), token4.clone());
        list.push_element(element1.clone(), token5.clone());

        assert_eq!(list.entries.len(), 4);
        assert_eq!(list.entries[3].token.as_ref(), Some(&token2));
        assert_eq!(list.entries[2].token.as_ref(), Some(&token3));
        assert_eq!(list.entries[1].token.as_ref(), Some(&token4));
        assert_eq!(list.entries[0].token.as_ref(), Some(&token5));

        list.insert_marker();
        list.push_element(element1, token6.clone());

        assert_eq!(list.entries.len(), 6);
        assert_eq!(list.entries[1].entry_type, EntryType::Marker);
        assert_eq!(list.entries[0].token.as_ref(), Some(&token6));
    }

    #[test]
    fn clears_to_last_marker_or_entire_list() {
        let mut list = FormattingElementList::new();
        let element1 = create_element(
            "div",
            NS_HTML,
            vec![attr("attr1", "val1"), attr("attr2", "val2")],
        );
        let element2 = create_element(
            "div",
            NS_HTML,
            vec![attr("attr1", "val1"), attr("attr2", "someOtherValue")],
        );

        list.push_element(element1.clone(), token("div"));
        list.push_element(element2.clone(), token("div"));
        list.insert_marker();
        list.push_element(element1.clone(), token("div"));
        list.push_element(element1, token("div"));
        list.push_element(element2, token("div"));
        list.clear_to_last_marker();

        assert_eq!(list.entries.len(), 2);

        list.clear_to_last_marker();

        assert_eq!(list.entries.len(), 0);
    }

    #[test]
    fn removes_entry_by_entry_identity() {
        let mut list = FormattingElementList::new();
        let element1 = create_element("div", NS_HTML, Vec::new());
        let element2 = create_element("div", NS_HTML, Vec::new());

        list.push_element(element1.clone(), token("div"));
        list.push_element(element2.clone(), token("div"));
        list.push_element(element2, token("div"));
        let bottom_entry = list.entries[2].clone();

        list.remove_entry(&bottom_entry);

        assert_eq!(list.entries.len(), 2);
        assert!(list
            .entries
            .iter()
            .all(|entry| !Rc::ptr_eq(entry.element.as_ref().unwrap(), &element1)));
    }

    #[test]
    fn finds_entry_in_scope_with_tag_name_until_marker() {
        let mut list = FormattingElementList::new();
        let element = create_element("div", NS_HTML, Vec::new());

        assert!(list
            .get_element_entry_in_scope_with_tag_name("div")
            .is_none());

        list.push_element(element.clone(), token("div"));
        list.push_element(element.clone(), token("div"));
        let entry = list
            .get_element_entry_in_scope_with_tag_name("div")
            .unwrap();
        assert!(Rc::ptr_eq(entry.element.as_ref().unwrap(), &element));

        list.insert_marker();
        assert!(list
            .get_element_entry_in_scope_with_tag_name("div")
            .is_none());

        list.push_element(element, token("div"));
        assert!(list
            .get_element_entry_in_scope_with_tag_name("div")
            .is_some());
    }

    #[test]
    fn gets_element_entry_by_node_identity() {
        let mut list = FormattingElementList::new();
        let element1 = create_element("div", NS_HTML, Vec::new());
        let element2 = create_element("a", NS_HTML, Vec::new());

        list.push_element(element2.clone(), token("div"));
        list.push_element(element1.clone(), token("div"));
        list.push_element(element2, token("div"));
        list.insert_marker();

        let entry = list.get_element_entry(&element1).unwrap();

        assert_eq!(entry.entry_type, EntryType::Element);
        assert!(Rc::ptr_eq(entry.element.as_ref().unwrap(), &element1));
    }
}
