use parse5::common::token::{
    Attribute, CharacterToken, CommentToken, DoctypeToken, EofToken, Location,
    LocationWithAttributes, TagToken, Token, TokenType,
};
use parse5::parser::{Parser as RustParser, ScriptEvent, StackEvent, StackEventKind};
use parse5::tokenizer::{State, Tokenizer, TokenizerOptions};
use parse5::tree_adapters::default::{append_child, create_element, NodeData, NodeRef};
use parse5::{
    parse, parse_fragment, parse_fragment_with_context, parse_fragment_with_context_and_errors,
    parse_fragment_with_errors, parse_with_errors, ParserOptions, SerializerOptions,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Map, Value};
use std::rc::Rc;
use wasm_bindgen::prelude::*;

mod node_json;

use node_json::NodeJson;

#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
struct WasmParserOptions {
    scripting_enabled: Option<bool>,
    source_code_location_info: Option<bool>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
struct WasmTokenizerOptions {
    source_code_location_info: Option<bool>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct FragmentContextDescriptor {
    tag_name: String,
    #[serde(default = "default_html_namespace", rename = "namespaceURI")]
    namespace_uri: String,
    #[serde(default)]
    attrs: Vec<Attribute>,
    #[serde(default, rename = "parentNode")]
    parent_node: Option<Box<FragmentContextDescriptor>>,
}

fn default_html_namespace() -> String {
    parse5::common::html::NS_HTML.to_string()
}

fn parser_options(options: JsValue) -> Result<ParserOptions, JsValue> {
    if options.is_null() || options.is_undefined() {
        return Ok(ParserOptions::default());
    }

    let options: WasmParserOptions = serde_wasm_bindgen::from_value(options)
        .map_err(|err| JsValue::from_str(&format!("invalid parser options: {err}")))?;

    Ok(ParserOptions {
        scripting_enabled: options.scripting_enabled.unwrap_or(true),
        source_code_location_info: options.source_code_location_info.unwrap_or(false),
    })
}

fn tokenizer_options(options: JsValue) -> Result<TokenizerOptions, JsValue> {
    if options.is_null() || options.is_undefined() {
        return Ok(TokenizerOptions::default());
    }

    let options: WasmTokenizerOptions = serde_wasm_bindgen::from_value(options)
        .map_err(|err| JsValue::from_str(&format!("invalid tokenizer options: {err}")))?;

    Ok(TokenizerOptions {
        source_code_location_info: options.source_code_location_info.unwrap_or(false),
    })
}

fn fragment_context_from_js(value: JsValue) -> Result<Option<NodeRef>, JsValue> {
    if value.is_null() || value.is_undefined() {
        return Ok(None);
    }

    let descriptor: FragmentContextDescriptor = serde_wasm_bindgen::from_value(value)
        .map_err(|err| JsValue::from_str(&format!("invalid fragment context: {err}")))?;
    Ok(Some(fragment_context_node(&descriptor)))
}

fn fragment_context_node(descriptor: &FragmentContextDescriptor) -> NodeRef {
    let element = create_element(
        descriptor.tag_name.clone(),
        descriptor.namespace_uri.clone(),
        descriptor.attrs.clone(),
    );

    if let Some(parent) = descriptor.parent_node.as_deref() {
        let parent = fragment_context_node(parent);
        append_child(&parent, element.clone());
    }

    element
}

fn location_value(location: Location) -> Value {
    json!({
        "startLine": location.start_line,
        "startCol": location.start_col,
        "startOffset": location.start_offset,
        "endLine": location.end_line,
        "endCol": location.end_col,
        "endOffset": location.end_offset,
    })
}

fn maybe_location_value(location: Option<Location>) -> Value {
    location.map(location_value).unwrap_or(Value::Null)
}

fn location_with_attrs_value(location: LocationWithAttributes) -> Value {
    let mut object = match location_value(location.location) {
        Value::Object(object) => object,
        _ => Map::new(),
    };

    if !location.attrs.is_empty() {
        let attrs = location
            .attrs
            .iter()
            .map(|(name, location)| (name.clone(), location_value(*location)))
            .collect();
        object.insert("attrs".to_string(), Value::Object(attrs));
    }

    Value::Object(object)
}

fn maybe_location_with_attrs_value(location: Option<LocationWithAttributes>) -> Value {
    location
        .map(location_with_attrs_value)
        .unwrap_or(Value::Null)
}

fn parser_error_value(error: parse5::ParserError) -> Value {
    let mut object = match location_value(error.location) {
        Value::Object(object) => object,
        _ => Map::new(),
    };
    object.insert("code".to_string(), json!(error.code.as_str()));
    Value::Object(object)
}

fn doctype_token_value(token: DoctypeToken) -> Value {
    json!({
        "type": TokenType::Doctype as u8,
        "name": token.name,
        "forceQuirks": token.force_quirks,
        "publicId": token.public_id,
        "systemId": token.system_id,
        "location": maybe_location_value(token.location),
    })
}

fn tag_token_value(token: TagToken) -> Value {
    json!({
        "type": token.token_type as u8,
        "tagName": token.tag_name,
        "tagID": token.tag_id as u8,
        "selfClosing": token.self_closing,
        "ackSelfClosing": token.ack_self_closing,
        "attrs": token.attrs,
        "location": maybe_location_with_attrs_value(token.location),
    })
}

fn comment_token_value(token: CommentToken) -> Value {
    json!({
        "type": TokenType::Comment as u8,
        "data": token.data,
        "location": maybe_location_value(token.location),
    })
}

fn eof_token_value(token: EofToken) -> Value {
    json!({
        "type": TokenType::Eof as u8,
        "location": maybe_location_value(token.location),
    })
}

fn character_token_value(token: CharacterToken) -> Value {
    json!({
        "type": token.token_type as u8,
        "chars": token.chars,
        "location": maybe_location_value(token.location),
    })
}

fn token_value(token: Token) -> Value {
    match token {
        Token::Doctype(token) => doctype_token_value(token),
        Token::Tag(token) => tag_token_value(token),
        Token::Comment(token) => comment_token_value(token),
        Token::Eof(token) => eof_token_value(token),
        Token::Character(token) => character_token_value(token),
    }
}

fn tokenizer_result_value(tokens: Vec<Token>, errors: Vec<parse5::ParserError>) -> Value {
    json!({
        "tokens": tokens.into_iter().map(token_value).collect::<Vec<_>>(),
        "errors": errors.into_iter().map(parser_error_value).collect::<Vec<_>>(),
    })
}

fn tokenizer_state_from_u8(value: u8) -> Option<State> {
    Some(match value {
        0 => State::Data,
        1 => State::RcData,
        2 => State::RawText,
        3 => State::ScriptData,
        4 => State::PlainText,
        5 => State::TagOpen,
        6 => State::EndTagOpen,
        7 => State::TagName,
        8 => State::RcDataLessThanSign,
        9 => State::RcDataEndTagOpen,
        10 => State::RcDataEndTagName,
        11 => State::RawTextLessThanSign,
        12 => State::RawTextEndTagOpen,
        13 => State::RawTextEndTagName,
        14 => State::ScriptDataLessThanSign,
        15 => State::ScriptDataEndTagOpen,
        16 => State::ScriptDataEndTagName,
        17 => State::ScriptDataEscapeStart,
        18 => State::ScriptDataEscapeStartDash,
        19 => State::ScriptDataEscaped,
        20 => State::ScriptDataEscapedDash,
        21 => State::ScriptDataEscapedDashDash,
        22 => State::ScriptDataEscapedLessThanSign,
        23 => State::ScriptDataEscapedEndTagOpen,
        24 => State::ScriptDataEscapedEndTagName,
        25 => State::ScriptDataDoubleEscapeStart,
        26 => State::ScriptDataDoubleEscaped,
        27 => State::ScriptDataDoubleEscapedDash,
        28 => State::ScriptDataDoubleEscapedDashDash,
        29 => State::ScriptDataDoubleEscapedLessThanSign,
        30 => State::ScriptDataDoubleEscapeEnd,
        31 => State::BeforeAttributeName,
        32 => State::AttributeName,
        33 => State::AfterAttributeName,
        34 => State::BeforeAttributeValue,
        35 => State::AttributeValueDoubleQuoted,
        36 => State::AttributeValueSingleQuoted,
        37 => State::AttributeValueUnquoted,
        38 => State::AfterAttributeValueQuoted,
        39 => State::SelfClosingStartTag,
        40 => State::BogusComment,
        41 => State::MarkupDeclarationOpen,
        42 => State::CommentStart,
        43 => State::CommentStartDash,
        44 => State::Comment,
        45 => State::CommentLessThanSign,
        46 => State::CommentLessThanSignBang,
        47 => State::CommentLessThanSignBangDash,
        48 => State::CommentLessThanSignBangDashDash,
        49 => State::CommentEndDash,
        50 => State::CommentEnd,
        51 => State::CommentEndBang,
        52 => State::Doctype,
        53 => State::BeforeDoctypeName,
        54 => State::DoctypeName,
        55 => State::AfterDoctypeName,
        56 => State::AfterDoctypePublicKeyword,
        57 => State::BeforeDoctypePublicIdentifier,
        58 => State::DoctypePublicIdentifierDoubleQuoted,
        59 => State::DoctypePublicIdentifierSingleQuoted,
        60 => State::AfterDoctypePublicIdentifier,
        61 => State::BetweenDoctypePublicAndSystemIdentifiers,
        62 => State::AfterDoctypeSystemKeyword,
        63 => State::BeforeDoctypeSystemIdentifier,
        64 => State::DoctypeSystemIdentifierDoubleQuoted,
        65 => State::DoctypeSystemIdentifierSingleQuoted,
        66 => State::AfterDoctypeSystemIdentifier,
        67 => State::BogusDoctype,
        68 => State::CDataSection,
        69 => State::CDataSectionBracket,
        70 => State::CDataSectionEnd,
        71 => State::CharacterReference,
        72 => State::AmbiguousAmpersand,
        _ => return None,
    })
}

/// Hands a result to JS as a JSON string decoded by the engine's native
/// `JSON.parse`, which is several times faster for large trees than building
/// the objects property by property across the wasm boundary.
fn to_js_value(value: &impl Serialize) -> Result<JsValue, JsValue> {
    let json = serde_json::to_string(value)
        .map_err(|err| JsValue::from_str(&format!("failed to convert parse5 value: {err}")))?;

    js_sys::JSON::parse(&json)
}

#[derive(Serialize)]
struct ParseResult<'a> {
    node: NodeJson<'a>,
    errors: Vec<Value>,
}

fn parse_result_value(document: &NodeRef, errors: Vec<parse5::ParserError>) -> ParseResult<'_> {
    ParseResult {
        node: NodeJson(document),
        errors: errors.into_iter().map(parser_error_value).collect(),
    }
}

fn node_path(root: &NodeRef, node: &NodeRef) -> Option<Vec<Value>> {
    let mut path = Vec::new();

    if node_path_from(root, node, &mut path) {
        Some(path)
    } else {
        None
    }
}

fn node_path_from(current: &NodeRef, target: &NodeRef, path: &mut Vec<Value>) -> bool {
    if Rc::ptr_eq(current, target) {
        return true;
    }

    let (children, template_content) = {
        let current_ref = current.borrow();
        let children = current_ref.child_nodes().cloned().unwrap_or_default();
        let template_content = match &current_ref.data {
            NodeData::Element {
                template_content, ..
            } => template_content.clone(),
            _ => None,
        };

        (children, template_content)
    };

    for (idx, child) in children.iter().enumerate() {
        path.push(json!(idx));

        if node_path_from(child, target, path) {
            return true;
        }

        path.pop();
    }

    if let Some(content) = template_content {
        path.push(json!("content"));

        if node_path_from(&content, target, path) {
            return true;
        }

        path.pop();
    }

    false
}

fn stack_event_value(root: &NodeRef, event: StackEvent) -> Option<Value> {
    let event_node_path = node_path(root, &event.node)?;
    let new_top_path = event
        .new_top
        .and_then(|new_top| node_path(root, &new_top))
        .map(Value::Array)
        .unwrap_or(Value::Null);
    let kind = match event.kind {
        StackEventKind::Push => "push",
        StackEventKind::Pop => "pop",
    };

    Some(json!({
        "type": kind,
        "nodePath": event_node_path,
        "newTopPath": new_top_path,
    }))
}

fn script_event_value(root: &NodeRef, event: ScriptEvent) -> Option<Value> {
    let event_node_path = node_path(root, &event.node)?;

    Some(json!({
        "nodePath": event_node_path,
    }))
}

fn parser_write_result_value(
    root: &NodeRef,
    errors: Vec<parse5::ParserError>,
    stack_events: Vec<StackEvent>,
    script_events: Vec<ScriptEvent>,
) -> Value {
    json!({
        "errors": errors.into_iter().map(parser_error_value).collect::<Vec<_>>(),
        "stackEvents": stack_events
            .into_iter()
            .filter_map(|event| stack_event_value(root, event))
            .collect::<Vec<_>>(),
        "scriptEvents": script_events
            .into_iter()
            .filter_map(|event| script_event_value(root, event))
            .collect::<Vec<_>>(),
    })
}

#[wasm_bindgen]
pub struct WasmParser {
    parser: RustParser,
}

#[wasm_bindgen]
impl WasmParser {
    #[wasm_bindgen(constructor)]
    pub fn new(
        options: JsValue,
        fragment_context: JsValue,
        is_fragment: bool,
    ) -> Result<WasmParser, JsValue> {
        let options = parser_options(options)?;
        let parser = if is_fragment {
            RustParser::get_fragment_parser(fragment_context_from_js(fragment_context)?, options)
        } else {
            RustParser::new(options, None, None)
        };

        Ok(Self { parser })
    }

    pub fn write(&mut self, chunk: &str, is_last_chunk: bool) -> Result<JsValue, JsValue> {
        self.parser.write_chunk(chunk, is_last_chunk);
        self.drain()
    }

    #[wasm_bindgen(js_name = initPlainTextConversion)]
    pub fn init_plain_text_conversion(&mut self) -> Result<JsValue, JsValue> {
        self.parser.init_plain_text_conversion();
        self.drain()
    }

    pub fn resume(&mut self) -> Result<JsValue, JsValue> {
        self.parser.resume();
        self.drain()
    }

    #[wasm_bindgen(js_name = insertHtmlAtCurrentPos)]
    pub fn insert_html_at_current_pos(&mut self, chunk: &str) -> Result<JsValue, JsValue> {
        self.parser.insert_html_at_current_pos(chunk);
        self.drain()
    }

    #[wasm_bindgen(js_name = setScriptHandlerEnabled)]
    pub fn set_script_handler_enabled(&mut self, enabled: bool) {
        self.parser.script_handler_enabled = enabled;

        if !enabled {
            self.parser.paused_on_script = false;
        }
    }

    #[wasm_bindgen(js_name = setBufferWaterline)]
    pub fn set_buffer_waterline(&mut self, buffer_waterline: usize) {
        self.parser.tokenizer.preprocessor.buffer_waterline = buffer_waterline;
    }

    #[wasm_bindgen(js_name = pausedOnScript)]
    pub fn paused_on_script(&self) -> bool {
        self.parser.paused_on_script
    }

    fn drain(&mut self) -> Result<JsValue, JsValue> {
        let errors = std::mem::take(&mut self.parser.errors);
        let stack_events = std::mem::take(&mut self.parser.stack_events);
        let script_events = std::mem::take(&mut self.parser.script_events);
        to_js_value(&parser_write_result_value(
            &self.parser.document,
            errors,
            stack_events,
            script_events,
        ))
    }

    pub fn document(&self) -> Result<JsValue, JsValue> {
        to_js_value(&NodeJson(&self.parser.document))
    }

    #[wasm_bindgen(js_name = getFragment)]
    pub fn get_fragment(&mut self) -> Result<JsValue, JsValue> {
        let fragment = self.parser.get_fragment();
        to_js_value(&NodeJson(&fragment))
    }

    pub fn stopped(&self) -> bool {
        self.parser.stopped
    }
}

#[wasm_bindgen]
pub struct WasmTokenizer {
    tokenizer: Tokenizer,
}

#[wasm_bindgen]
impl WasmTokenizer {
    #[wasm_bindgen(constructor)]
    pub fn new(options: JsValue) -> Result<WasmTokenizer, JsValue> {
        Ok(Self {
            tokenizer: Tokenizer::new(tokenizer_options(options)?),
        })
    }

    #[wasm_bindgen(js_name = setState)]
    pub fn set_state(&mut self, state: u8) -> Result<(), JsValue> {
        let Some(state) = tokenizer_state_from_u8(state) else {
            return Err(JsValue::from_str("Unknown state"));
        };

        self.tokenizer.state = state;
        Ok(())
    }

    #[wasm_bindgen(js_name = getState)]
    pub fn get_state(&self) -> u8 {
        self.tokenizer.state as u8
    }

    #[wasm_bindgen(js_name = setLastStartTagName)]
    pub fn set_last_start_tag_name(&mut self, name: String) {
        self.tokenizer.last_start_tag_name = name;
    }

    #[wasm_bindgen(js_name = getLastStartTagName)]
    pub fn get_last_start_tag_name(&self) -> String {
        self.tokenizer.last_start_tag_name.clone()
    }

    #[wasm_bindgen(js_name = setInForeignNode)]
    pub fn set_in_foreign_node(&mut self, in_foreign_node: bool) {
        self.tokenizer.in_foreign_node = in_foreign_node;
    }

    #[wasm_bindgen(js_name = getInForeignNode)]
    pub fn get_in_foreign_node(&self) -> bool {
        self.tokenizer.in_foreign_node
    }

    #[wasm_bindgen(js_name = setBufferWaterline)]
    pub fn set_buffer_waterline(&mut self, buffer_waterline: usize) {
        self.tokenizer.preprocessor.buffer_waterline = buffer_waterline;
    }

    pub fn active(&self) -> bool {
        self.tokenizer.active
    }

    #[wasm_bindgen(js_name = willDropParsedChunk)]
    pub fn will_drop_parsed_chunk(&self) -> bool {
        self.tokenizer.preprocessor.will_drop_parsed_chunk()
    }

    pub fn write(&mut self, chunk: &str, is_last_chunk: bool) -> Result<JsValue, JsValue> {
        self.tokenizer.write(chunk, is_last_chunk);
        self.drain()
    }

    #[wasm_bindgen(js_name = writeInput)]
    pub fn write_input(&mut self, chunk: &str, is_last_chunk: bool) {
        self.tokenizer.write_input(chunk, is_last_chunk);
    }

    #[wasm_bindgen(js_name = insertHtmlAtCurrentPos)]
    pub fn insert_html_at_current_pos(&mut self, chunk: &str) -> Result<JsValue, JsValue> {
        self.tokenizer.insert_html_at_current_pos(chunk);
        self.drain()
    }

    #[wasm_bindgen(js_name = insertHtmlAtCurrentPosInput)]
    pub fn insert_html_at_current_pos_input(&mut self, chunk: &str) {
        self.tokenizer.active = true;
        self.tokenizer
            .preprocessor
            .insert_html_at_current_pos(chunk);
    }

    #[wasm_bindgen(js_name = nextToken)]
    pub fn next_token(&mut self) -> Result<JsValue, JsValue> {
        let token = self.tokenizer.next_token().map(token_value);
        let errors = self.tokenizer.take_errors();

        to_js_value(&json!({
            "token": token.unwrap_or(Value::Null),
            "errors": errors.into_iter().map(parser_error_value).collect::<Vec<_>>(),
        }))
    }

    fn drain(&mut self) -> Result<JsValue, JsValue> {
        let mut tokens = Vec::new();

        while let Some(token) = self.tokenizer.next_token() {
            tokens.push(token);
        }

        let errors = self.tokenizer.take_errors();
        to_js_value(&tokenizer_result_value(tokens, errors))
    }
}

#[wasm_bindgen(js_name = parse)]
pub fn parse_js(html: &str, options: JsValue) -> Result<JsValue, JsValue> {
    let document = parse(html, parser_options(options)?);
    to_js_value(&NodeJson(&document))
}

#[wasm_bindgen(js_name = parseWithErrors)]
pub fn parse_with_errors_js(html: &str, options: JsValue) -> Result<JsValue, JsValue> {
    let output = parse_with_errors(html, parser_options(options)?);
    to_js_value(&parse_result_value(&output.document, output.errors))
}

#[wasm_bindgen(js_name = parseFragment)]
pub fn parse_fragment_js(html: &str, options: JsValue) -> Result<JsValue, JsValue> {
    let fragment = parse_fragment(html, parser_options(options)?);
    to_js_value(&NodeJson(&fragment))
}

#[wasm_bindgen(js_name = parseFragmentWithErrors)]
pub fn parse_fragment_with_errors_js(html: &str, options: JsValue) -> Result<JsValue, JsValue> {
    let output = parse_fragment_with_errors(html, parser_options(options)?);
    to_js_value(&parse_result_value(&output.document, output.errors))
}

#[wasm_bindgen(js_name = parseFragmentWithContext)]
pub fn parse_fragment_with_context_js(
    context: JsValue,
    html: &str,
    options: JsValue,
) -> Result<JsValue, JsValue> {
    let fragment = parse_fragment_with_context(
        fragment_context_from_js(context)?,
        html,
        parser_options(options)?,
    );
    to_js_value(&NodeJson(&fragment))
}

#[wasm_bindgen(js_name = parseFragmentWithContextAndErrors)]
pub fn parse_fragment_with_context_and_errors_js(
    context: JsValue,
    html: &str,
    options: JsValue,
) -> Result<JsValue, JsValue> {
    let output = parse_fragment_with_context_and_errors(
        fragment_context_from_js(context)?,
        html,
        parser_options(options)?,
    );
    to_js_value(&parse_result_value(&output.document, output.errors))
}

#[wasm_bindgen(js_name = parseFragmentAndSerialize)]
pub fn parse_fragment_and_serialize(html: &str) -> String {
    let fragment = parse_fragment(html, ParserOptions::default());
    parse5::serialize(&fragment, SerializerOptions::default())
}
