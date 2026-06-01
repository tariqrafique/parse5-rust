pub fn escape_attribute(input: &str) -> String {
    let mut out = String::new();
    let mut changed = false;

    for ch in input.chars() {
        match ch {
            '"' => {
                out.push_str("&quot;");
                changed = true;
            }
            '&' => {
                out.push_str("&amp;");
                changed = true;
            }
            '\u{a0}' => {
                out.push_str("&nbsp;");
                changed = true;
            }
            _ => out.push(ch),
        }
    }

    if changed {
        out
    } else {
        input.to_string()
    }
}

pub fn escape_text(input: &str) -> String {
    let mut out = String::new();
    let mut changed = false;

    for ch in input.chars() {
        match ch {
            '&' => {
                out.push_str("&amp;");
                changed = true;
            }
            '<' => {
                out.push_str("&lt;");
                changed = true;
            }
            '>' => {
                out.push_str("&gt;");
                changed = true;
            }
            '\u{a0}' => {
                out.push_str("&nbsp;");
                changed = true;
            }
            _ => out.push(ch),
        }
    }

    if changed {
        out
    } else {
        input.to_string()
    }
}

pub fn named_character_reference(entity: &str) -> Option<&'static str> {
    ::entities::ENTITIES
        .iter()
        .find(|entry| entry.entity == entity)
        .map(|entry| entry.characters)
}

#[cfg(test)]
mod tests {
    use super::{escape_attribute, escape_text, named_character_reference};

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
}
