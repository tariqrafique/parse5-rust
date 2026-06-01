let wasmModule = null;

export const NS = {
  HTML: 'http://www.w3.org/1999/xhtml',
  MATHML: 'http://www.w3.org/1998/Math/MathML',
  SVG: 'http://www.w3.org/2000/svg',
  XLINK: 'http://www.w3.org/1999/xlink',
  XML: 'http://www.w3.org/XML/1998/namespace',
  XMLNS: 'http://www.w3.org/2000/xmlns/',
};

export const DOCUMENT_MODE = {
  NO_QUIRKS: 'no-quirks',
  QUIRKS: 'quirks',
  LIMITED_QUIRKS: 'limited-quirks',
};

export const ATTRS = {
  TYPE: 'type',
  ACTION: 'action',
  ENCODING: 'encoding',
  PROMPT: 'prompt',
  NAME: 'name',
  COLOR: 'color',
  FACE: 'face',
  SIZE: 'size',
};

export const TAG_NAMES = {
  A: 'a',
  ADDRESS: 'address',
  ANNOTATION_XML: 'annotation-xml',
  APPLET: 'applet',
  AREA: 'area',
  ARTICLE: 'article',
  ASIDE: 'aside',
  B: 'b',
  BASE: 'base',
  BASEFONT: 'basefont',
  BGSOUND: 'bgsound',
  BIG: 'big',
  BLOCKQUOTE: 'blockquote',
  BODY: 'body',
  BR: 'br',
  BUTTON: 'button',
  CAPTION: 'caption',
  CENTER: 'center',
  CODE: 'code',
  COL: 'col',
  COLGROUP: 'colgroup',
  DD: 'dd',
  DESC: 'desc',
  DETAILS: 'details',
  DIALOG: 'dialog',
  DIR: 'dir',
  DIV: 'div',
  DL: 'dl',
  DT: 'dt',
  EM: 'em',
  EMBED: 'embed',
  FIELDSET: 'fieldset',
  FIGCAPTION: 'figcaption',
  FIGURE: 'figure',
  FONT: 'font',
  FOOTER: 'footer',
  FOREIGN_OBJECT: 'foreignObject',
  FORM: 'form',
  FRAME: 'frame',
  FRAMESET: 'frameset',
  H1: 'h1',
  H2: 'h2',
  H3: 'h3',
  H4: 'h4',
  H5: 'h5',
  H6: 'h6',
  HEAD: 'head',
  HEADER: 'header',
  HGROUP: 'hgroup',
  HR: 'hr',
  HTML: 'html',
  I: 'i',
  IMG: 'img',
  IMAGE: 'image',
  INPUT: 'input',
  IFRAME: 'iframe',
  KEYGEN: 'keygen',
  LABEL: 'label',
  LI: 'li',
  LINK: 'link',
  LISTING: 'listing',
  MAIN: 'main',
  MALIGNMARK: 'malignmark',
  MARQUEE: 'marquee',
  MATH: 'math',
  MENU: 'menu',
  META: 'meta',
  MGLYPH: 'mglyph',
  MI: 'mi',
  MO: 'mo',
  MN: 'mn',
  MS: 'ms',
  MTEXT: 'mtext',
  NAV: 'nav',
  NOBR: 'nobr',
  NOFRAMES: 'noframes',
  NOEMBED: 'noembed',
  NOSCRIPT: 'noscript',
  OBJECT: 'object',
  OL: 'ol',
  OPTGROUP: 'optgroup',
  OPTION: 'option',
  P: 'p',
  PARAM: 'param',
  PLAINTEXT: 'plaintext',
  PRE: 'pre',
  RB: 'rb',
  RP: 'rp',
  RT: 'rt',
  RTC: 'rtc',
  RUBY: 'ruby',
  S: 's',
  SCRIPT: 'script',
  SEARCH: 'search',
  SECTION: 'section',
  SELECT: 'select',
  SOURCE: 'source',
  SMALL: 'small',
  SPAN: 'span',
  STRIKE: 'strike',
  STRONG: 'strong',
  STYLE: 'style',
  SUB: 'sub',
  SUMMARY: 'summary',
  SUP: 'sup',
  TABLE: 'table',
  TBODY: 'tbody',
  TEMPLATE: 'template',
  TEXTAREA: 'textarea',
  TFOOT: 'tfoot',
  TD: 'td',
  TH: 'th',
  THEAD: 'thead',
  TITLE: 'title',
  TR: 'tr',
  TRACK: 'track',
  TT: 'tt',
  U: 'u',
  UL: 'ul',
  SVG: 'svg',
  VAR: 'var',
  WBR: 'wbr',
  XMP: 'xmp',
};

function numericEnum(entries) {
  const output = {};

  for (const [name, value] of entries) {
    output[name] = value;
    output[value] = name;
  }

  return output;
}

export const TAG_ID = numericEnum([
  ['UNKNOWN', 0],
  ['A', 1],
  ['ADDRESS', 2],
  ['ANNOTATION_XML', 3],
  ['APPLET', 4],
  ['AREA', 5],
  ['ARTICLE', 6],
  ['ASIDE', 7],
  ['B', 8],
  ['BASE', 9],
  ['BASEFONT', 10],
  ['BGSOUND', 11],
  ['BIG', 12],
  ['BLOCKQUOTE', 13],
  ['BODY', 14],
  ['BR', 15],
  ['BUTTON', 16],
  ['CAPTION', 17],
  ['CENTER', 18],
  ['CODE', 19],
  ['COL', 20],
  ['COLGROUP', 21],
  ['DD', 22],
  ['DESC', 23],
  ['DETAILS', 24],
  ['DIALOG', 25],
  ['DIR', 26],
  ['DIV', 27],
  ['DL', 28],
  ['DT', 29],
  ['EM', 30],
  ['EMBED', 31],
  ['FIELDSET', 32],
  ['FIGCAPTION', 33],
  ['FIGURE', 34],
  ['FONT', 35],
  ['FOOTER', 36],
  ['FOREIGN_OBJECT', 37],
  ['FORM', 38],
  ['FRAME', 39],
  ['FRAMESET', 40],
  ['H1', 41],
  ['H2', 42],
  ['H3', 43],
  ['H4', 44],
  ['H5', 45],
  ['H6', 46],
  ['HEAD', 47],
  ['HEADER', 48],
  ['HGROUP', 49],
  ['HR', 50],
  ['HTML', 51],
  ['I', 52],
  ['IMG', 53],
  ['IMAGE', 54],
  ['INPUT', 55],
  ['IFRAME', 56],
  ['KEYGEN', 57],
  ['LABEL', 58],
  ['LI', 59],
  ['LINK', 60],
  ['LISTING', 61],
  ['MAIN', 62],
  ['MALIGNMARK', 63],
  ['MARQUEE', 64],
  ['MATH', 65],
  ['MENU', 66],
  ['META', 67],
  ['MGLYPH', 68],
  ['MI', 69],
  ['MO', 70],
  ['MN', 71],
  ['MS', 72],
  ['MTEXT', 73],
  ['NAV', 74],
  ['NOBR', 75],
  ['NOFRAMES', 76],
  ['NOEMBED', 77],
  ['NOSCRIPT', 78],
  ['OBJECT', 79],
  ['OL', 80],
  ['OPTGROUP', 81],
  ['OPTION', 82],
  ['P', 83],
  ['PARAM', 84],
  ['PLAINTEXT', 85],
  ['PRE', 86],
  ['RB', 87],
  ['RP', 88],
  ['RT', 89],
  ['RTC', 90],
  ['RUBY', 91],
  ['S', 92],
  ['SCRIPT', 93],
  ['SEARCH', 94],
  ['SECTION', 95],
  ['SELECT', 96],
  ['SOURCE', 97],
  ['SMALL', 98],
  ['SPAN', 99],
  ['STRIKE', 100],
  ['STRONG', 101],
  ['STYLE', 102],
  ['SUB', 103],
  ['SUMMARY', 104],
  ['SUP', 105],
  ['TABLE', 106],
  ['TBODY', 107],
  ['TEMPLATE', 108],
  ['TEXTAREA', 109],
  ['TFOOT', 110],
  ['TD', 111],
  ['TH', 112],
  ['THEAD', 113],
  ['TITLE', 114],
  ['TR', 115],
  ['TRACK', 116],
  ['TT', 117],
  ['U', 118],
  ['UL', 119],
  ['SVG', 120],
  ['VAR', 121],
  ['WBR', 122],
  ['XMP', 123],
]);

const TAG_NAME_TO_ID = new Map(Object.values(TAG_NAMES).map((tagName) => [tagName, TAG_ID[tagNameToEnumName(tagName)]]));

function tagNameToEnumName(tagName) {
  if (tagName === 'annotation-xml') {
    return 'ANNOTATION_XML';
  }

  if (tagName === 'foreignObject') {
    return 'FOREIGN_OBJECT';
  }

  return tagName.replaceAll('-', '_').toUpperCase();
}

export function getTagID(tagName) {
  return TAG_NAME_TO_ID.get(tagName) ?? TAG_ID.UNKNOWN;
}

const $ = TAG_ID;

export const SPECIAL_ELEMENTS = {
  [NS.HTML]: new Set([
    $.ADDRESS,
    $.APPLET,
    $.AREA,
    $.ARTICLE,
    $.ASIDE,
    $.BASE,
    $.BASEFONT,
    $.BGSOUND,
    $.BLOCKQUOTE,
    $.BODY,
    $.BR,
    $.BUTTON,
    $.CAPTION,
    $.CENTER,
    $.COL,
    $.COLGROUP,
    $.DD,
    $.DETAILS,
    $.DIR,
    $.DIV,
    $.DL,
    $.DT,
    $.EMBED,
    $.FIELDSET,
    $.FIGCAPTION,
    $.FIGURE,
    $.FOOTER,
    $.FORM,
    $.FRAME,
    $.FRAMESET,
    $.H1,
    $.H2,
    $.H3,
    $.H4,
    $.H5,
    $.H6,
    $.HEAD,
    $.HEADER,
    $.HGROUP,
    $.HR,
    $.HTML,
    $.IFRAME,
    $.IMG,
    $.INPUT,
    $.LI,
    $.LINK,
    $.LISTING,
    $.MAIN,
    $.MARQUEE,
    $.MENU,
    $.META,
    $.NAV,
    $.NOEMBED,
    $.NOFRAMES,
    $.NOSCRIPT,
    $.OBJECT,
    $.OL,
    $.P,
    $.PARAM,
    $.PLAINTEXT,
    $.PRE,
    $.SCRIPT,
    $.SECTION,
    $.SELECT,
    $.SOURCE,
    $.STYLE,
    $.SUMMARY,
    $.TABLE,
    $.TBODY,
    $.TD,
    $.TEMPLATE,
    $.TEXTAREA,
    $.TFOOT,
    $.TH,
    $.THEAD,
    $.TITLE,
    $.TR,
    $.TRACK,
    $.UL,
    $.WBR,
    $.XMP,
  ]),
  [NS.MATHML]: new Set([$.MI, $.MO, $.MN, $.MS, $.MTEXT, $.ANNOTATION_XML]),
  [NS.SVG]: new Set([$.TITLE, $.FOREIGN_OBJECT, $.DESC]),
  [NS.XLINK]: new Set(),
  [NS.XML]: new Set(),
  [NS.XMLNS]: new Set(),
};

export const NUMBERED_HEADERS = new Set([$.H1, $.H2, $.H3, $.H4, $.H5, $.H6]);

const UNESCAPED_TEXT = new Set([
  TAG_NAMES.STYLE,
  TAG_NAMES.SCRIPT,
  TAG_NAMES.XMP,
  TAG_NAMES.IFRAME,
  TAG_NAMES.NOEMBED,
  TAG_NAMES.NOFRAMES,
  TAG_NAMES.PLAINTEXT,
]);

export function hasUnescapedText(tagName, scriptingEnabled) {
  return UNESCAPED_TEXT.has(tagName) || (scriptingEnabled && tagName === TAG_NAMES.NOSCRIPT);
}

const MIME_TYPES = {
  TEXT_HTML: 'text/html',
  APPLICATION_XML: 'application/xhtml+xml',
};

const SVG_ATTRS_ADJUSTMENT_MAP = new Map(
  [
    'attributeName',
    'attributeType',
    'baseFrequency',
    'baseProfile',
    'calcMode',
    'clipPathUnits',
    'diffuseConstant',
    'edgeMode',
    'filterUnits',
    'glyphRef',
    'gradientTransform',
    'gradientUnits',
    'kernelMatrix',
    'kernelUnitLength',
    'keyPoints',
    'keySplines',
    'keyTimes',
    'lengthAdjust',
    'limitingConeAngle',
    'markerHeight',
    'markerUnits',
    'markerWidth',
    'maskContentUnits',
    'maskUnits',
    'numOctaves',
    'pathLength',
    'patternContentUnits',
    'patternTransform',
    'patternUnits',
    'pointsAtX',
    'pointsAtY',
    'pointsAtZ',
    'preserveAlpha',
    'preserveAspectRatio',
    'primitiveUnits',
    'refX',
    'refY',
    'repeatCount',
    'repeatDur',
    'requiredExtensions',
    'requiredFeatures',
    'specularConstant',
    'specularExponent',
    'spreadMethod',
    'startOffset',
    'stdDeviation',
    'stitchTiles',
    'surfaceScale',
    'systemLanguage',
    'tableValues',
    'targetX',
    'targetY',
    'textLength',
    'viewBox',
    'viewTarget',
    'xChannelSelector',
    'yChannelSelector',
    'zoomAndPan',
  ].map((attr) => [attr.toLowerCase(), attr])
);

const XML_ATTRS_ADJUSTMENT_MAP = new Map([
  ['xlink:actuate', { prefix: 'xlink', name: 'actuate', namespace: NS.XLINK }],
  ['xlink:arcrole', { prefix: 'xlink', name: 'arcrole', namespace: NS.XLINK }],
  ['xlink:href', { prefix: 'xlink', name: 'href', namespace: NS.XLINK }],
  ['xlink:role', { prefix: 'xlink', name: 'role', namespace: NS.XLINK }],
  ['xlink:show', { prefix: 'xlink', name: 'show', namespace: NS.XLINK }],
  ['xlink:title', { prefix: 'xlink', name: 'title', namespace: NS.XLINK }],
  ['xlink:type', { prefix: 'xlink', name: 'type', namespace: NS.XLINK }],
  ['xml:lang', { prefix: 'xml', name: 'lang', namespace: NS.XML }],
  ['xml:space', { prefix: 'xml', name: 'space', namespace: NS.XML }],
  ['xmlns', { prefix: '', name: 'xmlns', namespace: NS.XMLNS }],
  ['xmlns:xlink', { prefix: 'xmlns', name: 'xlink', namespace: NS.XMLNS }],
]);

const SVG_TAG_NAMES_ADJUSTMENT_MAP = new Map(
  [
    'altGlyph',
    'altGlyphDef',
    'altGlyphItem',
    'animateColor',
    'animateMotion',
    'animateTransform',
    'clipPath',
    'feBlend',
    'feColorMatrix',
    'feComponentTransfer',
    'feComposite',
    'feConvolveMatrix',
    'feDiffuseLighting',
    'feDisplacementMap',
    'feDistantLight',
    'feFlood',
    'feFuncA',
    'feFuncB',
    'feFuncG',
    'feFuncR',
    'feGaussianBlur',
    'feImage',
    'feMerge',
    'feMergeNode',
    'feMorphology',
    'feOffset',
    'fePointLight',
    'feSpecularLighting',
    'feSpotLight',
    'feTile',
    'feTurbulence',
    'foreignObject',
    'glyphRef',
    'linearGradient',
    'radialGradient',
    'textPath',
  ].map((tagName) => [tagName.toLowerCase(), tagName])
);

const EXITS_FOREIGN_CONTENT = new Set([
  $.B,
  $.BIG,
  $.BLOCKQUOTE,
  $.BODY,
  $.BR,
  $.CENTER,
  $.CODE,
  $.DD,
  $.DIV,
  $.DL,
  $.DT,
  $.EM,
  $.EMBED,
  $.H1,
  $.H2,
  $.H3,
  $.H4,
  $.H5,
  $.H6,
  $.HEAD,
  $.HR,
  $.I,
  $.IMG,
  $.LI,
  $.LISTING,
  $.MENU,
  $.META,
  $.NOBR,
  $.OL,
  $.P,
  $.PRE,
  $.RUBY,
  $.S,
  $.SMALL,
  $.SPAN,
  $.STRONG,
  $.STRIKE,
  $.SUB,
  $.SUP,
  $.TABLE,
  $.TT,
  $.U,
  $.UL,
  $.VAR,
]);

function causesExit(startTagToken) {
  const tagID = startTagToken.tagID;
  const isFontWithAttrs =
    tagID === $.FONT &&
    startTagToken.attrs.some(({ name }) => name === ATTRS.COLOR || name === ATTRS.SIZE || name === ATTRS.FACE);

  return isFontWithAttrs || EXITS_FOREIGN_CONTENT.has(tagID);
}

function adjustTokenMathMLAttrs(token) {
  for (let i = 0; i < token.attrs.length; i++) {
    if (token.attrs[i].name === 'definitionurl') {
      token.attrs[i].name = 'definitionURL';
      break;
    }
  }
}

function adjustTokenSVGAttrs(token) {
  for (let i = 0; i < token.attrs.length; i++) {
    const adjustedAttrName = SVG_ATTRS_ADJUSTMENT_MAP.get(token.attrs[i].name);

    if (adjustedAttrName != null) {
      token.attrs[i].name = adjustedAttrName;
    }
  }
}

function adjustTokenXMLAttrs(token) {
  for (let i = 0; i < token.attrs.length; i++) {
    const adjustedAttrEntry = XML_ATTRS_ADJUSTMENT_MAP.get(token.attrs[i].name);

    if (adjustedAttrEntry) {
      token.attrs[i].prefix = adjustedAttrEntry.prefix;
      token.attrs[i].name = adjustedAttrEntry.name;
      token.attrs[i].namespace = adjustedAttrEntry.namespace;
    }
  }
}

function adjustTokenSVGTagName(token) {
  const adjustedTagName = SVG_TAG_NAMES_ADJUSTMENT_MAP.get(token.tagName);

  if (adjustedTagName != null) {
    token.tagName = adjustedTagName;
    token.tagID = getTagID(token.tagName);
  }
}

function isMathMLTextIntegrationPoint(tagID, namespaceURI) {
  return namespaceURI === NS.MATHML && (tagID === $.MI || tagID === $.MO || tagID === $.MN || tagID === $.MS || tagID === $.MTEXT);
}

function isHtmlIntegrationPoint(tagID, namespaceURI, attrs) {
  if (namespaceURI === NS.MATHML && tagID === $.ANNOTATION_XML) {
    for (let i = 0; i < attrs.length; i++) {
      if (attrs[i].name === ATTRS.ENCODING) {
        const value = attrs[i].value.toLowerCase();
        return value === MIME_TYPES.TEXT_HTML || value === MIME_TYPES.APPLICATION_XML;
      }
    }
  }

  return namespaceURI === NS.SVG && (tagID === $.FOREIGN_OBJECT || tagID === $.DESC || tagID === $.TITLE);
}

function isIntegrationPoint(tagID, namespaceURI, attrs, foreignNamespaceURI) {
  return (
    ((!foreignNamespaceURI || foreignNamespaceURI === NS.HTML) && isHtmlIntegrationPoint(tagID, namespaceURI, attrs)) ||
    ((!foreignNamespaceURI || foreignNamespaceURI === NS.MATHML) && isMathMLTextIntegrationPoint(tagID, namespaceURI))
  );
}

export const foreignContent = {
  SVG_TAG_NAMES_ADJUSTMENT_MAP,
  causesExit,
  adjustTokenMathMLAttrs,
  adjustTokenSVGAttrs,
  adjustTokenXMLAttrs,
  adjustTokenSVGTagName,
  isIntegrationPoint,
};

export const ErrorCodes = {
  controlCharacterInInputStream: 'control-character-in-input-stream',
  noncharacterInInputStream: 'noncharacter-in-input-stream',
  surrogateInInputStream: 'surrogate-in-input-stream',
  nonVoidHtmlElementStartTagWithTrailingSolidus: 'non-void-html-element-start-tag-with-trailing-solidus',
  endTagWithAttributes: 'end-tag-with-attributes',
  endTagWithTrailingSolidus: 'end-tag-with-trailing-solidus',
  unexpectedSolidusInTag: 'unexpected-solidus-in-tag',
  unexpectedNullCharacter: 'unexpected-null-character',
  unexpectedQuestionMarkInsteadOfTagName: 'unexpected-question-mark-instead-of-tag-name',
  invalidFirstCharacterOfTagName: 'invalid-first-character-of-tag-name',
  unexpectedEqualsSignBeforeAttributeName: 'unexpected-equals-sign-before-attribute-name',
  missingEndTagName: 'missing-end-tag-name',
  unexpectedCharacterInAttributeName: 'unexpected-character-in-attribute-name',
  unknownNamedCharacterReference: 'unknown-named-character-reference',
  missingSemicolonAfterCharacterReference: 'missing-semicolon-after-character-reference',
  unexpectedCharacterAfterDoctypeSystemIdentifier: 'unexpected-character-after-doctype-system-identifier',
  unexpectedCharacterInUnquotedAttributeValue: 'unexpected-character-in-unquoted-attribute-value',
  eofBeforeTagName: 'eof-before-tag-name',
  eofInTag: 'eof-in-tag',
  missingAttributeValue: 'missing-attribute-value',
  missingWhitespaceBetweenAttributes: 'missing-whitespace-between-attributes',
  missingWhitespaceAfterDoctypePublicKeyword: 'missing-whitespace-after-doctype-public-keyword',
  missingWhitespaceBetweenDoctypePublicAndSystemIdentifiers:
    'missing-whitespace-between-doctype-public-and-system-identifiers',
  missingWhitespaceAfterDoctypeSystemKeyword: 'missing-whitespace-after-doctype-system-keyword',
  missingQuoteBeforeDoctypePublicIdentifier: 'missing-quote-before-doctype-public-identifier',
  missingQuoteBeforeDoctypeSystemIdentifier: 'missing-quote-before-doctype-system-identifier',
  missingDoctypePublicIdentifier: 'missing-doctype-public-identifier',
  missingDoctypeSystemIdentifier: 'missing-doctype-system-identifier',
  abruptDoctypePublicIdentifier: 'abrupt-doctype-public-identifier',
  abruptDoctypeSystemIdentifier: 'abrupt-doctype-system-identifier',
  cdataInHtmlContent: 'cdata-in-html-content',
  incorrectlyOpenedComment: 'incorrectly-opened-comment',
  eofInScriptHtmlCommentLikeText: 'eof-in-script-html-comment-like-text',
  eofInDoctype: 'eof-in-doctype',
  nestedComment: 'nested-comment',
  abruptClosingOfEmptyComment: 'abrupt-closing-of-empty-comment',
  eofInComment: 'eof-in-comment',
  incorrectlyClosedComment: 'incorrectly-closed-comment',
  eofInCdata: 'eof-in-cdata',
  absenceOfDigitsInNumericCharacterReference: 'absence-of-digits-in-numeric-character-reference',
  nullCharacterReference: 'null-character-reference',
  surrogateCharacterReference: 'surrogate-character-reference',
  characterReferenceOutsideUnicodeRange: 'character-reference-outside-unicode-range',
  controlCharacterReference: 'control-character-reference',
  noncharacterCharacterReference: 'noncharacter-character-reference',
  missingWhitespaceBeforeDoctypeName: 'missing-whitespace-before-doctype-name',
  missingDoctypeName: 'missing-doctype-name',
  invalidCharacterSequenceAfterDoctypeName: 'invalid-character-sequence-after-doctype-name',
  duplicateAttribute: 'duplicate-attribute',
  nonConformingDoctype: 'non-conforming-doctype',
  missingDoctype: 'missing-doctype',
  misplacedDoctype: 'misplaced-doctype',
  endTagWithoutMatchingOpenElement: 'end-tag-without-matching-open-element',
  closingOfElementWithOpenChildElements: 'closing-of-element-with-open-child-elements',
  disallowedContentInNoscriptInHead: 'disallowed-content-in-noscript-in-head',
  openElementsLeftAfterEof: 'open-elements-left-after-eof',
  abandonedHeadElementChild: 'abandoned-head-element-child',
  misplacedStartTagForHeadElement: 'misplaced-start-tag-for-head-element',
  nestedNoscriptInHead: 'nested-noscript-in-head',
  eofInElementThatCanContainOnlyText: 'eof-in-element-that-can-contain-only-text',
};

const TokenType = {
  CHARACTER: 0,
  NULL_CHARACTER: 1,
  WHITESPACE_CHARACTER: 2,
  START_TAG: 3,
  END_TAG: 4,
  COMMENT: 5,
  DOCTYPE: 6,
  EOF: 7,
  HIBERNATION: 8,
};

export const TokenizerMode = {
  DATA: 0,
  RCDATA: 1,
  RAWTEXT: 2,
  SCRIPT_DATA: 3,
  PLAINTEXT: 4,
  CDATA_SECTION: 68,
};

const VOID_ELEMENTS = new Set([
  'area',
  'base',
  'basefont',
  'bgsound',
  'br',
  'col',
  'embed',
  'frame',
  'hr',
  'img',
  'input',
  'keygen',
  'link',
  'meta',
  'param',
  'source',
  'track',
  'wbr',
]);

function ensureWasm() {
  if (!wasmModule) {
    throw new Error('parse5-wasm is not initialized. Call await initParse5Wasm() before parsing.');
  }

  return wasmModule;
}

export async function initParse5Wasm() {
  if (!wasmModule) {
    const wasm = await import('../pkg/parse5_wasm.js');
    if (typeof wasm.default === 'function') {
      await wasm.default();
    }
    wasmModule = wasm;
  }

  return wasmModule;
}

function normalizeParserOptions(options) {
  const normalized = {};

  if (options?.scriptingEnabled !== undefined) {
    normalized.scriptingEnabled = options.scriptingEnabled;
  }

  if (options?.sourceCodeLocationInfo || typeof options?.onParseError === 'function') {
    normalized.sourceCodeLocationInfo = true;
  }

  return normalized;
}

function normalizeTokenizerOptions(options) {
  const normalized = {};

  if (options?.sourceCodeLocationInfo) {
    normalized.sourceCodeLocationInfo = true;
  }

  return normalized;
}

function reportErrors(errors, options) {
  if (typeof options?.onParseError === 'function') {
    for (const error of errors) {
      options.onParseError(error);
    }
  }
}

function reportTokenizerErrors(errors, handler) {
  if (typeof handler?.onParseError === 'function') {
    for (const error of errors) {
      handler.onParseError(error);
    }
  }
}

function attachParentLinks(node, parentNode = null) {
  if (!node || typeof node !== 'object') {
    return node;
  }

  if ('parentNode' in node) {
    node.parentNode = parentNode;
  }

  if (Array.isArray(node.childNodes)) {
    for (const child of node.childNodes) {
      attachParentLinks(child, node);
    }
  }

  if (node.nodeName === 'template' && node.namespaceURI === NS.HTML && node.content) {
    attachParentLinks(node.content, null);
  }

  return node;
}

function parseResult(result, options) {
  reportErrors(result.errors ?? [], options);
  return attachParentLinks(result.node);
}

function hasCustomTreeAdapter(options) {
  return options?.treeAdapter && options.treeAdapter !== defaultTreeAdapter;
}

function cloneAttributes(attrs) {
  return (attrs ?? []).map((attr) => ({ ...attr }));
}

function hasSourceCodeLocation(node) {
  return Object.prototype.hasOwnProperty.call(node, 'sourceCodeLocation');
}

function setAdaptedSourceCodeLocation(adapter, node, sourceNode) {
  if (hasSourceCodeLocation(sourceNode)) {
    adapter.setNodeSourceCodeLocation(node, sourceNode.sourceCodeLocation);
  }
}

function replaceObject(target, source) {
  for (const key of Object.keys(target)) {
    delete target[key];
  }

  Object.assign(target, source);
  return target;
}

function pathKey(path) {
  return path.join('/');
}

function nodeAtPath(root, path) {
  let node = root;

  for (const segment of path ?? []) {
    node = segment === 'content'
      ? node?.content ?? null
      : node?.childNodes?.[segment] ?? null;
  }

  return node;
}

function transformDocumentType(adapter, parentNode, sourceNode, pathMap, path) {
  adapter.setDocumentType(parentNode, sourceNode.name, sourceNode.publicId, sourceNode.systemId);

  const doctype = adapter.getChildNodes(parentNode).find((node) => adapter.isDocumentTypeNode(node));

  if (doctype) {
    pathMap.set(pathKey(path), doctype);
    setAdaptedSourceCodeLocation(adapter, doctype, sourceNode);
  }

  return doctype ?? null;
}

function appendTransformedChild(adapter, parentNode, sourceNode, pathMap, path) {
  if (sourceNode.nodeName === '#documentType') {
    transformDocumentType(adapter, parentNode, sourceNode, pathMap, path);
    return;
  }

  adapter.appendChild(parentNode, transformDefaultTreeNode(adapter, sourceNode, null, path, pathMap));
}

function transformDefaultTreeNode(adapter, sourceNode, targetDocument = null, path = [], pathMap = new Map()) {
  switch (sourceNode.nodeName) {
    case '#document': {
      const document = targetDocument ?? adapter.createDocument();
      pathMap.set(pathKey(path), document);
      adapter.setDocumentMode(document, sourceNode.mode);

      for (const [idx, child] of (sourceNode.childNodes ?? []).entries()) {
        appendTransformedChild(adapter, document, child, pathMap, [...path, idx]);
      }

      setAdaptedSourceCodeLocation(adapter, document, sourceNode);
      return document;
    }
    case '#document-fragment': {
      const fragment = adapter.createDocumentFragment();
      pathMap.set(pathKey(path), fragment);

      for (const [idx, child] of (sourceNode.childNodes ?? []).entries()) {
        appendTransformedChild(adapter, fragment, child, pathMap, [...path, idx]);
      }

      setAdaptedSourceCodeLocation(adapter, fragment, sourceNode);
      return fragment;
    }
    case '#comment': {
      const comment = adapter.createCommentNode(sourceNode.data);
      pathMap.set(pathKey(path), comment);
      setAdaptedSourceCodeLocation(adapter, comment, sourceNode);
      return comment;
    }
    case '#text': {
      const text = adapter.createTextNode(sourceNode.value);
      pathMap.set(pathKey(path), text);
      setAdaptedSourceCodeLocation(adapter, text, sourceNode);
      return text;
    }
    default: {
      const element = adapter.createElement(sourceNode.tagName, sourceNode.namespaceURI, cloneAttributes(sourceNode.attrs));
      pathMap.set(pathKey(path), element);

      if (sourceNode.nodeName === 'template' && sourceNode.namespaceURI === NS.HTML && sourceNode.content) {
        adapter.setTemplateContent(
          element,
          transformDefaultTreeNode(adapter, sourceNode.content, null, [...path, 'content'], pathMap)
        );
      }

      for (const [idx, child] of (sourceNode.childNodes ?? []).entries()) {
        appendTransformedChild(adapter, element, child, pathMap, [...path, idx]);
      }

      setAdaptedSourceCodeLocation(adapter, element, sourceNode);
      return element;
    }
  }
}

function adaptParsedResult(node, options, targetDocument = null) {
  if (hasCustomTreeAdapter(options)) {
    const pathMap = new Map();
    return {
      node: transformDefaultTreeNode(options.treeAdapter, node, targetDocument, [], pathMap),
      pathMap,
    };
  }

  const adaptedNode = targetDocument ? attachParentLinks(replaceObject(targetDocument, node)) : attachParentLinks(node);
  return {
    node: adaptedNode,
    pathMap: null,
  };
}

function adaptParsedNode(node, options, targetDocument = null) {
  return adaptParsedResult(node, options, targetDocument).node;
}

function replayStackEvents(adapter, pathMap, events) {
  if (!pathMap || !Array.isArray(events)) {
    return;
  }

  for (const event of events) {
    const node = pathMap.get(pathKey(event.nodePath ?? []));

    if (!node) {
      continue;
    }

    if (event.type === 'push') {
      adapter.onItemPush?.(node);
    } else if (event.type === 'pop') {
      const newTop = event.newTopPath === null ? null : pathMap.get(pathKey(event.newTopPath ?? []));
      adapter.onItemPop?.(node, newTop);
    }
  }
}

function fragmentContextDescriptor(node, treeAdapter = defaultTreeAdapter) {
  if (!node || !treeAdapter.isElementNode(node)) {
    return null;
  }

  return {
    tagName: treeAdapter.getTagName(node),
    namespaceURI: treeAdapter.getNamespaceURI(node) ?? NS.HTML,
    attrs: treeAdapter.getAttrList(node) ?? [],
    parentNode: fragmentContextDescriptor(treeAdapter.getParentNode(node), treeAdapter),
  };
}

export class Parser {
  #native;
  #externalDocument;
  #pathMap = null;
  #currentWriteCallback = null;
  #currentWriteCallbackCalled = false;
  #handlingScriptEvent = false;

  constructor(options = {}, document = null, fragmentContext = null, scriptHandler = null, isFragmentParser = false) {
    const wasm = ensureWasm();

    this.options = {
      scriptingEnabled: options?.scriptingEnabled ?? true,
      sourceCodeLocationInfo: Boolean(options?.sourceCodeLocationInfo || options?.onParseError),
      treeAdapter: options?.treeAdapter ?? defaultTreeAdapter,
      onParseError: options?.onParseError ?? null,
    };
    this.treeAdapter = this.options.treeAdapter;
    this.onParseError = this.options.onParseError;
    this.fragmentContext = fragmentContext;
    this.scriptHandler = scriptHandler;
    this.stopped = false;
    this.#externalDocument = document;

    this.#native = new wasm.WasmParser(
      normalizeParserOptions(this.options),
      fragmentContextDescriptor(fragmentContext, this.treeAdapter),
      Boolean(isFragmentParser)
    );
    const adapted = adaptParsedResult(this.#native.document(), this.options, document);
    this.document = adapted.node;
    this.#pathMap = adapted.pathMap;
    this.tokenizer = {
      preprocessor: {
        bufferWaterline: 1 << 16,
      },
      paused: false,
      active: false,
      pause: () => {
        this.tokenizer.paused = true;
      },
      resume: (writeCallback) => {
        if (!this.tokenizer.paused) {
          throw new Error('Parser was already resumed');
        }

        const resumeCalledDuringScriptEvent = this.#handlingScriptEvent;
        this.tokenizer.paused = false;
        this.#syncNativeTokenizerSettings();
        this.#handleParserResult(this.#native.resume());

        if (!resumeCalledDuringScriptEvent) {
          this.#maybeCallWriteCallback(writeCallback);
        }
      },
      write: (chunk, isLastChunk, writeCallback) => {
        this.#currentWriteCallback = writeCallback ?? null;
        this.#currentWriteCallbackCalled = false;
        this.#syncNativeTokenizerSettings();
        this.tokenizer.active = true;
        this.#handleParserResult(this.#native.write(String(chunk), Boolean(isLastChunk)));
        this.#maybeCallWriteCallback(writeCallback);
      },
      insertHtmlAtCurrentPos: (chunk) => {
        this.#syncNativeTokenizerSettings();
        this.tokenizer.active = true;
        this.#handleParserResult(this.#native.insertHtmlAtCurrentPos(String(chunk)));
      },
    };
  }

  #syncNativeTokenizerSettings() {
    this.#native.setScriptHandlerEnabled(typeof this.scriptHandler === 'function');

    if (Number.isFinite(this.tokenizer?.preprocessor?.bufferWaterline)) {
      this.#native.setBufferWaterline(Math.max(0, Math.trunc(this.tokenizer.preprocessor.bufferWaterline)));
    }
  }

  #handleParserResult(result) {
    reportErrors(result.errors ?? [], this.options);
    this.stopped = this.#native.stopped();
    this.tokenizer.active = !this.stopped;

    const adapted = adaptParsedResult(this.#native.document(), this.options, this.#externalDocument);
    this.document = adapted.node;
    this.#pathMap = adapted.pathMap;
    replayStackEvents(this.treeAdapter, this.#pathMap, result.stackEvents);

    for (const event of result.scriptEvents ?? []) {
      const scriptNode = this.#pathMap
        ? this.#pathMap.get(pathKey(event.nodePath ?? []))
        : nodeAtPath(this.document, event.nodePath ?? []);

      if (scriptNode) {
        this.#handlingScriptEvent = true;

        try {
          this.scriptHandler?.(scriptNode);
        } finally {
          this.#handlingScriptEvent = false;
        }
      }

      if (this.tokenizer.paused) {
        this.tokenizer.active = true;
        return;
      }

      if (this.#native.pausedOnScript()) {
        this.#handleParserResult(this.#native.resume());
      }

      if (this.tokenizer.paused) {
        return;
      }
    }
  }

  #maybeCallWriteCallback(writeCallback) {
    if (this.tokenizer.paused || typeof writeCallback !== 'function') {
      return;
    }

    if (writeCallback === this.#currentWriteCallback) {
      if (this.#currentWriteCallbackCalled) {
        return;
      }

      this.#currentWriteCallbackCalled = true;
    }

    writeCallback();
  }

  initPlainTextConversion() {
    this.#handleParserResult(this.#native.initPlainTextConversion());
  }

  static parse(html, options) {
    const parser = new this(options);
    parser.tokenizer.write(html, true);
    return parser.document;
  }

  static getFragmentParser(fragmentContext = null, options) {
    return new this(options, null, fragmentContext, null, true);
  }

  getFragment() {
    return adaptParsedNode(this.#native.getFragment(), this.options);
  }
}

export function parse(html, options = {}) {
  const wasm = ensureWasm();

  if (hasCustomTreeAdapter(options)) {
    return Parser.parse(html, options);
  }

  if (typeof options?.onParseError === 'function') {
    const result = wasm.parseWithErrors(String(html), normalizeParserOptions(options));
    reportErrors(result.errors ?? [], options);
    return adaptParsedNode(result.node, options);
  }

  return adaptParsedNode(wasm.parse(String(html), normalizeParserOptions(options)), options);
}

export function parseFragment(fragmentContext, html, options) {
  const wasm = ensureWasm();

  if (typeof fragmentContext === 'string') {
    options = html ?? {};
    html = fragmentContext;
    fragmentContext = null;
  }

  if (hasCustomTreeAdapter(options)) {
    const parser = Parser.getFragmentParser(fragmentContext, options);
    parser.tokenizer.write(html, true);
    return parser.getFragment();
  }

  const context = fragmentContextDescriptor(fragmentContext);

  if (typeof options?.onParseError === 'function') {
    const result =
      context === null
        ? wasm.parseFragmentWithErrors(String(html), normalizeParserOptions(options))
        : wasm.parseFragmentWithContextAndErrors(context, String(html), normalizeParserOptions(options));
    reportErrors(result.errors ?? [], options);
    return adaptParsedNode(result.node, options);
  }

  const fragment =
    context === null
      ? wasm.parseFragment(String(html), normalizeParserOptions(options))
      : wasm.parseFragmentWithContext(context, String(html), normalizeParserOptions(options));
  return adaptParsedNode(fragment, options);
}

function assertKnownTokenizerState(state) {
  if (!Number.isInteger(state) || state < 0 || state > 72) {
    throw new Error('Unknown state');
  }
}

function callTokenHandler(handler, token) {
  switch (token.type) {
    case TokenType.CHARACTER:
      handler.onCharacter?.(token);
      break;
    case TokenType.NULL_CHARACTER:
      handler.onNullCharacter?.(token);
      break;
    case TokenType.WHITESPACE_CHARACTER:
      handler.onWhitespaceCharacter?.(token);
      break;
    case TokenType.START_TAG:
      handler.onStartTag?.(token);
      break;
    case TokenType.END_TAG:
      handler.onEndTag?.(token);
      break;
    case TokenType.COMMENT:
      handler.onComment?.(token);
      break;
    case TokenType.DOCTYPE:
      handler.onDoctype?.(token);
      break;
    case TokenType.EOF:
      handler.onEof?.(token);
      break;
  }
}

export class Tokenizer {
  #native;
  #handler;
  #preprocessorHtml = '';
  #preprocessorDroppedBufferSize = 0;
  #preprocessorPos = -1;

  constructor(options = {}, handler = {}) {
    const wasm = ensureWasm();
    const tokenizer = this;

    this.preprocessor = {
      bufferWaterline: 1 << 16,
      get html() {
        return tokenizer.#preprocessorHtml;
      },
      get droppedBufferSize() {
        return tokenizer.#preprocessorDroppedBufferSize;
      },
      willDropParsedChunk() {
        return tokenizer.#preprocessorPos > this.bufferWaterline;
      },
    };
    this.paused = false;
    this.inForeignNode = false;
    this.lastStartTagName = '';
    this.active = false;
    this.state = TokenizerMode.DATA;

    this.#handler = handler;
    this.#native = new wasm.WasmTokenizer(normalizeTokenizerOptions(options));
  }

  pause() {
    this.paused = true;
  }

  resume(writeCallback) {
    if (!this.paused) {
      throw new Error('Parser was already resumed');
    }

    this.paused = false;
    this.#pump();

    if (!this.paused) {
      writeCallback?.();
    }
  }

  write(chunk, isLastChunk, writeCallback) {
    const input = String(chunk);

    this.active = true;
    this.#syncNativeState();
    this.#preprocessorHtml += input;
    this.#native.writeInput(input, Boolean(isLastChunk));

    if (!this.paused) {
      this.#pump();
    }

    if (!this.paused) {
      writeCallback?.();
    }
  }

  insertHtmlAtCurrentPos(chunk) {
    const input = String(chunk);
    const insertionPos = Math.max(0, this.#preprocessorPos + 1);

    this.active = true;
    this.#syncNativeState();
    this.#preprocessorHtml =
      this.#preprocessorHtml.slice(0, insertionPos) + input + this.#preprocessorHtml.slice(insertionPos);
    this.#native.insertHtmlAtCurrentPosInput(input);

    if (!this.paused) {
      this.#pump();
    }
  }

  #syncNativeState() {
    assertKnownTokenizerState(this.state);
    this.#native.setState(this.state);
    this.#native.setLastStartTagName(String(this.lastStartTagName));
    this.#native.setInForeignNode(Boolean(this.inForeignNode));

    if (Number.isFinite(this.preprocessor?.bufferWaterline)) {
      this.#native.setBufferWaterline(Math.max(0, Math.trunc(this.preprocessor.bufferWaterline)));
    }
  }

  #updatePreprocessorForToken(token) {
    const endOffset = token?.location?.endOffset;

    if (Number.isFinite(endOffset)) {
      this.#preprocessorPos = Math.max(0, Math.trunc(endOffset - this.#preprocessorDroppedBufferSize - 1));
    }
  }

  #dropParsedChunkAfterToken() {
    if (!this.preprocessor.willDropParsedChunk()) {
      return;
    }

    this.#preprocessorHtml = this.#preprocessorHtml.substring(this.#preprocessorPos);
    this.#preprocessorDroppedBufferSize += this.#preprocessorPos;
    this.#preprocessorPos = 0;
  }

  #syncStateFromNative() {
    this.state = this.#native.getState();
    this.lastStartTagName = this.#native.getLastStartTagName();
    this.inForeignNode = this.#native.getInForeignNode();
    this.active = this.#native.active();
  }

  #pump() {
    while (!this.paused) {
      this.#syncNativeState();
      const result = this.#native.nextToken();
      reportTokenizerErrors(result.errors ?? [], this.#handler);
      this.#syncStateFromNative();

      const token = result.token;
      if (token == null) {
        break;
      }

      this.#updatePreprocessorForToken(token);
      callTokenHandler(this.#handler, token);
      this.#dropParsedChunkAfterToken();
    }
  }
}

function isParentNode(node) {
  return (
    node?.nodeName === '#document' ||
    node?.nodeName === '#document-fragment' ||
    (typeof node?.tagName === 'string' && Array.isArray(node.childNodes))
  );
}

function isElementNode(node) {
  return typeof node?.tagName === 'string';
}

function isTextNode(node) {
  return node?.nodeName === '#text';
}

function isCommentNode(node) {
  return node?.nodeName === '#comment';
}

function isDocumentTypeNode(node) {
  return node?.nodeName === '#documentType';
}

function isVoidElement(node, options = { treeAdapter: defaultTreeAdapter }) {
  const { treeAdapter } = options;

  return (
    treeAdapter.isElementNode(node) &&
    treeAdapter.getNamespaceURI(node) === NS.HTML &&
    VOID_ELEMENTS.has(treeAdapter.getTagName(node))
  );
}

function escapeText(value) {
  return String(value)
    .replaceAll('&', '&amp;')
    .replaceAll('\u00a0', '&nbsp;')
    .replaceAll('<', '&lt;')
    .replaceAll('>', '&gt;');
}

function escapeAttribute(value) {
  return String(value).replaceAll('&', '&amp;').replaceAll('"', '&quot;').replaceAll('\u00a0', '&nbsp;');
}

function serializeAttribute(attr) {
  let name = attr.name;

  if (attr.namespace === NS.XML) {
    name = `xml:${attr.name}`;
  } else if (attr.namespace === NS.XMLNS) {
    name = attr.name === 'xmlns' ? attr.name : `xmlns:${attr.name}`;
  } else if (attr.namespace === NS.XLINK) {
    name = `xlink:${attr.name}`;
  } else if (attr.namespace) {
    name = `${attr.prefix}:${attr.name}`;
  }

  return ` ${name}="${escapeAttribute(attr.value)}"`;
}

function serializerOptions(options = {}) {
  return {
    treeAdapter: options.treeAdapter ?? defaultTreeAdapter,
    scriptingEnabled: options.scriptingEnabled ?? true,
  };
}

function serializeAttributes(node, options) {
  const { treeAdapter } = options;
  let html = '';

  for (const attr of treeAdapter.getAttrList(node) ?? []) {
    html += serializeAttribute(attr);
  }

  return html;
}

function serializeNode(node, options) {
  const { treeAdapter } = options;

  if (treeAdapter.isElementNode(node)) {
    const tagName = treeAdapter.getTagName(node);
    const children = isVoidElement(node, options) ? '' : `${serialize(node, options)}</${tagName}>`;
    return `<${tagName}${serializeAttributes(node, options)}>${children}`;
  }

  if (treeAdapter.isTextNode(node)) {
    const content = treeAdapter.getTextNodeContent(node);
    const parent = treeAdapter.getParentNode(node);
    const parentTagName = parent && treeAdapter.isElementNode(parent) && treeAdapter.getTagName(parent);
    const rawText =
      parentTagName &&
      treeAdapter.getNamespaceURI(parent) === NS.HTML &&
      hasUnescapedText(parentTagName, options.scriptingEnabled);

    return rawText ? content : escapeText(content);
  }

  if (treeAdapter.isCommentNode(node)) {
    return `<!--${treeAdapter.getCommentNodeContent(node)}-->`;
  }

  if (treeAdapter.isDocumentTypeNode(node)) {
    return `<!DOCTYPE ${treeAdapter.getDocumentTypeNodeName(node)}>`;
  }

  return '';
}

export function serialize(node, options = {}) {
  const opts = serializerOptions(options);
  const { treeAdapter } = opts;

  if (isVoidElement(node, opts)) {
    return '';
  }

  const container =
    treeAdapter.isElementNode(node) &&
    treeAdapter.getTagName(node) === TAG_NAMES.TEMPLATE &&
    treeAdapter.getNamespaceURI(node) === NS.HTML
      ? treeAdapter.getTemplateContent(node)
      : node;

  let html = '';

  for (const child of treeAdapter.getChildNodes(container) ?? []) {
    html += serializeNode(child, opts);
  }

  return html;
}

export function serializeOuter(node, options = {}) {
  return serializeNode(node, serializerOptions(options));
}

export const defaultTreeAdapter = {
  createDocument() {
    return { nodeName: '#document', mode: DOCUMENT_MODE.NO_QUIRKS, childNodes: [] };
  },
  createDocumentFragment() {
    return { nodeName: '#document-fragment', childNodes: [] };
  },
  createElement(tagName, namespaceURI, attrs) {
    return { nodeName: tagName, tagName, attrs, namespaceURI, childNodes: [], parentNode: null };
  },
  createCommentNode(data) {
    return { nodeName: '#comment', data, parentNode: null };
  },
  createTextNode(value) {
    return { nodeName: '#text', value, parentNode: null };
  },
  appendChild(parentNode, newNode) {
    parentNode.childNodes.push(newNode);
    newNode.parentNode = parentNode;
  },
  insertBefore(parentNode, newNode, referenceNode) {
    const idx = parentNode.childNodes.indexOf(referenceNode);
    parentNode.childNodes.splice(idx, 0, newNode);
    newNode.parentNode = parentNode;
  },
  detachNode(node) {
    if (node.parentNode) {
      const idx = node.parentNode.childNodes.indexOf(node);
      node.parentNode.childNodes.splice(idx, 1);
      node.parentNode = null;
    }
  },
  insertText(parentNode, text) {
    const previous = parentNode.childNodes.at(-1);

    if (isTextNode(previous)) {
      previous.value += text;
    } else {
      this.appendChild(parentNode, this.createTextNode(text));
    }
  },
  insertTextBefore(parentNode, text, referenceNode) {
    const previous = parentNode.childNodes[parentNode.childNodes.indexOf(referenceNode) - 1];

    if (isTextNode(previous)) {
      previous.value += text;
    } else {
      this.insertBefore(parentNode, this.createTextNode(text), referenceNode);
    }
  },
  adoptAttributes(recipient, attrs) {
    const recipientAttrs = new Set(recipient.attrs.map((attr) => attr.name));

    for (const attr of attrs) {
      if (!recipientAttrs.has(attr.name)) {
        recipient.attrs.push(attr);
      }
    }
  },
  setTemplateContent(templateElement, contentElement) {
    templateElement.content = contentElement;
  },
  getTemplateContent(templateElement) {
    return templateElement.content;
  },
  setDocumentType(document, name, publicId, systemId) {
    const existing = document.childNodes.find(isDocumentTypeNode);

    if (existing) {
      existing.name = name;
      existing.publicId = publicId;
      existing.systemId = systemId;
      return;
    }

    this.appendChild(document, {
      nodeName: '#documentType',
      name,
      publicId,
      systemId,
      parentNode: null,
    });
  },
  setDocumentMode(document, mode) {
    document.mode = mode;
  },
  getDocumentMode(document) {
    return document.mode;
  },
  getFirstChild(node) {
    return node.childNodes[0] ?? null;
  },
  getChildNodes(node) {
    return node.childNodes;
  },
  getParentNode(node) {
    return node.parentNode ?? null;
  },
  getAttrList(element) {
    return element.attrs;
  },
  getTagName(element) {
    return element.tagName;
  },
  getNamespaceURI(element) {
    return element.namespaceURI;
  },
  getTextNodeContent(node) {
    return node.value;
  },
  getCommentNodeContent(node) {
    return node.data;
  },
  getDocumentTypeNodeName(node) {
    return node.name;
  },
  getDocumentTypeNodePublicId(node) {
    return node.publicId;
  },
  getDocumentTypeNodeSystemId(node) {
    return node.systemId;
  },
  isTextNode,
  isCommentNode,
  isDocumentTypeNode,
  isElementNode,
  setNodeSourceCodeLocation(node, location) {
    node.sourceCodeLocation = location;
  },
  getNodeSourceCodeLocation(node) {
    return node.sourceCodeLocation;
  },
  updateNodeSourceCodeLocation(node, endLocation) {
    node.sourceCodeLocation = { ...node.sourceCodeLocation, ...endLocation };
  },
};

export const html = {
  NS,
  ATTRS,
  DOCUMENT_MODE,
  TAG_NAMES,
  TAG_ID,
  SPECIAL_ELEMENTS,
  NUMBERED_HEADERS,
  getTagID,
  hasUnescapedText,
};

export const Token = {
  TokenType,
  getTokenAttr(token, attrName) {
    for (let i = (token?.attrs?.length ?? 0) - 1; i >= 0; i--) {
      if (token.attrs[i].name === attrName) {
        return token.attrs[i].value;
      }
    }

    return null;
  },
};
