use parse5::common::token::{Location, Token};
use parse5::tokenizer::{State, Tokenizer, TokenizerOptions};

#[derive(Debug)]
struct TokenizerLocationCase {
    initial_state: State,
    last_start_tag_name: &'static str,
    in_foreign_node: bool,
    html_chunks: Vec<&'static str>,
}

#[test]
fn upstream_tokenizer_location_info_cases_match_parse5() {
    for test_case in tokenizer_location_cases() {
        let mut tokenizer = Tokenizer::new(TokenizerOptions {
            source_code_location_info: true,
        });

        tokenizer.preprocessor.buffer_waterline = 8;
        tokenizer.state = test_case.initial_state;
        tokenizer.last_start_tag_name = test_case.last_start_tag_name.to_string();
        tokenizer.in_foreign_node = test_case.in_foreign_node;

        for (idx, chunk) in test_case.html_chunks.iter().enumerate() {
            tokenizer.write(chunk, idx == test_case.html_chunks.len() - 1);
        }

        let html = test_case.html_chunks.join("");
        let normalized_html = normalize_new_line(&html);
        let lines = normalized_html.split('\n').collect::<Vec<_>>();
        let output = tokenizer.finish();
        let mut chunk_idx = 0usize;

        for token in output.tokens {
            if let Token::Eof(token) = token {
                let location = token.location.expect("EOF token has a location");
                assert_eq!(location.start_offset, html.encode_utf16().count());
                assert_eq!(location.end_offset, location.start_offset);
                assert_eq!(chunk_idx, test_case.html_chunks.len());
            } else {
                let location = token_location(&token).expect("token has a location");
                let expected = test_case.html_chunks[chunk_idx];
                assert_location(&location, expected, &html, &lines, &token);
                chunk_idx += 1;
            }
        }
    }
}

fn tokenizer_location_cases() -> Vec<TokenizerLocationCase> {
    vec![
        TokenizerLocationCase {
            initial_state: State::Data,
            last_start_tag_name: "",
            in_foreign_node: false,
            html_chunks: vec![
                "\r\n",
                "<!DOCTYPE html>",
                "\n",
                "<!-- Test -->",
                "\n",
                "<head>",
                "\n   ",
                "<meta charset=\"utf-8\">",
                "<title>",
                "   ",
                "node.js",
                "\u{0}",
                "</title>",
                "\n",
                "</head>",
                "\n",
                "<body id=\"front\">",
                "\n",
                "<div id=\"intro\">",
                "\n   ",
                "<p\n>",
                "\n       ",
                "Node.js",
                " ",
                "is",
                " ",
                "a",
                "\n       ",
                "platform",
                " ",
                "built",
                " ",
                "on",
                "\n       ",
                "<a href=\"http://code.google.com/p/v8/\">",
                "\n       ",
                "Chrome's",
                " ",
                "JavaScript",
                " ",
                "runtime",
                "\n       ",
                "</a>",
                "\n",
                "</div>",
                "</body>",
            ],
        },
        TokenizerLocationCase {
            initial_state: State::RcData,
            last_start_tag_name: "title",
            in_foreign_node: false,
            html_chunks: vec![
                "<div>Test",
                " \n   ",
                "hey",
                " ",
                "ya!",
                "</title>",
                "<!--Yo-->",
            ],
        },
        TokenizerLocationCase {
            initial_state: State::RawText,
            last_start_tag_name: "style",
            in_foreign_node: false,
            html_chunks: vec![
                ".header{",
                " \n   ",
                "color:red;",
                "\n",
                "}",
                "</style>",
                "Some",
                " ",
                "text",
            ],
        },
        TokenizerLocationCase {
            initial_state: State::ScriptData,
            last_start_tag_name: "script",
            in_foreign_node: false,
            html_chunks: vec![
                "var",
                " ",
                "a=c",
                " ",
                "-",
                " ",
                "d;",
                "\n",
                "a<--d;",
                "</script>",
                "<div>",
            ],
        },
        TokenizerLocationCase {
            initial_state: State::PlainText,
            last_start_tag_name: "plaintext",
            in_foreign_node: false,
            html_chunks: vec!["Text", " \n", "Test</plaintext><div>"],
        },
        TokenizerLocationCase {
            initial_state: State::Data,
            last_start_tag_name: "",
            in_foreign_node: false,
            html_chunks: vec![
                "\n",
                "<!-- regular comment -->",
                "<! bogus comment >",
                "<? another bogus comment >",
                "</!yet another bogus comment>",
                "<![CDATA[ cdata as a bogus comment >",
            ],
        },
        TokenizerLocationCase {
            initial_state: State::Data,
            last_start_tag_name: "",
            in_foreign_node: true,
            html_chunks: vec![
                "<a>",
                "<![CDATA[ ",
                "CDATA",
                " ]]>",
                "<test>",
                " <![CDATA[ ]]>\n",
            ],
        },
    ]
}

fn token_location(token: &Token) -> Option<Location> {
    match token {
        Token::Doctype(token) => token.location,
        Token::Tag(token) => token.location.as_ref().map(|location| location.location),
        Token::Comment(token) => token.location,
        Token::Character(token) => token.location,
        Token::Eof(token) => token.location,
    }
}

fn assert_location(location: &Location, expected: &str, html: &str, lines: &[&str], token: &Token) {
    let actual = utf16_substring(html, location.start_offset, location.end_offset);
    assert_eq!(actual, expected, "offset location mismatch for {token:?}");

    let actual = substring_by_line_col(lines, location);
    assert_eq!(
        actual,
        normalize_new_line(expected),
        "line/column location mismatch for {token:?}"
    );
}

fn substring_by_line_col(lines: &[&str], location: &Location) -> String {
    let selected = &lines[location.start_line - 1..location.end_line];

    if selected.len() == 1 {
        return utf16_substring(selected[0], location.start_col - 1, location.end_col - 1);
    }

    let mut output = String::new();
    output.push_str(&utf16_substring(
        selected[0],
        location.start_col - 1,
        utf16_len(selected[0]),
    ));

    for middle_line in &selected[1..selected.len() - 1] {
        output.push('\n');
        output.push_str(middle_line);
    }

    output.push('\n');
    output.push_str(&utf16_substring(
        selected[selected.len() - 1],
        0,
        location.end_col - 1,
    ));
    output
}

fn utf16_substring(input: &str, start: usize, end: usize) -> String {
    String::from_utf16_lossy(
        &input
            .encode_utf16()
            .skip(start)
            .take(end.saturating_sub(start))
            .collect::<Vec<_>>(),
    )
}

fn utf16_len(input: &str) -> usize {
    input.encode_utf16().count()
}

fn normalize_new_line(input: &str) -> String {
    input.replace("\r\n", "\n")
}
