use super::Tokenizer;
use super::*;

impl Tokenizer {
    pub(in crate::tokenizer) fn state_bogus_comment(&mut self, code_point: i32) {
        match code_point {
            cp::GREATER_THAN_SIGN => {
                self.state = State::Data;
                self.emit_current_comment();
            }
            cp::EOF => {
                self.emit_current_comment();
                self.emit_eof_token();
            }
            cp::NULL => {
                self.err(ErrorCode::UnexpectedNullCharacter);
                self.comment_token_mut().data.push(REPLACEMENT_CHARACTER);
            }
            _ => self
                .comment_token_mut()
                .data
                .extend(code_point_to_char(code_point)),
        }
    }

    pub(in crate::tokenizer) fn state_markup_declaration_open(&mut self, code_point: i32) {
        if self.consume_sequence_if_match(seq::DASH_DASH, true) {
            self.create_comment_token(seq::DASH_DASH.len() as isize + 1);
            self.state = State::CommentStart;
        } else if self.consume_sequence_if_match(seq::DOCTYPE, false) {
            self.current_location = self.get_current_location(seq::DOCTYPE.len() as isize + 1);
            self.state = State::Doctype;
        } else if self.consume_sequence_if_match(seq::CDATA_START, true) {
            if self.in_foreign_node {
                self.state = State::CDataSection;
            } else {
                self.err(ErrorCode::CdataInHtmlContent);
                self.create_comment_token(seq::CDATA_START.len() as isize + 1);
                self.comment_token_mut().data = "[CDATA[".to_string();
                self.state = State::BogusComment;
            }
        } else if !self.ensure_hibernation() {
            self.err(ErrorCode::IncorrectlyOpenedComment);
            self.create_comment_token(2);
            self.state = State::BogusComment;
            self.state_bogus_comment(code_point);
        }
    }

    pub(in crate::tokenizer) fn state_comment_start(&mut self, code_point: i32) {
        match code_point {
            cp::HYPHEN_MINUS => self.state = State::CommentStartDash,
            cp::GREATER_THAN_SIGN => {
                self.err(ErrorCode::AbruptClosingOfEmptyComment);
                self.state = State::Data;
                self.emit_current_comment();
            }
            _ => {
                self.state = State::Comment;
                self.state_comment(code_point);
            }
        }
    }

    pub(in crate::tokenizer) fn state_comment_start_dash(&mut self, code_point: i32) {
        match code_point {
            cp::HYPHEN_MINUS => self.state = State::CommentEnd,
            cp::GREATER_THAN_SIGN => {
                self.err(ErrorCode::AbruptClosingOfEmptyComment);
                self.state = State::Data;
                self.emit_current_comment();
            }
            cp::EOF => {
                self.err(ErrorCode::EofInComment);
                self.emit_current_comment();
                self.emit_eof_token();
            }
            _ => {
                self.comment_token_mut().data.push('-');
                self.state = State::Comment;
                self.state_comment(code_point);
            }
        }
    }

    pub(in crate::tokenizer) fn state_comment(&mut self, code_point: i32) {
        match code_point {
            cp::HYPHEN_MINUS => self.state = State::CommentEndDash,
            cp::LESS_THAN_SIGN => {
                self.comment_token_mut().data.push('<');
                self.state = State::CommentLessThanSign;
            }
            cp::NULL => {
                self.err(ErrorCode::UnexpectedNullCharacter);
                self.comment_token_mut().data.push(REPLACEMENT_CHARACTER);
            }
            cp::EOF => {
                self.err(ErrorCode::EofInComment);
                self.emit_current_comment();
                self.emit_eof_token();
            }
            _ => self
                .comment_token_mut()
                .data
                .extend(code_point_to_char(code_point)),
        }
    }

    pub(in crate::tokenizer) fn state_comment_less_than_sign(&mut self, code_point: i32) {
        match code_point {
            cp::EXCLAMATION_MARK => {
                self.comment_token_mut().data.push('!');
                self.state = State::CommentLessThanSignBang;
            }
            cp::LESS_THAN_SIGN => self.comment_token_mut().data.push('<'),
            _ => {
                self.state = State::Comment;
                self.state_comment(code_point);
            }
        }
    }

    pub(in crate::tokenizer) fn state_comment_less_than_sign_bang(&mut self, code_point: i32) {
        if code_point == cp::HYPHEN_MINUS {
            self.state = State::CommentLessThanSignBangDash;
        } else {
            self.state = State::Comment;
            self.state_comment(code_point);
        }
    }

    pub(in crate::tokenizer) fn state_comment_less_than_sign_bang_dash(&mut self, code_point: i32) {
        if code_point == cp::HYPHEN_MINUS {
            self.state = State::CommentLessThanSignBangDashDash;
        } else {
            self.state = State::CommentEndDash;
            self.state_comment_end_dash(code_point);
        }
    }

    pub(in crate::tokenizer) fn state_comment_less_than_sign_bang_dash_dash(
        &mut self,
        code_point: i32,
    ) {
        if code_point != cp::GREATER_THAN_SIGN && code_point != cp::EOF {
            self.err(ErrorCode::NestedComment);
        }

        self.state = State::CommentEnd;
        self.state_comment_end(code_point);
    }

    pub(in crate::tokenizer) fn state_comment_end_dash(&mut self, code_point: i32) {
        match code_point {
            cp::HYPHEN_MINUS => self.state = State::CommentEnd,
            cp::EOF => {
                self.err(ErrorCode::EofInComment);
                self.emit_current_comment();
                self.emit_eof_token();
            }
            _ => {
                self.comment_token_mut().data.push('-');
                self.state = State::Comment;
                self.state_comment(code_point);
            }
        }
    }

    pub(in crate::tokenizer) fn state_comment_end(&mut self, code_point: i32) {
        match code_point {
            cp::GREATER_THAN_SIGN => {
                self.state = State::Data;
                self.emit_current_comment();
            }
            cp::EXCLAMATION_MARK => self.state = State::CommentEndBang,
            cp::HYPHEN_MINUS => self.comment_token_mut().data.push('-'),
            cp::EOF => {
                self.err(ErrorCode::EofInComment);
                self.emit_current_comment();
                self.emit_eof_token();
            }
            _ => {
                self.comment_token_mut().data.push_str("--");
                self.state = State::Comment;
                self.state_comment(code_point);
            }
        }
    }

    pub(in crate::tokenizer) fn state_comment_end_bang(&mut self, code_point: i32) {
        match code_point {
            cp::HYPHEN_MINUS => {
                self.comment_token_mut().data.push_str("--!");
                self.state = State::CommentEndDash;
            }
            cp::GREATER_THAN_SIGN => {
                self.err(ErrorCode::IncorrectlyClosedComment);
                self.state = State::Data;
                self.emit_current_comment();
            }
            cp::EOF => {
                self.err(ErrorCode::EofInComment);
                self.emit_current_comment();
                self.emit_eof_token();
            }
            _ => {
                self.comment_token_mut().data.push_str("--!");
                self.state = State::Comment;
                self.state_comment(code_point);
            }
        }
    }
}
