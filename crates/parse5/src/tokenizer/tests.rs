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

/// Ordered `(code, start_line, start_col, start_offset, end_line, end_col, end_offset)`
/// lists recorded from JS parse5 8.0.1 `parseFragment(html, { sourceCodeLocationInfo:
/// true, onParseError })`. parse5 reports preprocessor, tokenizer and tree-construction
/// errors to one callback in occurrence order, so mixed documents must interleave them.
#[test]
fn mixed_parse_errors_match_parse5_occurrence_order() {
    type Expected = (&'static str, usize, usize, usize, usize, usize, usize);
    let trimmed_custom_header_cpe = concat!(
        "<template>\n",
        "    <lightning-radio-group class='displayModeClass' name=\"select_displayMode\" label={inputValues.displayMode.label}\n",
        "        options={displayChoicesAsOptions} value={inputValues.displayMode.value} type=\"radio\"\n",
        "        onchange={handleValueChange}\n",
        "    </lightning-radio-group>\n",
        "\n",
        "    <lightning-input type=\"number\" label={inputValues.orderNumber.label} name=\"select_orderNumber\"\n",
        "        value={inputValues.orderNumber.value} onchange={handleValueChange}></lightning-input>\n",
        "</template>\n",
    );
    let cases: [(&str, &str, &[Expected]); 3] = [
        (
            "unclosed start tag before its end tag (tokenizer then tree)",
            "<template><div a={b}\n</div></template>",
            &[
                ("unexpected-character-in-attribute-name", 2, 1, 21, 2, 1, 21),
                ("unexpected-solidus-in-tag", 2, 3, 23, 2, 3, 23),
                (
                    "closing-of-element-with-open-child-elements",
                    2,
                    7,
                    27,
                    2,
                    18,
                    38,
                ),
            ],
        ),
        (
            "control character inside attribute name (preprocessor then tokenizer)",
            "<template><div x\u{0001}\"y></div></template>",
            &[
                ("control-character-in-input-stream", 1, 17, 16, 1, 17, 16),
                (
                    "unexpected-character-in-attribute-name",
                    1,
                    18,
                    17,
                    1,
                    18,
                    17,
                ),
            ],
        ),
        (
            "trimmed LightningFlowComponents customHeaderCpe.html",
            trimmed_custom_header_cpe,
            &[
                (
                    "unexpected-character-in-attribute-name",
                    5,
                    5,
                    261,
                    5,
                    5,
                    261,
                ),
                ("unexpected-solidus-in-tag", 5, 7, 263, 5, 7, 263),
                (
                    "closing-of-element-with-open-child-elements",
                    9,
                    1,
                    480,
                    9,
                    12,
                    491,
                ),
            ],
        ),
    ];

    for (name, html, expected) in cases {
        let output = crate::parse_fragment_with_errors(html, crate::ParserOptions::default());
        let actual: Vec<Expected> = output
            .errors
            .iter()
            .map(|err| {
                let l = &err.location;
                (
                    err.code.as_str(),
                    l.start_line,
                    l.start_col,
                    l.start_offset,
                    l.end_line,
                    l.end_col,
                    l.end_offset,
                )
            })
            .collect();
        assert_eq!(actual, expected, "{name}");
    }
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
    let output = Tokenizer::tokenize("a&amp;b&nbsp;&notin;&notit;", TokenizerOptions::default());

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
    let output = Tokenizer::tokenize("<a href=\"a&amp;b&notit;\">", TokenizerOptions::default());

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
