use crate::common::token::Location;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ErrorCode {
    ControlCharacterInInputStream,
    NoncharacterInInputStream,
    SurrogateInInputStream,
    NonVoidHtmlElementStartTagWithTrailingSolidus,
    EndTagWithAttributes,
    EndTagWithTrailingSolidus,
    UnexpectedSolidusInTag,
    UnexpectedNullCharacter,
    UnexpectedQuestionMarkInsteadOfTagName,
    InvalidFirstCharacterOfTagName,
    UnexpectedEqualsSignBeforeAttributeName,
    MissingEndTagName,
    UnexpectedCharacterInAttributeName,
    UnknownNamedCharacterReference,
    MissingSemicolonAfterCharacterReference,
    UnexpectedCharacterAfterDoctypeSystemIdentifier,
    UnexpectedCharacterInUnquotedAttributeValue,
    EofBeforeTagName,
    EofInTag,
    MissingAttributeValue,
    MissingWhitespaceBetweenAttributes,
    MissingWhitespaceAfterDoctypePublicKeyword,
    MissingWhitespaceBetweenDoctypePublicAndSystemIdentifiers,
    MissingWhitespaceAfterDoctypeSystemKeyword,
    MissingQuoteBeforeDoctypePublicIdentifier,
    MissingQuoteBeforeDoctypeSystemIdentifier,
    MissingDoctypePublicIdentifier,
    MissingDoctypeSystemIdentifier,
    AbruptDoctypePublicIdentifier,
    AbruptDoctypeSystemIdentifier,
    CdataInHtmlContent,
    IncorrectlyOpenedComment,
    EofInScriptHtmlCommentLikeText,
    EofInDoctype,
    NestedComment,
    AbruptClosingOfEmptyComment,
    EofInComment,
    IncorrectlyClosedComment,
    EofInCdata,
    AbsenceOfDigitsInNumericCharacterReference,
    NullCharacterReference,
    SurrogateCharacterReference,
    CharacterReferenceOutsideUnicodeRange,
    ControlCharacterReference,
    NoncharacterCharacterReference,
    MissingWhitespaceBeforeDoctypeName,
    MissingDoctypeName,
    InvalidCharacterSequenceAfterDoctypeName,
    DuplicateAttribute,
    NonConformingDoctype,
    MissingDoctype,
    MisplacedDoctype,
    EndTagWithoutMatchingOpenElement,
    ClosingOfElementWithOpenChildElements,
    DisallowedContentInNoscriptInHead,
    OpenElementsLeftAfterEof,
    AbandonedHeadElementChild,
    MisplacedStartTagForHeadElement,
    NestedNoscriptInHead,
    EofInElementThatCanContainOnlyText,
}

impl ErrorCode {
    pub const ALL: [ErrorCode; 60] = [
        ErrorCode::ControlCharacterInInputStream,
        ErrorCode::NoncharacterInInputStream,
        ErrorCode::SurrogateInInputStream,
        ErrorCode::NonVoidHtmlElementStartTagWithTrailingSolidus,
        ErrorCode::EndTagWithAttributes,
        ErrorCode::EndTagWithTrailingSolidus,
        ErrorCode::UnexpectedSolidusInTag,
        ErrorCode::UnexpectedNullCharacter,
        ErrorCode::UnexpectedQuestionMarkInsteadOfTagName,
        ErrorCode::InvalidFirstCharacterOfTagName,
        ErrorCode::UnexpectedEqualsSignBeforeAttributeName,
        ErrorCode::MissingEndTagName,
        ErrorCode::UnexpectedCharacterInAttributeName,
        ErrorCode::UnknownNamedCharacterReference,
        ErrorCode::MissingSemicolonAfterCharacterReference,
        ErrorCode::UnexpectedCharacterAfterDoctypeSystemIdentifier,
        ErrorCode::UnexpectedCharacterInUnquotedAttributeValue,
        ErrorCode::EofBeforeTagName,
        ErrorCode::EofInTag,
        ErrorCode::MissingAttributeValue,
        ErrorCode::MissingWhitespaceBetweenAttributes,
        ErrorCode::MissingWhitespaceAfterDoctypePublicKeyword,
        ErrorCode::MissingWhitespaceBetweenDoctypePublicAndSystemIdentifiers,
        ErrorCode::MissingWhitespaceAfterDoctypeSystemKeyword,
        ErrorCode::MissingQuoteBeforeDoctypePublicIdentifier,
        ErrorCode::MissingQuoteBeforeDoctypeSystemIdentifier,
        ErrorCode::MissingDoctypePublicIdentifier,
        ErrorCode::MissingDoctypeSystemIdentifier,
        ErrorCode::AbruptDoctypePublicIdentifier,
        ErrorCode::AbruptDoctypeSystemIdentifier,
        ErrorCode::CdataInHtmlContent,
        ErrorCode::IncorrectlyOpenedComment,
        ErrorCode::EofInScriptHtmlCommentLikeText,
        ErrorCode::EofInDoctype,
        ErrorCode::NestedComment,
        ErrorCode::AbruptClosingOfEmptyComment,
        ErrorCode::EofInComment,
        ErrorCode::IncorrectlyClosedComment,
        ErrorCode::EofInCdata,
        ErrorCode::AbsenceOfDigitsInNumericCharacterReference,
        ErrorCode::NullCharacterReference,
        ErrorCode::SurrogateCharacterReference,
        ErrorCode::CharacterReferenceOutsideUnicodeRange,
        ErrorCode::ControlCharacterReference,
        ErrorCode::NoncharacterCharacterReference,
        ErrorCode::MissingWhitespaceBeforeDoctypeName,
        ErrorCode::MissingDoctypeName,
        ErrorCode::InvalidCharacterSequenceAfterDoctypeName,
        ErrorCode::DuplicateAttribute,
        ErrorCode::NonConformingDoctype,
        ErrorCode::MissingDoctype,
        ErrorCode::MisplacedDoctype,
        ErrorCode::EndTagWithoutMatchingOpenElement,
        ErrorCode::ClosingOfElementWithOpenChildElements,
        ErrorCode::DisallowedContentInNoscriptInHead,
        ErrorCode::OpenElementsLeftAfterEof,
        ErrorCode::AbandonedHeadElementChild,
        ErrorCode::MisplacedStartTagForHeadElement,
        ErrorCode::NestedNoscriptInHead,
        ErrorCode::EofInElementThatCanContainOnlyText,
    ];

    pub fn as_str(self) -> &'static str {
        match self {
            ErrorCode::ControlCharacterInInputStream => "control-character-in-input-stream",
            ErrorCode::NoncharacterInInputStream => "noncharacter-in-input-stream",
            ErrorCode::SurrogateInInputStream => "surrogate-in-input-stream",
            ErrorCode::NonVoidHtmlElementStartTagWithTrailingSolidus => {
                "non-void-html-element-start-tag-with-trailing-solidus"
            }
            ErrorCode::EndTagWithAttributes => "end-tag-with-attributes",
            ErrorCode::EndTagWithTrailingSolidus => "end-tag-with-trailing-solidus",
            ErrorCode::UnexpectedSolidusInTag => "unexpected-solidus-in-tag",
            ErrorCode::UnexpectedNullCharacter => "unexpected-null-character",
            ErrorCode::UnexpectedQuestionMarkInsteadOfTagName => {
                "unexpected-question-mark-instead-of-tag-name"
            }
            ErrorCode::InvalidFirstCharacterOfTagName => "invalid-first-character-of-tag-name",
            ErrorCode::UnexpectedEqualsSignBeforeAttributeName => {
                "unexpected-equals-sign-before-attribute-name"
            }
            ErrorCode::MissingEndTagName => "missing-end-tag-name",
            ErrorCode::UnexpectedCharacterInAttributeName => {
                "unexpected-character-in-attribute-name"
            }
            ErrorCode::UnknownNamedCharacterReference => "unknown-named-character-reference",
            ErrorCode::MissingSemicolonAfterCharacterReference => {
                "missing-semicolon-after-character-reference"
            }
            ErrorCode::UnexpectedCharacterAfterDoctypeSystemIdentifier => {
                "unexpected-character-after-doctype-system-identifier"
            }
            ErrorCode::UnexpectedCharacterInUnquotedAttributeValue => {
                "unexpected-character-in-unquoted-attribute-value"
            }
            ErrorCode::EofBeforeTagName => "eof-before-tag-name",
            ErrorCode::EofInTag => "eof-in-tag",
            ErrorCode::MissingAttributeValue => "missing-attribute-value",
            ErrorCode::MissingWhitespaceBetweenAttributes => {
                "missing-whitespace-between-attributes"
            }
            ErrorCode::MissingWhitespaceAfterDoctypePublicKeyword => {
                "missing-whitespace-after-doctype-public-keyword"
            }
            ErrorCode::MissingWhitespaceBetweenDoctypePublicAndSystemIdentifiers => {
                "missing-whitespace-between-doctype-public-and-system-identifiers"
            }
            ErrorCode::MissingWhitespaceAfterDoctypeSystemKeyword => {
                "missing-whitespace-after-doctype-system-keyword"
            }
            ErrorCode::MissingQuoteBeforeDoctypePublicIdentifier => {
                "missing-quote-before-doctype-public-identifier"
            }
            ErrorCode::MissingQuoteBeforeDoctypeSystemIdentifier => {
                "missing-quote-before-doctype-system-identifier"
            }
            ErrorCode::MissingDoctypePublicIdentifier => "missing-doctype-public-identifier",
            ErrorCode::MissingDoctypeSystemIdentifier => "missing-doctype-system-identifier",
            ErrorCode::AbruptDoctypePublicIdentifier => "abrupt-doctype-public-identifier",
            ErrorCode::AbruptDoctypeSystemIdentifier => "abrupt-doctype-system-identifier",
            ErrorCode::CdataInHtmlContent => "cdata-in-html-content",
            ErrorCode::IncorrectlyOpenedComment => "incorrectly-opened-comment",
            ErrorCode::EofInScriptHtmlCommentLikeText => "eof-in-script-html-comment-like-text",
            ErrorCode::EofInDoctype => "eof-in-doctype",
            ErrorCode::NestedComment => "nested-comment",
            ErrorCode::AbruptClosingOfEmptyComment => "abrupt-closing-of-empty-comment",
            ErrorCode::EofInComment => "eof-in-comment",
            ErrorCode::IncorrectlyClosedComment => "incorrectly-closed-comment",
            ErrorCode::EofInCdata => "eof-in-cdata",
            ErrorCode::AbsenceOfDigitsInNumericCharacterReference => {
                "absence-of-digits-in-numeric-character-reference"
            }
            ErrorCode::NullCharacterReference => "null-character-reference",
            ErrorCode::SurrogateCharacterReference => "surrogate-character-reference",
            ErrorCode::CharacterReferenceOutsideUnicodeRange => {
                "character-reference-outside-unicode-range"
            }
            ErrorCode::ControlCharacterReference => "control-character-reference",
            ErrorCode::NoncharacterCharacterReference => "noncharacter-character-reference",
            ErrorCode::MissingWhitespaceBeforeDoctypeName => {
                "missing-whitespace-before-doctype-name"
            }
            ErrorCode::MissingDoctypeName => "missing-doctype-name",
            ErrorCode::InvalidCharacterSequenceAfterDoctypeName => {
                "invalid-character-sequence-after-doctype-name"
            }
            ErrorCode::DuplicateAttribute => "duplicate-attribute",
            ErrorCode::NonConformingDoctype => "non-conforming-doctype",
            ErrorCode::MissingDoctype => "missing-doctype",
            ErrorCode::MisplacedDoctype => "misplaced-doctype",
            ErrorCode::EndTagWithoutMatchingOpenElement => "end-tag-without-matching-open-element",
            ErrorCode::ClosingOfElementWithOpenChildElements => {
                "closing-of-element-with-open-child-elements"
            }
            ErrorCode::DisallowedContentInNoscriptInHead => {
                "disallowed-content-in-noscript-in-head"
            }
            ErrorCode::OpenElementsLeftAfterEof => "open-elements-left-after-eof",
            ErrorCode::AbandonedHeadElementChild => "abandoned-head-element-child",
            ErrorCode::MisplacedStartTagForHeadElement => "misplaced-start-tag-for-head-element",
            ErrorCode::NestedNoscriptInHead => "nested-noscript-in-head",
            ErrorCode::EofInElementThatCanContainOnlyText => {
                "eof-in-element-that-can-contain-only-text"
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ParserError {
    pub code: ErrorCode,
    #[serde(flatten)]
    pub location: Location,
}
