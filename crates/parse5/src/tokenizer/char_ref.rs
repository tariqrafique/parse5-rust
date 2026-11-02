use super::Tokenizer;
use super::*;

impl Tokenizer {
    pub(in crate::tokenizer) fn emit_character_reference(
        &mut self,
        decoded: &str,
        in_attribute: bool,
    ) {
        if in_attribute {
            self.current_attr.value.push_str(decoded);
        } else {
            for ch in decoded.chars() {
                self.emit_code_point(ch as i32);
            }
        }
    }

    pub(in crate::tokenizer) fn emit_literal_ampersand(&mut self, in_attribute: bool) {
        if in_attribute {
            self.current_attr.value.push('&');
        } else {
            self.emit_code_point(cp::AMPERSAND);
        }
    }

    pub(in crate::tokenizer) fn consume_character_reference(&mut self, in_attribute: bool) {
        if self.character_reference_needs_more_input() {
            self.hibernate_at_current_character_reference();
            return;
        }

        if self.try_consume_numeric_character_reference(in_attribute) {
            return;
        }

        if self.try_consume_named_character_reference(in_attribute) {
            return;
        }

        if !in_attribute && is_ascii_alpha_numeric(self.preprocessor.peek(1)) {
            self.return_state = self.state;
            self.state = State::AmbiguousAmpersand;
            self.emit_literal_ampersand(in_attribute);
            return;
        }

        self.emit_literal_ampersand(in_attribute);
    }

    pub(in crate::tokenizer) fn character_reference_needs_more_input(&self) -> bool {
        if self.preprocessor.last_chunk_written {
            return false;
        }

        let remaining = self.preprocessor.remaining_from_current_pos();

        if remaining.len() <= 1 {
            return true;
        }

        if remaining[1] == b'#' as u16 {
            return numeric_character_reference_needs_more_input(&remaining[1..]);
        }

        is_proper_prefix_of_named_character_reference(remaining)
    }

    pub(in crate::tokenizer) fn hibernate_at_current_character_reference(&mut self) {
        self.preprocessor.retreat(self.consumed_after_snapshot);
        self.consumed_after_snapshot = 0;
        self.active = false;
        self.preprocessor.end_of_chunk_hit = true;
    }

    pub(in crate::tokenizer) fn try_consume_named_character_reference(
        &mut self,
        in_attribute: bool,
    ) -> bool {
        let best_match = longest_named_character_reference_prefix(
            self.preprocessor.remaining_from_current_pos(),
        );

        // Matching against every entity used to probe past the end of the
        // buffer for longer names; keep that end-of-chunk signal for streaming.
        self.preprocessor
            .mark_end_of_chunk_if_shorter_than(max_named_character_reference_len());

        let Some((entity, characters)) = best_match else {
            return false;
        };
        let has_semicolon = entity.ends_with(';');
        let entity_len = entity.encode_utf16().count();
        let next = self.preprocessor.peek(entity_len as isize);

        if in_attribute
            && !has_semicolon
            && (is_ascii_alpha_numeric(next) || next == cp::EQUALS_SIGN)
        {
            return false;
        }

        if !has_semicolon {
            self.err_with_offset(
                ErrorCode::MissingSemicolonAfterCharacterReference,
                entity_len as isize,
            );
        }

        self.advance_by(entity_len.saturating_sub(1));
        self.emit_character_reference(characters, in_attribute);
        true
    }

    pub(in crate::tokenizer) fn try_consume_numeric_character_reference(
        &mut self,
        in_attribute: bool,
    ) -> bool {
        if self.preprocessor.peek(1) != '#' as i32 {
            return false;
        }

        let mut offset = 2;
        let hex =
            matches!(self.preprocessor.peek(offset), cp if cp == 'x' as i32 || cp == 'X' as i32);

        if hex {
            offset += 1;
        }

        let digits_start = offset;
        let radix = if hex { 16 } else { 10 };
        let mut value = 0u32;

        loop {
            let code_point = self.preprocessor.peek(offset);
            let digit = digit_value(code_point, radix);

            let Some(digit) = digit else {
                break;
            };

            value = value.saturating_mul(radix).saturating_add(digit);
            offset += 1;
        }

        if offset == digits_start {
            self.err_with_offset(
                ErrorCode::AbsenceOfDigitsInNumericCharacterReference,
                offset,
            );
            self.emit_literal_ampersand(in_attribute);
            return true;
        }

        let has_semicolon = self.preprocessor.peek(offset) == cp::SEMICOLON;
        if has_semicolon {
            offset += 1;
        } else {
            self.err_with_offset(ErrorCode::MissingSemicolonAfterCharacterReference, offset);
        }

        if let Some(error) = get_error_for_numeric_character_reference(value) {
            self.err_with_offset(error, offset);
        }

        let replacement = replacement_for_numeric_character_reference(value);
        self.advance_by(offset as usize - 1);
        self.emit_character_reference(&code_point_to_string(replacement as i32), in_attribute);
        true
    }
}
