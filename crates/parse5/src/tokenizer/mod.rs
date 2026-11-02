//! Tokenizer port scaffold.
//!
//! The TypeScript tokenizer lives upstream at `packages/parse5/lib/tokenizer`.
//! This module mirrors its state names and low-level helpers before the full
//! state machine is wired into `Tokenizer`.

pub(in crate::tokenizer) use crate::common::entities::{
    is_proper_prefix_of_named_character_reference, longest_named_character_reference_prefix,
    max_named_character_reference_len,
};
pub(in crate::tokenizer) use crate::common::error_codes::{ErrorCode, ParserError};
pub(in crate::tokenizer) use crate::common::html::{get_tag_id, TagId};
pub(in crate::tokenizer) use crate::common::token::{
    get_token_attr, Attribute, CharacterToken, CommentToken, DoctypeToken, EofToken, Location,
    LocationWithAttributes, TagToken, Token, TokenType,
};
pub(in crate::tokenizer) use crate::common::unicode::{
    code_points as cp, is_control_code_point, is_surrogate, is_undefined_code_point,
    sequences as seq, REPLACEMENT_CHARACTER, REPLACEMENT_CHARACTER_STR,
};
pub(in crate::tokenizer) use preprocessor::Preprocessor;
pub(in crate::tokenizer) use std::collections::BTreeMap;

#[path = "states/attribute.rs"]
mod attribute_states;
#[path = "states/cdata.rs"]
mod cdata_states;
#[path = "char_ref.rs"]
mod char_ref;
#[path = "states/comment.rs"]
mod comment_states;
#[path = "states/data.rs"]
mod data_states;
#[path = "states/doctype.rs"]
mod doctype_states;
pub mod preprocessor;
#[path = "states/script.rs"]
mod script_states;
#[path = "states/tag.rs"]
mod tag_states;

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

pub(crate) fn is_ascii_digit(cp: i32) -> bool {
    (cp::DIGIT_0..=cp::DIGIT_9).contains(&cp)
}

pub(crate) fn is_ascii_upper(cp: i32) -> bool {
    (cp::LATIN_CAPITAL_A..=cp::LATIN_CAPITAL_Z).contains(&cp)
}

pub(crate) fn is_ascii_lower(cp: i32) -> bool {
    (cp::LATIN_SMALL_A..=cp::LATIN_SMALL_Z).contains(&cp)
}

pub(crate) fn is_ascii_letter(cp: i32) -> bool {
    is_ascii_lower(cp) || is_ascii_upper(cp)
}

pub(crate) fn is_ascii_alpha_numeric(cp: i32) -> bool {
    is_ascii_letter(cp) || is_ascii_digit(cp)
}

pub(crate) fn to_ascii_lower(cp: i32) -> i32 {
    cp + 0x20
}

pub(crate) fn is_whitespace(cp: i32) -> bool {
    matches!(
        cp,
        cp::SPACE | cp::LINE_FEED | cp::TABULATION | cp::FORM_FEED
    )
}

pub(crate) fn is_script_data_double_escape_sequence_end(cp: i32) -> bool {
    is_whitespace(cp) || matches!(cp, cp::SOLIDUS | cp::GREATER_THAN_SIGN)
}

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

pub(in crate::tokenizer) fn replacement_for_numeric_character_reference(code: u32) -> u32 {
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

    pub(in crate::tokenizer) fn run_parsing_loop(&mut self) {
        while self.active {
            self.consumed_after_snapshot = 0;
            let code_point = self.consume();

            if !self.ensure_hibernation() {
                self.call_state(code_point);
            }
        }
    }

    pub(in crate::tokenizer) fn ensure_hibernation(&mut self) -> bool {
        if self.preprocessor.end_of_chunk_hit {
            self.preprocessor.retreat(self.consumed_after_snapshot);
            self.consumed_after_snapshot = 0;
            self.active = false;
            true
        } else {
            false
        }
    }

    pub(in crate::tokenizer) fn consume(&mut self) -> i32 {
        self.consumed_after_snapshot += 1;
        self.preprocessor.advance()
    }

    pub(in crate::tokenizer) fn advance_by(&mut self, count: usize) {
        self.consumed_after_snapshot += count as isize;

        for _ in 0..count {
            self.preprocessor.advance();
        }
    }

    pub(in crate::tokenizer) fn consume_sequence_if_match(
        &mut self,
        pattern: &str,
        case_sensitive: bool,
    ) -> bool {
        if self.preprocessor.starts_with(pattern, case_sensitive) {
            self.advance_by(pattern.encode_utf16().count().saturating_sub(1));
            true
        } else {
            false
        }
    }

    pub(in crate::tokenizer) fn err(&mut self, code: ErrorCode) {
        self.err_with_offset(code, 0);
    }

    pub(in crate::tokenizer) fn err_with_offset(&mut self, code: ErrorCode, cp_offset: isize) {
        self.errors
            .push(self.preprocessor.get_error(code, cp_offset));
    }

    pub(in crate::tokenizer) fn get_current_location(&self, offset: isize) -> Option<Location> {
        current_location_from(&self.preprocessor, self.options, offset)
    }

    pub(in crate::tokenizer) fn create_start_tag_token(&mut self) {
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

    pub(in crate::tokenizer) fn create_end_tag_token(&mut self) {
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

    pub(in crate::tokenizer) fn create_comment_token(&mut self, offset: isize) {
        self.current_token = Some(CurrentToken::Comment(CommentToken {
            token_type: TokenType::Comment,
            data: String::new(),
            location: self.get_current_location(offset),
        }));
    }

    pub(in crate::tokenizer) fn create_doctype_token(&mut self, initial_name: Option<String>) {
        self.current_token = Some(CurrentToken::Doctype(DoctypeToken {
            token_type: TokenType::Doctype,
            name: initial_name,
            force_quirks: false,
            public_id: None,
            system_id: None,
            location: self.current_location,
        }));
    }

    pub(in crate::tokenizer) fn create_character_token(
        &mut self,
        token_type: TokenType,
        chars: String,
    ) {
        let raw_chars = chars.encode_utf16().collect();
        self.current_character_token = Some(CharacterToken {
            token_type,
            chars,
            raw_chars,
            location: self.current_location,
        });
    }

    pub(in crate::tokenizer) fn create_attr(&mut self, attr_name_first_ch: &str) {
        self.current_attr = Attribute::new(attr_name_first_ch, "");
        self.current_attr_location = self.get_current_location(0);
        self.current_attr_committed_name = None;
    }

    pub(in crate::tokenizer) fn leave_attr_name(&mut self) {
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

    /// Mirrors parse5's `_leaveAttrValue`: records the attribute's end
    /// location at the current position and syncs the value.
    ///
    /// Only call this where upstream calls `_leaveAttrValue` (end of the
    /// attribute name, and leaving a quoted/unquoted value on whitespace, `/`
    /// or `>`). Calling it anywhere else moves the end of the span.
    pub(in crate::tokenizer) fn leave_attr_value(&mut self) {
        if self.current_attr_committed_name.is_none() {
            return;
        }

        if let Some(attr_location) = &mut self.current_attr_location {
            attr_location.end_line = self.preprocessor.line;
            attr_location.end_col = self.preprocessor.col();
            attr_location.end_offset = self.preprocessor.offset();
        }

        self.sync_committed_attr();
    }

    /// Copies the in-progress attribute value and location onto the token.
    ///
    /// Upstream pushes `currentAttr` and `currentLocation` into the token by
    /// reference, so later value edits show up there with no extra step. This
    /// port stores copies, so it syncs them here. Doing so never moves the
    /// end of the span.
    pub(in crate::tokenizer) fn sync_committed_attr(&mut self) {
        let Some(attr_name) = self.current_attr_committed_name.clone() else {
            return;
        };

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

    pub(in crate::tokenizer) fn prepare_location(&mut self, location: Option<Location>) {
        self.emit_current_character_token(location);
        self.current_token = None;
        self.current_location = self.get_current_location(-1);
    }

    pub(in crate::tokenizer) fn finish_location(&self, location: &mut Location) {
        location.end_line = self.preprocessor.line;
        location.end_col = self.preprocessor.col() + 1;
        location.end_offset = self.preprocessor.offset() + 1;
    }

    pub(in crate::tokenizer) fn emit_current_tag_token(&mut self) {
        self.sync_committed_attr();
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

    pub(in crate::tokenizer) fn emit_current_comment(&mut self) {
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

    pub(in crate::tokenizer) fn emit_current_doctype(&mut self) {
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

    pub(in crate::tokenizer) fn emit_current_character_token(
        &mut self,
        next_location: Option<Location>,
    ) {
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

    pub(in crate::tokenizer) fn emit_eof_token(&mut self) {
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

    pub(in crate::tokenizer) fn emit_token(&mut self, token: Token) {
        self.tokens.push(token);

        if self.pause_on_token && self.active {
            self.paused_on_token = true;
            self.active = false;
        }
    }

    pub(in crate::tokenizer) fn append_char_to_current_character_token(
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

    pub(in crate::tokenizer) fn emit_code_point(&mut self, code_point: i32) {
        let token_type = if is_whitespace(code_point) {
            TokenType::WhitespaceCharacter
        } else if code_point == cp::NULL {
            TokenType::NullCharacter
        } else {
            TokenType::Character
        };
        let mut value_buf = [0u8; 4];
        let value = code_point_to_char(code_point).map_or("", |ch| ch.encode_utf8(&mut value_buf));
        let mut raw_buf = [0u16; 2];
        let raw_value = code_point_to_utf16(code_point, &mut raw_buf);

        self.append_char_to_current_character_token(token_type, value, raw_value);
    }

    pub(in crate::tokenizer) fn emit_chars(&mut self, value: &str) {
        let raw_value = value.encode_utf16().collect::<Vec<_>>();

        self.append_char_to_current_character_token(TokenType::Character, value, &raw_value);
    }

    pub(in crate::tokenizer) fn tag_token_mut(&mut self) -> &mut TagToken {
        match self.current_token.as_mut() {
            Some(CurrentToken::Tag(token)) => token,
            _ => panic!("current token must be a tag token"),
        }
    }

    pub(in crate::tokenizer) fn comment_token_mut(&mut self) -> &mut CommentToken {
        match self.current_token.as_mut() {
            Some(CurrentToken::Comment(token)) => token,
            _ => panic!("current token must be a comment token"),
        }
    }

    pub(in crate::tokenizer) fn doctype_token_mut(&mut self) -> &mut DoctypeToken {
        match self.current_token.as_mut() {
            Some(CurrentToken::Doctype(token)) => token,
            _ => panic!("current token must be a doctype token"),
        }
    }

    pub(in crate::tokenizer) fn current_doctype_name_mut(&mut self) -> &mut String {
        self.doctype_token_mut()
            .name
            .get_or_insert_with(String::new)
    }

    pub(in crate::tokenizer) fn current_doctype_public_id_mut(&mut self) -> &mut String {
        self.doctype_token_mut()
            .public_id
            .get_or_insert_with(String::new)
    }

    pub(in crate::tokenizer) fn current_doctype_system_id_mut(&mut self) -> &mut String {
        self.doctype_token_mut()
            .system_id
            .get_or_insert_with(String::new)
    }

    pub(in crate::tokenizer) fn call_state(&mut self, code_point: i32) {
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

    pub(in crate::tokenizer) fn state_bogus_doctype(&mut self, code_point: i32) {
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
}

pub(in crate::tokenizer) fn current_location_from(
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

pub(in crate::tokenizer) fn code_point_to_char(code_point: i32) -> Option<char> {
    u32::try_from(code_point)
        .ok()
        .map(|code_point| char::from_u32(code_point).unwrap_or(REPLACEMENT_CHARACTER))
}

pub(in crate::tokenizer) fn code_point_to_string(code_point: i32) -> String {
    code_point_to_char(code_point)
        .map(String::from)
        .unwrap_or_default()
}

/// Encodes `code_point` as UTF-16 into `buf`, keeping lone surrogates as-is.
pub(in crate::tokenizer) fn code_point_to_utf16(code_point: i32, buf: &mut [u16; 2]) -> &[u16] {
    match code_point {
        ..0 => &[],
        0xd800..=0xdfff => {
            buf[0] = code_point as u16;
            &buf[..1]
        }
        _ => code_point_to_char(code_point).map_or(&[][..], |ch| ch.encode_utf16(buf)),
    }
}

pub(in crate::tokenizer) fn digit_value(code_point: i32, radix: u32) -> Option<u32> {
    let ch = char::from_u32(code_point as u32)?;
    ch.to_digit(radix)
}

pub(in crate::tokenizer) fn numeric_character_reference_needs_more_input(
    remaining_after_ampersand: &[u16],
) -> bool {
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
mod tests;
