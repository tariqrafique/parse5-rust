use crate::common::html::{DocumentMode, NS_HTML};
use crate::common::token::{Attribute, ElementLocation};
use crate::tree_adapters::interface::TreeAdapter;
use std::cell::RefCell;
use std::rc::{Rc, Weak};

pub use crate::common::html::DocumentMode as DefaultDocumentMode;

pub type NodeRef = Rc<RefCell<Node>>;

#[derive(Debug, Clone, Copy, Default)]
pub struct DefaultTreeAdapter;

pub const DEFAULT_TREE_ADAPTER: DefaultTreeAdapter = DefaultTreeAdapter;

#[derive(Debug)]
pub struct Node {
    parent: Option<Weak<RefCell<Node>>>,
    source_code_location_set: bool,
    source_code_location: Option<ElementLocation>,
    pub data: NodeData,
}

#[derive(Debug)]
pub enum NodeData {
    Document {
        mode: DocumentMode,
        child_nodes: Vec<NodeRef>,
    },
    DocumentFragment {
        child_nodes: Vec<NodeRef>,
    },
    Element {
        node_name: String,
        tag_name: String,
        attrs: Vec<Attribute>,
        namespace_uri: String,
        child_nodes: Vec<NodeRef>,
        template_content: Option<NodeRef>,
    },
    Comment {
        data: String,
    },
    Text {
        value: String,
    },
    DocumentType {
        name: String,
        public_id: String,
        system_id: String,
    },
}

impl Node {
    pub fn new_ref(data: NodeData) -> NodeRef {
        Rc::new(RefCell::new(Self {
            parent: None,
            source_code_location_set: false,
            source_code_location: None,
            data,
        }))
    }

    pub fn document() -> NodeRef {
        Self::new_ref(NodeData::Document {
            mode: DocumentMode::NoQuirks,
            child_nodes: Vec::new(),
        })
    }

    pub fn document_fragment() -> NodeRef {
        Self::new_ref(NodeData::DocumentFragment {
            child_nodes: Vec::new(),
        })
    }

    pub fn element(
        tag_name: impl Into<String>,
        namespace_uri: impl Into<String>,
        attrs: Vec<Attribute>,
    ) -> NodeRef {
        let tag_name = tag_name.into();
        Self::new_ref(NodeData::Element {
            node_name: tag_name.clone(),
            tag_name,
            attrs,
            namespace_uri: namespace_uri.into(),
            child_nodes: Vec::new(),
            template_content: None,
        })
    }

    pub fn html_element(tag_name: impl Into<String>, attrs: Vec<Attribute>) -> NodeRef {
        Self::element(tag_name, NS_HTML, attrs)
    }

    pub fn comment(data: impl Into<String>) -> NodeRef {
        Self::new_ref(NodeData::Comment { data: data.into() })
    }

    pub fn text(value: impl Into<String>) -> NodeRef {
        Self::new_ref(NodeData::Text {
            value: value.into(),
        })
    }

    pub fn document_type(
        name: impl Into<String>,
        public_id: impl Into<String>,
        system_id: impl Into<String>,
    ) -> NodeRef {
        Self::new_ref(NodeData::DocumentType {
            name: name.into(),
            public_id: public_id.into(),
            system_id: system_id.into(),
        })
    }

    pub fn node_name(&self) -> &str {
        match &self.data {
            NodeData::Document { .. } => "#document",
            NodeData::DocumentFragment { .. } => "#document-fragment",
            NodeData::Element { node_name, .. } => node_name,
            NodeData::Comment { .. } => "#comment",
            NodeData::Text { .. } => "#text",
            NodeData::DocumentType { .. } => "#documentType",
        }
    }

    pub fn parent(&self) -> Option<NodeRef> {
        self.parent.as_ref().and_then(Weak::upgrade)
    }

    pub fn source_code_location(&self) -> Option<ElementLocation> {
        self.source_code_location.clone()
    }

    pub fn source_code_location_is_set(&self) -> bool {
        self.source_code_location_set
    }

    pub fn set_source_code_location(&mut self, location: Option<ElementLocation>) {
        self.source_code_location_set = true;
        self.source_code_location = location;
    }

    pub fn is_document(&self) -> bool {
        matches!(self.data, NodeData::Document { .. })
    }

    pub fn is_document_fragment(&self) -> bool {
        matches!(self.data, NodeData::DocumentFragment { .. })
    }

    pub fn is_element(&self) -> bool {
        matches!(self.data, NodeData::Element { .. })
    }

    pub fn is_template(&self) -> bool {
        matches!(&self.data, NodeData::Element { tag_name, .. } if tag_name == "template")
    }

    pub fn is_comment(&self) -> bool {
        matches!(self.data, NodeData::Comment { .. })
    }

    pub fn is_text(&self) -> bool {
        matches!(self.data, NodeData::Text { .. })
    }

    pub fn is_document_type(&self) -> bool {
        matches!(self.data, NodeData::DocumentType { .. })
    }

    pub fn child_nodes(&self) -> Option<&Vec<NodeRef>> {
        match &self.data {
            NodeData::Document { child_nodes, .. }
            | NodeData::DocumentFragment { child_nodes }
            | NodeData::Element { child_nodes, .. } => Some(child_nodes),
            _ => None,
        }
    }

    pub fn child_nodes_mut(&mut self) -> Option<&mut Vec<NodeRef>> {
        match &mut self.data {
            NodeData::Document { child_nodes, .. }
            | NodeData::DocumentFragment { child_nodes }
            | NodeData::Element { child_nodes, .. } => Some(child_nodes),
            _ => None,
        }
    }
}

pub fn create_document() -> NodeRef {
    Node::document()
}

pub fn create_document_fragment() -> NodeRef {
    Node::document_fragment()
}

pub fn create_element(
    tag_name: impl Into<String>,
    namespace_uri: impl Into<String>,
    attrs: Vec<Attribute>,
) -> NodeRef {
    Node::element(tag_name, namespace_uri, attrs)
}

pub fn create_comment_node(data: impl Into<String>) -> NodeRef {
    Node::comment(data)
}

pub fn create_text_node(value: impl Into<String>) -> NodeRef {
    Node::text(value)
}

pub fn create_document_type(
    name: impl Into<String>,
    public_id: impl Into<String>,
    system_id: impl Into<String>,
) -> NodeRef {
    Node::document_type(name, public_id, system_id)
}

pub fn append_child(parent: &NodeRef, new_node: NodeRef) {
    detach_node(&new_node);
    new_node.borrow_mut().parent = Some(Rc::downgrade(parent));

    let mut parent_ref = parent.borrow_mut();
    parent_ref
        .child_nodes_mut()
        .expect("append_child parent must be a parent node")
        .push(new_node);
}

pub fn insert_before(parent: &NodeRef, new_node: NodeRef, reference_node: &NodeRef) {
    detach_node(&new_node);
    new_node.borrow_mut().parent = Some(Rc::downgrade(parent));

    let mut parent_ref = parent.borrow_mut();
    let child_nodes = parent_ref
        .child_nodes_mut()
        .expect("insert_before parent must be a parent node");
    let insertion_idx = child_nodes
        .iter()
        .position(|node| Rc::ptr_eq(node, reference_node))
        .expect("reference node must be a child of parent");
    child_nodes.insert(insertion_idx, new_node);
}

pub fn detach_node(node: &NodeRef) {
    let parent = node.borrow().parent();

    if let Some(parent) = parent {
        if let Some(child_nodes) = parent.borrow_mut().child_nodes_mut() {
            child_nodes.retain(|child| !Rc::ptr_eq(child, node));
        }
    }

    node.borrow_mut().parent = None;
}

pub fn insert_text(parent: &NodeRef, text: &str) {
    let maybe_last = parent
        .borrow()
        .child_nodes()
        .and_then(|children| children.last().cloned());

    if let Some(last) = maybe_last {
        if let NodeData::Text { value } = &mut last.borrow_mut().data {
            value.push_str(text);
            return;
        }
    }

    append_child(parent, create_text_node(text));
}

pub fn insert_text_before(parent: &NodeRef, text: &str, reference_node: &NodeRef) {
    let maybe_prev = {
        let parent_ref = parent.borrow();
        parent_ref.child_nodes().and_then(|children| {
            children
                .iter()
                .position(|node| Rc::ptr_eq(node, reference_node))
                .and_then(|idx| idx.checked_sub(1))
                .and_then(|idx| children.get(idx).cloned())
        })
    };

    if let Some(prev) = maybe_prev {
        if let NodeData::Text { value } = &mut prev.borrow_mut().data {
            value.push_str(text);
            return;
        }
    }

    insert_before(parent, create_text_node(text), reference_node);
}

pub fn set_template_content(template_element: &NodeRef, content: NodeRef) {
    let mut template = template_element.borrow_mut();
    match &mut template.data {
        NodeData::Element {
            template_content, ..
        } => {
            content.borrow_mut().parent = None;
            *template_content = Some(content);
        }
        _ => panic!("set_template_content target must be an element"),
    }
}

pub fn get_template_content(template_element: &NodeRef) -> Option<NodeRef> {
    match &template_element.borrow().data {
        NodeData::Element {
            template_content, ..
        } => template_content.clone(),
        _ => None,
    }
}

pub fn adopt_attributes(recipient: &NodeRef, attrs: Vec<Attribute>) {
    let mut recipient_ref = recipient.borrow_mut();
    let NodeData::Element {
        attrs: recipient_attrs,
        ..
    } = &mut recipient_ref.data
    else {
        panic!("adopt_attributes recipient must be an element");
    };

    for attr in attrs {
        if !recipient_attrs
            .iter()
            .any(|existing| existing.name == attr.name)
        {
            recipient_attrs.push(attr);
        }
    }
}

pub fn set_document_type(
    document: &NodeRef,
    name: impl Into<String>,
    public_id: impl Into<String>,
    system_id: impl Into<String>,
) {
    let name = name.into();
    let public_id = public_id.into();
    let system_id = system_id.into();
    let existing_doctype = document.borrow().child_nodes().and_then(|children| {
        children
            .iter()
            .find(|node| node.borrow().is_document_type())
            .cloned()
    });

    if let Some(doctype) = existing_doctype {
        doctype.borrow_mut().data = NodeData::DocumentType {
            name,
            public_id,
            system_id,
        };
    } else {
        append_child(document, create_document_type(name, public_id, system_id));
    }
}

pub fn set_document_mode(document: &NodeRef, mode: DocumentMode) {
    let mut document_ref = document.borrow_mut();
    let NodeData::Document {
        mode: document_mode,
        ..
    } = &mut document_ref.data
    else {
        panic!("set_document_mode target must be a document");
    };

    *document_mode = mode;
}

pub fn get_document_mode(document: &NodeRef) -> DocumentMode {
    match &document.borrow().data {
        NodeData::Document { mode, .. } => *mode,
        _ => panic!("get_document_mode target must be a document"),
    }
}

pub fn get_first_child(node: &NodeRef) -> Option<NodeRef> {
    node.borrow()
        .child_nodes()
        .and_then(|children| children.first().cloned())
}

pub fn get_child_nodes(node: &NodeRef) -> Option<Vec<NodeRef>> {
    node.borrow().child_nodes().cloned()
}

pub fn get_parent_node(node: &NodeRef) -> Option<NodeRef> {
    node.borrow().parent()
}

pub fn get_attr_list(element: &NodeRef) -> Vec<Attribute> {
    match &element.borrow().data {
        NodeData::Element { attrs, .. } => attrs.clone(),
        _ => panic!("get_attr_list target must be an element"),
    }
}

pub fn get_tag_name(element: &NodeRef) -> String {
    match &element.borrow().data {
        NodeData::Element { tag_name, .. } => tag_name.clone(),
        _ => panic!("get_tag_name target must be an element"),
    }
}

pub fn get_namespace_uri(element: &NodeRef) -> String {
    match &element.borrow().data {
        NodeData::Element { namespace_uri, .. } => namespace_uri.clone(),
        _ => panic!("get_namespace_uri target must be an element"),
    }
}

pub fn get_text_node_content(text_node: &NodeRef) -> String {
    match &text_node.borrow().data {
        NodeData::Text { value } => value.clone(),
        _ => panic!("get_text_node_content target must be a text node"),
    }
}

pub fn get_comment_node_content(comment_node: &NodeRef) -> String {
    match &comment_node.borrow().data {
        NodeData::Comment { data } => data.clone(),
        _ => panic!("get_comment_node_content target must be a comment node"),
    }
}

pub fn get_document_type_node_name(doctype_node: &NodeRef) -> String {
    match &doctype_node.borrow().data {
        NodeData::DocumentType { name, .. } => name.clone(),
        _ => panic!("get_document_type_node_name target must be a document type node"),
    }
}

pub fn get_document_type_node_public_id(doctype_node: &NodeRef) -> String {
    match &doctype_node.borrow().data {
        NodeData::DocumentType { public_id, .. } => public_id.clone(),
        _ => panic!("get_document_type_node_public_id target must be a document type node"),
    }
}

pub fn get_document_type_node_system_id(doctype_node: &NodeRef) -> String {
    match &doctype_node.borrow().data {
        NodeData::DocumentType { system_id, .. } => system_id.clone(),
        _ => panic!("get_document_type_node_system_id target must be a document type node"),
    }
}

pub fn set_node_source_code_location(node: &NodeRef, location: Option<ElementLocation>) {
    node.borrow_mut().set_source_code_location(location);
}

pub fn get_node_source_code_location(node: &NodeRef) -> Option<ElementLocation> {
    node.borrow().source_code_location()
}

pub fn update_node_source_code_location(node: &NodeRef, end_location: ElementLocation) {
    let mut node_ref = node.borrow_mut();
    let mut location = node_ref
        .source_code_location
        .clone()
        .unwrap_or_else(|| end_location.clone());

    location.location.end_line = end_location.location.end_line;
    location.location.end_col = end_location.location.end_col;
    location.location.end_offset = end_location.location.end_offset;

    if !end_location.attrs.is_empty() {
        location.attrs = end_location.attrs;
    }
    if end_location.start_tag.is_some() {
        location.start_tag = end_location.start_tag;
    }
    if end_location.end_tag.is_some() {
        location.end_tag = end_location.end_tag;
    }

    node_ref.source_code_location = Some(location);
}

impl TreeAdapter for DefaultTreeAdapter {
    type Node = NodeRef;
    type ParentNode = NodeRef;
    type ChildNode = NodeRef;
    type Document = NodeRef;
    type DocumentFragment = NodeRef;
    type Element = NodeRef;
    type CommentNode = NodeRef;
    type TextNode = NodeRef;
    type Template = NodeRef;
    type DocumentType = NodeRef;

    fn adopt_attributes(&self, recipient: &Self::Element, attrs: Vec<Attribute>) {
        adopt_attributes(recipient, attrs);
    }

    fn append_child(&self, parent_node: &Self::ParentNode, new_node: Self::ChildNode) {
        append_child(parent_node, new_node);
    }

    fn create_comment_node(&self, data: &str) -> Self::CommentNode {
        create_comment_node(data)
    }

    fn create_text_node(&self, value: &str) -> Self::TextNode {
        create_text_node(value)
    }

    fn create_document(&self) -> Self::Document {
        create_document()
    }

    fn create_document_fragment(&self) -> Self::DocumentFragment {
        create_document_fragment()
    }

    fn create_element(
        &self,
        tag_name: &str,
        namespace_uri: &str,
        attrs: Vec<Attribute>,
    ) -> Self::Element {
        create_element(tag_name, namespace_uri, attrs)
    }

    fn detach_node(&self, node: &Self::ChildNode) {
        detach_node(node);
    }

    fn get_attr_list(&self, element: &Self::Element) -> Vec<Attribute> {
        get_attr_list(element)
    }

    fn get_child_nodes(&self, node: &Self::ParentNode) -> Vec<Self::ChildNode> {
        get_child_nodes(node).expect("get_child_nodes target must be a parent node")
    }

    fn get_comment_node_content(&self, comment_node: &Self::CommentNode) -> String {
        get_comment_node_content(comment_node)
    }

    fn get_document_mode(&self, document: &Self::Document) -> DocumentMode {
        get_document_mode(document)
    }

    fn get_document_type_node_name(&self, doctype_node: &Self::DocumentType) -> String {
        get_document_type_node_name(doctype_node)
    }

    fn get_document_type_node_public_id(&self, doctype_node: &Self::DocumentType) -> String {
        get_document_type_node_public_id(doctype_node)
    }

    fn get_document_type_node_system_id(&self, doctype_node: &Self::DocumentType) -> String {
        get_document_type_node_system_id(doctype_node)
    }

    fn get_first_child(&self, node: &Self::ParentNode) -> Option<Self::ChildNode> {
        get_first_child(node)
    }

    fn get_namespace_uri(&self, element: &Self::Element) -> String {
        get_namespace_uri(element)
    }

    fn get_node_source_code_location(&self, node: &Self::Node) -> Option<ElementLocation> {
        get_node_source_code_location(node)
    }

    fn get_parent_node(&self, node: &Self::Node) -> Option<Self::ParentNode> {
        get_parent_node(node)
    }

    fn get_tag_name(&self, element: &Self::Element) -> String {
        get_tag_name(element)
    }

    fn get_text_node_content(&self, text_node: &Self::TextNode) -> String {
        get_text_node_content(text_node)
    }

    fn get_template_content(
        &self,
        template_element: &Self::Template,
    ) -> Option<Self::DocumentFragment> {
        get_template_content(template_element)
    }

    fn insert_before(
        &self,
        parent_node: &Self::ParentNode,
        new_node: Self::ChildNode,
        reference_node: &Self::ChildNode,
    ) {
        insert_before(parent_node, new_node, reference_node);
    }

    fn insert_text(&self, parent_node: &Self::ParentNode, text: &str) {
        insert_text(parent_node, text);
    }

    fn insert_text_before(
        &self,
        parent_node: &Self::ParentNode,
        text: &str,
        reference_node: &Self::ChildNode,
    ) {
        insert_text_before(parent_node, text, reference_node);
    }

    fn is_comment_node(&self, node: &Self::Node) -> bool {
        node.borrow().is_comment()
    }

    fn is_document_type_node(&self, node: &Self::Node) -> bool {
        node.borrow().is_document_type()
    }

    fn is_element_node(&self, node: &Self::Node) -> bool {
        node.borrow().is_element()
    }

    fn is_text_node(&self, node: &Self::Node) -> bool {
        node.borrow().is_text()
    }

    fn set_document_mode(&self, document: &Self::Document, mode: DocumentMode) {
        set_document_mode(document, mode);
    }

    fn set_document_type(
        &self,
        document: &Self::Document,
        name: &str,
        public_id: &str,
        system_id: &str,
    ) {
        set_document_type(document, name, public_id, system_id);
    }

    fn set_node_source_code_location(&self, node: &Self::Node, location: Option<ElementLocation>) {
        set_node_source_code_location(node, location);
    }

    fn update_node_source_code_location(&self, node: &Self::Node, location: ElementLocation) {
        update_node_source_code_location(node, location);
    }

    fn set_template_content(
        &self,
        template_element: &Self::Template,
        content_element: Self::DocumentFragment,
    ) {
        set_template_content(template_element, content_element);
    }
}

#[cfg(test)]
mod tests {
    use super::{
        adopt_attributes, append_child, create_comment_node, create_document,
        create_document_fragment, create_element, create_text_node, get_attr_list, get_child_nodes,
        get_comment_node_content, get_document_mode, get_document_type_node_name,
        get_document_type_node_public_id, get_document_type_node_system_id, get_first_child,
        get_namespace_uri, get_node_source_code_location, get_parent_node, get_tag_name,
        get_template_content, get_text_node_content, insert_before, set_document_mode,
        set_document_type, set_node_source_code_location, set_template_content,
        update_node_source_code_location, DEFAULT_TREE_ADAPTER,
    };
    use crate::common::html::{DocumentMode, NS_HTML, NS_SVG};
    use crate::common::token::{Attribute, ElementLocation, Location};
    use crate::tree_adapters::interface::TreeAdapter;
    use std::collections::BTreeMap;
    use std::rc::Rc;

    #[test]
    fn append_child_sets_parent() {
        let fragment = create_document_fragment();
        let element = create_element("div", NS_HTML, Vec::new());
        append_child(&fragment, element.clone());

        assert!(element.borrow().parent().is_some());
        assert_eq!(fragment.borrow().child_nodes().unwrap().len(), 1);
    }

    #[test]
    fn adjacent_insert_text_coalesces() {
        let fragment = create_document_fragment();
        super::insert_text(&fragment, "a");
        super::insert_text(&fragment, "b");

        let child = fragment.borrow().child_nodes().unwrap()[0].clone();
        match &child.borrow().data {
            super::NodeData::Text { value } => assert_eq!(value, "ab"),
            _ => panic!("expected text node"),
        }

        append_child(&fragment, create_text_node("c"));
        assert_eq!(fragment.borrow().child_nodes().unwrap().len(), 2);
    }

    #[test]
    fn insert_before_sets_parent_and_preserves_order() {
        let fragment = create_document_fragment();
        let first = create_element("first", NS_HTML, Vec::new());
        let second = create_element("second", NS_HTML, Vec::new());
        let inserted = create_element("inserted", NS_HTML, Vec::new());
        append_child(&fragment, first.clone());
        append_child(&fragment, second.clone());
        insert_before(&fragment, inserted.clone(), &second);

        let children = get_child_nodes(&fragment).unwrap();
        assert!(Rc::ptr_eq(&children[0], &first));
        assert!(Rc::ptr_eq(&children[1], &inserted));
        assert!(Rc::ptr_eq(&children[2], &second));
        assert!(Rc::ptr_eq(
            get_parent_node(&inserted).as_ref().unwrap(),
            &fragment
        ));
    }

    #[test]
    fn template_content_is_not_parented_to_template() {
        let template = create_element("template", NS_HTML, Vec::new());
        let content = create_document_fragment();
        set_template_content(&template, content.clone());

        assert!(Rc::ptr_eq(
            get_template_content(&template).as_ref().unwrap(),
            &content
        ));
        assert!(get_parent_node(&content).is_none());
    }

    #[test]
    fn set_document_type_appends_or_updates_existing_doctype() {
        let document = create_document();
        set_document_type(&document, "html", "public", "system");

        let doctype = get_first_child(&document).unwrap();
        assert_eq!(get_document_type_node_name(&doctype), "html");
        assert_eq!(get_document_type_node_public_id(&doctype), "public");
        assert_eq!(get_document_type_node_system_id(&doctype), "system");
        assert!(Rc::ptr_eq(
            get_parent_node(&doctype).as_ref().unwrap(),
            &document
        ));

        set_document_type(&document, "svg", "", "");
        let children = get_child_nodes(&document).unwrap();
        assert_eq!(children.len(), 1);
        assert!(Rc::ptr_eq(&children[0], &doctype));
        assert_eq!(get_document_type_node_name(&doctype), "svg");
        assert_eq!(get_document_type_node_public_id(&doctype), "");
        assert_eq!(get_document_type_node_system_id(&doctype), "");
    }

    #[test]
    fn document_mode_round_trips() {
        let document = create_document();
        assert_eq!(get_document_mode(&document), DocumentMode::NoQuirks);

        set_document_mode(&document, DocumentMode::LimitedQuirks);
        assert_eq!(get_document_mode(&document), DocumentMode::LimitedQuirks);
    }

    #[test]
    fn adapter_accessors_return_node_data() {
        let element = create_element("svg", NS_SVG, vec![Attribute::new("viewBox", "0 0 10 10")]);
        let text = create_text_node("hello");
        let comment = create_comment_node("note");

        assert_eq!(get_tag_name(&element), "svg");
        assert_eq!(get_namespace_uri(&element), NS_SVG);
        assert_eq!(
            get_attr_list(&element),
            vec![Attribute::new("viewBox", "0 0 10 10")]
        );
        assert_eq!(get_text_node_content(&text), "hello");
        assert_eq!(get_comment_node_content(&comment), "note");
    }

    #[test]
    fn adopt_attributes_only_adds_missing_names() {
        let element = create_element("div", NS_HTML, vec![Attribute::new("id", "old")]);

        adopt_attributes(
            &element,
            vec![Attribute::new("id", "new"), Attribute::new("class", "x")],
        );

        assert_eq!(
            get_attr_list(&element),
            vec![Attribute::new("id", "old"), Attribute::new("class", "x")]
        );
    }

    #[test]
    fn source_code_location_can_be_set_and_updated() {
        let element = create_element("div", NS_HTML, Vec::new());
        let mut attrs = BTreeMap::new();
        attrs.insert(
            "id".to_string(),
            Location {
                start_line: 1,
                start_col: 6,
                start_offset: 5,
                end_line: 1,
                end_col: 13,
                end_offset: 12,
            },
        );
        let start = ElementLocation {
            location: Location {
                start_line: 1,
                start_col: 1,
                start_offset: 0,
                end_line: 1,
                end_col: 14,
                end_offset: 13,
            },
            attrs,
            start_tag: Some(Location {
                start_line: 1,
                start_col: 1,
                start_offset: 0,
                end_line: 1,
                end_col: 14,
                end_offset: 13,
            }),
            end_tag: None,
        };
        let end = ElementLocation {
            location: Location {
                start_line: 1,
                start_col: 1,
                start_offset: 0,
                end_line: 1,
                end_col: 20,
                end_offset: 19,
            },
            attrs: BTreeMap::new(),
            start_tag: None,
            end_tag: Some(Location {
                start_line: 1,
                start_col: 14,
                start_offset: 13,
                end_line: 1,
                end_col: 20,
                end_offset: 19,
            }),
        };

        set_node_source_code_location(&element, Some(start));
        update_node_source_code_location(&element, end);
        let location = get_node_source_code_location(&element).unwrap();

        assert_eq!(location.location.start_offset, 0);
        assert_eq!(location.location.end_offset, 19);
        assert!(location.attrs.contains_key("id"));
        assert!(location.start_tag.is_some());
        assert!(location.end_tag.is_some());
    }

    #[test]
    fn default_tree_adapter_trait_delegates_to_default_operations() {
        let adapter = DEFAULT_TREE_ADAPTER;
        let document = adapter.create_document();
        let element = adapter.create_element("div", NS_HTML, vec![Attribute::new("id", "x")]);
        let text = adapter.create_text_node("hello");

        adapter.append_child(&document, element.clone());
        adapter.append_child(&element, text.clone());
        adapter.set_document_type(&document, "html", "", "");

        assert_eq!(adapter.get_document_mode(&document), DocumentMode::NoQuirks);
        assert!(adapter.is_element_node(&element));
        assert!(adapter.is_text_node(&text));
        assert_eq!(adapter.get_tag_name(&element), "div");
        assert_eq!(adapter.get_text_node_content(&text), "hello");
        assert_eq!(
            adapter.get_attr_list(&element),
            vec![Attribute::new("id", "x")]
        );
        assert!(Rc::ptr_eq(
            adapter.get_parent_node(&text).as_ref().unwrap(),
            &element
        ));

        let first_child = adapter.get_first_child(&document).unwrap();
        assert!(Rc::ptr_eq(&first_child, &element));
        assert_eq!(adapter.get_child_nodes(&document).len(), 2);
        assert!(adapter.is_document_type_node(&adapter.get_child_nodes(&document)[1]));
    }
}
