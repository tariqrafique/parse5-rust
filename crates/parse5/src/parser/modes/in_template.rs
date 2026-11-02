use super::Parser;
use super::*;

impl Parser {
    // Included into parser/mod.rs. Pure move.
    pub(in crate::parser) fn start_tag_in_template(&mut self, token: TagToken) {
        match token.tag_id {
            TagId::Base
            | TagId::Basefont
            | TagId::Bgsound
            | TagId::Link
            | TagId::Meta
            | TagId::Noframes
            | TagId::Script
            | TagId::Style
            | TagId::Template
            | TagId::Title => self.start_tag_in_head(token),
            TagId::Caption | TagId::Colgroup | TagId::Tbody | TagId::Tfoot | TagId::Thead => {
                self.set_current_template_insertion_mode(InsertionMode::InTable);
                self.insertion_mode = InsertionMode::InTable;
                self.start_tag_in_table(token);
            }
            TagId::Col => {
                self.set_current_template_insertion_mode(InsertionMode::InColumnGroup);
                self.insertion_mode = InsertionMode::InColumnGroup;
                self.start_tag_in_column_group(token);
            }
            TagId::Tr => {
                self.set_current_template_insertion_mode(InsertionMode::InTableBody);
                self.insertion_mode = InsertionMode::InTableBody;
                self.start_tag_in_table_body(token);
            }
            TagId::Td | TagId::Th => {
                self.set_current_template_insertion_mode(InsertionMode::InRow);
                self.insertion_mode = InsertionMode::InRow;
                self.start_tag_in_row(token);
            }
            _ => {
                self.set_current_template_insertion_mode(InsertionMode::InBody);
                self.insertion_mode = InsertionMode::InBody;
                self.start_tag_in_body(token);
            }
        }
    }

    pub(in crate::parser) fn set_current_template_insertion_mode(&mut self, mode: InsertionMode) {
        if let Some(current_mode) = self.tmpl_insertion_mode_stack.first_mut() {
            *current_mode = mode;
        }
    }

    pub(in crate::parser) fn end_tag_in_template(&mut self, token: TagToken) {
        if token.tag_id == TagId::Template {
            self.template_end_tag_in_head(token);
        }
    }

    pub(in crate::parser) fn eof_in_template(&mut self, token: Token) {
        if self.open_elements.tmpl_count > 0 {
            self.pop_open_elements_until_tag_name_popped(TagId::Template);
            self.active_formatting_elements.clear_to_last_marker();
            if !self.tmpl_insertion_mode_stack.is_empty() {
                self.tmpl_insertion_mode_stack.remove(0);
            }
            self.reset_insertion_mode();
            self.process_token(token);
        } else {
            self.stop_parsing(token);
        }
    }
}
