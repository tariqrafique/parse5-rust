import assert from 'node:assert/strict';

import * as portParse5 from '../packages/parse5/src/index.js';
import {
  adapter as portAdapter,
  serializeDoctypeContent as portSerializeDoctypeContent,
} from '../packages/parse5-htmlparser2-tree-adapter/src/index.js';

function sortedRecord(record) {
  if (!record || typeof record !== 'object') {
    return record;
  }

  const output = {};

  for (const key of Object.keys(record).sort()) {
    output[key] = record[key];
  }

  return output;
}

function compactLocation(location) {
  if (Array.isArray(location)) {
    return location.map(compactLocation);
  }

  if (!location || typeof location !== 'object') {
    return location;
  }

  const output = {};

  for (const key of Object.keys(location).sort()) {
    output[key] = compactLocation(location[key]);
  }

  return output;
}

function normalizeNode(node) {
  const output = {
    type: node.type,
    nodeType: node.nodeType,
    startIndex: node.startIndex,
    endIndex: node.endIndex,
  };

  for (const key of ['name', 'data', 'namespace', 'x-mode', 'x-name', 'x-publicId', 'x-systemId']) {
    if (key in node) {
      output[key] = node[key];
    }
  }

  if ('attribs' in node) {
    output.attribs = sortedRecord(node.attribs);
  }

  if ('x-attribsNamespace' in node) {
    output['x-attribsNamespace'] = sortedRecord(node['x-attribsNamespace']);
  }

  if ('x-attribsPrefix' in node) {
    output['x-attribsPrefix'] = sortedRecord(node['x-attribsPrefix']);
  }

  if ('sourceCodeLocation' in node) {
    output.sourceCodeLocation = compactLocation(node.sourceCodeLocation);
  }

  if (Array.isArray(node.children)) {
    output.children = node.children.map(normalizeNode);
  }

  return output;
}

function assertSiblingLinks(node, parent = null) {
  assert.equal(node.parent, parent);

  if (!Array.isArray(node.children)) {
    return;
  }

  for (let idx = 0; idx < node.children.length; idx++) {
    const child = node.children[idx];
    assert.equal(child.parent, node);
    assert.equal(child.prev, node.children[idx - 1] ?? null);
    assert.equal(child.next, node.children[idx + 1] ?? null);
    assertSiblingLinks(child, node);
  }
}

const doctypeCases = [
  { args: ['', '', ''], expected: '!DOCTYPE ' },
  { args: ['html', '', ''], expected: '!DOCTYPE html' },
  { args: ['html', 'public-id', 'system-id'], expected: '!DOCTYPE html PUBLIC "public-id" "system-id"' },
  { args: ['html', '', 'system-id'], expected: '!DOCTYPE html SYSTEM "system-id"' },
  { args: ['html', 'public"id', 'system-id'], expected: '!DOCTYPE html PUBLIC \'public"id\' "system-id"' },
];

for (const { args, expected } of doctypeCases) {
  assert.equal(portSerializeDoctypeContent(...args), expected);
}

function exerciseAdapter(adapter, parse5) {
  const document = adapter.createDocument();
  adapter.setDocumentMode(document, parse5.html.DOCUMENT_MODE.QUIRKS);
  adapter.setDocumentType(document, 'html', 'public-id', 'system-id');
  adapter.setDocumentType(document, 'html', '', 'system-id-2');

  const first = adapter.createElement('div', parse5.html.NS.HTML, [
    { name: 'id', value: 'a' },
    { name: 'xlink:href', value: '#x', namespace: parse5.html.NS.XLINK, prefix: 'xlink' },
  ]);
  const second = adapter.createElement('span', parse5.html.NS.HTML, [{ name: 'class', value: 'b' }]);
  const comment = adapter.createCommentNode('comment');

  adapter.appendChild(document, first);
  adapter.appendChild(document, comment);
  adapter.insertBefore(document, second, comment);
  adapter.insertText(first, 'hello');
  adapter.insertText(first, ' world');
  adapter.insertTextBefore(document, 'before', second);
  adapter.adoptAttributes(first, [
    { name: 'id', value: 'ignored' },
    { name: 'title', value: 'kept' },
  ]);
  adapter.detachNode(comment);

  return document;
}

const portManual = exerciseAdapter(portAdapter, portParse5);
assert.equal(portAdapter.getDocumentMode(portManual), portParse5.html.DOCUMENT_MODE.QUIRKS);
assert.equal(portAdapter.getDocumentTypeNodeSystemId(portManual.children[0]), 'system-id-2');
assert.equal(portManual.children[1].attribs.id, 'a');
assert.equal(portManual.children[1].attribs.title, 'kept');
assert.equal(portManual.children[1].children[0].data, 'hello world');
assert.equal(portManual.children[2].data, 'before');
assert.equal(portManual.children[3].name, 'span');
assertSiblingLinks(portManual);

const htmlCases = [
  '<!doctype html><html><head><title>x</title></head><body><p id="a">hello<!--c--></p></body></html>',
  '<template><table><tr><td>A</table></template><svg><foreignObject><p data-x="1">html</p></foreignObject></svg>',
  '<math><mi definitionurl="x">n</mi><annotation-xml encoding="text/html"><p>html</p></annotation-xml></math>',
  '<table><tr><td>A<td>B</table><select><option>x</select>',
];

for (const html of htmlCases) {
  const portDocument = portParse5.parse(html, {
    sourceCodeLocationInfo: true,
    treeAdapter: portAdapter,
  });

  assert.equal(
    portParse5.serialize(portDocument, { treeAdapter: portAdapter }),
    portParse5.serialize(portParse5.parse(html, { sourceCodeLocationInfo: true }))
  );
  assert.ok(normalizeNode(portDocument));
  assertSiblingLinks(portDocument);
}

const portContext = portAdapter.createElement('table', portParse5.html.NS.HTML, []);
const fragmentHtml = '<tbody><tr><td>A<td>B</tbody>';
const portFragment = portParse5.parseFragment(portContext, fragmentHtml, {
  treeAdapter: portAdapter,
});

assert.equal(
  portParse5.serialize(portFragment, { treeAdapter: portAdapter }),
  portParse5.serialize(
    portParse5.parseFragment(
      portParse5.defaultTreeAdapter.createElement('table', portParse5.html.NS.HTML, []),
      fragmentHtml
    )
  )
);
assertSiblingLinks(portFragment);

console.log('parse5-htmlparser2-tree-adapter tests passed.');
