use crate::common::html::DocumentMode;

const VALID_DOCTYPE_NAME: &str = "html";
const VALID_SYSTEM_ID: &str = "about:legacy-compat";
const QUIRKS_MODE_SYSTEM_ID: &str = "http://www.ibm.com/data/dtd/v11/ibmxhtml1-transitional.dtd";

const QUIRKS_MODE_PUBLIC_ID_PREFIXES: &[&str] = &[
    "+//silmaril//dtd html pro v0r11 19970101//",
    "-//as//dtd html 3.0 aswedit + extensions//",
    "-//advasoft ltd//dtd html 3.0 aswedit + extensions//",
    "-//ietf//dtd html 2.0 level 1//",
    "-//ietf//dtd html 2.0 level 2//",
    "-//ietf//dtd html 2.0 strict level 1//",
    "-//ietf//dtd html 2.0 strict level 2//",
    "-//ietf//dtd html 2.0 strict//",
    "-//ietf//dtd html 2.0//",
    "-//ietf//dtd html 2.1e//",
    "-//ietf//dtd html 3.0//",
    "-//ietf//dtd html 3.2 final//",
    "-//ietf//dtd html 3.2//",
    "-//ietf//dtd html 3//",
    "-//ietf//dtd html level 0//",
    "-//ietf//dtd html level 1//",
    "-//ietf//dtd html level 2//",
    "-//ietf//dtd html level 3//",
    "-//ietf//dtd html strict level 0//",
    "-//ietf//dtd html strict level 1//",
    "-//ietf//dtd html strict level 2//",
    "-//ietf//dtd html strict level 3//",
    "-//ietf//dtd html strict//",
    "-//ietf//dtd html//",
    "-//metrius//dtd metrius presentational//",
    "-//microsoft//dtd internet explorer 2.0 html strict//",
    "-//microsoft//dtd internet explorer 2.0 html//",
    "-//microsoft//dtd internet explorer 2.0 tables//",
    "-//microsoft//dtd internet explorer 3.0 html strict//",
    "-//microsoft//dtd internet explorer 3.0 html//",
    "-//microsoft//dtd internet explorer 3.0 tables//",
    "-//netscape comm. corp.//dtd html//",
    "-//netscape comm. corp.//dtd strict html//",
    "-//o'reilly and associates//dtd html 2.0//",
    "-//o'reilly and associates//dtd html extended 1.0//",
    "-//o'reilly and associates//dtd html extended relaxed 1.0//",
    "-//sq//dtd html 2.0 hotmetal + extensions//",
    "-//softquad software//dtd hotmetal pro 6.0::19990601::extensions to html 4.0//",
    "-//softquad//dtd hotmetal pro 4.0::19971010::extensions to html 4.0//",
    "-//spyglass//dtd html 2.0 extended//",
    "-//sun microsystems corp.//dtd hotjava html//",
    "-//sun microsystems corp.//dtd hotjava strict html//",
    "-//w3c//dtd html 3 1995-03-24//",
    "-//w3c//dtd html 3.2 draft//",
    "-//w3c//dtd html 3.2 final//",
    "-//w3c//dtd html 3.2//",
    "-//w3c//dtd html 3.2s draft//",
    "-//w3c//dtd html 4.0 frameset//",
    "-//w3c//dtd html 4.0 transitional//",
    "-//w3c//dtd html experimental 19960712//",
    "-//w3c//dtd html experimental 970421//",
    "-//w3c//dtd w3 html//",
    "-//w3o//dtd w3 html 3.0//",
    "-//webtechs//dtd mozilla html 2.0//",
    "-//webtechs//dtd mozilla html//",
];

const QUIRKS_MODE_NO_SYSTEM_ID_EXTRA_PUBLIC_ID_PREFIXES: &[&str] = &[
    "-//w3c//dtd html 4.01 frameset//",
    "-//w3c//dtd html 4.01 transitional//",
];

const QUIRKS_MODE_PUBLIC_IDS: &[&str] = &[
    "-//w3o//dtd w3 html strict 3.0//en//",
    "-/w3c/dtd html 4.0 transitional/en",
    "html",
];

const LIMITED_QUIRKS_PUBLIC_ID_PREFIXES: &[&str] = &[
    "-//w3c//dtd xhtml 1.0 frameset//",
    "-//w3c//dtd xhtml 1.0 transitional//",
];

const LIMITED_QUIRKS_WITH_SYSTEM_ID_EXTRA_PUBLIC_ID_PREFIXES: &[&str] = &[
    "-//w3c//dtd html 4.01 frameset//",
    "-//w3c//dtd html 4.01 transitional//",
];

pub fn is_conforming(name: Option<&str>, public_id: Option<&str>, system_id: Option<&str>) -> bool {
    name == Some(VALID_DOCTYPE_NAME)
        && public_id.is_none()
        && (system_id.is_none() || system_id == Some(VALID_SYSTEM_ID))
}

pub fn document_mode(
    name: Option<&str>,
    public_id: Option<&str>,
    system_id: Option<&str>,
    force_quirks: bool,
) -> DocumentMode {
    if force_quirks {
        DocumentMode::Quirks
    } else {
        get_document_mode(name, public_id, system_id)
    }
}

pub fn get_document_mode(
    name: Option<&str>,
    public_id: Option<&str>,
    system_id: Option<&str>,
) -> DocumentMode {
    if name != Some(VALID_DOCTYPE_NAME) {
        return DocumentMode::Quirks;
    }

    if system_id
        .filter(|value| !value.is_empty())
        .is_some_and(|value| value.eq_ignore_ascii_case(QUIRKS_MODE_SYSTEM_ID))
    {
        return DocumentMode::Quirks;
    }

    let Some(public_id) = public_id else {
        return DocumentMode::NoQuirks;
    };

    let public_id = public_id.to_ascii_lowercase();

    if QUIRKS_MODE_PUBLIC_IDS.contains(&public_id.as_str()) {
        return DocumentMode::Quirks;
    }

    if has_prefix(&public_id, QUIRKS_MODE_PUBLIC_ID_PREFIXES)
        || (system_id.is_none()
            && has_prefix(
                &public_id,
                QUIRKS_MODE_NO_SYSTEM_ID_EXTRA_PUBLIC_ID_PREFIXES,
            ))
    {
        return DocumentMode::Quirks;
    }

    if has_prefix(&public_id, LIMITED_QUIRKS_PUBLIC_ID_PREFIXES)
        || (system_id.is_some()
            && has_prefix(
                &public_id,
                LIMITED_QUIRKS_WITH_SYSTEM_ID_EXTRA_PUBLIC_ID_PREFIXES,
            ))
    {
        return DocumentMode::LimitedQuirks;
    }

    DocumentMode::NoQuirks
}

fn has_prefix(public_id: &str, prefixes: &[&str]) -> bool {
    prefixes.iter().any(|prefix| public_id.starts_with(prefix))
}

#[cfg(test)]
mod tests {
    use super::{document_mode, get_document_mode, is_conforming};
    use crate::common::html::DocumentMode;

    #[test]
    fn conforming_doctype_matches_parse5() {
        assert!(is_conforming(Some("html"), None, None));
        assert!(is_conforming(
            Some("html"),
            None,
            Some("about:legacy-compat")
        ));
        assert!(!is_conforming(Some("HTML"), None, None));
        assert!(!is_conforming(Some("html"), Some(""), None));
        assert!(!is_conforming(Some("html"), None, Some("")));
    }

    #[test]
    fn force_quirks_wins() {
        assert_eq!(
            document_mode(Some("html"), None, None, true),
            DocumentMode::Quirks
        );
    }

    #[test]
    fn name_must_be_exactly_html() {
        assert_eq!(
            get_document_mode(Some("html"), None, None),
            DocumentMode::NoQuirks
        );
        assert_eq!(
            get_document_mode(Some("HTML"), None, None),
            DocumentMode::Quirks
        );
        assert_eq!(get_document_mode(None, None, None), DocumentMode::Quirks);
    }

    #[test]
    fn quirks_system_id_matches_case_insensitively_when_non_empty() {
        assert_eq!(
            get_document_mode(
                Some("html"),
                None,
                Some("HTTP://WWW.IBM.COM/DATA/DTD/V11/IBMXHTML1-TRANSITIONAL.DTD")
            ),
            DocumentMode::Quirks
        );
        assert_eq!(
            get_document_mode(Some("html"), None, Some("")),
            DocumentMode::NoQuirks
        );
        assert_eq!(
            get_document_mode(Some("html"), None, Some("something-else")),
            DocumentMode::NoQuirks
        );
    }

    #[test]
    fn quirks_public_ids_match_sets_and_prefixes() {
        assert_eq!(
            get_document_mode(Some("html"), Some("HTML"), None),
            DocumentMode::Quirks
        );
        assert_eq!(
            get_document_mode(Some("html"), Some("-//IETF//DTD HTML 2.0//rest"), Some("")),
            DocumentMode::Quirks
        );
        assert_eq!(
            get_document_mode(
                Some("html"),
                Some("-//W3C//DTD HTML 4.01 Transitional//"),
                None
            ),
            DocumentMode::Quirks
        );
    }

    #[test]
    fn html_401_public_ids_depend_on_system_id_presence() {
        assert_eq!(
            get_document_mode(
                Some("html"),
                Some("-//W3C//DTD HTML 4.01 Transitional//"),
                None
            ),
            DocumentMode::Quirks
        );
        assert_eq!(
            get_document_mode(
                Some("html"),
                Some("-//W3C//DTD HTML 4.01 Transitional//"),
                Some("")
            ),
            DocumentMode::LimitedQuirks
        );
        assert_eq!(
            get_document_mode(
                Some("html"),
                Some("-//W3C//DTD HTML 4.01 Frameset//"),
                Some("anything")
            ),
            DocumentMode::LimitedQuirks
        );
    }

    #[test]
    fn xhtml_public_ids_are_limited_quirks() {
        assert_eq!(
            get_document_mode(
                Some("html"),
                Some("-//W3C//DTD XHTML 1.0 Transitional//"),
                None
            ),
            DocumentMode::LimitedQuirks
        );
    }
}
