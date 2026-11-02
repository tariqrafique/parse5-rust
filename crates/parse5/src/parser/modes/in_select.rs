use super::Parser;
use super::*;

impl Parser {
    // Included into parser/mod.rs. Pure move.
    pub(in crate::parser) fn start_tag_in_select(&mut self, token: TagToken) {
        match token.tag_id {
            TagId::Html => self.start_tag_in_body(token),
            TagId::Option => {
                if self.open_elements.current_tag_id == Some(TagId::Option) {
                    self.pop_open_element();
                }

                self.insert_element(token, NS_HTML);
            }
            TagId::Optgroup => {
                if self.open_elements.current_tag_id == Some(TagId::Option) {
                    self.pop_open_element();
                }

                if self.open_elements.current_tag_id == Some(TagId::Optgroup) {
                    self.pop_open_element();
                }

                self.insert_element(token, NS_HTML);
            }
            TagId::Hr => {
                if self.open_elements.current_tag_id == Some(TagId::Option) {
                    self.pop_open_element();
                }

                if self.open_elements.current_tag_id == Some(TagId::Optgroup) {
                    self.pop_open_element();
                }

                let mut token = token;
                token.ack_self_closing = true;
                self.append_element(token, NS_HTML);
            }
            TagId::Input | TagId::Keygen | TagId::Textarea | TagId::Select => {
                if self.open_elements.has_in_select_scope(TagId::Select) {
                    self.pop_open_elements_until_tag_name_popped(TagId::Select);
                    self.reset_insertion_mode();

                    if token.tag_id != TagId::Select {
                        self.process_start_tag(token, false);
                    }
                }
            }
            TagId::Script | TagId::Template => self.start_tag_in_head(token),
            _ => {}
        }
    }

    pub(in crate::parser) fn end_tag_in_select(&mut self, token: TagToken) {
        match token.tag_id {
            TagId::Optgroup => {
                if self.open_elements.stack_top > 0
                    && self.open_elements.current_tag_id == Some(TagId::Option)
                    && self
                        .open_elements
                        .tag_ids
                        .get(self.open_elements.stack_top as usize - 1)
                        == Some(&TagId::Optgroup)
                {
                    self.pop_open_element();
                }

                if self.open_elements.current_tag_id == Some(TagId::Optgroup) {
                    self.pop_current_element_for_token(
                        token.location.map(|location| location.location),
                    );
                }
            }
            TagId::Option => {
                if self.open_elements.current_tag_id == Some(TagId::Option) {
                    self.pop_current_element_for_token(
                        token.location.map(|location| location.location),
                    );
                }
            }
            TagId::Select => {
                if self.open_elements.has_in_select_scope(TagId::Select) {
                    self.update_last_element_end_location(TagId::Select, token.location);
                    self.pop_open_elements_until_tag_name_popped(TagId::Select);
                    self.reset_insertion_mode();
                }
            }
            TagId::Template => self.template_end_tag_in_head(token),
            _ => {}
        }
    }

    pub(in crate::parser) fn start_tag_in_select_in_table(&mut self, token: TagToken) {
        if is_table_select_boundary_tag(token.tag_id) {
            self.pop_open_elements_until_tag_name_popped(TagId::Select);
            self.reset_insertion_mode();
            self.process_start_tag(token, false);
        } else {
            self.start_tag_in_select(token);
        }
    }

    pub(in crate::parser) fn end_tag_in_select_in_table(&mut self, token: TagToken) {
        if is_table_select_boundary_tag(token.tag_id) {
            if self.open_elements.has_in_table_scope(token.tag_id) {
                self.pop_open_elements_until_tag_name_popped(TagId::Select);
                self.reset_insertion_mode();
                self.process_end_tag(token);
            }
        } else {
            self.end_tag_in_select(token);
        }
    }
}
