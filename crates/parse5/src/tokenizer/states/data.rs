use super::Tokenizer;
use super::*;

impl Tokenizer {
    pub(in crate::tokenizer) fn state_data(&mut self, code_point: i32) {
        match code_point {
            cp::LESS_THAN_SIGN => self.state = State::TagOpen,
            cp::AMPERSAND => self.consume_character_reference(false),
            cp::NULL => {
                self.err(ErrorCode::UnexpectedNullCharacter);
                self.emit_code_point(code_point);
            }
            cp::EOF => self.emit_eof_token(),
            _ => self.emit_code_point(code_point),
        }
    }

    pub(in crate::tokenizer) fn state_ambiguous_ampersand(&mut self, code_point: i32) {
        if is_ascii_alpha_numeric(code_point) {
            self.emit_character_reference(&code_point_to_string(code_point), false);
        } else {
            if code_point == cp::SEMICOLON {
                self.err(ErrorCode::UnknownNamedCharacterReference);
            }

            self.state = self.return_state;
            self.call_state(code_point);
        }
    }

    pub(in crate::tokenizer) fn state_rcdata(&mut self, code_point: i32) {
        match code_point {
            cp::AMPERSAND => self.consume_character_reference(false),
            cp::LESS_THAN_SIGN => self.state = State::RcDataLessThanSign,
            cp::NULL => {
                self.err(ErrorCode::UnexpectedNullCharacter);
                self.emit_chars(REPLACEMENT_CHARACTER_STR);
            }
            cp::EOF => self.emit_eof_token(),
            _ => self.emit_code_point(code_point),
        }
    }

    pub(in crate::tokenizer) fn state_rawtext(&mut self, code_point: i32) {
        match code_point {
            cp::LESS_THAN_SIGN => self.state = State::RawTextLessThanSign,
            cp::NULL => {
                self.err(ErrorCode::UnexpectedNullCharacter);
                self.emit_chars(REPLACEMENT_CHARACTER_STR);
            }
            cp::EOF => self.emit_eof_token(),
            _ => self.emit_code_point(code_point),
        }
    }

    pub(in crate::tokenizer) fn state_plaintext(&mut self, code_point: i32) {
        match code_point {
            cp::NULL => {
                self.err(ErrorCode::UnexpectedNullCharacter);
                self.emit_chars(REPLACEMENT_CHARACTER_STR);
            }
            cp::EOF => self.emit_eof_token(),
            _ => self.emit_code_point(code_point),
        }
    }
}
