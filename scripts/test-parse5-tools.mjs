import assert from 'node:assert/strict';

import * as tools from '../packages/parse5-tools/src/index.js';

const root = tools.createDocumentFragment();
const div = tools.createElement('div', { id: 'root' });
const template = tools.createTemplateNode();
const text = tools.createTextNode('hello');
const comment = tools.createCommentNode('note');

tools.appendChild(root, div);
tools.appendChild(div, text);
tools.appendChild(div, comment);

assert.equal(tools.isDocumentFragment(root), true);
assert.equal(tools.isElementNode(div), true);
assert.equal(tools.isTextNode(text), true);
assert.equal(tools.isCommentNode(comment), true);
assert.equal(tools.isTemplateNode(template), true);
assert.equal(text.parentNode, div);
assert.equal(tools.getAttribute(div, 'id'), 'root');
assert.equal(tools.hasAttribute(div, 'id'), true);

tools.setAttribute(div, 'id', 'updated');
assert.equal(tools.getAttribute(div, 'id'), 'updated');
tools.removeAttribute(div, 'id');
assert.equal(tools.getAttribute(div, 'id'), null);

tools.appendChild(root, template);
tools.appendChild(template.content, tools.createTextNode('content'));

assert.equal(tools.getTextContent(root), 'hello');
assert.deepEqual([...tools.queryAll(root, tools.isTextNode)].map((node) => node.value), ['hello']);
assert.equal(tools.query(root, tools.isCommentNode), comment);
assert.deepEqual([...tools.previousSiblings(comment)], [text]);
assert.deepEqual([...tools.nextSiblings(text)], [comment]);
assert.deepEqual([...tools.ancestors(text)], [text, div, root]);

const visited = [];
tools.traverse(root, {
  element(node) {
    visited.push(node.tagName);
  },
  text(node) {
    visited.push(`#text:${node.value}`);
  },
  comment(node) {
    visited.push(`#comment:${node.data}`);
  },
  template() {
    visited.push('template-visitor');
  },
});
assert.deepEqual(visited, ['#text:hello', '#comment:note', 'div', 'template', 'template-visitor']);

const replacement = tools.createTextNode('replacement');
tools.replaceWith(comment, replacement);
assert.equal(comment.parentNode, null);
assert.equal(replacement.parentNode, div);
assert.deepEqual(div.childNodes, [text, replacement]);

tools.removeNode(replacement);
assert.equal(replacement.parentNode, null);
assert.deepEqual(div.childNodes, [text]);

console.log('@parse5/tools facade tests passed.');
