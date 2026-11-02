//! Offline parity between the Rust tables in `crates/parse5/src/common` and the
//! hand-written copies in `packages/parse5-wasm/src/index.js`.
//!
//! This test reads those two trees only. Deleting or renaming an entry on
//! either side fails it.

use parse5::common::error_codes::ErrorCode;
use parse5::common::foreign_content::{
    exits_foreign_content, SVG_ATTR_ADJUSTMENTS, SVG_TAG_NAME_ADJUSTMENTS, XML_ATTR_ADJUSTMENTS,
};
use parse5::common::html::{
    get_tag_id, has_unescaped_text, is_numbered_header, is_special_element, is_void_element, TagId,
    ATTR_ACTION, ATTR_COLOR, ATTR_ENCODING, ATTR_FACE, ATTR_NAME, ATTR_PROMPT, ATTR_SIZE,
    ATTR_TYPE, NS_HTML, NS_MATHML, NS_SVG, NS_XLINK, NS_XML, NS_XMLNS, TAG_NAMES,
};
use std::collections::{BTreeMap, BTreeSet};

fn js_source() -> &'static str {
    include_str!("../../../packages/parse5-wasm/src/index.js")
}

fn between<'a>(source: &'a str, start: &str, end: &str) -> &'a str {
    let at = source
        .find(start)
        .unwrap_or_else(|| panic!("missing start marker {start}"));
    let rest = &source[at + start.len()..];
    let stop = rest
        .find(end)
        .unwrap_or_else(|| panic!("missing end marker after {start}"));
    &rest[..stop]
}

fn parse_quoted_pairs(block: &str) -> BTreeMap<String, String> {
    let mut pairs = BTreeMap::new();
    let bytes = block.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        while i < bytes.len() && !bytes[i].is_ascii_alphanumeric() && bytes[i] != b'_' {
            i += 1;
        }
        if i >= bytes.len() {
            break;
        }
        let key_start = i;
        while i < bytes.len() && (bytes[i].is_ascii_alphanumeric() || bytes[i] == b'_') {
            i += 1;
        }
        let key = block[key_start..i].to_string();
        while i < bytes.len() && bytes[i].is_ascii_whitespace() {
            i += 1;
        }
        if i >= bytes.len() || bytes[i] != b':' {
            continue;
        }
        i += 1;
        while i < bytes.len() && bytes[i].is_ascii_whitespace() {
            i += 1;
        }
        if i >= bytes.len() || (bytes[i] != b'\'' && bytes[i] != b'"') {
            continue;
        }
        let quote = bytes[i];
        i += 1;
        let value_start = i;
        while i < bytes.len() && bytes[i] != quote {
            i += 1;
        }
        let value = block[value_start..i].to_string();
        if i < bytes.len() {
            i += 1;
        }
        assert!(
            pairs.insert(key.clone(), value).is_none(),
            "duplicate JS key {key}"
        );
    }
    pairs
}

fn parse_enum_entries(block: &str) -> Vec<(String, u16)> {
    let mut entries = Vec::new();
    for line in block.lines() {
        let line = line.trim();
        if !line.starts_with("['") && !line.starts_with("[\"") {
            continue;
        }
        let quote = if line.starts_with("['") { '\'' } else { '"' };
        let name_start = 2;
        let name_end = line[name_start..]
            .find(quote)
            .map(|offset| name_start + offset)
            .expect("tag id name");
        let name = &line[name_start..name_end];
        let comma = line.find(',').expect("tag id comma");
        let number: u16 = line[comma + 1..]
            .trim()
            .trim_end_matches("],")
            .trim_end_matches(']')
            .trim()
            .parse()
            .expect("tag id number");
        entries.push((name.to_string(), number));
    }
    entries
}

fn parse_dollar_names(block: &str) -> BTreeSet<String> {
    let mut names = BTreeSet::new();
    let marker = "$.";
    let mut rest = block;
    while let Some(at) = rest.find(marker) {
        rest = &rest[at + marker.len()..];
        let end = rest
            .find(|ch: char| !ch.is_ascii_alphanumeric() && ch != '_')
            .unwrap_or(rest.len());
        names.insert(rest[..end].to_string());
        rest = &rest[end..];
    }
    names
}

fn parse_single_quoted_list(block: &str) -> BTreeSet<String> {
    let mut values = BTreeSet::new();
    let mut rest = block;
    while let Some(at) = rest.find('\'') {
        rest = &rest[at + 1..];
        let end = rest.find('\'').expect("closing quote");
        values.insert(rest[..end].to_string());
        rest = &rest[end + 1..];
    }
    values
}

fn kebab_to_camel(value: &str) -> String {
    let mut out = String::new();
    let mut upper = false;
    for ch in value.chars() {
        if ch == '-' {
            upper = true;
            continue;
        }
        if upper {
            out.push(ch.to_ascii_uppercase());
            upper = false;
        } else {
            out.push(ch);
        }
    }
    out
}

fn tag_ids() -> Vec<TagId> {
    let mut ids = vec![TagId::Unknown];
    ids.extend(TAG_NAMES.iter().copied().map(get_tag_id));
    ids
}

fn id_by_enum_name(entries: &[(String, u16)]) -> BTreeMap<String, u16> {
    entries.iter().cloned().collect()
}

#[test]
fn namespaces_attributes_and_document_mode_match() {
    let js = js_source();
    let ns = parse_quoted_pairs(between(js, "export const NS = {", "\n};"));
    assert_eq!(ns["HTML"], NS_HTML);
    assert_eq!(ns["MATHML"], NS_MATHML);
    assert_eq!(ns["SVG"], NS_SVG);
    assert_eq!(ns["XLINK"], NS_XLINK);
    assert_eq!(ns["XML"], NS_XML);
    assert_eq!(ns["XMLNS"], NS_XMLNS);
    assert_eq!(ns.len(), 6);

    let attrs = parse_quoted_pairs(between(js, "export const ATTRS = {", "\n};"));
    assert_eq!(attrs["TYPE"], ATTR_TYPE);
    assert_eq!(attrs["ACTION"], ATTR_ACTION);
    assert_eq!(attrs["ENCODING"], ATTR_ENCODING);
    assert_eq!(attrs["PROMPT"], ATTR_PROMPT);
    assert_eq!(attrs["NAME"], ATTR_NAME);
    assert_eq!(attrs["COLOR"], ATTR_COLOR);
    assert_eq!(attrs["FACE"], ATTR_FACE);
    assert_eq!(attrs["SIZE"], ATTR_SIZE);
    assert_eq!(attrs.len(), 8);

    let modes = parse_quoted_pairs(between(js, "export const DOCUMENT_MODE = {", "\n};"));
    assert_eq!(modes["NO_QUIRKS"], "no-quirks");
    assert_eq!(modes["QUIRKS"], "quirks");
    assert_eq!(modes["LIMITED_QUIRKS"], "limited-quirks");
    assert_eq!(modes.len(), 3);
}

#[test]
fn tag_names_and_ids_match() {
    let js = js_source();
    let names = parse_quoted_pairs(between(js, "export const TAG_NAMES = {", "\n};"));
    let entries = parse_enum_entries(between(js, "export const TAG_ID = numericEnum([", "]);"));

    assert_eq!(
        entries.first().map(|(name, id)| (name.as_str(), *id)),
        Some(("UNKNOWN", 0))
    );
    assert_eq!(names.len(), TAG_NAMES.len());
    assert_eq!(entries.len(), TAG_NAMES.len() + 1);

    for (index, rust_name) in TAG_NAMES.iter().copied().enumerate() {
        let id = (index + 1) as u16;
        let (js_enum, js_id) = &entries[index + 1];
        let js_name = names
            .get(js_enum)
            .unwrap_or_else(|| panic!("TAG_NAMES missing {js_enum}"));
        assert_eq!(js_name, rust_name, "tag order diverged at {index}");
        assert_eq!(*js_id, id);
        assert_eq!(get_tag_id(rust_name) as u16, id);
    }
}

#[test]
fn error_codes_match() {
    let js = js_source();
    let codes = parse_quoted_pairs(between(js, "export const ErrorCodes = {", "\n};"));
    assert_eq!(codes.len(), ErrorCode::ALL.len());

    for code in ErrorCode::ALL {
        let value = code.as_str();
        let key = kebab_to_camel(value);
        assert_eq!(
            codes.get(&key).map(String::as_str),
            Some(value),
            "ErrorCodes.{key} should be {value}"
        );
    }
}

#[test]
fn foreign_content_adjustment_maps_match() {
    let js = js_source();
    let svg_tags = parse_single_quoted_list(between(
        js,
        "const SVG_TAG_NAMES_ADJUSTMENT_MAP = new Map(",
        "].map(",
    ));
    let svg_attrs = parse_single_quoted_list(between(
        js,
        "const SVG_ATTRS_ADJUSTMENT_MAP = new Map(",
        "].map(",
    ));
    let xml_block = between(js, "const XML_ATTRS_ADJUSTMENT_MAP = new Map([", "]);");

    let rust_tags: BTreeSet<_> = SVG_TAG_NAME_ADJUSTMENTS
        .iter()
        .map(|(_, adjusted)| (*adjusted).to_string())
        .collect();
    assert_eq!(svg_tags, rust_tags);

    let rust_attrs: BTreeSet<_> = SVG_ATTR_ADJUSTMENTS
        .iter()
        .map(|(_, adjusted)| (*adjusted).to_string())
        .collect();
    assert_eq!(svg_attrs, rust_attrs);

    let mut js_xml = BTreeSet::new();
    for line in xml_block.lines() {
        let line = line.trim();
        if !line.starts_with('[') {
            continue;
        }
        let mut quotes = Vec::new();
        let mut rest = line;
        while let Some(at) = rest.find('\'') {
            rest = &rest[at + 1..];
            let end = rest.find('\'').expect("xml attr quote");
            quotes.push(rest[..end].to_string());
            rest = &rest[end + 1..];
        }
        assert!(quotes.len() >= 3, "xml attr row: {line}");
        let namespace = if line.contains("NS.XLINK") {
            NS_XLINK
        } else if line.contains("NS.XMLNS") {
            NS_XMLNS
        } else if line.contains("NS.XML") {
            NS_XML
        } else {
            panic!("xml attr row missing namespace: {line}");
        };
        js_xml.insert((
            quotes[0].clone(),
            quotes[1].clone(),
            quotes[2].clone(),
            namespace.to_string(),
        ));
    }
    let rust_xml: BTreeSet<_> = XML_ATTR_ADJUSTMENTS
        .iter()
        .map(|(key, prefix, name, namespace)| {
            (
                (*key).to_string(),
                (*prefix).to_string(),
                (*name).to_string(),
                (*namespace).to_string(),
            )
        })
        .collect();
    assert_eq!(js_xml, rust_xml);
}

#[test]
fn special_void_header_and_exit_sets_match() {
    let js = js_source();
    let entries = parse_enum_entries(between(js, "export const TAG_ID = numericEnum([", "]);"));
    let ids = id_by_enum_name(&entries);
    let special = between(js, "export const SPECIAL_ELEMENTS = {", "\n};");
    let mut special_by_ns = BTreeMap::<&str, BTreeSet<u16>>::new();
    for (label, namespace) in [
        ("[NS.HTML]", NS_HTML),
        ("[NS.MATHML]", NS_MATHML),
        ("[NS.SVG]", NS_SVG),
        ("[NS.XLINK]", NS_XLINK),
        ("[NS.XML]", NS_XML),
        ("[NS.XMLNS]", NS_XMLNS),
    ] {
        let marker = format!("{label}: new Set(");
        let at = special
            .find(&marker)
            .unwrap_or_else(|| panic!("missing {marker}"));
        let rest = &special[at + marker.len()..];
        let set = if rest.starts_with('[') {
            let block = between(rest, "[", "]");
            parse_dollar_names(block)
                .into_iter()
                .map(|name| ids[&name])
                .collect()
        } else {
            BTreeSet::new()
        };
        special_by_ns.insert(namespace, set);
    }

    for namespace in [NS_HTML, NS_MATHML, NS_SVG, NS_XLINK, NS_XML, NS_XMLNS] {
        let js_set = &special_by_ns[namespace];
        for tag_id in tag_ids() {
            let present = js_set.contains(&(tag_id as u16));
            assert_eq!(
                is_special_element(namespace, tag_id),
                present,
                "{namespace} special set disagrees on {tag_id:?}"
            );
        }
    }

    let voids = parse_single_quoted_list(between(js, "const VOID_ELEMENTS = new Set([", "]);"));
    for name in TAG_NAMES {
        assert_eq!(
            is_void_element(name),
            voids.contains(*name),
            "void element set disagrees on {name}"
        );
    }

    let headers = parse_dollar_names(between(
        js,
        "export const NUMBERED_HEADERS = new Set([",
        "]);",
    ));
    let header_ids: BTreeSet<u16> = headers.into_iter().map(|name| ids[&name]).collect();
    for tag_id in tag_ids() {
        assert_eq!(
            is_numbered_header(tag_id),
            header_ids.contains(&(tag_id as u16)),
            "numbered headers disagree on {tag_id:?}"
        );
    }

    let exits = parse_dollar_names(between(
        js,
        "const EXITS_FOREIGN_CONTENT = new Set([",
        "]);",
    ));
    let exit_ids: BTreeSet<u16> = exits.into_iter().map(|name| ids[&name]).collect();
    for tag_id in tag_ids() {
        assert_eq!(
            exits_foreign_content(tag_id),
            exit_ids.contains(&(tag_id as u16)),
            "foreign-content exit set disagrees on {tag_id:?}"
        );
    }

    let unescaped = parse_quoted_pairs(between(js, "export const TAG_NAMES = {", "\n};"));
    let unescaped_block = between(js, "const UNESCAPED_TEXT = new Set([", "]);");
    let unescaped_names: BTreeSet<String> = unescaped_block
        .split("TAG_NAMES.")
        .skip(1)
        .map(|part| {
            let end = part
                .find(|ch: char| !ch.is_ascii_alphanumeric() && ch != '_')
                .unwrap_or(part.len());
            unescaped[&part[..end]].clone()
        })
        .collect();
    for name in TAG_NAMES {
        assert_eq!(
            has_unescaped_text(name, false),
            unescaped_names.contains(*name),
            "unescaped text set disagrees on {name}"
        );
    }
    assert!(!has_unescaped_text("noscript", false));
    assert!(has_unescaped_text("noscript", true));
    assert!(
        js.contains("scriptingEnabled && tagName === TAG_NAMES.NOSCRIPT"),
        "JS hasUnescapedText should still special-case noscript when scripting is enabled"
    );
}
