import assert from 'node:assert/strict';

import {
  DOCUMENT_MODE,
  ErrorCodes,
  NS,
  Token,
  Tokenizer,
  TokenizerMode,
  defaultTreeAdapter,
  foreignContent,
  html as htmlNamespace,
  serialize,
  serializeOuter,
} from '../packages/parse5-wasm/src/index.js';

assert.equal(Object.keys(ErrorCodes).length, 60);
assert.equal(ErrorCodes.missingDoctype, 'missing-doctype');
assert.equal(DOCUMENT_MODE.NO_QUIRKS, 'no-quirks');
assert.equal(TokenizerMode.DATA, 0);
assert.equal(TokenizerMode.CDATA_SECTION, 68);
assert.equal(Token.TokenType.START_TAG, 3);
assert.equal(Token.getTokenAttr({ attrs: [{ name: 'id', value: 'x' }] }, 'id'), 'x');
assert.deepEqual(htmlNamespace.NS, NS);
assert.deepEqual(htmlNamespace.ATTRS, {
  TYPE: 'type',
  ACTION: 'action',
  ENCODING: 'encoding',
  PROMPT: 'prompt',
  NAME: 'name',
  COLOR: 'color',
  FACE: 'face',
  SIZE: 'size',
});
assert.deepEqual(htmlNamespace.DOCUMENT_MODE, DOCUMENT_MODE);
assert.equal(Object.keys(htmlNamespace.TAG_NAMES).length, 123);
assert.equal(htmlNamespace.TAG_NAMES.FOREIGN_OBJECT, 'foreignObject');
assert.equal(htmlNamespace.TAG_NAMES.TEMPLATE, 'template');
assert.equal(htmlNamespace.TAG_ID.FOREIGN_OBJECT, 37);
assert.equal(htmlNamespace.TAG_ID.TEMPLATE, 108);
assert.equal(htmlNamespace.getTagID('foreignObject'), htmlNamespace.TAG_ID.FOREIGN_OBJECT);
assert.equal(htmlNamespace.hasUnescapedText('noscript', true), true);
assert.equal(htmlNamespace.hasUnescapedText('noscript', false), false);
assert.deepEqual(
  [...htmlNamespace.SPECIAL_ELEMENTS[htmlNamespace.NS.HTML]].sort((a, b) => a - b),
  [
    2, 4, 5, 6, 7, 9, 10, 11, 13, 14, 15, 16, 17, 18, 20, 21, 22, 24, 26, 27, 28, 29, 31, 32,
    33, 34, 36, 38, 39, 40, 41, 42, 43, 44, 45, 46, 47, 48, 49, 50, 51, 53, 55, 56, 59, 60, 61,
    62, 64, 66, 67, 74, 76, 77, 78, 79, 80, 83, 84, 85, 86, 93, 95, 96, 97, 102, 104, 106, 107,
    108, 109, 110, 111, 112, 113, 114, 115, 116, 119, 122, 123,
  ]
);
assert.deepEqual([...htmlNamespace.NUMBERED_HEADERS], [41, 42, 43, 44, 45, 46]);
assert.deepEqual(
  Object.keys(defaultTreeAdapter).sort(),
  [
    'adoptAttributes',
    'appendChild',
    'createCommentNode',
    'createDocument',
    'createDocumentFragment',
    'createElement',
    'createTextNode',
    'detachNode',
    'getAttrList',
    'getChildNodes',
    'getCommentNodeContent',
    'getDocumentMode',
    'getDocumentTypeNodeName',
    'getDocumentTypeNodePublicId',
    'getDocumentTypeNodeSystemId',
    'getFirstChild',
    'getNamespaceURI',
    'getNodeSourceCodeLocation',
    'getParentNode',
    'getTagName',
    'getTemplateContent',
    'getTextNodeContent',
    'insertBefore',
    'insertText',
    'insertTextBefore',
    'isCommentNode',
    'isDocumentTypeNode',
    'isElementNode',
    'isTextNode',
    'setDocumentMode',
    'setDocumentType',
    'setNodeSourceCodeLocation',
    'setTemplateContent',
    'updateNodeSourceCodeLocation',
  ],
  'defaultTreeAdapter should expose the same methods as upstream parse5'
);
assert.throws(
  () => new Tokenizer({}, {}),
  /parse5-wasm is not initialized/,
  'Tokenizer should use the wasm-backed native tokenizer'
);

const fontToken = { tagID: htmlNamespace.TAG_ID.FONT, attrs: [{ name: 'color', value: 'red' }] };
assert.equal(foreignContent.causesExit(fontToken), true);

const mathMlToken = {
  tagID: htmlNamespace.TAG_ID.ANNOTATION_XML,
  tagName: 'annotation-xml',
  attrs: [{ name: 'definitionurl', value: 'x' }],
};
foreignContent.adjustTokenMathMLAttrs(mathMlToken);
assert.deepEqual(mathMlToken.attrs, [{ name: 'definitionURL', value: 'x' }]);

const svgToken = { tagID: 0, tagName: 'foreignobject', attrs: [{ name: 'viewbox', value: '0 0 10 10' }] };
foreignContent.adjustTokenSVGTagName(svgToken);
foreignContent.adjustTokenSVGAttrs(svgToken);
assert.equal(svgToken.tagName, 'foreignObject');
assert.deepEqual(svgToken.attrs, [{ name: 'viewBox', value: '0 0 10 10' }]);

const xmlToken = {
  tagID: htmlNamespace.TAG_ID.SVG,
  tagName: 'svg',
  attrs: [
    { name: 'xlink:href', value: '#x' },
    { name: 'xmlns:xlink', value: htmlNamespace.NS.XLINK },
  ],
};
foreignContent.adjustTokenXMLAttrs(xmlToken);
assert.deepEqual(xmlToken.attrs, [
  { name: 'href', value: '#x', prefix: 'xlink', namespace: htmlNamespace.NS.XLINK },
  { name: 'xlink', value: htmlNamespace.NS.XLINK, prefix: 'xmlns', namespace: htmlNamespace.NS.XMLNS },
]);
assert.equal(
  foreignContent.isIntegrationPoint(
    htmlNamespace.TAG_ID.ANNOTATION_XML,
    htmlNamespace.NS.MATHML,
    [{ name: 'encoding', value: 'TEXT/HTML' }],
    htmlNamespace.NS.HTML
  ),
  true
);

const document = defaultTreeAdapter.createDocument();
const doctype = {
  nodeName: '#documentType',
  name: 'html',
  publicId: '',
  systemId: '',
  parentNode: null,
};
const htmlElement = defaultTreeAdapter.createElement('html', NS.HTML, []);
const head = defaultTreeAdapter.createElement('head', NS.HTML, []);
const body = defaultTreeAdapter.createElement('body', NS.HTML, []);
const div = defaultTreeAdapter.createElement('div', NS.HTML, [{ name: 'id', value: 'a&b' }]);
const svg = defaultTreeAdapter.createElement('svg', NS.SVG, []);
const use = defaultTreeAdapter.createElement('use', NS.SVG, [
  { name: 'href', value: '#x', namespace: NS.XLINK, prefix: 'xlink' },
]);
const text = defaultTreeAdapter.createTextNode('hello <world>');
const comment = defaultTreeAdapter.createCommentNode('note');
const br = defaultTreeAdapter.createElement('br', NS.HTML, []);

defaultTreeAdapter.appendChild(document, doctype);
defaultTreeAdapter.appendChild(document, htmlElement);
defaultTreeAdapter.appendChild(htmlElement, head);
defaultTreeAdapter.appendChild(htmlElement, body);
defaultTreeAdapter.appendChild(body, div);
defaultTreeAdapter.appendChild(div, text);
defaultTreeAdapter.appendChild(div, comment);
defaultTreeAdapter.appendChild(body, br);
defaultTreeAdapter.appendChild(body, svg);
defaultTreeAdapter.appendChild(svg, use);

assert.equal(text.parentNode, div);
assert.equal(defaultTreeAdapter.getDocumentMode(document), DOCUMENT_MODE.NO_QUIRKS);
assert.equal(defaultTreeAdapter.isElementNode(div), true);
assert.equal(defaultTreeAdapter.isTextNode(text), true);
defaultTreeAdapter.adoptAttributes(div, [
  { name: 'id', value: 'replacement' },
  { name: 'data-added', value: 'yes' },
]);
assert.deepEqual(div.attrs, [
  { name: 'id', value: 'a&b' },
  { name: 'data-added', value: 'yes' },
]);

assert.equal(
  serialize(document),
  '<!DOCTYPE html><html><head></head><body><div id="a&amp;b" data-added="yes">hello &lt;world&gt;<!--note--></div><br><svg><use xlink:href="#x"></use></svg></body></html>'
);
assert.equal(serialize(body), '<div id="a&amp;b" data-added="yes">hello &lt;world&gt;<!--note--></div><br><svg><use xlink:href="#x"></use></svg>');
assert.equal(serializeOuter(div), '<div id="a&amp;b" data-added="yes">hello &lt;world&gt;<!--note--></div>');

function customSerializerAdapterResult(mod) {
  let getTagNameCalls = 0;
  const adapter = {
    ...mod.defaultTreeAdapter,
    getTagName(element) {
      assert.ok(element.tagName);
      getTagNameCalls++;
      return mod.defaultTreeAdapter.getTagName(element);
    },
  };
  const template = mod.defaultTreeAdapter.createElement('template', mod.html.NS.HTML, []);
  const content = mod.defaultTreeAdapter.createDocumentFragment();
  const div = mod.defaultTreeAdapter.createElement('div', mod.html.NS.HTML, [
    { name: 'data-x', value: 'a&b' },
  ]);

  mod.defaultTreeAdapter.setTemplateContent(template, content);
  mod.defaultTreeAdapter.appendChild(content, mod.defaultTreeAdapter.createTextNode('yo'));
  mod.defaultTreeAdapter.appendChild(content, div);
  mod.defaultTreeAdapter.appendChild(content, mod.defaultTreeAdapter.createTextNode('42'));

  return {
    inner: mod.serialize(template, { treeAdapter: adapter }),
    outer: mod.serializeOuter(template, { treeAdapter: adapter }),
    sawElements: getTagNameCalls > 0,
  };
}

assert.deepEqual(
  customSerializerAdapterResult({ defaultTreeAdapter, html: htmlNamespace, serialize, serializeOuter }),
  {
    inner: 'yo<div data-x="a&amp;b"></div>42',
    outer: '<template>yo<div data-x="a&amp;b"></div>42</template>',
    sawElements: true,
  }
);
assert.equal(serialize({}), '');
assert.equal(serializeOuter({}), '');

console.log('parse5-wasm facade tests passed.');
