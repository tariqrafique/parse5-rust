//! Rust port of the parse5 package.
//!
//! The module layout intentionally follows upstream `packages/parse5/lib`.

pub mod common;
pub mod parser;
pub mod serializer;
pub mod tokenizer;
pub mod tree_adapters;

pub use common::error_codes::{ErrorCode, ParserError};
pub use common::html::DocumentMode;
pub use common::token::{
    Attribute, CharacterToken, CommentToken, DoctypeToken, ElementLocation, EofToken, Location,
    TagToken, Token, TokenType,
};
pub use parser::{
    parse, parse_fragment, parse_fragment_with_context, parse_fragment_with_context_and_errors,
    parse_fragment_with_errors, parse_with_errors, ParserOptions, ParserOutput,
};
pub use serializer::{serialize, serialize_outer, SerializerOptions};
pub use tree_adapters::default::{Node, NodeData, NodeRef};
