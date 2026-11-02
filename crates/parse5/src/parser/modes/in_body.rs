use super::Parser;
use super::*;

impl Parser {
    // Included into parser/mod.rs. Pure move.
    pub(in crate::parser) fn start_tag_in_body(&mut self, token: TagToken) {
        match token.tag_id {
            TagId::I
            | TagId::S
            | TagId::B
            | TagId::U
            | TagId::Em
            | TagId::Tt
            | TagId::Big
            | TagId::Code
            | TagId::Font
            | TagId::Small
            | TagId::Strike
            | TagId::Strong => self.formatting_start_tag_in_body(token),
            TagId::A => self.anchor_start_tag_in_body(token),
            TagId::H1 | TagId::H2 | TagId::H3 | TagId::H4 | TagId::H5 | TagId::H6 => {
                self.numbered_header_start_tag_in_body(token)
            }
            TagId::P
            | TagId::Dl
            | TagId::Ol
            | TagId::Ul
            | TagId::Div
            | TagId::Dir
            | TagId::Nav
            | TagId::Main
            | TagId::Menu
            | TagId::Aside
            | TagId::Center
            | TagId::Figure
            | TagId::Footer
            | TagId::Header
            | TagId::Hgroup
            | TagId::Dialog
            | TagId::Details
            | TagId::Address
            | TagId::Article
            | TagId::Search
            | TagId::Section
            | TagId::Summary
            | TagId::Fieldset
            | TagId::Blockquote
            | TagId::Figcaption => self.address_start_tag_in_body(token),
            TagId::Li | TagId::Dd | TagId::Dt => self.list_item_start_tag_in_body(token),
            TagId::Br | TagId::Img | TagId::Wbr | TagId::Area | TagId::Embed | TagId::Keygen => {
                self.area_start_tag_in_body(token)
            }
            TagId::Hr => self.hr_start_tag_in_body(token),
            TagId::Rb | TagId::Rtc => self.rb_start_tag_in_body(token),
            TagId::Rt | TagId::Rp => self.rt_start_tag_in_body(token),
            TagId::Pre | TagId::Listing => self.pre_start_tag_in_body(token),
            TagId::Base
            | TagId::Basefont
            | TagId::Bgsound
            | TagId::Link
            | TagId::Meta
            | TagId::Script
            | TagId::Style
            | TagId::Template
            | TagId::Title => self.start_tag_in_head(token),
            TagId::Form => self.form_start_tag_in_body(token),
            TagId::Nobr => self.nobr_start_tag_in_body(token),
            TagId::Iframe => self.iframe_start_tag_in_body(token),
            TagId::Math => self.math_start_tag_in_body(token),
            TagId::Table => self.table_start_tag_in_body(token),
            TagId::Input => self.input_start_tag_in_body(token),
            TagId::Param | TagId::Track | TagId::Source => self.param_start_tag_in_body(token),
            TagId::Image => self.image_start_tag_in_body(token),
            TagId::Button => self.button_start_tag_in_body(token),
            TagId::Applet | TagId::Object | TagId::Marquee => self.applet_start_tag_in_body(token),
            TagId::Noembed | TagId::Noframes => self.raw_text_start_tag_in_body(token),
            TagId::Select => self.select_start_tag_in_body(token),
            TagId::Option | TagId::Optgroup => self.optgroup_start_tag_in_body(token),
            TagId::Noscript if self.options.scripting_enabled => {
                self.raw_text_start_tag_in_body(token)
            }
            TagId::Plaintext => self.plaintext_start_tag_in_body(token),
            TagId::Svg => self.svg_start_tag_in_body(token),
            TagId::Textarea => self.textarea_start_tag_in_body(token),
            TagId::Xmp => self.xmp_start_tag_in_body(token),
            TagId::Html => self.html_start_tag_in_body(token),
            TagId::Body => self.body_start_tag_in_body(token),
            TagId::Frameset => self.frameset_start_tag_in_body(token),
            TagId::Col
            | TagId::Th
            | TagId::Td
            | TagId::Tr
            | TagId::Head
            | TagId::Frame
            | TagId::Tbody
            | TagId::Tfoot
            | TagId::Thead
            | TagId::Caption
            | TagId::Colgroup => {}
            _ => {
                if is_void_element(&token.tag_name) {
                    self.reconstruct_active_formatting_elements();
                    self.append_element(token, NS_HTML);
                } else {
                    self.generic_start_tag_in_body(token);
                }
            }
        }
    }

    pub(in crate::parser) fn mode_in_body(&mut self, token: Token) {
        match token {
            Token::Character(token) => {
                if token.token_type == TokenType::WhitespaceCharacter {
                    self.whitespace_character_in_body(token);
                } else if token.token_type == TokenType::Character {
                    self.character_in_body(token);
                }
            }
            Token::Comment(token) => self.on_comment(token),
            Token::Tag(token) if token.token_type == TokenType::StartTag => {
                self.start_tag_in_body(token)
            }
            Token::Tag(token) if token.token_type == TokenType::EndTag => {
                self.end_tag_in_body(token)
            }
            Token::Eof(token) => self.stop_parsing(Token::Eof(token)),
            _ => {}
        }
    }

    pub(in crate::parser) fn whitespace_character_in_body(
        &mut self,
        token: crate::common::token::CharacterToken,
    ) {
        self.reconstruct_active_formatting_elements();
        self.insert_characters(&token.chars, token.location);
    }

    pub(in crate::parser) fn character_in_body(
        &mut self,
        token: crate::common::token::CharacterToken,
    ) {
        self.reconstruct_active_formatting_elements();
        self.insert_characters(&token.chars, token.location);
        self.frameset_ok = false;
    }

    pub(in crate::parser) fn reconstruct_active_formatting_elements(&mut self) {
        let list_length = self.active_formatting_elements.entries.len();

        if list_length == 0 {
            return;
        }

        let end_index = self
            .active_formatting_elements
            .entries
            .iter()
            .position(|entry| {
                entry.is_marker()
                    || entry
                        .element
                        .as_ref()
                        .is_some_and(|element| self.open_elements.contains(element))
            });

        let Some(unopen_idx) = end_index
            .map(|idx| idx.checked_sub(1))
            .unwrap_or_else(|| Some(list_length - 1))
        else {
            return;
        };

        for idx in (0..=unopen_idx).rev() {
            let Some(token) = self.active_formatting_elements.entries[idx].token.clone() else {
                continue;
            };
            let namespace_uri = self.active_formatting_elements.entries[idx]
                .element
                .as_ref()
                .and_then(namespace_uri)
                .unwrap_or_else(|| NS_HTML.to_string());
            let element = self.insert_element(token, &namespace_uri);
            self.active_formatting_elements.entries[idx].element = Some(element);
        }
    }

    pub(in crate::parser) fn close_p_element(&mut self) {
        self.generate_implied_end_tags_with_exclusion(TagId::P);
        self.pop_open_elements_until_tag_name_popped(TagId::P);
    }

    pub(in crate::parser) fn address_start_tag_in_body(&mut self, token: TagToken) {
        if self.open_elements.has_in_button_scope(TagId::P) {
            self.close_p_element();
        }

        self.insert_element(token, NS_HTML);
    }

    pub(in crate::parser) fn numbered_header_start_tag_in_body(&mut self, token: TagToken) {
        if self.open_elements.has_in_button_scope(TagId::P) {
            self.close_p_element();
        }

        if self
            .open_elements
            .current_tag_id
            .is_some_and(is_numbered_header)
        {
            self.pop_open_element();
        }

        self.insert_element(token, NS_HTML);
    }

    pub(in crate::parser) fn pre_start_tag_in_body(&mut self, token: TagToken) {
        if self.open_elements.has_in_button_scope(TagId::P) {
            self.close_p_element();
        }

        self.insert_element(token, NS_HTML);
        self.skip_next_new_line = true;
        self.frameset_ok = false;
    }

    pub(in crate::parser) fn form_start_tag_in_body(&mut self, token: TagToken) {
        let in_template = self.open_elements.tmpl_count > 0;

        if self.form_element.is_none() || in_template {
            if self.open_elements.has_in_button_scope(TagId::P) {
                self.close_p_element();
            }

            let form = self.insert_element(token, NS_HTML);

            if !in_template {
                self.form_element = Some(form);
            }
        }
    }

    pub(in crate::parser) fn list_item_start_tag_in_body(&mut self, token: TagToken) {
        self.frameset_ok = false;
        let tag_id = token.tag_id;

        for idx in (0..=self.open_elements.stack_top.max(0) as usize).rev() {
            let element_id = self.open_elements.tag_ids[idx];

            if (tag_id == TagId::Li && element_id == TagId::Li)
                || (matches!(tag_id, TagId::Dd | TagId::Dt)
                    && matches!(element_id, TagId::Dd | TagId::Dt))
            {
                self.generate_implied_end_tags_with_exclusion(element_id);
                self.pop_open_elements_until_tag_name_popped(element_id);
                break;
            }

            if element_id != TagId::Address
                && element_id != TagId::Div
                && element_id != TagId::P
                && self.is_special_element(&self.open_elements.items[idx], element_id)
            {
                break;
            }
        }

        if self.open_elements.has_in_button_scope(TagId::P) {
            self.close_p_element();
        }

        self.insert_element(token, NS_HTML);
    }

    pub(in crate::parser) fn button_start_tag_in_body(&mut self, token: TagToken) {
        if self.open_elements.has_in_scope(TagId::Button) {
            self.generate_implied_end_tags();
            self.pop_open_elements_until_tag_name_popped(TagId::Button);
        }

        self.reconstruct_active_formatting_elements();
        self.insert_element(token, NS_HTML);
        self.frameset_ok = false;
    }

    pub(in crate::parser) fn nobr_start_tag_in_body(&mut self, token: TagToken) {
        self.reconstruct_active_formatting_elements();

        if self.open_elements.has_in_scope(TagId::Nobr) {
            self.call_adoption_agency(token.clone());
            self.reconstruct_active_formatting_elements();
        }

        let element = self.insert_element(token.clone(), NS_HTML);
        self.active_formatting_elements.push_element(element, token);
    }

    pub(in crate::parser) fn applet_start_tag_in_body(&mut self, token: TagToken) {
        self.reconstruct_active_formatting_elements();
        self.insert_element(token, NS_HTML);
        self.active_formatting_elements.insert_marker();
        self.frameset_ok = false;
    }

    pub(in crate::parser) fn table_start_tag_in_body(&mut self, token: TagToken) {
        if self.document_mode() != DocumentMode::Quirks
            && self.open_elements.has_in_button_scope(TagId::P)
        {
            self.close_p_element();
        }

        self.insert_element(token, NS_HTML);
        self.frameset_ok = false;
        self.insertion_mode = InsertionMode::InTable;
    }

    pub(in crate::parser) fn select_start_tag_in_body(&mut self, token: TagToken) {
        self.reconstruct_active_formatting_elements();
        self.insert_element(token, NS_HTML);
        self.frameset_ok = false;

        self.insertion_mode = match self.insertion_mode {
            InsertionMode::InTable
            | InsertionMode::InCaption
            | InsertionMode::InTableBody
            | InsertionMode::InRow
            | InsertionMode::InCell => InsertionMode::InSelectInTable,
            _ => InsertionMode::InSelect,
        };
    }

    pub(in crate::parser) fn optgroup_start_tag_in_body(&mut self, token: TagToken) {
        if self.open_elements.current_tag_id == Some(TagId::Option) {
            self.pop_open_element();
        }

        self.reconstruct_active_formatting_elements();
        self.insert_element(token, NS_HTML);
    }

    pub(in crate::parser) fn area_start_tag_in_body(&mut self, mut token: TagToken) {
        self.reconstruct_active_formatting_elements();
        token.ack_self_closing = true;
        self.append_element(token, NS_HTML);
        self.frameset_ok = false;
    }

    pub(in crate::parser) fn input_start_tag_in_body(&mut self, mut token: TagToken) {
        self.reconstruct_active_formatting_elements();
        let is_hidden = get_token_attr(&token, "type")
            .is_some_and(|value| value.eq_ignore_ascii_case("hidden"));
        token.ack_self_closing = true;
        self.append_element(token, NS_HTML);

        if !is_hidden {
            self.frameset_ok = false;
        }
    }

    pub(in crate::parser) fn param_start_tag_in_body(&mut self, mut token: TagToken) {
        token.ack_self_closing = true;
        self.append_element(token, NS_HTML);
    }

    pub(in crate::parser) fn hr_start_tag_in_body(&mut self, mut token: TagToken) {
        if self.open_elements.has_in_button_scope(TagId::P) {
            self.close_p_element();
        }

        token.ack_self_closing = true;
        self.append_element(token, NS_HTML);
        self.frameset_ok = false;
    }

    pub(in crate::parser) fn image_start_tag_in_body(&mut self, mut token: TagToken) {
        token.tag_name = "img".to_string();
        token.tag_id = TagId::Img;
        self.area_start_tag_in_body(token);
    }

    pub(in crate::parser) fn rb_start_tag_in_body(&mut self, token: TagToken) {
        if self.open_elements.has_in_scope(TagId::Ruby) {
            self.generate_implied_end_tags();
        }

        self.insert_element(token, NS_HTML);
    }

    pub(in crate::parser) fn rt_start_tag_in_body(&mut self, token: TagToken) {
        if self.open_elements.has_in_scope(TagId::Ruby) {
            self.generate_implied_end_tags_with_exclusion(TagId::Rtc);
        }

        self.insert_element(token, NS_HTML);
    }

    pub(in crate::parser) fn formatting_start_tag_in_body(&mut self, token: TagToken) {
        self.reconstruct_active_formatting_elements();
        let element = self.insert_element(token.clone(), NS_HTML);
        self.active_formatting_elements.push_element(element, token);
    }

    pub(in crate::parser) fn anchor_start_tag_in_body(&mut self, token: TagToken) {
        if let Some(active_element_entry) = self
            .active_formatting_elements
            .get_element_entry_in_scope_with_tag_name("a")
        {
            self.call_adoption_agency(token.clone());

            if let Some(element) = active_element_entry.element_ref() {
                self.remove_open_element(&element);
            }

            self.active_formatting_elements
                .remove_entry(&active_element_entry);
        }

        self.reconstruct_active_formatting_elements();
        let element = self.insert_element(token.clone(), NS_HTML);
        self.active_formatting_elements.push_element(element, token);
    }

    pub(in crate::parser) fn generic_start_tag_in_body(&mut self, token: TagToken) {
        self.reconstruct_active_formatting_elements();
        self.insert_element(token, NS_HTML);
    }

    pub(in crate::parser) fn switch_to_text_parsing(
        &mut self,
        token: TagToken,
        next_tokenizer_state: TokenizerState,
    ) {
        self.insert_element(token, NS_HTML);
        self.tokenizer.state = next_tokenizer_state;
        self.original_insertion_mode = self.insertion_mode;
        self.insertion_mode = InsertionMode::Text;
    }

    pub(in crate::parser) fn plaintext_start_tag_in_body(&mut self, token: TagToken) {
        if self.open_elements.has_in_button_scope(TagId::P) {
            self.close_p_element();
        }

        self.insert_element(token, NS_HTML);
        self.tokenizer.state = TokenizerState::PlainText;
    }

    pub(in crate::parser) fn textarea_start_tag_in_body(&mut self, token: TagToken) {
        self.insert_element(token, NS_HTML);
        self.skip_next_new_line = true;
        self.tokenizer.state = TokenizerState::RcData;
        self.original_insertion_mode = self.insertion_mode;
        self.frameset_ok = false;
        self.insertion_mode = InsertionMode::Text;
    }

    pub(in crate::parser) fn xmp_start_tag_in_body(&mut self, token: TagToken) {
        if self.open_elements.has_in_button_scope(TagId::P) {
            self.close_p_element();
        }

        self.reconstruct_active_formatting_elements();
        self.frameset_ok = false;
        self.switch_to_text_parsing(token, TokenizerState::RawText);
    }

    pub(in crate::parser) fn iframe_start_tag_in_body(&mut self, token: TagToken) {
        self.frameset_ok = false;
        self.switch_to_text_parsing(token, TokenizerState::RawText);
    }

    pub(in crate::parser) fn raw_text_start_tag_in_body(&mut self, token: TagToken) {
        self.switch_to_text_parsing(token, TokenizerState::RawText);
    }

    pub(in crate::parser) fn html_start_tag_in_body(&mut self, token: TagToken) {
        if self.open_elements.tmpl_count == 0 {
            if let Some(html) = self.open_elements.items.first() {
                crate::tree_adapters::default::adopt_attributes(html, token.attrs);
            }
        }
    }

    pub(in crate::parser) fn body_start_tag_in_body(&mut self, token: TagToken) {
        if let Some(body) = self.open_elements.try_peek_properly_nested_body_element() {
            if self.open_elements.tmpl_count == 0 {
                self.frameset_ok = false;
                crate::tree_adapters::default::adopt_attributes(&body, token.attrs);
            }
        }
    }

    pub(in crate::parser) fn frameset_start_tag_in_body(&mut self, token: TagToken) {
        let Some(body) = self.open_elements.try_peek_properly_nested_body_element() else {
            return;
        };

        if self.frameset_ok {
            detach_node(&body);
            self.pop_open_elements_all_up_to_html_element();
            self.insert_element(token, NS_HTML);
            self.insertion_mode = InsertionMode::InFrameset;
        }
    }

    pub(in crate::parser) fn eof_in_body(&mut self, token: Token) {
        if !self.tmpl_insertion_mode_stack.is_empty() {
            self.eof_in_template(token);
        } else {
            self.stop_parsing(token);
        }
    }

    pub(in crate::parser) fn end_tag_in_body(&mut self, token: TagToken) {
        match token.tag_id {
            TagId::A
            | TagId::B
            | TagId::I
            | TagId::S
            | TagId::U
            | TagId::Em
            | TagId::Tt
            | TagId::Big
            | TagId::Code
            | TagId::Font
            | TagId::Nobr
            | TagId::Small
            | TagId::Strike
            | TagId::Strong => self.call_adoption_agency(token),
            TagId::P => self.p_end_tag_in_body(),
            TagId::Dl
            | TagId::Ul
            | TagId::Ol
            | TagId::Dir
            | TagId::Div
            | TagId::Nav
            | TagId::Pre
            | TagId::Main
            | TagId::Menu
            | TagId::Aside
            | TagId::Button
            | TagId::Center
            | TagId::Figure
            | TagId::Footer
            | TagId::Header
            | TagId::Hgroup
            | TagId::Dialog
            | TagId::Address
            | TagId::Article
            | TagId::Details
            | TagId::Search
            | TagId::Section
            | TagId::Summary
            | TagId::Listing
            | TagId::Fieldset
            | TagId::Blockquote
            | TagId::Figcaption => self.address_end_tag_in_body(token),
            TagId::Li => self.li_end_tag_in_body(token),
            TagId::Dd | TagId::Dt => self.dd_end_tag_in_body(token),
            TagId::H1 | TagId::H2 | TagId::H3 | TagId::H4 | TagId::H5 | TagId::H6 => {
                self.numbered_header_end_tag_in_body(token)
            }
            TagId::Br => self.br_end_tag_in_body(),
            TagId::Template => self.template_end_tag_in_head(token),
            TagId::Body => {
                if self.open_elements.has_in_scope(TagId::Body) {
                    if let Some(body) = self.open_elements.try_peek_properly_nested_body_element() {
                        if let Some(location) = token.location {
                            update_element_end_location(&body, location.location);
                        }
                    }
                    self.insertion_mode = InsertionMode::AfterBody;
                }
            }
            TagId::Html => {
                if self.open_elements.has_in_scope(TagId::Body) {
                    self.insertion_mode = InsertionMode::AfterBody;
                    self.end_tag_after_body(token);
                }
            }
            TagId::Form => self.form_end_tag_in_body(token),
            TagId::Applet | TagId::Object | TagId::Marquee => self.applet_end_tag_in_body(token),
            _ => self.generic_end_tag_in_body(token),
        }
    }

    pub(in crate::parser) fn p_end_tag_in_body(&mut self) {
        if !self.open_elements.has_in_button_scope(TagId::P) {
            self.insert_fake_element("p", TagId::P);
        }

        self.close_p_element();
    }

    pub(in crate::parser) fn address_end_tag_in_body(&mut self, token: TagToken) {
        if self.open_elements.has_in_scope(token.tag_id) {
            self.update_last_element_end_location(token.tag_id, token.location);
            self.generate_implied_end_tags();
            self.pop_open_elements_until_tag_name_popped(token.tag_id);
        }
    }

    pub(in crate::parser) fn form_end_tag_in_body(&mut self, token: TagToken) {
        let in_template = self.open_elements.tmpl_count > 0;
        let form_element = self.form_element.clone();

        if !in_template {
            self.form_element = None;
        }

        if (form_element.is_some() || in_template) && self.open_elements.has_in_scope(TagId::Form) {
            self.update_last_element_end_location(TagId::Form, token.location);
            self.generate_implied_end_tags();

            if in_template {
                self.pop_open_elements_until_tag_name_popped(TagId::Form);
            } else if let Some(form_element) = form_element {
                self.remove_open_element(&form_element);
            }
        }
    }

    pub(in crate::parser) fn li_end_tag_in_body(&mut self, token: TagToken) {
        if self.open_elements.has_in_list_item_scope(TagId::Li) {
            self.update_last_element_end_location(TagId::Li, token.location);
            self.generate_implied_end_tags_with_exclusion(TagId::Li);
            self.pop_open_elements_until_tag_name_popped(TagId::Li);
        }
    }

    pub(in crate::parser) fn dd_end_tag_in_body(&mut self, token: TagToken) {
        if self.open_elements.has_in_scope(token.tag_id) {
            self.update_last_element_end_location(token.tag_id, token.location);
            self.generate_implied_end_tags_with_exclusion(token.tag_id);
            self.pop_open_elements_until_tag_name_popped(token.tag_id);
        }
    }

    pub(in crate::parser) fn numbered_header_end_tag_in_body(&mut self, token: TagToken) {
        if self.open_elements.has_numbered_header_in_scope() {
            if let Some(location) = token.location {
                for idx in (0..=self.open_elements.stack_top.max(0) as usize).rev() {
                    if is_numbered_header(self.open_elements.tag_ids[idx]) {
                        update_element_end_location(
                            &self.open_elements.items[idx],
                            location.location,
                        );
                        break;
                    }
                }
            }

            self.generate_implied_end_tags();
            self.pop_open_elements_until_numbered_header_popped();
        }
    }

    pub(in crate::parser) fn applet_end_tag_in_body(&mut self, token: TagToken) {
        if self.open_elements.has_in_scope(token.tag_id) {
            self.update_last_element_end_location(token.tag_id, token.location);
            self.generate_implied_end_tags();
            self.pop_open_elements_until_tag_name_popped(token.tag_id);
            self.active_formatting_elements.clear_to_last_marker();
        }
    }

    pub(in crate::parser) fn br_end_tag_in_body(&mut self) {
        self.reconstruct_active_formatting_elements();
        self.insert_fake_element("br", TagId::Br);
        self.pop_open_element();
        self.frameset_ok = false;
    }

    pub(in crate::parser) fn generic_end_tag_in_body(&mut self, token: TagToken) {
        if self.open_elements.stack_top <= 0 {
            return;
        }

        for idx in (1..=self.open_elements.stack_top as usize).rev() {
            let element = self.open_elements.items[idx].clone();
            let element_id = self.open_elements.tag_ids[idx];

            if token.tag_id == element_id
                && (token.tag_id != TagId::Unknown
                    || element_tag_name(&element).as_deref() == Some(&token.tag_name))
            {
                if let Some(location) = token.location {
                    update_element_end_location(&element, location.location);
                }

                self.generate_implied_end_tags_with_exclusion(token.tag_id);

                if self.open_elements.stack_top >= idx as isize {
                    self.shorten_open_elements_to_length(idx as isize);
                }

                break;
            }

            if self.is_special_element(&element, element_id) {
                break;
            }
        }
    }

    pub(in crate::parser) fn math_start_tag_in_body(&mut self, mut token: TagToken) {
        self.reconstruct_active_formatting_elements();
        foreign_content::adjust_token_mathml_attrs(&mut token);
        foreign_content::adjust_token_xml_attrs(&mut token);

        if token.self_closing {
            self.append_element(token, NS_MATHML);
        } else {
            self.insert_element(token, NS_MATHML);
        }
    }

    pub(in crate::parser) fn svg_start_tag_in_body(&mut self, mut token: TagToken) {
        self.reconstruct_active_formatting_elements();
        foreign_content::adjust_token_svg_attrs(&mut token);
        foreign_content::adjust_token_xml_attrs(&mut token);

        if token.self_closing {
            self.append_element(token, NS_SVG);
        } else {
            self.insert_element(token, NS_SVG);
        }
    }
}
