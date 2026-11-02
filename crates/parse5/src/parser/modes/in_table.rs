use super::Parser;
use super::*;

impl Parser {
    // Included into parser/mod.rs. Pure move.
    pub(in crate::parser) fn character_in_table(&mut self, token: CharacterToken) {
        if self
            .open_elements
            .current_tag_id
            .is_some_and(is_table_structure_tag)
        {
            self.pending_character_tokens.clear();
            self.has_non_whitespace_pending_character_token = false;
            self.original_insertion_mode = self.insertion_mode;
            self.insertion_mode = InsertionMode::InTableText;

            if token.token_type == TokenType::WhitespaceCharacter {
                self.whitespace_character_in_table_text(token);
            } else if token.token_type != TokenType::NullCharacter {
                self.character_in_table_text(token);
            }
        } else {
            self.token_in_table(Token::Character(token));
        }
    }

    pub(in crate::parser) fn whitespace_character_in_table_text(&mut self, token: CharacterToken) {
        self.pending_character_tokens.push(token);
    }

    pub(in crate::parser) fn character_in_table_text(&mut self, token: CharacterToken) {
        self.pending_character_tokens.push(token);
        self.has_non_whitespace_pending_character_token = true;
    }

    pub(in crate::parser) fn token_in_table_text(&mut self, token: Token) {
        let pending = std::mem::take(&mut self.pending_character_tokens);

        if self.has_non_whitespace_pending_character_token {
            for pending_token in pending {
                self.token_in_table(Token::Character(pending_token));
            }
        } else {
            for pending_token in pending {
                self.insert_characters(&pending_token.chars, pending_token.location);
            }
        }

        self.has_non_whitespace_pending_character_token = false;
        self.insertion_mode = self.original_insertion_mode;
        self.process_token(token);
    }

    pub(in crate::parser) fn caption_start_tag_in_table(&mut self, token: TagToken) {
        self.clear_open_elements_back_to_table_context();
        self.active_formatting_elements.insert_marker();
        self.insert_element(token, NS_HTML);
        self.insertion_mode = InsertionMode::InCaption;
    }

    pub(in crate::parser) fn colgroup_start_tag_in_table(&mut self, token: TagToken) {
        self.clear_open_elements_back_to_table_context();
        self.insert_element(token, NS_HTML);
        self.insertion_mode = InsertionMode::InColumnGroup;
    }

    pub(in crate::parser) fn col_start_tag_in_table(&mut self, token: TagToken) {
        self.clear_open_elements_back_to_table_context();
        self.insert_fake_element("colgroup", TagId::Colgroup);
        self.insertion_mode = InsertionMode::InColumnGroup;
        self.start_tag_in_column_group(token);
    }

    pub(in crate::parser) fn tbody_start_tag_in_table(&mut self, token: TagToken) {
        self.clear_open_elements_back_to_table_context();
        self.insert_element(token, NS_HTML);
        self.insertion_mode = InsertionMode::InTableBody;
    }

    pub(in crate::parser) fn td_start_tag_in_table(&mut self, token: TagToken) {
        self.clear_open_elements_back_to_table_context();
        self.insert_fake_element("tbody", TagId::Tbody);
        self.insertion_mode = InsertionMode::InTableBody;
        self.start_tag_in_table_body(token);
    }

    pub(in crate::parser) fn table_start_tag_in_table(&mut self, token: TagToken) {
        if self.open_elements.has_in_table_scope(TagId::Table) {
            self.pop_open_elements_until_tag_name_popped(TagId::Table);
            self.reset_insertion_mode();
            self.process_start_tag(token, false);
        }
    }

    pub(in crate::parser) fn input_start_tag_in_table(&mut self, mut token: TagToken) {
        token.ack_self_closing = true;

        if get_token_attr(&token, "type").is_some_and(|value| value.eq_ignore_ascii_case("hidden"))
        {
            self.append_element(token, NS_HTML);
        } else {
            self.token_in_table(Token::Tag(token));
        }
    }

    pub(in crate::parser) fn form_start_tag_in_table(&mut self, token: TagToken) {
        if self.form_element.is_none() && self.open_elements.tmpl_count == 0 {
            let form = self.insert_element(token, NS_HTML);
            self.form_element = Some(form);
            self.pop_open_element();
        }
    }

    pub(in crate::parser) fn start_tag_in_table(&mut self, token: TagToken) {
        match token.tag_id {
            TagId::Td | TagId::Th | TagId::Tr => self.td_start_tag_in_table(token),
            TagId::Style | TagId::Script | TagId::Template => self.start_tag_in_head(token),
            TagId::Col => self.col_start_tag_in_table(token),
            TagId::Form => self.form_start_tag_in_table(token),
            TagId::Table => self.table_start_tag_in_table(token),
            TagId::Tbody | TagId::Tfoot | TagId::Thead => self.tbody_start_tag_in_table(token),
            TagId::Input => self.input_start_tag_in_table(token),
            TagId::Caption => self.caption_start_tag_in_table(token),
            TagId::Colgroup => self.colgroup_start_tag_in_table(token),
            _ => self.token_in_table(Token::Tag(token)),
        }
    }

    pub(in crate::parser) fn end_tag_in_table(&mut self, token: TagToken) {
        match token.tag_id {
            TagId::Table => {
                if self.open_elements.has_in_table_scope(TagId::Table) {
                    self.update_last_element_end_location(TagId::Table, token.location);
                    self.pop_open_elements_until_tag_name_popped(TagId::Table);
                    self.reset_insertion_mode();
                }
            }
            TagId::Template => self.template_end_tag_in_head(token),
            TagId::Body
            | TagId::Caption
            | TagId::Col
            | TagId::Colgroup
            | TagId::Html
            | TagId::Tbody
            | TagId::Td
            | TagId::Tfoot
            | TagId::Th
            | TagId::Thead
            | TagId::Tr => {}
            _ => self.token_in_table(Token::Tag(token)),
        }
    }

    pub(in crate::parser) fn token_in_table(&mut self, token: Token) {
        let saved_foster_parenting_state = self.foster_parenting_enabled;
        self.foster_parenting_enabled = true;
        self.mode_in_body(token);
        self.foster_parenting_enabled = saved_foster_parenting_state;
    }

    pub(in crate::parser) fn start_tag_in_caption(&mut self, token: TagToken) {
        if is_table_void_element(token.tag_id) {
            if self.open_elements.has_in_table_scope(TagId::Caption) {
                self.generate_implied_end_tags();
                self.pop_open_elements_until_tag_name_popped(TagId::Caption);
                self.active_formatting_elements.clear_to_last_marker();
                self.insertion_mode = InsertionMode::InTable;
                self.start_tag_in_table(token);
            }
        } else {
            self.start_tag_in_body(token);
        }
    }

    pub(in crate::parser) fn end_tag_in_caption(&mut self, token: TagToken) {
        match token.tag_id {
            TagId::Caption | TagId::Table => {
                if self.open_elements.has_in_table_scope(TagId::Caption) {
                    self.update_last_element_end_location(TagId::Caption, token.location.clone());
                    self.generate_implied_end_tags();
                    self.pop_open_elements_until_tag_name_popped(TagId::Caption);
                    self.active_formatting_elements.clear_to_last_marker();
                    self.insertion_mode = InsertionMode::InTable;

                    if token.tag_id == TagId::Table {
                        self.end_tag_in_table(token);
                    }
                }
            }
            TagId::Body
            | TagId::Col
            | TagId::Colgroup
            | TagId::Html
            | TagId::Tbody
            | TagId::Td
            | TagId::Tfoot
            | TagId::Th
            | TagId::Thead
            | TagId::Tr => {}
            _ => self.end_tag_in_body(token),
        }
    }

    pub(in crate::parser) fn start_tag_in_column_group(&mut self, token: TagToken) {
        match token.tag_id {
            TagId::Html => self.start_tag_in_body(token),
            TagId::Col => {
                let mut token = token;
                token.ack_self_closing = true;
                self.append_element(token, NS_HTML);
            }
            TagId::Template => self.start_tag_in_head(token),
            _ => self.token_in_column_group(Token::Tag(token)),
        }
    }

    pub(in crate::parser) fn end_tag_in_column_group(&mut self, token: TagToken) {
        match token.tag_id {
            TagId::Colgroup => {
                if self.open_elements.current_tag_id == Some(TagId::Colgroup) {
                    self.pop_current_element_for_token(
                        token.location.map(|location| location.location),
                    );
                    self.insertion_mode = InsertionMode::InTable;
                }
            }
            TagId::Template => self.template_end_tag_in_head(token),
            TagId::Col => {}
            _ => self.token_in_column_group(Token::Tag(token)),
        }
    }

    pub(in crate::parser) fn token_in_column_group(&mut self, token: Token) {
        if self.open_elements.current_tag_id == Some(TagId::Colgroup) {
            self.pop_open_element();
            self.insertion_mode = InsertionMode::InTable;
            self.process_token(token);
        }
    }

    pub(in crate::parser) fn start_tag_in_table_body(&mut self, token: TagToken) {
        match token.tag_id {
            TagId::Tr => {
                self.clear_open_elements_back_to_table_body_context();
                self.insert_element(token, NS_HTML);
                self.insertion_mode = InsertionMode::InRow;
            }
            TagId::Th | TagId::Td => {
                self.clear_open_elements_back_to_table_body_context();
                self.insert_fake_element("tr", TagId::Tr);
                self.insertion_mode = InsertionMode::InRow;
                self.start_tag_in_row(token);
            }
            TagId::Caption
            | TagId::Col
            | TagId::Colgroup
            | TagId::Tbody
            | TagId::Tfoot
            | TagId::Thead => {
                if self.open_elements.has_table_body_context_in_table_scope() {
                    self.clear_open_elements_back_to_table_body_context();
                    self.pop_open_element();
                    self.insertion_mode = InsertionMode::InTable;
                    self.start_tag_in_table(token);
                }
            }
            _ => self.start_tag_in_table(token),
        }
    }

    pub(in crate::parser) fn end_tag_in_table_body(&mut self, token: TagToken) {
        match token.tag_id {
            TagId::Tbody | TagId::Tfoot | TagId::Thead => {
                if self.open_elements.has_in_table_scope(token.tag_id) {
                    self.update_last_element_end_location(token.tag_id, token.location);
                    self.clear_open_elements_back_to_table_body_context();
                    self.pop_open_element();
                    self.insertion_mode = InsertionMode::InTable;
                }
            }
            TagId::Table => {
                if self.open_elements.has_table_body_context_in_table_scope() {
                    self.clear_open_elements_back_to_table_body_context();
                    self.pop_open_element();
                    self.insertion_mode = InsertionMode::InTable;
                    self.end_tag_in_table(token);
                }
            }
            TagId::Body
            | TagId::Caption
            | TagId::Col
            | TagId::Colgroup
            | TagId::Html
            | TagId::Td
            | TagId::Th
            | TagId::Tr => {}
            _ => self.end_tag_in_table(token),
        }
    }

    pub(in crate::parser) fn start_tag_in_row(&mut self, token: TagToken) {
        match token.tag_id {
            TagId::Th | TagId::Td => {
                self.clear_open_elements_back_to_table_row_context();
                self.insert_element(token, NS_HTML);
                self.insertion_mode = InsertionMode::InCell;
                self.active_formatting_elements.insert_marker();
            }
            TagId::Caption
            | TagId::Col
            | TagId::Colgroup
            | TagId::Tbody
            | TagId::Tfoot
            | TagId::Thead
            | TagId::Tr => {
                if self.open_elements.has_in_table_scope(TagId::Tr) {
                    self.clear_open_elements_back_to_table_row_context();
                    self.pop_open_element();
                    self.insertion_mode = InsertionMode::InTableBody;
                    self.start_tag_in_table_body(token);
                }
            }
            _ => self.start_tag_in_table(token),
        }
    }

    pub(in crate::parser) fn end_tag_in_row(&mut self, token: TagToken) {
        match token.tag_id {
            TagId::Tr => {
                if self.open_elements.has_in_table_scope(TagId::Tr) {
                    self.update_last_element_end_location(TagId::Tr, token.location);
                    self.clear_open_elements_back_to_table_row_context();
                    self.pop_open_element();
                    self.insertion_mode = InsertionMode::InTableBody;
                }
            }
            TagId::Table => {
                if self.open_elements.has_in_table_scope(TagId::Tr) {
                    self.clear_open_elements_back_to_table_row_context();
                    self.pop_open_element();
                    self.insertion_mode = InsertionMode::InTableBody;
                    self.end_tag_in_table_body(token);
                }
            }
            TagId::Tbody | TagId::Tfoot | TagId::Thead => {
                if self.open_elements.has_in_table_scope(token.tag_id)
                    || self.open_elements.has_in_table_scope(TagId::Tr)
                {
                    self.clear_open_elements_back_to_table_row_context();
                    self.pop_open_element();
                    self.insertion_mode = InsertionMode::InTableBody;
                    self.end_tag_in_table_body(token);
                }
            }
            TagId::Body
            | TagId::Caption
            | TagId::Col
            | TagId::Colgroup
            | TagId::Html
            | TagId::Td
            | TagId::Th => {}
            _ => self.end_tag_in_table(token),
        }
    }

    pub(in crate::parser) fn close_table_cell(&mut self) {
        self.generate_implied_end_tags();
        self.pop_open_elements_until_table_cell_popped();
        self.active_formatting_elements.clear_to_last_marker();
        self.insertion_mode = InsertionMode::InRow;
    }

    pub(in crate::parser) fn start_tag_in_cell(&mut self, token: TagToken) {
        if is_table_void_element(token.tag_id) {
            if self.open_elements.has_in_table_scope(TagId::Td)
                || self.open_elements.has_in_table_scope(TagId::Th)
            {
                self.close_table_cell();
                self.start_tag_in_row(token);
            }
        } else {
            self.start_tag_in_body(token);
        }
    }

    pub(in crate::parser) fn end_tag_in_cell(&mut self, token: TagToken) {
        match token.tag_id {
            TagId::Td | TagId::Th => {
                if self.open_elements.has_in_table_scope(token.tag_id) {
                    self.update_last_element_end_location(token.tag_id, token.location);
                    self.generate_implied_end_tags();
                    self.pop_open_elements_until_tag_name_popped(token.tag_id);
                    self.active_formatting_elements.clear_to_last_marker();
                    self.insertion_mode = InsertionMode::InRow;
                }
            }
            TagId::Table | TagId::Tbody | TagId::Tfoot | TagId::Thead | TagId::Tr => {
                if self.open_elements.has_in_table_scope(token.tag_id) {
                    self.close_table_cell();
                    self.end_tag_in_row(token);
                }
            }
            TagId::Body | TagId::Caption | TagId::Col | TagId::Colgroup | TagId::Html => {}
            _ => self.end_tag_in_body(token),
        }
    }
}
