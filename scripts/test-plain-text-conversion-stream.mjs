import assert from 'node:assert/strict';
import { finished } from 'node:stream/promises';

import * as portParse5 from '../packages/parse5/src/index.js';
import { PlainTextConversionStream as PortPlainTextConversionStream } from '../packages/parse5-plain-text-conversion-stream/src/index.js';

async function convert(StreamClass, parse5, chunks, options = {}) {
  const stream = new StreamClass(options);

  for (let i = 0; i < chunks.length - 1; i++) {
    stream.write(chunks[i]);
  }

  stream.end(chunks.at(-1) ?? '');
  await finished(stream);

  return parse5.serialize(stream.document, options);
}

const chunks = ['Hey', '\r\nyo', '\u0000', '<html><head><body>'];

assert.equal(
  await convert(PortPlainTextConversionStream, portParse5, chunks),
  '<html><head></head><body><pre>\nHey\nyo\uFFFD&lt;html&gt;&lt;head&gt;&lt;body&gt;</pre></body></html>'
);

assert.equal(
  await convert(PortPlainTextConversionStream, portParse5, ['<p>plain & raw</p>']),
  '<html><head></head><body><pre>\n&lt;p&gt;plain &amp; raw&lt;/p&gt;</pre></body></html>'
);

const portAdapter = {
  ...portParse5.defaultTreeAdapter,
  getTagName(element) {
    assert.ok(element.tagName);
    return portParse5.defaultTreeAdapter.getTagName(element);
  },
};
assert.equal(
  await convert(PortPlainTextConversionStream, portParse5, chunks, { treeAdapter: portAdapter }),
  '<html><head></head><body><pre>\nHey\nyo\uFFFD&lt;html&gt;&lt;head&gt;&lt;body&gt;</pre></body></html>'
);

assert.throws(() => new PortPlainTextConversionStream().write(Buffer.from('test')), TypeError);

console.log('parse5-plain-text-conversion-stream tests passed.');
