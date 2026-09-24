pub const REPLACEMENT_CHARACTER: char = '\u{fffd}';
pub const REPLACEMENT_CHARACTER_STR: &str = "\u{fffd}";

pub mod code_points {
    pub const EOF: i32 = -1;
    pub const NULL: i32 = 0x00;
    pub const TABULATION: i32 = 0x09;
    pub const CARRIAGE_RETURN: i32 = 0x0d;
    pub const LINE_FEED: i32 = 0x0a;
    pub const FORM_FEED: i32 = 0x0c;
    pub const SPACE: i32 = 0x20;
    pub const EXCLAMATION_MARK: i32 = 0x21;
    pub const QUOTATION_MARK: i32 = 0x22;
    pub const AMPERSAND: i32 = 0x26;
    pub const APOSTROPHE: i32 = 0x27;
    pub const HYPHEN_MINUS: i32 = 0x2d;
    pub const SOLIDUS: i32 = 0x2f;
    pub const DIGIT_0: i32 = 0x30;
    pub const DIGIT_9: i32 = 0x39;
    pub const SEMICOLON: i32 = 0x3b;
    pub const LESS_THAN_SIGN: i32 = 0x3c;
    pub const EQUALS_SIGN: i32 = 0x3d;
    pub const GREATER_THAN_SIGN: i32 = 0x3e;
    pub const QUESTION_MARK: i32 = 0x3f;
    pub const LATIN_CAPITAL_A: i32 = 0x41;
    pub const LATIN_CAPITAL_Z: i32 = 0x5a;
    pub const RIGHT_SQUARE_BRACKET: i32 = 0x5d;
    pub const GRAVE_ACCENT: i32 = 0x60;
    pub const LATIN_SMALL_A: i32 = 0x61;
    pub const LATIN_SMALL_Z: i32 = 0x7a;
}

pub mod sequences {
    pub const DASH_DASH: &str = "--";
    pub const CDATA_START: &str = "[CDATA[";
    pub const DOCTYPE: &str = "doctype";
    pub const SCRIPT: &str = "script";
    pub const PUBLIC: &str = "public";
    pub const SYSTEM: &str = "system";
}

pub fn is_surrogate(code: u32) -> bool {
    (0xd800..=0xdfff).contains(&code)
}

pub fn is_undefined_code_point(code: u32) -> bool {
    matches!(
        code,
        0xfdd0
            ..=0xfdef
                | 0xfffe
                | 0xffff
                | 0x1fffe
                | 0x1ffff
                | 0x2fffe
                | 0x2ffff
                | 0x3fffe
                | 0x3ffff
                | 0x4fffe
                | 0x4ffff
                | 0x5fffe
                | 0x5ffff
                | 0x6fffe
                | 0x6ffff
                | 0x7fffe
                | 0x7ffff
                | 0x8fffe
                | 0x8ffff
                | 0x9fffe
                | 0x9ffff
                | 0xafffe
                | 0xaffff
                | 0xbfffe
                | 0xbffff
                | 0xcfffe
                | 0xcffff
                | 0xdfffe
                | 0xdffff
                | 0xefffe
                | 0xeffff
                | 0xffffe
                | 0xfffff
                | 0x10fffe
                | 0x10ffff
    )
}

pub fn is_control_code_point(code: u32) -> bool {
    !matches!(code, 0x20 | 0x0a | 0x0d | 0x09 | 0x0c)
        && matches!(code, 0x0001..=0x001f | 0x007f..=0x009f)
}
