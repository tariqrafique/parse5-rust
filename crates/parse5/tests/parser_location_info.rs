use parse5::tree_adapters::default::{
    get_attr_list, get_child_nodes, get_document_type_node_name, get_document_type_node_public_id,
    get_document_type_node_system_id, get_node_source_code_location, NodeRef,
};
use parse5::{parse, parse_fragment, serialize_outer, Location, ParserOptions};
use std::fs;
use std::path::{Path, PathBuf};

#[test]
fn upstream_parser_location_info_fixtures_match_parse5() {
    let mut fixtures = fs::read_dir(location_info_fixture_dir())
        .expect("location-info fixture directory is readable")
        .map(|entry| {
            entry
                .expect("location-info fixture entry is readable")
                .path()
                .join("data.html")
        })
        .collect::<Vec<_>>();

    fixtures.sort();

    for fixture in fixtures {
        let html = normalize_new_line(
            &fs::read_to_string(&fixture).expect("location-info data.html is readable"),
        );
        let fixture_name = fixture
            .parent()
            .and_then(Path::file_name)
            .and_then(|name| name.to_str())
            .expect("fixture has a UTF-8 directory name");
        let lines = html.split('\n').collect::<Vec<_>>();
        let document = parse(
            &html,
            ParserOptions {
                scripting_enabled: true,
                source_code_location_info: true,
            },
        );

        walk_tree(&document, &mut |node| {
            let location = get_node_source_code_location(node).unwrap_or_else(|| {
                panic!(
                    "{fixture_name}: expected source code location for {}",
                    node.borrow().node_name()
                )
            });
            let serialized_node = serialized_node_for_location_assertion(node);
            let node_name = node.borrow().node_name().to_string();

            assert_location(
                &location.location,
                &serialized_node,
                &html,
                &lines,
                fixture_name,
                &node_name,
            );

            if node.borrow().is_element() {
                assert_start_tag_location(
                    &location,
                    &serialized_node,
                    &html,
                    &lines,
                    fixture_name,
                    &node_name,
                );

                if let Some(end_tag) = &location.end_tag {
                    assert_end_tag_location(
                        end_tag,
                        &serialized_node,
                        &html,
                        &lines,
                        fixture_name,
                        &node_name,
                    );
                }

                if location.attrs.is_empty() {
                    assert_eq!(
                        get_attr_list(node).len(),
                        0,
                        "{fixture_name}: {node_name} has attributes but no attribute locations"
                    );
                } else {
                    for (attr_name, attr_location) in &location.attrs {
                        let relative_start = attr_location
                            .start_offset
                            .checked_sub(location.location.start_offset)
                            .unwrap_or_else(|| {
                                panic!(
                                    "{fixture_name}: {node_name}.{attr_name} starts before element; attr {}:{} offset {}, element {}:{} offset {}",
                                    attr_location.start_line,
                                    attr_location.start_col,
                                    attr_location.start_offset,
                                    location.location.start_line,
                                    location.location.start_col,
                                    location.location.start_offset,
                                )
                            });
                        let relative_end = attr_location
                            .end_offset
                            .checked_sub(location.location.start_offset)
                            .unwrap_or_else(|| {
                                panic!(
                                    "{fixture_name}: {node_name}.{attr_name} ends before element; attr {}:{} offset {}, element {}:{} offset {}",
                                    attr_location.end_line,
                                    attr_location.end_col,
                                    attr_location.end_offset,
                                    location.location.start_line,
                                    location.location.start_col,
                                    location.location.start_offset,
                                )
                            });
                        let expected =
                            utf16_substring(&serialized_node, relative_start, relative_end);
                        assert_location(
                            attr_location,
                            &expected,
                            &html,
                            &lines,
                            fixture_name,
                            &node_name,
                        );
                    }
                }
            }
        });
    }
}

#[test]
fn implicitly_closed_p_location_ends_before_next_p_start_tag_like_upstream() {
    let html = "<p>1<p class=\"2\">3";
    let fragment = parse_fragment(html, location_parser_options());
    let first_p = child_at(&fragment, 0);
    let location = get_node_source_code_location(&first_p).expect("first p has a location");

    assert_eq!(
        utf16_substring(
            html,
            location.location.start_offset,
            location.location.end_offset
        ),
        "<p>1"
    );
}

#[test]
fn element_with_closing_tag_location_includes_end_tag_like_upstream() {
    let html = "<i>1</i>2";
    let fragment = parse_fragment(html, location_parser_options());
    let first_child = child_at(&fragment, 0);
    let location = get_node_source_code_location(&first_child).expect("first child has a location");

    assert_eq!(
        utf16_substring(
            html,
            location.location.start_offset,
            location.location.end_offset
        ),
        "<i>1</i>"
    );
}

#[test]
fn parse_fragment_exposes_location_info_like_upstream() {
    let html = "<html><head></head><body>foo</body></html>";
    let fragment = parse_fragment(html, location_parser_options());
    let first_child = child_at(&fragment, 0);

    assert!(get_node_source_code_location(&first_child).is_some());
}

#[test]
fn implicit_head_location_remains_null_like_upstream() {
    let document = parse(
        "<!doctype><body>Testing location</body>",
        location_parser_options(),
    );
    let html = child_at(&document, 1);
    let head = child_at(&html, 0);

    assert!(head.borrow().source_code_location_is_set());
    assert!(get_node_source_code_location(&head).is_none());
}

#[test]
fn template_location_info_parses_without_panicking_like_upstream() {
    let fragment = parse_fragment("<template>hello</template>", location_parser_options());
    assert_eq!(get_child_nodes(&fragment).unwrap_or_default().len(), 1);
}

#[test]
fn empty_attribute_location_is_attached_like_upstream() {
    let html = "<div test-attr></div>";
    let fragment = parse_fragment(html, location_parser_options());
    let div = child_at(&fragment, 0);
    let location = get_node_source_code_location(&div).expect("div has a location");

    assert!(location.attrs.contains_key("test-attr"));
}

#[test]
fn unconsumed_character_keeps_script_end_tag_line_like_upstream() {
    let html =
        "<html><body><script>\n  var x = window.scrollY <\n      100;\n</script>\n</body></html>";
    let document = parse(html, location_parser_options());
    let html_element = child_at(&document, 0);
    let body = child_at(&html_element, 1);
    let script = child_at(&body, 0);
    let script_location = get_node_source_code_location(&script).expect("script has a location");

    assert_eq!(
        script_location.end_tag.as_ref().map(|tag| tag.start_line),
        Some(4)
    );
}

#[test]
fn start_tag_location_is_available_when_end_tag_is_missing_like_upstream() {
    let html = "<p>test";
    let fragment = parse_fragment(html, location_parser_options());
    let p = child_at(&fragment, 0);
    let location = get_node_source_code_location(&p).expect("p has a location");
    let lines = [html];

    assert_location(&location.location, html, html, &lines, "gh-181", "p");
    assert_start_tag_location(&location, html, html, &lines, "gh-181", "p");
    assert!(location.end_tag.is_none());
}

#[test]
fn adjusted_svg_element_end_tag_location_is_available_like_upstream() {
    let html = "<svg><foreignObject></foreignObject></svg>";
    let fragment = parse_fragment(html, location_parser_options());
    let svg = child_at(&fragment, 0);
    let foreign_object = child_at(&svg, 0);
    let location =
        get_node_source_code_location(&foreign_object).expect("foreignObject has a location");
    let start = location.start_tag.expect("foreignObject has a start tag");
    let end = location.end_tag.expect("foreignObject has an end tag");

    assert_eq!(
        utf16_substring(html, start.start_offset, end.end_offset),
        "<foreignObject></foreignObject>"
    );
}

#[test]
fn escaped_script_text_location_matches_text_content_like_upstream() {
    let html = "<script>\"<!--\";</script>";
    let fragment = parse_fragment(html, location_parser_options());
    let script = child_at(&fragment, 0);
    let text = child_at(&script, 0);
    let script_location = get_node_source_code_location(&script).expect("script has a location");
    let text_location = get_node_source_code_location(&text).expect("script text has a location");
    let lines = [html];

    assert_location(
        &script_location.location,
        html,
        html,
        &lines,
        "gh-265",
        "script",
    );
    assert_start_tag_location(&script_location, html, html, &lines, "gh-265", "script");
    assert_location(
        &text_location.location,
        &utf16_substring(html, 8, 15),
        html,
        &lines,
        "gh-265",
        "#text",
    );
}

#[test]
fn body_location_uses_html_end_tag_when_body_is_not_closed_like_upstream() {
    let html = "<html>\n  <body>\n    <p>test</p>\n</html>\n<!-- comment -->";
    let document = parse(html, location_parser_options());
    let html_element = child_at(&document, 0);
    let body = child_at(&html_element, 1);
    let html_location = get_node_source_code_location(&html_element).expect("html has a location");
    let body_location = get_node_source_code_location(&body).expect("body has a location");
    let html_end_tag = html_location.end_tag.expect("html has an end tag");

    assert_eq!(html_end_tag.start_offset, body_location.location.end_offset);
    assert_eq!(html_end_tag.start_line, body_location.location.end_line);
    assert_eq!(html_end_tag.start_col, body_location.location.end_col);
    assert_ne!(html_location.location.end_offset, utf16_len(html));
}

#[test]
fn html_and_body_locations_extend_to_eof_when_html_end_tag_is_missing_like_upstream() {
    let html = "<html>\n  <body>\n    <p>test</p>\n    <!-- comment -->";
    let document = parse(html, location_parser_options());
    let html_element = child_at(&document, 0);
    let body = child_at(&html_element, 1);
    let html_location = get_node_source_code_location(&html_element).expect("html has a location");
    let body_location = get_node_source_code_location(&body).expect("body has a location");

    assert_eq!(html_location.location.end_offset, utf16_len(html));
    assert_eq!(body_location.location.end_offset, utf16_len(html));
}

fn location_info_fixture_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/parse5/test/data/location-info")
}

fn location_parser_options() -> ParserOptions {
    ParserOptions {
        scripting_enabled: true,
        source_code_location_info: true,
    }
}

fn child_at(parent: &NodeRef, index: usize) -> NodeRef {
    get_child_nodes(parent)
        .expect("node has child nodes")
        .get(index)
        .cloned()
        .expect("child exists")
}

fn walk_tree(parent: &NodeRef, handler: &mut impl FnMut(&NodeRef)) {
    for node in get_child_nodes(parent).unwrap_or_default() {
        if node.borrow().is_element() {
            walk_tree(&node, handler);
        }

        handler(&node);
    }
}

fn serialized_node_for_location_assertion(node: &NodeRef) -> String {
    if node.borrow().is_document_type() {
        format!(
            "<{}>",
            serialize_doctype_content(
                &get_document_type_node_name(node),
                &get_document_type_node_public_id(node),
                &get_document_type_node_system_id(node),
            )
        )
    } else {
        serialize_outer(node, parse5::SerializerOptions::default())
    }
}

fn serialize_doctype_content(name: &str, public_id: &str, system_id: &str) -> String {
    let mut content = String::from("!DOCTYPE ");

    if !name.is_empty() {
        content.push_str(name);
    }

    if !public_id.is_empty() {
        content.push_str(" PUBLIC ");
        content.push_str(&enquote_doctype_id(public_id));
    } else if !system_id.is_empty() {
        content.push_str(" SYSTEM");
    }

    if !system_id.is_empty() {
        content.push(' ');
        content.push_str(&enquote_doctype_id(system_id));
    }

    content
}

fn enquote_doctype_id(id: &str) -> String {
    let quote = if id.contains('"') { '\'' } else { '"' };
    format!("{quote}{id}{quote}")
}

fn assert_start_tag_location(
    location: &parse5::ElementLocation,
    serialized_node: &str,
    html: &str,
    lines: &[&str],
    fixture_name: &str,
    node_name: &str,
) {
    let start_tag = location
        .start_tag
        .as_ref()
        .expect("element location has a start tag");
    let expected = utf16_substring(
        serialized_node,
        0,
        start_tag.end_offset - start_tag.start_offset,
    );

    assert_location(start_tag, &expected, html, lines, fixture_name, node_name);
}

fn assert_end_tag_location(
    end_tag: &Location,
    serialized_node: &str,
    html: &str,
    lines: &[&str],
    fixture_name: &str,
    node_name: &str,
) {
    let length = end_tag.end_offset - end_tag.start_offset;
    let serialized_length = utf16_len(serialized_node);
    let expected = utf16_substring(
        serialized_node,
        serialized_length - length,
        serialized_length,
    );

    assert_location(end_tag, &expected, html, lines, fixture_name, node_name);
}

fn assert_location(
    location: &Location,
    expected: &str,
    html: &str,
    lines: &[&str],
    fixture_name: &str,
    node_name: &str,
) {
    let expected = remove_new_lines(expected);
    let actual = remove_new_lines(&utf16_substring(
        html,
        location.start_offset,
        location.end_offset,
    ));
    assert_eq!(
        actual, expected,
        "{fixture_name}: offset location mismatch for {node_name} at {}:{}",
        location.start_line, location.start_col
    );

    let actual = remove_new_lines(&substring_by_line_col(lines, location));
    assert_eq!(
        actual, expected,
        "{fixture_name}: line/column location mismatch for {node_name} at {}:{}",
        location.start_line, location.start_col
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

fn remove_new_lines(input: &str) -> String {
    input.replace(['\n', '\r'], "")
}
