use parse5::common::html::{NS_HTML, NS_MATHML, NS_SVG};
use parse5::tree_adapters::default::{
    create_element, get_attr_list, get_child_nodes, get_comment_node_content,
    get_document_type_node_name, get_document_type_node_public_id,
    get_document_type_node_system_id, get_namespace_uri, get_tag_name, get_template_content,
    get_text_node_content, NodeRef,
};
use parse5::{parse_fragment_with_context_and_errors, parse_with_errors, ParserOptions};
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug)]
struct TreeConstructionTest {
    input: String,
    expected: String,
    expected_errors: Vec<String>,
    scripting_enabled: bool,
    fragment_context: Option<NodeRef>,
    line_number: usize,
}

#[test]
fn html5lib_tree_construction_tests1_fixture_matches_upstream() {
    run_tree_construction_fixture(
        &fork_tree_construction_fixture_dir().join("tests1.dat"),
        ErrorAssertion::Compare,
        false,
    );
}

#[test]
fn html5lib_tree_construction_adoption01_fixture_matches_upstream() {
    run_tree_construction_fixture(
        &fork_tree_construction_fixture_dir().join("adoption01.dat"),
        ErrorAssertion::Compare,
        false,
    );
}

#[test]
fn html5lib_fork_tree_construction_fixtures_match_upstream() {
    let mut fixtures = fs::read_dir(fork_tree_construction_fixture_dir())
        .expect("tree-construction fixture directory is readable")
        .map(|entry| {
            entry
                .expect("tree-construction fixture entry is readable")
                .path()
        })
        .filter(|path| path.extension().is_some_and(|ext| ext == "dat"))
        .collect::<Vec<_>>();

    fixtures.sort();

    for fixture in fixtures {
        run_tree_construction_fixture(&fixture, ErrorAssertion::Compare, true);
    }
}

#[test]
fn html5lib_upstream_tree_construction_fixtures_match_upstream() {
    let mut fixtures = fs::read_dir(upstream_tree_construction_fixture_dir())
        .expect("upstream tree-construction fixture directory is readable")
        .map(|entry| {
            entry
                .expect("upstream tree-construction fixture entry is readable")
                .path()
        })
        .filter(|path| path.extension().is_some_and(|ext| ext == "dat"))
        .collect::<Vec<_>>();

    fixtures.sort();

    for fixture in fixtures {
        run_tree_construction_fixture(&fixture, ErrorAssertion::Ignore, false);
    }
}

#[derive(Debug, Clone, Copy)]
enum ErrorAssertion {
    Compare,
    Ignore,
}

fn run_tree_construction_fixture(
    path: &Path,
    error_assertion: ErrorAssertion,
    skip_upstream_expected_failures: bool,
) {
    let fixture = fs::read_to_string(path).expect("tree-construction fixture is readable");
    let fixture_name = path
        .file_name()
        .and_then(|name| name.to_str())
        .expect("fixture has a UTF-8 file name");

    for (idx, test) in parse_dat_file(&fixture).into_iter().enumerate() {
        if skip_upstream_expected_failures && is_upstream_expected_failure(fixture_name, idx) {
            continue;
        }

        let options = ParserOptions {
            scripting_enabled: test.scripting_enabled,
            source_code_location_info: false,
        };
        let output = if test.fragment_context.is_some() {
            parse_fragment_with_context_and_errors(test.fragment_context, &test.input, options)
        } else {
            parse_with_errors(&test.input, options)
        };
        let actual = serialize_to_dat_file_format(&output.document);
        let mut actual_errors = output
            .errors
            .iter()
            .map(format_parse_error)
            .collect::<Vec<_>>();
        let mut expected_errors = test.expected_errors;

        actual_errors.sort();
        expected_errors.sort();

        assert_eq!(
            actual, test.expected,
            "{fixture_name}: case {idx} starting at line {}",
            test.line_number
        );
        if matches!(error_assertion, ErrorAssertion::Compare) {
            assert_eq!(
                actual_errors, expected_errors,
                "{fixture_name}: case {idx} starting at line {}",
                test.line_number
            );
        }
    }
}

fn is_upstream_expected_failure(fixture_name: &str, idx: usize) -> bool {
    fixture_name == "foreign-fragment.dat" && matches!(idx, 0 | 1 | 38 | 40 | 47 | 48)
}

fn format_parse_error(error: &parse5::ParserError) -> String {
    let mut location = format!(
        "({}:{}",
        error.location.start_line, error.location.start_col
    );

    if error.location.start_line != error.location.end_line
        || error.location.start_col != error.location.end_col
    {
        location.push_str(&format!(
            "-{}:{}",
            error.location.end_line, error.location.end_col
        ));
    }

    format!("{location}) {}", error.code.as_str())
}

fn fork_tree_construction_fixture_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures/parse5/test/data/html5lib-tests-fork/tree-construction")
}

fn upstream_tree_construction_fixture_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures/parse5/test/data/html5lib-tests/tree-construction")
}

fn parse_dat_file(input: &str) -> Vec<TreeConstructionTest> {
    let mut tests = Vec::<BTreeMap<String, Vec<String>>>::new();
    let mut current_directive = String::new();

    for (idx, line) in input
        .split('\n')
        .map(|line| line.strip_suffix('\r').unwrap_or(line))
        .enumerate()
    {
        if line == "#data" {
            let mut test = BTreeMap::new();
            test.insert("#line".to_string(), vec![(idx + 1).to_string()]);
            tests.push(test);
        }

        if line.starts_with('#') {
            current_directive = line.to_string();

            if let Some(test) = tests.last_mut() {
                test.entry(current_directive.clone()).or_default();
            }
        } else if let Some(test) = tests.last_mut() {
            test.entry(current_directive.clone())
                .or_default()
                .push(line.to_string());
        }
    }

    tests
        .into_iter()
        .map(|test| {
            let fragment_context = test
                .get("#document-fragment")
                .and_then(|lines| lines.first())
                .and_then(|tag_name| create_fragment_context(tag_name));

            TreeConstructionTest {
                input: test
                    .get("#data")
                    .expect("tree-construction test has #data")
                    .join("\n"),
                expected: test
                    .get("#document")
                    .expect("tree-construction test has #document")
                    .join("\n"),
                expected_errors: test.get("#new-errors").cloned().unwrap_or_default(),
                scripting_enabled: !test.contains_key("#script-off"),
                fragment_context,
                line_number: test
                    .get("#line")
                    .and_then(|lines| lines.first())
                    .and_then(|line| line.parse().ok())
                    .unwrap_or_default(),
            }
        })
        .collect()
}

fn create_fragment_context(tag_name: &str) -> Option<NodeRef> {
    if tag_name.is_empty() {
        return None;
    }

    let mut namespace = NS_HTML;
    let mut tag_name = tag_name;
    let parts = tag_name.split_once(' ');

    if let Some((prefix, local_name)) = parts {
        tag_name = local_name;
        namespace = match prefix {
            "svg" => NS_SVG,
            "math" => NS_MATHML,
            _ => NS_HTML,
        };
    }

    Some(create_element(tag_name, namespace, Vec::new()))
}

fn serialize_to_dat_file_format(root: &NodeRef) -> String {
    serialize_node_list(&get_child_nodes(root).unwrap_or_default(), 0)
}

fn serialize_node_list(nodes: &[NodeRef], indent: usize) -> String {
    let mut output = String::new();

    for node in nodes {
        output.push_str(&serialized_tree_indent(indent));

        let node_ref = node.borrow();

        if node_ref.is_comment() {
            output.push_str(&format!("<!-- {} -->\n", get_comment_node_content(node)));
        } else if node_ref.is_text() {
            output.push_str(&format!("\"{}\"\n", get_text_node_content(node)));
        } else if node_ref.is_document_type() {
            let public_id = get_document_type_node_public_id(node);
            let system_id = get_document_type_node_system_id(node);

            output.push_str(&format!("<!DOCTYPE {}", get_document_type_node_name(node)));

            if !public_id.is_empty() || !system_id.is_empty() {
                output.push_str(&format!(" \"{public_id}\" \"{system_id}\""));
            }

            output.push_str(">\n");
        } else if node_ref.is_element() {
            let tag_name = get_tag_name(node);

            output.push_str(&format!(
                "<{}{}>\n",
                serialized_namespace_prefix(node),
                tag_name
            ));

            let mut children_indent = indent + 2;
            let mut serialized_attrs = get_attr_list(node)
                .iter()
                .map(|attr| {
                    let prefix = attr
                        .prefix
                        .as_ref()
                        .map(|prefix| format!("{prefix} "))
                        .unwrap_or_default();

                    format!(
                        "{}{}{}=\"{}\"\n",
                        serialized_tree_indent(children_indent),
                        prefix,
                        attr.name,
                        attr.value
                    )
                })
                .collect::<Vec<_>>();

            serialized_attrs.sort();
            output.push_str(&serialized_attrs.join(""));

            let child_container = if tag_name == "template" && get_namespace_uri(node) == NS_HTML {
                output.push_str(&format!(
                    "{}content\n",
                    serialized_tree_indent(children_indent)
                ));
                children_indent += 2;
                get_template_content(node).unwrap_or_else(|| node.clone())
            } else {
                node.clone()
            };

            output.push_str(&serialize_node_list(
                &get_child_nodes(&child_container).unwrap_or_default(),
                children_indent,
            ));
        }
    }

    output
}

fn serialized_tree_indent(indent: usize) -> String {
    format!("|{}", " ".repeat(indent + 1))
}

fn serialized_namespace_prefix(node: &NodeRef) -> &'static str {
    match get_namespace_uri(node).as_str() {
        NS_SVG => "svg ",
        NS_MATHML => "math ",
        _ => "",
    }
}
