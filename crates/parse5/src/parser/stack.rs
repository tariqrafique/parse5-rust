use super::Parser;
use super::*;

impl Parser {
    // Included into parser/mod.rs. Pure move.
    pub(in crate::parser) fn pop_current_element_for_token(
        &mut self,
        end_location: Option<Location>,
    ) {
        if let (Some(element), Some(end_location)) =
            (self.open_elements.current.clone(), end_location)
        {
            if element.borrow().source_code_location().is_some() {
                update_element_end_location(&element, end_location);
            }
        }

        if let Some(popped) = self.open_elements.pop() {
            self.record_stack_pop(popped);
        }
    }

    pub(in crate::parser) fn pop_open_element(&mut self) -> Option<NodeRef> {
        let popped = self.open_elements.pop();

        if let Some(element) = &popped {
            self.set_end_location_for_popped_element(element);
        }

        if let Some(element) = &popped {
            self.record_stack_pop(element.clone());
        }

        popped
    }

    pub(in crate::parser) fn shorten_open_elements_to_length(&mut self, idx: isize) {
        while self.open_elements.stack_top >= idx {
            self.pop_open_element();
        }
    }

    pub(in crate::parser) fn generate_implied_end_tags(&mut self) {
        while self
            .open_elements
            .current_tag_id
            .is_some_and(is_implicit_end_tag_required)
        {
            self.pop_open_element();
        }
    }

    pub(in crate::parser) fn generate_implied_end_tags_thoroughly(&mut self) {
        while self
            .open_elements
            .current_tag_id
            .is_some_and(is_implicit_end_tag_required_thoroughly)
        {
            self.pop_open_element();
        }
    }

    pub(in crate::parser) fn generate_implied_end_tags_with_exclusion(
        &mut self,
        exclusion_id: TagId,
    ) {
        while self
            .open_elements
            .current_tag_id
            .is_some_and(|id| id != exclusion_id && is_implicit_end_tag_required_thoroughly(id))
        {
            self.pop_open_element();
        }
    }

    pub(in crate::parser) fn pop_open_elements_until_tag_name_popped(&mut self, tag_id: TagId) {
        let mut target_idx = self.open_elements.stack_top + 1;

        loop {
            let Some(idx) = (0..target_idx.max(0) as usize)
                .rev()
                .find(|&idx| self.open_elements.tag_ids[idx] == tag_id)
            else {
                target_idx = -1;
                break;
            };

            target_idx = idx as isize;

            if target_idx <= 0
                || is_in_namespace(&self.open_elements.items[target_idx as usize], NS_HTML)
            {
                break;
            }
        }

        self.shorten_open_elements_to_length(target_idx.max(0));
    }

    pub(in crate::parser) fn pop_open_elements_until_numbered_header_popped(&mut self) {
        self.pop_open_elements_until_popped(is_numbered_header_tag_id);
    }

    pub(in crate::parser) fn pop_open_elements_until_table_cell_popped(&mut self) {
        self.pop_open_elements_until_popped(is_table_cell_tag_id);
    }

    pub(in crate::parser) fn pop_open_elements_until_popped(
        &mut self,
        tag_names: impl Fn(TagId) -> bool,
    ) {
        let target_idx = self
            .index_of_open_element_tag_names(tag_names)
            .unwrap_or(-1);
        self.shorten_open_elements_to_length(target_idx.max(0));
    }

    pub(in crate::parser) fn clear_open_elements_back_to_table_context(&mut self) {
        self.clear_open_elements_back_to(is_table_context_tag_id);
    }

    pub(in crate::parser) fn clear_open_elements_back_to_table_body_context(&mut self) {
        self.clear_open_elements_back_to(is_table_body_context_tag_id);
    }

    pub(in crate::parser) fn clear_open_elements_back_to_table_row_context(&mut self) {
        self.clear_open_elements_back_to(is_table_row_context_tag_id);
    }

    pub(in crate::parser) fn clear_open_elements_back_to(
        &mut self,
        tag_names: impl Fn(TagId) -> bool,
    ) {
        let target_idx = self
            .index_of_open_element_tag_names(tag_names)
            .unwrap_or(-1);
        self.shorten_open_elements_to_length(target_idx + 1);
    }

    pub(in crate::parser) fn pop_open_elements_all_up_to_html_element(&mut self) {
        self.open_elements.tmpl_count = 0;
        self.shorten_open_elements_to_length(1);
    }

    pub(in crate::parser) fn push_open_element(&mut self, element: NodeRef, tag_id: TagId) {
        self.open_elements.push(element.clone(), tag_id);
        self.stack_events.push(StackEvent {
            kind: StackEventKind::Push,
            node: element,
            new_top: self.open_elements.current.clone(),
        });
    }

    pub(in crate::parser) fn record_stack_pop(&mut self, node: NodeRef) {
        self.stack_events.push(StackEvent {
            kind: StackEventKind::Pop,
            node,
            new_top: self.open_elements.current.clone(),
        });
    }

    pub(in crate::parser) fn remove_open_element(&mut self, element: &NodeRef) {
        self.set_end_location_for_popped_element(element);
        let was_present = self.open_elements.contains(element);
        self.open_elements.remove(element);

        if was_present {
            self.record_stack_pop(element.clone());
        }
    }

    pub(in crate::parser) fn index_of_open_element_tag_names(
        &self,
        tag_names: impl Fn(TagId) -> bool,
    ) -> Option<isize> {
        if self.open_elements.stack_top < 0 {
            return None;
        }

        (0..=self.open_elements.stack_top as usize)
            .rev()
            .find(|&idx| {
                tag_names(self.open_elements.tag_ids[idx])
                    && is_in_namespace(&self.open_elements.items[idx], NS_HTML)
            })
            .map(|idx| idx as isize)
    }

    pub(in crate::parser) fn set_end_location_for_popped_element(&mut self, element: &NodeRef) {
        if !self.options.source_code_location_info {
            return;
        }

        let Some(current_token) = self.current_token.take() else {
            return;
        };

        self.set_end_location_for_element_with_token(element, &current_token);
        self.current_token = Some(current_token);
    }

    pub(in crate::parser) fn set_end_location_for_element_with_token(
        &mut self,
        element: &NodeRef,
        token: &Token,
    ) {
        if !self.options.source_code_location_info
            || element.borrow().source_code_location().is_none()
        {
            return;
        }

        let Some(closing_location) = token_location(token) else {
            return;
        };

        let is_matching_end_tag = match token {
            Token::Tag(token) if token.token_type == TokenType::EndTag => {
                element_tag_name(element).as_deref() == Some(token.tag_name.as_str())
            }
            _ => false,
        };

        if is_matching_end_tag {
            update_element_end_location(element, closing_location);
        } else {
            update_element_implicit_end_location(element, closing_location);
        }
    }

    pub(in crate::parser) fn update_last_element_end_location(
        &mut self,
        tag_id: TagId,
        end_tag_location: Option<LocationWithAttributes>,
    ) {
        let Some(end_tag_location) = end_tag_location else {
            return;
        };

        for idx in (0..=self.open_elements.stack_top.max(0) as usize).rev() {
            if self.open_elements.tag_ids[idx] == tag_id {
                update_element_end_location(
                    &self.open_elements.items[idx],
                    end_tag_location.location,
                );
                break;
            }
        }
    }
}
