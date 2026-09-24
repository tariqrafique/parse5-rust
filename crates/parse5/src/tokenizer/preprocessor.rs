use crate::common::error_codes::{ErrorCode, ParserError};
use crate::common::token::Location;
use crate::common::unicode::code_points;
use crate::common::unicode::{is_control_code_point, is_surrogate, is_undefined_code_point};

pub const EOF: i32 = code_points::EOF;
pub const NULL: i32 = code_points::NULL;
pub const CARRIAGE_RETURN: i32 = code_points::CARRIAGE_RETURN;
pub const LINE_FEED: i32 = code_points::LINE_FEED;

const DEFAULT_BUFFER_WATERLINE: usize = 1 << 16;

#[derive(Debug, Clone)]
pub struct Preprocessor {
    /// Input buffer. Code units before `html_start` were dropped as already
    /// parsed and are only kept until compaction is worth the copy.
    html: Vec<u16>,
    html_start: usize,
    pub pos: isize,
    last_gap_pos: isize,
    gap_stack: Vec<isize>,
    skip_next_new_line: bool,
    pub last_chunk_written: bool,
    pub end_of_chunk_hit: bool,
    pub buffer_waterline: usize,
    is_eol: bool,
    line_start_pos: isize,
    pub dropped_buffer_size: usize,
    pub line: usize,
    last_err_offset: Option<usize>,
    errors: Vec<ParserError>,
}

impl Default for Preprocessor {
    fn default() -> Self {
        Self {
            html: Vec::new(),
            html_start: 0,
            pos: -1,
            last_gap_pos: -2,
            gap_stack: Vec::new(),
            skip_next_new_line: false,
            last_chunk_written: false,
            end_of_chunk_hit: false,
            buffer_waterline: DEFAULT_BUFFER_WATERLINE,
            is_eol: false,
            line_start_pos: 0,
            dropped_buffer_size: 0,
            line: 1,
            last_err_offset: None,
            errors: Vec::new(),
        }
    }
}

impl Preprocessor {
    pub fn new(input: impl AsRef<str>) -> Self {
        let mut preprocessor = Self::default();
        preprocessor.write(input.as_ref(), true);
        preprocessor
    }

    pub fn streaming() -> Self {
        Self::default()
    }

    pub fn col(&self) -> usize {
        let col = self.pos - self.line_start_pos + isize::from(self.last_gap_pos != self.pos);
        col.max(0) as usize
    }

    pub fn offset(&self) -> usize {
        (self.dropped_buffer_size as isize + self.pos).max(0) as usize
    }

    pub fn errors(&self) -> &[ParserError] {
        &self.errors
    }

    pub fn take_errors(&mut self) -> Vec<ParserError> {
        std::mem::take(&mut self.errors)
    }

    pub fn current_location(&self) -> Location {
        let line = self.line;
        let col = self.col();
        let offset = self.offset();

        Location {
            start_line: line,
            start_col: col,
            start_offset: offset,
            end_line: line,
            end_col: col,
            end_offset: offset,
        }
    }

    pub fn get_error(&self, code: ErrorCode, cp_offset: isize) -> ParserError {
        let col = (self.col() as isize + cp_offset).max(0) as usize;
        let offset = (self.offset() as isize + cp_offset).max(0) as usize;

        ParserError {
            code,
            location: Location {
                start_line: self.line,
                start_col: col,
                start_offset: offset,
                end_line: self.line,
                end_col: col,
                end_offset: offset,
            },
        }
    }

    fn err(&mut self, code: ErrorCode) {
        let offset = self.offset();

        if self.last_err_offset != Some(offset) {
            self.last_err_offset = Some(offset);
            self.errors.push(self.get_error(code, 0));
        }
    }

    fn add_gap(&mut self) {
        self.gap_stack.push(self.last_gap_pos);
        self.last_gap_pos = self.pos;
    }

    fn process_surrogate(&mut self, cp: u16) -> i32 {
        if self.pos != self.buf().len() as isize - 1 {
            let next_cp = self.buf()[self.pos as usize + 1];

            if is_surrogate_pair(next_cp) {
                self.pos += 1;
                self.add_gap();

                return get_surrogate_pair_code_point(cp, next_cp) as i32;
            }
        } else if !self.last_chunk_written {
            self.end_of_chunk_hit = true;
            return EOF;
        }

        self.err(ErrorCode::SurrogateInInputStream);
        cp as i32
    }

    pub fn will_drop_parsed_chunk(&self) -> bool {
        self.pos > self.buffer_waterline as isize
    }

    pub fn drop_parsed_chunk(&mut self) {
        if self.will_drop_parsed_chunk() {
            let pos = self.pos as usize;

            self.html_start += pos;

            // Compact only once the dead prefix outweighs the live data, so
            // repeated drops stay amortized O(1) instead of recopying the
            // remaining input every time the waterline is crossed.
            if self.html_start >= self.html.len() - self.html_start {
                self.html.drain(..self.html_start);
                self.html_start = 0;
            }

            self.line_start_pos -= self.pos;
            self.dropped_buffer_size += pos;
            self.pos = 0;
            self.last_gap_pos = -2;
            self.gap_stack.clear();
        }
    }

    pub fn write(&mut self, chunk: &str, is_last_chunk: bool) {
        self.html.extend(chunk.encode_utf16());
        self.end_of_chunk_hit = false;
        self.last_chunk_written = is_last_chunk;
    }

    pub fn write_utf16(&mut self, chunk: &[u16], is_last_chunk: bool) {
        self.html.extend_from_slice(chunk);
        self.end_of_chunk_hit = false;
        self.last_chunk_written = is_last_chunk;
    }

    pub fn insert_html_at_current_pos(&mut self, chunk: &str) {
        let insertion_pos = self.html_start + (self.pos + 1).max(0) as usize;
        self.html
            .splice(insertion_pos..insertion_pos, chunk.encode_utf16());
        self.end_of_chunk_hit = false;
    }

    pub fn remaining_from_current_pos(&self) -> &[u16] {
        let start = self.pos.max(0) as usize;
        &self.buf()[start..]
    }

    /// The live (not yet dropped) part of the input buffer.
    fn buf(&self) -> &[u16] {
        &self.html[self.html_start..]
    }

    pub fn starts_with(&mut self, pattern: &str, case_sensitive: bool) -> bool {
        let start = self.pos.max(0) as usize;
        let pattern_len = pattern.encode_utf16().count();

        if start + pattern_len > self.buf().len() {
            self.end_of_chunk_hit = !self.last_chunk_written;
            return false;
        }

        let candidate = &self.buf()[start..start + pattern_len];

        if case_sensitive {
            return candidate.iter().copied().eq(pattern.encode_utf16());
        }

        candidate
            .iter()
            .zip(pattern.encode_utf16())
            .all(|(&actual, expected)| (actual | 0x20) == expected)
    }

    /// Flags the end of the chunk when fewer than `len` code units remain from
    /// the current position, mirroring what a failed `starts_with` of that
    /// length would do.
    pub fn mark_end_of_chunk_if_shorter_than(&mut self, len: usize) {
        let start = self.pos.max(0) as usize;

        if start + len > self.buf().len() {
            self.end_of_chunk_hit = !self.last_chunk_written;
        }
    }

    pub fn peek(&mut self, offset: isize) -> i32 {
        let pos = self.pos + offset;

        if pos < 0 || pos >= self.buf().len() as isize {
            self.end_of_chunk_hit = !self.last_chunk_written;
            return EOF;
        }

        let code = self.buf()[pos as usize] as i32;

        if code == CARRIAGE_RETURN {
            LINE_FEED
        } else {
            code
        }
    }

    pub fn advance(&mut self) -> i32 {
        self.pos += 1;

        if self.is_eol {
            self.is_eol = false;
            self.line += 1;
            self.line_start_pos = self.pos;
        }

        if self.pos >= self.buf().len() as isize {
            self.end_of_chunk_hit = !self.last_chunk_written;
            return EOF;
        }

        let mut cp = self.buf()[self.pos as usize] as i32;

        if cp == CARRIAGE_RETURN {
            self.is_eol = true;
            self.skip_next_new_line = true;
            return LINE_FEED;
        }

        if cp == LINE_FEED {
            self.is_eol = true;

            if self.skip_next_new_line {
                self.line = self.line.saturating_sub(1);
                self.skip_next_new_line = false;
                self.add_gap();
                return self.advance();
            }
        }

        self.skip_next_new_line = false;

        if is_surrogate(cp as u32) {
            cp = self.process_surrogate(cp as u16);
        }

        let is_common_valid_range = (cp > 0x1f && cp < 0x7f)
            || cp == LINE_FEED
            || cp == CARRIAGE_RETURN
            || (cp > 0x9f && cp < 0xfdd0);

        if !is_common_valid_range {
            self.check_for_problematic_characters(cp);
        }

        cp
    }

    fn check_for_problematic_characters(&mut self, cp: i32) {
        if cp < 0 {
            return;
        }

        let cp = cp as u32;

        if is_control_code_point(cp) {
            self.err(ErrorCode::ControlCharacterInInputStream);
        } else if is_undefined_code_point(cp) {
            self.err(ErrorCode::NoncharacterInInputStream);
        }
    }

    pub fn retreat(&mut self, count: isize) {
        self.pos -= count;

        while self.pos < self.last_gap_pos {
            self.last_gap_pos = self.gap_stack.pop().unwrap_or(-2);
            self.pos -= 1;
        }

        self.is_eol = false;
    }
}

fn is_surrogate_pair(cp: u16) -> bool {
    (0xdc00..=0xdfff).contains(&cp)
}

fn get_surrogate_pair_code_point(cp1: u16, cp2: u16) -> u32 {
    (cp1 as u32 - 0xd800) * 0x400 + 0x2400 + cp2 as u32
}

#[cfg(test)]
mod tests {
    use super::{Preprocessor, EOF, LINE_FEED};
    use crate::common::error_codes::ErrorCode;

    #[test]
    fn advances_and_normalizes_newlines_like_parse5() {
        let mut preprocessor = Preprocessor::new("a\r\nb\rc\n");

        assert_eq!(preprocessor.advance(), 'a' as i32);
        assert_eq!(preprocessor.line, 1);
        assert_eq!(preprocessor.col(), 1);

        assert_eq!(preprocessor.advance(), LINE_FEED);
        assert_eq!(preprocessor.line, 1);
        assert_eq!(preprocessor.col(), 2);

        assert_eq!(preprocessor.advance(), 'b' as i32);
        assert_eq!(preprocessor.line, 2);
        assert_eq!(preprocessor.col(), 1);

        assert_eq!(preprocessor.advance(), LINE_FEED);
        assert_eq!(preprocessor.line, 2);
        assert_eq!(preprocessor.col(), 2);

        assert_eq!(preprocessor.advance(), 'c' as i32);
        assert_eq!(preprocessor.line, 3);
        assert_eq!(preprocessor.col(), 1);

        assert_eq!(preprocessor.advance(), LINE_FEED);
        assert_eq!(preprocessor.advance(), EOF);
        assert!(!preprocessor.end_of_chunk_hit);
    }

    #[test]
    fn peeks_and_starts_with_without_consuming() {
        let mut preprocessor = Preprocessor::new("<!DOCTYPE html>");

        preprocessor.advance();

        assert_eq!(preprocessor.peek(0), '<' as i32);
        assert!(preprocessor.starts_with("<!DOCTYPE", true));
        assert!(!preprocessor.starts_with("<!doctype", true));
        assert!(preprocessor.starts_with("<!doctype", false));
        assert_eq!(preprocessor.pos, 0);
    }

    #[test]
    fn detects_end_of_chunk_when_match_needs_more_input() {
        let mut preprocessor = Preprocessor::streaming();
        preprocessor.write("<!DOC", false);
        preprocessor.advance();

        assert!(!preprocessor.starts_with("<!DOCTYPE", false));
        assert!(preprocessor.end_of_chunk_hit);

        preprocessor.write("TYPE", true);
        assert!(preprocessor.starts_with("<!doctype", false));
        assert!(!preprocessor.end_of_chunk_hit);
    }

    #[test]
    fn inserts_html_after_current_position() {
        let mut preprocessor = Preprocessor::new("ab");

        assert_eq!(preprocessor.advance(), 'a' as i32);
        preprocessor.insert_html_at_current_pos("XY");

        assert_eq!(preprocessor.advance(), 'X' as i32);
        assert_eq!(preprocessor.advance(), 'Y' as i32);
        assert_eq!(preprocessor.advance(), 'b' as i32);
        assert_eq!(preprocessor.advance(), EOF);
    }

    #[test]
    fn combines_surrogate_pairs_and_retreats_over_gaps() {
        let mut preprocessor = Preprocessor::new("a\u{1f4a9}b");

        assert_eq!(preprocessor.advance(), 'a' as i32);
        assert_eq!(preprocessor.advance(), 0x1f4a9);
        assert_eq!(preprocessor.pos, 2);
        assert_eq!(preprocessor.col(), 2);

        preprocessor.retreat(1);

        assert_eq!(preprocessor.pos, 0);
        assert_eq!(preprocessor.advance(), 0x1f4a9);
        assert_eq!(preprocessor.advance(), 'b' as i32);
    }

    #[test]
    fn reports_problematic_input_stream_code_points_once_per_offset() {
        let mut preprocessor = Preprocessor::streaming();
        preprocessor.write_utf16(&[0x0001, 0xfdd0, 0xd800], true);

        assert_eq!(preprocessor.advance(), 0x0001);
        assert_eq!(preprocessor.advance(), 0xfdd0);
        assert_eq!(preprocessor.advance(), 0xd800);

        let errors = preprocessor.errors();
        assert_eq!(errors.len(), 3);
        assert_eq!(errors[0].code, ErrorCode::ControlCharacterInInputStream);
        assert_eq!(errors[1].code, ErrorCode::NoncharacterInInputStream);
        assert_eq!(errors[2].code, ErrorCode::SurrogateInInputStream);

        preprocessor.retreat(1);
        assert_eq!(preprocessor.advance(), 0xd800);
        assert_eq!(preprocessor.errors().len(), 3);
    }

    #[test]
    fn drops_parsed_chunks_after_waterline() {
        let mut preprocessor = Preprocessor::new("abcdef");
        preprocessor.buffer_waterline = 1;

        assert_eq!(preprocessor.advance(), 'a' as i32);
        assert_eq!(preprocessor.advance(), 'b' as i32);
        assert_eq!(preprocessor.advance(), 'c' as i32);
        assert!(preprocessor.will_drop_parsed_chunk());
        preprocessor.drop_parsed_chunk();

        assert_eq!(preprocessor.pos, 0);
        assert_eq!(preprocessor.offset(), 2);
        assert_eq!(preprocessor.advance(), 'd' as i32);
    }
}
