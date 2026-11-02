use super::Tokenizer;
use super::*;

impl Tokenizer {
    pub(in crate::tokenizer) fn state_cdata_section(&mut self, code_point: i32) {
        match code_point {
            cp::RIGHT_SQUARE_BRACKET => self.state = State::CDataSectionBracket,
            cp::EOF => {
                self.err(ErrorCode::EofInCdata);
                self.emit_eof_token();
            }
            _ => self.emit_code_point(code_point),
        }
    }

    pub(in crate::tokenizer) fn state_cdata_section_bracket(&mut self, code_point: i32) {
        if code_point == cp::RIGHT_SQUARE_BRACKET {
            self.state = State::CDataSectionEnd;
        } else {
            self.emit_chars("]");
            self.state = State::CDataSection;
            self.state_cdata_section(code_point);
        }
    }

    pub(in crate::tokenizer) fn state_cdata_section_end(&mut self, code_point: i32) {
        match code_point {
            cp::GREATER_THAN_SIGN => self.state = State::Data,
            cp::RIGHT_SQUARE_BRACKET => self.emit_chars("]"),
            _ => {
                self.emit_chars("]]");
                self.state = State::CDataSection;
                self.state_cdata_section(code_point);
            }
        }
    }
}
