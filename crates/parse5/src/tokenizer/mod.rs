//! Tokenizer port scaffold.
//!
//! The TypeScript tokenizer lives upstream at `packages/parse5/lib/tokenizer`.
//! This module mirrors its state names and low-level helpers before the full
//! state machine is wired into `Tokenizer`.

use crate::common::error_codes::{ErrorCode, ParserError};
use crate::common::html::{get_tag_id, TagId};
use crate::common::token::{
    get_token_attr, Attribute, CharacterToken, CommentToken, DoctypeToken, EofToken, Location,
    LocationWithAttributes, TagToken, Token, TokenType,
};
use crate::common::unicode::{
    code_points as cp, is_control_code_point, is_surrogate, is_undefined_code_point,
    sequences as seq, REPLACEMENT_CHARACTER,
};
use preprocessor::Preprocessor;
use std::collections::BTreeMap;

pub mod preprocessor;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum State {
    Data = 0,
    RcData,
    RawText,
    ScriptData,
    PlainText,
    TagOpen,
    EndTagOpen,
    TagName,
    RcDataLessThanSign,
    RcDataEndTagOpen,
    RcDataEndTagName,
    RawTextLessThanSign,
    RawTextEndTagOpen,
    RawTextEndTagName,
    ScriptDataLessThanSign,
    ScriptDataEndTagOpen,
    ScriptDataEndTagName,
    ScriptDataEscapeStart,
    ScriptDataEscapeStartDash,
    ScriptDataEscaped,
    ScriptDataEscapedDash,
    ScriptDataEscapedDashDash,
    ScriptDataEscapedLessThanSign,
    ScriptDataEscapedEndTagOpen,
    ScriptDataEscapedEndTagName,
    ScriptDataDoubleEscapeStart,
    ScriptDataDoubleEscaped,
    ScriptDataDoubleEscapedDash,
    ScriptDataDoubleEscapedDashDash,
    ScriptDataDoubleEscapedLessThanSign,
    ScriptDataDoubleEscapeEnd,
    BeforeAttributeName,
    AttributeName,
    AfterAttributeName,
    BeforeAttributeValue,
    AttributeValueDoubleQuoted,
    AttributeValueSingleQuoted,
    AttributeValueUnquoted,
    AfterAttributeValueQuoted,
    SelfClosingStartTag,
    BogusComment,
    MarkupDeclarationOpen,
    CommentStart,
    CommentStartDash,
    Comment,
    CommentLessThanSign,
    CommentLessThanSignBang,
    CommentLessThanSignBangDash,
    CommentLessThanSignBangDashDash,
    CommentEndDash,
    CommentEnd,
    CommentEndBang,
    Doctype,
    BeforeDoctypeName,
    DoctypeName,
    AfterDoctypeName,
    AfterDoctypePublicKeyword,
    BeforeDoctypePublicIdentifier,
    DoctypePublicIdentifierDoubleQuoted,
    DoctypePublicIdentifierSingleQuoted,
    AfterDoctypePublicIdentifier,
    BetweenDoctypePublicAndSystemIdentifiers,
    AfterDoctypeSystemKeyword,
    BeforeDoctypeSystemIdentifier,
    DoctypeSystemIdentifierDoubleQuoted,
    DoctypeSystemIdentifierSingleQuoted,
    AfterDoctypeSystemIdentifier,
    BogusDoctype,
    CDataSection,
    CDataSectionBracket,
    CDataSectionEnd,
    CharacterReference,
    AmbiguousAmpersand,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TokenizerMode {
    Data,
    RcData,
    RawText,
    ScriptData,
    PlainText,
    CDataSection,
}

impl TokenizerMode {
    pub fn state(self) -> State {
        match self {
            Self::Data => State::Data,
            Self::RcData => State::RcData,
            Self::RawText => State::RawText,
            Self::ScriptData => State::ScriptData,
            Self::PlainText => State::PlainText,
            Self::CDataSection => State::CDataSection,
        }
    }
}

#[allow(dead_code)]
pub(crate) fn is_ascii_digit(cp: i32) -> bool {
    (cp::DIGIT_0..=cp::DIGIT_9).contains(&cp)
}

#[allow(dead_code)]
pub(crate) fn is_ascii_upper(cp: i32) -> bool {
    (cp::LATIN_CAPITAL_A..=cp::LATIN_CAPITAL_Z).contains(&cp)
}

#[allow(dead_code)]
pub(crate) fn is_ascii_lower(cp: i32) -> bool {
    (cp::LATIN_SMALL_A..=cp::LATIN_SMALL_Z).contains(&cp)
}

#[allow(dead_code)]
pub(crate) fn is_ascii_letter(cp: i32) -> bool {
    is_ascii_lower(cp) || is_ascii_upper(cp)
}

#[allow(dead_code)]
pub(crate) fn is_ascii_alpha_numeric(cp: i32) -> bool {
    is_ascii_letter(cp) || is_ascii_digit(cp)
}

#[allow(dead_code)]
pub(crate) fn to_ascii_lower(cp: i32) -> i32 {
    cp + 0x20
}

#[allow(dead_code)]
pub(crate) fn is_whitespace(cp: i32) -> bool {
    matches!(
        cp,
        cp::SPACE | cp::LINE_FEED | cp::TABULATION | cp::FORM_FEED
    )
}

#[allow(dead_code)]
pub(crate) fn is_script_data_double_escape_sequence_end(cp: i32) -> bool {
    is_whitespace(cp) || matches!(cp, cp::SOLIDUS | cp::GREATER_THAN_SIGN)
}

#[allow(dead_code)]
pub(crate) fn get_error_for_numeric_character_reference(code: u32) -> Option<ErrorCode> {
    if code == cp::NULL as u32 {
        Some(ErrorCode::NullCharacterReference)
    } else if code > 0x10ffff {
        Some(ErrorCode::CharacterReferenceOutsideUnicodeRange)
    } else if is_surrogate(code) {
        Some(ErrorCode::SurrogateCharacterReference)
    } else if is_undefined_code_point(code) {
        Some(ErrorCode::NoncharacterCharacterReference)
    } else if is_control_code_point(code) || code == cp::CARRIAGE_RETURN as u32 {
        Some(ErrorCode::ControlCharacterReference)
    } else {
        None
    }
}

fn replacement_for_numeric_character_reference(code: u32) -> u32 {
    match code {
        0x00 | 0xd800..=0xdfff | 0x110000..=u32::MAX => 0xfffd,
        0x80 => 0x20ac,
        0x82 => 0x201a,
        0x83 => 0x0192,
        0x84 => 0x201e,
        0x85 => 0x2026,
        0x86 => 0x2020,
        0x87 => 0x2021,
        0x88 => 0x02c6,
        0x89 => 0x2030,
        0x8a => 0x0160,
        0x8b => 0x2039,
        0x8c => 0x0152,
        0x8e => 0x017d,
        0x91 => 0x2018,
        0x92 => 0x2019,
        0x93 => 0x201c,
        0x94 => 0x201d,
        0x95 => 0x2022,
        0x96 => 0x2013,
        0x97 => 0x2014,
        0x98 => 0x02dc,
        0x99 => 0x2122,
        0x9a => 0x0161,
        0x9b => 0x203a,
        0x9c => 0x0153,
        0x9e => 0x017e,
        0x9f => 0x0178,
        _ => code,
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct TokenizerOptions {
    pub source_code_location_info: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TokenizerOutput {
    pub tokens: Vec<Token>,
    pub errors: Vec<ParserError>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum CurrentToken {
    Tag(TagToken),
    Comment(CommentToken),
    Doctype(DoctypeToken),
}

#[derive(Debug)]
pub struct Tokenizer {
    pub preprocessor: Preprocessor,
    pub in_foreign_node: bool,
    pub last_start_tag_name: String,
    pub active: bool,
    pub state: State,
    return_state: State,
    options: TokenizerOptions,
    current_location: Option<Location>,
    current_character_token: Option<CharacterToken>,
    current_token: Option<CurrentToken>,
    current_attr: Attribute,
    current_attr_location: Option<Location>,
    current_attr_committed_name: Option<String>,
    tokens: Vec<Token>,
    errors: Vec<ParserError>,
    consumed_after_snapshot: isize,
    pause_on_token: bool,
    paused_on_token: bool,
}

impl Tokenizer {
    pub fn new(options: TokenizerOptions) -> Self {
        let preprocessor = Preprocessor::streaming();
        let current_location = current_location_from(&preprocessor, options, -1);

        Self {
            preprocessor,
            in_foreign_node: false,
            last_start_tag_name: String::new(),
            active: false,
            state: State::Data,
            return_state: State::Data,
            options,
            current_location,
            current_character_token: None,
            current_token: None,
            current_attr: Attribute::new("", ""),
            current_attr_location: None,
            current_attr_committed_name: None,
            tokens: Vec::new(),
            errors: Vec::new(),
            consumed_after_snapshot: 0,
            pause_on_token: false,
            paused_on_token: false,
        }
    }

    pub fn tokenize(html: &str, options: TokenizerOptions) -> TokenizerOutput {
        let mut tokenizer = Self::new(options);
        tokenizer.write(html, true);
        tokenizer.finish()
    }

    pub fn write(&mut self, chunk: &str, is_last_chunk: bool) {
        self.active = true;
        self.preprocessor.write(chunk, is_last_chunk);
        self.run_parsing_loop();
    }

    pub fn write_utf16(&mut self, chunk: &[u16], is_last_chunk: bool) {
        self.active = true;
        self.preprocessor.write_utf16(chunk, is_last_chunk);
        self.run_parsing_loop();
    }

    pub fn write_input(&mut self, chunk: &str, is_last_chunk: bool) {
        self.active = true;
        self.preprocessor.write(chunk, is_last_chunk);
    }

    pub fn next_token(&mut self) -> Option<Token> {
        if !self.tokens.is_empty() {
            return Some(self.tokens.remove(0));
        }

        if !self.active {
            if self.paused_on_token {
                self.paused_on_token = false;
                self.active = true;
            } else {
                return None;
            }
        }

        self.pause_on_token = true;
        self.run_parsing_loop();
        self.pause_on_token = false;

        if self.tokens.is_empty() {
            None
        } else {
            Some(self.tokens.remove(0))
        }
    }

    pub fn insert_html_at_current_pos(&mut self, chunk: &str) {
        self.active = true;
        self.preprocessor.insert_html_at_current_pos(chunk);
        self.run_parsing_loop();
    }

    pub fn finish(mut self) -> TokenizerOutput {
        self.errors.extend(self.preprocessor.take_errors());
        TokenizerOutput {
            tokens: self.tokens,
            errors: self.errors,
        }
    }

    pub fn take_errors(&mut self) -> Vec<ParserError> {
        self.errors.extend(self.preprocessor.take_errors());
        std::mem::take(&mut self.errors)
    }

    fn run_parsing_loop(&mut self) {
        while self.active {
            self.consumed_after_snapshot = 0;
            let code_point = self.consume();

            if !self.ensure_hibernation() {
                self.call_state(code_point);
            }
        }
    }

    fn ensure_hibernation(&mut self) -> bool {
        if self.preprocessor.end_of_chunk_hit {
            self.preprocessor.retreat(self.consumed_after_snapshot);
            self.consumed_after_snapshot = 0;
            self.active = false;
            true
        } else {
            false
        }
    }

    fn consume(&mut self) -> i32 {
        self.consumed_after_snapshot += 1;
        self.preprocessor.advance()
    }

    fn advance_by(&mut self, count: usize) {
        self.consumed_after_snapshot += count as isize;

        for _ in 0..count {
            self.preprocessor.advance();
        }
    }

    fn consume_sequence_if_match(&mut self, pattern: &str, case_sensitive: bool) -> bool {
        if self.preprocessor.starts_with(pattern, case_sensitive) {
            self.advance_by(pattern.encode_utf16().count().saturating_sub(1));
            true
        } else {
            false
        }
    }

    fn err(&mut self, code: ErrorCode) {
        self.err_with_offset(code, 0);
    }

    fn err_with_offset(&mut self, code: ErrorCode, cp_offset: isize) {
        self.errors
            .push(self.preprocessor.get_error(code, cp_offset));
    }

    fn get_current_location(&self, offset: isize) -> Option<Location> {
        current_location_from(&self.preprocessor, self.options, offset)
    }

    fn create_start_tag_token(&mut self) {
        self.current_attr_committed_name = None;
        self.current_token = Some(CurrentToken::Tag(TagToken {
            token_type: TokenType::StartTag,
            tag_name: String::new(),
            tag_id: TagId::Unknown,
            self_closing: false,
            ack_self_closing: false,
            attrs: Vec::new(),
            location: self
                .get_current_location(1)
                .map(|location| LocationWithAttributes {
                    location,
                    attrs: BTreeMap::new(),
                }),
        }));
    }

    fn create_end_tag_token(&mut self) {
        self.current_attr_committed_name = None;
        self.current_token = Some(CurrentToken::Tag(TagToken {
            token_type: TokenType::EndTag,
            tag_name: String::new(),
            tag_id: TagId::Unknown,
            self_closing: false,
            ack_self_closing: false,
            attrs: Vec::new(),
            location: self
                .get_current_location(2)
                .map(|location| LocationWithAttributes {
                    location,
                    attrs: BTreeMap::new(),
                }),
        }));
    }

    fn create_comment_token(&mut self, offset: isize) {
        self.current_token = Some(CurrentToken::Comment(CommentToken {
            token_type: TokenType::Comment,
            data: String::new(),
            location: self.get_current_location(offset),
        }));
    }

    fn create_doctype_token(&mut self, initial_name: Option<String>) {
        self.current_token = Some(CurrentToken::Doctype(DoctypeToken {
            token_type: TokenType::Doctype,
            name: initial_name,
            force_quirks: false,
            public_id: None,
            system_id: None,
            location: self.current_location,
        }));
    }

    fn create_character_token(&mut self, token_type: TokenType, chars: String) {
        let raw_chars = chars.encode_utf16().collect();
        self.current_character_token = Some(CharacterToken {
            token_type,
            chars,
            raw_chars,
            location: self.current_location,
        });
    }

    fn create_attr(&mut self, attr_name_first_ch: &str) {
        self.current_attr = Attribute::new(attr_name_first_ch, "");
        self.current_attr_location = self.get_current_location(0);
        self.current_attr_committed_name = None;
    }

    fn leave_attr_name(&mut self) {
        let duplicate = match &self.current_token {
            Some(CurrentToken::Tag(token)) => {
                get_token_attr(token, &self.current_attr.name).is_some()
            }
            _ => false,
        };

        if duplicate {
            self.current_attr_committed_name = None;
            self.err(ErrorCode::DuplicateAttribute);
            return;
        }

        let attr = self.current_attr.clone();
        self.current_attr_committed_name = Some(attr.name.clone());

        if let Some(CurrentToken::Tag(token)) = &mut self.current_token {
            token.attrs.push(attr);
        }

        self.leave_attr_value();
    }

    fn leave_attr_value(&mut self) {
        let Some(attr_name) = self.current_attr_committed_name.clone() else {
            return;
        };

        if let Some(attr_location) = &mut self.current_attr_location {
            attr_location.end_line = self.preprocessor.line;
            attr_location.end_col = self.preprocessor.col();
            attr_location.end_offset = self.preprocessor.offset();
        }

        if let Some(CurrentToken::Tag(token)) = &mut self.current_token {
            let attr_exists = if let Some(attr) = token
                .attrs
                .iter_mut()
                .rev()
                .find(|attr| attr.name == attr_name)
            {
                attr.value = self.current_attr.value.clone();
                true
            } else {
                false
            };

            if attr_exists {
                if let (Some(token_location), Some(attr_location)) =
                    (&mut token.location, self.current_attr_location)
                {
                    token_location.attrs.insert(attr_name, attr_location);
                }
            }
        }
    }

    fn prepare_location(&mut self, location: Option<Location>) {
        self.emit_current_character_token(location);
        self.current_token = None;
        self.current_location = self.get_current_location(-1);
    }

    fn finish_location(&self, location: &mut Location) {
        location.end_line = self.preprocessor.line;
        location.end_col = self.preprocessor.col() + 1;
        location.end_offset = self.preprocessor.offset() + 1;
    }

    fn emit_current_tag_token(&mut self) {
        self.leave_attr_value();
        let Some(CurrentToken::Tag(mut token)) = self.current_token.take() else {
            return;
        };
        let start_location = token.location.as_ref().map(|location| location.location);
        self.prepare_location(start_location);

        if let Some(location) = &mut token.location {
            self.finish_location(&mut location.location);
        }

        token.tag_id = get_tag_id(&token.tag_name);

        if token.token_type == TokenType::StartTag {
            self.last_start_tag_name = token.tag_name.clone();
            self.emit_token(Token::Tag(token));
        } else {
            if !token.attrs.is_empty() {
                self.err(ErrorCode::EndTagWithAttributes);
            }
            if token.self_closing {
                self.err(ErrorCode::EndTagWithTrailingSolidus);
            }
            self.emit_token(Token::Tag(token));
        }

        self.preprocessor.drop_parsed_chunk();
    }

    fn emit_current_comment(&mut self) {
        let Some(CurrentToken::Comment(mut token)) = self.current_token.take() else {
            return;
        };
        let start_location = token.location;
        self.prepare_location(start_location);

        if let Some(location) = &mut token.location {
            self.finish_location(location);
        }

        self.emit_token(Token::Comment(token));
        self.preprocessor.drop_parsed_chunk();
    }

    fn emit_current_doctype(&mut self) {
        let Some(CurrentToken::Doctype(mut token)) = self.current_token.take() else {
            return;
        };
        let start_location = token.location;
        self.prepare_location(start_location);

        if let Some(location) = &mut token.location {
            self.finish_location(location);
        }

        self.emit_token(Token::Doctype(token));
        self.preprocessor.drop_parsed_chunk();
    }

    fn emit_current_character_token(&mut self, next_location: Option<Location>) {
        let Some(mut token) = self.current_character_token.take() else {
            return;
        };

        if let (Some(next_location), Some(token_location)) = (next_location, &mut token.location) {
            token_location.end_line = next_location.start_line;
            token_location.end_col = next_location.start_col;
            token_location.end_offset = next_location.start_offset;
        }

        self.emit_token(Token::Character(token));
    }

    fn emit_eof_token(&mut self) {
        let mut location = self.get_current_location(0);

        if let Some(location) = &mut location {
            location.end_line = location.start_line;
            location.end_col = location.start_col;
            location.end_offset = location.start_offset;
        }

        let pause_on_token = self.pause_on_token;
        self.pause_on_token = false;
        self.emit_current_character_token(location);
        self.tokens.push(Token::Eof(EofToken {
            token_type: TokenType::Eof,
            location,
        }));
        self.pause_on_token = pause_on_token;
        self.active = false;
        self.paused_on_token = false;
    }

    fn emit_token(&mut self, token: Token) {
        self.tokens.push(token);

        if self.pause_on_token && self.active {
            self.paused_on_token = true;
            self.active = false;
        }
    }

    fn append_char_to_current_character_token(
        &mut self,
        token_type: TokenType,
        value: &str,
        raw_value: &[u16],
    ) {
        if let Some(current) = &mut self.current_character_token {
            if current.token_type == token_type {
                current.chars.push_str(value);
                current.raw_chars.extend_from_slice(raw_value);
                return;
            }

            self.current_location = self.get_current_location(0);
            self.emit_current_character_token(self.current_location);
            self.preprocessor.drop_parsed_chunk();
        }

        self.create_character_token(token_type, value.to_string());
        if let Some(current) = &mut self.current_character_token {
            current.raw_chars.clear();
            current.raw_chars.extend_from_slice(raw_value);
        }
    }

    fn emit_code_point(&mut self, code_point: i32) {
        let token_type = if is_whitespace(code_point) {
            TokenType::WhitespaceCharacter
        } else if code_point == cp::NULL {
            TokenType::NullCharacter
        } else {
            TokenType::Character
        };
        let value = code_point_to_string(code_point);
        let raw_value = code_point_to_utf16(code_point);

        self.append_char_to_current_character_token(token_type, &value, &raw_value);
    }

    fn emit_chars(&mut self, value: &str) {
        self.append_char_to_current_character_token(
            TokenType::Character,
            value,
            &value.encode_utf16().collect::<Vec<_>>(),
        );
    }

    fn emit_character_reference(&mut self, decoded: &str, in_attribute: bool) {
        if in_attribute {
            self.current_attr.value.push_str(decoded);
        } else {
            for ch in decoded.chars() {
                self.emit_code_point(ch as i32);
            }
        }
    }

    fn emit_literal_ampersand(&mut self, in_attribute: bool) {
        if in_attribute {
            self.current_attr.value.push('&');
        } else {
            self.emit_code_point(cp::AMPERSAND);
        }
    }

    fn consume_character_reference(&mut self, in_attribute: bool) {
        if self.character_reference_needs_more_input() {
            self.hibernate_at_current_character_reference();
            return;
        }

        if self.try_consume_numeric_character_reference(in_attribute) {
            return;
        }

        if self.try_consume_named_character_reference(in_attribute) {
            return;
        }

        if !in_attribute && is_ascii_alpha_numeric(self.preprocessor.peek(1)) {
            self.return_state = self.state;
            self.state = State::AmbiguousAmpersand;
            self.emit_literal_ampersand(in_attribute);
            return;
        }

        self.emit_literal_ampersand(in_attribute);
    }

    fn character_reference_needs_more_input(&self) -> bool {
        if self.preprocessor.last_chunk_written {
            return false;
        }

        let remaining = self.preprocessor.remaining_from_current_pos();

        if remaining.len() <= 1 {
            return true;
        }

        if remaining[1] == b'#' as u16 {
            return numeric_character_reference_needs_more_input(&remaining[1..]);
        }

        ::entities::ENTITIES.iter().any(|entry| {
            let entity: Vec<u16> = entry.entity.encode_utf16().collect();

            remaining.len() < entity.len() && entity.starts_with(remaining)
        })
    }

    fn hibernate_at_current_character_reference(&mut self) {
        self.preprocessor.retreat(self.consumed_after_snapshot);
        self.consumed_after_snapshot = 0;
        self.active = false;
        self.preprocessor.end_of_chunk_hit = true;
    }

    fn try_consume_named_character_reference(&mut self, in_attribute: bool) -> bool {
        let mut best_match: Option<(&'static str, &'static str)> = None;

        for entry in ::entities::ENTITIES.iter() {
            if self.preprocessor.starts_with(entry.entity, true) {
                let replace = best_match
                    .map(|(entity, _)| entry.entity.len() > entity.len())
                    .unwrap_or(true);

                if replace {
                    best_match = Some((entry.entity, entry.characters));
                }
            }
        }

        let Some((entity, characters)) = best_match else {
            return false;
        };
        let has_semicolon = entity.ends_with(';');
        let entity_len = entity.encode_utf16().count();
        let next = self.preprocessor.peek(entity_len as isize);

        if in_attribute
            && !has_semicolon
            && (is_ascii_alpha_numeric(next) || next == cp::EQUALS_SIGN)
        {
            return false;
        }

        if !has_semicolon {
            self.err_with_offset(
                ErrorCode::MissingSemicolonAfterCharacterReference,
                entity_len as isize,
            );
        }

        self.advance_by(entity_len.saturating_sub(1));
        self.emit_character_reference(characters, in_attribute);
        true
    }

    fn try_consume_numeric_character_reference(&mut self, in_attribute: bool) -> bool {
        if self.preprocessor.peek(1) != '#' as i32 {
            return false;
        }

        let mut offset = 2;
        let hex =
            matches!(self.preprocessor.peek(offset), cp if cp == 'x' as i32 || cp == 'X' as i32);

        if hex {
            offset += 1;
        }

        let digits_start = offset;
        let radix = if hex { 16 } else { 10 };
        let mut value = 0u32;

        loop {
            let code_point = self.preprocessor.peek(offset);
            let digit = digit_value(code_point, radix);

            let Some(digit) = digit else {
                break;
            };

            value = value.saturating_mul(radix).saturating_add(digit);
            offset += 1;
        }

        if offset == digits_start {
            self.err_with_offset(
                ErrorCode::AbsenceOfDigitsInNumericCharacterReference,
                offset,
            );
            self.emit_literal_ampersand(in_attribute);
            return true;
        }

        let has_semicolon = self.preprocessor.peek(offset) == cp::SEMICOLON;
        if has_semicolon {
            offset += 1;
        } else {
            self.err_with_offset(ErrorCode::MissingSemicolonAfterCharacterReference, offset);
        }

        if let Some(error) = get_error_for_numeric_character_reference(value) {
            self.err_with_offset(error, offset);
        }

        let replacement = replacement_for_numeric_character_reference(value);
        self.advance_by(offset as usize - 1);
        self.emit_character_reference(&code_point_to_string(replacement as i32), in_attribute);
        true
    }

    fn tag_token_mut(&mut self) -> &mut TagToken {
        match self.current_token.as_mut() {
            Some(CurrentToken::Tag(token)) => token,
            _ => panic!("current token must be a tag token"),
        }
    }

    fn comment_token_mut(&mut self) -> &mut CommentToken {
        match self.current_token.as_mut() {
            Some(CurrentToken::Comment(token)) => token,
            _ => panic!("current token must be a comment token"),
        }
    }

    fn doctype_token_mut(&mut self) -> &mut DoctypeToken {
        match self.current_token.as_mut() {
            Some(CurrentToken::Doctype(token)) => token,
            _ => panic!("current token must be a doctype token"),
        }
    }

    fn current_doctype_name_mut(&mut self) -> &mut String {
        self.doctype_token_mut()
            .name
            .get_or_insert_with(String::new)
    }

    fn current_doctype_public_id_mut(&mut self) -> &mut String {
        self.doctype_token_mut()
            .public_id
            .get_or_insert_with(String::new)
    }

    fn current_doctype_system_id_mut(&mut self) -> &mut String {
        self.doctype_token_mut()
            .system_id
            .get_or_insert_with(String::new)
    }

    fn call_state(&mut self, code_point: i32) {
        match self.state {
            State::Data => self.state_data(code_point),
            State::RcData => self.state_rcdata(code_point),
            State::RawText => self.state_rawtext(code_point),
            State::ScriptData => self.state_script_data(code_point),
            State::PlainText => self.state_plaintext(code_point),
            State::TagOpen => self.state_tag_open(code_point),
            State::EndTagOpen => self.state_end_tag_open(code_point),
            State::TagName => self.state_tag_name(code_point),
            State::RcDataLessThanSign => self.state_rcdata_less_than_sign(code_point),
            State::RcDataEndTagOpen => self.state_rcdata_end_tag_open(code_point),
            State::RcDataEndTagName => self.state_rcdata_end_tag_name(code_point),
            State::RawTextLessThanSign => self.state_rawtext_less_than_sign(code_point),
            State::RawTextEndTagOpen => self.state_rawtext_end_tag_open(code_point),
            State::RawTextEndTagName => self.state_rawtext_end_tag_name(code_point),
            State::ScriptDataLessThanSign => self.state_script_data_less_than_sign(code_point),
            State::ScriptDataEndTagOpen => self.state_script_data_end_tag_open(code_point),
            State::ScriptDataEndTagName => self.state_script_data_end_tag_name(code_point),
            State::ScriptDataEscapeStart => self.state_script_data_escape_start(code_point),
            State::ScriptDataEscapeStartDash => {
                self.state_script_data_escape_start_dash(code_point);
            }
            State::ScriptDataEscaped => self.state_script_data_escaped(code_point),
            State::ScriptDataEscapedDash => self.state_script_data_escaped_dash(code_point),
            State::ScriptDataEscapedDashDash => {
                self.state_script_data_escaped_dash_dash(code_point);
            }
            State::ScriptDataEscapedLessThanSign => {
                self.state_script_data_escaped_less_than_sign(code_point);
            }
            State::ScriptDataEscapedEndTagOpen => {
                self.state_script_data_escaped_end_tag_open(code_point);
            }
            State::ScriptDataEscapedEndTagName => {
                self.state_script_data_escaped_end_tag_name(code_point);
            }
            State::ScriptDataDoubleEscapeStart => {
                self.state_script_data_double_escape_start(code_point);
            }
            State::ScriptDataDoubleEscaped => self.state_script_data_double_escaped(code_point),
            State::ScriptDataDoubleEscapedDash => {
                self.state_script_data_double_escaped_dash(code_point);
            }
            State::ScriptDataDoubleEscapedDashDash => {
                self.state_script_data_double_escaped_dash_dash(code_point);
            }
            State::ScriptDataDoubleEscapedLessThanSign => {
                self.state_script_data_double_escaped_less_than_sign(code_point);
            }
            State::ScriptDataDoubleEscapeEnd => {
                self.state_script_data_double_escape_end(code_point);
            }
            State::BeforeAttributeName => self.state_before_attribute_name(code_point),
            State::AttributeName => self.state_attribute_name(code_point),
            State::AfterAttributeName => self.state_after_attribute_name(code_point),
            State::BeforeAttributeValue => self.state_before_attribute_value(code_point),
            State::AttributeValueDoubleQuoted => {
                self.state_attribute_value_double_quoted(code_point);
            }
            State::AttributeValueSingleQuoted => {
                self.state_attribute_value_single_quoted(code_point);
            }
            State::AttributeValueUnquoted => self.state_attribute_value_unquoted(code_point),
            State::AfterAttributeValueQuoted => {
                self.state_after_attribute_value_quoted(code_point);
            }
            State::SelfClosingStartTag => self.state_self_closing_start_tag(code_point),
            State::BogusComment => self.state_bogus_comment(code_point),
            State::MarkupDeclarationOpen => self.state_markup_declaration_open(code_point),
            State::CommentStart => self.state_comment_start(code_point),
            State::CommentStartDash => self.state_comment_start_dash(code_point),
            State::Comment => self.state_comment(code_point),
            State::CommentLessThanSign => self.state_comment_less_than_sign(code_point),
            State::CommentLessThanSignBang => self.state_comment_less_than_sign_bang(code_point),
            State::CommentLessThanSignBangDash => {
                self.state_comment_less_than_sign_bang_dash(code_point);
            }
            State::CommentLessThanSignBangDashDash => {
                self.state_comment_less_than_sign_bang_dash_dash(code_point);
            }
            State::CommentEndDash => self.state_comment_end_dash(code_point),
            State::CommentEnd => self.state_comment_end(code_point),
            State::CommentEndBang => self.state_comment_end_bang(code_point),
            State::Doctype => self.state_doctype(code_point),
            State::BeforeDoctypeName => self.state_before_doctype_name(code_point),
            State::DoctypeName => self.state_doctype_name(code_point),
            State::AfterDoctypeName => self.state_after_doctype_name(code_point),
            State::AfterDoctypePublicKeyword => {
                self.state_after_doctype_public_keyword(code_point);
            }
            State::BeforeDoctypePublicIdentifier => {
                self.state_before_doctype_public_identifier(code_point);
            }
            State::DoctypePublicIdentifierDoubleQuoted => {
                self.state_doctype_public_identifier_double_quoted(code_point);
            }
            State::DoctypePublicIdentifierSingleQuoted => {
                self.state_doctype_public_identifier_single_quoted(code_point);
            }
            State::AfterDoctypePublicIdentifier => {
                self.state_after_doctype_public_identifier(code_point);
            }
            State::BetweenDoctypePublicAndSystemIdentifiers => {
                self.state_between_doctype_public_and_system_identifiers(code_point);
            }
            State::AfterDoctypeSystemKeyword => {
                self.state_after_doctype_system_keyword(code_point);
            }
            State::BeforeDoctypeSystemIdentifier => {
                self.state_before_doctype_system_identifier(code_point);
            }
            State::DoctypeSystemIdentifierDoubleQuoted => {
                self.state_doctype_system_identifier_double_quoted(code_point);
            }
            State::DoctypeSystemIdentifierSingleQuoted => {
                self.state_doctype_system_identifier_single_quoted(code_point);
            }
            State::AfterDoctypeSystemIdentifier => {
                self.state_after_doctype_system_identifier(code_point);
            }
            State::BogusDoctype => self.state_bogus_doctype(code_point),
            State::CDataSection => self.state_cdata_section(code_point),
            State::CDataSectionBracket => self.state_cdata_section_bracket(code_point),
            State::CDataSectionEnd => self.state_cdata_section_end(code_point),
            State::CharacterReference => {
                self.state = self.return_state;
                self.emit_code_point(cp::AMPERSAND);
                self.call_state(code_point);
            }
            State::AmbiguousAmpersand => self.state_ambiguous_ampersand(code_point),
        }
    }

    fn state_data(&mut self, code_point: i32) {
        match code_point {
            cp::LESS_THAN_SIGN => self.state = State::TagOpen,
            cp::AMPERSAND => self.consume_character_reference(false),
            cp::NULL => {
                self.err(ErrorCode::UnexpectedNullCharacter);
                self.emit_code_point(code_point);
            }
            cp::EOF => self.emit_eof_token(),
            _ => self.emit_code_point(code_point),
        }
    }

    fn state_ambiguous_ampersand(&mut self, code_point: i32) {
        if is_ascii_alpha_numeric(code_point) {
            self.emit_character_reference(&code_point_to_string(code_point), false);
        } else {
            if code_point == cp::SEMICOLON {
                self.err(ErrorCode::UnknownNamedCharacterReference);
            }

            self.state = self.return_state;
            self.call_state(code_point);
        }
    }

    fn state_rcdata(&mut self, code_point: i32) {
        match code_point {
            cp::AMPERSAND => self.consume_character_reference(false),
            cp::LESS_THAN_SIGN => self.state = State::RcDataLessThanSign,
            cp::NULL => {
                self.err(ErrorCode::UnexpectedNullCharacter);
                self.emit_chars(&REPLACEMENT_CHARACTER.to_string());
            }
            cp::EOF => self.emit_eof_token(),
            _ => self.emit_code_point(code_point),
        }
    }

    fn state_rawtext(&mut self, code_point: i32) {
        match code_point {
            cp::LESS_THAN_SIGN => self.state = State::RawTextLessThanSign,
            cp::NULL => {
                self.err(ErrorCode::UnexpectedNullCharacter);
                self.emit_chars(&REPLACEMENT_CHARACTER.to_string());
            }
            cp::EOF => self.emit_eof_token(),
            _ => self.emit_code_point(code_point),
        }
    }

    fn state_script_data(&mut self, code_point: i32) {
        match code_point {
            cp::LESS_THAN_SIGN => self.state = State::ScriptDataLessThanSign,
            cp::NULL => {
                self.err(ErrorCode::UnexpectedNullCharacter);
                self.emit_chars(&REPLACEMENT_CHARACTER.to_string());
            }
            cp::EOF => self.emit_eof_token(),
            _ => self.emit_code_point(code_point),
        }
    }

    fn state_plaintext(&mut self, code_point: i32) {
        match code_point {
            cp::NULL => {
                self.err(ErrorCode::UnexpectedNullCharacter);
                self.emit_chars(&REPLACEMENT_CHARACTER.to_string());
            }
            cp::EOF => self.emit_eof_token(),
            _ => self.emit_code_point(code_point),
        }
    }

    fn state_tag_open(&mut self, code_point: i32) {
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

    fn state_end_tag_open(&mut self, code_point: i32) {
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

    fn state_tag_name(&mut self, code_point: i32) {
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
                    .push_str(&code_point_to_string(code_point));
            }
        }
    }

    fn state_rcdata_less_than_sign(&mut self, code_point: i32) {
        if code_point == cp::SOLIDUS {
            self.state = State::RcDataEndTagOpen;
        } else {
            self.emit_chars("<");
            self.state = State::RcData;
            self.state_rcdata(code_point);
        }
    }

    fn state_rcdata_end_tag_open(&mut self, code_point: i32) {
        if is_ascii_letter(code_point) {
            self.state = State::RcDataEndTagName;
            self.state_rcdata_end_tag_name(code_point);
        } else {
            self.emit_chars("</");
            self.state = State::RcData;
            self.state_rcdata(code_point);
        }
    }

    fn state_rcdata_end_tag_name(&mut self, code_point: i32) {
        if self.handle_special_end_tag(State::RcData) {
            self.emit_chars("</");
            self.state = State::RcData;
            self.state_rcdata(code_point);
        }
    }

    fn state_rawtext_less_than_sign(&mut self, code_point: i32) {
        if code_point == cp::SOLIDUS {
            self.state = State::RawTextEndTagOpen;
        } else {
            self.emit_chars("<");
            self.state = State::RawText;
            self.state_rawtext(code_point);
        }
    }

    fn state_rawtext_end_tag_open(&mut self, code_point: i32) {
        if is_ascii_letter(code_point) {
            self.state = State::RawTextEndTagName;
            self.state_rawtext_end_tag_name(code_point);
        } else {
            self.emit_chars("</");
            self.state = State::RawText;
            self.state_rawtext(code_point);
        }
    }

    fn state_rawtext_end_tag_name(&mut self, code_point: i32) {
        if self.handle_special_end_tag(State::RawText) {
            self.emit_chars("</");
            self.state = State::RawText;
            self.state_rawtext(code_point);
        }
    }

    fn state_script_data_less_than_sign(&mut self, code_point: i32) {
        match code_point {
            cp::SOLIDUS => self.state = State::ScriptDataEndTagOpen,
            cp::EXCLAMATION_MARK => {
                self.state = State::ScriptDataEscapeStart;
                self.emit_chars("<!");
            }
            _ => {
                self.emit_chars("<");
                self.state = State::ScriptData;
                self.state_script_data(code_point);
            }
        }
    }

    fn state_script_data_end_tag_open(&mut self, code_point: i32) {
        if is_ascii_letter(code_point) {
            self.state = State::ScriptDataEndTagName;
            self.state_script_data_end_tag_name(code_point);
        } else {
            self.emit_chars("</");
            self.state = State::ScriptData;
            self.state_script_data(code_point);
        }
    }

    fn state_script_data_end_tag_name(&mut self, code_point: i32) {
        if self.handle_special_end_tag(State::ScriptData) {
            self.emit_chars("</");
            self.state = State::ScriptData;
            self.state_script_data(code_point);
        }
    }

    fn state_script_data_escape_start(&mut self, code_point: i32) {
        if code_point == cp::HYPHEN_MINUS {
            self.state = State::ScriptDataEscapeStartDash;
            self.emit_chars("-");
        } else {
            self.state = State::ScriptData;
            self.state_script_data(code_point);
        }
    }

    fn state_script_data_escape_start_dash(&mut self, code_point: i32) {
        if code_point == cp::HYPHEN_MINUS {
            self.state = State::ScriptDataEscapedDashDash;
            self.emit_chars("-");
        } else {
            self.state = State::ScriptData;
            self.state_script_data(code_point);
        }
    }

    fn state_script_data_escaped(&mut self, code_point: i32) {
        match code_point {
            cp::HYPHEN_MINUS => {
                self.state = State::ScriptDataEscapedDash;
                self.emit_chars("-");
            }
            cp::LESS_THAN_SIGN => self.state = State::ScriptDataEscapedLessThanSign,
            cp::NULL => {
                self.err(ErrorCode::UnexpectedNullCharacter);
                self.emit_chars(&REPLACEMENT_CHARACTER.to_string());
            }
            cp::EOF => {
                self.err(ErrorCode::EofInScriptHtmlCommentLikeText);
                self.emit_eof_token();
            }
            _ => self.emit_code_point(code_point),
        }
    }

    fn state_script_data_escaped_dash(&mut self, code_point: i32) {
        match code_point {
            cp::HYPHEN_MINUS => {
                self.state = State::ScriptDataEscapedDashDash;
                self.emit_chars("-");
            }
            cp::LESS_THAN_SIGN => self.state = State::ScriptDataEscapedLessThanSign,
            cp::NULL => {
                self.err(ErrorCode::UnexpectedNullCharacter);
                self.state = State::ScriptDataEscaped;
                self.emit_chars(&REPLACEMENT_CHARACTER.to_string());
            }
            cp::EOF => {
                self.err(ErrorCode::EofInScriptHtmlCommentLikeText);
                self.emit_eof_token();
            }
            _ => {
                self.state = State::ScriptDataEscaped;
                self.emit_code_point(code_point);
            }
        }
    }

    fn state_script_data_escaped_dash_dash(&mut self, code_point: i32) {
        match code_point {
            cp::HYPHEN_MINUS => self.emit_chars("-"),
            cp::LESS_THAN_SIGN => self.state = State::ScriptDataEscapedLessThanSign,
            cp::GREATER_THAN_SIGN => {
                self.state = State::ScriptData;
                self.emit_chars(">");
            }
            cp::NULL => {
                self.err(ErrorCode::UnexpectedNullCharacter);
                self.state = State::ScriptDataEscaped;
                self.emit_chars(&REPLACEMENT_CHARACTER.to_string());
            }
            cp::EOF => {
                self.err(ErrorCode::EofInScriptHtmlCommentLikeText);
                self.emit_eof_token();
            }
            _ => {
                self.state = State::ScriptDataEscaped;
                self.emit_code_point(code_point);
            }
        }
    }

    fn state_script_data_escaped_less_than_sign(&mut self, code_point: i32) {
        if code_point == cp::SOLIDUS {
            self.state = State::ScriptDataEscapedEndTagOpen;
        } else if is_ascii_letter(code_point) {
            self.emit_chars("<");
            self.state = State::ScriptDataDoubleEscapeStart;
            self.state_script_data_double_escape_start(code_point);
        } else {
            self.emit_chars("<");
            self.state = State::ScriptDataEscaped;
            self.state_script_data_escaped(code_point);
        }
    }

    fn state_script_data_escaped_end_tag_open(&mut self, code_point: i32) {
        if is_ascii_letter(code_point) {
            self.state = State::ScriptDataEscapedEndTagName;
            self.state_script_data_escaped_end_tag_name(code_point);
        } else {
            self.emit_chars("</");
            self.state = State::ScriptDataEscaped;
            self.state_script_data_escaped(code_point);
        }
    }

    fn state_script_data_escaped_end_tag_name(&mut self, code_point: i32) {
        if self.handle_special_end_tag(State::ScriptDataEscaped) {
            self.emit_chars("</");
            self.state = State::ScriptDataEscaped;
            self.state_script_data_escaped(code_point);
        }
    }

    fn state_script_data_double_escape_start(&mut self, code_point: i32) {
        if self.preprocessor.starts_with(seq::SCRIPT, false)
            && is_script_data_double_escape_sequence_end(
                self.preprocessor.peek(seq::SCRIPT.len() as isize),
            )
        {
            self.emit_code_point(code_point);

            for _ in 0..seq::SCRIPT.len() {
                let code_point = self.consume();
                self.emit_code_point(code_point);
            }

            self.state = State::ScriptDataDoubleEscaped;
        } else if !self.ensure_hibernation() {
            self.state = State::ScriptDataEscaped;
            self.state_script_data_escaped(code_point);
        }
    }

    fn state_script_data_double_escaped(&mut self, code_point: i32) {
        match code_point {
            cp::HYPHEN_MINUS => {
                self.state = State::ScriptDataDoubleEscapedDash;
                self.emit_chars("-");
            }
            cp::LESS_THAN_SIGN => {
                self.state = State::ScriptDataDoubleEscapedLessThanSign;
                self.emit_chars("<");
            }
            cp::NULL => {
                self.err(ErrorCode::UnexpectedNullCharacter);
                self.emit_chars(&REPLACEMENT_CHARACTER.to_string());
            }
            cp::EOF => {
                self.err(ErrorCode::EofInScriptHtmlCommentLikeText);
                self.emit_eof_token();
            }
            _ => self.emit_code_point(code_point),
        }
    }

    fn state_script_data_double_escaped_dash(&mut self, code_point: i32) {
        match code_point {
            cp::HYPHEN_MINUS => {
                self.state = State::ScriptDataDoubleEscapedDashDash;
                self.emit_chars("-");
            }
            cp::LESS_THAN_SIGN => {
                self.state = State::ScriptDataDoubleEscapedLessThanSign;
                self.emit_chars("<");
            }
            cp::NULL => {
                self.err(ErrorCode::UnexpectedNullCharacter);
                self.state = State::ScriptDataDoubleEscaped;
                self.emit_chars(&REPLACEMENT_CHARACTER.to_string());
            }
            cp::EOF => {
                self.err(ErrorCode::EofInScriptHtmlCommentLikeText);
                self.emit_eof_token();
            }
            _ => {
                self.state = State::ScriptDataDoubleEscaped;
                self.emit_code_point(code_point);
            }
        }
    }

    fn state_script_data_double_escaped_dash_dash(&mut self, code_point: i32) {
        match code_point {
            cp::HYPHEN_MINUS => self.emit_chars("-"),
            cp::LESS_THAN_SIGN => {
                self.state = State::ScriptDataDoubleEscapedLessThanSign;
                self.emit_chars("<");
            }
            cp::GREATER_THAN_SIGN => {
                self.state = State::ScriptData;
                self.emit_chars(">");
            }
            cp::NULL => {
                self.err(ErrorCode::UnexpectedNullCharacter);
                self.state = State::ScriptDataDoubleEscaped;
                self.emit_chars(&REPLACEMENT_CHARACTER.to_string());
            }
            cp::EOF => {
                self.err(ErrorCode::EofInScriptHtmlCommentLikeText);
                self.emit_eof_token();
            }
            _ => {
                self.state = State::ScriptDataDoubleEscaped;
                self.emit_code_point(code_point);
            }
        }
    }

    fn state_script_data_double_escaped_less_than_sign(&mut self, code_point: i32) {
        if code_point == cp::SOLIDUS {
            self.state = State::ScriptDataDoubleEscapeEnd;
            self.emit_chars("/");
        } else {
            self.state = State::ScriptDataDoubleEscaped;
            self.state_script_data_double_escaped(code_point);
        }
    }

    fn state_script_data_double_escape_end(&mut self, code_point: i32) {
        if self.preprocessor.starts_with(seq::SCRIPT, false)
            && is_script_data_double_escape_sequence_end(
                self.preprocessor.peek(seq::SCRIPT.len() as isize),
            )
        {
            self.emit_code_point(code_point);

            for _ in 0..seq::SCRIPT.len() {
                let code_point = self.consume();
                self.emit_code_point(code_point);
            }

            self.state = State::ScriptDataEscaped;
        } else if !self.ensure_hibernation() {
            self.state = State::ScriptDataDoubleEscaped;
            self.state_script_data_double_escaped(code_point);
        }
    }

    fn handle_special_end_tag(&mut self, _text_state: State) -> bool {
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

    fn state_before_attribute_name(&mut self, code_point: i32) {
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

    fn state_attribute_name(&mut self, code_point: i32) {
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
                    .push_str(&code_point_to_string(code_point));
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
                    .push_str(&code_point_to_string(code_point));
            }
        }
    }

    fn state_after_attribute_name(&mut self, code_point: i32) {
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

    fn state_before_attribute_value(&mut self, code_point: i32) {
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

    fn state_attribute_value_double_quoted(&mut self, code_point: i32) {
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
                .push_str(&code_point_to_string(code_point)),
        }
    }

    fn state_attribute_value_single_quoted(&mut self, code_point: i32) {
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
                .push_str(&code_point_to_string(code_point)),
        }
    }

    fn state_attribute_value_unquoted(&mut self, code_point: i32) {
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
                    .push_str(&code_point_to_string(code_point));
            }
            cp::EOF => {
                self.err(ErrorCode::EofInTag);
                self.emit_eof_token();
            }
            _ => self
                .current_attr
                .value
                .push_str(&code_point_to_string(code_point)),
        }
    }

    fn state_after_attribute_value_quoted(&mut self, code_point: i32) {
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
                self.leave_attr_value();
                self.state = State::BeforeAttributeName;
                self.state_before_attribute_name(code_point);
            }
        }
    }

    fn state_self_closing_start_tag(&mut self, code_point: i32) {
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

    fn state_bogus_comment(&mut self, code_point: i32) {
        match code_point {
            cp::GREATER_THAN_SIGN => {
                self.state = State::Data;
                self.emit_current_comment();
            }
            cp::EOF => {
                self.emit_current_comment();
                self.emit_eof_token();
            }
            cp::NULL => {
                self.err(ErrorCode::UnexpectedNullCharacter);
                self.comment_token_mut().data.push(REPLACEMENT_CHARACTER);
            }
            _ => self
                .comment_token_mut()
                .data
                .push_str(&code_point_to_string(code_point)),
        }
    }

    fn state_markup_declaration_open(&mut self, code_point: i32) {
        if self.consume_sequence_if_match(seq::DASH_DASH, true) {
            self.create_comment_token(seq::DASH_DASH.len() as isize + 1);
            self.state = State::CommentStart;
        } else if self.consume_sequence_if_match(seq::DOCTYPE, false) {
            self.current_location = self.get_current_location(seq::DOCTYPE.len() as isize + 1);
            self.state = State::Doctype;
        } else if self.consume_sequence_if_match(seq::CDATA_START, true) {
            if self.in_foreign_node {
                self.state = State::CDataSection;
            } else {
                self.err(ErrorCode::CdataInHtmlContent);
                self.create_comment_token(seq::CDATA_START.len() as isize + 1);
                self.comment_token_mut().data = "[CDATA[".to_string();
                self.state = State::BogusComment;
            }
        } else if !self.ensure_hibernation() {
            self.err(ErrorCode::IncorrectlyOpenedComment);
            self.create_comment_token(2);
            self.state = State::BogusComment;
            self.state_bogus_comment(code_point);
        }
    }

    fn state_comment_start(&mut self, code_point: i32) {
        match code_point {
            cp::HYPHEN_MINUS => self.state = State::CommentStartDash,
            cp::GREATER_THAN_SIGN => {
                self.err(ErrorCode::AbruptClosingOfEmptyComment);
                self.state = State::Data;
                self.emit_current_comment();
            }
            _ => {
                self.state = State::Comment;
                self.state_comment(code_point);
            }
        }
    }

    fn state_comment_start_dash(&mut self, code_point: i32) {
        match code_point {
            cp::HYPHEN_MINUS => self.state = State::CommentEnd,
            cp::GREATER_THAN_SIGN => {
                self.err(ErrorCode::AbruptClosingOfEmptyComment);
                self.state = State::Data;
                self.emit_current_comment();
            }
            cp::EOF => {
                self.err(ErrorCode::EofInComment);
                self.emit_current_comment();
                self.emit_eof_token();
            }
            _ => {
                self.comment_token_mut().data.push('-');
                self.state = State::Comment;
                self.state_comment(code_point);
            }
        }
    }

    fn state_comment(&mut self, code_point: i32) {
        match code_point {
            cp::HYPHEN_MINUS => self.state = State::CommentEndDash,
            cp::LESS_THAN_SIGN => {
                self.comment_token_mut().data.push('<');
                self.state = State::CommentLessThanSign;
            }
            cp::NULL => {
                self.err(ErrorCode::UnexpectedNullCharacter);
                self.comment_token_mut().data.push(REPLACEMENT_CHARACTER);
            }
            cp::EOF => {
                self.err(ErrorCode::EofInComment);
                self.emit_current_comment();
                self.emit_eof_token();
            }
            _ => self
                .comment_token_mut()
                .data
                .push_str(&code_point_to_string(code_point)),
        }
    }

    fn state_comment_less_than_sign(&mut self, code_point: i32) {
        match code_point {
            cp::EXCLAMATION_MARK => {
                self.comment_token_mut().data.push('!');
                self.state = State::CommentLessThanSignBang;
            }
            cp::LESS_THAN_SIGN => self.comment_token_mut().data.push('<'),
            _ => {
                self.state = State::Comment;
                self.state_comment(code_point);
            }
        }
    }

    fn state_comment_less_than_sign_bang(&mut self, code_point: i32) {
        if code_point == cp::HYPHEN_MINUS {
            self.state = State::CommentLessThanSignBangDash;
        } else {
            self.state = State::Comment;
            self.state_comment(code_point);
        }
    }

    fn state_comment_less_than_sign_bang_dash(&mut self, code_point: i32) {
        if code_point == cp::HYPHEN_MINUS {
            self.state = State::CommentLessThanSignBangDashDash;
        } else {
            self.state = State::CommentEndDash;
            self.state_comment_end_dash(code_point);
        }
    }

    fn state_comment_less_than_sign_bang_dash_dash(&mut self, code_point: i32) {
        if code_point != cp::GREATER_THAN_SIGN && code_point != cp::EOF {
            self.err(ErrorCode::NestedComment);
        }

        self.state = State::CommentEnd;
        self.state_comment_end(code_point);
    }

    fn state_comment_end_dash(&mut self, code_point: i32) {
        match code_point {
            cp::HYPHEN_MINUS => self.state = State::CommentEnd,
            cp::EOF => {
                self.err(ErrorCode::EofInComment);
                self.emit_current_comment();
                self.emit_eof_token();
            }
            _ => {
                self.comment_token_mut().data.push('-');
                self.state = State::Comment;
                self.state_comment(code_point);
            }
        }
    }

    fn state_comment_end(&mut self, code_point: i32) {
        match code_point {
            cp::GREATER_THAN_SIGN => {
                self.state = State::Data;
                self.emit_current_comment();
            }
            cp::EXCLAMATION_MARK => self.state = State::CommentEndBang,
            cp::HYPHEN_MINUS => self.comment_token_mut().data.push('-'),
            cp::EOF => {
                self.err(ErrorCode::EofInComment);
                self.emit_current_comment();
                self.emit_eof_token();
            }
            _ => {
                self.comment_token_mut().data.push_str("--");
                self.state = State::Comment;
                self.state_comment(code_point);
            }
        }
    }

    fn state_comment_end_bang(&mut self, code_point: i32) {
        match code_point {
            cp::HYPHEN_MINUS => {
                self.comment_token_mut().data.push_str("--!");
                self.state = State::CommentEndDash;
            }
            cp::GREATER_THAN_SIGN => {
                self.err(ErrorCode::IncorrectlyClosedComment);
                self.state = State::Data;
                self.emit_current_comment();
            }
            cp::EOF => {
                self.err(ErrorCode::EofInComment);
                self.emit_current_comment();
                self.emit_eof_token();
            }
            _ => {
                self.comment_token_mut().data.push_str("--!");
                self.state = State::Comment;
                self.state_comment(code_point);
            }
        }
    }

    fn state_doctype(&mut self, code_point: i32) {
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

    fn state_before_doctype_name(&mut self, code_point: i32) {
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

    fn state_doctype_name(&mut self, code_point: i32) {
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
                    .push_str(&code_point_to_string(code_point));
            }
        }
    }

    fn state_after_doctype_name(&mut self, code_point: i32) {
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

    fn state_after_doctype_public_keyword(&mut self, code_point: i32) {
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

    fn state_before_doctype_public_identifier(&mut self, code_point: i32) {
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

    fn state_doctype_public_identifier_double_quoted(&mut self, code_point: i32) {
        self.state_doctype_identifier_quoted(code_point, true, true);
    }

    fn state_doctype_public_identifier_single_quoted(&mut self, code_point: i32) {
        self.state_doctype_identifier_quoted(code_point, true, false);
    }

    fn state_after_doctype_public_identifier(&mut self, code_point: i32) {
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

    fn state_between_doctype_public_and_system_identifiers(&mut self, code_point: i32) {
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

    fn state_after_doctype_system_keyword(&mut self, code_point: i32) {
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

    fn state_before_doctype_system_identifier(&mut self, code_point: i32) {
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

    fn state_doctype_system_identifier_double_quoted(&mut self, code_point: i32) {
        self.state_doctype_identifier_quoted(code_point, false, true);
    }

    fn state_doctype_system_identifier_single_quoted(&mut self, code_point: i32) {
        self.state_doctype_identifier_quoted(code_point, false, false);
    }

    fn state_after_doctype_system_identifier(&mut self, code_point: i32) {
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

    fn state_doctype_identifier_quoted(
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
                        .push_str(&code_point_to_string(code_point));
                } else {
                    self.current_doctype_system_id_mut()
                        .push_str(&code_point_to_string(code_point));
                }
            }
        }
    }

    fn state_bogus_doctype(&mut self, code_point: i32) {
        match code_point {
            cp::GREATER_THAN_SIGN => {
                self.emit_current_doctype();
                self.state = State::Data;
            }
            cp::NULL => self.err(ErrorCode::UnexpectedNullCharacter),
            cp::EOF => {
                self.emit_current_doctype();
                self.emit_eof_token();
            }
            _ => {}
        }
    }

    fn state_cdata_section(&mut self, code_point: i32) {
        match code_point {
            cp::RIGHT_SQUARE_BRACKET => self.state = State::CDataSectionBracket,
            cp::EOF => {
                self.err(ErrorCode::EofInCdata);
                self.emit_eof_token();
            }
            _ => self.emit_code_point(code_point),
        }
    }

    fn state_cdata_section_bracket(&mut self, code_point: i32) {
        if code_point == cp::RIGHT_SQUARE_BRACKET {
            self.state = State::CDataSectionEnd;
        } else {
            self.emit_chars("]");
            self.state = State::CDataSection;
            self.state_cdata_section(code_point);
        }
    }

    fn state_cdata_section_end(&mut self, code_point: i32) {
        match code_point {
            cp::GREATER_THAN_SIGN => self.state = State::Data,
            cp::RIGHT_SQUARE_BRACKET => self.emit_chars("]"),
            _ => {
                self.emit_chars("]]");
                self.state = State::CDataSection;
                self.state_cdata_section(code_point);
            }
        }
    }
}

fn current_location_from(
    preprocessor: &Preprocessor,
    options: TokenizerOptions,
    offset: isize,
) -> Option<Location> {
    if !options.source_code_location_info {
        return None;
    }

    let raw_offset = preprocessor.dropped_buffer_size as isize + preprocessor.pos;

    Some(Location {
        start_line: preprocessor.line,
        start_col: (preprocessor.col() as isize - offset).max(0) as usize,
        start_offset: (raw_offset - offset).max(0) as usize,
        end_line: 0,
        end_col: 0,
        end_offset: 0,
    })
}

fn code_point_to_string(code_point: i32) -> String {
    if code_point < 0 {
        String::new()
    } else {
        char::from_u32(code_point as u32)
            .unwrap_or(REPLACEMENT_CHARACTER)
            .to_string()
    }
}

fn code_point_to_utf16(code_point: i32) -> Vec<u16> {
    if code_point < 0 {
        Vec::new()
    } else if (0xd800..=0xdfff).contains(&code_point) {
        vec![code_point as u16]
    } else {
        code_point_to_string(code_point).encode_utf16().collect()
    }
}

fn digit_value(code_point: i32, radix: u32) -> Option<u32> {
    let ch = char::from_u32(code_point as u32)?;
    ch.to_digit(radix)
}

fn numeric_character_reference_needs_more_input(remaining_after_ampersand: &[u16]) -> bool {
    debug_assert_eq!(
        remaining_after_ampersand.first().copied(),
        Some(b'#' as u16)
    );

    if remaining_after_ampersand.len() == 1 {
        return true;
    }

    let mut offset = 1;
    let hex = matches!(
        remaining_after_ampersand.get(offset).copied(),
        Some(cp) if cp == b'x' as u16 || cp == b'X' as u16
    );

    if hex {
        offset += 1;

        if offset == remaining_after_ampersand.len() {
            return true;
        }

        while offset < remaining_after_ampersand.len()
            && digit_value(remaining_after_ampersand[offset] as i32, 16).is_some()
        {
            offset += 1;
        }
    } else {
        while offset < remaining_after_ampersand.len()
            && is_ascii_digit(remaining_after_ampersand[offset] as i32)
        {
            offset += 1;
        }
    }

    offset == remaining_after_ampersand.len()
}

#[cfg(test)]
mod tests {
    use super::{
        get_error_for_numeric_character_reference, is_ascii_alpha_numeric, is_ascii_digit,
        is_ascii_letter, is_ascii_lower, is_ascii_upper, is_script_data_double_escape_sequence_end,
        is_whitespace, to_ascii_lower, State, Tokenizer, TokenizerMode, TokenizerOptions,
    };
    use crate::common::error_codes::ErrorCode;
    use crate::common::token::{Token, TokenType};
    use crate::common::unicode::code_points as cp;

    #[test]
    fn state_discriminants_follow_reference_order() {
        assert_eq!(State::Data as u8, 0);
        assert_eq!(State::RcData as u8, 1);
        assert_eq!(State::PlainText as u8, 4);
        assert_eq!(State::TagOpen as u8, 5);
        assert_eq!(State::BeforeAttributeName as u8, 31);
        assert_eq!(State::Doctype as u8, 52);
        assert_eq!(State::CDataSection as u8, 68);
        assert_eq!(State::AmbiguousAmpersand as u8, 72);
    }

    #[test]
    fn tokenizer_modes_map_to_initial_states() {
        assert_eq!(TokenizerMode::Data.state(), State::Data);
        assert_eq!(TokenizerMode::RcData.state(), State::RcData);
        assert_eq!(TokenizerMode::RawText.state(), State::RawText);
        assert_eq!(TokenizerMode::ScriptData.state(), State::ScriptData);
        assert_eq!(TokenizerMode::PlainText.state(), State::PlainText);
        assert_eq!(TokenizerMode::CDataSection.state(), State::CDataSection);
    }

    #[test]
    fn ascii_helpers_match_parse5_predicates() {
        assert!(is_ascii_digit('0' as i32));
        assert!(is_ascii_digit('9' as i32));
        assert!(!is_ascii_digit('/' as i32));
        assert!(is_ascii_upper('A' as i32));
        assert!(is_ascii_upper('Z' as i32));
        assert!(is_ascii_lower('a' as i32));
        assert!(is_ascii_lower('z' as i32));
        assert!(is_ascii_letter('Q' as i32));
        assert!(is_ascii_alpha_numeric('7' as i32));
        assert_eq!(to_ascii_lower('A' as i32), 'a' as i32);
    }

    #[test]
    fn whitespace_and_script_escape_helpers_match_reference() {
        assert!(is_whitespace(cp::SPACE));
        assert!(is_whitespace(cp::LINE_FEED));
        assert!(is_whitespace(cp::TABULATION));
        assert!(is_whitespace(cp::FORM_FEED));
        assert!(!is_whitespace(cp::CARRIAGE_RETURN));
        assert!(is_script_data_double_escape_sequence_end(cp::SOLIDUS));
        assert!(is_script_data_double_escape_sequence_end(
            cp::GREATER_THAN_SIGN
        ));
        assert!(!is_script_data_double_escape_sequence_end('x' as i32));
    }

    #[test]
    fn numeric_character_reference_errors_match_parse5_order() {
        assert_eq!(
            get_error_for_numeric_character_reference(0),
            Some(ErrorCode::NullCharacterReference)
        );
        assert_eq!(
            get_error_for_numeric_character_reference(0x110000),
            Some(ErrorCode::CharacterReferenceOutsideUnicodeRange)
        );
        assert_eq!(
            get_error_for_numeric_character_reference(0xd800),
            Some(ErrorCode::SurrogateCharacterReference)
        );
        assert_eq!(
            get_error_for_numeric_character_reference(0xfdd0),
            Some(ErrorCode::NoncharacterCharacterReference)
        );
        assert_eq!(
            get_error_for_numeric_character_reference(0x0001),
            Some(ErrorCode::ControlCharacterReference)
        );
        assert_eq!(
            get_error_for_numeric_character_reference(cp::CARRIAGE_RETURN as u32),
            Some(ErrorCode::ControlCharacterReference)
        );
        assert_eq!(get_error_for_numeric_character_reference('A' as u32), None);
    }

    #[test]
    fn tokenizer_groups_character_tokens_and_emits_eof() {
        let output = Tokenizer::tokenize("hi \n\u{0}x", TokenizerOptions::default());
        let tokens: Vec<_> = output
            .tokens
            .iter()
            .map(|token| match token {
                Token::Character(token) => (token.token_type, token.chars.as_str()),
                Token::Eof(_) => (TokenType::Eof, ""),
                _ => panic!("unexpected token: {token:?}"),
            })
            .collect();

        assert_eq!(
            tokens,
            vec![
                (TokenType::Character, "hi"),
                (TokenType::WhitespaceCharacter, " \n"),
                (TokenType::NullCharacter, "\u{0}"),
                (TokenType::Character, "x"),
                (TokenType::Eof, ""),
            ]
        );
        assert_eq!(
            output.errors.iter().map(|err| err.code).collect::<Vec<_>>(),
            vec![ErrorCode::UnexpectedNullCharacter]
        );
    }

    #[test]
    fn tokenizer_emits_start_tags_with_attributes() {
        let output = Tokenizer::tokenize(
            "<DiV id=\"x\" disabled class=y />",
            TokenizerOptions::default(),
        );

        let Token::Tag(token) = &output.tokens[0] else {
            panic!("expected tag token");
        };

        assert_eq!(token.token_type, TokenType::StartTag);
        assert_eq!(token.tag_name, "div");
        assert!(token.self_closing);
        assert_eq!(token.attr("id"), Some("x"));
        assert_eq!(token.attr("disabled"), Some(""));
        assert_eq!(token.attr("class"), Some("y"));
        assert!(matches!(output.tokens[1], Token::Eof(_)));
        assert!(output.errors.is_empty());
    }

    #[test]
    fn tokenizer_reports_end_tag_attributes_and_trailing_solidus() {
        let output = Tokenizer::tokenize("</div class=x />", TokenizerOptions::default());

        let Token::Tag(token) = &output.tokens[0] else {
            panic!("expected tag token");
        };

        assert_eq!(token.token_type, TokenType::EndTag);
        assert_eq!(token.tag_name, "div");
        assert_eq!(token.attr("class"), Some("x"));
        assert!(token.self_closing);
        assert_eq!(
            output.errors.iter().map(|err| err.code).collect::<Vec<_>>(),
            vec![
                ErrorCode::EndTagWithAttributes,
                ErrorCode::EndTagWithTrailingSolidus
            ]
        );
    }

    #[test]
    fn tokenizer_emits_comments_and_bogus_comments() {
        let output = Tokenizer::tokenize("<!-- Test --><?x>", TokenizerOptions::default());

        let Token::Comment(comment) = &output.tokens[0] else {
            panic!("expected comment token");
        };
        assert_eq!(comment.data, " Test ");

        let Token::Comment(bogus) = &output.tokens[1] else {
            panic!("expected bogus comment token");
        };
        assert_eq!(bogus.data, "?x");
        assert_eq!(
            output.errors.iter().map(|err| err.code).collect::<Vec<_>>(),
            vec![ErrorCode::UnexpectedQuestionMarkInsteadOfTagName]
        );
    }

    #[test]
    fn tokenizer_emits_doctype_public_and_system_ids() {
        let output = Tokenizer::tokenize(
            "<!DOCTYPE html PUBLIC \"pub\" \"sys\">",
            TokenizerOptions::default(),
        );

        let Token::Doctype(doctype) = &output.tokens[0] else {
            panic!("expected doctype token");
        };

        assert_eq!(doctype.name.as_deref(), Some("html"));
        assert_eq!(doctype.public_id.as_deref(), Some("pub"));
        assert_eq!(doctype.system_id.as_deref(), Some("sys"));
        assert!(!doctype.force_quirks);
        assert!(output.errors.is_empty());
    }

    #[test]
    fn tokenizer_handles_rawtext_end_tags_for_last_start_tag_name() {
        let mut tokenizer = Tokenizer::new(TokenizerOptions::default());
        tokenizer.state = State::RawText;
        tokenizer.last_start_tag_name = "style".to_string();
        tokenizer.write("a</style>b", true);
        let output = tokenizer.finish();

        assert!(matches!(&output.tokens[0], Token::Character(token) if token.chars == "a"));
        assert!(
            matches!(&output.tokens[1], Token::Tag(token) if token.token_type == TokenType::EndTag && token.tag_name == "style")
        );
        assert!(matches!(&output.tokens[2], Token::Character(token) if token.chars == "b"));
        assert!(matches!(output.tokens[3], Token::Eof(_)));
    }

    #[test]
    fn tokenizer_hibernates_inside_special_end_tag_delimiter_like_upstream() {
        let mut tokenizer = Tokenizer::new(TokenizerOptions::default());
        tokenizer.state = State::RcData;
        tokenizer.last_start_tag_name = "title".to_string();
        tokenizer.write("abc</title", false);
        tokenizer.write("><p>x", true);
        let output = tokenizer.finish();

        assert!(matches!(&output.tokens[0], Token::Character(token) if token.chars == "abc"));
        assert!(
            matches!(&output.tokens[1], Token::Tag(token) if token.token_type == TokenType::EndTag && token.tag_name == "title")
        );
        assert!(
            matches!(&output.tokens[2], Token::Tag(token) if token.token_type == TokenType::StartTag && token.tag_name == "p")
        );
        assert!(matches!(&output.tokens[3], Token::Character(token) if token.chars == "x"));
        assert!(matches!(output.tokens[4], Token::Eof(_)));
        assert!(output.errors.is_empty());
    }

    #[test]
    fn tokenizer_handles_script_comment_like_escaped_text() {
        let mut tokenizer = Tokenizer::new(TokenizerOptions::default());
        tokenizer.state = State::ScriptData;
        tokenizer.last_start_tag_name = "script".to_string();
        tokenizer.write("<!--x--></script>z", true);
        let output = tokenizer.finish();

        assert!(matches!(&output.tokens[0], Token::Character(token) if token.chars == "<!--x-->"));
        assert!(
            matches!(&output.tokens[1], Token::Tag(token) if token.token_type == TokenType::EndTag && token.tag_name == "script")
        );
        assert!(matches!(&output.tokens[2], Token::Character(token) if token.chars == "z"));
        assert!(matches!(output.tokens[3], Token::Eof(_)));
        assert!(output.errors.is_empty());
    }

    #[test]
    fn tokenizer_reports_script_comment_like_eof() {
        let mut tokenizer = Tokenizer::new(TokenizerOptions::default());
        tokenizer.state = State::ScriptData;
        tokenizer.write("<!--x", true);
        let output = tokenizer.finish();

        assert!(matches!(
            &output.tokens[0],
            Token::Character(token) if token.chars == "<!--x"
        ));
        assert!(matches!(output.tokens[1], Token::Eof(_)));
        assert_eq!(
            output.errors.iter().map(|err| err.code).collect::<Vec<_>>(),
            vec![ErrorCode::EofInScriptHtmlCommentLikeText]
        );
    }

    #[test]
    fn tokenizer_replaces_null_in_script_comment_like_text() {
        let mut tokenizer = Tokenizer::new(TokenizerOptions::default());
        tokenizer.state = State::ScriptData;
        tokenizer.write("<!--\0-->", true);
        let output = tokenizer.finish();

        assert!(matches!(
            &output.tokens[0],
            Token::Character(token) if token.chars == "<!--\u{fffd}-->"
        ));
        assert!(matches!(output.tokens[1], Token::Eof(_)));
        assert_eq!(
            output.errors.iter().map(|err| err.code).collect::<Vec<_>>(),
            vec![ErrorCode::UnexpectedNullCharacter]
        );
    }

    #[test]
    fn tokenizer_keeps_inner_script_end_tag_text_while_double_escaped() {
        let mut tokenizer = Tokenizer::new(TokenizerOptions::default());
        tokenizer.state = State::ScriptData;
        tokenizer.last_start_tag_name = "script".to_string();
        tokenizer.write("<!--<script>var x='</script>';</script>tail", true);
        let output = tokenizer.finish();

        let leading_text = output
            .tokens
            .iter()
            .take_while(|token| matches!(token, Token::Character(_)))
            .fold(String::new(), |mut text, token| {
                if let Token::Character(token) = token {
                    text.push_str(&token.chars);
                }

                text
            });

        assert_eq!(leading_text, "<!--<script>var x='</script>';");
        assert!(
            matches!(&output.tokens[3], Token::Tag(token) if token.token_type == TokenType::EndTag && token.tag_name == "script")
        );
        assert!(matches!(
            &output.tokens[4],
            Token::Character(token) if token.chars == "tail"
        ));
        assert!(matches!(output.tokens[5], Token::Eof(_)));
        assert!(output.errors.is_empty());
    }

    #[test]
    fn tokenizer_reports_eof_in_script_double_escaped_text() {
        let mut tokenizer = Tokenizer::new(TokenizerOptions::default());
        tokenizer.state = State::ScriptData;
        tokenizer.write("<!--<script>", true);
        let output = tokenizer.finish();

        assert!(matches!(
            &output.tokens[0],
            Token::Character(token) if token.chars == "<!--<script>"
        ));
        assert!(matches!(output.tokens[1], Token::Eof(_)));
        assert_eq!(
            output.errors.iter().map(|err| err.code).collect::<Vec<_>>(),
            vec![ErrorCode::EofInScriptHtmlCommentLikeText]
        );
    }

    #[test]
    fn tokenizer_records_basic_tag_locations() {
        let output = Tokenizer::tokenize(
            "<div id=\"x\">",
            TokenizerOptions {
                source_code_location_info: true,
            },
        );

        let Token::Tag(token) = &output.tokens[0] else {
            panic!("expected tag token");
        };
        let location = token.location.as_ref().unwrap();

        assert_eq!(location.location.start_offset, 0);
        assert_eq!(location.location.end_offset, 12);
        assert!(location.attrs.contains_key("id"));
    }

    #[test]
    fn tokenizer_decodes_named_character_references_in_text() {
        let output =
            Tokenizer::tokenize("a&amp;b&nbsp;&notin;&notit;", TokenizerOptions::default());

        assert!(matches!(
            &output.tokens[0],
            Token::Character(token) if token.chars == "a&b\u{a0}∉¬it;"
        ));
        assert_eq!(
            output.errors.iter().map(|err| err.code).collect::<Vec<_>>(),
            vec![ErrorCode::MissingSemicolonAfterCharacterReference]
        );
    }

    #[test]
    fn tokenizer_hibernates_inside_named_character_references_like_upstream() {
        let mut tokenizer = Tokenizer::new(TokenizerOptions::default());

        tokenizer.write("a&nb", false);
        tokenizer.write("sp;b&nbsp", false);
        tokenizer.write(";c", true);

        let output = tokenizer.finish();

        assert!(matches!(
            &output.tokens[0],
            Token::Character(token) if token.chars == "a\u{a0}b\u{a0}c"
        ));
        assert!(output.errors.is_empty());
    }

    #[test]
    fn tokenizer_blocks_legacy_named_references_in_attributes() {
        let output =
            Tokenizer::tokenize("<a href=\"a&amp;b&notit;\">", TokenizerOptions::default());

        let Token::Tag(token) = &output.tokens[0] else {
            panic!("expected tag token");
        };

        assert_eq!(token.attr("href"), Some("a&b&notit;"));
        assert!(output.errors.is_empty());
    }

    #[test]
    fn tokenizer_reports_unknown_named_character_references() {
        let output = Tokenizer::tokenize("&bogus;", TokenizerOptions::default());

        assert!(matches!(
            &output.tokens[0],
            Token::Character(token) if token.chars == "&bogus;"
        ));
        assert_eq!(
            output.errors.iter().map(|err| err.code).collect::<Vec<_>>(),
            vec![ErrorCode::UnknownNamedCharacterReference]
        );
    }

    #[test]
    fn tokenizer_keeps_ambiguous_ampersand_state_across_chunks_like_upstream() {
        let mut tokenizer = Tokenizer::new(TokenizerOptions::default());

        tokenizer.write("&bog", false);
        tokenizer.write("us", false);
        tokenizer.write(";", true);

        let output = tokenizer.finish();

        assert!(matches!(
            &output.tokens[0],
            Token::Character(token) if token.chars == "&bogus;"
        ));
        assert_eq!(
            output.errors.iter().map(|err| err.code).collect::<Vec<_>>(),
            vec![ErrorCode::UnknownNamedCharacterReference]
        );
    }

    #[test]
    fn tokenizer_decodes_numeric_character_references() {
        let output = Tokenizer::tokenize(
            "&#65&#x41;&#x80;&#0;&#x110000;",
            TokenizerOptions::default(),
        );

        assert!(matches!(
            &output.tokens[0],
            Token::Character(token) if token.chars == "AA€��"
        ));
        assert_eq!(
            output.errors.iter().map(|err| err.code).collect::<Vec<_>>(),
            vec![
                ErrorCode::MissingSemicolonAfterCharacterReference,
                ErrorCode::ControlCharacterReference,
                ErrorCode::NullCharacterReference,
                ErrorCode::CharacterReferenceOutsideUnicodeRange,
            ]
        );
    }

    #[test]
    fn tokenizer_hibernates_inside_numeric_character_references_like_upstream() {
        let mut tokenizer = Tokenizer::new(TokenizerOptions::default());

        tokenizer.write("&#x", false);
        tokenizer.write("41;&#160", false);
        tokenizer.write("x", true);

        let output = tokenizer.finish();

        assert!(matches!(
            &output.tokens[0],
            Token::Character(token) if token.chars == "A\u{a0}x"
        ));
        assert_eq!(
            output.errors.iter().map(|err| err.code).collect::<Vec<_>>(),
            vec![ErrorCode::MissingSemicolonAfterCharacterReference]
        );
    }

    #[test]
    fn tokenizer_reports_absence_of_numeric_reference_digits() {
        let output = Tokenizer::tokenize("&#x; &#;", TokenizerOptions::default());

        assert!(matches!(
            &output.tokens[0],
            Token::Character(token) if token.chars == "&#x;"
        ));
        assert!(matches!(
            &output.tokens[1],
            Token::Character(token) if token.token_type == TokenType::WhitespaceCharacter && token.chars == " "
        ));
        assert!(matches!(
            &output.tokens[2],
            Token::Character(token) if token.chars == "&#;"
        ));
        assert_eq!(
            output.errors.iter().map(|err| err.code).collect::<Vec<_>>(),
            vec![
                ErrorCode::AbsenceOfDigitsInNumericCharacterReference,
                ErrorCode::AbsenceOfDigitsInNumericCharacterReference,
            ]
        );
    }
}
