import assert from 'node:assert/strict';
import fs from 'node:fs';
import path from 'node:path';
import { Writable } from 'node:stream';
import { finished } from 'node:stream/promises';
import { fileURLToPath } from 'node:url';

import { RewritingStream as PortRewritingStream } from '../packages/parse5-html-rewriting-stream/src/index.js';

const __dirname = path.dirname(fileURLToPath(import.meta.url));

const srcHtml = [
  '<!DOCTYPE html "">',
  '<html>',
  '    <!-- comment1 -->',
  '    <head /// 123>',
  '    </head>',
  '    <!-- comment2 -->',
  '    <body =123>',
  '        <div>Hey ya</div>',
  '    </body>',
  '</html>',
].join('\n');

const LONG_TEXT = 'a'.repeat((1 << 16) + 1);
const LONG_TEXT_WITH_COMMENT = `${'a'.repeat((1 << 16) - 5)}<!-- comment -->`;

class WritableStreamStub extends Writable {
  writtenData = '';

  constructor() {
    super({ decodeStrings: false });
  }

  _write(chunk, _encoding, callback) {
    assert.equal(typeof chunk, 'string', 'Expected output to be a string stream');
    this.writtenData += chunk;
    callback();
  }
}

function normalizeNewLine(str) {
  return str.replace(/\r\n/g, '\n');
}

function loadSAXParserTestData() {
  const dataDir = path.join(__dirname, '../fixtures/parse5/test/data/sax');

  return fs.readdirSync(dataDir).map((name) => ({
    name,
    src: normalizeNewLine(fs.readFileSync(path.join(dataDir, name, 'src.html'), 'utf8')),
  }));
}

function makeChunks(str) {
  if (str.length === 0) {
    return [''];
  }

  const chunks = [];
  const sizes = [1, 7, 3, 19, 5, 31, 11, 47, 4096];
  let offset = 0;
  let sizeIndex = 0;

  while (offset < str.length) {
    const size = sizes[sizeIndex % sizes.length];
    chunks.push(str.slice(offset, offset + size));
    offset += size;
    sizeIndex++;
  }

  return chunks;
}

function writeChunkedToStream(str, stream) {
  const chunks = makeChunks(str);
  const last = chunks.length - 1;

  for (const [idx, chunk] of chunks.entries()) {
    if (idx === last) {
      stream.end(chunk);
    } else {
      stream.write(chunk);
    }
  }
}

async function rewrite(StreamClass, src, assignTokenHandlers = () => {}) {
  const rewriter = new StreamClass();
  const writable = new WritableStreamStub();

  rewriter.pipe(writable);
  assignTokenHandlers(rewriter);
  writeChunkedToStream(src, rewriter);

  await finished(writable);
  return writable.writtenData;
}

async function assertRewrite({ src, expected, assignTokenHandlers }) {
  const portOutput = await rewrite(PortRewritingStream, src, assignTokenHandlers);

  assert.equal(portOutput, expected);
}

for (const data of loadSAXParserTestData()) {
  await assertRewrite({
    src: data.src,
    expected: data.src,
  });
}

await assertRewrite({
  src: srcHtml,
  expected: [
    '<!DOCTYPE html "">',
    '<html>',
    '    <!-- comment1 -->',
    '    <body 123="">',
    '    </head>',
    '    <!-- comment2 -->',
    '    <head =123="">',
    '        <div>Hey ya</div>',
    '    </body>',
    '</html>',
  ].join('\n'),
  assignTokenHandlers: (rewriter) => {
    rewriter.on('startTag', (token) => {
      if (token.tagName === 'head') {
        token.tagName = 'body';
      } else if (token.tagName === 'body') {
        token.tagName = 'head';
      }

      rewriter.emitStartTag(token);
    });
  },
});

await assertRewrite({
  src: srcHtml,
  expected: [
    '<!DOCTYPE html "">',
    '<html>',
    '    <!-- comment1 -->',
    '    <head /// 123>',
    '    </rewritten>',
    '    <!-- comment2 -->',
    '    <body =123>',
    '        <div>Hey ya</rewritten>',
    '    </rewritten>',
    '</rewritten>',
  ].join('\n'),
  assignTokenHandlers: (rewriter) => {
    rewriter.on('endTag', (token) => {
      token.tagName = 'rewritten';
      rewriter.emitEndTag(token);
    });
  },
});

await assertRewrite({
  src: srcHtml,
  expected: [
    '<!DOCTYPE html "">',
    '<html>',
    '    <!-- comment1 -->',
    '    <head /// 123>',
    '    </head>',
    '    <!-- comment2 -->',
    '    <body =123>',
    '        <div>42</div>',
    '    </body>',
    '</html>',
  ].join('\n'),
  assignTokenHandlers: (rewriter) => {
    rewriter.on('text', (token) => {
      if (token.text.trim().length > 0) {
        token.text = '42';
      }

      rewriter.emitText(token);
    });
  },
});

await assertRewrite({
  src: srcHtml,
  expected: [
    '<!DOCTYPE html "">',
    '<html>',
    '    <!--42-->',
    '    <head /// 123>',
    '    </head>',
    '    <!--42-->',
    '    <body =123>',
    '        <div>Hey ya</div>',
    '    </body>',
    '</html>',
  ].join('\n'),
  assignTokenHandlers: (rewriter) => {
    rewriter.on('comment', (token) => {
      token.text = '42';
      rewriter.emitComment(token);
    });
  },
});

await assertRewrite({
  src: srcHtml,
  expected: [
    '<!DOCTYPE html PUBLIC "42" "hey">',
    '<html>',
    '    <!-- comment1 -->',
    '    <head /// 123>',
    '    </head>',
    '    <!-- comment2 -->',
    '    <body =123>',
    '        <div>Hey ya</div>',
    '    </body>',
    '</html>',
  ].join('\n'),
  assignTokenHandlers: (rewriter) => {
    rewriter.on('doctype', (token) => {
      token.publicId = '42';
      token.systemId = 'hey';
      rewriter.emitDoctype(token);
    });
  },
});

await assertRewrite({
  src: srcHtml,
  expected: [
    '<!DOCTYPE html SYSTEM "hey">',
    '<html>',
    '    <!-- comment1 -->',
    '    <head /// 123>',
    '    </head>',
    '    <!-- comment2 -->',
    '    <body =123>',
    '        <div>Hey ya</div>',
    '    </body>',
    '</html>',
  ].join('\n'),
  assignTokenHandlers: (rewriter) => {
    rewriter.on('doctype', (token) => {
      token.publicId = null;
      token.systemId = 'hey';
      rewriter.emitDoctype(token);
    });
  },
});

await assertRewrite({
  src: srcHtml,
  expected: [
    '<!DOCTYPE html "">',
    '<wrap><html></wrap>',
    '    <!-- comment1 -->',
    '    <wrap><head 123=""/></wrap>',
    '    </head>',
    '    <!-- comment2 -->',
    '    <wrap><body =123=""></wrap>',
    '        <wrap><div></wrap>Hey ya</div>',
    '    </body>',
    '</html>',
  ].join('\n'),
  assignTokenHandlers: (rewriter) => {
    rewriter.on('startTag', (token) => {
      rewriter.emitRaw('<wrap>');

      if (token.tagName === 'head') {
        token.selfClosing = true;
      }

      rewriter.emitStartTag(token);
      rewriter.emitRaw('</wrap>');
    });
  },
});

await assertRewrite({
  src: srcHtml,
  expected: [
    '<!DOCTYPE html "">42',
    '<html>42',
    '    <!-- comment1 -->42',
    '    <head /// 123>42',
    '    </head>42',
    '    <!-- comment2 -->42',
    '    <body =123>42',
    '        <div>42Hey ya</div>42',
    '    </body>42',
    '</html>42',
  ].join('\n'),
  assignTokenHandlers: (rewriter) => {
    const rewriteRaw = (_token, raw) => rewriter.emitRaw(`${raw}42`);

    rewriter
      .on('doctype', rewriteRaw)
      .on('startTag', rewriteRaw)
      .on('endTag', rewriteRaw)
      .on('comment', rewriteRaw);
  },
});

await assertRewrite({
  src: [
    '<!DOCTYPE html "">',
    '<html>',
    '    <head foo=\'bar"baz"\'>',
    '    </head>',
    '    <body>',
    '        <div>foo&amp;bar</div>',
    '    </body>',
    '</html>',
  ].join('\n'),
  expected: [
    '<!DOCTYPE html "">',
    '<html>',
    '    <head foo="bar&quot;baz&quot;">',
    '    </head>',
    '    <body>',
    '        <div>foo&amp;bar</div>',
    '    </body>',
    '</html>',
  ].join('\n'),
  assignTokenHandlers: (rewriter) => {
    rewriter.on('startTag', (token) => rewriter.emitStartTag(token));
    rewriter.on('text', (token) => rewriter.emitText(token));
  },
});

async function textChunkIsFlushed(StreamClass) {
  const parser = new StreamClass();
  let foundText = false;

  parser.on('text', ({ text }) => {
    foundText = true;
    assert.equal(text, 'text');
  });

  parser.write('text');
  parser.end();
  await finished(parser);
  assert.equal(foundText, true);
}

await textChunkIsFlushed(PortRewritingStream);

assert.throws(() => new PortRewritingStream().write(Buffer.from('test')), TypeError);

await assertRewrite({
  src: LONG_TEXT,
  expected: LONG_TEXT,
});

await assertRewrite({
  src: LONG_TEXT_WITH_COMMENT,
  expected: LONG_TEXT_WITH_COMMENT,
});

await assertRewrite({
  src: '<script></script>',
  expected: '<script>foo && bar</script>',
  assignTokenHandlers: (rewriter) => {
    rewriter.on('startTag', (token) => {
      rewriter.emitStartTag(token);

      if (token.tagName === 'script') {
        rewriter.emitText({ text: 'foo && bar' });
      }
    });
  },
});

console.log('parse5-html-rewriting-stream tests passed.');
