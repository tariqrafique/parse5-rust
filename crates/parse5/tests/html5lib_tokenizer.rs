use parse5::common::token::{Token, TokenType};
use parse5::tokenizer::{State, Tokenizer, TokenizerOptions};
use serde::Deserialize;
use serde_json::{json, Map, Value};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct TestSet {
    tests: Vec<TestCase>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct TestCase {
    description: String,
    input: String,
    #[serde(skip)]
    input_utf16: Option<Vec<u16>>,
    output: Vec<Value>,
    #[serde(default)]
    errors: Vec<ExpectedError>,
    #[serde(default)]
    initial_states: Vec<String>,
    #[serde(default)]
    last_start_tag: Option<String>,
    #[serde(default)]
    double_escaped: bool,
}

#[derive(Debug, Deserialize)]
struct ExpectedError {
    code: String,
    line: usize,
    col: usize,
}

#[test]
fn html5lib_tokenizer_test1_fixture_matches_upstream() {
    run_tokenizer_fixture(&tokenizer_fixture_dir().join("test1.test"));
}

#[test]
fn html5lib_tokenizer_fixtures_match_upstream() {
    let mut fixtures = fs::read_dir(tokenizer_fixture_dir())
        .expect("tokenizer fixture directory is readable")
        .map(|entry| entry.expect("tokenizer fixture entry is readable").path())
        .filter(|path| path.extension().is_some_and(|ext| ext == "test"))
        .collect::<Vec<_>>();

    fixtures.sort();

    for fixture in fixtures {
        run_tokenizer_fixture(&fixture);
    }
}

fn run_tokenizer_fixture(path: &Path) {
    let fixture = fs::read_to_string(path).expect("tokenizer fixture is readable");
    let fixture_json: Value =
        serde_json::from_str(&fixture).expect("tokenizer fixture is valid JSON");
    if fixture_json.get("tests").is_none() {
        return;
    }
    let test_set: TestSet =
        serde_json::from_value(fixture_json).expect("tokenizer fixture has valid tests");
    let fixture_name = path
        .file_name()
        .and_then(|name| name.to_str())
        .expect("fixture has a UTF-8 file name");

    for mut test_case in test_set.tests {
        if test_case.double_escaped {
            unescape_double_escaped_io(&mut test_case);
        }

        let initial_states = if test_case.initial_states.is_empty() {
            vec!["Data state".to_string()]
        } else {
            test_case.initial_states.clone()
        };

        for initial_state in initial_states {
            let mut tokenizer = Tokenizer::new(TokenizerOptions {
                source_code_location_info: true,
            });

            tokenizer.state = state_from_html5lib_name(&initial_state);

            if let Some(last_start_tag) = &test_case.last_start_tag {
                tokenizer.last_start_tag_name = last_start_tag.clone();
            }

            if let Some(input_utf16) = &test_case.input_utf16 {
                tokenizer.write_utf16(input_utf16, true);
            } else {
                tokenizer.write(&test_case.input, true);
            }

            let output = tokenizer.finish();
            let actual_tokens = html5lib_tokens(&output.tokens);
            let mut actual_errors = output
                .errors
                .iter()
                .map(|error| {
                    json!({
                        "code": error.code.as_str(),
                        "line": error.location.start_line,
                        "col": error.location.start_col,
                    })
                })
                .collect::<Vec<_>>();
            let mut expected_errors = test_case
                .errors
                .iter()
                .map(|error| {
                    json!({
                        "code": error.code,
                        "line": error.line,
                        "col": error.col,
                    })
                })
                .collect::<Vec<_>>();
            sort_errors(&mut actual_errors);
            sort_errors(&mut expected_errors);

            assert_eq!(
                actual_tokens, test_case.output,
                "{fixture_name}: {} ({initial_state})",
                test_case.description
            );
            assert_eq!(
                actual_errors, expected_errors,
                "{fixture_name}: {} ({initial_state})",
                test_case.description
            );
        }
    }
}

fn tokenizer_fixture_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures/parse5/test/data/html5lib-tests/tokenizer")
}

fn html5lib_tokens(tokens: &[Token]) -> Vec<Value> {
    let mut output = Vec::new();

    for token in tokens {
        match token {
            Token::Doctype(token) => output.push(json!([
                "DOCTYPE",
                token.name,
                token.public_id,
                token.system_id,
                !token.force_quirks
            ])),
            Token::Tag(token) if token.token_type == TokenType::StartTag => {
                let attrs = token
                    .attrs
                    .iter()
                    .map(|attr| (attr.name.clone(), Value::String(attr.value.clone())))
                    .collect::<Map<_, _>>();
                let mut serialized = vec![
                    Value::String("StartTag".to_string()),
                    Value::String(token.tag_name.clone()),
                    Value::Object(attrs),
                ];

                if token.self_closing {
                    serialized.push(Value::Bool(true));
                }

                output.push(Value::Array(serialized));
            }
            Token::Tag(token) if token.token_type == TokenType::EndTag => {
                output.push(json!(["EndTag", token.tag_name.to_lowercase()]))
            }
            Token::Comment(token) => output.push(json!(["Comment", token.data])),
            Token::Character(token) => {
                let token_chars = lossless_utf16_string(&token.raw_chars);

                if let Some(Value::Array(previous)) = output.last_mut() {
                    if previous.first() == Some(&Value::String("Character".to_string())) {
                        let previous_chars = previous
                            .get(1)
                            .and_then(Value::as_str)
                            .expect("character token has string payload")
                            .to_string();
                        previous[1] = Value::String(format!("{previous_chars}{token_chars}"));
                        continue;
                    }
                }

                output.push(json!(["Character", token_chars]));
            }
            Token::Eof(_) => {}
            _ => panic!("unexpected token: {token:?}"),
        }
    }

    output
}

fn state_from_html5lib_name(name: &str) -> State {
    match name {
        "Data state" => State::Data,
        "RCDATA state" => State::RcData,
        "RAWTEXT state" => State::RawText,
        "Script data state" => State::ScriptData,
        "PLAINTEXT state" => State::PlainText,
        "CDATA section state" => State::CDataSection,
        _ => panic!("unsupported html5lib tokenizer state: {name}"),
    }
}

fn sort_errors(errors: &mut [Value]) {
    errors.sort_by_key(|error| {
        let line = error["line"].as_u64().unwrap_or_default();
        let col = error["col"].as_u64().unwrap_or_default();
        let code = error["code"].as_str().unwrap_or_default().to_string();

        (line, col, code)
    });
}

fn unescape_double_escaped_io(test_case: &mut TestCase) {
    test_case.input_utf16 = Some(unicode_unescape_utf16(&test_case.input));
    test_case.input = lossless_utf16_string(test_case.input_utf16.as_deref().unwrap_or_default());

    for token in &mut test_case.output {
        let Value::Array(token) = token else {
            continue;
        };

        if let Some(Value::String(value)) = token.get_mut(1) {
            *value = lossless_utf16_string(&unicode_unescape_utf16(value));
        }
    }
}

fn unicode_unescape_utf16(value: &str) -> Vec<u16> {
    let mut result = Vec::new();
    let mut chars = value.chars().peekable();

    while let Some(ch) = chars.next() {
        if ch != '\\' || !matches!(chars.peek(), Some('u' | 'U')) {
            let mut buffer = [0; 2];
            result.extend_from_slice(ch.encode_utf16(&mut buffer));
            continue;
        }

        chars.next();
        let hex = chars.by_ref().take(4).collect::<String>();

        if hex.len() != 4 {
            result.extend("\\".encode_utf16());
            result.extend("u".encode_utf16());
            result.extend(hex.encode_utf16());
            continue;
        }

        match u16::from_str_radix(&hex, 16) {
            Ok(decoded) => result.push(decoded),
            Err(_) => {
                result.extend("\\".encode_utf16());
                result.extend("u".encode_utf16());
                result.extend(hex.encode_utf16());
            }
        }
    }

    result
}

fn lossless_utf16_string(units: &[u16]) -> String {
    let mut result = String::new();
    let mut idx = 0;

    while idx < units.len() {
        let unit = units[idx];

        if (0xd800..=0xdbff).contains(&unit) && idx + 1 < units.len() {
            let next = units[idx + 1];

            if (0xdc00..=0xdfff).contains(&next) {
                let code_point =
                    0x10000 + (((unit as u32 - 0xd800) << 10) | (next as u32 - 0xdc00));

                if let Some(ch) = char::from_u32(code_point) {
                    result.push(ch);
                    idx += 2;
                    continue;
                }
            }
        }

        if (0xd800..=0xdfff).contains(&unit) {
            result.push_str(&format!("\\u{unit:04X}"));
        } else if let Some(ch) = char::from_u32(unit as u32) {
            result.push(ch);
        } else {
            result.push('\u{fffd}');
        }

        idx += 1;
    }

    result
}
