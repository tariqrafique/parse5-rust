use super::Tokenizer;
use super::*;

impl Tokenizer {
    pub(in crate::tokenizer) fn state_script_data(&mut self, code_point: i32) {
        match code_point {
            cp::LESS_THAN_SIGN => self.state = State::ScriptDataLessThanSign,
            cp::NULL => {
                self.err(ErrorCode::UnexpectedNullCharacter);
                self.emit_chars(REPLACEMENT_CHARACTER_STR);
            }
            cp::EOF => self.emit_eof_token(),
            _ => self.emit_code_point(code_point),
        }
    }

    pub(in crate::tokenizer) fn state_script_data_less_than_sign(&mut self, code_point: i32) {
        match code_point {
            cp::SOLIDUS => self.state = State::ScriptDataEndTagOpen,
            cp::EXCLAMATION_MARK => {
                self.state = State::ScriptDataEscapeStart;
                self.emit_chars("<!");
            }
            _ => {
                self.emit_chars("<");
                self.state = State::ScriptData;
                self.state_script_data(code_point);
            }
        }
    }

    pub(in crate::tokenizer) fn state_script_data_end_tag_open(&mut self, code_point: i32) {
        if is_ascii_letter(code_point) {
            self.state = State::ScriptDataEndTagName;
            self.state_script_data_end_tag_name(code_point);
        } else {
            self.emit_chars("</");
            self.state = State::ScriptData;
            self.state_script_data(code_point);
        }
    }

    pub(in crate::tokenizer) fn state_script_data_end_tag_name(&mut self, code_point: i32) {
        if self.handle_special_end_tag(State::ScriptData) {
            self.emit_chars("</");
            self.state = State::ScriptData;
            self.state_script_data(code_point);
        }
    }

    pub(in crate::tokenizer) fn state_script_data_escape_start(&mut self, code_point: i32) {
        if code_point == cp::HYPHEN_MINUS {
            self.state = State::ScriptDataEscapeStartDash;
            self.emit_chars("-");
        } else {
            self.state = State::ScriptData;
            self.state_script_data(code_point);
        }
    }

    pub(in crate::tokenizer) fn state_script_data_escape_start_dash(&mut self, code_point: i32) {
        if code_point == cp::HYPHEN_MINUS {
            self.state = State::ScriptDataEscapedDashDash;
            self.emit_chars("-");
        } else {
            self.state = State::ScriptData;
            self.state_script_data(code_point);
        }
    }

    pub(in crate::tokenizer) fn state_script_data_escaped(&mut self, code_point: i32) {
        match code_point {
            cp::HYPHEN_MINUS => {
                self.state = State::ScriptDataEscapedDash;
                self.emit_chars("-");
            }
            cp::LESS_THAN_SIGN => self.state = State::ScriptDataEscapedLessThanSign,
            cp::NULL => {
                self.err(ErrorCode::UnexpectedNullCharacter);
                self.emit_chars(REPLACEMENT_CHARACTER_STR);
            }
            cp::EOF => {
                self.err(ErrorCode::EofInScriptHtmlCommentLikeText);
                self.emit_eof_token();
            }
            _ => self.emit_code_point(code_point),
        }
    }

    pub(in crate::tokenizer) fn state_script_data_escaped_dash(&mut self, code_point: i32) {
        match code_point {
            cp::HYPHEN_MINUS => {
                self.state = State::ScriptDataEscapedDashDash;
                self.emit_chars("-");
            }
            cp::LESS_THAN_SIGN => self.state = State::ScriptDataEscapedLessThanSign,
            cp::NULL => {
                self.err(ErrorCode::UnexpectedNullCharacter);
                self.state = State::ScriptDataEscaped;
                self.emit_chars(REPLACEMENT_CHARACTER_STR);
            }
            cp::EOF => {
                self.err(ErrorCode::EofInScriptHtmlCommentLikeText);
                self.emit_eof_token();
            }
            _ => {
                self.state = State::ScriptDataEscaped;
                self.emit_code_point(code_point);
            }
        }
    }

    pub(in crate::tokenizer) fn state_script_data_escaped_dash_dash(&mut self, code_point: i32) {
        match code_point {
            cp::HYPHEN_MINUS => self.emit_chars("-"),
            cp::LESS_THAN_SIGN => self.state = State::ScriptDataEscapedLessThanSign,
            cp::GREATER_THAN_SIGN => {
                self.state = State::ScriptData;
                self.emit_chars(">");
            }
            cp::NULL => {
                self.err(ErrorCode::UnexpectedNullCharacter);
                self.state = State::ScriptDataEscaped;
                self.emit_chars(REPLACEMENT_CHARACTER_STR);
            }
            cp::EOF => {
                self.err(ErrorCode::EofInScriptHtmlCommentLikeText);
                self.emit_eof_token();
            }
            _ => {
                self.state = State::ScriptDataEscaped;
                self.emit_code_point(code_point);
            }
        }
    }

    pub(in crate::tokenizer) fn state_script_data_escaped_less_than_sign(
        &mut self,
        code_point: i32,
    ) {
        if code_point == cp::SOLIDUS {
            self.state = State::ScriptDataEscapedEndTagOpen;
        } else if is_ascii_letter(code_point) {
            self.emit_chars("<");
            self.state = State::ScriptDataDoubleEscapeStart;
            self.state_script_data_double_escape_start(code_point);
        } else {
            self.emit_chars("<");
            self.state = State::ScriptDataEscaped;
            self.state_script_data_escaped(code_point);
        }
    }

    pub(in crate::tokenizer) fn state_script_data_escaped_end_tag_open(&mut self, code_point: i32) {
        if is_ascii_letter(code_point) {
            self.state = State::ScriptDataEscapedEndTagName;
            self.state_script_data_escaped_end_tag_name(code_point);
        } else {
            self.emit_chars("</");
            self.state = State::ScriptDataEscaped;
            self.state_script_data_escaped(code_point);
        }
    }

    pub(in crate::tokenizer) fn state_script_data_escaped_end_tag_name(&mut self, code_point: i32) {
        if self.handle_special_end_tag(State::ScriptDataEscaped) {
            self.emit_chars("</");
            self.state = State::ScriptDataEscaped;
            self.state_script_data_escaped(code_point);
        }
    }

    pub(in crate::tokenizer) fn state_script_data_double_escape_start(&mut self, code_point: i32) {
        if self.preprocessor.starts_with(seq::SCRIPT, false)
            && is_script_data_double_escape_sequence_end(
                self.preprocessor.peek(seq::SCRIPT.len() as isize),
            )
        {
            self.emit_code_point(code_point);

            for _ in 0..seq::SCRIPT.len() {
                let code_point = self.consume();
                self.emit_code_point(code_point);
            }

            self.state = State::ScriptDataDoubleEscaped;
        } else if !self.ensure_hibernation() {
            self.state = State::ScriptDataEscaped;
            self.state_script_data_escaped(code_point);
        }
    }

    pub(in crate::tokenizer) fn state_script_data_double_escaped(&mut self, code_point: i32) {
        match code_point {
            cp::HYPHEN_MINUS => {
                self.state = State::ScriptDataDoubleEscapedDash;
                self.emit_chars("-");
            }
            cp::LESS_THAN_SIGN => {
                self.state = State::ScriptDataDoubleEscapedLessThanSign;
                self.emit_chars("<");
            }
            cp::NULL => {
                self.err(ErrorCode::UnexpectedNullCharacter);
                self.emit_chars(REPLACEMENT_CHARACTER_STR);
            }
            cp::EOF => {
                self.err(ErrorCode::EofInScriptHtmlCommentLikeText);
                self.emit_eof_token();
            }
            _ => self.emit_code_point(code_point),
        }
    }

    pub(in crate::tokenizer) fn state_script_data_double_escaped_dash(&mut self, code_point: i32) {
        match code_point {
            cp::HYPHEN_MINUS => {
                self.state = State::ScriptDataDoubleEscapedDashDash;
                self.emit_chars("-");
            }
            cp::LESS_THAN_SIGN => {
                self.state = State::ScriptDataDoubleEscapedLessThanSign;
                self.emit_chars("<");
            }
            cp::NULL => {
                self.err(ErrorCode::UnexpectedNullCharacter);
                self.state = State::ScriptDataDoubleEscaped;
                self.emit_chars(REPLACEMENT_CHARACTER_STR);
            }
            cp::EOF => {
                self.err(ErrorCode::EofInScriptHtmlCommentLikeText);
                self.emit_eof_token();
            }
            _ => {
                self.state = State::ScriptDataDoubleEscaped;
                self.emit_code_point(code_point);
            }
        }
    }

    pub(in crate::tokenizer) fn state_script_data_double_escaped_dash_dash(
        &mut self,
        code_point: i32,
    ) {
        match code_point {
            cp::HYPHEN_MINUS => self.emit_chars("-"),
            cp::LESS_THAN_SIGN => {
                self.state = State::ScriptDataDoubleEscapedLessThanSign;
                self.emit_chars("<");
            }
            cp::GREATER_THAN_SIGN => {
                self.state = State::ScriptData;
                self.emit_chars(">");
            }
            cp::NULL => {
                self.err(ErrorCode::UnexpectedNullCharacter);
                self.state = State::ScriptDataDoubleEscaped;
                self.emit_chars(REPLACEMENT_CHARACTER_STR);
            }
            cp::EOF => {
                self.err(ErrorCode::EofInScriptHtmlCommentLikeText);
                self.emit_eof_token();
            }
            _ => {
                self.state = State::ScriptDataDoubleEscaped;
                self.emit_code_point(code_point);
            }
        }
    }

    pub(in crate::tokenizer) fn state_script_data_double_escaped_less_than_sign(
        &mut self,
        code_point: i32,
    ) {
        if code_point == cp::SOLIDUS {
            self.state = State::ScriptDataDoubleEscapeEnd;
            self.emit_chars("/");
        } else {
            self.state = State::ScriptDataDoubleEscaped;
            self.state_script_data_double_escaped(code_point);
        }
    }

    pub(in crate::tokenizer) fn state_script_data_double_escape_end(&mut self, code_point: i32) {
        if self.preprocessor.starts_with(seq::SCRIPT, false)
            && is_script_data_double_escape_sequence_end(
                self.preprocessor.peek(seq::SCRIPT.len() as isize),
            )
        {
            self.emit_code_point(code_point);

            for _ in 0..seq::SCRIPT.len() {
                let code_point = self.consume();
                self.emit_code_point(code_point);
            }

            self.state = State::ScriptDataEscaped;
        } else if !self.ensure_hibernation() {
            self.state = State::ScriptDataDoubleEscaped;
            self.state_script_data_double_escaped(code_point);
        }
    }
}
