use crate::common::html::{TagId, NS_HTML, NS_MATHML, NS_SVG};
use crate::tree_adapters::default::{get_template_content, NodeData, NodeRef};
use std::rc::Rc;

#[derive(Debug)]
pub struct OpenElementStack {
    pub items: Vec<NodeRef>,
    pub tag_ids: Vec<TagId>,
    pub current: Option<NodeRef>,
    pub stack_top: isize,
    pub tmpl_count: usize,
    pub current_tag_id: Option<TagId>,
}

impl OpenElementStack {
    pub fn new(document: NodeRef) -> Self {
        Self {
            items: Vec::new(),
            tag_ids: Vec::new(),
            current: Some(document),
            stack_top: -1,
            tmpl_count: 0,
            current_tag_id: Some(TagId::Unknown),
        }
    }

    pub fn current_tmpl_content_or_node(&self) -> Option<NodeRef> {
        if self.is_in_template() {
            self.current.as_ref().and_then(get_template_content)
        } else {
            self.current.clone()
        }
    }

    fn index_of(&self, element: &NodeRef) -> Option<usize> {
        if self.stack_top < 0 {
            return None;
        }

        (0..=self.stack_top as usize)
            .rev()
            .find(|&idx| Rc::ptr_eq(&self.items[idx], element))
    }

    fn is_in_template(&self) -> bool {
        self.current_tag_id == Some(TagId::Template)
            && self
                .current
                .as_ref()
                .is_some_and(|current| namespace_uri(current).as_deref() == Some(NS_HTML))
    }

    fn update_current_element(&mut self) {
        if self.stack_top >= 0 {
            let idx = self.stack_top as usize;
            self.current = self.items.get(idx).cloned();
            self.current_tag_id = self.tag_ids.get(idx).copied();
        } else {
            self.current = None;
            self.current_tag_id = None;
        }
    }

    pub fn push(&mut self, element: NodeRef, tag_id: TagId) {
        self.stack_top += 1;
        let idx = self.stack_top as usize;

        if idx < self.items.len() {
            self.items[idx] = element.clone();
            self.tag_ids[idx] = tag_id;
            self.items.truncate(idx + 1);
            self.tag_ids.truncate(idx + 1);
        } else {
            self.items.push(element.clone());
            self.tag_ids.push(tag_id);
        }

        self.current = Some(element);
        self.current_tag_id = Some(tag_id);

        if self.is_in_template() {
            self.tmpl_count += 1;
        }
    }

    pub fn pop(&mut self) -> Option<NodeRef> {
        let popped = self.current.clone()?;

        if self.tmpl_count > 0 && self.is_in_template() {
            self.tmpl_count -= 1;
        }

        self.stack_top -= 1;
        self.update_current_element();
        Some(popped)
    }

    pub fn replace(&mut self, old_element: &NodeRef, new_element: NodeRef) {
        let Some(idx) = self.index_of(old_element) else {
            return;
        };

        self.items[idx] = new_element.clone();

        if idx as isize == self.stack_top {
            self.current = Some(new_element);
        }
    }

    pub fn insert_after(
        &mut self,
        reference_element: &NodeRef,
        new_element: NodeRef,
        new_element_id: TagId,
    ) {
        let insertion_idx = self.index_of(reference_element).map_or(0, |idx| idx + 1);

        self.items.insert(insertion_idx, new_element);
        self.tag_ids.insert(insertion_idx, new_element_id);
        self.stack_top += 1;

        if insertion_idx as isize == self.stack_top {
            self.update_current_element();
        }
    }

    pub fn pop_until_tag_name_popped(&mut self, tag_id: TagId) {
        let mut target_idx = self.stack_top + 1;

        loop {
            let Some(idx) = self.last_index_of_tag_id_before(tag_id, target_idx) else {
                target_idx = -1;
                break;
            };

            target_idx = idx as isize;

            if target_idx <= 0
                || namespace_uri(&self.items[target_idx as usize]).as_deref() == Some(NS_HTML)
            {
                break;
            }
        }

        self.shorten_to_length(target_idx.max(0));
    }

    pub fn shorten_to_length(&mut self, idx: isize) {
        while self.stack_top >= idx {
            if self.tmpl_count > 0 && self.is_in_template() {
                self.tmpl_count -= 1;
            }

            self.stack_top -= 1;
            self.update_current_element();
        }

        let keep_len = (self.stack_top + 1).max(0) as usize;
        self.items.truncate(keep_len);
        self.tag_ids.truncate(keep_len);
    }

    pub fn pop_until_element_popped(&mut self, element: &NodeRef) {
        let idx = self.index_of(element).map_or(-1, |idx| idx as isize);
        self.shorten_to_length(idx.max(0));
    }

    fn pop_until_popped(&mut self, tag_names: impl Fn(TagId) -> bool, target_ns: &str) {
        let idx = self
            .index_of_tag_names(tag_names, target_ns)
            .map_or(-1, |idx| idx as isize);
        self.shorten_to_length(idx.max(0));
    }

    pub fn pop_until_numbered_header_popped(&mut self) {
        self.pop_until_popped(is_numbered_header_tag, NS_HTML);
    }

    pub fn pop_until_table_cell_popped(&mut self) {
        self.pop_until_popped(is_table_cell, NS_HTML);
    }

    pub fn pop_all_up_to_html_element(&mut self) {
        self.tmpl_count = 0;
        self.shorten_to_length(1);
    }

    fn index_of_tag_names(
        &self,
        tag_names: impl Fn(TagId) -> bool,
        namespace: &str,
    ) -> Option<usize> {
        if self.stack_top < 0 {
            return None;
        }

        (0..=self.stack_top as usize).rev().find(|&idx| {
            tag_names(self.tag_ids[idx])
                && namespace_uri(&self.items[idx]).as_deref() == Some(namespace)
        })
    }

    fn clear_back_to(&mut self, tag_names: impl Fn(TagId) -> bool, target_ns: &str) {
        let idx = self
            .index_of_tag_names(tag_names, target_ns)
            .map_or(-1, |idx| idx as isize);
        self.shorten_to_length(idx + 1);
    }

    pub fn clear_back_to_table_context(&mut self) {
        self.clear_back_to(is_table_context, NS_HTML);
    }

    pub fn clear_back_to_table_body_context(&mut self) {
        self.clear_back_to(is_table_body_context, NS_HTML);
    }

    pub fn clear_back_to_table_row_context(&mut self) {
        self.clear_back_to(is_table_row_context, NS_HTML);
    }

    pub fn remove(&mut self, element: &NodeRef) {
        let Some(idx) = self.index_of(element) else {
            return;
        };

        if idx as isize == self.stack_top {
            self.pop();
        } else {
            self.items.remove(idx);
            self.tag_ids.remove(idx);
            self.stack_top -= 1;
            self.update_current_element();
        }
    }

    pub fn try_peek_properly_nested_body_element(&self) -> Option<NodeRef> {
        (self.stack_top >= 1 && self.tag_ids.get(1) == Some(&TagId::Body))
            .then(|| self.items[1].clone())
    }

    pub fn contains(&self, element: &NodeRef) -> bool {
        self.index_of(element).is_some()
    }

    pub fn get_common_ancestor(&self, element: &NodeRef) -> Option<NodeRef> {
        let element_idx = self.index_of(element)?.checked_sub(1)?;
        self.items.get(element_idx).cloned()
    }

    pub fn is_root_html_element_current(&self) -> bool {
        self.stack_top == 0 && self.tag_ids.first() == Some(&TagId::Html)
    }

    fn has_in_dynamic_scope(&self, tag_id: TagId, html_scope: impl Fn(TagId) -> bool) -> bool {
        if self.stack_top < 0 {
            return true;
        }

        for idx in (0..=self.stack_top as usize).rev() {
            let tn = self.tag_ids[idx];

            match namespace_uri(&self.items[idx]).as_deref() {
                Some(NS_HTML) => {
                    if tn == tag_id {
                        return true;
                    }
                    if html_scope(tn) {
                        return false;
                    }
                }
                Some(NS_SVG) if is_svg_scoping_element(tn) => return false,
                Some(NS_MATHML) if is_mathml_scoping_element(tn) => return false,
                _ => {}
            }
        }

        true
    }

    pub fn has_in_scope(&self, tag_id: TagId) -> bool {
        self.has_in_dynamic_scope(tag_id, is_html_scoping_element)
    }

    pub fn has_in_list_item_scope(&self, tag_id: TagId) -> bool {
        self.has_in_dynamic_scope(tag_id, |id| {
            is_html_scoping_element(id) || id == TagId::Ol || id == TagId::Ul
        })
    }

    pub fn has_in_button_scope(&self, tag_id: TagId) -> bool {
        self.has_in_dynamic_scope(tag_id, |id| {
            is_html_scoping_element(id) || id == TagId::Button
        })
    }

    pub fn has_numbered_header_in_scope(&self) -> bool {
        if self.stack_top < 0 {
            return true;
        }

        for idx in (0..=self.stack_top as usize).rev() {
            let tn = self.tag_ids[idx];

            match namespace_uri(&self.items[idx]).as_deref() {
                Some(NS_HTML) => {
                    if is_numbered_header_tag(tn) {
                        return true;
                    }
                    if is_html_scoping_element(tn) {
                        return false;
                    }
                }
                Some(NS_SVG) if is_svg_scoping_element(tn) => return false,
                Some(NS_MATHML) if is_mathml_scoping_element(tn) => return false,
                _ => {}
            }
        }

        true
    }

    pub fn has_in_table_scope(&self, tag_id: TagId) -> bool {
        if self.stack_top < 0 {
            return true;
        }

        for idx in (0..=self.stack_top as usize).rev() {
            if namespace_uri(&self.items[idx]).as_deref() != Some(NS_HTML) {
                continue;
            }

            match self.tag_ids[idx] {
                id if id == tag_id => return true,
                TagId::Table | TagId::Html => return false,
                _ => {}
            }
        }

        true
    }

    pub fn has_table_body_context_in_table_scope(&self) -> bool {
        if self.stack_top < 0 {
            return true;
        }

        for idx in (0..=self.stack_top as usize).rev() {
            if namespace_uri(&self.items[idx]).as_deref() != Some(NS_HTML) {
                continue;
            }

            match self.tag_ids[idx] {
                TagId::Tbody | TagId::Thead | TagId::Tfoot => return true,
                TagId::Table | TagId::Html => return false,
                _ => {}
            }
        }

        true
    }

    pub fn has_in_select_scope(&self, tag_id: TagId) -> bool {
        if self.stack_top < 0 {
            return true;
        }

        for idx in (0..=self.stack_top as usize).rev() {
            if namespace_uri(&self.items[idx]).as_deref() != Some(NS_HTML) {
                continue;
            }

            match self.tag_ids[idx] {
                id if id == tag_id => return true,
                TagId::Option | TagId::Optgroup => {}
                _ => return false,
            }
        }

        true
    }

    pub fn generate_implied_end_tags(&mut self) {
        while self
            .current_tag_id
            .is_some_and(is_implicit_end_tag_required)
        {
            self.pop();
        }
    }

    pub fn generate_implied_end_tags_thoroughly(&mut self) {
        while self
            .current_tag_id
            .is_some_and(is_implicit_end_tag_required_thoroughly)
        {
            self.pop();
        }
    }

    pub fn generate_implied_end_tags_with_exclusion(&mut self, exclusion_id: TagId) {
        while self
            .current_tag_id
            .is_some_and(|id| id != exclusion_id && is_implicit_end_tag_required_thoroughly(id))
        {
            self.pop();
        }
    }

    pub fn len(&self) -> usize {
        (self.stack_top + 1).max(0) as usize
    }

    pub fn is_empty(&self) -> bool {
        self.stack_top < 0
    }

    fn last_index_of_tag_id_before(&self, tag_id: TagId, before: isize) -> Option<usize> {
        if before <= 0 {
            return None;
        }

        let end = before.min(self.stack_top + 1) as usize;
        (0..end).rev().find(|&idx| self.tag_ids[idx] == tag_id)
    }
}

fn namespace_uri(node: &NodeRef) -> Option<String> {
    match &node.borrow().data {
        NodeData::Element { namespace_uri, .. } => Some(namespace_uri.clone()),
        _ => None,
    }
}

fn is_implicit_end_tag_required(tag_id: TagId) -> bool {
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

fn is_implicit_end_tag_required_thoroughly(tag_id: TagId) -> bool {
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

fn is_html_scoping_element(tag_id: TagId) -> bool {
    matches!(
        tag_id,
        TagId::Applet
            | TagId::Caption
            | TagId::Html
            | TagId::Marquee
            | TagId::Object
            | TagId::Table
            | TagId::Td
            | TagId::Template
            | TagId::Th
    )
}

fn is_mathml_scoping_element(tag_id: TagId) -> bool {
    matches!(
        tag_id,
        TagId::AnnotationXml | TagId::Mi | TagId::Mn | TagId::Mo | TagId::Ms | TagId::Mtext
    )
}

fn is_svg_scoping_element(tag_id: TagId) -> bool {
    matches!(tag_id, TagId::Desc | TagId::ForeignObject | TagId::Title)
}

fn is_table_row_context(tag_id: TagId) -> bool {
    matches!(tag_id, TagId::Tr | TagId::Template | TagId::Html)
}

fn is_table_body_context(tag_id: TagId) -> bool {
    matches!(
        tag_id,
        TagId::Tbody | TagId::Tfoot | TagId::Thead | TagId::Template | TagId::Html
    )
}

fn is_table_context(tag_id: TagId) -> bool {
    matches!(tag_id, TagId::Table | TagId::Template | TagId::Html)
}

fn is_table_cell(tag_id: TagId) -> bool {
    matches!(tag_id, TagId::Td | TagId::Th)
}

fn is_numbered_header_tag(tag_id: TagId) -> bool {
    matches!(
        tag_id,
        TagId::H1 | TagId::H2 | TagId::H3 | TagId::H4 | TagId::H5 | TagId::H6
    )
}

#[cfg(test)]
mod tests {
    use super::OpenElementStack;
    use crate::common::html::{TagId, NS_HTML, NS_MATHML, NS_SVG, NS_XLINK, NS_XML};
    use crate::tree_adapters::default::{create_document, create_element};
    use std::rc::Rc;

    fn element(tag_name: &str) -> crate::tree_adapters::default::NodeRef {
        create_element(tag_name, NS_HTML, Vec::new())
    }

    fn ns_element(tag_name: &str, namespace: &str) -> crate::tree_adapters::default::NodeRef {
        create_element(tag_name, namespace, Vec::new())
    }

    fn assert_current(stack: &OpenElementStack, node: &crate::tree_adapters::default::NodeRef) {
        assert!(Rc::ptr_eq(stack.current.as_ref().unwrap(), node));
    }

    #[test]
    fn starts_with_document_current_and_empty_stack() {
        let document = create_document();
        let stack = OpenElementStack::new(document.clone());

        assert_current(&stack, &document);
        assert_eq!(stack.stack_top, -1);
        assert_eq!(stack.current_tag_id, Some(TagId::Unknown));
    }

    #[test]
    fn pushes_and_pops_elements() {
        let element1 = ns_element("#element1", NS_XLINK);
        let element2 = ns_element("#element2", NS_SVG);
        let mut stack = OpenElementStack::new(create_document());

        stack.push(element1.clone(), TagId::Unknown);
        assert_current(&stack, &element1);
        assert_eq!(stack.stack_top, 0);

        stack.push(element2.clone(), TagId::Unknown);
        assert_current(&stack, &element2);
        assert_eq!(stack.stack_top, 1);

        stack.pop();
        assert_current(&stack, &element1);
        assert_eq!(stack.stack_top, 0);

        stack.pop();
        assert!(stack.current.is_none());
        assert_eq!(stack.current_tag_id, None);
        assert_eq!(stack.stack_top, -1);
    }

    #[test]
    fn replaces_and_inserts_elements() {
        let element1 = ns_element("#element1", NS_XLINK);
        let element2 = ns_element("#element2", NS_SVG);
        let element3 = ns_element("#element3", NS_XML);
        let replacement = ns_element("#replacement", NS_MATHML);
        let mut stack = OpenElementStack::new(create_document());

        stack.push(element1.clone(), TagId::Unknown);
        stack.push(element2.clone(), TagId::Unknown);
        stack.replace(&element2, replacement.clone());
        assert_current(&stack, &replacement);

        stack.insert_after(&element1, element3.clone(), TagId::Unknown);
        assert_eq!(stack.stack_top, 2);
        assert!(Rc::ptr_eq(&stack.items[1], &element3));
    }

    #[test]
    fn pops_until_tag_name_or_element_is_popped() {
        let aside = element("aside");
        let main = element("main");
        let mut stack = OpenElementStack::new(create_document());

        for _ in 0..4 {
            stack.push(main.clone(), TagId::Main);
        }
        stack.pop_until_tag_name_popped(TagId::Aside);
        assert!(stack.current.is_none());
        assert_eq!(stack.stack_top, -1);

        stack.push(main.clone(), TagId::Main);
        stack.push(aside.clone(), TagId::Aside);
        stack.push(main.clone(), TagId::Main);
        stack.pop_until_tag_name_popped(TagId::Aside);
        assert_current(&stack, &main);
        assert_eq!(stack.stack_top, 0);

        stack.push(aside.clone(), TagId::Aside);
        stack.push(main.clone(), TagId::Main);
        stack.pop_until_element_popped(&aside);
        assert_current(&stack, &main);
        assert_eq!(stack.stack_top, 0);
    }

    #[test]
    fn clears_back_to_table_contexts() {
        let html = element("html");
        let table = element("table");
        let thead = element("thead");
        let tr = element("tr");
        let div = element("div");
        let mut stack = OpenElementStack::new(create_document());

        stack.push(html.clone(), TagId::Html);
        stack.push(div.clone(), TagId::Div);
        stack.push(div.clone(), TagId::Div);
        stack.clear_back_to_table_context();
        assert_current(&stack, &html);
        assert_eq!(stack.stack_top, 0);

        stack.push(div.clone(), TagId::Div);
        stack.push(table.clone(), TagId::Table);
        stack.push(div.clone(), TagId::Div);
        stack.clear_back_to_table_context();
        assert_current(&stack, &table);
        assert_eq!(stack.stack_top, 2);

        stack.push(div.clone(), TagId::Div);
        stack.push(thead.clone(), TagId::Thead);
        stack.push(div.clone(), TagId::Div);
        stack.clear_back_to_table_body_context();
        assert_current(&stack, &thead);
        assert_eq!(stack.stack_top, 4);

        stack.push(div.clone(), TagId::Div);
        stack.push(tr.clone(), TagId::Tr);
        stack.push(div, TagId::Div);
        stack.clear_back_to_table_row_context();
        assert_current(&stack, &tr);
        assert_eq!(stack.stack_top, 6);
    }

    #[test]
    fn removes_contains_and_finds_ancestors() {
        let target = element("#element");
        let ancestor = element("#ancestor");
        let other = element("#other");
        let mut stack = OpenElementStack::new(create_document());

        assert!(!stack.contains(&target));
        assert!(stack.get_common_ancestor(&target).is_none());

        stack.push(ancestor.clone(), TagId::Unknown);
        stack.push(other.clone(), TagId::Unknown);
        stack.push(target.clone(), TagId::Unknown);

        assert!(stack.contains(&target));
        assert!(Rc::ptr_eq(
            stack.get_common_ancestor(&target).as_ref().unwrap(),
            &other
        ));

        stack.remove(&other);
        assert_eq!(stack.stack_top, 1);
        assert!(!stack.items.iter().any(|node| Rc::ptr_eq(node, &other)));
    }

    #[test]
    fn detects_body_and_root_html_positions() {
        let html = element("html");
        let body = element("body");
        let div = element("div");
        let mut stack = OpenElementStack::new(create_document());

        stack.push(html.clone(), TagId::Html);
        assert!(stack.is_root_html_element_current());

        stack.push(body.clone(), TagId::Body);
        stack.push(div, TagId::Div);

        assert!(!stack.is_root_html_element_current());
        assert!(Rc::ptr_eq(
            stack
                .try_peek_properly_nested_body_element()
                .as_ref()
                .unwrap(),
            &body
        ));
    }

    #[test]
    fn checks_dynamic_scopes() {
        let mut stack = OpenElementStack::new(create_document());

        assert!(stack.has_in_list_item_scope(TagId::P));
        assert!(stack.has_in_button_scope(TagId::P));
        assert!(stack.has_in_select_scope(TagId::P));

        stack.push(element("html"), TagId::Html);
        stack.push(element("div"), TagId::Div);
        assert!(!stack.has_in_scope(TagId::P));
        assert!(!stack.has_in_list_item_scope(TagId::P));
        assert!(!stack.has_in_button_scope(TagId::P));
        assert!(!stack.has_in_table_scope(TagId::P));
        assert!(!stack.has_table_body_context_in_table_scope());
        assert!(!stack.has_in_select_scope(TagId::P));

        stack.push(element("p"), TagId::P);
        stack.push(element("ul"), TagId::Ul);
        stack.push(element("button"), TagId::Button);
        stack.push(element("option"), TagId::Option);
        assert!(stack.has_in_scope(TagId::P));
        assert!(stack.has_in_table_scope(TagId::P));

        stack.push(ns_element("title", NS_SVG), TagId::Title);
        assert!(!stack.has_in_scope(TagId::P));
        assert!(!stack.has_numbered_header_in_scope());

        let mut select_stack = OpenElementStack::new(create_document());
        select_stack.push(element("html"), TagId::Html);
        select_stack.push(element("div"), TagId::Div);
        assert!(!select_stack.has_in_select_scope(TagId::P));

        select_stack.push(element("p"), TagId::P);
        select_stack.push(element("option"), TagId::Option);
        assert!(select_stack.has_in_select_scope(TagId::P));

        select_stack.push(element("div"), TagId::Div);
        assert!(!select_stack.has_in_select_scope(TagId::P));
    }

    #[test]
    fn checks_numbered_headers_and_table_body_scope() {
        let mut stack = OpenElementStack::new(create_document());

        assert!(stack.has_numbered_header_in_scope());

        stack.push(element("html"), TagId::Html);
        stack.push(element("div"), TagId::Div);
        assert!(!stack.has_numbered_header_in_scope());

        stack.push(element("h3"), TagId::H3);
        assert!(stack.has_numbered_header_in_scope());

        stack.push(element("table"), TagId::Table);
        assert!(!stack.has_table_body_context_in_table_scope());

        stack.push(element("tbody"), TagId::Tbody);
        assert!(stack.has_table_body_context_in_table_scope());
    }

    #[test]
    fn generates_implied_end_tags() {
        let mut stack = OpenElementStack::new(create_document());

        stack.push(element("html"), TagId::Html);
        stack.push(element("li"), TagId::Li);
        stack.push(element("div"), TagId::Div);
        stack.push(element("li"), TagId::Li);
        stack.push(element("option"), TagId::Option);
        stack.push(element("p"), TagId::P);
        stack.generate_implied_end_tags();

        assert_eq!(stack.stack_top, 2);
        assert_eq!(stack.current_tag_id, Some(TagId::Div));

        stack.push(element("li"), TagId::Li);
        stack.push(element("option"), TagId::Option);
        stack.push(element("p"), TagId::P);
        stack.generate_implied_end_tags_with_exclusion(TagId::Li);

        assert_eq!(stack.stack_top, 3);
        assert_eq!(stack.current_tag_id, Some(TagId::Li));
    }

    #[test]
    fn tracks_html_template_count_only() {
        let mut stack = OpenElementStack::new(create_document());

        stack.push(element("html"), TagId::Html);
        stack.push(ns_element("template", NS_MATHML), TagId::Template);
        assert_eq!(stack.tmpl_count, 0);

        stack.push(element("template"), TagId::Template);
        stack.push(element("li"), TagId::Li);
        assert_eq!(stack.tmpl_count, 1);

        stack.push(element("option"), TagId::Option);
        stack.push(element("template"), TagId::Template);
        assert_eq!(stack.tmpl_count, 2);

        stack.pop();
        assert_eq!(stack.tmpl_count, 1);

        stack.pop();
        stack.pop();
        stack.pop();
        assert_eq!(stack.tmpl_count, 0);
    }
}
