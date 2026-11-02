use super::Parser;
use super::*;

impl Parser {
    // Included into parser/mod.rs. Pure move.
    pub(in crate::parser) fn end_tag_in_text(&mut self, token: TagToken) {
        if token.tag_id == TagId::Script && self.script_handler_enabled {
            if let Some(node) = self.open_elements.current.clone() {
                self.script_events.push(ScriptEvent { node });
                self.paused_on_script = true;
            }
        }

        self.pop_current_element_for_token(token.location.map(|location| location.location));
        self.insertion_mode = self.original_insertion_mode;
    }

    pub(in crate::parser) fn eof_in_text(&mut self, token: EofToken) {
        self.err_at_location(
            token.location,
            ErrorCode::EofInElementThatCanContainOnlyText,
            false,
        );
        self.pop_open_element();
        self.insertion_mode = self.original_insertion_mode;
        self.on_eof(token);
    }

    pub(in crate::parser) fn start_tag_after_body(&mut self, token: TagToken) {
        if token.tag_id == TagId::Html {
            self.start_tag_in_body(token);
        } else {
            self.token_after_body(Token::Tag(token));
        }
    }

    pub(in crate::parser) fn end_tag_after_body(&mut self, token: TagToken) {
        if token.tag_id == TagId::Html {
            if self.fragment_context.is_none() {
                self.insertion_mode = InsertionMode::AfterAfterBody;
            }

            let token = Token::Tag(token);

            if let Some(html) = self.open_elements.items.first().cloned() {
                self.set_end_location_for_element_with_token(&html, &token);
            }

            if let Some(body) = self.open_elements.items.get(1).cloned() {
                let has_end_tag = body
                    .borrow()
                    .source_code_location()
                    .and_then(|location| location.end_tag)
                    .is_some();

                if !has_end_tag {
                    self.set_end_location_for_element_with_token(&body, &token);
                }
            }
        } else {
            self.token_after_body(Token::Tag(token));
        }
    }

    pub(in crate::parser) fn token_after_body(&mut self, token: Token) {
        self.insertion_mode = InsertionMode::InBody;
        self.mode_in_body(token);
    }

    pub(in crate::parser) fn start_tag_in_frameset(&mut self, mut token: TagToken) {
        match token.tag_id {
            TagId::Html => self.start_tag_in_body(token),
            TagId::Frameset => {
                self.insert_element(token, NS_HTML);
            }
            TagId::Frame => {
                token.ack_self_closing = true;
                self.append_element(token, NS_HTML);
            }
            TagId::Noframes => self.start_tag_in_head(token),
            _ => {}
        }
    }

    pub(in crate::parser) fn end_tag_in_frameset(&mut self, token: TagToken) {
        if token.tag_id == TagId::Frameset && !self.open_elements.is_root_html_element_current() {
            self.pop_current_element_for_token(token.location.map(|location| location.location));

            if self.fragment_context.is_none()
                && self.open_elements.current_tag_id != Some(TagId::Frameset)
            {
                self.insertion_mode = InsertionMode::AfterFrameset;
            }
        }
    }

    pub(in crate::parser) fn start_tag_after_frameset(&mut self, token: TagToken) {
        match token.tag_id {
            TagId::Html => self.start_tag_in_body(token),
            TagId::Noframes => self.start_tag_in_head(token),
            _ => {}
        }
    }

    pub(in crate::parser) fn end_tag_after_frameset(&mut self, token: TagToken) {
        if token.tag_id == TagId::Html {
            self.insertion_mode = InsertionMode::AfterAfterFrameset;
        }
    }

    pub(in crate::parser) fn start_tag_after_after_body(&mut self, token: TagToken) {
        if token.tag_id == TagId::Html {
            self.start_tag_in_body(token);
        } else {
            self.token_after_after_body(Token::Tag(token));
        }
    }

    pub(in crate::parser) fn token_after_after_body(&mut self, token: Token) {
        self.insertion_mode = InsertionMode::InBody;
        self.mode_in_body(token);
    }

    pub(in crate::parser) fn start_tag_after_after_frameset(&mut self, token: TagToken) {
        match token.tag_id {
            TagId::Html => self.start_tag_in_body(token),
            TagId::Noframes => self.start_tag_in_head(token),
            _ => {}
        }
    }

    pub(in crate::parser) fn stop_parsing(&mut self, token: Token) {
        self.stopped = true;

        if token_location(&token).is_none() {
            return;
        }

        let target = if self.fragment_context.is_some() {
            0
        } else {
            2
        };

        for idx in (target..=self.open_elements.stack_top.max(-1)).rev() {
            if let Some(element) = self.open_elements.items.get(idx as usize).cloned() {
                self.set_end_location_for_element_with_token(&element, &token);
            }
        }

        if self.fragment_context.is_none() && self.open_elements.stack_top >= 0 {
            let Some(html_element) = self.open_elements.items.first().cloned() else {
                return;
            };
            let html_has_end_tag = html_element
                .borrow()
                .source_code_location()
                .and_then(|location| location.end_tag)
                .is_some();

            if !html_has_end_tag {
                self.set_end_location_for_element_with_token(&html_element, &token);

                if let Some(body_element) = self.open_elements.items.get(1).cloned() {
                    let body_has_end_tag = body_element
                        .borrow()
                        .source_code_location()
                        .and_then(|location| location.end_tag)
                        .is_some();

                    if !body_has_end_tag {
                        self.set_end_location_for_element_with_token(&body_element, &token);
                    }
                }
            }
        }
    }
}
