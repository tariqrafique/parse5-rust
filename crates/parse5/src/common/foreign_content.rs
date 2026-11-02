use crate::common::html::{
    get_tag_id, TagId, ATTR_COLOR, ATTR_ENCODING, ATTR_FACE, ATTR_SIZE, NS_HTML, NS_MATHML, NS_SVG,
    NS_XLINK, NS_XML, NS_XMLNS,
};
use crate::common::token::{Attribute, TagToken};

const MIME_TEXT_HTML: &str = "text/html";
const MIME_APPLICATION_XML: &str = "application/xhtml+xml";

const DEFINITION_URL_ATTR: &str = "definitionurl";
const ADJUSTED_DEFINITION_URL_ATTR: &str = "definitionURL";

pub fn causes_exit(start_tag_token: &TagToken) -> bool {
    let tag_id = start_tag_token.tag_id;
    let is_font_with_attrs = tag_id == TagId::Font
        && start_tag_token
            .attrs
            .iter()
            .any(|attr| matches!(attr.name.as_str(), ATTR_COLOR | ATTR_SIZE | ATTR_FACE));

    is_font_with_attrs || exits_foreign_content(tag_id)
}

pub fn adjust_token_mathml_attrs(token: &mut TagToken) {
    if let Some(attr) = token
        .attrs
        .iter_mut()
        .find(|attr| attr.name == DEFINITION_URL_ATTR)
    {
        attr.name = ADJUSTED_DEFINITION_URL_ATTR.to_string();
    }
}

pub fn adjust_token_svg_attrs(token: &mut TagToken) {
    for attr in &mut token.attrs {
        if let Some(adjusted_name) = adjusted_svg_attr_name(&attr.name) {
            attr.name = adjusted_name.to_string();
        }
    }
}

pub fn adjust_token_xml_attrs(token: &mut TagToken) {
    for attr in &mut token.attrs {
        if let Some((prefix, name, namespace)) = adjusted_xml_attr(&attr.name) {
            attr.prefix = Some(prefix.to_string());
            attr.name = name.to_string();
            attr.namespace = Some(namespace.to_string());
        }
    }
}

pub fn adjust_token_svg_tag_name(token: &mut TagToken) {
    if let Some(adjusted_tag_name) = adjusted_svg_tag_name(&token.tag_name) {
        token.tag_name = adjusted_tag_name.to_string();
        token.tag_id = get_tag_id(&token.tag_name);
    }
}

pub fn is_integration_point(
    tag_id: TagId,
    namespace_uri: &str,
    attrs: &[Attribute],
    foreign_namespace: Option<&str>,
) -> bool {
    let html_requested = foreign_namespace.is_none_or(|ns| ns == NS_HTML);
    let mathml_requested = foreign_namespace.is_none_or(|ns| ns == NS_MATHML);

    (html_requested && is_html_integration_point(tag_id, namespace_uri, attrs))
        || (mathml_requested && is_mathml_text_integration_point(tag_id, namespace_uri))
}

pub fn is_html_integration_point(tag_id: TagId, namespace_uri: &str, attrs: &[Attribute]) -> bool {
    if namespace_uri == NS_MATHML && tag_id == TagId::AnnotationXml {
        return attrs.iter().any(|attr| {
            attr.name == ATTR_ENCODING
                && matches!(
                    attr.value.to_ascii_lowercase().as_str(),
                    MIME_TEXT_HTML | MIME_APPLICATION_XML
                )
        });
    }

    namespace_uri == NS_SVG && matches!(tag_id, TagId::ForeignObject | TagId::Desc | TagId::Title)
}

pub fn is_mathml_text_integration_point(tag_id: TagId, namespace_uri: &str) -> bool {
    namespace_uri == NS_MATHML
        && matches!(
            tag_id,
            TagId::Mi | TagId::Mo | TagId::Mn | TagId::Ms | TagId::Mtext
        )
}

pub fn exits_foreign_content(tag_id: TagId) -> bool {
    matches!(
        tag_id,
        TagId::B
            | TagId::Big
            | TagId::Blockquote
            | TagId::Body
            | TagId::Br
            | TagId::Center
            | TagId::Code
            | TagId::Dd
            | TagId::Div
            | TagId::Dl
            | TagId::Dt
            | TagId::Em
            | TagId::Embed
            | TagId::H1
            | TagId::H2
            | TagId::H3
            | TagId::H4
            | TagId::H5
            | TagId::H6
            | TagId::Head
            | TagId::Hr
            | TagId::I
            | TagId::Img
            | TagId::Li
            | TagId::Listing
            | TagId::Menu
            | TagId::Meta
            | TagId::Nobr
            | TagId::Ol
            | TagId::P
            | TagId::Pre
            | TagId::Ruby
            | TagId::S
            | TagId::Small
            | TagId::Span
            | TagId::Strong
            | TagId::Strike
            | TagId::Sub
            | TagId::Sup
            | TagId::Table
            | TagId::Tt
            | TagId::U
            | TagId::Ul
            | TagId::Var
    )
}

pub const SVG_TAG_NAME_ADJUSTMENTS: &[(&str, &str)] = &[
    ("altglyph", "altGlyph"),
    ("altglyphdef", "altGlyphDef"),
    ("altglyphitem", "altGlyphItem"),
    ("animatecolor", "animateColor"),
    ("animatemotion", "animateMotion"),
    ("animatetransform", "animateTransform"),
    ("clippath", "clipPath"),
    ("feblend", "feBlend"),
    ("fecolormatrix", "feColorMatrix"),
    ("fecomponenttransfer", "feComponentTransfer"),
    ("fecomposite", "feComposite"),
    ("feconvolvematrix", "feConvolveMatrix"),
    ("fediffuselighting", "feDiffuseLighting"),
    ("fedisplacementmap", "feDisplacementMap"),
    ("fedistantlight", "feDistantLight"),
    ("feflood", "feFlood"),
    ("fefunca", "feFuncA"),
    ("fefuncb", "feFuncB"),
    ("fefuncg", "feFuncG"),
    ("fefuncr", "feFuncR"),
    ("fegaussianblur", "feGaussianBlur"),
    ("feimage", "feImage"),
    ("femerge", "feMerge"),
    ("femergenode", "feMergeNode"),
    ("femorphology", "feMorphology"),
    ("feoffset", "feOffset"),
    ("fepointlight", "fePointLight"),
    ("fespecularlighting", "feSpecularLighting"),
    ("fespotlight", "feSpotLight"),
    ("fetile", "feTile"),
    ("feturbulence", "feTurbulence"),
    ("foreignobject", "foreignObject"),
    ("glyphref", "glyphRef"),
    ("lineargradient", "linearGradient"),
    ("radialgradient", "radialGradient"),
    ("textpath", "textPath"),
];

pub fn adjusted_svg_tag_name(tag_name: &str) -> Option<&'static str> {
    lookup(SVG_TAG_NAME_ADJUSTMENTS, tag_name)
}

pub const SVG_ATTR_ADJUSTMENTS: &[(&str, &str)] = &[
    ("attributename", "attributeName"),
    ("attributetype", "attributeType"),
    ("basefrequency", "baseFrequency"),
    ("baseprofile", "baseProfile"),
    ("calcmode", "calcMode"),
    ("clippathunits", "clipPathUnits"),
    ("diffuseconstant", "diffuseConstant"),
    ("edgemode", "edgeMode"),
    ("filterunits", "filterUnits"),
    ("glyphref", "glyphRef"),
    ("gradienttransform", "gradientTransform"),
    ("gradientunits", "gradientUnits"),
    ("kernelmatrix", "kernelMatrix"),
    ("kernelunitlength", "kernelUnitLength"),
    ("keypoints", "keyPoints"),
    ("keysplines", "keySplines"),
    ("keytimes", "keyTimes"),
    ("lengthadjust", "lengthAdjust"),
    ("limitingconeangle", "limitingConeAngle"),
    ("markerheight", "markerHeight"),
    ("markerunits", "markerUnits"),
    ("markerwidth", "markerWidth"),
    ("maskcontentunits", "maskContentUnits"),
    ("maskunits", "maskUnits"),
    ("numoctaves", "numOctaves"),
    ("pathlength", "pathLength"),
    ("patterncontentunits", "patternContentUnits"),
    ("patterntransform", "patternTransform"),
    ("patternunits", "patternUnits"),
    ("pointsatx", "pointsAtX"),
    ("pointsaty", "pointsAtY"),
    ("pointsatz", "pointsAtZ"),
    ("preservealpha", "preserveAlpha"),
    ("preserveaspectratio", "preserveAspectRatio"),
    ("primitiveunits", "primitiveUnits"),
    ("refx", "refX"),
    ("refy", "refY"),
    ("repeatcount", "repeatCount"),
    ("repeatdur", "repeatDur"),
    ("requiredextensions", "requiredExtensions"),
    ("requiredfeatures", "requiredFeatures"),
    ("specularconstant", "specularConstant"),
    ("specularexponent", "specularExponent"),
    ("spreadmethod", "spreadMethod"),
    ("startoffset", "startOffset"),
    ("stddeviation", "stdDeviation"),
    ("stitchtiles", "stitchTiles"),
    ("surfacescale", "surfaceScale"),
    ("systemlanguage", "systemLanguage"),
    ("tablevalues", "tableValues"),
    ("targetx", "targetX"),
    ("targety", "targetY"),
    ("textlength", "textLength"),
    ("viewbox", "viewBox"),
    ("viewtarget", "viewTarget"),
    ("xchannelselector", "xChannelSelector"),
    ("ychannelselector", "yChannelSelector"),
    ("zoomandpan", "zoomAndPan"),
];

pub fn adjusted_svg_attr_name(attr_name: &str) -> Option<&'static str> {
    lookup(SVG_ATTR_ADJUSTMENTS, attr_name)
}

pub const XML_ATTR_ADJUSTMENTS: &[(&str, &str, &str, &str)] = &[
    ("xlink:actuate", "xlink", "actuate", NS_XLINK),
    ("xlink:arcrole", "xlink", "arcrole", NS_XLINK),
    ("xlink:href", "xlink", "href", NS_XLINK),
    ("xlink:role", "xlink", "role", NS_XLINK),
    ("xlink:show", "xlink", "show", NS_XLINK),
    ("xlink:title", "xlink", "title", NS_XLINK),
    ("xlink:type", "xlink", "type", NS_XLINK),
    ("xml:lang", "xml", "lang", NS_XML),
    ("xml:space", "xml", "space", NS_XML),
    ("xmlns", "", "xmlns", NS_XMLNS),
    ("xmlns:xlink", "xmlns", "xlink", NS_XMLNS),
];

fn adjusted_xml_attr(attr_name: &str) -> Option<(&'static str, &'static str, &'static str)> {
    XML_ATTR_ADJUSTMENTS
        .iter()
        .find(|(name, _, _, _)| *name == attr_name)
        .map(|(_, prefix, name, namespace)| (*prefix, *name, *namespace))
}

fn lookup<'a>(table: &'a [(&str, &str)], key: &str) -> Option<&'a str> {
    table
        .iter()
        .find(|(name, _)| *name == key)
        .map(|(_, value)| *value)
}

#[cfg(test)]
mod tests {
    use super::{
        adjust_token_mathml_attrs, adjust_token_svg_attrs, adjust_token_svg_tag_name,
        adjust_token_xml_attrs, causes_exit, is_integration_point,
    };
    use crate::common::html::{get_tag_id, TagId, NS_HTML, NS_MATHML, NS_SVG, NS_XLINK, NS_XMLNS};
    use crate::common::token::{Attribute, TagToken};

    fn token(tag_name: &str, attrs: Vec<Attribute>) -> TagToken {
        TagToken::new_start_tag(tag_name, get_tag_id(tag_name), attrs)
    }

    #[test]
    fn adjusts_mathml_definition_url() {
        let mut token = token("annotation-xml", vec![Attribute::new("definitionurl", "x")]);
        adjust_token_mathml_attrs(&mut token);
        assert_eq!(token.attrs[0].name, "definitionURL");
    }

    #[test]
    fn adjusts_svg_attrs_and_tags() {
        let mut token = token(
            "foreignobject",
            vec![
                Attribute::new("viewbox", "0 0 10 10"),
                Attribute::new("preserveaspectratio", "xMidYMid"),
            ],
        );
        adjust_token_svg_tag_name(&mut token);
        adjust_token_svg_attrs(&mut token);

        assert_eq!(token.tag_name, "foreignObject");
        assert_eq!(token.tag_id, TagId::ForeignObject);
        assert_eq!(token.attrs[0].name, "viewBox");
        assert_eq!(token.attrs[1].name, "preserveAspectRatio");
    }

    #[test]
    fn adjusts_xml_namespace_attrs() {
        let mut token = token(
            "use",
            vec![
                Attribute::new("xlink:href", "#icon"),
                Attribute::new("xmlns", "http://www.w3.org/2000/svg"),
                Attribute::new("xmlns:xlink", NS_XLINK),
            ],
        );
        adjust_token_xml_attrs(&mut token);

        assert_eq!(token.attrs[0].prefix.as_deref(), Some("xlink"));
        assert_eq!(token.attrs[0].name, "href");
        assert_eq!(token.attrs[0].namespace.as_deref(), Some(NS_XLINK));
        assert_eq!(token.attrs[0].qualified_name(), "xlink:href");

        assert_eq!(token.attrs[1].prefix.as_deref(), Some(""));
        assert_eq!(token.attrs[1].name, "xmlns");
        assert_eq!(token.attrs[1].namespace.as_deref(), Some(NS_XMLNS));
        assert_eq!(token.attrs[1].qualified_name(), "xmlns");

        assert_eq!(token.attrs[2].prefix.as_deref(), Some("xmlns"));
        assert_eq!(token.attrs[2].name, "xlink");
        assert_eq!(token.attrs[2].namespace.as_deref(), Some(NS_XMLNS));
        assert_eq!(token.attrs[2].qualified_name(), "xmlns:xlink");
    }

    #[test]
    fn detects_foreign_content_exit_tags() {
        assert!(causes_exit(&token("div", Vec::new())));
        assert!(causes_exit(&token(
            "font",
            vec![Attribute::new("color", "red")]
        )));
        assert!(!causes_exit(&token("font", Vec::new())));
        assert!(!causes_exit(&token("svg", Vec::new())));
    }

    #[test]
    fn detects_integration_points_with_foreign_namespace_filter() {
        let attrs = vec![Attribute::new("encoding", "TEXT/HTML")];
        assert!(is_integration_point(
            TagId::AnnotationXml,
            NS_MATHML,
            &attrs,
            None
        ));
        assert!(is_integration_point(
            TagId::AnnotationXml,
            NS_MATHML,
            &attrs,
            Some(NS_HTML)
        ));
        assert!(!is_integration_point(
            TagId::AnnotationXml,
            NS_MATHML,
            &attrs,
            Some(NS_MATHML)
        ));
        assert!(is_integration_point(
            TagId::Mi,
            NS_MATHML,
            &[],
            Some(NS_MATHML)
        ));
        assert!(is_integration_point(
            TagId::ForeignObject,
            NS_SVG,
            &[],
            None
        ));
    }
}
