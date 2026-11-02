use super::Tokenizer;
use super::*;

impl Tokenizer {
    pub(in crate::tokenizer) fn state_before_attribute_name(&mut self, code_point: i32) {
        match code_point {
            cp::SPACE | cp::LINE_FEED | cp::TABULATION | cp::FORM_FEED => {}
            cp::SOLIDUS | cp::GREATER_THAN_SIGN | cp::EOF => {
                self.state = State::AfterAttributeName;
                self.state_after_attribute_name(code_point);
            }
            cp::EQUALS_SIGN => {
                self.err(ErrorCode::UnexpectedEqualsSignBeforeAttributeName);
                self.create_attr("=");
                self.state = State::AttributeName;
            }
            _ => {
                self.create_attr("");
                self.state = State::AttributeName;
                self.state_attribute_name(code_point);
            }
        }
    }

    pub(in crate::tokenizer) fn state_attribute_name(&mut self, code_point: i32) {
        match code_point {
            cp::SPACE
            | cp::LINE_FEED
            | cp::TABULATION
            | cp::FORM_FEED
            | cp::SOLIDUS
            | cp::GREATER_THAN_SIGN
            | cp::EOF => {
                self.leave_attr_name();
                self.state = State::AfterAttributeName;
                self.state_after_attribute_name(code_point);
            }
            cp::EQUALS_SIGN => {
                self.leave_attr_name();
                self.state = State::BeforeAttributeValue;
            }
            cp::QUOTATION_MARK | cp::APOSTROPHE | cp::LESS_THAN_SIGN => {
                self.err(ErrorCode::UnexpectedCharacterInAttributeName);
                self.current_attr
                    .name
                    .extend(code_point_to_char(code_point));
            }
            cp::NULL => {
                self.err(ErrorCode::UnexpectedNullCharacter);
                self.current_attr.name.push(REPLACEMENT_CHARACTER);
            }
            _ => {
                let code_point = if is_ascii_upper(code_point) {
                    to_ascii_lower(code_point)
                } else {
                    code_point
                };
                self.current_attr
                    .name
                    .extend(code_point_to_char(code_point));
            }
        }
    }

    pub(in crate::tokenizer) fn state_after_attribute_name(&mut self, code_point: i32) {
        match code_point {
            cp::SPACE | cp::LINE_FEED | cp::TABULATION | cp::FORM_FEED => {}
            cp::SOLIDUS => self.state = State::SelfClosingStartTag,
            cp::EQUALS_SIGN => self.state = State::BeforeAttributeValue,
            cp::GREATER_THAN_SIGN => {
                self.state = State::Data;
                self.emit_current_tag_token();
            }
            cp::EOF => {
                self.err(ErrorCode::EofInTag);
                self.emit_eof_token();
            }
            _ => {
                self.create_attr("");
                self.state = State::AttributeName;
                self.state_attribute_name(code_point);
            }
        }
    }

    pub(in crate::tokenizer) fn state_before_attribute_value(&mut self, code_point: i32) {
        match code_point {
            cp::SPACE | cp::LINE_FEED | cp::TABULATION | cp::FORM_FEED => {}
            cp::QUOTATION_MARK => self.state = State::AttributeValueDoubleQuoted,
            cp::APOSTROPHE => self.state = State::AttributeValueSingleQuoted,
            cp::GREATER_THAN_SIGN => {
                self.err(ErrorCode::MissingAttributeValue);
                self.state = State::Data;
                self.emit_current_tag_token();
            }
            _ => {
                self.state = State::AttributeValueUnquoted;
                self.state_attribute_value_unquoted(code_point);
            }
        }
    }

    pub(in crate::tokenizer) fn state_attribute_value_double_quoted(&mut self, code_point: i32) {
        match code_point {
            cp::QUOTATION_MARK => self.state = State::AfterAttributeValueQuoted,
            cp::AMPERSAND => self.consume_character_reference(true),
            cp::NULL => {
                self.err(ErrorCode::UnexpectedNullCharacter);
                self.current_attr.value.push(REPLACEMENT_CHARACTER);
            }
            cp::EOF => {
                self.err(ErrorCode::EofInTag);
                self.emit_eof_token();
            }
            _ => self
                .current_attr
                .value
                .extend(code_point_to_char(code_point)),
        }
    }

    pub(in crate::tokenizer) fn state_attribute_value_single_quoted(&mut self, code_point: i32) {
        match code_point {
            cp::APOSTROPHE => self.state = State::AfterAttributeValueQuoted,
            cp::AMPERSAND => self.consume_character_reference(true),
            cp::NULL => {
                self.err(ErrorCode::UnexpectedNullCharacter);
                self.current_attr.value.push(REPLACEMENT_CHARACTER);
            }
            cp::EOF => {
                self.err(ErrorCode::EofInTag);
                self.emit_eof_token();
            }
            _ => self
                .current_attr
                .value
                .extend(code_point_to_char(code_point)),
        }
    }

    pub(in crate::tokenizer) fn state_attribute_value_unquoted(&mut self, code_point: i32) {
        match code_point {
            cp::SPACE | cp::LINE_FEED | cp::TABULATION | cp::FORM_FEED => {
                self.leave_attr_value();
                self.state = State::BeforeAttributeName;
            }
            cp::AMPERSAND => self.consume_character_reference(true),
            cp::GREATER_THAN_SIGN => {
                self.leave_attr_value();
                self.state = State::Data;
                self.emit_current_tag_token();
            }
            cp::NULL => {
                self.err(ErrorCode::UnexpectedNullCharacter);
                self.current_attr.value.push(REPLACEMENT_CHARACTER);
            }
            cp::QUOTATION_MARK
            | cp::APOSTROPHE
            | cp::LESS_THAN_SIGN
            | cp::EQUALS_SIGN
            | cp::GRAVE_ACCENT => {
                self.err(ErrorCode::UnexpectedCharacterInUnquotedAttributeValue);
                self.current_attr
                    .value
                    .extend(code_point_to_char(code_point));
            }
            cp::EOF => {
                self.err(ErrorCode::EofInTag);
                self.emit_eof_token();
            }
            _ => self
                .current_attr
                .value
                .extend(code_point_to_char(code_point)),
        }
    }

    pub(in crate::tokenizer) fn state_after_attribute_value_quoted(&mut self, code_point: i32) {
        match code_point {
            cp::SPACE | cp::LINE_FEED | cp::TABULATION | cp::FORM_FEED => {
                self.leave_attr_value();
                self.state = State::BeforeAttributeName;
            }
            cp::SOLIDUS => {
                self.leave_attr_value();
                self.state = State::SelfClosingStartTag;
            }
            cp::GREATER_THAN_SIGN => {
                self.leave_attr_value();
                self.state = State::Data;
                self.emit_current_tag_token();
            }
            cp::EOF => {
                self.err(ErrorCode::EofInTag);
                self.emit_eof_token();
            }
            _ => {
                self.err(ErrorCode::MissingWhitespaceBetweenAttributes);
                // Upstream does not call `_leaveAttrValue` here, so the span
                // keeps the end recorded when the attribute name ended.
                self.sync_committed_attr();
                self.state = State::BeforeAttributeName;
                self.state_before_attribute_name(code_point);
            }
        }
    }

    pub(in crate::tokenizer) fn state_self_closing_start_tag(&mut self, code_point: i32) {
        match code_point {
            cp::GREATER_THAN_SIGN => {
                self.tag_token_mut().self_closing = true;
                self.state = State::Data;
                self.emit_current_tag_token();
            }
            cp::EOF => {
                self.err(ErrorCode::EofInTag);
                self.emit_eof_token();
            }
            _ => {
                self.err(ErrorCode::UnexpectedSolidusInTag);
                self.state = State::BeforeAttributeName;
                self.state_before_attribute_name(code_point);
            }
        }
    }
}
