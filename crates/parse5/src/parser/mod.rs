pub(in crate::parser) use crate::common::doctype;
pub(in crate::parser) use crate::common::error_codes::{ErrorCode, ParserError};
pub(in crate::parser) use crate::common::foreign_content;
pub(in crate::parser) use crate::common::html::{
    get_tag_id, is_numbered_header, is_special_element, is_void_element, DocumentMode, TagId,
    NS_HTML, NS_MATHML, NS_SVG,
};
pub(in crate::parser) use crate::common::token::{
    get_token_attr, CharacterToken, CommentToken, DoctypeToken, ElementLocation, EofToken,
    Location, LocationWithAttributes, TagToken, Token, TokenType,
};
pub(in crate::parser) use crate::parser::formatting_element_list::{Entry, FormattingElementList};
pub(in crate::parser) use crate::parser::open_element_stack::OpenElementStack;
pub(in crate::parser) use crate::tokenizer::{
    State as TokenizerState, Tokenizer, TokenizerOptions,
};
pub(in crate::parser) use crate::tree_adapters::default::{
    append_child, create_comment_node, create_document, create_document_fragment, create_element,
    detach_node, get_first_child, is_in_namespace, set_document_mode, set_document_type,
    set_template_content, NodeData, NodeRef,
};

#[path = "adoption_agency.rs"]
mod adoption_agency;
#[path = "foreign.rs"]
mod foreign;
#[path = "foster_parenting.rs"]
mod foster_parenting;
#[path = "modes/frameset.rs"]
mod frameset;
#[path = "modes/in_body.rs"]
mod in_body;
#[path = "modes/in_select.rs"]
mod in_select;
#[path = "modes/in_table.rs"]
mod in_table;
#[path = "modes/in_template.rs"]
mod in_template;
#[path = "modes/initial.rs"]
mod initial;
#[path = "location.rs"]
mod location;
#[path = "stack.rs"]
mod stack;
pub(in crate::parser) use location::*;

pub mod formatting_element_list;
pub mod open_element_stack;

pub(in crate::parser) const TAG_NAME_TEMPLATE: &str = "template";
pub(in crate::parser) const TAG_NAME_DOCUMENT_MOCK: &str = "documentmock";
pub(in crate::parser) const TAG_NAME_HTML: &str = "html";
pub(in crate::parser) const AA_OUTER_LOOP_ITER: usize = 8;
pub(in crate::parser) const AA_INNER_LOOP_ITER: usize = 3;

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

    pub fn get_fragment(&mut self) -> NodeRef {
        let fragment = create_document_fragment();

        if let Some(root_element) = get_first_child(&self.document) {
            self.adopt_nodes(&root_element, &fragment);
        }

        fragment
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

    fn document_mode(&self) -> DocumentMode {
        match &self.document.borrow().data {
            NodeData::Document { mode, .. } => *mode,
            _ => DocumentMode::NoQuirks,
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

#[cfg(test)]
mod tests;
