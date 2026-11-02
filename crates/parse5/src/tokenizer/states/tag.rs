use super::Tokenizer;
use super::*;

impl Tokenizer {
    pub(in crate::tokenizer) fn state_tag_open(&mut self, code_point: i32) {
        if is_ascii_letter(code_point) {
            self.create_start_tag_token();
            self.state = State::TagName;
            self.state_tag_name(code_point);
            return;
        }

        match code_point {
            cp::EXCLAMATION_MARK => self.state = State::MarkupDeclarationOpen,
            cp::SOLIDUS => self.state = State::EndTagOpen,
            cp::QUESTION_MARK => {
                self.err(ErrorCode::UnexpectedQuestionMarkInsteadOfTagName);
                self.create_comment_token(1);
                self.state = State::BogusComment;
                self.state_bogus_comment(code_point);
            }
            cp::EOF => {
                self.err(ErrorCode::EofBeforeTagName);
                self.emit_chars("<");
                self.emit_eof_token();
            }
            _ => {
                self.err(ErrorCode::InvalidFirstCharacterOfTagName);
                self.emit_chars("<");
                self.state = State::Data;
                self.state_data(code_point);
            }
        }
    }

    pub(in crate::tokenizer) fn state_end_tag_open(&mut self, code_point: i32) {
        if is_ascii_letter(code_point) {
            self.create_end_tag_token();
            self.state = State::TagName;
            self.state_tag_name(code_point);
            return;
        }

        match code_point {
            cp::GREATER_THAN_SIGN => {
                self.err(ErrorCode::MissingEndTagName);
                self.state = State::Data;
            }
            cp::EOF => {
                self.err(ErrorCode::EofBeforeTagName);
                self.emit_chars("</");
                self.emit_eof_token();
            }
            _ => {
                self.err(ErrorCode::InvalidFirstCharacterOfTagName);
                self.create_comment_token(2);
                self.state = State::BogusComment;
                self.state_bogus_comment(code_point);
            }
        }
    }

    pub(in crate::tokenizer) fn state_tag_name(&mut self, code_point: i32) {
        match code_point {
            cp::SPACE | cp::LINE_FEED | cp::TABULATION | cp::FORM_FEED => {
                self.state = State::BeforeAttributeName;
            }
            cp::SOLIDUS => self.state = State::SelfClosingStartTag,
            cp::GREATER_THAN_SIGN => {
                self.state = State::Data;
                self.emit_current_tag_token();
            }
            cp::NULL => {
                self.err(ErrorCode::UnexpectedNullCharacter);
                self.tag_token_mut().tag_name.push(REPLACEMENT_CHARACTER);
            }
            cp::EOF => {
                self.err(ErrorCode::EofInTag);
                self.emit_eof_token();
            }
            _ => {
                let code_point = if is_ascii_upper(code_point) {
                    to_ascii_lower(code_point)
                } else {
                    code_point
                };
                self.tag_token_mut()
                    .tag_name
                    .extend(code_point_to_char(code_point));
            }
        }
    }

    pub(in crate::tokenizer) fn state_rcdata_less_than_sign(&mut self, code_point: i32) {
        if code_point == cp::SOLIDUS {
            self.state = State::RcDataEndTagOpen;
        } else {
            self.emit_chars("<");
            self.state = State::RcData;
            self.state_rcdata(code_point);
        }
    }

    pub(in crate::tokenizer) fn state_rcdata_end_tag_open(&mut self, code_point: i32) {
        if is_ascii_letter(code_point) {
            self.state = State::RcDataEndTagName;
            self.state_rcdata_end_tag_name(code_point);
        } else {
            self.emit_chars("</");
            self.state = State::RcData;
            self.state_rcdata(code_point);
        }
    }

    pub(in crate::tokenizer) fn state_rcdata_end_tag_name(&mut self, code_point: i32) {
        if self.handle_special_end_tag(State::RcData) {
            self.emit_chars("</");
            self.state = State::RcData;
            self.state_rcdata(code_point);
        }
    }

    pub(in crate::tokenizer) fn state_rawtext_less_than_sign(&mut self, code_point: i32) {
        if code_point == cp::SOLIDUS {
            self.state = State::RawTextEndTagOpen;
        } else {
            self.emit_chars("<");
            self.state = State::RawText;
            self.state_rawtext(code_point);
        }
    }

    pub(in crate::tokenizer) fn state_rawtext_end_tag_open(&mut self, code_point: i32) {
        if is_ascii_letter(code_point) {
            self.state = State::RawTextEndTagName;
            self.state_rawtext_end_tag_name(code_point);
        } else {
            self.emit_chars("</");
            self.state = State::RawText;
            self.state_rawtext(code_point);
        }
    }

    pub(in crate::tokenizer) fn state_rawtext_end_tag_name(&mut self, code_point: i32) {
        if self.handle_special_end_tag(State::RawText) {
            self.emit_chars("</");
            self.state = State::RawText;
            self.state_rawtext(code_point);
        }
    }

    pub(in crate::tokenizer) fn handle_special_end_tag(&mut self, _text_state: State) -> bool {
        if self.last_start_tag_name.is_empty()
            || !self
                .preprocessor
                .starts_with(&self.last_start_tag_name, false)
        {
            return !self.ensure_hibernation();
        }

        let tag_name = self.last_start_tag_name.clone();
        self.create_end_tag_token();
        self.tag_token_mut().tag_name = tag_name.clone();
        let peek = self
            .preprocessor
            .peek(tag_name.encode_utf16().count() as isize);

        match peek {
            cp::SPACE | cp::LINE_FEED | cp::TABULATION | cp::FORM_FEED => {
                self.advance_by(tag_name.encode_utf16().count());
                self.state = State::BeforeAttributeName;
                false
            }
            cp::SOLIDUS => {
                self.advance_by(tag_name.encode_utf16().count());
                self.state = State::SelfClosingStartTag;
                false
            }
            cp::GREATER_THAN_SIGN => {
                self.advance_by(tag_name.encode_utf16().count());
                self.emit_current_tag_token();
                self.state = State::Data;
                false
            }
            _ => !self.ensure_hibernation(),
        }
    }
}
