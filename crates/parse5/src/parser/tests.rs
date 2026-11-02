fn child_at(parent: &NodeRef, idx: usize) -> NodeRef {
    parent.borrow().child_nodes().unwrap()[idx].clone()
}

fn child_count(parent: &NodeRef) -> usize {
    parent.borrow().child_nodes().unwrap().len()
}

fn text_value(node: &NodeRef) -> String {
    match &node.borrow().data {
        NodeData::Text { value } => value.clone(),
        _ => panic!("expected text node"),
    }
}

fn comment_data(node: &NodeRef) -> String {
    match &node.borrow().data {
        NodeData::Comment { data } => data.clone(),
        _ => panic!("expected comment node"),
    }
}

fn element_attr(node: &NodeRef, name: &str) -> Option<crate::common::token::Attribute> {
    match &node.borrow().data {
        NodeData::Element { attrs, .. } => attrs.iter().find(|attr| attr.name == name).cloned(),
        _ => panic!("expected element node"),
    }
}

fn element_name(node: &NodeRef) -> String {
    element_tag_name(node).expect("expected element node")
}

fn element_namespace(node: &NodeRef) -> String {
    namespace_uri(node).expect("expected element node")
}

fn is_parent_of(parent: &NodeRef, child: &NodeRef) -> bool {
    child
        .borrow()
        .parent()
        .as_ref()
        .is_some_and(|actual| std::rc::Rc::ptr_eq(actual, parent))
}

use super::{
    element_tag_name, namespace_uri, parse, parse_fragment, parse_fragment_with_context,
    parse_with_errors, InsertionMode, Parser, ParserOptions,
};
use crate::common::error_codes::ErrorCode;
use crate::common::html::{DocumentMode, NS_HTML, NS_MATHML, NS_SVG, NS_XLINK};
use crate::serializer::{serialize, SerializerOptions};
use crate::tree_adapters::default::{
    append_child, create_element, get_document_mode, get_template_content, NodeData, NodeRef,
};

#[test]
fn insertion_mode_discriminants_follow_upstream_order() {
    assert_eq!(InsertionMode::Initial as u8, 0);
    assert_eq!(InsertionMode::InBody as u8, 6);
    assert_eq!(InsertionMode::InTemplate as u8, 17);
    assert_eq!(InsertionMode::AfterAfterFrameset as u8, 22);
}

#[test]
fn fragment_preserves_text_location_for_initial_scaffold() {
    let fragment = parse_fragment(
        "hello",
        ParserOptions {
            source_code_location_info: true,
            ..ParserOptions::default()
        },
    );
    let text = child_at(&fragment, 0);
    let location = text.borrow().source_code_location().unwrap();

    assert_eq!(text_value(&text), "hello");
    assert_eq!(location.location.start_offset, 0);
    assert_eq!(location.location.end_offset, 5);
}

#[test]
fn script_handler_events_pause_and_resume_like_upstream() {
    let mut parser = Parser::new(ParserOptions::default(), None, None);
    parser.script_handler_enabled = true;

    parser.write_chunk(
        "<!doctype html><script>document.write('<p>a</p>')</script><p>after</p>",
        true,
    );

    assert!(parser.paused_on_script);
    assert_eq!(parser.script_events.len(), 1);
    assert_eq!(element_name(&parser.script_events[0].node), "script");
    assert_eq!(
        serialize(&parser.document, SerializerOptions::default()),
        "<!DOCTYPE html><html><head><script>document.write('<p>a</p>')</script></head></html>"
    );

    parser.insert_html_at_current_pos("<p>written</p>");
    assert!(parser.paused_on_script);

    parser.resume();
    assert!(!parser.paused_on_script);
    assert_eq!(
        serialize(&parser.document, SerializerOptions::default()),
        "<!DOCTYPE html><html><head><script>document.write('<p>a</p>')</script></head><body><p>written</p><p>after</p></body></html>"
    );
}

#[test]
fn plain_text_conversion_initializes_document_like_upstream() {
    let mut parser = Parser::new(ParserOptions::default(), None, None);
    parser.init_plain_text_conversion();
    parser.write_chunk("Hey\r\nyo\u{0}<html><head><body>", true);

    assert_eq!(
        serialize(&parser.document, SerializerOptions::default()),
        "<html><head></head><body><pre>\nHey\nyo\u{fffd}&lt;html&gt;&lt;head&gt;&lt;body&gt;</pre></body></html>"
    );
}

#[test]
fn fragment_builds_nested_elements_comments_and_text() {
    let fragment = parse_fragment(
        "<template><div id=\"x\">hi<!--c--></div></template>",
        ParserOptions {
            source_code_location_info: true,
            ..ParserOptions::default()
        },
    );

    let template = child_at(&fragment, 0);
    assert_eq!(element_name(&template), "template");
    assert_eq!(child_count(&fragment), 1);

    let content = get_template_content(&template).unwrap();
    let div = child_at(&content, 0);
    let text = child_at(&div, 0);
    let comment = child_at(&div, 1);

    assert_eq!(element_name(&div), "div");
    assert_eq!(element_attr(&div, "id").unwrap().value, "x");
    assert_eq!(text_value(&text), "hi");
    assert_eq!(comment_data(&comment), "c");
    assert!(is_parent_of(&content, &div));
    assert!(is_parent_of(&div, &text));
    assert!(is_parent_of(&div, &comment));
    assert_eq!(
        serialize(&fragment, SerializerOptions::default()),
        "<template><div id=\"x\">hi<!--c--></div></template>"
    );

    let template_location = template.borrow().source_code_location().unwrap();
    assert_eq!(template_location.location.start_offset, 0);
    assert_eq!(template_location.location.end_offset, 49);
    assert_eq!(template_location.start_tag.unwrap().start_offset, 0);
    assert_eq!(template_location.end_tag.unwrap().end_offset, 49);

    let div_location = div.borrow().source_code_location().unwrap();
    assert_eq!(div_location.attrs["id"].start_offset, 15);
    assert_eq!(div_location.end_tag.unwrap().end_offset, 38);
}

#[test]
fn fragment_keeps_void_elements_from_capturing_following_siblings() {
    let fragment = parse_fragment("<br><span>x</span>", ParserOptions::default());

    let br = child_at(&fragment, 0);
    let span = child_at(&fragment, 1);

    assert_eq!(element_name(&br), "br");
    assert_eq!(element_name(&span), "span");
    assert_eq!(child_count(&span), 1);
    assert_eq!(text_value(&child_at(&span, 0)), "x");
    assert_eq!(
        serialize(&fragment, SerializerOptions::default()),
        "<br><span>x</span>"
    );
}

#[test]
fn fragment_keeps_self_closing_non_void_html_elements_open_like_upstream() {
    let fragment = parse_fragment("<x/>after", ParserOptions::default());

    assert_eq!(
        serialize(&fragment, SerializerOptions::default()),
        "<x>after</x>"
    );
}

#[test]
fn fragment_appends_adjacent_character_tokens_into_one_text_node() {
    let fragment = parse_fragment(
        "a &amp; b",
        ParserOptions {
            source_code_location_info: true,
            ..ParserOptions::default()
        },
    );

    assert_eq!(child_count(&fragment), 1);
    let text = child_at(&fragment, 0);
    assert_eq!(text_value(&text), "a & b");

    let location = text.borrow().source_code_location().unwrap();
    assert_eq!(location.location.start_offset, 0);
    assert_eq!(location.location.end_offset, 9);
}

#[test]
fn fragment_ignores_body_doctypes_and_keeps_comments_like_upstream() {
    let fragment = parse_fragment(
        "<div><!doctype html><!--ok--></div>",
        ParserOptions::default(),
    );
    let div = child_at(&fragment, 0);

    assert_eq!(child_count(&div), 1);
    assert_eq!(comment_data(&child_at(&div, 0)), "ok");
}

#[test]
fn document_parse_builds_implicit_html_head_and_body() {
    let document = parse("hello", ParserOptions::default());
    let html = child_at(&document, 0);
    let head = child_at(&html, 0);
    let body = child_at(&html, 1);

    assert_eq!(element_name(&html), "html");
    assert_eq!(element_name(&head), "head");
    assert_eq!(element_name(&body), "body");
    assert_eq!(text_value(&child_at(&body, 0)), "hello");
    assert_eq!(get_document_mode(&document), DocumentMode::Quirks);
}

#[test]
fn document_parse_uses_doctype_and_adopts_html_body_attrs() {
    let document = parse(
        "<!doctype html><html lang=\"en\"><head><meta charset=\"utf-8\"></head><body class=\"x\"><p>hi</p></body></html>",
        ParserOptions {
            source_code_location_info: true,
            ..ParserOptions::default()
        },
    );
    let html = child_at(&document, 1);
    let head = child_at(&html, 0);
    let body = child_at(&html, 1);

    assert_eq!(get_document_mode(&document), DocumentMode::NoQuirks);
    assert_eq!(element_attr(&html, "lang").unwrap().value, "en");
    assert_eq!(element_attr(&body, "class").unwrap().value, "x");
    assert_eq!(element_name(&head), "head");
    assert_eq!(element_name(&child_at(&head, 0)), "meta");
    assert_eq!(element_name(&child_at(&body, 0)), "p");

    let body_location = body.borrow().source_code_location().unwrap();
    assert!(body_location.start_tag.is_some());
    assert!(body_location.end_tag.is_some());
}

#[test]
fn document_reports_parser_errors_like_upstream() {
    for (html, expected) in [
        ("hello", vec![ErrorCode::MissingDoctype]),
        (
            "<!doctype foo><html></html>",
            vec![ErrorCode::NonConformingDoctype],
        ),
        (
            "<html><head><!doctype html></head></html>",
            vec![ErrorCode::MissingDoctype, ErrorCode::MisplacedDoctype],
        ),
        (
            "<html><head><head></head></html>",
            vec![
                ErrorCode::MissingDoctype,
                ErrorCode::MisplacedStartTagForHeadElement,
            ],
        ),
        (
            "<html><head></div></head></html>",
            vec![
                ErrorCode::MissingDoctype,
                ErrorCode::EndTagWithoutMatchingOpenElement,
            ],
        ),
        (
            "<html><head><template><div></template></head></html>",
            vec![
                ErrorCode::MissingDoctype,
                ErrorCode::ClosingOfElementWithOpenChildElements,
            ],
        ),
        (
            "<html><head></head><meta><body></body></html>",
            vec![
                ErrorCode::MissingDoctype,
                ErrorCode::AbandonedHeadElementChild,
            ],
        ),
        (
            "<title>x",
            vec![
                ErrorCode::MissingDoctype,
                ErrorCode::EofInElementThatCanContainOnlyText,
            ],
        ),
        (
            "<x/>",
            vec![
                ErrorCode::MissingDoctype,
                ErrorCode::NonVoidHtmlElementStartTagWithTrailingSolidus,
            ],
        ),
    ] {
        let output = parse_with_errors(html, ParserOptions::default());
        let actual = output
            .errors
            .iter()
            .map(|error| error.code)
            .collect::<Vec<_>>();

        assert_eq!(actual, expected, "{html}");
    }
}

#[test]
fn document_reports_in_head_noscript_errors_like_upstream() {
    for (html, expected) in [
        (
            "<html><head><noscript><noscript></noscript></head></html>",
            vec![ErrorCode::MissingDoctype, ErrorCode::NestedNoscriptInHead],
        ),
        (
            "<html><head><noscript>x</head></html>",
            vec![
                ErrorCode::MissingDoctype,
                ErrorCode::DisallowedContentInNoscriptInHead,
            ],
        ),
    ] {
        let output = parse_with_errors(
            html,
            ParserOptions {
                scripting_enabled: false,
                ..ParserOptions::default()
            },
        );
        let actual = output
            .errors
            .iter()
            .map(|error| error.code)
            .collect::<Vec<_>>();

        assert_eq!(actual, expected, "{html}");
    }
}

#[test]
fn document_builds_frameset_modes_like_upstream() {
    for (html, expected) in [
        (
            "<!doctype html><html><head></head><frameset><frame src=x><frameset><frame></frameset></frameset></html><!--tail-->",
            "<!DOCTYPE html><html><head></head><frameset><frame src=\"x\"><frameset><frame></frameset></frameset></html><!--tail-->",
        ),
        (
            "<html><head></head><body>text<frameset><frame></frameset></body></html>",
            "<html><head></head><body>text</body></html>",
        ),
        (
            "<html><head></head><frameset><frame></frameset><noframes>x</noframes></html>after",
            "<html><head></head><frameset><frame></frameset><noframes>x</noframes></html>",
        ),
        (
            "<html><body>x</body></html><p>y",
            "<html><head></head><body>x<p>y</p></body></html>",
        ),
        (
            "<html><head></head><frameset><frame></frameset></html><p>x",
            "<html><head></head><frameset><frame></frameset></html>",
        ),
    ] {
        let document = parse(html, ParserOptions::default());
        assert_eq!(serialize(&document, SerializerOptions::default()), expected);
    }
}

#[test]
fn document_builds_in_head_noscript_mode_like_upstream() {
    let document = parse(
        "<html><head><noscript><link rel=x><p>body</noscript></head><body>ok</body></html>",
        ParserOptions {
            scripting_enabled: false,
            ..ParserOptions::default()
        },
    );

    assert_eq!(
        serialize(&document, SerializerOptions::default()),
        "<html><head><noscript><link rel=\"x\"></noscript></head><body><p>bodyok</p></body></html>"
    );
}

#[test]
fn fragment_uses_upstream_svg_and_xml_attr_adjustments() {
    let fragment = parse_fragment(
        "<svg viewbox=\"0 0 1 1\"><lineargradient><use xlink:href=\"#x\"></use></lineargradient></svg>",
        ParserOptions {
            source_code_location_info: true,
            ..ParserOptions::default()
        },
    );

    let svg = child_at(&fragment, 0);
    let linear_gradient = child_at(&svg, 0);
    let use_element = child_at(&linear_gradient, 0);

    assert_eq!(element_namespace(&svg), NS_SVG);
    assert_eq!(element_attr(&svg, "viewBox").unwrap().value, "0 0 1 1");
    assert_eq!(element_name(&linear_gradient), "linearGradient");
    assert_eq!(element_namespace(&linear_gradient), NS_SVG);

    let href = element_attr(&use_element, "href").unwrap();
    assert_eq!(href.prefix.as_deref(), Some("xlink"));
    assert_eq!(href.namespace.as_deref(), Some(NS_XLINK));
    assert_eq!(href.qualified_name(), "xlink:href");

    let svg_location = svg.borrow().source_code_location().unwrap();
    assert!(svg_location.attrs.contains_key("viewbox"));
}

#[test]
fn fragment_switches_from_svg_foreign_object_to_html_integration_point() {
    let fragment = parse_fragment(
        "<svg><foreignobject><div>html</div></foreignobject></svg>",
        ParserOptions::default(),
    );

    let svg = child_at(&fragment, 0);
    let foreign_object = child_at(&svg, 0);
    let div = child_at(&foreign_object, 0);

    assert_eq!(element_namespace(&svg), NS_SVG);
    assert_eq!(element_name(&foreign_object), "foreignObject");
    assert_eq!(element_namespace(&foreign_object), NS_SVG);
    assert_eq!(element_name(&div), "div");
    assert_eq!(element_namespace(&div), NS_HTML);
    assert_eq!(text_value(&child_at(&div, 0)), "html");
}

#[test]
fn fragment_uses_upstream_mathml_attr_adjustments() {
    let fragment = parse_fragment(
        "<math><annotation-xml definitionurl=\"x\" encoding=\"text/html\"><svg></svg></annotation-xml></math>",
        ParserOptions::default(),
    );

    let math = child_at(&fragment, 0);
    let annotation = child_at(&math, 0);
    let svg = child_at(&annotation, 0);

    assert_eq!(element_namespace(&math), NS_MATHML);
    assert_eq!(element_namespace(&annotation), NS_MATHML);
    assert_eq!(
        element_attr(&annotation, "definitionURL").unwrap().value,
        "x"
    );
    assert_eq!(element_namespace(&svg), NS_SVG);
}

#[test]
fn fragment_uses_parser_driven_text_parsing_modes() {
    for tag_name in [
        "iframe", "noembed", "noframes", "noscript", "script", "style", "textarea", "title", "xmp",
    ] {
        let html = format!("<r><{tag_name}><math id=\"</{tag_name}><b>should be outside</b>\">");
        let fragment = parse_fragment(&html, ParserOptions::default());

        let r = child_at(&fragment, 0);
        assert_eq!(element_name(&r), "r", "{tag_name}");
        assert_eq!(child_count(&r), 3, "{tag_name}");

        let target = child_at(&r, 0);
        assert_eq!(element_name(&target), tag_name);
        assert_eq!(child_count(&target), 1, "{tag_name}");
        assert_eq!(text_value(&child_at(&target, 0)), "<math id=\"");

        let b = child_at(&r, 1);
        assert_eq!(element_name(&b), "b", "{tag_name}");
        assert_eq!(text_value(&child_at(&b, 0)), "should be outside");
        assert_eq!(text_value(&child_at(&r, 2)), "\">");
    }
}

#[test]
fn fragment_context_controls_modes_foreign_content_and_form_ancestry_like_upstream() {
    let table_context = create_element("table", NS_HTML, Vec::new());
    let table_fragment =
        parse_fragment_with_context(Some(table_context), "<tr><td>x", ParserOptions::default());
    assert_eq!(
        serialize(&table_fragment, SerializerOptions::default()),
        "<tbody><tr><td>x</td></tr></tbody>"
    );

    let textarea_context = create_element("textarea", NS_HTML, Vec::new());
    let textarea_fragment =
        parse_fragment_with_context(Some(textarea_context), "<b>x</b>", ParserOptions::default());
    assert_eq!(
        serialize(&textarea_fragment, SerializerOptions::default()),
        "&lt;b&gt;x&lt;/b&gt;"
    );

    let svg_context = create_element("svg", NS_SVG, Vec::new());
    let svg_fragment = parse_fragment_with_context(
        Some(svg_context),
        "<g><foreignObject><div>x</div></foreignObject></g>",
        ParserOptions::default(),
    );
    let g = child_at(&svg_fragment, 0);
    let foreign_object = child_at(&g, 0);
    let div = child_at(&foreign_object, 0);
    assert_eq!(element_namespace(&g), NS_SVG);
    assert_eq!(element_namespace(&foreign_object), NS_SVG);
    assert_eq!(element_namespace(&div), NS_HTML);
    assert_eq!(
        serialize(&svg_fragment, SerializerOptions::default()),
        "<g><foreignObject><div>x</div></foreignObject></g>"
    );

    let form_context = create_element("form", NS_HTML, Vec::new());
    let div_context = create_element("div", NS_HTML, Vec::new());
    append_child(&form_context, div_context.clone());
    let form_fragment = parse_fragment_with_context(
        Some(div_context),
        "<form id=x><input>",
        ParserOptions::default(),
    );
    assert_eq!(
        serialize(&form_fragment, SerializerOptions::default()),
        "<input>"
    );
}

#[test]
fn fragment_reconstructs_active_formatting_elements_like_upstream() {
    let fragment = parse_fragment("<p><b class=\"x\">one</p>two", ParserOptions::default());

    assert_eq!(
        serialize(&fragment, SerializerOptions::default()),
        "<p><b class=\"x\">one</b></p><b class=\"x\">two</b>"
    );
}

#[test]
fn fragment_runs_adoption_agency_for_misnested_formatting_end_tags() {
    let fragment = parse_fragment("<b>1<i>2<p>3</b>4", ParserOptions::default());

    assert_eq!(
        serialize(&fragment, SerializerOptions::default()),
        "<b>1<i>2</i></b><i><p><b>3</b>4</p></i>"
    );
}

#[test]
fn fragment_closes_prior_anchor_before_nested_anchor_like_upstream() {
    let fragment = parse_fragment("<a X>0<b>1<a Y>2", ParserOptions::default());

    assert_eq!(
        serialize(&fragment, SerializerOptions::default()),
        "<a x=\"\">0<b>1</b></a><b><a y=\"\">2</a></b>"
    );
}

#[test]
fn fragment_handles_in_body_list_pre_and_form_cases_like_upstream() {
    for (html, expected) in [
        ("<p>One<p>Two", "<p>One</p><p>Two</p>"),
        (
            "<ul><li>one<li>two</ul>",
            "<ul><li>one</li><li>two</li></ul>",
        ),
        (
            "<dl><dt>a<dd>b<dt>c</dl>",
            "<dl><dt>a</dt><dd>b</dd><dt>c</dt></dl>",
        ),
        ("<pre>\nfoo</pre>", "<pre>foo</pre>"),
        (
            "<form id=a><form id=b><input></form>",
            "<form id=\"a\"><input></form>",
        ),
    ] {
        let fragment = parse_fragment(html, ParserOptions::default());
        assert_eq!(serialize(&fragment, SerializerOptions::default()), expected);
    }
}

#[test]
fn fragment_handles_in_body_void_button_and_marker_cases_like_upstream() {
    for (html, expected) in [
        ("<p><hr></p>", "<p></p><hr><p></p>"),
        ("<image>", "<img>"),
        ("<button><button>x", "<button></button><button>x</button>"),
        ("<applet><b>x</applet>y", "<applet><b>x</b></applet>y"),
    ] {
        let fragment = parse_fragment(html, ParserOptions::default());
        assert_eq!(serialize(&fragment, SerializerOptions::default()), expected);
    }
}

#[test]
fn fragment_builds_core_table_modes_like_upstream() {
    for (html, expected) in [
        (
            "<table><tr><td>x</td></tr></table>",
            "<table><tbody><tr><td>x</td></tr></tbody></table>",
        ),
        (
            "<table><td>x",
            "<table><tbody><tr><td>x</td></tr></tbody></table>",
        ),
        (
            "<table><tbody><tr><td>a<td>b</tr><tr><td>c</table>",
            "<table><tbody><tr><td>a</td><td>b</td></tr><tr><td>c</td></tr></tbody></table>",
        ),
    ] {
        let fragment = parse_fragment(html, ParserOptions::default());
        assert_eq!(serialize(&fragment, SerializerOptions::default()), expected);
    }
}

#[test]
fn fragment_uses_table_foster_parenting_and_caption_colgroup_modes_like_upstream() {
    for (html, expected) in [
        (
            "<table>before<tr><td>cell</td></tr>after</table>",
            "beforeafter<table><tbody><tr><td>cell</td></tr></tbody></table>",
        ),
        (
            "<table><tr>loose<td>x</td></tr></table>",
            "loose<table><tbody><tr><td>x</td></tr></tbody></table>",
        ),
        (
            "<table><caption>cap<tr><td>x</td></tr></table>",
            "<table><caption>cap</caption><tbody><tr><td>x</td></tr></tbody></table>",
        ),
        (
            "<table><col><colgroup><col></table>",
            "<table><colgroup><col></colgroup><colgroup><col></colgroup></table>",
        ),
    ] {
        let fragment = parse_fragment(html, ParserOptions::default());
        assert_eq!(serialize(&fragment, SerializerOptions::default()), expected);
    }
}

#[test]
fn fragment_builds_select_modes_like_upstream() {
    for (html, expected) in [
        (
            "<select><option>a<option>b</select>x",
            "<select><option>a</option><option>b</option></select>x",
        ),
        (
            "<select><optgroup label=g><option>a<optgroup><option>b</select>",
            "<select><optgroup label=\"g\"><option>a</option></optgroup><optgroup><option>b</option></optgroup></select>",
        ),
        (
            "<select><option>a<hr><option>b</select>",
            "<select><option>a</option><hr><option>b</option></select>",
        ),
        (
            "<select><input><textarea>x</textarea>y",
            "<select></select><input><textarea>x</textarea>y",
        ),
    ] {
        let fragment = parse_fragment(html, ParserOptions::default());
        assert_eq!(serialize(&fragment, SerializerOptions::default()), expected);
    }
}

#[test]
fn fragment_builds_select_in_table_mode_like_upstream() {
    for (html, expected) in [
        (
            "<table><tr><td><select><option>a</td><td>b</table>",
            "<table><tbody><tr><td><select><option>a</option></select></td><td>b</td></tr></tbody></table>",
        ),
        (
            "<table><select><option>a<tr><td>b</table>",
            "<select><option>a</option></select><table><tbody><tr><td>b</td></tr></tbody></table>",
        ),
    ] {
        let fragment = parse_fragment(html, ParserOptions::default());
        assert_eq!(serialize(&fragment, SerializerOptions::default()), expected);
    }
}

#[test]
fn fragment_builds_template_modes_like_upstream() {
    for (html, expected) in [
        (
            "<template><tr><td>x</td></tr></template>",
            "<template><tr><td>x</td></tr></template>",
        ),
        (
            "<template><col><tr><td>x</td></tr></template>",
            "<template><col></template>",
        ),
        (
            "<template><caption>x</caption></template>",
            "<template><caption>x</caption></template>",
        ),
        (
            "<template><p><template><b>x</template>y</template>",
            "<template><p><template><b>x</b></template>y</p></template>",
        ),
        (
            "<template><p><b>x</template>y",
            "<template><p><b>x</b></p></template>y",
        ),
    ] {
        let fragment = parse_fragment(html, ParserOptions::default());
        assert_eq!(serialize(&fragment, SerializerOptions::default()), expected);
    }
}

#[test]
fn plaintext_start_tag_switches_tokenizer_without_text_insertion_mode() {
    let fragment = parse_fragment("<plaintext><b>x</b>", ParserOptions::default());
    let plaintext = child_at(&fragment, 0);

    assert_eq!(element_name(&plaintext), "plaintext");
    assert_eq!(child_count(&plaintext), 1);
    assert_eq!(text_value(&child_at(&plaintext, 0)), "<b>x</b>");
}
