import assert from 'node:assert/strict';
import path from 'node:path';
import { finished } from 'node:stream/promises';
import { runInNewContext } from 'node:vm';

import * as portParse5 from '../packages/parse5/src/index.js';
import { ParserStream as PortParserStream } from '../packages/parse5-parser-stream/src/index.js';
import { adapter as portHtmlparser2Adapter } from '../packages/parse5-htmlparser2-tree-adapter/src/index.js';
import {
  loadTreeConstructionTestData,
  makeChunks,
  serializeToDatFileFormat,
} from './parse5-test-utils.mjs';

const root = path.resolve(import.meta.dirname, '..');

async function parseDocumentWithStream(StreamClass, chunks, options = {}) {
  const stream = new StreamClass(options);

  for (let i = 0; i < chunks.length - 1; i++) {
    stream.write(chunks[i]);
  }

  stream.end(chunks.at(-1) ?? '');
  await finished(stream);
  return stream.document;
}

async function parseFragmentWithStream(StreamClass, fragmentContext, chunks, options = {}) {
  const stream = StreamClass.getFragmentStream(fragmentContext, options);

  for (let i = 0; i < chunks.length - 1; i++) {
    stream.write(chunks[i]);
  }

  stream.end(chunks.at(-1) ?? '');
  await finished(stream);
  return stream.getFragment();
}

async function parseChunkedWithStream(StreamClass, test, options = {}, minChunkSize = 1, maxChunkSize = 10) {
  const stream = test.fragmentContext
    ? StreamClass.getFragmentStream(test.fragmentContext, options)
    : new StreamClass(options);
  const chunks = makeChunks(test.input, minChunkSize, maxChunkSize);

  stream.parser.tokenizer.preprocessor.bufferWaterline = 8;

  for (let i = 0; i < chunks.length - 1; i++) {
    stream.write(chunks[i]);
  }

  stream.end(chunks.at(-1) ?? '');
  await finished(stream);

  return {
    node: test.fragmentContext ? stream.getFragment() : stream.document,
    chunks,
  };
}

function formatParseError(error) {
  let errorString = `(${error.startLine}:${error.startCol}`;

  if (error.startLine !== error.endLine || error.startCol !== error.endCol) {
    errorString += `-${error.endLine}:${error.endCol}`;
  }

  return `${errorString}) ${error.code}`;
}

async function assertTreeConstructionFixture(StreamClass, test, treeAdapter, options = {}) {
  const errors = [];
  const parserOptions = {
    scriptingEnabled: test.scriptingEnabled,
    treeAdapter,
    onParseError: (error) => errors.push(formatParseError(error)),
  };
  const { node, chunks } = await parseChunkedWithStream(StreamClass, test, parserOptions);
  const actual = serializeToDatFileFormat(node, treeAdapter);
  const message = [
    `${test.dirName}/${test.setName}#${test.idx} line ${test.lineNum}`,
    `input: ${JSON.stringify(test.input)}`,
    `chunks: ${JSON.stringify(chunks)}`,
    'expected:',
    test.expected,
    'actual:',
    actual,
  ].join('\n');

  try {
    assert.equal(actual, test.expected, message);

    if (!options.withoutErrors) {
      assert.deepEqual(errors.sort(), [...test.expectedErrors].sort(), `${test.setName}#${test.idx} parse errors`);
    }
  } catch (error) {
    if (options.expectError) {
      return;
    }

    throw error;
  }

  if (options.expectError) {
    throw new Error(`Expected parser-stream fixture mismatch but none occurred: ${test.idx}.${test.setName}`);
  }
}

async function assertGeneratedTreeConstructionSuite(suitePath, options = {}) {
  const adapters = [
    { name: 'default', adapter: portParse5.defaultTreeAdapter },
    { name: 'htmlparser2', adapter: portHtmlparser2Adapter },
  ];

  for (const { name, adapter } of adapters) {
    const expectedErrors = new Set(options.expectErrors ?? []);

    for (const test of loadTreeConstructionTestData(root, suitePath, adapter)) {
      const key = `${test.idx}.${test.setName}`;
      const expectError = expectedErrors.delete(key);

      await assertTreeConstructionFixture(PortParserStream, test, adapter, {
        withoutErrors: options.withoutErrors,
        expectError,
      });
    }

    assert.equal(
      expectedErrors.size,
      0,
      `${name}: expected parser-stream fixture failures not found: ${[...expectedErrors].join(', ')}`
    );
  }
}

async function assertGeneratedScriptingSuite() {
  const adapters = [
    { name: 'default', adapter: portParse5.defaultTreeAdapter },
    { name: 'htmlparser2', adapter: portHtmlparser2Adapter },
  ];
  const pause = () => new Promise((resolve) => setTimeout(resolve, 0));

  for (const { adapter } of adapters) {
    for (const test of loadTreeConstructionTestData(
      root,
      'fixtures/parse5/test/data/tree-construction-scripting',
      adapter
    )) {
      const chunks = makeChunks(test.input);
      const stream = test.fragmentContext
        ? PortParserStream.getFragmentStream(test.fragmentContext, {
            treeAdapter: adapter,
            scriptingEnabled: test.scriptingEnabled,
          })
        : new PortParserStream({
            treeAdapter: adapter,
            scriptingEnabled: test.scriptingEnabled,
          });

      stream.on('script', async (scriptElement, documentWrite, resume) => {
        const scriptTextNode = adapter.getChildNodes(scriptElement)[0];
        const script = scriptTextNode ? adapter.getTextNodeContent(scriptTextNode) : '';

        await pause();

        try {
          runInNewContext(script, { document: { write: documentWrite } });
          resume();
        } catch (error) {
          stream.emit('error', error);
        }
      });

      for (const chunk of chunks) {
        stream.write(chunk);
        await pause();
      }

      stream.end();
      await finished(stream);

      const node = test.fragmentContext ? stream.getFragment() : stream.document;
      assert.equal(serializeToDatFileFormat(node, adapter), test.expected, `${test.setName}#${test.idx}`);
    }
  }
}

async function parseWithScript(StreamClass, parse5, html, resumeMode) {
  const stream = new StreamClass();
  const scripts = [];

  stream.on('script', async (scriptElement, documentWrite, resume) => {
    const scriptTextNode = parse5.defaultTreeAdapter.getChildNodes(scriptElement)[0];
    const script = scriptTextNode ? parse5.defaultTreeAdapter.getTextNodeContent(scriptTextNode) : '';
    scripts.push(script);

    if (resumeMode === 'sync') {
      runInNewContext(script, { document: { write: documentWrite } });
      resume();
      return;
    }

    await new Promise((resolve) => setTimeout(resolve, 0));
    runInNewContext(script, { document: { write: documentWrite } });
    resume();
  });

  stream.end(html);
  await finished(stream);

  return {
    html: parse5.serialize(stream.document),
    scripts,
  };
}

const chunks = [
  '<!doctype html><html><head>',
  '<title>x</title></head><body>',
  '<table><tr><td>A<td>B</table>',
  '</body></html>',
];

assert.equal(
  portParse5.serialize(await parseDocumentWithStream(PortParserStream, chunks, { sourceCodeLocationInfo: true })),
  '<!DOCTYPE html><html><head><title>x</title></head><body><table><tbody><tr><td>A</td><td>B</td></tr></tbody></table></body></html>'
);

const portTable = portParse5.defaultTreeAdapter.createElement('table', portParse5.html.NS.HTML, []);
const fragmentChunks = ['<tbody><tr>', '<td>A<td>B</tbody>'];

assert.equal(
  portParse5.serialize(await parseFragmentWithStream(PortParserStream, portTable, fragmentChunks)),
  '<tbody><tr><td>A</td><td>B</td></tr></tbody>'
);

const emptyPortStream = new PortParserStream();
emptyPortStream.end();
await finished(emptyPortStream);
assert.equal(portParse5.serialize(emptyPortStream.document), '<html><head></head><body></body></html>');

assert.throws(() => new PortParserStream().write(Buffer.from('test')), TypeError);

const scriptInput =
  '<!doctype html><script>document.write("<p>written</p>")</script><p>after</p>';

const scriptExpected = {
  html: '<!DOCTYPE html><html><head><script>document.write("<p>written</p>")</script></head><body><p>written</p><p>after</p></body></html>',
  scripts: ['document.write("<p>written</p>")'],
};

assert.deepEqual(await parseWithScript(PortParserStream, portParse5, scriptInput, 'async'), scriptExpected);
assert.deepEqual(await parseWithScript(PortParserStream, portParse5, scriptInput, 'sync'), scriptExpected);

async function parseWithDeferredResume(StreamClass, parse5) {
  const stream = new StreamClass();

  stream.on('script', (_scriptElement, _documentWrite, resume) => {
    process.nextTick(resume);
  });

  stream.write('<script>yo</script>');
  stream.end('dawg');
  await finished(stream);

  return parse5.serialize(stream.document);
}

assert.equal(
  await parseWithDeferredResume(PortParserStream, portParse5),
  '<html><head><script>yo</script></head><body>dawg</body></html>'
);

await assertGeneratedTreeConstructionSuite('fixtures/parse5/test/data/html5lib-tests-fork/tree-construction', {
  expectErrors: [
    '0.foreign-fragment',
    '1.foreign-fragment',
    '38.foreign-fragment',
    '40.foreign-fragment',
    '47.foreign-fragment',
    '48.foreign-fragment',
  ],
});
await assertGeneratedScriptingSuite();

console.log('parse5-parser-stream tests passed.');
