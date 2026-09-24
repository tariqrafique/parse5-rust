use crate::common::doctype;
use crate::common::error_codes::{ErrorCode, ParserError};
use crate::common::foreign_content;
use crate::common::html::{
    get_tag_id, is_numbered_header, is_special_element, is_void_element, DocumentMode, TagId,
    NS_HTML, NS_MATHML, NS_SVG,
};
use crate::common::token::{
    get_token_attr, CharacterToken, CommentToken, DoctypeToken, ElementLocation, EofToken,
    Location, LocationWithAttributes, TagToken, Token, TokenType,
};
use crate::parser::formatting_element_list::{Entry, FormattingElementList};
use crate::parser::open_element_stack::OpenElementStack;
use crate::tokenizer::{State as TokenizerState, Tokenizer, TokenizerOptions};
use crate::tree_adapters::default::{
    append_child, create_comment_node, create_document, create_document_fragment, create_element,
    create_text_node, detach_node, get_first_child, is_in_namespace, set_document_mode,
    set_document_type, set_template_content, NodeData, NodeRef,
};

pub mod formatting_element_list;
pub mod open_element_stack;

const TAG_NAME_TEMPLATE: &str = "template";
const TAG_NAME_DOCUMENT_MOCK: &str = "documentmock";
const TAG_NAME_HTML: &str = "html";
const AA_OUTER_LOOP_ITER: usize = 8;
const AA_INNER_LOOP_ITER: usize = 3;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum InsertionMode {
    Initial = 0,
    BeforeHtml,
    BeforeHead,
    InHead,
    InHeadNoScript,
    AfterHead,
    InBody,
    Text,
    InTable,
    InTableText,
    InCaption,
    InColumnGroup,
    InTableBody,
    InRow,
    InCell,
    InSelect,
    InSelectInTable,
    InTemplate,
    AfterBody,
    InFrameset,
    AfterFrameset,
    AfterAfterBody,
    AfterAfterFrameset,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ParserOptions {
    pub scripting_enabled: bool,
    pub source_code_location_info: bool,
}

impl Default for ParserOptions {
    fn default() -> Self {
        Self {
            scripting_enabled: true,
            source_code_location_info: false,
        }
    }
}

#[derive(Debug, Clone)]
pub struct ParserOutput {
    pub document: NodeRef,
    pub errors: Vec<ParserError>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StackEventKind {
    Push,
    Pop,
}

#[derive(Debug, Clone)]
pub struct StackEvent {
    pub kind: StackEventKind,
    pub node: NodeRef,
    pub new_top: Option<NodeRef>,
}

#[derive(Debug, Clone)]
pub struct ScriptEvent {
    pub node: NodeRef,
}

#[derive(Debug)]
pub struct Parser {
    pub options: ParserOptions,
    pub document: NodeRef,
    pub fragment_context: Option<NodeRef>,
    pub tokenizer: Tokenizer,
    pub insertion_mode: InsertionMode,
    pub original_insertion_mode: InsertionMode,
    pub fragment_context_id: TagId,
    pub head_element: Option<NodeRef>,
    pub form_element: Option<NodeRef>,
    pub open_elements: OpenElementStack,
    pub active_formatting_elements: FormattingElementList,
    pub tmpl_insertion_mode_stack: Vec<InsertionMode>,
    pub frameset_ok: bool,
    pub skip_next_new_line: bool,
    pub foster_parenting_enabled: bool,
    pub pending_character_tokens: Vec<CharacterToken>,
    pub has_non_whitespace_pending_character_token: bool,
    pub current_token: Option<Token>,
    pub stopped: bool,
    pub errors: Vec<ParserError>,
    pub stack_events: Vec<StackEvent>,
    pub script_handler_enabled: bool,
    pub paused_on_script: bool,
    pub script_events: Vec<ScriptEvent>,
}

impl Parser {
    pub fn new(
        options: ParserOptions,
        document: Option<NodeRef>,
        fragment_context: Option<NodeRef>,
    ) -> Self {
        let document = document.unwrap_or_else(create_document);
        let fragment_context_id = fragment_context
            .as_ref()
            .and_then(element_tag_name)
            .map(|tag_name| get_tag_id(&tag_name))
            .unwrap_or(TagId::Unknown);
        let open_elements = OpenElementStack::new(document.clone());
        let tokenizer = Tokenizer::new(TokenizerOptions {
            source_code_location_info: options.source_code_location_info,
        });

        Self {
            options,
            document,
            fragment_context,
            tokenizer,
            insertion_mode: InsertionMode::Initial,
            original_insertion_mode: InsertionMode::Initial,
            fragment_context_id,
            head_element: None,
            form_element: None,
            open_elements,
            active_formatting_elements: FormattingElementList::new(),
            tmpl_insertion_mode_stack: Vec::new(),
            frameset_ok: true,
            skip_next_new_line: false,
            foster_parenting_enabled: false,
            pending_character_tokens: Vec::new(),
            has_non_whitespace_pending_character_token: false,
            current_token: None,
            stopped: false,
            errors: Vec::new(),
            stack_events: Vec::new(),
            script_handler_enabled: false,
            paused_on_script: false,
            script_events: Vec::new(),
        }
    }

    pub fn parse(html: &str, options: ParserOptions) -> NodeRef {
        let mut parser = Self::new(options, None, None);
        parser.write(html);
        parser.document
    }

    pub fn parse_with_errors(html: &str, mut options: ParserOptions) -> ParserOutput {
        options.source_code_location_info = true;
        let mut parser = Self::new(options, None, None);
        parser.write(html);
        ParserOutput {
            document: parser.document,
            errors: parser.errors,
        }
    }

    pub fn get_fragment_parser(fragment_context: Option<NodeRef>, options: ParserOptions) -> Self {
        // NOTE: parse5 uses a <template> element as the default fragment
        // context so fragments parse in the same forgiving mode as upstream.
        let fragment_context = fragment_context
            .unwrap_or_else(|| create_element(TAG_NAME_TEMPLATE, NS_HTML, Vec::new()));
        let document_mock = create_element(TAG_NAME_DOCUMENT_MOCK, NS_HTML, Vec::new());
        let mut parser = Self::new(options, Some(document_mock), Some(fragment_context));

        if parser.fragment_context_id == TagId::Template {
            parser
                .tmpl_insertion_mode_stack
                .insert(0, InsertionMode::InTemplate);
        }

        parser.init_tokenizer_for_fragment_parsing();
        parser.insert_fake_root_element();
        parser.reset_insertion_mode();
        parser.find_form_in_fragment_context();
        parser
    }

    pub fn write(&mut self, html: &str) {
        self.write_chunk(html, true);
    }

    pub fn write_chunk(&mut self, html: &str, is_last_chunk: bool) {
        self.tokenizer.in_foreign_node = self.tokenizer_in_foreign_node();
        self.tokenizer.write_input(html, is_last_chunk);
        self.run_parsing_loop();
        self.errors.extend(self.tokenizer.take_errors());
    }

    pub fn resume(&mut self) {
        self.paused_on_script = false;
        self.run_parsing_loop();
        self.errors.extend(self.tokenizer.take_errors());
    }

    pub fn insert_html_at_current_pos(&mut self, html: &str) {
        self.tokenizer.active = true;
        self.tokenizer.preprocessor.insert_html_at_current_pos(html);
        self.run_parsing_loop();
        self.errors.extend(self.tokenizer.take_errors());
    }

    fn run_parsing_loop(&mut self) {
        if self.paused_on_script {
            return;
        }

        while let Some(token) = self.tokenizer.next_token() {
            self.process_token_from_tokenizer(token);
            self.tokenizer.in_foreign_node = self.tokenizer_in_foreign_node();

            if self.paused_on_script {
                break;
            }
        }
    }

    pub fn get_fragment(&mut self) -> NodeRef {
        let fragment = create_document_fragment();

        if let Some(root_element) = get_first_child(&self.document) {
            self.adopt_nodes(&root_element, &fragment);
        }

        fragment
    }

    pub fn init_plain_text_conversion(&mut self) {
        self.insert_fake_root_element();
        self.insert_fake_element("head", TagId::Head);
        self.pop_open_element();
        self.insert_fake_element("body", TagId::Body);
        self.insert_fake_element("pre", TagId::Pre);
        self.insert_characters("\n", None);
        self.insertion_mode = InsertionMode::Text;
        self.original_insertion_mode = InsertionMode::InBody;
        self.tokenizer.state = TokenizerState::PlainText;
    }

    fn init_tokenizer_for_fragment_parsing(&mut self) {
        let Some(fragment_context) = &self.fragment_context else {
            return;
        };

        if !is_in_namespace(fragment_context, NS_HTML) {
            return;
        }

        self.tokenizer.state = match self.fragment_context_id {
            TagId::Title | TagId::Textarea => TokenizerState::RcData,
            TagId::Style
            | TagId::Xmp
            | TagId::Iframe
            | TagId::Noembed
            | TagId::Noframes
            | TagId::Noscript => TokenizerState::RawText,
            TagId::Script => TokenizerState::ScriptData,
            TagId::Plaintext => TokenizerState::PlainText,
            _ => TokenizerState::Data,
        };
    }

    fn reset_insertion_mode(&mut self) {
        for idx in (0..=self.open_elements.stack_top.max(0) as usize).rev() {
            let tag_id = if idx == 0 && self.fragment_context.is_some() {
                self.fragment_context_id
            } else {
                self.open_elements.tag_ids[idx]
            };

            match tag_id {
                TagId::Tr => {
                    self.insertion_mode = InsertionMode::InRow;
                    return;
                }
                TagId::Tbody | TagId::Thead | TagId::Tfoot => {
                    self.insertion_mode = InsertionMode::InTableBody;
                    return;
                }
                TagId::Caption => {
                    self.insertion_mode = InsertionMode::InCaption;
                    return;
                }
                TagId::Colgroup => {
                    self.insertion_mode = InsertionMode::InColumnGroup;
                    return;
                }
                TagId::Table => {
                    self.insertion_mode = InsertionMode::InTable;
                    return;
                }
                TagId::Body => {
                    self.insertion_mode = InsertionMode::InBody;
                    return;
                }
                TagId::Frameset => {
                    self.insertion_mode = InsertionMode::InFrameset;
                    return;
                }
                TagId::Select => {
                    self.reset_insertion_mode_for_select(idx);
                    return;
                }
                TagId::Template => {
                    self.insertion_mode = self
                        .tmpl_insertion_mode_stack
                        .first()
                        .copied()
                        .unwrap_or(InsertionMode::InTemplate);
                    return;
                }
                TagId::Html => {
                    self.insertion_mode = if self.head_element.is_some() {
                        InsertionMode::AfterHead
                    } else {
                        InsertionMode::BeforeHead
                    };
                    return;
                }
                TagId::Td | TagId::Th if idx > 0 => {
                    self.insertion_mode = InsertionMode::InCell;
                    return;
                }
                TagId::Head if idx > 0 => {
                    self.insertion_mode = InsertionMode::InHead;
                    return;
                }
                _ => {}
            }
        }

        self.insertion_mode = InsertionMode::InBody;
    }

    fn reset_insertion_mode_for_select(&mut self, select_idx: usize) {
        if select_idx > 0 {
            for idx in (1..select_idx).rev() {
                match self.open_elements.tag_ids[idx] {
                    TagId::Template => break,
                    TagId::Table => {
                        self.insertion_mode = InsertionMode::InSelectInTable;
                        return;
                    }
                    _ => {}
                }
            }
        }

        self.insertion_mode = InsertionMode::InSelect;
    }

    fn find_form_in_fragment_context(&mut self) {
        let mut node = self.fragment_context.clone();

        while let Some(current) = node {
            if element_tag_name(&current).as_deref() == Some("form") {
                self.form_element = Some(current);
                break;
            }

            node = current.borrow().parent();
        }
    }

    fn process_token(&mut self, token: Token) {
        self.process_token_impl(token, false);
    }

    fn process_token_from_tokenizer(&mut self, token: Token) {
        self.process_token_impl(token, true);
    }

    fn process_token_impl(&mut self, token: Token, report_self_closing_error: bool) {
        match token {
            Token::Character(token) if token.token_type == TokenType::NullCharacter => {
                self.on_null_character_token(token);
            }
            Token::Character(token) if token.token_type == TokenType::WhitespaceCharacter => {
                self.on_whitespace_character(token);
            }
            Token::Character(token) => self.on_character_token(token),
            Token::Comment(token) => self.on_comment(token),
            Token::Doctype(token) => self.on_doctype(token),
            Token::Tag(token) if token.token_type == TokenType::StartTag => {
                if report_self_closing_error {
                    self.remember_current_token(&token);
                }
                self.process_start_tag(token, report_self_closing_error);
            }
            Token::Tag(token) if token.token_type == TokenType::EndTag => {
                self.remember_current_token(&token);
                self.process_end_tag(token);
            }
            Token::Eof(token) => self.on_eof(token),
            _ => {}
        }
    }

    /// Records the tag token being processed. It is only consulted to compute
    /// end locations of popped elements, so the copy is skipped when source
    /// code location info is disabled.
    fn remember_current_token(&mut self, token: &TagToken) {
        if self.options.source_code_location_info {
            self.current_token = Some(Token::Tag(token.clone()));
        }
    }

    fn err_for_token(&mut self, token: &Token, code: ErrorCode, before_token: bool) {
        self.err_at_location(token_location(token), code, before_token);
    }

    fn err_for_tag(&mut self, token: &TagToken, code: ErrorCode) {
        self.err_at_location(
            token.location.as_ref().map(|location| location.location),
            code,
            false,
        );
    }

    fn err_for_doctype(&mut self, token: &DoctypeToken, code: ErrorCode) {
        self.err_at_location(token.location, code, false);
    }

    fn err_at_location(&mut self, location: Option<Location>, code: ErrorCode, before_token: bool) {
        let mut location = location.unwrap_or_else(Location::zero_width_at_start);

        if before_token {
            location.end_line = location.start_line;
            location.end_col = location.start_col;
            location.end_offset = location.start_offset;
        }

        self.errors.push(ParserError { code, location });
    }

    fn on_null_character_token(&mut self, mut token: CharacterToken) {
        self.skip_next_new_line = false;

        if self.tokenizer_in_foreign_node() {
            token.chars = "\u{fffd}".to_string();
            self.insert_characters(&token.chars, token.location);
            return;
        }

        match self.insertion_mode {
            InsertionMode::Initial => self.token_in_initial_mode(Token::Character(token)),
            InsertionMode::BeforeHtml => self.token_before_html(Token::Character(token)),
            InsertionMode::BeforeHead => self.token_before_head(Token::Character(token)),
            InsertionMode::InHead => self.token_in_head(Token::Character(token)),
            InsertionMode::InHeadNoScript => self.token_in_head_no_script(Token::Character(token)),
            InsertionMode::AfterHead => self.token_after_head(Token::Character(token)),
            InsertionMode::Text => self.insert_characters(&token.chars, token.location),
            InsertionMode::InTable | InsertionMode::InTableBody | InsertionMode::InRow => {
                self.character_in_table(token)
            }
            InsertionMode::InColumnGroup => self.token_in_column_group(Token::Character(token)),
            InsertionMode::AfterBody => self.token_after_body(Token::Character(token)),
            InsertionMode::AfterAfterBody => self.token_after_after_body(Token::Character(token)),
            InsertionMode::InBody
            | InsertionMode::InCaption
            | InsertionMode::InCell
            | InsertionMode::InTemplate
            | InsertionMode::InSelect
            | InsertionMode::InSelectInTable
            | InsertionMode::InTableText
            | InsertionMode::InFrameset
            | InsertionMode::AfterFrameset
            | InsertionMode::AfterAfterFrameset => {}
        }
    }

    fn on_character_token(&mut self, token: CharacterToken) {
        self.skip_next_new_line = false;

        match self.insertion_mode {
            InsertionMode::Initial => self.token_in_initial_mode(Token::Character(token)),
            InsertionMode::BeforeHtml => self.token_before_html(Token::Character(token)),
            InsertionMode::BeforeHead => self.token_before_head(Token::Character(token)),
            InsertionMode::InHead => self.token_in_head(Token::Character(token)),
            InsertionMode::InHeadNoScript => self.token_in_head_no_script(Token::Character(token)),
            InsertionMode::AfterHead => self.token_after_head(Token::Character(token)),
            InsertionMode::InBody
            | InsertionMode::InCaption
            | InsertionMode::InCell
            | InsertionMode::InTemplate => {
                self.character_in_body(token);
            }
            InsertionMode::Text | InsertionMode::InSelect | InsertionMode::InSelectInTable => {
                self.insert_characters(&token.chars, token.location)
            }
            InsertionMode::InTable | InsertionMode::InTableBody | InsertionMode::InRow => {
                self.character_in_table(token)
            }
            InsertionMode::InTableText => self.character_in_table_text(token),
            InsertionMode::InColumnGroup => self.token_in_column_group(Token::Character(token)),
            InsertionMode::AfterBody => self.token_after_body(Token::Character(token)),
            InsertionMode::AfterAfterBody => self.token_after_after_body(Token::Character(token)),
            InsertionMode::InFrameset
            | InsertionMode::AfterFrameset
            | InsertionMode::AfterAfterFrameset => {}
        }
    }

    fn on_whitespace_character(&mut self, mut token: CharacterToken) {
        if self.skip_next_new_line {
            self.skip_next_new_line = false;

            if token.chars.starts_with('\n') {
                token.chars.remove(0);
                if token.chars.is_empty() {
                    return;
                }
            }
        }

        match self.insertion_mode {
            InsertionMode::Initial | InsertionMode::BeforeHtml | InsertionMode::BeforeHead => {}
            InsertionMode::InHead
            | InsertionMode::InHeadNoScript
            | InsertionMode::AfterHead
            | InsertionMode::Text
            | InsertionMode::InColumnGroup
            | InsertionMode::InSelect
            | InsertionMode::InSelectInTable
            | InsertionMode::InFrameset
            | InsertionMode::AfterFrameset => {
                self.insert_characters(&token.chars, token.location);
            }
            InsertionMode::InBody
            | InsertionMode::InCaption
            | InsertionMode::InCell
            | InsertionMode::InTemplate
            | InsertionMode::AfterBody
            | InsertionMode::AfterAfterBody
            | InsertionMode::AfterAfterFrameset => self.whitespace_character_in_body(token),
            InsertionMode::InTable | InsertionMode::InTableBody | InsertionMode::InRow => {
                self.character_in_table(token)
            }
            InsertionMode::InTableText => self.whitespace_character_in_table_text(token),
        }
    }

    fn on_comment(&mut self, token: CommentToken) {
        self.skip_next_new_line = false;

        if self.insertion_mode == InsertionMode::InTableText {
            self.token_in_table_text(Token::Comment(token));
            return;
        }

        let parent = match self.insertion_mode {
            InsertionMode::AfterAfterBody | InsertionMode::AfterAfterFrameset => {
                self.document.clone()
            }
            InsertionMode::AfterBody => self
                .open_elements
                .items
                .first()
                .cloned()
                .unwrap_or_else(|| self.document.clone()),
            _ => self
                .open_elements
                .current_tmpl_content_or_node()
                .unwrap_or_else(|| self.document.clone()),
        };

        self.append_comment_node(token.data, token.location, &parent);
    }

    fn on_doctype(&mut self, token: DoctypeToken) {
        self.skip_next_new_line = false;

        match self.insertion_mode {
            InsertionMode::Initial => self.doctype_in_initial_mode(token),
            InsertionMode::BeforeHead
            | InsertionMode::InHead
            | InsertionMode::InHeadNoScript
            | InsertionMode::AfterHead => {
                self.err_for_doctype(&token, ErrorCode::MisplacedDoctype);
            }
            InsertionMode::InTableText => self.token_in_table_text(Token::Doctype(token)),
            _ => {}
        }
    }

    fn on_eof(&mut self, token: EofToken) {
        match self.insertion_mode {
            InsertionMode::Initial => self.token_in_initial_mode(Token::Eof(token)),
            InsertionMode::BeforeHtml => self.token_before_html(Token::Eof(token)),
            InsertionMode::BeforeHead => self.token_before_head(Token::Eof(token)),
            InsertionMode::InHead => self.token_in_head(Token::Eof(token)),
            InsertionMode::InHeadNoScript => self.token_in_head_no_script(Token::Eof(token)),
            InsertionMode::AfterHead => self.token_after_head(Token::Eof(token)),
            InsertionMode::Text => self.eof_in_text(token),
            InsertionMode::InTableText => self.token_in_table_text(Token::Eof(token)),
            InsertionMode::InTemplate => self.eof_in_template(Token::Eof(token)),
            InsertionMode::InBody
            | InsertionMode::InTable
            | InsertionMode::InCaption
            | InsertionMode::InColumnGroup
            | InsertionMode::InTableBody
            | InsertionMode::InRow
            | InsertionMode::InCell
            | InsertionMode::InSelect
            | InsertionMode::InSelectInTable => self.eof_in_body(Token::Eof(token)),
            _ => self.stop_parsing(Token::Eof(token)),
        }
    }

    fn process_start_tag(&mut self, token: TagToken, report_self_closing_error: bool) {
        self.skip_next_new_line = false;

        let should_report_unacknowledged_self_closing = report_self_closing_error
            && token.self_closing
            && !self.start_tag_acknowledges_self_closing(&token);

        if self.should_process_start_tag_token_in_foreign_content(&token) {
            self.start_tag_in_foreign_content(token.clone());
        } else {
            self.start_tag_outside_foreign_content(token.clone());
        }

        if should_report_unacknowledged_self_closing {
            self.err_for_tag(
                &token,
                ErrorCode::NonVoidHtmlElementStartTagWithTrailingSolidus,
            );
        }
    }

    fn start_tag_acknowledges_self_closing(&self, token: &TagToken) -> bool {
        is_void_element(&token.tag_name)
            || matches!(token.tag_id, TagId::Image | TagId::Math | TagId::Svg)
            || (self.should_process_start_tag_token_in_foreign_content(token)
                && !foreign_content::causes_exit(token))
    }

    fn start_tag_outside_foreign_content(&mut self, token: TagToken) {
        match self.insertion_mode {
            InsertionMode::Initial => {
                self.token_in_initial_mode(Token::Tag(token));
                return;
            }
            InsertionMode::BeforeHtml => {
                self.start_tag_before_html(token);
                return;
            }
            InsertionMode::BeforeHead => {
                self.start_tag_before_head(token);
                return;
            }
            InsertionMode::InHead => {
                self.start_tag_in_head(token);
                return;
            }
            InsertionMode::InHeadNoScript => {
                self.start_tag_in_head_no_script(token);
                return;
            }
            InsertionMode::AfterHead => {
                self.start_tag_after_head(token);
                return;
            }
            InsertionMode::InTable => {
                self.start_tag_in_table(token);
                return;
            }
            InsertionMode::InTableText => {
                self.token_in_table_text(Token::Tag(token));
                return;
            }
            InsertionMode::InCaption => {
                self.start_tag_in_caption(token);
                return;
            }
            InsertionMode::InColumnGroup => {
                self.start_tag_in_column_group(token);
                return;
            }
            InsertionMode::InTableBody => {
                self.start_tag_in_table_body(token);
                return;
            }
            InsertionMode::InRow => {
                self.start_tag_in_row(token);
                return;
            }
            InsertionMode::InCell => {
                self.start_tag_in_cell(token);
                return;
            }
            InsertionMode::InSelect => {
                self.start_tag_in_select(token);
                return;
            }
            InsertionMode::InSelectInTable => {
                self.start_tag_in_select_in_table(token);
                return;
            }
            InsertionMode::InTemplate => {
                self.start_tag_in_template(token);
                return;
            }
            InsertionMode::AfterBody => {
                self.start_tag_after_body(token);
                return;
            }
            InsertionMode::InFrameset => {
                self.start_tag_in_frameset(token);
                return;
            }
            InsertionMode::AfterFrameset => {
                self.start_tag_after_frameset(token);
                return;
            }
            InsertionMode::AfterAfterBody => {
                self.start_tag_after_after_body(token);
                return;
            }
            InsertionMode::AfterAfterFrameset => {
                self.start_tag_after_after_frameset(token);
                return;
            }
            _ => {}
        }

        self.start_tag_in_body(token);
    }

    fn start_tag_in_body(&mut self, token: TagToken) {
        match token.tag_id {
            TagId::I
            | TagId::S
            | TagId::B
            | TagId::U
            | TagId::Em
            | TagId::Tt
            | TagId::Big
            | TagId::Code
            | TagId::Font
            | TagId::Small
            | TagId::Strike
            | TagId::Strong => self.formatting_start_tag_in_body(token),
            TagId::A => self.anchor_start_tag_in_body(token),
            TagId::H1 | TagId::H2 | TagId::H3 | TagId::H4 | TagId::H5 | TagId::H6 => {
                self.numbered_header_start_tag_in_body(token)
            }
            TagId::P
            | TagId::Dl
            | TagId::Ol
            | TagId::Ul
            | TagId::Div
            | TagId::Dir
            | TagId::Nav
            | TagId::Main
            | TagId::Menu
            | TagId::Aside
            | TagId::Center
            | TagId::Figure
            | TagId::Footer
            | TagId::Header
            | TagId::Hgroup
            | TagId::Dialog
            | TagId::Details
            | TagId::Address
            | TagId::Article
            | TagId::Search
            | TagId::Section
            | TagId::Summary
            | TagId::Fieldset
            | TagId::Blockquote
            | TagId::Figcaption => self.address_start_tag_in_body(token),
            TagId::Li | TagId::Dd | TagId::Dt => self.list_item_start_tag_in_body(token),
            TagId::Br | TagId::Img | TagId::Wbr | TagId::Area | TagId::Embed | TagId::Keygen => {
                self.area_start_tag_in_body(token)
            }
            TagId::Hr => self.hr_start_tag_in_body(token),
            TagId::Rb | TagId::Rtc => self.rb_start_tag_in_body(token),
            TagId::Rt | TagId::Rp => self.rt_start_tag_in_body(token),
            TagId::Pre | TagId::Listing => self.pre_start_tag_in_body(token),
            TagId::Base
            | TagId::Basefont
            | TagId::Bgsound
            | TagId::Link
            | TagId::Meta
            | TagId::Script
            | TagId::Style
            | TagId::Template
            | TagId::Title => self.start_tag_in_head(token),
            TagId::Form => self.form_start_tag_in_body(token),
            TagId::Nobr => self.nobr_start_tag_in_body(token),
            TagId::Iframe => self.iframe_start_tag_in_body(token),
            TagId::Math => self.math_start_tag_in_body(token),
            TagId::Table => self.table_start_tag_in_body(token),
            TagId::Input => self.input_start_tag_in_body(token),
            TagId::Param | TagId::Track | TagId::Source => self.param_start_tag_in_body(token),
            TagId::Image => self.image_start_tag_in_body(token),
            TagId::Button => self.button_start_tag_in_body(token),
            TagId::Applet | TagId::Object | TagId::Marquee => self.applet_start_tag_in_body(token),
            TagId::Noembed | TagId::Noframes => self.raw_text_start_tag_in_body(token),
            TagId::Select => self.select_start_tag_in_body(token),
            TagId::Option | TagId::Optgroup => self.optgroup_start_tag_in_body(token),
            TagId::Noscript if self.options.scripting_enabled => {
                self.raw_text_start_tag_in_body(token)
            }
            TagId::Plaintext => self.plaintext_start_tag_in_body(token),
            TagId::Svg => self.svg_start_tag_in_body(token),
            TagId::Textarea => self.textarea_start_tag_in_body(token),
            TagId::Xmp => self.xmp_start_tag_in_body(token),
            TagId::Html => self.html_start_tag_in_body(token),
            TagId::Body => self.body_start_tag_in_body(token),
            TagId::Frameset => self.frameset_start_tag_in_body(token),
            TagId::Col
            | TagId::Th
            | TagId::Td
            | TagId::Tr
            | TagId::Head
            | TagId::Frame
            | TagId::Tbody
            | TagId::Tfoot
            | TagId::Thead
            | TagId::Caption
            | TagId::Colgroup => {}
            _ => {
                if is_void_element(&token.tag_name) {
                    self.reconstruct_active_formatting_elements();
                    self.append_element(token, NS_HTML);
                } else {
                    self.generic_start_tag_in_body(token);
                }
            }
        }
    }

    fn process_end_tag(&mut self, token: TagToken) {
        self.skip_next_new_line = false;

        if self.current_not_in_html() {
            self.end_tag_in_foreign_content(token);
        } else {
            self.end_tag_outside_foreign_content(token);
        }
    }

    fn end_tag_outside_foreign_content(&mut self, token: TagToken) {
        match self.insertion_mode {
            InsertionMode::Initial => self.token_in_initial_mode(Token::Tag(token)),
            InsertionMode::BeforeHtml => self.end_tag_before_html(token),
            InsertionMode::BeforeHead => self.end_tag_before_head(token),
            InsertionMode::InHead => self.end_tag_in_head(token),
            InsertionMode::InHeadNoScript => self.end_tag_in_head_no_script(token),
            InsertionMode::AfterHead => self.end_tag_after_head(token),
            InsertionMode::Text => self.end_tag_in_text(token),
            InsertionMode::InTable => self.end_tag_in_table(token),
            InsertionMode::InTableText => self.token_in_table_text(Token::Tag(token)),
            InsertionMode::InCaption => self.end_tag_in_caption(token),
            InsertionMode::InColumnGroup => self.end_tag_in_column_group(token),
            InsertionMode::InTableBody => self.end_tag_in_table_body(token),
            InsertionMode::InRow => self.end_tag_in_row(token),
            InsertionMode::InCell => self.end_tag_in_cell(token),
            InsertionMode::InSelect => self.end_tag_in_select(token),
            InsertionMode::InSelectInTable => self.end_tag_in_select_in_table(token),
            InsertionMode::InBody => self.end_tag_in_body(token),
            InsertionMode::InTemplate => self.end_tag_in_template(token),
            InsertionMode::AfterBody => self.end_tag_after_body(token),
            InsertionMode::InFrameset => self.end_tag_in_frameset(token),
            InsertionMode::AfterFrameset => self.end_tag_after_frameset(token),
            InsertionMode::AfterAfterBody => self.token_after_after_body(Token::Tag(token)),
            InsertionMode::AfterAfterFrameset => {}
        }
    }

    fn doctype_in_initial_mode(&mut self, token: DoctypeToken) {
        let name = token.name.clone().unwrap_or_default();
        let public_id = token.public_id.clone().unwrap_or_default();
        let system_id = token.system_id.clone().unwrap_or_default();
        set_document_type(&self.document, &name, &public_id, &system_id);

        if self.options.source_code_location_info {
            if let Some(location) = token.location {
                if let Some(doctype_node) =
                    self.document.borrow().child_nodes().and_then(|children| {
                        children
                            .iter()
                            .find(|node| node.borrow().is_document_type())
                            .cloned()
                    })
                {
                    doctype_node
                        .borrow_mut()
                        .set_source_code_location(Some(ElementLocation::from(location)));
                }
            }
        }

        let mode = doctype::document_mode(
            token.name.as_deref(),
            token.public_id.as_deref(),
            token.system_id.as_deref(),
            token.force_quirks,
        );

        if !doctype::is_conforming(
            token.name.as_deref(),
            token.public_id.as_deref(),
            token.system_id.as_deref(),
        ) {
            self.err_for_doctype(&token, ErrorCode::NonConformingDoctype);
        }

        set_document_mode(&self.document, mode);
        self.insertion_mode = InsertionMode::BeforeHtml;
    }

    fn token_in_initial_mode(&mut self, token: Token) {
        self.err_for_token(&token, ErrorCode::MissingDoctype, true);
        set_document_mode(&self.document, DocumentMode::Quirks);
        self.insertion_mode = InsertionMode::BeforeHtml;
        self.process_token(token);
    }

    fn start_tag_before_html(&mut self, token: TagToken) {
        if token.tag_id == TagId::Html {
            self.insert_element(token, NS_HTML);
            self.insertion_mode = InsertionMode::BeforeHead;
        } else {
            self.token_before_html(Token::Tag(token));
        }
    }

    fn end_tag_before_html(&mut self, token: TagToken) {
        if matches!(
            token.tag_id,
            TagId::Html | TagId::Head | TagId::Body | TagId::Br
        ) {
            self.token_before_html(Token::Tag(token));
        }
    }

    fn token_before_html(&mut self, token: Token) {
        self.insert_fake_root_element();
        self.insertion_mode = InsertionMode::BeforeHead;
        self.process_token(token);
    }

    fn start_tag_before_head(&mut self, token: TagToken) {
        match token.tag_id {
            TagId::Html => self.start_tag_in_body(token),
            TagId::Head => {
                self.insert_element(token, NS_HTML);
                self.head_element = self.open_elements.current.clone();
                self.insertion_mode = InsertionMode::InHead;
            }
            _ => self.token_before_head(Token::Tag(token)),
        }
    }

    fn end_tag_before_head(&mut self, token: TagToken) {
        if matches!(
            token.tag_id,
            TagId::Head | TagId::Body | TagId::Html | TagId::Br
        ) {
            self.token_before_head(Token::Tag(token));
        } else {
            self.err_for_tag(&token, ErrorCode::EndTagWithoutMatchingOpenElement);
        }
    }

    fn token_before_head(&mut self, token: Token) {
        self.insert_fake_element("head", TagId::Head);
        self.head_element = self.open_elements.current.clone();
        self.insertion_mode = InsertionMode::InHead;
        self.process_token(token);
    }

    fn start_tag_in_head(&mut self, token: TagToken) {
        match token.tag_id {
            TagId::Html => self.start_tag_in_body(token),
            TagId::Base | TagId::Basefont | TagId::Bgsound | TagId::Link | TagId::Meta => {
                self.append_element(token, NS_HTML);
            }
            TagId::Title => self.switch_to_text_parsing(token, TokenizerState::RcData),
            TagId::Noscript if self.options.scripting_enabled => {
                self.switch_to_text_parsing(token, TokenizerState::RawText);
            }
            TagId::Noscript => {
                self.insert_element(token, NS_HTML);
                self.insertion_mode = InsertionMode::InHeadNoScript;
            }
            TagId::Noframes | TagId::Style => {
                self.switch_to_text_parsing(token, TokenizerState::RawText);
            }
            TagId::Script => self.switch_to_text_parsing(token, TokenizerState::ScriptData),
            TagId::Template => {
                self.insert_template(token);
                self.active_formatting_elements.insert_marker();
                self.frameset_ok = false;
                self.insertion_mode = InsertionMode::InTemplate;
                self.tmpl_insertion_mode_stack
                    .insert(0, InsertionMode::InTemplate);
            }
            TagId::Head => self.err_for_tag(&token, ErrorCode::MisplacedStartTagForHeadElement),
            _ => self.token_in_head(Token::Tag(token)),
        }
    }

    fn end_tag_in_head(&mut self, token: TagToken) {
        match token.tag_id {
            TagId::Head => {
                self.pop_current_element_for_token(
                    token.location.map(|location| location.location),
                );
                self.insertion_mode = InsertionMode::AfterHead;
            }
            TagId::Body | TagId::Br | TagId::Html => self.token_in_head(Token::Tag(token)),
            TagId::Template => self.template_end_tag_in_head(token),
            _ => self.err_for_tag(&token, ErrorCode::EndTagWithoutMatchingOpenElement),
        }
    }

    fn template_end_tag_in_head(&mut self, token: TagToken) {
        if self.open_elements.tmpl_count > 0 {
            self.generate_implied_end_tags_thoroughly();

            if self.open_elements.current_tag_id != Some(TagId::Template) {
                self.err_for_tag(&token, ErrorCode::ClosingOfElementWithOpenChildElements);
            }

            self.update_last_element_end_location(TagId::Template, token.location);
            self.pop_open_elements_until_tag_name_popped(TagId::Template);
            self.active_formatting_elements.clear_to_last_marker();

            if !self.tmpl_insertion_mode_stack.is_empty() {
                self.tmpl_insertion_mode_stack.remove(0);
            }

            self.reset_insertion_mode();
        } else {
            self.err_for_tag(&token, ErrorCode::EndTagWithoutMatchingOpenElement);
        }
    }

    fn token_in_head(&mut self, token: Token) {
        self.pop_current_element_for_token(token_start_location(&token));
        self.insertion_mode = InsertionMode::AfterHead;
        self.process_token(token);
    }

    fn start_tag_in_head_no_script(&mut self, token: TagToken) {
        match token.tag_id {
            TagId::Html => self.start_tag_in_body(token),
            TagId::Basefont
            | TagId::Bgsound
            | TagId::Head
            | TagId::Link
            | TagId::Meta
            | TagId::Noframes
            | TagId::Style => self.start_tag_in_head(token),
            TagId::Noscript => self.err_for_tag(&token, ErrorCode::NestedNoscriptInHead),
            _ => self.token_in_head_no_script(Token::Tag(token)),
        }
    }

    fn end_tag_in_head_no_script(&mut self, token: TagToken) {
        match token.tag_id {
            TagId::Noscript => {
                self.pop_current_element_for_token(
                    token.location.map(|location| location.location),
                );
                self.insertion_mode = InsertionMode::InHead;
            }
            TagId::Br => self.token_in_head_no_script(Token::Tag(token)),
            _ => self.err_for_tag(&token, ErrorCode::EndTagWithoutMatchingOpenElement),
        }
    }

    fn token_in_head_no_script(&mut self, token: Token) {
        let code = if matches!(token, Token::Eof(_)) {
            ErrorCode::OpenElementsLeftAfterEof
        } else {
            ErrorCode::DisallowedContentInNoscriptInHead
        };
        self.err_for_token(&token, code, false);
        self.pop_current_element_for_token(token_start_location(&token));
        self.insertion_mode = InsertionMode::InHead;
        self.process_token(token);
    }

    fn start_tag_after_head(&mut self, token: TagToken) {
        match token.tag_id {
            TagId::Html => self.start_tag_in_body(token),
            TagId::Body => {
                self.insert_element(token, NS_HTML);
                self.frameset_ok = false;
                self.insertion_mode = InsertionMode::InBody;
            }
            TagId::Frameset => {
                self.insert_element(token, NS_HTML);
                self.insertion_mode = InsertionMode::InFrameset;
            }
            TagId::Base
            | TagId::Basefont
            | TagId::Bgsound
            | TagId::Link
            | TagId::Meta
            | TagId::Noframes
            | TagId::Script
            | TagId::Style
            | TagId::Template
            | TagId::Title => {
                self.err_for_tag(&token, ErrorCode::AbandonedHeadElementChild);
                if let Some(head) = self.head_element.clone() {
                    self.push_open_element(head.clone(), TagId::Head);
                    self.start_tag_in_head(token);
                    self.remove_open_element(&head);
                }
            }
            TagId::Head => self.err_for_tag(&token, ErrorCode::MisplacedStartTagForHeadElement),
            _ => self.token_after_head(Token::Tag(token)),
        }
    }

    fn end_tag_after_head(&mut self, token: TagToken) {
        match token.tag_id {
            TagId::Body | TagId::Html | TagId::Br => self.token_after_head(Token::Tag(token)),
            TagId::Template => self.template_end_tag_in_head(token),
            _ => self.err_for_tag(&token, ErrorCode::EndTagWithoutMatchingOpenElement),
        }
    }

    fn token_after_head(&mut self, token: Token) {
        self.insert_fake_element("body", TagId::Body);
        self.insertion_mode = InsertionMode::InBody;
        self.mode_in_body(token);
    }

    fn mode_in_body(&mut self, token: Token) {
        match token {
            Token::Character(token) => {
                if token.token_type == TokenType::WhitespaceCharacter {
                    self.whitespace_character_in_body(token);
                } else if token.token_type == TokenType::Character {
                    self.character_in_body(token);
                }
            }
            Token::Comment(token) => self.on_comment(token),
            Token::Tag(token) if token.token_type == TokenType::StartTag => {
                self.start_tag_in_body(token)
            }
            Token::Tag(token) if token.token_type == TokenType::EndTag => {
                self.end_tag_in_body(token)
            }
            Token::Eof(token) => self.stop_parsing(Token::Eof(token)),
            _ => {}
        }
    }

    fn whitespace_character_in_body(&mut self, token: crate::common::token::CharacterToken) {
        self.reconstruct_active_formatting_elements();
        self.insert_characters(&token.chars, token.location);
    }

    fn character_in_body(&mut self, token: crate::common::token::CharacterToken) {
        self.reconstruct_active_formatting_elements();
        self.insert_characters(&token.chars, token.location);
        self.frameset_ok = false;
    }

    fn reconstruct_active_formatting_elements(&mut self) {
        let list_length = self.active_formatting_elements.entries.len();

        if list_length == 0 {
            return;
        }

        let end_index = self
            .active_formatting_elements
            .entries
            .iter()
            .position(|entry| {
                entry.is_marker()
                    || entry
                        .element
                        .as_ref()
                        .is_some_and(|element| self.open_elements.contains(element))
            });

        let Some(unopen_idx) = end_index
            .map(|idx| idx.checked_sub(1))
            .unwrap_or_else(|| Some(list_length - 1))
        else {
            return;
        };

        for idx in (0..=unopen_idx).rev() {
            let Some(token) = self.active_formatting_elements.entries[idx].token.clone() else {
                continue;
            };
            let namespace_uri = self.active_formatting_elements.entries[idx]
                .element
                .as_ref()
                .and_then(namespace_uri)
                .unwrap_or_else(|| NS_HTML.to_string());
            let element = self.insert_element(token, &namespace_uri);
            self.active_formatting_elements.entries[idx].element = Some(element);
        }
    }

    fn close_p_element(&mut self) {
        self.generate_implied_end_tags_with_exclusion(TagId::P);
        self.pop_open_elements_until_tag_name_popped(TagId::P);
    }

    fn address_start_tag_in_body(&mut self, token: TagToken) {
        if self.open_elements.has_in_button_scope(TagId::P) {
            self.close_p_element();
        }

        self.insert_element(token, NS_HTML);
    }

    fn numbered_header_start_tag_in_body(&mut self, token: TagToken) {
        if self.open_elements.has_in_button_scope(TagId::P) {
            self.close_p_element();
        }

        if self
            .open_elements
            .current_tag_id
            .is_some_and(is_numbered_header)
        {
            self.pop_open_element();
        }

        self.insert_element(token, NS_HTML);
    }

    fn pre_start_tag_in_body(&mut self, token: TagToken) {
        if self.open_elements.has_in_button_scope(TagId::P) {
            self.close_p_element();
        }

        self.insert_element(token, NS_HTML);
        self.skip_next_new_line = true;
        self.frameset_ok = false;
    }

    fn form_start_tag_in_body(&mut self, token: TagToken) {
        let in_template = self.open_elements.tmpl_count > 0;

        if self.form_element.is_none() || in_template {
            if self.open_elements.has_in_button_scope(TagId::P) {
                self.close_p_element();
            }

            let form = self.insert_element(token, NS_HTML);

            if !in_template {
                self.form_element = Some(form);
            }
        }
    }

    fn list_item_start_tag_in_body(&mut self, token: TagToken) {
        self.frameset_ok = false;
        let tag_id = token.tag_id;

        for idx in (0..=self.open_elements.stack_top.max(0) as usize).rev() {
            let element_id = self.open_elements.tag_ids[idx];

            if (tag_id == TagId::Li && element_id == TagId::Li)
                || (matches!(tag_id, TagId::Dd | TagId::Dt)
                    && matches!(element_id, TagId::Dd | TagId::Dt))
            {
                self.generate_implied_end_tags_with_exclusion(element_id);
                self.pop_open_elements_until_tag_name_popped(element_id);
                break;
            }

            if element_id != TagId::Address
                && element_id != TagId::Div
                && element_id != TagId::P
                && self.is_special_element(&self.open_elements.items[idx], element_id)
            {
                break;
            }
        }

        if self.open_elements.has_in_button_scope(TagId::P) {
            self.close_p_element();
        }

        self.insert_element(token, NS_HTML);
    }

    fn button_start_tag_in_body(&mut self, token: TagToken) {
        if self.open_elements.has_in_scope(TagId::Button) {
            self.generate_implied_end_tags();
            self.pop_open_elements_until_tag_name_popped(TagId::Button);
        }

        self.reconstruct_active_formatting_elements();
        self.insert_element(token, NS_HTML);
        self.frameset_ok = false;
    }

    fn nobr_start_tag_in_body(&mut self, token: TagToken) {
        self.reconstruct_active_formatting_elements();

        if self.open_elements.has_in_scope(TagId::Nobr) {
            self.call_adoption_agency(token.clone());
            self.reconstruct_active_formatting_elements();
        }

        let element = self.insert_element(token.clone(), NS_HTML);
        self.active_formatting_elements.push_element(element, token);
    }

    fn applet_start_tag_in_body(&mut self, token: TagToken) {
        self.reconstruct_active_formatting_elements();
        self.insert_element(token, NS_HTML);
        self.active_formatting_elements.insert_marker();
        self.frameset_ok = false;
    }

    fn table_start_tag_in_body(&mut self, token: TagToken) {
        if self.document_mode() != DocumentMode::Quirks
            && self.open_elements.has_in_button_scope(TagId::P)
        {
            self.close_p_element();
        }

        self.insert_element(token, NS_HTML);
        self.frameset_ok = false;
        self.insertion_mode = InsertionMode::InTable;
    }

    fn select_start_tag_in_body(&mut self, token: TagToken) {
        self.reconstruct_active_formatting_elements();
        self.insert_element(token, NS_HTML);
        self.frameset_ok = false;

        self.insertion_mode = match self.insertion_mode {
            InsertionMode::InTable
            | InsertionMode::InCaption
            | InsertionMode::InTableBody
            | InsertionMode::InRow
            | InsertionMode::InCell => InsertionMode::InSelectInTable,
            _ => InsertionMode::InSelect,
        };
    }

    fn optgroup_start_tag_in_body(&mut self, token: TagToken) {
        if self.open_elements.current_tag_id == Some(TagId::Option) {
            self.pop_open_element();
        }

        self.reconstruct_active_formatting_elements();
        self.insert_element(token, NS_HTML);
    }

    fn start_tag_in_select(&mut self, token: TagToken) {
        match token.tag_id {
            TagId::Html => self.start_tag_in_body(token),
            TagId::Option => {
                if self.open_elements.current_tag_id == Some(TagId::Option) {
                    self.pop_open_element();
                }

                self.insert_element(token, NS_HTML);
            }
            TagId::Optgroup => {
                if self.open_elements.current_tag_id == Some(TagId::Option) {
                    self.pop_open_element();
                }

                if self.open_elements.current_tag_id == Some(TagId::Optgroup) {
                    self.pop_open_element();
                }

                self.insert_element(token, NS_HTML);
            }
            TagId::Hr => {
                if self.open_elements.current_tag_id == Some(TagId::Option) {
                    self.pop_open_element();
                }

                if self.open_elements.current_tag_id == Some(TagId::Optgroup) {
                    self.pop_open_element();
                }

                let mut token = token;
                token.ack_self_closing = true;
                self.append_element(token, NS_HTML);
            }
            TagId::Input | TagId::Keygen | TagId::Textarea | TagId::Select => {
                if self.open_elements.has_in_select_scope(TagId::Select) {
                    self.pop_open_elements_until_tag_name_popped(TagId::Select);
                    self.reset_insertion_mode();

                    if token.tag_id != TagId::Select {
                        self.process_start_tag(token, false);
                    }
                }
            }
            TagId::Script | TagId::Template => self.start_tag_in_head(token),
            _ => {}
        }
    }

    fn end_tag_in_select(&mut self, token: TagToken) {
        match token.tag_id {
            TagId::Optgroup => {
                if self.open_elements.stack_top > 0
                    && self.open_elements.current_tag_id == Some(TagId::Option)
                    && self
                        .open_elements
                        .tag_ids
                        .get(self.open_elements.stack_top as usize - 1)
                        == Some(&TagId::Optgroup)
                {
                    self.pop_open_element();
                }

                if self.open_elements.current_tag_id == Some(TagId::Optgroup) {
                    self.pop_current_element_for_token(
                        token.location.map(|location| location.location),
                    );
                }
            }
            TagId::Option => {
                if self.open_elements.current_tag_id == Some(TagId::Option) {
                    self.pop_current_element_for_token(
                        token.location.map(|location| location.location),
                    );
                }
            }
            TagId::Select => {
                if self.open_elements.has_in_select_scope(TagId::Select) {
                    self.update_last_element_end_location(TagId::Select, token.location);
                    self.pop_open_elements_until_tag_name_popped(TagId::Select);
                    self.reset_insertion_mode();
                }
            }
            TagId::Template => self.template_end_tag_in_head(token),
            _ => {}
        }
    }

    fn start_tag_in_select_in_table(&mut self, token: TagToken) {
        if is_table_select_boundary_tag(token.tag_id) {
            self.pop_open_elements_until_tag_name_popped(TagId::Select);
            self.reset_insertion_mode();
            self.process_start_tag(token, false);
        } else {
            self.start_tag_in_select(token);
        }
    }

    fn end_tag_in_select_in_table(&mut self, token: TagToken) {
        if is_table_select_boundary_tag(token.tag_id) {
            if self.open_elements.has_in_table_scope(token.tag_id) {
                self.pop_open_elements_until_tag_name_popped(TagId::Select);
                self.reset_insertion_mode();
                self.process_end_tag(token);
            }
        } else {
            self.end_tag_in_select(token);
        }
    }

    fn character_in_table(&mut self, token: CharacterToken) {
        if self
            .open_elements
            .current_tag_id
            .is_some_and(is_table_structure_tag)
        {
            self.pending_character_tokens.clear();
            self.has_non_whitespace_pending_character_token = false;
            self.original_insertion_mode = self.insertion_mode;
            self.insertion_mode = InsertionMode::InTableText;

            if token.token_type == TokenType::WhitespaceCharacter {
                self.whitespace_character_in_table_text(token);
            } else if token.token_type != TokenType::NullCharacter {
                self.character_in_table_text(token);
            }
        } else {
            self.token_in_table(Token::Character(token));
        }
    }

    fn whitespace_character_in_table_text(&mut self, token: CharacterToken) {
        self.pending_character_tokens.push(token);
    }

    fn character_in_table_text(&mut self, token: CharacterToken) {
        self.pending_character_tokens.push(token);
        self.has_non_whitespace_pending_character_token = true;
    }

    fn token_in_table_text(&mut self, token: Token) {
        let pending = std::mem::take(&mut self.pending_character_tokens);

        if self.has_non_whitespace_pending_character_token {
            for pending_token in pending {
                self.token_in_table(Token::Character(pending_token));
            }
        } else {
            for pending_token in pending {
                self.insert_characters(&pending_token.chars, pending_token.location);
            }
        }

        self.has_non_whitespace_pending_character_token = false;
        self.insertion_mode = self.original_insertion_mode;
        self.process_token(token);
    }

    fn caption_start_tag_in_table(&mut self, token: TagToken) {
        self.clear_open_elements_back_to_table_context();
        self.active_formatting_elements.insert_marker();
        self.insert_element(token, NS_HTML);
        self.insertion_mode = InsertionMode::InCaption;
    }

    fn colgroup_start_tag_in_table(&mut self, token: TagToken) {
        self.clear_open_elements_back_to_table_context();
        self.insert_element(token, NS_HTML);
        self.insertion_mode = InsertionMode::InColumnGroup;
    }

    fn col_start_tag_in_table(&mut self, token: TagToken) {
        self.clear_open_elements_back_to_table_context();
        self.insert_fake_element("colgroup", TagId::Colgroup);
        self.insertion_mode = InsertionMode::InColumnGroup;
        self.start_tag_in_column_group(token);
    }

    fn tbody_start_tag_in_table(&mut self, token: TagToken) {
        self.clear_open_elements_back_to_table_context();
        self.insert_element(token, NS_HTML);
        self.insertion_mode = InsertionMode::InTableBody;
    }

    fn td_start_tag_in_table(&mut self, token: TagToken) {
        self.clear_open_elements_back_to_table_context();
        self.insert_fake_element("tbody", TagId::Tbody);
        self.insertion_mode = InsertionMode::InTableBody;
        self.start_tag_in_table_body(token);
    }

    fn table_start_tag_in_table(&mut self, token: TagToken) {
        if self.open_elements.has_in_table_scope(TagId::Table) {
            self.pop_open_elements_until_tag_name_popped(TagId::Table);
            self.reset_insertion_mode();
            self.process_start_tag(token, false);
        }
    }

    fn input_start_tag_in_table(&mut self, mut token: TagToken) {
        token.ack_self_closing = true;

        if get_token_attr(&token, "type").is_some_and(|value| value.eq_ignore_ascii_case("hidden"))
        {
            self.append_element(token, NS_HTML);
        } else {
            self.token_in_table(Token::Tag(token));
        }
    }

    fn form_start_tag_in_table(&mut self, token: TagToken) {
        if self.form_element.is_none() && self.open_elements.tmpl_count == 0 {
            let form = self.insert_element(token, NS_HTML);
            self.form_element = Some(form);
            self.pop_open_element();
        }
    }

    fn start_tag_in_table(&mut self, token: TagToken) {
        match token.tag_id {
            TagId::Td | TagId::Th | TagId::Tr => self.td_start_tag_in_table(token),
            TagId::Style | TagId::Script | TagId::Template => self.start_tag_in_head(token),
            TagId::Col => self.col_start_tag_in_table(token),
            TagId::Form => self.form_start_tag_in_table(token),
            TagId::Table => self.table_start_tag_in_table(token),
            TagId::Tbody | TagId::Tfoot | TagId::Thead => self.tbody_start_tag_in_table(token),
            TagId::Input => self.input_start_tag_in_table(token),
            TagId::Caption => self.caption_start_tag_in_table(token),
            TagId::Colgroup => self.colgroup_start_tag_in_table(token),
            _ => self.token_in_table(Token::Tag(token)),
        }
    }

    fn end_tag_in_table(&mut self, token: TagToken) {
        match token.tag_id {
            TagId::Table => {
                if self.open_elements.has_in_table_scope(TagId::Table) {
                    self.update_last_element_end_location(TagId::Table, token.location);
                    self.pop_open_elements_until_tag_name_popped(TagId::Table);
                    self.reset_insertion_mode();
                }
            }
            TagId::Template => self.template_end_tag_in_head(token),
            TagId::Body
            | TagId::Caption
            | TagId::Col
            | TagId::Colgroup
            | TagId::Html
            | TagId::Tbody
            | TagId::Td
            | TagId::Tfoot
            | TagId::Th
            | TagId::Thead
            | TagId::Tr => {}
            _ => self.token_in_table(Token::Tag(token)),
        }
    }

    fn token_in_table(&mut self, token: Token) {
        let saved_foster_parenting_state = self.foster_parenting_enabled;
        self.foster_parenting_enabled = true;
        self.mode_in_body(token);
        self.foster_parenting_enabled = saved_foster_parenting_state;
    }

    fn start_tag_in_caption(&mut self, token: TagToken) {
        if is_table_void_element(token.tag_id) {
            if self.open_elements.has_in_table_scope(TagId::Caption) {
                self.generate_implied_end_tags();
                self.pop_open_elements_until_tag_name_popped(TagId::Caption);
                self.active_formatting_elements.clear_to_last_marker();
                self.insertion_mode = InsertionMode::InTable;
                self.start_tag_in_table(token);
            }
        } else {
            self.start_tag_in_body(token);
        }
    }

    fn end_tag_in_caption(&mut self, token: TagToken) {
        match token.tag_id {
            TagId::Caption | TagId::Table => {
                if self.open_elements.has_in_table_scope(TagId::Caption) {
                    self.update_last_element_end_location(TagId::Caption, token.location.clone());
                    self.generate_implied_end_tags();
                    self.pop_open_elements_until_tag_name_popped(TagId::Caption);
                    self.active_formatting_elements.clear_to_last_marker();
                    self.insertion_mode = InsertionMode::InTable;

                    if token.tag_id == TagId::Table {
                        self.end_tag_in_table(token);
                    }
                }
            }
            TagId::Body
            | TagId::Col
            | TagId::Colgroup
            | TagId::Html
            | TagId::Tbody
            | TagId::Td
            | TagId::Tfoot
            | TagId::Th
            | TagId::Thead
            | TagId::Tr => {}
            _ => self.end_tag_in_body(token),
        }
    }

    fn start_tag_in_column_group(&mut self, token: TagToken) {
        match token.tag_id {
            TagId::Html => self.start_tag_in_body(token),
            TagId::Col => {
                let mut token = token;
                token.ack_self_closing = true;
                self.append_element(token, NS_HTML);
            }
            TagId::Template => self.start_tag_in_head(token),
            _ => self.token_in_column_group(Token::Tag(token)),
        }
    }

    fn end_tag_in_column_group(&mut self, token: TagToken) {
        match token.tag_id {
            TagId::Colgroup => {
                if self.open_elements.current_tag_id == Some(TagId::Colgroup) {
                    self.pop_current_element_for_token(
                        token.location.map(|location| location.location),
                    );
                    self.insertion_mode = InsertionMode::InTable;
                }
            }
            TagId::Template => self.template_end_tag_in_head(token),
            TagId::Col => {}
            _ => self.token_in_column_group(Token::Tag(token)),
        }
    }

    fn token_in_column_group(&mut self, token: Token) {
        if self.open_elements.current_tag_id == Some(TagId::Colgroup) {
            self.pop_open_element();
            self.insertion_mode = InsertionMode::InTable;
            self.process_token(token);
        }
    }

    fn start_tag_in_table_body(&mut self, token: TagToken) {
        match token.tag_id {
            TagId::Tr => {
                self.clear_open_elements_back_to_table_body_context();
                self.insert_element(token, NS_HTML);
                self.insertion_mode = InsertionMode::InRow;
            }
            TagId::Th | TagId::Td => {
                self.clear_open_elements_back_to_table_body_context();
                self.insert_fake_element("tr", TagId::Tr);
                self.insertion_mode = InsertionMode::InRow;
                self.start_tag_in_row(token);
            }
            TagId::Caption
            | TagId::Col
            | TagId::Colgroup
            | TagId::Tbody
            | TagId::Tfoot
            | TagId::Thead => {
                if self.open_elements.has_table_body_context_in_table_scope() {
                    self.clear_open_elements_back_to_table_body_context();
                    self.pop_open_element();
                    self.insertion_mode = InsertionMode::InTable;
                    self.start_tag_in_table(token);
                }
            }
            _ => self.start_tag_in_table(token),
        }
    }

    fn end_tag_in_table_body(&mut self, token: TagToken) {
        match token.tag_id {
            TagId::Tbody | TagId::Tfoot | TagId::Thead => {
                if self.open_elements.has_in_table_scope(token.tag_id) {
                    self.update_last_element_end_location(token.tag_id, token.location);
                    self.clear_open_elements_back_to_table_body_context();
                    self.pop_open_element();
                    self.insertion_mode = InsertionMode::InTable;
                }
            }
            TagId::Table => {
                if self.open_elements.has_table_body_context_in_table_scope() {
                    self.clear_open_elements_back_to_table_body_context();
                    self.pop_open_element();
                    self.insertion_mode = InsertionMode::InTable;
                    self.end_tag_in_table(token);
                }
            }
            TagId::Body
            | TagId::Caption
            | TagId::Col
            | TagId::Colgroup
            | TagId::Html
            | TagId::Td
            | TagId::Th
            | TagId::Tr => {}
            _ => self.end_tag_in_table(token),
        }
    }

    fn start_tag_in_row(&mut self, token: TagToken) {
        match token.tag_id {
            TagId::Th | TagId::Td => {
                self.clear_open_elements_back_to_table_row_context();
                self.insert_element(token, NS_HTML);
                self.insertion_mode = InsertionMode::InCell;
                self.active_formatting_elements.insert_marker();
            }
            TagId::Caption
            | TagId::Col
            | TagId::Colgroup
            | TagId::Tbody
            | TagId::Tfoot
            | TagId::Thead
            | TagId::Tr => {
                if self.open_elements.has_in_table_scope(TagId::Tr) {
                    self.clear_open_elements_back_to_table_row_context();
                    self.pop_open_element();
                    self.insertion_mode = InsertionMode::InTableBody;
                    self.start_tag_in_table_body(token);
                }
            }
            _ => self.start_tag_in_table(token),
        }
    }

    fn end_tag_in_row(&mut self, token: TagToken) {
        match token.tag_id {
            TagId::Tr => {
                if self.open_elements.has_in_table_scope(TagId::Tr) {
                    self.update_last_element_end_location(TagId::Tr, token.location);
                    self.clear_open_elements_back_to_table_row_context();
                    self.pop_open_element();
                    self.insertion_mode = InsertionMode::InTableBody;
                }
            }
            TagId::Table => {
                if self.open_elements.has_in_table_scope(TagId::Tr) {
                    self.clear_open_elements_back_to_table_row_context();
                    self.pop_open_element();
                    self.insertion_mode = InsertionMode::InTableBody;
                    self.end_tag_in_table_body(token);
                }
            }
            TagId::Tbody | TagId::Tfoot | TagId::Thead => {
                if self.open_elements.has_in_table_scope(token.tag_id)
                    || self.open_elements.has_in_table_scope(TagId::Tr)
                {
                    self.clear_open_elements_back_to_table_row_context();
                    self.pop_open_element();
                    self.insertion_mode = InsertionMode::InTableBody;
                    self.end_tag_in_table_body(token);
                }
            }
            TagId::Body
            | TagId::Caption
            | TagId::Col
            | TagId::Colgroup
            | TagId::Html
            | TagId::Td
            | TagId::Th => {}
            _ => self.end_tag_in_table(token),
        }
    }

    fn close_table_cell(&mut self) {
        self.generate_implied_end_tags();
        self.pop_open_elements_until_table_cell_popped();
        self.active_formatting_elements.clear_to_last_marker();
        self.insertion_mode = InsertionMode::InRow;
    }

    fn start_tag_in_cell(&mut self, token: TagToken) {
        if is_table_void_element(token.tag_id) {
            if self.open_elements.has_in_table_scope(TagId::Td)
                || self.open_elements.has_in_table_scope(TagId::Th)
            {
                self.close_table_cell();
                self.start_tag_in_row(token);
            }
        } else {
            self.start_tag_in_body(token);
        }
    }

    fn end_tag_in_cell(&mut self, token: TagToken) {
        match token.tag_id {
            TagId::Td | TagId::Th => {
                if self.open_elements.has_in_table_scope(token.tag_id) {
                    self.update_last_element_end_location(token.tag_id, token.location);
                    self.generate_implied_end_tags();
                    self.pop_open_elements_until_tag_name_popped(token.tag_id);
                    self.active_formatting_elements.clear_to_last_marker();
                    self.insertion_mode = InsertionMode::InRow;
                }
            }
            TagId::Table | TagId::Tbody | TagId::Tfoot | TagId::Thead | TagId::Tr => {
                if self.open_elements.has_in_table_scope(token.tag_id) {
                    self.close_table_cell();
                    self.end_tag_in_row(token);
                }
            }
            TagId::Body | TagId::Caption | TagId::Col | TagId::Colgroup | TagId::Html => {}
            _ => self.end_tag_in_body(token),
        }
    }

    fn area_start_tag_in_body(&mut self, mut token: TagToken) {
        self.reconstruct_active_formatting_elements();
        token.ack_self_closing = true;
        self.append_element(token, NS_HTML);
        self.frameset_ok = false;
    }

    fn input_start_tag_in_body(&mut self, mut token: TagToken) {
        self.reconstruct_active_formatting_elements();
        let is_hidden = get_token_attr(&token, "type")
            .is_some_and(|value| value.eq_ignore_ascii_case("hidden"));
        token.ack_self_closing = true;
        self.append_element(token, NS_HTML);

        if !is_hidden {
            self.frameset_ok = false;
        }
    }

    fn param_start_tag_in_body(&mut self, mut token: TagToken) {
        token.ack_self_closing = true;
        self.append_element(token, NS_HTML);
    }

    fn hr_start_tag_in_body(&mut self, mut token: TagToken) {
        if self.open_elements.has_in_button_scope(TagId::P) {
            self.close_p_element();
        }

        token.ack_self_closing = true;
        self.append_element(token, NS_HTML);
        self.frameset_ok = false;
    }

    fn image_start_tag_in_body(&mut self, mut token: TagToken) {
        token.tag_name = "img".to_string();
        token.tag_id = TagId::Img;
        self.area_start_tag_in_body(token);
    }

    fn rb_start_tag_in_body(&mut self, token: TagToken) {
        if self.open_elements.has_in_scope(TagId::Ruby) {
            self.generate_implied_end_tags();
        }

        self.insert_element(token, NS_HTML);
    }

    fn rt_start_tag_in_body(&mut self, token: TagToken) {
        if self.open_elements.has_in_scope(TagId::Ruby) {
            self.generate_implied_end_tags_with_exclusion(TagId::Rtc);
        }

        self.insert_element(token, NS_HTML);
    }

    fn formatting_start_tag_in_body(&mut self, token: TagToken) {
        self.reconstruct_active_formatting_elements();
        let element = self.insert_element(token.clone(), NS_HTML);
        self.active_formatting_elements.push_element(element, token);
    }

    fn anchor_start_tag_in_body(&mut self, token: TagToken) {
        if let Some(active_element_entry) = self
            .active_formatting_elements
            .get_element_entry_in_scope_with_tag_name("a")
        {
            self.call_adoption_agency(token.clone());

            if let Some(element) = active_element_entry.element_ref() {
                self.remove_open_element(&element);
            }

            self.active_formatting_elements
                .remove_entry(&active_element_entry);
        }

        self.reconstruct_active_formatting_elements();
        let element = self.insert_element(token.clone(), NS_HTML);
        self.active_formatting_elements.push_element(element, token);
    }

    fn generic_start_tag_in_body(&mut self, token: TagToken) {
        self.reconstruct_active_formatting_elements();
        self.insert_element(token, NS_HTML);
    }

    fn switch_to_text_parsing(&mut self, token: TagToken, next_tokenizer_state: TokenizerState) {
        self.insert_element(token, NS_HTML);
        self.tokenizer.state = next_tokenizer_state;
        self.original_insertion_mode = self.insertion_mode;
        self.insertion_mode = InsertionMode::Text;
    }

    fn plaintext_start_tag_in_body(&mut self, token: TagToken) {
        if self.open_elements.has_in_button_scope(TagId::P) {
            self.close_p_element();
        }

        self.insert_element(token, NS_HTML);
        self.tokenizer.state = TokenizerState::PlainText;
    }

    fn textarea_start_tag_in_body(&mut self, token: TagToken) {
        self.insert_element(token, NS_HTML);
        self.skip_next_new_line = true;
        self.tokenizer.state = TokenizerState::RcData;
        self.original_insertion_mode = self.insertion_mode;
        self.frameset_ok = false;
        self.insertion_mode = InsertionMode::Text;
    }

    fn xmp_start_tag_in_body(&mut self, token: TagToken) {
        if self.open_elements.has_in_button_scope(TagId::P) {
            self.close_p_element();
        }

        self.reconstruct_active_formatting_elements();
        self.frameset_ok = false;
        self.switch_to_text_parsing(token, TokenizerState::RawText);
    }

    fn iframe_start_tag_in_body(&mut self, token: TagToken) {
        self.frameset_ok = false;
        self.switch_to_text_parsing(token, TokenizerState::RawText);
    }

    fn raw_text_start_tag_in_body(&mut self, token: TagToken) {
        self.switch_to_text_parsing(token, TokenizerState::RawText);
    }

    fn html_start_tag_in_body(&mut self, token: TagToken) {
        if self.open_elements.tmpl_count == 0 {
            if let Some(html) = self.open_elements.items.first() {
                crate::tree_adapters::default::adopt_attributes(html, token.attrs);
            }
        }
    }

    fn body_start_tag_in_body(&mut self, token: TagToken) {
        if let Some(body) = self.open_elements.try_peek_properly_nested_body_element() {
            if self.open_elements.tmpl_count == 0 {
                self.frameset_ok = false;
                crate::tree_adapters::default::adopt_attributes(&body, token.attrs);
            }
        }
    }

    fn frameset_start_tag_in_body(&mut self, token: TagToken) {
        let Some(body) = self.open_elements.try_peek_properly_nested_body_element() else {
            return;
        };

        if self.frameset_ok {
            detach_node(&body);
            self.pop_open_elements_all_up_to_html_element();
            self.insert_element(token, NS_HTML);
            self.insertion_mode = InsertionMode::InFrameset;
        }
    }

    fn start_tag_in_template(&mut self, token: TagToken) {
        match token.tag_id {
            TagId::Base
            | TagId::Basefont
            | TagId::Bgsound
            | TagId::Link
            | TagId::Meta
            | TagId::Noframes
            | TagId::Script
            | TagId::Style
            | TagId::Template
            | TagId::Title => self.start_tag_in_head(token),
            TagId::Caption | TagId::Colgroup | TagId::Tbody | TagId::Tfoot | TagId::Thead => {
                self.set_current_template_insertion_mode(InsertionMode::InTable);
                self.insertion_mode = InsertionMode::InTable;
                self.start_tag_in_table(token);
            }
            TagId::Col => {
                self.set_current_template_insertion_mode(InsertionMode::InColumnGroup);
                self.insertion_mode = InsertionMode::InColumnGroup;
                self.start_tag_in_column_group(token);
            }
            TagId::Tr => {
                self.set_current_template_insertion_mode(InsertionMode::InTableBody);
                self.insertion_mode = InsertionMode::InTableBody;
                self.start_tag_in_table_body(token);
            }
            TagId::Td | TagId::Th => {
                self.set_current_template_insertion_mode(InsertionMode::InRow);
                self.insertion_mode = InsertionMode::InRow;
                self.start_tag_in_row(token);
            }
            _ => {
                self.set_current_template_insertion_mode(InsertionMode::InBody);
                self.insertion_mode = InsertionMode::InBody;
                self.start_tag_in_body(token);
            }
        }
    }

    fn set_current_template_insertion_mode(&mut self, mode: InsertionMode) {
        if let Some(current_mode) = self.tmpl_insertion_mode_stack.first_mut() {
            *current_mode = mode;
        }
    }

    fn end_tag_in_template(&mut self, token: TagToken) {
        if token.tag_id == TagId::Template {
            self.template_end_tag_in_head(token);
        }
    }

    fn eof_in_template(&mut self, token: Token) {
        if self.open_elements.tmpl_count > 0 {
            self.pop_open_elements_until_tag_name_popped(TagId::Template);
            self.active_formatting_elements.clear_to_last_marker();
            if !self.tmpl_insertion_mode_stack.is_empty() {
                self.tmpl_insertion_mode_stack.remove(0);
            }
            self.reset_insertion_mode();
            self.process_token(token);
        } else {
            self.stop_parsing(token);
        }
    }

    fn eof_in_body(&mut self, token: Token) {
        if !self.tmpl_insertion_mode_stack.is_empty() {
            self.eof_in_template(token);
        } else {
            self.stop_parsing(token);
        }
    }

    fn end_tag_in_body(&mut self, token: TagToken) {
        match token.tag_id {
            TagId::A
            | TagId::B
            | TagId::I
            | TagId::S
            | TagId::U
            | TagId::Em
            | TagId::Tt
            | TagId::Big
            | TagId::Code
            | TagId::Font
            | TagId::Nobr
            | TagId::Small
            | TagId::Strike
            | TagId::Strong => self.call_adoption_agency(token),
            TagId::P => self.p_end_tag_in_body(),
            TagId::Dl
            | TagId::Ul
            | TagId::Ol
            | TagId::Dir
            | TagId::Div
            | TagId::Nav
            | TagId::Pre
            | TagId::Main
            | TagId::Menu
            | TagId::Aside
            | TagId::Button
            | TagId::Center
            | TagId::Figure
            | TagId::Footer
            | TagId::Header
            | TagId::Hgroup
            | TagId::Dialog
            | TagId::Address
            | TagId::Article
            | TagId::Details
            | TagId::Search
            | TagId::Section
            | TagId::Summary
            | TagId::Listing
            | TagId::Fieldset
            | TagId::Blockquote
            | TagId::Figcaption => self.address_end_tag_in_body(token),
            TagId::Li => self.li_end_tag_in_body(token),
            TagId::Dd | TagId::Dt => self.dd_end_tag_in_body(token),
            TagId::H1 | TagId::H2 | TagId::H3 | TagId::H4 | TagId::H5 | TagId::H6 => {
                self.numbered_header_end_tag_in_body(token)
            }
            TagId::Br => self.br_end_tag_in_body(),
            TagId::Template => self.template_end_tag_in_head(token),
            TagId::Body => {
                if self.open_elements.has_in_scope(TagId::Body) {
                    if let Some(body) = self.open_elements.try_peek_properly_nested_body_element() {
                        if let Some(location) = token.location {
                            update_element_end_location(&body, location.location);
                        }
                    }
                    self.insertion_mode = InsertionMode::AfterBody;
                }
            }
            TagId::Html => {
                if self.open_elements.has_in_scope(TagId::Body) {
                    self.insertion_mode = InsertionMode::AfterBody;
                    self.end_tag_after_body(token);
                }
            }
            TagId::Form => self.form_end_tag_in_body(token),
            TagId::Applet | TagId::Object | TagId::Marquee => self.applet_end_tag_in_body(token),
            _ => self.generic_end_tag_in_body(token),
        }
    }

    fn p_end_tag_in_body(&mut self) {
        if !self.open_elements.has_in_button_scope(TagId::P) {
            self.insert_fake_element("p", TagId::P);
        }

        self.close_p_element();
    }

    fn address_end_tag_in_body(&mut self, token: TagToken) {
        if self.open_elements.has_in_scope(token.tag_id) {
            self.update_last_element_end_location(token.tag_id, token.location);
            self.generate_implied_end_tags();
            self.pop_open_elements_until_tag_name_popped(token.tag_id);
        }
    }

    fn form_end_tag_in_body(&mut self, token: TagToken) {
        let in_template = self.open_elements.tmpl_count > 0;
        let form_element = self.form_element.clone();

        if !in_template {
            self.form_element = None;
        }

        if (form_element.is_some() || in_template) && self.open_elements.has_in_scope(TagId::Form) {
            self.update_last_element_end_location(TagId::Form, token.location);
            self.generate_implied_end_tags();

            if in_template {
                self.pop_open_elements_until_tag_name_popped(TagId::Form);
            } else if let Some(form_element) = form_element {
                self.remove_open_element(&form_element);
            }
        }
    }

    fn li_end_tag_in_body(&mut self, token: TagToken) {
        if self.open_elements.has_in_list_item_scope(TagId::Li) {
            self.update_last_element_end_location(TagId::Li, token.location);
            self.generate_implied_end_tags_with_exclusion(TagId::Li);
            self.pop_open_elements_until_tag_name_popped(TagId::Li);
        }
    }

    fn dd_end_tag_in_body(&mut self, token: TagToken) {
        if self.open_elements.has_in_scope(token.tag_id) {
            self.update_last_element_end_location(token.tag_id, token.location);
            self.generate_implied_end_tags_with_exclusion(token.tag_id);
            self.pop_open_elements_until_tag_name_popped(token.tag_id);
        }
    }

    fn numbered_header_end_tag_in_body(&mut self, token: TagToken) {
        if self.open_elements.has_numbered_header_in_scope() {
            if let Some(location) = token.location {
                for idx in (0..=self.open_elements.stack_top.max(0) as usize).rev() {
                    if is_numbered_header(self.open_elements.tag_ids[idx]) {
                        update_element_end_location(
                            &self.open_elements.items[idx],
                            location.location,
                        );
                        break;
                    }
                }
            }

            self.generate_implied_end_tags();
            self.pop_open_elements_until_numbered_header_popped();
        }
    }

    fn applet_end_tag_in_body(&mut self, token: TagToken) {
        if self.open_elements.has_in_scope(token.tag_id) {
            self.update_last_element_end_location(token.tag_id, token.location);
            self.generate_implied_end_tags();
            self.pop_open_elements_until_tag_name_popped(token.tag_id);
            self.active_formatting_elements.clear_to_last_marker();
        }
    }

    fn br_end_tag_in_body(&mut self) {
        self.reconstruct_active_formatting_elements();
        self.insert_fake_element("br", TagId::Br);
        self.pop_open_element();
        self.frameset_ok = false;
    }

    fn generic_end_tag_in_body(&mut self, token: TagToken) {
        if self.open_elements.stack_top <= 0 {
            return;
        }

        for idx in (1..=self.open_elements.stack_top as usize).rev() {
            let element = self.open_elements.items[idx].clone();
            let element_id = self.open_elements.tag_ids[idx];

            if token.tag_id == element_id
                && (token.tag_id != TagId::Unknown
                    || element_tag_name(&element).as_deref() == Some(&token.tag_name))
            {
                if let Some(location) = token.location {
                    update_element_end_location(&element, location.location);
                }

                self.generate_implied_end_tags_with_exclusion(token.tag_id);

                if self.open_elements.stack_top >= idx as isize {
                    self.shorten_open_elements_to_length(idx as isize);
                }

                break;
            }

            if self.is_special_element(&element, element_id) {
                break;
            }
        }
    }

    fn call_adoption_agency(&mut self, token: TagToken) {
        for _ in 0..AA_OUTER_LOOP_ITER {
            let Some(formatting_element_entry) =
                self.aa_obtain_formatting_element_entry(token.clone())
            else {
                break;
            };

            let Some(furthest_block) = self.aa_obtain_furthest_block(&formatting_element_entry)
            else {
                break;
            };

            self.active_formatting_elements
                .set_bookmark(&formatting_element_entry);

            let Some(formatting_element) = formatting_element_entry.element_ref() else {
                break;
            };
            let last_element = self.aa_inner_loop(&furthest_block, &formatting_element);
            let common_ancestor = self.open_elements.get_common_ancestor(&formatting_element);

            detach_node(&last_element);
            if let Some(common_ancestor) = common_ancestor {
                self.aa_insert_last_node_in_common_ancestor(&common_ancestor, last_element);
            }

            self.aa_replace_formatting_element(&furthest_block, &formatting_element_entry);
        }
    }

    fn aa_obtain_formatting_element_entry(&mut self, token: TagToken) -> Option<Entry> {
        let Some(formatting_element_entry) = self
            .active_formatting_elements
            .get_element_entry_in_scope_with_tag_name(&token.tag_name)
        else {
            self.generic_end_tag_in_body(token);
            return None;
        };

        if let Some(element) = formatting_element_entry.element_ref() {
            if !self.open_elements.contains(&element) {
                self.active_formatting_elements
                    .remove_entry(&formatting_element_entry);
                return None;
            } else if !self.open_elements.has_in_scope(token.tag_id) {
                return None;
            }
        }

        Some(formatting_element_entry)
    }

    fn aa_obtain_furthest_block(&mut self, formatting_element_entry: &Entry) -> Option<NodeRef> {
        let formatting_element = formatting_element_entry.element_ref()?;
        let mut furthest_block = None;
        let mut formatting_element_idx = 0isize;

        for idx in (0..=self.open_elements.stack_top.max(0) as usize).rev() {
            let element = self.open_elements.items[idx].clone();

            if std::rc::Rc::ptr_eq(&element, &formatting_element) {
                formatting_element_idx = idx as isize;
                break;
            }

            if self.is_special_element(&element, self.open_elements.tag_ids[idx]) {
                furthest_block = Some(element);
            }
        }

        if furthest_block.is_none() {
            self.shorten_open_elements_to_length(formatting_element_idx.max(0));
            self.active_formatting_elements
                .remove_entry(formatting_element_entry);
        }

        furthest_block
    }

    fn aa_inner_loop(&mut self, furthest_block: &NodeRef, formatting_element: &NodeRef) -> NodeRef {
        let mut last_element = furthest_block.clone();
        let Some(mut element) = self.open_elements.get_common_ancestor(furthest_block) else {
            return last_element;
        };

        for idx in 0.. {
            if std::rc::Rc::ptr_eq(&element, formatting_element) {
                break;
            }

            let next_element = self.open_elements.get_common_ancestor(&element);
            let element_entry = self.active_formatting_elements.get_element_entry(&element);
            let counter_overflow = element_entry.is_some() && idx >= AA_INNER_LOOP_ITER;

            if let Some(element_entry) = element_entry {
                if counter_overflow {
                    self.active_formatting_elements.remove_entry(&element_entry);
                    self.remove_open_element(&element);
                } else {
                    let new_element = self.aa_recreate_element_from_entry(&element_entry);

                    if std::rc::Rc::ptr_eq(&last_element, furthest_block) {
                        self.active_formatting_elements.set_bookmark(&element_entry);
                    }

                    detach_node(&last_element);
                    append_child(&new_element, last_element);
                    last_element = new_element;
                }
            } else {
                self.remove_open_element(&element);
            }

            let Some(next_element) = next_element else {
                break;
            };
            element = next_element;
        }

        last_element
    }

    fn aa_recreate_element_from_entry(&mut self, element_entry: &Entry) -> NodeRef {
        let element = element_entry
            .element_ref()
            .expect("formatting element entry must have an element");
        let token = element_entry
            .token
            .clone()
            .expect("formatting element entry must have a token");
        let namespace_uri = namespace_uri(&element).unwrap_or_else(|| NS_HTML.to_string());
        let new_element = create_element(token.tag_name.clone(), namespace_uri, token.attrs);

        self.open_elements.replace(&element, new_element.clone());
        self.active_formatting_elements
            .replace_entry_element(element_entry, new_element.clone());

        new_element
    }

    fn aa_insert_last_node_in_common_ancestor(
        &mut self,
        common_ancestor: &NodeRef,
        last_element: NodeRef,
    ) {
        let tag_id = element_tag_name(common_ancestor)
            .as_deref()
            .map(get_tag_id)
            .unwrap_or(TagId::Unknown);

        if is_table_structure_tag(tag_id) {
            self.foster_parent_element(last_element);
            return;
        }

        let target = if element_tag_name(common_ancestor).as_deref() == Some(TAG_NAME_TEMPLATE)
            && is_in_namespace(common_ancestor, NS_HTML)
        {
            crate::tree_adapters::default::get_template_content(common_ancestor)
                .unwrap_or_else(|| common_ancestor.clone())
        } else {
            common_ancestor.clone()
        };

        append_child(&target, last_element);
    }

    fn aa_replace_formatting_element(
        &mut self,
        furthest_block: &NodeRef,
        formatting_element_entry: &Entry,
    ) {
        let Some(formatting_element) = formatting_element_entry.element_ref() else {
            return;
        };
        let Some(token) = formatting_element_entry.token.clone() else {
            return;
        };
        let namespace_uri =
            namespace_uri(&formatting_element).unwrap_or_else(|| NS_HTML.to_string());
        let new_element =
            create_element(token.tag_name.clone(), namespace_uri, token.attrs.clone());

        self.adopt_nodes(furthest_block, &new_element);
        append_child(furthest_block, new_element.clone());

        self.active_formatting_elements
            .insert_element_after_bookmark(new_element.clone(), token.clone());
        self.active_formatting_elements
            .remove_entry(formatting_element_entry);

        self.remove_open_element(&formatting_element);
        self.open_elements
            .insert_after(furthest_block, new_element, token.tag_id);
    }

    fn end_tag_in_text(&mut self, token: TagToken) {
        if token.tag_id == TagId::Script && self.script_handler_enabled {
            if let Some(node) = self.open_elements.current.clone() {
                self.script_events.push(ScriptEvent { node });
                self.paused_on_script = true;
            }
        }

        self.pop_current_element_for_token(token.location.map(|location| location.location));
        self.insertion_mode = self.original_insertion_mode;
    }

    fn eof_in_text(&mut self, token: EofToken) {
        self.err_at_location(
            token.location,
            ErrorCode::EofInElementThatCanContainOnlyText,
            false,
        );
        self.pop_open_element();
        self.insertion_mode = self.original_insertion_mode;
        self.on_eof(token);
    }

    fn start_tag_after_body(&mut self, token: TagToken) {
        if token.tag_id == TagId::Html {
            self.start_tag_in_body(token);
        } else {
            self.token_after_body(Token::Tag(token));
        }
    }

    fn end_tag_after_body(&mut self, token: TagToken) {
        if token.tag_id == TagId::Html {
            if self.fragment_context.is_none() {
                self.insertion_mode = InsertionMode::AfterAfterBody;
            }

            let token = Token::Tag(token);

            if let Some(html) = self.open_elements.items.first().cloned() {
                self.set_end_location_for_element_with_token(&html, &token);
            }

            if let Some(body) = self.open_elements.items.get(1).cloned() {
                let has_end_tag = body
                    .borrow()
                    .source_code_location()
                    .and_then(|location| location.end_tag)
                    .is_some();

                if !has_end_tag {
                    self.set_end_location_for_element_with_token(&body, &token);
                }
            }
        } else {
            self.token_after_body(Token::Tag(token));
        }
    }

    fn token_after_body(&mut self, token: Token) {
        self.insertion_mode = InsertionMode::InBody;
        self.mode_in_body(token);
    }

    fn start_tag_in_frameset(&mut self, mut token: TagToken) {
        match token.tag_id {
            TagId::Html => self.start_tag_in_body(token),
            TagId::Frameset => {
                self.insert_element(token, NS_HTML);
            }
            TagId::Frame => {
                token.ack_self_closing = true;
                self.append_element(token, NS_HTML);
            }
            TagId::Noframes => self.start_tag_in_head(token),
            _ => {}
        }
    }

    fn end_tag_in_frameset(&mut self, token: TagToken) {
        if token.tag_id == TagId::Frameset && !self.open_elements.is_root_html_element_current() {
            self.pop_current_element_for_token(token.location.map(|location| location.location));

            if self.fragment_context.is_none()
                && self.open_elements.current_tag_id != Some(TagId::Frameset)
            {
                self.insertion_mode = InsertionMode::AfterFrameset;
            }
        }
    }

    fn start_tag_after_frameset(&mut self, token: TagToken) {
        match token.tag_id {
            TagId::Html => self.start_tag_in_body(token),
            TagId::Noframes => self.start_tag_in_head(token),
            _ => {}
        }
    }

    fn end_tag_after_frameset(&mut self, token: TagToken) {
        if token.tag_id == TagId::Html {
            self.insertion_mode = InsertionMode::AfterAfterFrameset;
        }
    }

    fn start_tag_after_after_body(&mut self, token: TagToken) {
        if token.tag_id == TagId::Html {
            self.start_tag_in_body(token);
        } else {
            self.token_after_after_body(Token::Tag(token));
        }
    }

    fn token_after_after_body(&mut self, token: Token) {
        self.insertion_mode = InsertionMode::InBody;
        self.mode_in_body(token);
    }

    fn start_tag_after_after_frameset(&mut self, token: TagToken) {
        match token.tag_id {
            TagId::Html => self.start_tag_in_body(token),
            TagId::Noframes => self.start_tag_in_head(token),
            _ => {}
        }
    }

    fn stop_parsing(&mut self, token: Token) {
        self.stopped = true;

        if token_location(&token).is_none() {
            return;
        }

        let target = if self.fragment_context.is_some() {
            0
        } else {
            2
        };

        for idx in (target..=self.open_elements.stack_top.max(-1)).rev() {
            if let Some(element) = self.open_elements.items.get(idx as usize).cloned() {
                self.set_end_location_for_element_with_token(&element, &token);
            }
        }

        if self.fragment_context.is_none() && self.open_elements.stack_top >= 0 {
            let Some(html_element) = self.open_elements.items.first().cloned() else {
                return;
            };
            let html_has_end_tag = html_element
                .borrow()
                .source_code_location()
                .and_then(|location| location.end_tag)
                .is_some();

            if !html_has_end_tag {
                self.set_end_location_for_element_with_token(&html_element, &token);

                if let Some(body_element) = self.open_elements.items.get(1).cloned() {
                    let body_has_end_tag = body_element
                        .borrow()
                        .source_code_location()
                        .and_then(|location| location.end_tag)
                        .is_some();

                    if !body_has_end_tag {
                        self.set_end_location_for_element_with_token(&body_element, &token);
                    }
                }
            }
        }
    }

    fn pop_current_element_for_token(&mut self, end_location: Option<Location>) {
        if let (Some(element), Some(end_location)) =
            (self.open_elements.current.clone(), end_location)
        {
            if element.borrow().source_code_location().is_some() {
                update_element_end_location(&element, end_location);
            }
        }

        if let Some(popped) = self.open_elements.pop() {
            self.record_stack_pop(popped);
        }
    }

    fn pop_open_element(&mut self) -> Option<NodeRef> {
        let popped = self.open_elements.pop();

        if let Some(element) = &popped {
            self.set_end_location_for_popped_element(element);
        }

        if let Some(element) = &popped {
            self.record_stack_pop(element.clone());
        }

        popped
    }

    fn shorten_open_elements_to_length(&mut self, idx: isize) {
        while self.open_elements.stack_top >= idx {
            self.pop_open_element();
        }
    }

    fn generate_implied_end_tags(&mut self) {
        while self
            .open_elements
            .current_tag_id
            .is_some_and(is_implicit_end_tag_required)
        {
            self.pop_open_element();
        }
    }

    fn generate_implied_end_tags_thoroughly(&mut self) {
        while self
            .open_elements
            .current_tag_id
            .is_some_and(is_implicit_end_tag_required_thoroughly)
        {
            self.pop_open_element();
        }
    }

    fn generate_implied_end_tags_with_exclusion(&mut self, exclusion_id: TagId) {
        while self
            .open_elements
            .current_tag_id
            .is_some_and(|id| id != exclusion_id && is_implicit_end_tag_required_thoroughly(id))
        {
            self.pop_open_element();
        }
    }

    fn pop_open_elements_until_tag_name_popped(&mut self, tag_id: TagId) {
        let mut target_idx = self.open_elements.stack_top + 1;

        loop {
            let Some(idx) = (0..target_idx.max(0) as usize)
                .rev()
                .find(|&idx| self.open_elements.tag_ids[idx] == tag_id)
            else {
                target_idx = -1;
                break;
            };

            target_idx = idx as isize;

            if target_idx <= 0
                || is_in_namespace(&self.open_elements.items[target_idx as usize], NS_HTML)
            {
                break;
            }
        }

        self.shorten_open_elements_to_length(target_idx.max(0));
    }

    fn pop_open_elements_until_numbered_header_popped(&mut self) {
        self.pop_open_elements_until_popped(is_numbered_header_tag_id);
    }

    fn pop_open_elements_until_table_cell_popped(&mut self) {
        self.pop_open_elements_until_popped(is_table_cell_tag_id);
    }

    fn pop_open_elements_until_popped(&mut self, tag_names: impl Fn(TagId) -> bool) {
        let target_idx = self
            .index_of_open_element_tag_names(tag_names)
            .unwrap_or(-1);
        self.shorten_open_elements_to_length(target_idx.max(0));
    }

    fn clear_open_elements_back_to_table_context(&mut self) {
        self.clear_open_elements_back_to(is_table_context_tag_id);
    }

    fn clear_open_elements_back_to_table_body_context(&mut self) {
        self.clear_open_elements_back_to(is_table_body_context_tag_id);
    }

    fn clear_open_elements_back_to_table_row_context(&mut self) {
        self.clear_open_elements_back_to(is_table_row_context_tag_id);
    }

    fn clear_open_elements_back_to(&mut self, tag_names: impl Fn(TagId) -> bool) {
        let target_idx = self
            .index_of_open_element_tag_names(tag_names)
            .unwrap_or(-1);
        self.shorten_open_elements_to_length(target_idx + 1);
    }

    fn pop_open_elements_all_up_to_html_element(&mut self) {
        self.open_elements.tmpl_count = 0;
        self.shorten_open_elements_to_length(1);
    }

    fn push_open_element(&mut self, element: NodeRef, tag_id: TagId) {
        self.open_elements.push(element.clone(), tag_id);
        self.stack_events.push(StackEvent {
            kind: StackEventKind::Push,
            node: element,
            new_top: self.open_elements.current.clone(),
        });
    }

    fn record_stack_pop(&mut self, node: NodeRef) {
        self.stack_events.push(StackEvent {
            kind: StackEventKind::Pop,
            node,
            new_top: self.open_elements.current.clone(),
        });
    }

    fn remove_open_element(&mut self, element: &NodeRef) {
        self.set_end_location_for_popped_element(element);
        let was_present = self.open_elements.contains(element);
        self.open_elements.remove(element);

        if was_present {
            self.record_stack_pop(element.clone());
        }
    }

    fn index_of_open_element_tag_names(&self, tag_names: impl Fn(TagId) -> bool) -> Option<isize> {
        if self.open_elements.stack_top < 0 {
            return None;
        }

        (0..=self.open_elements.stack_top as usize)
            .rev()
            .find(|&idx| {
                tag_names(self.open_elements.tag_ids[idx])
                    && is_in_namespace(&self.open_elements.items[idx], NS_HTML)
            })
            .map(|idx| idx as isize)
    }

    fn set_end_location_for_popped_element(&mut self, element: &NodeRef) {
        if !self.options.source_code_location_info {
            return;
        }

        let Some(current_token) = self.current_token.take() else {
            return;
        };

        self.set_end_location_for_element_with_token(element, &current_token);
        self.current_token = Some(current_token);
    }

    fn set_end_location_for_element_with_token(&mut self, element: &NodeRef, token: &Token) {
        if !self.options.source_code_location_info
            || element.borrow().source_code_location().is_none()
        {
            return;
        }

        let Some(closing_location) = token_location(token) else {
            return;
        };

        let is_matching_end_tag = match token {
            Token::Tag(token) if token.token_type == TokenType::EndTag => {
                element_tag_name(element).as_deref() == Some(token.tag_name.as_str())
            }
            _ => false,
        };

        if is_matching_end_tag {
            update_element_end_location(element, closing_location);
        } else {
            update_element_implicit_end_location(element, closing_location);
        }
    }

    fn update_last_element_end_location(
        &mut self,
        tag_id: TagId,
        end_tag_location: Option<LocationWithAttributes>,
    ) {
        let Some(end_tag_location) = end_tag_location else {
            return;
        };

        for idx in (0..=self.open_elements.stack_top.max(0) as usize).rev() {
            if self.open_elements.tag_ids[idx] == tag_id {
                update_element_end_location(
                    &self.open_elements.items[idx],
                    end_tag_location.location,
                );
                break;
            }
        }
    }

    fn math_start_tag_in_body(&mut self, mut token: TagToken) {
        self.reconstruct_active_formatting_elements();
        foreign_content::adjust_token_mathml_attrs(&mut token);
        foreign_content::adjust_token_xml_attrs(&mut token);

        if token.self_closing {
            self.append_element(token, NS_MATHML);
        } else {
            self.insert_element(token, NS_MATHML);
        }
    }

    fn svg_start_tag_in_body(&mut self, mut token: TagToken) {
        self.reconstruct_active_formatting_elements();
        foreign_content::adjust_token_svg_attrs(&mut token);
        foreign_content::adjust_token_xml_attrs(&mut token);

        if token.self_closing {
            self.append_element(token, NS_SVG);
        } else {
            self.insert_element(token, NS_SVG);
        }
    }

    fn should_process_start_tag_token_in_foreign_content(&self, token: &TagToken) -> bool {
        if !self.current_not_in_html() {
            return false;
        }

        let (current, current_tag_id) = self.current_node_and_tag_id_for_context();

        if token.tag_id == TagId::Svg
            && element_tag_name(&current).as_deref() == Some("annotation-xml")
            && is_in_namespace(&current, NS_MATHML)
        {
            return false;
        }

        let tokenizer_in_foreign_node = !self.is_integration_point(current_tag_id, &current, None);

        tokenizer_in_foreign_node
            || (matches!(token.tag_id, TagId::Mglyph | TagId::Malignmark)
                && !self.is_integration_point(current_tag_id, &current, Some(NS_HTML)))
    }

    fn start_tag_in_foreign_content(&mut self, mut token: TagToken) {
        if foreign_content::causes_exit(&token) {
            self.pop_until_html_or_integration_point();
            self.start_tag_outside_foreign_content(token);
            return;
        }

        let current = self.get_adjusted_current_element();
        let current_ns = namespace_uri(&current).unwrap_or_else(|| NS_HTML.to_string());

        if current_ns == NS_MATHML {
            foreign_content::adjust_token_mathml_attrs(&mut token);
        } else if current_ns == NS_SVG {
            foreign_content::adjust_token_svg_tag_name(&mut token);
            foreign_content::adjust_token_svg_attrs(&mut token);
        }

        foreign_content::adjust_token_xml_attrs(&mut token);

        if token.self_closing {
            self.append_element(token, &current_ns);
        } else {
            self.insert_element(token, &current_ns);
        }
    }

    fn end_tag_in_foreign_content(&mut self, mut token: TagToken) {
        if matches!(token.tag_id, TagId::P | TagId::Br) {
            self.pop_until_html_or_integration_point();
            self.end_tag_outside_foreign_content(token);
            return;
        }

        for idx in (1..=self.open_elements.stack_top.max(0) as usize).rev() {
            let element = self.open_elements.items[idx].clone();

            if is_in_namespace(&element, NS_HTML) {
                self.end_tag_outside_foreign_content(token);
                break;
            }

            let tag_name = element_tag_name(&element).unwrap_or_default();

            if tag_name.to_ascii_lowercase() == token.tag_name {
                token.tag_name = tag_name;
                self.current_token = Some(Token::Tag(token));
                self.shorten_open_elements_to_length(idx as isize);
                break;
            }
        }
    }

    fn pop_until_html_or_integration_point(&mut self) {
        while let Some(current) = self.open_elements.current.clone() {
            let current_tag_id = self.open_elements.current_tag_id.unwrap_or(TagId::Unknown);

            if is_in_namespace(&current, NS_HTML)
                || self.is_integration_point(current_tag_id, &current, None)
            {
                break;
            }

            self.pop_open_element();
        }
    }

    fn is_integration_point(
        &self,
        tag_id: TagId,
        element: &NodeRef,
        foreign_namespace: Option<&str>,
    ) -> bool {
        let element = element.borrow();

        foreign_content::is_integration_point(
            tag_id,
            element.namespace_uri().unwrap_or(NS_HTML),
            element.attrs().unwrap_or_default(),
            foreign_namespace,
        )
    }

    fn current_not_in_html(&self) -> bool {
        let current = self.current_node_and_tag_id_for_context().0;
        !std::rc::Rc::ptr_eq(&current, &self.document) && !is_in_namespace(&current, NS_HTML)
    }

    fn document_mode(&self) -> DocumentMode {
        match &self.document.borrow().data {
            NodeData::Document { mode, .. } => *mode,
            _ => DocumentMode::NoQuirks,
        }
    }

    fn should_foster_parent_on_insertion(&self) -> bool {
        self.foster_parenting_enabled
            && self
                .open_elements
                .current_tag_id
                .is_some_and(is_table_structure_tag)
    }

    fn find_foster_parenting_location(&self) -> (NodeRef, Option<NodeRef>) {
        for idx in (0..=self.open_elements.stack_top.max(0) as usize).rev() {
            let open_element = self.open_elements.items[idx].clone();

            match self.open_elements.tag_ids[idx] {
                TagId::Template if is_in_namespace(&open_element, NS_HTML) => {
                    if let Some(content) =
                        crate::tree_adapters::default::get_template_content(&open_element)
                    {
                        return (content, None);
                    }
                }
                TagId::Table => {
                    let parent = open_element.borrow().parent();

                    if let Some(parent) = parent {
                        return (parent, Some(open_element));
                    }

                    let parent = idx
                        .checked_sub(1)
                        .and_then(|parent_idx| self.open_elements.items.get(parent_idx))
                        .cloned()
                        .unwrap_or_else(|| self.document.clone());
                    return (parent, None);
                }
                _ => {}
            }
        }

        (
            self.open_elements
                .items
                .first()
                .cloned()
                .unwrap_or_else(|| self.document.clone()),
            None,
        )
    }

    fn foster_parent_element(&mut self, element: NodeRef) {
        let (parent, before_element) = self.find_foster_parenting_location();

        if let Some(before_element) = before_element {
            crate::tree_adapters::default::insert_before(&parent, element, &before_element);
        } else {
            append_child(&parent, element);
        }
    }

    fn is_special_element(&self, element: &NodeRef, tag_id: TagId) -> bool {
        is_special_element(element.borrow().namespace_uri().unwrap_or(NS_HTML), tag_id)
    }

    fn tokenizer_in_foreign_node(&self) -> bool {
        let (current, current_tag_id) = self.current_node_and_tag_id_for_context();

        !is_in_namespace(&current, NS_HTML)
            && !std::rc::Rc::ptr_eq(&current, &self.document)
            && !self.is_integration_point(current_tag_id, &current, None)
    }

    fn current_node_and_tag_id_for_context(&self) -> (NodeRef, TagId) {
        if self.open_elements.stack_top == 0 {
            if let Some(fragment_context) = &self.fragment_context {
                return (fragment_context.clone(), self.fragment_context_id);
            }
        }

        (
            self.open_elements
                .current
                .clone()
                .unwrap_or_else(|| self.document.clone()),
            self.open_elements.current_tag_id.unwrap_or(TagId::Unknown),
        )
    }

    fn get_adjusted_current_element(&self) -> NodeRef {
        if self.open_elements.stack_top == 0 {
            if let Some(fragment_context) = &self.fragment_context {
                return fragment_context.clone();
            }
        }

        self.open_elements
            .current
            .clone()
            .unwrap_or_else(|| self.document.clone())
    }

    fn append_element(&mut self, token: TagToken, namespace_uri: &str) -> NodeRef {
        let element = create_element(&token.tag_name, namespace_uri, token.attrs);
        self.attach_element_to_tree(&element, token.location);
        element
    }

    fn insert_element(&mut self, token: TagToken, namespace_uri: &str) -> NodeRef {
        let element = create_element(&token.tag_name, namespace_uri, token.attrs);
        let tag_id = token.tag_id;

        self.attach_element_to_tree(&element, token.location);
        self.push_open_element(element.clone(), tag_id);
        element
    }

    fn insert_template(&mut self, token: TagToken) {
        let template = create_element(&token.tag_name, NS_HTML, token.attrs);
        let content = create_document_fragment();
        if self.options.source_code_location_info {
            content.borrow_mut().set_source_code_location(None);
        }
        set_template_content(&template, content);
        self.attach_element_to_tree(&template, token.location);
        self.push_open_element(template, token.tag_id);
    }

    fn insert_fake_root_element(&mut self) {
        let element = create_element(TAG_NAME_HTML, NS_HTML, Vec::new());
        if self.options.source_code_location_info {
            element.borrow_mut().set_source_code_location(None);
        }

        append_child(
            self.open_elements
                .current
                .as_ref()
                .expect("parser document is current before fake root insertion"),
            element.clone(),
        );
        self.push_open_element(element, TagId::Html);
    }

    fn insert_fake_element(&mut self, tag_name: &str, tag_id: TagId) {
        let element = create_element(tag_name, NS_HTML, Vec::new());
        self.attach_element_to_tree(&element, None);
        self.push_open_element(element, tag_id);
    }

    fn attach_element_to_tree(
        &mut self,
        element: &NodeRef,
        location: Option<LocationWithAttributes>,
    ) {
        if self.options.source_code_location_info {
            let loc = location.map(element_location_from_start_tag);
            element.borrow_mut().set_source_code_location(loc);
        }

        if self.should_foster_parent_on_insertion() {
            self.foster_parent_element(element.clone());
        } else {
            let parent = self
                .open_elements
                .current_tmpl_content_or_node()
                .unwrap_or_else(|| self.document.clone());
            append_child(&parent, element.clone());
        }
    }

    fn append_comment_node(&mut self, data: String, location: Option<Location>, parent: &NodeRef) {
        let comment = create_comment_node(data);
        append_child(parent, comment.clone());

        if self.options.source_code_location_info {
            if let Some(location) = location {
                comment
                    .borrow_mut()
                    .set_source_code_location(Some(ElementLocation::from(location)));
            }
        }
    }

    fn insert_characters(&mut self, chars: &str, location: Option<Location>) {
        if self.should_foster_parent_on_insertion() {
            let (parent, before_element) = self.find_foster_parenting_location();

            if let Some(before_element) = before_element {
                append_text_before(&parent, chars, location, &before_element);
            } else {
                append_text(&parent, chars, location);
            }
        } else {
            let parent = self
                .open_elements
                .current_tmpl_content_or_node()
                .unwrap_or_else(|| self.document.clone());
            append_text(&parent, chars, location);
        }
    }

    fn adopt_nodes(&mut self, donor: &NodeRef, recipient: &NodeRef) {
        while let Some(child) = get_first_child(donor) {
            detach_node(&child);
            append_child(recipient, child);
        }
    }
}

pub fn parse(html: &str, options: ParserOptions) -> NodeRef {
    Parser::parse(html, options)
}

pub fn parse_with_errors(html: &str, options: ParserOptions) -> ParserOutput {
    Parser::parse_with_errors(html, options)
}

pub fn parse_fragment(html: &str, options: ParserOptions) -> NodeRef {
    parse_fragment_with_context(None, html, options)
}

pub fn parse_fragment_with_context(
    fragment_context: Option<NodeRef>,
    html: &str,
    options: ParserOptions,
) -> NodeRef {
    let mut parser = Parser::get_fragment_parser(fragment_context, options);
    parser.write(html);
    parser.get_fragment()
}

pub fn parse_fragment_with_errors(html: &str, options: ParserOptions) -> ParserOutput {
    parse_fragment_with_context_and_errors(None, html, options)
}

pub fn parse_fragment_with_context_and_errors(
    fragment_context: Option<NodeRef>,
    html: &str,
    mut options: ParserOptions,
) -> ParserOutput {
    options.source_code_location_info = true;
    let mut parser = Parser::get_fragment_parser(fragment_context, options);
    parser.write(html);
    let document = parser.get_fragment();
    ParserOutput {
        document,
        errors: parser.errors,
    }
}

fn element_location_from_start_tag(start_tag: LocationWithAttributes) -> ElementLocation {
    ElementLocation {
        location: start_tag.location,
        attrs: start_tag.attrs,
        start_tag: Some(start_tag.location),
        end_tag: None,
    }
}

fn append_text(parent: &NodeRef, chars: &str, location: Option<Location>) {
    let last_child = parent
        .borrow()
        .child_nodes()
        .and_then(|children| children.last().cloned());

    if let Some(last_child) = last_child {
        let mut last_ref = last_child.borrow_mut();

        if let NodeData::Text { value } = &mut last_ref.data {
            value.push_str(chars);

            if let Some(location) = location {
                let mut source_location = last_ref
                    .source_code_location()
                    .unwrap_or_else(|| ElementLocation::from(location));
                extend_location_end(&mut source_location.location, location);
                last_ref.set_source_code_location(Some(source_location));
            }

            return;
        }
    }

    let text = create_text_node(chars);
    if let Some(location) = location {
        text.borrow_mut()
            .set_source_code_location(Some(ElementLocation::from(location)));
    }
    append_child(parent, text);
}

fn append_text_before(
    parent: &NodeRef,
    chars: &str,
    location: Option<Location>,
    reference_node: &NodeRef,
) {
    let maybe_prev = {
        let parent_ref = parent.borrow();
        parent_ref.child_nodes().and_then(|children| {
            children
                .iter()
                .position(|node| std::rc::Rc::ptr_eq(node, reference_node))
                .and_then(|idx| idx.checked_sub(1))
                .and_then(|idx| children.get(idx).cloned())
        })
    };

    if let Some(prev) = maybe_prev {
        let mut prev_ref = prev.borrow_mut();

        if let NodeData::Text { value } = &mut prev_ref.data {
            value.push_str(chars);

            if let Some(location) = location {
                let mut source_location = prev_ref
                    .source_code_location()
                    .unwrap_or_else(|| ElementLocation::from(location));
                extend_location_end(&mut source_location.location, location);
                prev_ref.set_source_code_location(Some(source_location));
            }

            return;
        }
    }

    let text = create_text_node(chars);
    if let Some(location) = location {
        text.borrow_mut()
            .set_source_code_location(Some(ElementLocation::from(location)));
    }
    crate::tree_adapters::default::insert_before(parent, text, reference_node);
}

fn update_element_end_location(element: &NodeRef, end_tag: Location) {
    let mut element_ref = element.borrow_mut();
    let mut location = element_ref
        .source_code_location()
        .unwrap_or_else(|| ElementLocation::from(end_tag));

    extend_location_end(&mut location.location, end_tag);
    location.end_tag = Some(end_tag);
    element_ref.set_source_code_location(Some(location));
}

fn update_element_implicit_end_location(element: &NodeRef, next_token_location: Location) {
    let mut element_ref = element.borrow_mut();
    let mut location = element_ref
        .source_code_location()
        .unwrap_or_else(|| ElementLocation::from(next_token_location));

    location.location.end_line = next_token_location.start_line;
    location.location.end_col = next_token_location.start_col;
    location.location.end_offset = next_token_location.start_offset;
    element_ref.set_source_code_location(Some(location));
}

fn extend_location_end(location: &mut Location, end: Location) {
    location.end_line = end.end_line;
    location.end_col = end.end_col;
    location.end_offset = end.end_offset;
}

fn token_start_location(token: &Token) -> Option<Location> {
    token_location(token).map(|location| Location {
        start_line: location.start_line,
        start_col: location.start_col,
        start_offset: location.start_offset,
        end_line: location.start_line,
        end_col: location.start_col,
        end_offset: location.start_offset,
    })
}

fn token_location(token: &Token) -> Option<Location> {
    match token {
        Token::Doctype(token) => token.location,
        Token::Tag(token) => token.location.as_ref().map(|location| location.location),
        Token::Comment(token) => token.location,
        Token::Eof(token) => token.location,
        Token::Character(token) => token.location,
    }
}

fn element_tag_name(element: &NodeRef) -> Option<String> {
    match &element.borrow().data {
        NodeData::Element { tag_name, .. } => Some(tag_name.clone()),
        _ => None,
    }
}

fn namespace_uri(element: &NodeRef) -> Option<String> {
    match &element.borrow().data {
        NodeData::Element { namespace_uri, .. } => Some(namespace_uri.clone()),
        _ => None,
    }
}

fn is_table_structure_tag(tag_id: TagId) -> bool {
    matches!(
        tag_id,
        TagId::Table | TagId::Tbody | TagId::Tfoot | TagId::Thead | TagId::Tr
    )
}

fn is_numbered_header_tag_id(tag_id: TagId) -> bool {
    matches!(
        tag_id,
        TagId::H1 | TagId::H2 | TagId::H3 | TagId::H4 | TagId::H5 | TagId::H6
    )
}

fn is_table_cell_tag_id(tag_id: TagId) -> bool {
    matches!(tag_id, TagId::Td | TagId::Th)
}

fn is_table_context_tag_id(tag_id: TagId) -> bool {
    matches!(tag_id, TagId::Table | TagId::Template | TagId::Html)
}

fn is_table_body_context_tag_id(tag_id: TagId) -> bool {
    matches!(
        tag_id,
        TagId::Tbody | TagId::Tfoot | TagId::Thead | TagId::Template | TagId::Html
    )
}

fn is_table_row_context_tag_id(tag_id: TagId) -> bool {
    matches!(tag_id, TagId::Tr | TagId::Template | TagId::Html)
}

fn is_implicit_end_tag_required(tag_id: TagId) -> bool {
    matches!(
        tag_id,
        TagId::Dd
            | TagId::Dt
            | TagId::Li
            | TagId::Optgroup
            | TagId::Option
            | TagId::P
            | TagId::Rb
            | TagId::Rp
            | TagId::Rt
            | TagId::Rtc
    )
}

fn is_implicit_end_tag_required_thoroughly(tag_id: TagId) -> bool {
    is_implicit_end_tag_required(tag_id)
        || matches!(
            tag_id,
            TagId::Caption
                | TagId::Colgroup
                | TagId::Tbody
                | TagId::Td
                | TagId::Tfoot
                | TagId::Th
                | TagId::Thead
                | TagId::Tr
        )
}

fn is_table_void_element(tag_id: TagId) -> bool {
    matches!(
        tag_id,
        TagId::Caption
            | TagId::Col
            | TagId::Colgroup
            | TagId::Tbody
            | TagId::Td
            | TagId::Tfoot
            | TagId::Th
            | TagId::Thead
            | TagId::Tr
    )
}

fn is_table_select_boundary_tag(tag_id: TagId) -> bool {
    matches!(
        tag_id,
        TagId::Caption
            | TagId::Table
            | TagId::Tbody
            | TagId::Tfoot
            | TagId::Thead
            | TagId::Tr
            | TagId::Td
            | TagId::Th
    )
}

#[cfg(test)]
fn child_at(parent: &NodeRef, idx: usize) -> NodeRef {
    parent.borrow().child_nodes().unwrap()[idx].clone()
}

#[cfg(test)]
fn child_count(parent: &NodeRef) -> usize {
    parent.borrow().child_nodes().unwrap().len()
}

#[cfg(test)]
fn text_value(node: &NodeRef) -> String {
    match &node.borrow().data {
        NodeData::Text { value } => value.clone(),
        _ => panic!("expected text node"),
    }
}

#[cfg(test)]
fn comment_data(node: &NodeRef) -> String {
    match &node.borrow().data {
        NodeData::Comment { data } => data.clone(),
        _ => panic!("expected comment node"),
    }
}

#[cfg(test)]
fn element_attr(node: &NodeRef, name: &str) -> Option<crate::common::token::Attribute> {
    match &node.borrow().data {
        NodeData::Element { attrs, .. } => attrs.iter().find(|attr| attr.name == name).cloned(),
        _ => panic!("expected element node"),
    }
}

#[cfg(test)]
fn element_name(node: &NodeRef) -> String {
    element_tag_name(node).expect("expected element node")
}

#[cfg(test)]
fn element_namespace(node: &NodeRef) -> String {
    namespace_uri(node).expect("expected element node")
}

#[cfg(test)]
fn is_parent_of(parent: &NodeRef, child: &NodeRef) -> bool {
    child
        .borrow()
        .parent()
        .as_ref()
        .is_some_and(|actual| std::rc::Rc::ptr_eq(actual, parent))
}

#[cfg(test)]
mod tests {
    use super::{
        child_at, child_count, comment_data, element_attr, element_name, element_namespace,
        is_parent_of, parse, parse_fragment, parse_fragment_with_context, parse_with_errors,
        text_value, InsertionMode, Parser, ParserOptions,
    };
    use crate::common::error_codes::ErrorCode;
    use crate::common::html::{DocumentMode, NS_HTML, NS_MATHML, NS_SVG, NS_XLINK};
    use crate::serializer::{serialize, SerializerOptions};
    use crate::tree_adapters::default::{
        append_child, create_element, get_document_mode, get_template_content,
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
            "iframe", "noembed", "noframes", "noscript", "script", "style", "textarea", "title",
            "xmp",
        ] {
            let html =
                format!("<r><{tag_name}><math id=\"</{tag_name}><b>should be outside</b>\">");
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
        let textarea_fragment = parse_fragment_with_context(
            Some(textarea_context),
            "<b>x</b>",
            ParserOptions::default(),
        );
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
}
