use parse5::parser::Parser;
use parse5::{parse, serialize, serialize_outer, ParserOptions, SerializerOptions};

/// Deep enough that recursive tree teardown or serialization would overflow
/// the 2 MiB stack of a test thread.
const DEEP_NESTING: usize = 5_000;

#[test]
fn deeply_nested_documents_serialize_and_drop_without_overflowing_the_stack() {
    let html = "<div>".repeat(DEEP_NESTING);
    let document = parse(&html, ParserOptions::default());

    let serialized = serialize(&document, SerializerOptions::default());
    let expected = format!(
        "<html><head></head><body>{}{}</body></html>",
        "<div>".repeat(DEEP_NESTING),
        "</div>".repeat(DEEP_NESTING)
    );
    assert_eq!(serialized, expected);
    assert_eq!(
        serialize_outer(&document, SerializerOptions::default()),
        expected
    );

    drop(document);
}

#[test]
fn chunked_input_matches_single_write_when_character_references_are_split() {
    let html =
        "<p title=\"&notin;&amp\">a&notit; b&CounterClockwiseContourIntegral;c&#x41;&ampx</p>";

    for options in [
        ParserOptions::default(),
        ParserOptions {
            source_code_location_info: true,
            ..ParserOptions::default()
        },
    ] {
        let expected = serialize(&parse(html, options), SerializerOptions::default());

        for split in 1..html.len() {
            let mut parser = Parser::new(options, None, None);
            parser.write_chunk(&html[..split], false);
            parser.write_chunk(&html[split..], true);

            assert_eq!(
                serialize(&parser.document, SerializerOptions::default()),
                expected,
                "input split at byte {split}"
            );
        }
    }
}
