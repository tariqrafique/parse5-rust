use crate::common::html::DocumentMode;
use crate::common::token::{Attribute, ElementLocation};

pub trait TreeAdapter {
    type Node;
    type ParentNode;
    type ChildNode;
    type Document;
    type DocumentFragment;
    type Element;
    type CommentNode;
    type TextNode;
    type Template;
    type DocumentType;

    fn adopt_attributes(&self, recipient: &Self::Element, attrs: Vec<Attribute>);
    fn append_child(&self, parent_node: &Self::ParentNode, new_node: Self::ChildNode);
    fn create_comment_node(&self, data: &str) -> Self::CommentNode;
    fn create_text_node(&self, value: &str) -> Self::TextNode;
    fn create_document(&self) -> Self::Document;
    fn create_document_fragment(&self) -> Self::DocumentFragment;
    fn create_element(
        &self,
        tag_name: &str,
        namespace_uri: &str,
        attrs: Vec<Attribute>,
    ) -> Self::Element;
    fn detach_node(&self, node: &Self::ChildNode);
    fn get_attr_list(&self, element: &Self::Element) -> Vec<Attribute>;
    fn get_child_nodes(&self, node: &Self::ParentNode) -> Vec<Self::ChildNode>;
    fn get_comment_node_content(&self, comment_node: &Self::CommentNode) -> String;
    fn get_document_mode(&self, document: &Self::Document) -> DocumentMode;
    fn get_document_type_node_name(&self, doctype_node: &Self::DocumentType) -> String;
    fn get_document_type_node_public_id(&self, doctype_node: &Self::DocumentType) -> String;
    fn get_document_type_node_system_id(&self, doctype_node: &Self::DocumentType) -> String;
    fn get_first_child(&self, node: &Self::ParentNode) -> Option<Self::ChildNode>;
    fn get_namespace_uri(&self, element: &Self::Element) -> String;
    fn get_node_source_code_location(&self, node: &Self::Node) -> Option<ElementLocation>;
    fn get_parent_node(&self, node: &Self::Node) -> Option<Self::ParentNode>;
    fn get_tag_name(&self, element: &Self::Element) -> String;
    fn get_text_node_content(&self, text_node: &Self::TextNode) -> String;
    fn get_template_content(
        &self,
        template_element: &Self::Template,
    ) -> Option<Self::DocumentFragment>;
    fn insert_before(
        &self,
        parent_node: &Self::ParentNode,
        new_node: Self::ChildNode,
        reference_node: &Self::ChildNode,
    );
    fn insert_text(&self, parent_node: &Self::ParentNode, text: &str);
    fn insert_text_before(
        &self,
        parent_node: &Self::ParentNode,
        text: &str,
        reference_node: &Self::ChildNode,
    );
    fn is_comment_node(&self, node: &Self::Node) -> bool;
    fn is_document_type_node(&self, node: &Self::Node) -> bool;
    fn is_element_node(&self, node: &Self::Node) -> bool;
    fn is_text_node(&self, node: &Self::Node) -> bool;
    fn set_document_mode(&self, document: &Self::Document, mode: DocumentMode);
    fn set_document_type(
        &self,
        document: &Self::Document,
        name: &str,
        public_id: &str,
        system_id: &str,
    );
    fn set_node_source_code_location(&self, node: &Self::Node, location: Option<ElementLocation>);
    fn update_node_source_code_location(&self, node: &Self::Node, location: ElementLocation);
    fn set_template_content(
        &self,
        template_element: &Self::Template,
        content_element: Self::DocumentFragment,
    );
    fn on_item_push(&self, _item: &Self::Element) {}
    fn on_item_pop(&self, _item: &Self::Element, _new_top: Option<&Self::ParentNode>) {}
}
