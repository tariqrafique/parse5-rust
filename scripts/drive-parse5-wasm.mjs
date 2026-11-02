// Drives the package surface LWC uses: parseFragment + locations + onParseError.
import assert from 'node:assert/strict';
import path from 'node:path';
import { pathToFileURL } from 'node:url';

const root = path.resolve(import.meta.dirname, '..');
const port = await import(pathToFileURL(path.join(root, 'packages/parse5/src/index.js')).href);

const template = `<template>
  <div class="card" data-id={recordId}>
    <p>Hello &amp; more</p>
  </div>
</template>`;

const fragment = port.parseFragment(template, {
  sourceCodeLocationInfo: true,
  onParseError() {},
});

assert.equal(fragment.nodeName, '#document-fragment');
const templateNode = fragment.childNodes[0];
assert.equal(templateNode.tagName, 'template');
assert.equal(templateNode.namespaceURI, port.html.NS.HTML);
assert.equal(typeof templateNode.sourceCodeLocation.startOffset, 'number');
assert.equal(templateNode.sourceCodeLocation.startOffset, 0);
assert.ok(templateNode.sourceCodeLocation.startTag);
assert.equal(templateNode.parentNode, fragment);

const div = templateNode.content.childNodes.find((node) => node.tagName === 'div');
assert.ok(div, 'template content should contain the div');
assert.equal(div.attrs.find((attr) => attr.name === 'class')?.value, 'card');
assert.equal(typeof div.sourceCodeLocation.attrs.class.startOffset, 'number');
assert.equal(div.parentNode, templateNode.content);

const errors = [];
port.parseFragment('<div id="a" id="b"></div>', {
  sourceCodeLocationInfo: true,
  onParseError(error) {
    errors.push(error);
  },
});

assert.ok(errors.length > 0, 'onParseError should report the duplicate attribute');
assert.equal(typeof errors[0].code, 'string');
assert.equal(typeof errors[0].startOffset, 'number');
assert.ok(
  Object.values(port.ErrorCodes).includes(errors[0].code),
  `unexpected error code ${errors[0].code}`
);

console.log(`drive ok: parseFragment locations and ${errors.length} parse error(s)`);
