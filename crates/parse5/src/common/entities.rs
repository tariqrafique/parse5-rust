use std::sync::OnceLock;

/// Appends `input` to `out`, escaping it for use in a double-quoted attribute
/// value (`&`, `"` and U+00A0), like the `entities` npm package.
pub fn push_escaped_attribute(out: &mut String, input: &str) {
    push_escaped(out, input, |ch| match ch {
        '"' => Some("&quot;"),
        '&' => Some("&amp;"),
        '\u{a0}' => Some("&nbsp;"),
        _ => None,
    });
}

/// Appends `input` to `out`, escaping it for use as text content (`&`, `<`,
/// `>` and U+00A0), like the `entities` npm package.
pub fn push_escaped_text(out: &mut String, input: &str) {
    push_escaped(out, input, |ch| match ch {
        '&' => Some("&amp;"),
        '<' => Some("&lt;"),
        '>' => Some("&gt;"),
        '\u{a0}' => Some("&nbsp;"),
        _ => None,
    });
}

pub fn escape_attribute(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    push_escaped_attribute(&mut out, input);
    out
}

pub fn escape_text(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    push_escaped_text(&mut out, input);
    out
}

/// Copies unescaped runs of `input` in bulk and substitutes the characters for
/// which `replacement` returns an entity.
fn push_escaped(out: &mut String, input: &str, replacement: impl Fn(char) -> Option<&'static str>) {
    let mut unescaped_start = 0;

    for (idx, ch) in input.char_indices() {
        if let Some(entity) = replacement(ch) {
            out.push_str(&input[unescaped_start..idx]);
            out.push_str(entity);
            unescaped_start = idx + ch.len_utf8();
        }
    }

    out.push_str(&input[unescaped_start..]);
}

/// Sorted lookup table over the HTML5 named character references.
///
/// Every entity name is ASCII (`&` followed by alphanumerics and an optional
/// `;`), so lookups can work on UTF-16 input by narrowing to bytes first.
struct NamedCharacterReferenceIndex {
    /// `(entity, characters)` pairs sorted by entity, first occurrence wins.
    entries: Vec<(&'static str, &'static str)>,
    /// Length of the longest entity name, in UTF-16 code units (== bytes).
    max_len: usize,
}

fn index() -> &'static NamedCharacterReferenceIndex {
    static INDEX: OnceLock<NamedCharacterReferenceIndex> = OnceLock::new();

    INDEX.get_or_init(|| {
        let mut entries = ::entities::ENTITIES
            .iter()
            .map(|entry| (entry.entity, entry.characters))
            .collect::<Vec<_>>();

        debug_assert!(entries.iter().all(|(entity, _)| entity.is_ascii()));
        // Stable sort + dedup keeps the first table entry for duplicate names.
        entries.sort_by_key(|(entity, _)| *entity);
        entries.dedup_by_key(|(entity, _)| *entity);

        let max_len = entries
            .iter()
            .map(|(entity, _)| entity.len())
            .max()
            .unwrap_or(0);

        NamedCharacterReferenceIndex { entries, max_len }
    })
}

impl NamedCharacterReferenceIndex {
    fn get(&self, entity: &[u8]) -> Option<(&'static str, &'static str)> {
        self.entries
            .binary_search_by(|(candidate, _)| candidate.as_bytes().cmp(entity))
            .ok()
            .map(|idx| self.entries[idx])
    }
}

/// Converts the leading ASCII code units of `input` (at most `limit`) to bytes.
fn ascii_prefix(input: &[u16], limit: usize) -> Vec<u8> {
    input
        .iter()
        .take(limit)
        .map_while(|&unit| u8::try_from(unit).ok().filter(u8::is_ascii))
        .collect()
}

/// Length in UTF-16 code units of the longest named character reference.
pub fn max_named_character_reference_len() -> usize {
    index().max_len
}

pub fn named_character_reference(entity: &str) -> Option<&'static str> {
    index()
        .get(entity.as_bytes())
        .map(|(_, characters)| characters)
}

/// Finds the longest named character reference that `input` starts with.
///
/// `input` is expected to start at the `&`. Returns `(entity, characters)`.
pub fn longest_named_character_reference_prefix(
    input: &[u16],
) -> Option<(&'static str, &'static str)> {
    let index = index();
    let candidate = ascii_prefix(input, index.max_len);

    // Entity names are `&` + alphanumerics + optional `;`, so only prefixes of
    // that shape can match; try them from longest to shortest.
    let name_end = candidate
        .iter()
        .skip(1)
        .position(|byte| !byte.is_ascii_alphanumeric())
        .map_or(candidate.len(), |pos| pos + 1);
    let longest = if candidate.get(name_end) == Some(&b';') {
        name_end + 1
    } else {
        name_end
    };

    (2..=longest)
        .rev()
        .find_map(|len| index.get(&candidate[..len]))
}

/// Returns `true` when `input` is a proper prefix of some named character
/// reference, i.e. more input could still complete a longer match.
pub fn is_proper_prefix_of_named_character_reference(input: &[u16]) -> bool {
    let index = index();

    if input.len() >= index.max_len {
        return false;
    }

    let prefix = ascii_prefix(input, input.len());

    if prefix.len() != input.len() {
        return false;
    }

    let start = index
        .entries
        .partition_point(|(entity, _)| entity.as_bytes() < prefix.as_slice());

    index.entries[start..]
        .iter()
        .take_while(|(entity, _)| entity.as_bytes().starts_with(&prefix))
        .any(|(entity, _)| entity.len() > prefix.len())
}

#[cfg(test)]
mod tests {
    use super::{
        escape_attribute, escape_text, is_proper_prefix_of_named_character_reference,
        longest_named_character_reference_prefix, named_character_reference,
    };

    fn utf16(input: &str) -> Vec<u16> {
        input.encode_utf16().collect()
    }

    #[test]
    fn escapes_text_like_entities_package() {
        assert_eq!(escape_text("a&b<c>d\u{a0}"), "a&amp;b&lt;c&gt;d&nbsp;");
    }

    #[test]
    fn escapes_attributes_like_entities_package() {
        assert_eq!(escape_attribute("\"a&b\u{a0}<"), "&quot;a&amp;b&nbsp;<");
    }

    #[test]
    fn looks_up_html5_named_character_references() {
        assert_eq!(named_character_reference("&amp;"), Some("&"));
        assert_eq!(named_character_reference("&notin;"), Some("∉"));
        assert_eq!(named_character_reference("&not"), Some("¬"));
        assert_eq!(named_character_reference("&not-a-reference;"), None);
    }

    #[test]
    fn finds_longest_named_character_reference_prefix() {
        let longest = |input: &str| longest_named_character_reference_prefix(&utf16(input));

        assert_eq!(longest("&amp;x"), Some(("&amp;", "&")));
        assert_eq!(longest("&ampx"), Some(("&amp", "&")));
        assert_eq!(longest("&notin;"), Some(("&notin;", "∉")));
        assert_eq!(longest("&notit;"), Some(("&not", "¬")));
        assert_eq!(longest("&xyz;"), None);
        assert_eq!(longest("&"), None);
        assert_eq!(longest("&\u{e9}"), None);
    }

    #[test]
    fn detects_proper_prefixes_of_named_character_references() {
        let is_prefix = |input: &str| is_proper_prefix_of_named_character_reference(&utf16(input));

        assert!(is_prefix("&"));
        assert!(is_prefix("&am"));
        assert!(is_prefix("&amp"));
        assert!(!is_prefix("&amp;"));
        assert!(!is_prefix("&ampx"));
        assert!(!is_prefix("&\u{e9}"));
    }
}
