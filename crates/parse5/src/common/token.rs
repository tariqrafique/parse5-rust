use crate::common::html::TagId;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[repr(u8)]
pub enum TokenType {
    Character = 0,
    NullCharacter = 1,
    WhitespaceCharacter = 2,
    StartTag = 3,
    EndTag = 4,
    Comment = 5,
    Doctype = 6,
    Eof = 7,
    Hibernation = 8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Location {
    pub start_line: usize,
    pub start_col: usize,
    pub start_offset: usize,
    pub end_line: usize,
    pub end_col: usize,
    pub end_offset: usize,
}

impl Location {
    pub fn zero_width_at_start() -> Self {
        Self {
            start_line: 1,
            start_col: 1,
            start_offset: 0,
            end_line: 1,
            end_col: 1,
            end_offset: 0,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LocationWithAttributes {
    #[serde(flatten)]
    pub location: Location,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub attrs: BTreeMap<String, Location>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ElementLocation {
    #[serde(flatten)]
    pub location: Location,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub attrs: BTreeMap<String, Location>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub start_tag: Option<Location>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub end_tag: Option<Location>,
}

impl From<Location> for ElementLocation {
    fn from(location: Location) -> Self {
        Self {
            location,
            attrs: BTreeMap::new(),
            start_tag: None,
            end_tag: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Attribute {
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub namespace: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub prefix: Option<String>,
    pub value: String,
}

impl Attribute {
    pub fn new(name: impl Into<String>, value: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            namespace: None,
            prefix: None,
            value: value.into(),
        }
    }

    pub fn qualified_name(&self) -> String {
        match &self.prefix {
            Some(prefix) if !prefix.is_empty() => format!("{prefix}:{}", self.name),
            None => self.name.clone(),
            _ => self.name.clone(),
        }
    }
}

pub fn get_token_attr<'a>(token: &'a TagToken, attr_name: &str) -> Option<&'a str> {
    token
        .attrs
        .iter()
        .rev()
        .find(|attr| attr.name == attr_name)
        .map(|attr| attr.value.as_str())
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DoctypeToken {
    pub token_type: TokenType,
    pub name: Option<String>,
    pub force_quirks: bool,
    pub public_id: Option<String>,
    pub system_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub location: Option<Location>,
}

impl DoctypeToken {
    pub fn new(initial_name: Option<impl Into<String>>) -> Self {
        Self {
            token_type: TokenType::Doctype,
            name: initial_name.map(Into::into),
            force_quirks: false,
            public_id: None,
            system_id: None,
            location: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TagToken {
    pub token_type: TokenType,
    pub tag_name: String,
    pub tag_id: TagId,
    pub self_closing: bool,
    pub ack_self_closing: bool,
    pub attrs: Vec<Attribute>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub location: Option<LocationWithAttributes>,
}

impl TagToken {
    pub fn new_start_tag(
        tag_name: impl Into<String>,
        tag_id: TagId,
        attrs: Vec<Attribute>,
    ) -> Self {
        Self {
            token_type: TokenType::StartTag,
            tag_name: tag_name.into(),
            tag_id,
            self_closing: false,
            ack_self_closing: false,
            attrs,
            location: None,
        }
    }

    pub fn new_end_tag(tag_name: impl Into<String>, tag_id: TagId, attrs: Vec<Attribute>) -> Self {
        Self {
            token_type: TokenType::EndTag,
            tag_name: tag_name.into(),
            tag_id,
            self_closing: false,
            ack_self_closing: false,
            attrs,
            location: None,
        }
    }

    pub fn attr(&self, name: &str) -> Option<&str> {
        get_token_attr(self, name)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CommentToken {
    pub token_type: TokenType,
    pub data: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub location: Option<Location>,
}

impl CommentToken {
    pub fn new(data: impl Into<String>) -> Self {
        Self {
            token_type: TokenType::Comment,
            data: data.into(),
            location: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EofToken {
    pub token_type: TokenType,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub location: Option<Location>,
}

impl EofToken {
    pub fn new() -> Self {
        Self {
            token_type: TokenType::Eof,
            location: None,
        }
    }
}

impl Default for EofToken {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CharacterToken {
    pub token_type: TokenType,
    pub chars: String,
    #[serde(skip)]
    pub raw_chars: Vec<u16>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub location: Option<Location>,
}

impl CharacterToken {
    pub fn new(token_type: TokenType, chars: impl Into<String>) -> Self {
        debug_assert!(matches!(
            token_type,
            TokenType::Character | TokenType::NullCharacter | TokenType::WhitespaceCharacter
        ));
        let chars = chars.into();
        let raw_chars = chars.encode_utf16().collect();

        Self {
            token_type,
            chars,
            raw_chars,
            location: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "token")]
pub enum Token {
    Doctype(DoctypeToken),
    Tag(TagToken),
    Comment(CommentToken),
    Eof(EofToken),
    Character(CharacterToken),
}

impl Token {
    pub fn token_type(&self) -> TokenType {
        match self {
            Self::Doctype(token) => token.token_type,
            Self::Tag(token) => token.token_type,
            Self::Comment(token) => token.token_type,
            Self::Eof(token) => token.token_type,
            Self::Character(token) => token.token_type,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{
        get_token_attr, Attribute, CharacterToken, CommentToken, DoctypeToken, EofToken, TagToken,
        Token, TokenType,
    };
    use crate::common::html::TagId;

    #[test]
    fn token_type_discriminants_match_parse5_order() {
        assert_eq!(TokenType::Character as u8, 0);
        assert_eq!(TokenType::NullCharacter as u8, 1);
        assert_eq!(TokenType::WhitespaceCharacter as u8, 2);
        assert_eq!(TokenType::StartTag as u8, 3);
        assert_eq!(TokenType::EndTag as u8, 4);
        assert_eq!(TokenType::Comment as u8, 5);
        assert_eq!(TokenType::Doctype as u8, 6);
        assert_eq!(TokenType::Eof as u8, 7);
        assert_eq!(TokenType::Hibernation as u8, 8);
    }

    #[test]
    fn get_token_attr_searches_from_the_end_like_parse5() {
        let token = TagToken::new_start_tag(
            "div",
            TagId::Div,
            vec![
                Attribute::new("id", "first"),
                Attribute::new("class", "name"),
                Attribute::new("id", "last"),
            ],
        );

        assert_eq!(get_token_attr(&token, "id"), Some("last"));
        assert_eq!(token.attr("class"), Some("name"));
        assert_eq!(token.attr("missing"), None);
    }

    #[test]
    fn token_constructors_use_parse5_token_types() {
        let doctype = DoctypeToken::new(Some("html"));
        let start = TagToken::new_start_tag("p", TagId::P, Vec::new());
        let end = TagToken::new_end_tag("p", TagId::P, Vec::new());
        let comment = CommentToken::new("hello");
        let eof = EofToken::new();
        let character = CharacterToken::new(TokenType::Character, "text");

        assert_eq!(doctype.token_type, TokenType::Doctype);
        assert_eq!(start.token_type, TokenType::StartTag);
        assert_eq!(end.token_type, TokenType::EndTag);
        assert_eq!(comment.token_type, TokenType::Comment);
        assert_eq!(eof.token_type, TokenType::Eof);
        assert_eq!(character.token_type, TokenType::Character);
        assert_eq!(Token::Tag(start).token_type(), TokenType::StartTag);
    }
}
