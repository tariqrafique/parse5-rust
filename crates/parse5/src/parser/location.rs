use crate::common::html::TagId;
use crate::common::token::{ElementLocation, Location, LocationWithAttributes, Token};
use crate::tree_adapters::default::{append_child, create_text_node, NodeData, NodeRef};

// Included into parser/mod.rs. Pure move.
pub(in crate::parser) fn element_location_from_start_tag(
    start_tag: LocationWithAttributes,
) -> ElementLocation {
    ElementLocation {
        location: start_tag.location,
        attrs: start_tag.attrs,
        start_tag: Some(start_tag.location),
        end_tag: None,
    }
}

pub(in crate::parser) fn append_text(parent: &NodeRef, chars: &str, location: Option<Location>) {
    let last_child = parent
        .borrow()
        .child_nodes()
        .and_then(|children| children.last().cloned());

    if let Some(last_child) = last_child {
        let mut last_ref = last_child.borrow_mut();

        if let NodeData::Text { value } = &mut last_ref.data {
            value.push_str(chars);

            if let Some(location) = location {
                let mut source_location = last_ref
                    .source_code_location()
                    .unwrap_or_else(|| ElementLocation::from(location));
                extend_location_end(&mut source_location.location, location);
                last_ref.set_source_code_location(Some(source_location));
            }

            return;
        }
    }

    let text = create_text_node(chars);
    if let Some(location) = location {
        text.borrow_mut()
            .set_source_code_location(Some(ElementLocation::from(location)));
    }
    append_child(parent, text);
}

pub(in crate::parser) fn append_text_before(
    parent: &NodeRef,
    chars: &str,
    location: Option<Location>,
    reference_node: &NodeRef,
) {
    let maybe_prev = {
        let parent_ref = parent.borrow();
        parent_ref.child_nodes().and_then(|children| {
            children
                .iter()
                .position(|node| std::rc::Rc::ptr_eq(node, reference_node))
                .and_then(|idx| idx.checked_sub(1))
                .and_then(|idx| children.get(idx).cloned())
        })
    };

    if let Some(prev) = maybe_prev {
        let mut prev_ref = prev.borrow_mut();

        if let NodeData::Text { value } = &mut prev_ref.data {
            value.push_str(chars);

            if let Some(location) = location {
                let mut source_location = prev_ref
                    .source_code_location()
                    .unwrap_or_else(|| ElementLocation::from(location));
                extend_location_end(&mut source_location.location, location);
                prev_ref.set_source_code_location(Some(source_location));
            }

            return;
        }
    }

    let text = create_text_node(chars);
    if let Some(location) = location {
        text.borrow_mut()
            .set_source_code_location(Some(ElementLocation::from(location)));
    }
    crate::tree_adapters::default::insert_before(parent, text, reference_node);
}

pub(in crate::parser) fn update_element_end_location(element: &NodeRef, end_tag: Location) {
    let mut element_ref = element.borrow_mut();
    let mut location = element_ref
        .source_code_location()
        .unwrap_or_else(|| ElementLocation::from(end_tag));

    extend_location_end(&mut location.location, end_tag);
    location.end_tag = Some(end_tag);
    element_ref.set_source_code_location(Some(location));
}

pub(in crate::parser) fn update_element_implicit_end_location(
    element: &NodeRef,
    next_token_location: Location,
) {
    let mut element_ref = element.borrow_mut();
    let mut location = element_ref
        .source_code_location()
        .unwrap_or_else(|| ElementLocation::from(next_token_location));

    location.location.end_line = next_token_location.start_line;
    location.location.end_col = next_token_location.start_col;
    location.location.end_offset = next_token_location.start_offset;
    element_ref.set_source_code_location(Some(location));
}

pub(in crate::parser) fn extend_location_end(location: &mut Location, end: Location) {
    location.end_line = end.end_line;
    location.end_col = end.end_col;
    location.end_offset = end.end_offset;
}

pub(in crate::parser) fn token_start_location(token: &Token) -> Option<Location> {
    token_location(token).map(|location| Location {
        start_line: location.start_line,
        start_col: location.start_col,
        start_offset: location.start_offset,
        end_line: location.start_line,
        end_col: location.start_col,
        end_offset: location.start_offset,
    })
}

pub(in crate::parser) fn token_location(token: &Token) -> Option<Location> {
    match token {
        Token::Doctype(token) => token.location,
        Token::Tag(token) => token.location.as_ref().map(|location| location.location),
        Token::Comment(token) => token.location,
        Token::Eof(token) => token.location,
        Token::Character(token) => token.location,
    }
}

pub(in crate::parser) fn element_tag_name(element: &NodeRef) -> Option<String> {
    match &element.borrow().data {
        NodeData::Element { tag_name, .. } => Some(tag_name.clone()),
        _ => None,
    }
}

pub(in crate::parser) fn namespace_uri(element: &NodeRef) -> Option<String> {
    match &element.borrow().data {
        NodeData::Element { namespace_uri, .. } => Some(namespace_uri.clone()),
        _ => None,
    }
}

pub(in crate::parser) fn is_table_structure_tag(tag_id: TagId) -> bool {
    matches!(
        tag_id,
        TagId::Table | TagId::Tbody | TagId::Tfoot | TagId::Thead | TagId::Tr
    )
}

pub(in crate::parser) fn is_numbered_header_tag_id(tag_id: TagId) -> bool {
    matches!(
        tag_id,
        TagId::H1 | TagId::H2 | TagId::H3 | TagId::H4 | TagId::H5 | TagId::H6
    )
}

pub(in crate::parser) fn is_table_cell_tag_id(tag_id: TagId) -> bool {
    matches!(tag_id, TagId::Td | TagId::Th)
}

pub(in crate::parser) fn is_table_context_tag_id(tag_id: TagId) -> bool {
    matches!(tag_id, TagId::Table | TagId::Template | TagId::Html)
}

pub(in crate::parser) fn is_table_body_context_tag_id(tag_id: TagId) -> bool {
    matches!(
        tag_id,
        TagId::Tbody | TagId::Tfoot | TagId::Thead | TagId::Template | TagId::Html
    )
}

pub(in crate::parser) fn is_table_row_context_tag_id(tag_id: TagId) -> bool {
    matches!(tag_id, TagId::Tr | TagId::Template | TagId::Html)
}

pub(in crate::parser) fn is_implicit_end_tag_required(tag_id: TagId) -> bool {
    matches!(
        tag_id,
        TagId::Dd
            | TagId::Dt
            | TagId::Li
            | TagId::Optgroup
            | TagId::Option
            | TagId::P
            | TagId::Rb
            | TagId::Rp
            | TagId::Rt
            | TagId::Rtc
    )
}

pub(in crate::parser) fn is_implicit_end_tag_required_thoroughly(tag_id: TagId) -> bool {
    is_implicit_end_tag_required(tag_id)
        || matches!(
            tag_id,
            TagId::Caption
                | TagId::Colgroup
                | TagId::Tbody
                | TagId::Td
                | TagId::Tfoot
                | TagId::Th
                | TagId::Thead
                | TagId::Tr
        )
}

pub(in crate::parser) fn is_table_void_element(tag_id: TagId) -> bool {
    matches!(
        tag_id,
        TagId::Caption
            | TagId::Col
            | TagId::Colgroup
            | TagId::Tbody
            | TagId::Td
            | TagId::Tfoot
            | TagId::Th
            | TagId::Thead
            | TagId::Tr
    )
}

pub(in crate::parser) fn is_table_select_boundary_tag(tag_id: TagId) -> bool {
    matches!(
        tag_id,
        TagId::Caption
            | TagId::Table
            | TagId::Tbody
            | TagId::Tfoot
            | TagId::Thead
            | TagId::Tr
            | TagId::Td
            | TagId::Th
    )
}
