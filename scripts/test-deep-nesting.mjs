// parse() must survive deep trees. The Rust core already did; the wasm JSON
// encoder and the JS parent/adapter walks used to recurse until RangeError.
// See docs/architecture.md concern 6.
import assert from 'node:assert/strict';
import path from 'node:path';
import { pathToFileURL } from 'node:url';

const root = path.resolve(import.meta.dirname, '..');
const port = await import(pathToFileURL(path.join(root, 'packages/parse5/src/index.js')).href);

function countDivs(rootNode) {
  let count = 0;
  const stack = [rootNode];

  while (stack.length > 0) {
    const node = stack.pop();

    if (!node || typeof node !== 'object') {
      continue;
    }

    if (node.tagName === 'div') {
      count += 1;
    }

    if (node.content) {
      stack.push(node.content);
    }

    if (Array.isArray(node.childNodes)) {
      for (let index = node.childNodes.length - 1; index >= 0; index -= 1) {
        stack.push(node.childNodes[index]);
      }
    }
  }

  return count;
}

for (const depth of [5_000, 10_000]) {
  const html = '<div>'.repeat(depth);
  const document = port.parse(html);
  const divs = countDivs(document);
  assert.equal(divs, depth, `expected ${depth} divs, found ${divs}`);
  console.log(`deep nesting ok: ${depth} divs`);
}
