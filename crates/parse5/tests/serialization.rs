use parse5::{parse, serialize, ParserOptions, SerializerOptions};
use serde::Deserialize;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct FixtureOptions {
    #[serde(default = "default_scripting_enabled")]
    scripting_enabled: bool,
}

#[derive(Debug, Deserialize)]
struct SerializationTest {
    name: String,
    #[serde(default)]
    options: Option<FixtureOptions>,
    input: String,
    expected: String,
}

#[test]
fn upstream_serialization_fixture_matches_parse5() {
    let fixture_path = serialization_fixture_path();
    let fixture = fs::read_to_string(&fixture_path).expect("serialization fixture is readable");
    let tests: Vec<SerializationTest> =
        serde_json::from_str(&fixture).expect("serialization fixture is valid JSON");

    for (idx, test) in tests.into_iter().enumerate() {
        let scripting_enabled = test
            .options
            .as_ref()
            .map(|options| options.scripting_enabled)
            .unwrap_or_else(default_scripting_enabled);
        let parser_options = ParserOptions {
            scripting_enabled,
            source_code_location_info: false,
        };
        let serializer_options = SerializerOptions { scripting_enabled };
        let document = parse(&test.input, parser_options);
        let actual = serialize(&document, serializer_options);

        assert_eq!(actual, test.expected, "{idx}.{}", test.name);
    }
}

fn serialization_fixture_path() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures/parse5/test/data/serialization/tests.json")
}

fn default_scripting_enabled() -> bool {
    true
}
