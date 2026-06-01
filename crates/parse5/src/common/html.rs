use serde::{Deserialize, Serialize};

pub const NS_HTML: &str = "http://www.w3.org/1999/xhtml";
pub const NS_MATHML: &str = "http://www.w3.org/1998/Math/MathML";
pub const NS_SVG: &str = "http://www.w3.org/2000/svg";
pub const NS_XLINK: &str = "http://www.w3.org/1999/xlink";
pub const NS_XML: &str = "http://www.w3.org/XML/1998/namespace";
pub const NS_XMLNS: &str = "http://www.w3.org/2000/xmlns/";

pub const ATTR_TYPE: &str = "type";
pub const ATTR_ACTION: &str = "action";
pub const ATTR_ENCODING: &str = "encoding";
pub const ATTR_PROMPT: &str = "prompt";
pub const ATTR_NAME: &str = "name";
pub const ATTR_COLOR: &str = "color";
pub const ATTR_FACE: &str = "face";
pub const ATTR_SIZE: &str = "size";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Namespace {
    Html,
    MathMl,
    Svg,
    XLink,
    Xml,
    Xmlns,
    Other,
}

impl Namespace {
    pub fn as_str(self) -> &'static str {
        match self {
            Namespace::Html => NS_HTML,
            Namespace::MathMl => NS_MATHML,
            Namespace::Svg => NS_SVG,
            Namespace::XLink => NS_XLINK,
            Namespace::Xml => NS_XML,
            Namespace::Xmlns => NS_XMLNS,
            Namespace::Other => "",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum DocumentMode {
    NoQuirks,
    Quirks,
    LimitedQuirks,
}

macro_rules! tag_ids {
    ($($variant:ident => $name:literal,)+) => {
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
        #[repr(u16)]
        pub enum TagId {
            Unknown = 0,
            $($variant,)+
        }

        impl TagId {
            pub fn as_str(self) -> Option<&'static str> {
                match self {
                    TagId::Unknown => None,
                    $(TagId::$variant => Some($name),)+
                }
            }
        }

        pub fn get_tag_id(tag_name: &str) -> TagId {
            match tag_name {
                $($name => TagId::$variant,)+
                _ => TagId::Unknown,
            }
        }

        pub const TAG_NAMES: &[&str] = &[$($name,)+];
    };
}

tag_ids! {
    A => "a",
    Address => "address",
    AnnotationXml => "annotation-xml",
    Applet => "applet",
    Area => "area",
    Article => "article",
    Aside => "aside",
    B => "b",
    Base => "base",
    Basefont => "basefont",
    Bgsound => "bgsound",
    Big => "big",
    Blockquote => "blockquote",
    Body => "body",
    Br => "br",
    Button => "button",
    Caption => "caption",
    Center => "center",
    Code => "code",
    Col => "col",
    Colgroup => "colgroup",
    Dd => "dd",
    Desc => "desc",
    Details => "details",
    Dialog => "dialog",
    Dir => "dir",
    Div => "div",
    Dl => "dl",
    Dt => "dt",
    Em => "em",
    Embed => "embed",
    Fieldset => "fieldset",
    Figcaption => "figcaption",
    Figure => "figure",
    Font => "font",
    Footer => "footer",
    ForeignObject => "foreignObject",
    Form => "form",
    Frame => "frame",
    Frameset => "frameset",
    H1 => "h1",
    H2 => "h2",
    H3 => "h3",
    H4 => "h4",
    H5 => "h5",
    H6 => "h6",
    Head => "head",
    Header => "header",
    Hgroup => "hgroup",
    Hr => "hr",
    Html => "html",
    I => "i",
    Img => "img",
    Image => "image",
    Input => "input",
    Iframe => "iframe",
    Keygen => "keygen",
    Label => "label",
    Li => "li",
    Link => "link",
    Listing => "listing",
    Main => "main",
    Malignmark => "malignmark",
    Marquee => "marquee",
    Math => "math",
    Menu => "menu",
    Meta => "meta",
    Mglyph => "mglyph",
    Mi => "mi",
    Mo => "mo",
    Mn => "mn",
    Ms => "ms",
    Mtext => "mtext",
    Nav => "nav",
    Nobr => "nobr",
    Noframes => "noframes",
    Noembed => "noembed",
    Noscript => "noscript",
    Object => "object",
    Ol => "ol",
    Optgroup => "optgroup",
    Option => "option",
    P => "p",
    Param => "param",
    Plaintext => "plaintext",
    Pre => "pre",
    Rb => "rb",
    Rp => "rp",
    Rt => "rt",
    Rtc => "rtc",
    Ruby => "ruby",
    S => "s",
    Script => "script",
    Search => "search",
    Section => "section",
    Select => "select",
    Source => "source",
    Small => "small",
    Span => "span",
    Strike => "strike",
    Strong => "strong",
    Style => "style",
    Sub => "sub",
    Summary => "summary",
    Sup => "sup",
    Table => "table",
    Tbody => "tbody",
    Template => "template",
    Textarea => "textarea",
    Tfoot => "tfoot",
    Td => "td",
    Th => "th",
    Thead => "thead",
    Title => "title",
    Tr => "tr",
    Track => "track",
    Tt => "tt",
    U => "u",
    Ul => "ul",
    Svg => "svg",
    Var => "var",
    Wbr => "wbr",
    Xmp => "xmp",
}

pub fn is_void_element(tag_name: &str) -> bool {
    matches!(
        get_tag_id(tag_name),
        TagId::Area
            | TagId::Base
            | TagId::Basefont
            | TagId::Bgsound
            | TagId::Br
            | TagId::Col
            | TagId::Embed
            | TagId::Frame
            | TagId::Hr
            | TagId::Img
            | TagId::Input
            | TagId::Keygen
            | TagId::Link
            | TagId::Meta
            | TagId::Param
            | TagId::Source
            | TagId::Track
            | TagId::Wbr
    )
}

pub fn is_special_element(namespace_uri: &str, tag_id: TagId) -> bool {
    match namespace_uri {
        NS_HTML => matches!(
            tag_id,
            TagId::Address
                | TagId::Applet
                | TagId::Area
                | TagId::Article
                | TagId::Aside
                | TagId::Base
                | TagId::Basefont
                | TagId::Bgsound
                | TagId::Blockquote
                | TagId::Body
                | TagId::Br
                | TagId::Button
                | TagId::Caption
                | TagId::Center
                | TagId::Col
                | TagId::Colgroup
                | TagId::Dd
                | TagId::Details
                | TagId::Dir
                | TagId::Div
                | TagId::Dl
                | TagId::Dt
                | TagId::Embed
                | TagId::Fieldset
                | TagId::Figcaption
                | TagId::Figure
                | TagId::Footer
                | TagId::Form
                | TagId::Frame
                | TagId::Frameset
                | TagId::H1
                | TagId::H2
                | TagId::H3
                | TagId::H4
                | TagId::H5
                | TagId::H6
                | TagId::Head
                | TagId::Header
                | TagId::Hgroup
                | TagId::Hr
                | TagId::Html
                | TagId::Iframe
                | TagId::Img
                | TagId::Input
                | TagId::Li
                | TagId::Link
                | TagId::Listing
                | TagId::Main
                | TagId::Marquee
                | TagId::Menu
                | TagId::Meta
                | TagId::Nav
                | TagId::Noembed
                | TagId::Noframes
                | TagId::Noscript
                | TagId::Object
                | TagId::Ol
                | TagId::P
                | TagId::Param
                | TagId::Plaintext
                | TagId::Pre
                | TagId::Script
                | TagId::Section
                | TagId::Select
                | TagId::Source
                | TagId::Style
                | TagId::Summary
                | TagId::Table
                | TagId::Tbody
                | TagId::Td
                | TagId::Template
                | TagId::Textarea
                | TagId::Tfoot
                | TagId::Th
                | TagId::Thead
                | TagId::Title
                | TagId::Tr
                | TagId::Track
                | TagId::Ul
                | TagId::Wbr
                | TagId::Xmp
        ),
        NS_MATHML => matches!(
            tag_id,
            TagId::Mi | TagId::Mo | TagId::Mn | TagId::Ms | TagId::Mtext | TagId::AnnotationXml
        ),
        NS_SVG => matches!(tag_id, TagId::Title | TagId::ForeignObject | TagId::Desc),
        NS_XLINK | NS_XML | NS_XMLNS => false,
        _ => false,
    }
}

pub fn is_numbered_header(tag_id: TagId) -> bool {
    matches!(
        tag_id,
        TagId::H1 | TagId::H2 | TagId::H3 | TagId::H4 | TagId::H5 | TagId::H6
    )
}

pub fn has_unescaped_text(tag_name: &str, scripting_enabled: bool) -> bool {
    matches!(
        tag_name,
        "style" | "script" | "xmp" | "iframe" | "noembed" | "noframes" | "plaintext"
    ) || (scripting_enabled && tag_name == "noscript")
}

#[cfg(test)]
mod tests {
    use super::{
        get_tag_id, has_unescaped_text, is_numbered_header, is_special_element, is_void_element,
        TagId, NS_HTML, NS_MATHML, NS_SVG, TAG_NAMES,
    };

    #[test]
    fn tag_ids_keep_parse5_order() {
        assert_eq!(TagId::Unknown as u16, 0);
        assert_eq!(TagId::A as u16, 1);
        assert_eq!(TagId::Address as u16, 2);
        assert_eq!(TagId::ForeignObject.as_str(), Some("foreignObject"));
        assert_eq!(TagId::Xmp.as_str(), Some("xmp"));
        assert_eq!(TAG_NAMES.first(), Some(&"a"));
        assert_eq!(TAG_NAMES.last(), Some(&"xmp"));
    }

    #[test]
    fn get_tag_id_is_case_sensitive_like_parse5() {
        assert_eq!(get_tag_id("template"), TagId::Template);
        assert_eq!(get_tag_id("foreignObject"), TagId::ForeignObject);
        assert_eq!(get_tag_id("foreignobject"), TagId::Unknown);
        assert_eq!(get_tag_id("TEMPLATE"), TagId::Unknown);
        assert_eq!(get_tag_id("not-a-real-tag"), TagId::Unknown);
    }

    #[test]
    fn void_element_set_matches_html_voids() {
        assert!(is_void_element("input"));
        assert!(is_void_element("wbr"));
        assert!(!is_void_element("textarea"));
        assert!(!is_void_element("foreignObject"));
    }

    #[test]
    fn special_elements_are_namespace_sensitive() {
        assert!(is_special_element(NS_HTML, TagId::Template));
        assert!(is_special_element(NS_MATHML, TagId::AnnotationXml));
        assert!(is_special_element(NS_SVG, TagId::ForeignObject));
        assert!(!is_special_element(NS_HTML, TagId::Span));
        assert!(!is_special_element(NS_SVG, TagId::Div));
    }

    #[test]
    fn numbered_headers_and_unescaped_text_match_parse5_sets() {
        assert!(is_numbered_header(TagId::H1));
        assert!(is_numbered_header(TagId::H6));
        assert!(!is_numbered_header(TagId::Header));
        assert!(has_unescaped_text("script", false));
        assert!(!has_unescaped_text("noscript", false));
        assert!(has_unescaped_text("noscript", true));
    }
}
