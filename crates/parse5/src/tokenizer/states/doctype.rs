use super::Tokenizer;
use super::*;

impl Tokenizer {
    pub(in crate::tokenizer) fn state_doctype(&mut self, code_point: i32) {
        match code_point {
            cp::SPACE | cp::LINE_FEED | cp::TABULATION | cp::FORM_FEED => {
                self.state = State::BeforeDoctypeName;
            }
            cp::GREATER_THAN_SIGN => {
                self.state = State::BeforeDoctypeName;
                self.state_before_doctype_name(code_point);
            }
            cp::EOF => {
                self.err(ErrorCode::EofInDoctype);
                self.create_doctype_token(None);
                self.doctype_token_mut().force_quirks = true;
                self.emit_current_doctype();
                self.emit_eof_token();
            }
            _ => {
                self.err(ErrorCode::MissingWhitespaceBeforeDoctypeName);
                self.state = State::BeforeDoctypeName;
                self.state_before_doctype_name(code_point);
            }
        }
    }

    pub(in crate::tokenizer) fn state_before_doctype_name(&mut self, code_point: i32) {
        if is_ascii_upper(code_point) {
            self.create_doctype_token(Some(code_point_to_string(to_ascii_lower(code_point))));
            self.state = State::DoctypeName;
            return;
        }

        match code_point {
            cp::SPACE | cp::LINE_FEED | cp::TABULATION | cp::FORM_FEED => {}
            cp::NULL => {
                self.err(ErrorCode::UnexpectedNullCharacter);
                self.create_doctype_token(Some(REPLACEMENT_CHARACTER.to_string()));
                self.state = State::DoctypeName;
            }
            cp::GREATER_THAN_SIGN => {
                self.err(ErrorCode::MissingDoctypeName);
                self.create_doctype_token(None);
                self.doctype_token_mut().force_quirks = true;
                self.emit_current_doctype();
                self.state = State::Data;
            }
            cp::EOF => {
                self.err(ErrorCode::EofInDoctype);
                self.create_doctype_token(None);
                self.doctype_token_mut().force_quirks = true;
                self.emit_current_doctype();
                self.emit_eof_token();
            }
            _ => {
                self.create_doctype_token(Some(code_point_to_string(code_point)));
                self.state = State::DoctypeName;
            }
        }
    }

    pub(in crate::tokenizer) fn state_doctype_name(&mut self, code_point: i32) {
        match code_point {
            cp::SPACE | cp::LINE_FEED | cp::TABULATION | cp::FORM_FEED => {
                self.state = State::AfterDoctypeName;
            }
            cp::GREATER_THAN_SIGN => {
                self.state = State::Data;
                self.emit_current_doctype();
            }
            cp::NULL => {
                self.err(ErrorCode::UnexpectedNullCharacter);
                self.current_doctype_name_mut().push(REPLACEMENT_CHARACTER);
            }
            cp::EOF => {
                self.err(ErrorCode::EofInDoctype);
                self.doctype_token_mut().force_quirks = true;
                self.emit_current_doctype();
                self.emit_eof_token();
            }
            _ => {
                let code_point = if is_ascii_upper(code_point) {
                    to_ascii_lower(code_point)
                } else {
                    code_point
                };
                self.current_doctype_name_mut()
                    .extend(code_point_to_char(code_point));
            }
        }
    }

    pub(in crate::tokenizer) fn state_after_doctype_name(&mut self, code_point: i32) {
        match code_point {
            cp::SPACE | cp::LINE_FEED | cp::TABULATION | cp::FORM_FEED => {}
            cp::GREATER_THAN_SIGN => {
                self.state = State::Data;
                self.emit_current_doctype();
            }
            cp::EOF => {
                self.err(ErrorCode::EofInDoctype);
                self.doctype_token_mut().force_quirks = true;
                self.emit_current_doctype();
                self.emit_eof_token();
            }
            _ => {
                if self.consume_sequence_if_match(seq::PUBLIC, false) {
                    self.state = State::AfterDoctypePublicKeyword;
                } else if self.consume_sequence_if_match(seq::SYSTEM, false) {
                    self.state = State::AfterDoctypeSystemKeyword;
                } else if !self.ensure_hibernation() {
                    self.err(ErrorCode::InvalidCharacterSequenceAfterDoctypeName);
                    self.doctype_token_mut().force_quirks = true;
                    self.state = State::BogusDoctype;
                    self.state_bogus_doctype(code_point);
                }
            }
        }
    }

    pub(in crate::tokenizer) fn state_after_doctype_public_keyword(&mut self, code_point: i32) {
        match code_point {
            cp::SPACE | cp::LINE_FEED | cp::TABULATION | cp::FORM_FEED => {
                self.state = State::BeforeDoctypePublicIdentifier;
            }
            cp::QUOTATION_MARK => {
                self.err(ErrorCode::MissingWhitespaceAfterDoctypePublicKeyword);
                self.doctype_token_mut().public_id = Some(String::new());
                self.state = State::DoctypePublicIdentifierDoubleQuoted;
            }
            cp::APOSTROPHE => {
                self.err(ErrorCode::MissingWhitespaceAfterDoctypePublicKeyword);
                self.doctype_token_mut().public_id = Some(String::new());
                self.state = State::DoctypePublicIdentifierSingleQuoted;
            }
            cp::GREATER_THAN_SIGN => {
                self.err(ErrorCode::MissingDoctypePublicIdentifier);
                self.doctype_token_mut().force_quirks = true;
                self.state = State::Data;
                self.emit_current_doctype();
            }
            cp::EOF => {
                self.err(ErrorCode::EofInDoctype);
                self.doctype_token_mut().force_quirks = true;
                self.emit_current_doctype();
                self.emit_eof_token();
            }
            _ => {
                self.err(ErrorCode::MissingQuoteBeforeDoctypePublicIdentifier);
                self.doctype_token_mut().force_quirks = true;
                self.state = State::BogusDoctype;
                self.state_bogus_doctype(code_point);
            }
        }
    }

    pub(in crate::tokenizer) fn state_before_doctype_public_identifier(&mut self, code_point: i32) {
        match code_point {
            cp::SPACE | cp::LINE_FEED | cp::TABULATION | cp::FORM_FEED => {}
            cp::QUOTATION_MARK => {
                self.doctype_token_mut().public_id = Some(String::new());
                self.state = State::DoctypePublicIdentifierDoubleQuoted;
            }
            cp::APOSTROPHE => {
                self.doctype_token_mut().public_id = Some(String::new());
                self.state = State::DoctypePublicIdentifierSingleQuoted;
            }
            cp::GREATER_THAN_SIGN => {
                self.err(ErrorCode::MissingDoctypePublicIdentifier);
                self.doctype_token_mut().force_quirks = true;
                self.state = State::Data;
                self.emit_current_doctype();
            }
            cp::EOF => {
                self.err(ErrorCode::EofInDoctype);
                self.doctype_token_mut().force_quirks = true;
                self.emit_current_doctype();
                self.emit_eof_token();
            }
            _ => {
                self.err(ErrorCode::MissingQuoteBeforeDoctypePublicIdentifier);
                self.doctype_token_mut().force_quirks = true;
                self.state = State::BogusDoctype;
                self.state_bogus_doctype(code_point);
            }
        }
    }

    pub(in crate::tokenizer) fn state_doctype_public_identifier_double_quoted(
        &mut self,
        code_point: i32,
    ) {
        self.state_doctype_identifier_quoted(code_point, true, true);
    }

    pub(in crate::tokenizer) fn state_doctype_public_identifier_single_quoted(
        &mut self,
        code_point: i32,
    ) {
        self.state_doctype_identifier_quoted(code_point, true, false);
    }

    pub(in crate::tokenizer) fn state_after_doctype_public_identifier(&mut self, code_point: i32) {
        match code_point {
            cp::SPACE | cp::LINE_FEED | cp::TABULATION | cp::FORM_FEED => {
                self.state = State::BetweenDoctypePublicAndSystemIdentifiers;
            }
            cp::GREATER_THAN_SIGN => {
                self.state = State::Data;
                self.emit_current_doctype();
            }
            cp::QUOTATION_MARK => {
                self.err(ErrorCode::MissingWhitespaceBetweenDoctypePublicAndSystemIdentifiers);
                self.doctype_token_mut().system_id = Some(String::new());
                self.state = State::DoctypeSystemIdentifierDoubleQuoted;
            }
            cp::APOSTROPHE => {
                self.err(ErrorCode::MissingWhitespaceBetweenDoctypePublicAndSystemIdentifiers);
                self.doctype_token_mut().system_id = Some(String::new());
                self.state = State::DoctypeSystemIdentifierSingleQuoted;
            }
            cp::EOF => {
                self.err(ErrorCode::EofInDoctype);
                self.doctype_token_mut().force_quirks = true;
                self.emit_current_doctype();
                self.emit_eof_token();
            }
            _ => {
                self.err(ErrorCode::MissingQuoteBeforeDoctypeSystemIdentifier);
                self.doctype_token_mut().force_quirks = true;
                self.state = State::BogusDoctype;
                self.state_bogus_doctype(code_point);
            }
        }
    }

    pub(in crate::tokenizer) fn state_between_doctype_public_and_system_identifiers(
        &mut self,
        code_point: i32,
    ) {
        match code_point {
            cp::SPACE | cp::LINE_FEED | cp::TABULATION | cp::FORM_FEED => {}
            cp::GREATER_THAN_SIGN => {
                self.emit_current_doctype();
                self.state = State::Data;
            }
            cp::QUOTATION_MARK => {
                self.doctype_token_mut().system_id = Some(String::new());
                self.state = State::DoctypeSystemIdentifierDoubleQuoted;
            }
            cp::APOSTROPHE => {
                self.doctype_token_mut().system_id = Some(String::new());
                self.state = State::DoctypeSystemIdentifierSingleQuoted;
            }
            cp::EOF => {
                self.err(ErrorCode::EofInDoctype);
                self.doctype_token_mut().force_quirks = true;
                self.emit_current_doctype();
                self.emit_eof_token();
            }
            _ => {
                self.err(ErrorCode::MissingQuoteBeforeDoctypeSystemIdentifier);
                self.doctype_token_mut().force_quirks = true;
                self.state = State::BogusDoctype;
                self.state_bogus_doctype(code_point);
            }
        }
    }

    pub(in crate::tokenizer) fn state_after_doctype_system_keyword(&mut self, code_point: i32) {
        match code_point {
            cp::SPACE | cp::LINE_FEED | cp::TABULATION | cp::FORM_FEED => {
                self.state = State::BeforeDoctypeSystemIdentifier;
            }
            cp::QUOTATION_MARK => {
                self.err(ErrorCode::MissingWhitespaceAfterDoctypeSystemKeyword);
                self.doctype_token_mut().system_id = Some(String::new());
                self.state = State::DoctypeSystemIdentifierDoubleQuoted;
            }
            cp::APOSTROPHE => {
                self.err(ErrorCode::MissingWhitespaceAfterDoctypeSystemKeyword);
                self.doctype_token_mut().system_id = Some(String::new());
                self.state = State::DoctypeSystemIdentifierSingleQuoted;
            }
            cp::GREATER_THAN_SIGN => {
                self.err(ErrorCode::MissingDoctypeSystemIdentifier);
                self.doctype_token_mut().force_quirks = true;
                self.state = State::Data;
                self.emit_current_doctype();
            }
            cp::EOF => {
                self.err(ErrorCode::EofInDoctype);
                self.doctype_token_mut().force_quirks = true;
                self.emit_current_doctype();
                self.emit_eof_token();
            }
            _ => {
                self.err(ErrorCode::MissingQuoteBeforeDoctypeSystemIdentifier);
                self.doctype_token_mut().force_quirks = true;
                self.state = State::BogusDoctype;
                self.state_bogus_doctype(code_point);
            }
        }
    }

    pub(in crate::tokenizer) fn state_before_doctype_system_identifier(&mut self, code_point: i32) {
        match code_point {
            cp::SPACE | cp::LINE_FEED | cp::TABULATION | cp::FORM_FEED => {}
            cp::QUOTATION_MARK => {
                self.doctype_token_mut().system_id = Some(String::new());
                self.state = State::DoctypeSystemIdentifierDoubleQuoted;
            }
            cp::APOSTROPHE => {
                self.doctype_token_mut().system_id = Some(String::new());
                self.state = State::DoctypeSystemIdentifierSingleQuoted;
            }
            cp::GREATER_THAN_SIGN => {
                self.err(ErrorCode::MissingDoctypeSystemIdentifier);
                self.doctype_token_mut().force_quirks = true;
                self.state = State::Data;
                self.emit_current_doctype();
            }
            cp::EOF => {
                self.err(ErrorCode::EofInDoctype);
                self.doctype_token_mut().force_quirks = true;
                self.emit_current_doctype();
                self.emit_eof_token();
            }
            _ => {
                self.err(ErrorCode::MissingQuoteBeforeDoctypeSystemIdentifier);
                self.doctype_token_mut().force_quirks = true;
                self.state = State::BogusDoctype;
                self.state_bogus_doctype(code_point);
            }
        }
    }

    pub(in crate::tokenizer) fn state_doctype_system_identifier_double_quoted(
        &mut self,
        code_point: i32,
    ) {
        self.state_doctype_identifier_quoted(code_point, false, true);
    }

    pub(in crate::tokenizer) fn state_doctype_system_identifier_single_quoted(
        &mut self,
        code_point: i32,
    ) {
        self.state_doctype_identifier_quoted(code_point, false, false);
    }

    pub(in crate::tokenizer) fn state_after_doctype_system_identifier(&mut self, code_point: i32) {
        match code_point {
            cp::SPACE | cp::LINE_FEED | cp::TABULATION | cp::FORM_FEED => {}
            cp::GREATER_THAN_SIGN => {
                self.emit_current_doctype();
                self.state = State::Data;
            }
            cp::EOF => {
                self.err(ErrorCode::EofInDoctype);
                self.doctype_token_mut().force_quirks = true;
                self.emit_current_doctype();
                self.emit_eof_token();
            }
            _ => {
                self.err(ErrorCode::UnexpectedCharacterAfterDoctypeSystemIdentifier);
                self.state = State::BogusDoctype;
                self.state_bogus_doctype(code_point);
            }
        }
    }

    pub(in crate::tokenizer) fn state_doctype_identifier_quoted(
        &mut self,
        code_point: i32,
        is_public: bool,
        double_quoted: bool,
    ) {
        let quote = if double_quoted {
            cp::QUOTATION_MARK
        } else {
            cp::APOSTROPHE
        };

        if code_point == quote {
            self.state = if is_public {
                State::AfterDoctypePublicIdentifier
            } else {
                State::AfterDoctypeSystemIdentifier
            };
            return;
        }

        match code_point {
            cp::NULL => {
                self.err(ErrorCode::UnexpectedNullCharacter);
                if is_public {
                    self.current_doctype_public_id_mut()
                        .push(REPLACEMENT_CHARACTER);
                } else {
                    self.current_doctype_system_id_mut()
                        .push(REPLACEMENT_CHARACTER);
                }
            }
            cp::GREATER_THAN_SIGN => {
                self.err(if is_public {
                    ErrorCode::AbruptDoctypePublicIdentifier
                } else {
                    ErrorCode::AbruptDoctypeSystemIdentifier
                });
                self.doctype_token_mut().force_quirks = true;
                self.emit_current_doctype();
                self.state = State::Data;
            }
            cp::EOF => {
                self.err(ErrorCode::EofInDoctype);
                self.doctype_token_mut().force_quirks = true;
                self.emit_current_doctype();
                self.emit_eof_token();
            }
            _ => {
                if is_public {
                    self.current_doctype_public_id_mut()
                        .extend(code_point_to_char(code_point));
                } else {
                    self.current_doctype_system_id_mut()
                        .extend(code_point_to_char(code_point));
                }
            }
        }
    }
}
