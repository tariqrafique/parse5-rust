use super::Parser;
use super::*;

impl Parser {
    // Included into parser/mod.rs. Pure move.
    pub(in crate::parser) fn doctype_in_initial_mode(&mut self, token: DoctypeToken) {
        let name = token.name.clone().unwrap_or_default();
        let public_id = token.public_id.clone().unwrap_or_default();
        let system_id = token.system_id.clone().unwrap_or_default();
        set_document_type(&self.document, &name, &public_id, &system_id);

        if self.options.source_code_location_info {
            if let Some(location) = token.location {
                if let Some(doctype_node) =
                    self.document.borrow().child_nodes().and_then(|children| {
                        children
                            .iter()
                            .find(|node| node.borrow().is_document_type())
                            .cloned()
                    })
                {
                    doctype_node
                        .borrow_mut()
                        .set_source_code_location(Some(ElementLocation::from(location)));
                }
            }
        }

        let mode = doctype::document_mode(
            token.name.as_deref(),
            token.public_id.as_deref(),
            token.system_id.as_deref(),
            token.force_quirks,
        );

        if !doctype::is_conforming(
            token.name.as_deref(),
            token.public_id.as_deref(),
            token.system_id.as_deref(),
        ) {
            self.err_for_doctype(&token, ErrorCode::NonConformingDoctype);
        }

        set_document_mode(&self.document, mode);
        self.insertion_mode = InsertionMode::BeforeHtml;
    }

    pub(in crate::parser) fn token_in_initial_mode(&mut self, token: Token) {
        self.err_for_token(&token, ErrorCode::MissingDoctype, true);
        set_document_mode(&self.document, DocumentMode::Quirks);
        self.insertion_mode = InsertionMode::BeforeHtml;
        self.process_token(token);
    }

    pub(in crate::parser) fn start_tag_before_html(&mut self, token: TagToken) {
        if token.tag_id == TagId::Html {
            self.insert_element(token, NS_HTML);
            self.insertion_mode = InsertionMode::BeforeHead;
        } else {
            self.token_before_html(Token::Tag(token));
        }
    }

    pub(in crate::parser) fn end_tag_before_html(&mut self, token: TagToken) {
        if matches!(
            token.tag_id,
            TagId::Html | TagId::Head | TagId::Body | TagId::Br
        ) {
            self.token_before_html(Token::Tag(token));
        }
    }

    pub(in crate::parser) fn token_before_html(&mut self, token: Token) {
        self.insert_fake_root_element();
        self.insertion_mode = InsertionMode::BeforeHead;
        self.process_token(token);
    }

    pub(in crate::parser) fn start_tag_before_head(&mut self, token: TagToken) {
        match token.tag_id {
            TagId::Html => self.start_tag_in_body(token),
            TagId::Head => {
                self.insert_element(token, NS_HTML);
                self.head_element = self.open_elements.current.clone();
                self.insertion_mode = InsertionMode::InHead;
            }
            _ => self.token_before_head(Token::Tag(token)),
        }
    }

    pub(in crate::parser) fn end_tag_before_head(&mut self, token: TagToken) {
        if matches!(
            token.tag_id,
            TagId::Head | TagId::Body | TagId::Html | TagId::Br
        ) {
            self.token_before_head(Token::Tag(token));
        } else {
            self.err_for_tag(&token, ErrorCode::EndTagWithoutMatchingOpenElement);
        }
    }

    pub(in crate::parser) fn token_before_head(&mut self, token: Token) {
        self.insert_fake_element("head", TagId::Head);
        self.head_element = self.open_elements.current.clone();
        self.insertion_mode = InsertionMode::InHead;
        self.process_token(token);
    }

    pub(in crate::parser) fn start_tag_in_head(&mut self, token: TagToken) {
        match token.tag_id {
            TagId::Html => self.start_tag_in_body(token),
            TagId::Base | TagId::Basefont | TagId::Bgsound | TagId::Link | TagId::Meta => {
                self.append_element(token, NS_HTML);
            }
            TagId::Title => self.switch_to_text_parsing(token, TokenizerState::RcData),
            TagId::Noscript if self.options.scripting_enabled => {
                self.switch_to_text_parsing(token, TokenizerState::RawText);
            }
            TagId::Noscript => {
                self.insert_element(token, NS_HTML);
                self.insertion_mode = InsertionMode::InHeadNoScript;
            }
            TagId::Noframes | TagId::Style => {
                self.switch_to_text_parsing(token, TokenizerState::RawText);
            }
            TagId::Script => self.switch_to_text_parsing(token, TokenizerState::ScriptData),
            TagId::Template => {
                self.insert_template(token);
                self.active_formatting_elements.insert_marker();
                self.frameset_ok = false;
                self.insertion_mode = InsertionMode::InTemplate;
                self.tmpl_insertion_mode_stack
                    .insert(0, InsertionMode::InTemplate);
            }
            TagId::Head => self.err_for_tag(&token, ErrorCode::MisplacedStartTagForHeadElement),
            _ => self.token_in_head(Token::Tag(token)),
        }
    }

    pub(in crate::parser) fn end_tag_in_head(&mut self, token: TagToken) {
        match token.tag_id {
            TagId::Head => {
                self.pop_current_element_for_token(
                    token.location.map(|location| location.location),
                );
                self.insertion_mode = InsertionMode::AfterHead;
            }
            TagId::Body | TagId::Br | TagId::Html => self.token_in_head(Token::Tag(token)),
            TagId::Template => self.template_end_tag_in_head(token),
            _ => self.err_for_tag(&token, ErrorCode::EndTagWithoutMatchingOpenElement),
        }
    }

    pub(in crate::parser) fn template_end_tag_in_head(&mut self, token: TagToken) {
        if self.open_elements.tmpl_count > 0 {
            self.generate_implied_end_tags_thoroughly();

            if self.open_elements.current_tag_id != Some(TagId::Template) {
                self.err_for_tag(&token, ErrorCode::ClosingOfElementWithOpenChildElements);
            }

            self.update_last_element_end_location(TagId::Template, token.location);
            self.pop_open_elements_until_tag_name_popped(TagId::Template);
            self.active_formatting_elements.clear_to_last_marker();

            if !self.tmpl_insertion_mode_stack.is_empty() {
                self.tmpl_insertion_mode_stack.remove(0);
            }

            self.reset_insertion_mode();
        } else {
            self.err_for_tag(&token, ErrorCode::EndTagWithoutMatchingOpenElement);
        }
    }

    pub(in crate::parser) fn token_in_head(&mut self, token: Token) {
        self.pop_current_element_for_token(token_start_location(&token));
        self.insertion_mode = InsertionMode::AfterHead;
        self.process_token(token);
    }

    pub(in crate::parser) fn start_tag_in_head_no_script(&mut self, token: TagToken) {
        match token.tag_id {
            TagId::Html => self.start_tag_in_body(token),
            TagId::Basefont
            | TagId::Bgsound
            | TagId::Head
            | TagId::Link
            | TagId::Meta
            | TagId::Noframes
            | TagId::Style => self.start_tag_in_head(token),
            TagId::Noscript => self.err_for_tag(&token, ErrorCode::NestedNoscriptInHead),
            _ => self.token_in_head_no_script(Token::Tag(token)),
        }
    }

    pub(in crate::parser) fn end_tag_in_head_no_script(&mut self, token: TagToken) {
        match token.tag_id {
            TagId::Noscript => {
                self.pop_current_element_for_token(
                    token.location.map(|location| location.location),
                );
                self.insertion_mode = InsertionMode::InHead;
            }
            TagId::Br => self.token_in_head_no_script(Token::Tag(token)),
            _ => self.err_for_tag(&token, ErrorCode::EndTagWithoutMatchingOpenElement),
        }
    }

    pub(in crate::parser) fn token_in_head_no_script(&mut self, token: Token) {
        let code = if matches!(token, Token::Eof(_)) {
            ErrorCode::OpenElementsLeftAfterEof
        } else {
            ErrorCode::DisallowedContentInNoscriptInHead
        };
        self.err_for_token(&token, code, false);
        self.pop_current_element_for_token(token_start_location(&token));
        self.insertion_mode = InsertionMode::InHead;
        self.process_token(token);
    }

    pub(in crate::parser) fn start_tag_after_head(&mut self, token: TagToken) {
        match token.tag_id {
            TagId::Html => self.start_tag_in_body(token),
            TagId::Body => {
                self.insert_element(token, NS_HTML);
                self.frameset_ok = false;
                self.insertion_mode = InsertionMode::InBody;
            }
            TagId::Frameset => {
                self.insert_element(token, NS_HTML);
                self.insertion_mode = InsertionMode::InFrameset;
            }
            TagId::Base
            | TagId::Basefont
            | TagId::Bgsound
            | TagId::Link
            | TagId::Meta
            | TagId::Noframes
            | TagId::Script
            | TagId::Style
            | TagId::Template
            | TagId::Title => {
                self.err_for_tag(&token, ErrorCode::AbandonedHeadElementChild);
                if let Some(head) = self.head_element.clone() {
                    self.push_open_element(head.clone(), TagId::Head);
                    self.start_tag_in_head(token);
                    self.remove_open_element(&head);
                }
            }
            TagId::Head => self.err_for_tag(&token, ErrorCode::MisplacedStartTagForHeadElement),
            _ => self.token_after_head(Token::Tag(token)),
        }
    }

    pub(in crate::parser) fn end_tag_after_head(&mut self, token: TagToken) {
        match token.tag_id {
            TagId::Body | TagId::Html | TagId::Br => self.token_after_head(Token::Tag(token)),
            TagId::Template => self.template_end_tag_in_head(token),
            _ => self.err_for_tag(&token, ErrorCode::EndTagWithoutMatchingOpenElement),
        }
    }

    pub(in crate::parser) fn token_after_head(&mut self, token: Token) {
        self.insert_fake_element("body", TagId::Body);
        self.insertion_mode = InsertionMode::InBody;
        self.mode_in_body(token);
    }
}
