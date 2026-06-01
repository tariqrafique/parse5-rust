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

pub fn adjusted_svg_tag_name(tag_name: &str) -> Option<&'static str> {
    match tag_name {
        "altglyph" => Some("altGlyph"),
        "altglyphdef" => Some("altGlyphDef"),
        "altglyphitem" => Some("altGlyphItem"),
        "animatecolor" => Some("animateColor"),
        "animatemotion" => Some("animateMotion"),
        "animatetransform" => Some("animateTransform"),
        "clippath" => Some("clipPath"),
        "feblend" => Some("feBlend"),
        "fecolormatrix" => Some("feColorMatrix"),
        "fecomponenttransfer" => Some("feComponentTransfer"),
        "fecomposite" => Some("feComposite"),
        "feconvolvematrix" => Some("feConvolveMatrix"),
        "fediffuselighting" => Some("feDiffuseLighting"),
        "fedisplacementmap" => Some("feDisplacementMap"),
        "fedistantlight" => Some("feDistantLight"),
        "feflood" => Some("feFlood"),
        "fefunca" => Some("feFuncA"),
        "fefuncb" => Some("feFuncB"),
        "fefuncg" => Some("feFuncG"),
        "fefuncr" => Some("feFuncR"),
        "fegaussianblur" => Some("feGaussianBlur"),
        "feimage" => Some("feImage"),
        "femerge" => Some("feMerge"),
        "femergenode" => Some("feMergeNode"),
        "femorphology" => Some("feMorphology"),
        "feoffset" => Some("feOffset"),
        "fepointlight" => Some("fePointLight"),
        "fespecularlighting" => Some("feSpecularLighting"),
        "fespotlight" => Some("feSpotLight"),
        "fetile" => Some("feTile"),
        "feturbulence" => Some("feTurbulence"),
        "foreignobject" => Some("foreignObject"),
        "glyphref" => Some("glyphRef"),
        "lineargradient" => Some("linearGradient"),
        "radialgradient" => Some("radialGradient"),
        "textpath" => Some("textPath"),
        _ => None,
    }
}

pub fn adjusted_svg_attr_name(attr_name: &str) -> Option<&'static str> {
    match attr_name {
        "attributename" => Some("attributeName"),
        "attributetype" => Some("attributeType"),
        "basefrequency" => Some("baseFrequency"),
        "baseprofile" => Some("baseProfile"),
        "calcmode" => Some("calcMode"),
        "clippathunits" => Some("clipPathUnits"),
        "diffuseconstant" => Some("diffuseConstant"),
        "edgemode" => Some("edgeMode"),
        "filterunits" => Some("filterUnits"),
        "glyphref" => Some("glyphRef"),
        "gradienttransform" => Some("gradientTransform"),
        "gradientunits" => Some("gradientUnits"),
        "kernelmatrix" => Some("kernelMatrix"),
        "kernelunitlength" => Some("kernelUnitLength"),
        "keypoints" => Some("keyPoints"),
        "keysplines" => Some("keySplines"),
        "keytimes" => Some("keyTimes"),
        "lengthadjust" => Some("lengthAdjust"),
        "limitingconeangle" => Some("limitingConeAngle"),
        "markerheight" => Some("markerHeight"),
        "markerunits" => Some("markerUnits"),
        "markerwidth" => Some("markerWidth"),
        "maskcontentunits" => Some("maskContentUnits"),
        "maskunits" => Some("maskUnits"),
        "numoctaves" => Some("numOctaves"),
        "pathlength" => Some("pathLength"),
        "patterncontentunits" => Some("patternContentUnits"),
        "patterntransform" => Some("patternTransform"),
        "patternunits" => Some("patternUnits"),
        "pointsatx" => Some("pointsAtX"),
        "pointsaty" => Some("pointsAtY"),
        "pointsatz" => Some("pointsAtZ"),
        "preservealpha" => Some("preserveAlpha"),
        "preserveaspectratio" => Some("preserveAspectRatio"),
        "primitiveunits" => Some("primitiveUnits"),
        "refx" => Some("refX"),
        "refy" => Some("refY"),
        "repeatcount" => Some("repeatCount"),
        "repeatdur" => Some("repeatDur"),
        "requiredextensions" => Some("requiredExtensions"),
        "requiredfeatures" => Some("requiredFeatures"),
        "specularconstant" => Some("specularConstant"),
        "specularexponent" => Some("specularExponent"),
        "spreadmethod" => Some("spreadMethod"),
        "startoffset" => Some("startOffset"),
        "stddeviation" => Some("stdDeviation"),
        "stitchtiles" => Some("stitchTiles"),
        "surfacescale" => Some("surfaceScale"),
        "systemlanguage" => Some("systemLanguage"),
        "tablevalues" => Some("tableValues"),
        "targetx" => Some("targetX"),
        "targety" => Some("targetY"),
        "textlength" => Some("textLength"),
        "viewbox" => Some("viewBox"),
        "viewtarget" => Some("viewTarget"),
        "xchannelselector" => Some("xChannelSelector"),
        "ychannelselector" => Some("yChannelSelector"),
        "zoomandpan" => Some("zoomAndPan"),
        _ => None,
    }
}

fn adjusted_xml_attr(attr_name: &str) -> Option<(&'static str, &'static str, &'static str)> {
    match attr_name {
        "xlink:actuate" => Some(("xlink", "actuate", NS_XLINK)),
        "xlink:arcrole" => Some(("xlink", "arcrole", NS_XLINK)),
        "xlink:href" => Some(("xlink", "href", NS_XLINK)),
        "xlink:role" => Some(("xlink", "role", NS_XLINK)),
        "xlink:show" => Some(("xlink", "show", NS_XLINK)),
        "xlink:title" => Some(("xlink", "title", NS_XLINK)),
        "xlink:type" => Some(("xlink", "type", NS_XLINK)),
        "xml:lang" => Some(("xml", "lang", NS_XML)),
        "xml:space" => Some(("xml", "space", NS_XML)),
        "xmlns" => Some(("", "xmlns", NS_XMLNS)),
        "xmlns:xlink" => Some(("xmlns", "xlink", NS_XMLNS)),
        _ => None,
    }
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
