import assert from 'node:assert/strict';
import fs from 'node:fs';
import path from 'node:path';
import { Writable } from 'node:stream';
import { finished } from 'node:stream/promises';

import { SAXParser as PortSAXParser } from '../packages/parse5-sax-parser/src/index.js';
import { ParserFeedbackSimulator as PortParserFeedbackSimulator } from '../packages/parse5-sax-parser/src/parser-feedback-simulator.js';
import {
  loadParserFeedbackTokenizationTests,
  loadSAXParserTestData,
  makeChunks,
  removeNewLines,
} from './parse5-test-utils.mjs';

const root = path.resolve(import.meta.dirname, '..');

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

function sanitizeForComparison(str) {
  return removeNewLines(str).replace(/\s/g, '').replaceAll("'", '"').toLowerCase();
}

async function saxSerializedOutput(ParserClass, html, options = {}) {
  let actual = '';
  const parser = new ParserClass(options);

  parser.on('doctype', ({ name, publicId, systemId }) => {
    actual += `<!DOCTYPE ${name}`;

    if (publicId !== null) {
      actual += ` PUBLIC "${publicId}"`;
    } else if (systemId !== null) {
      actual += ' SYSTEM';
    }

    if (systemId !== null) {
      actual += ` "${systemId}"`;
    }

    actual += '>';
  });

  parser.on('startTag', ({ tagName, attrs, selfClosing }) => {
    actual += `<${tagName}`;

    for (const attr of attrs) {
      actual += ` ${attr.name}="${attr.value}"`;
    }

    actual += selfClosing ? '/>' : '>';
  });

  parser.on('endTag', ({ tagName }) => {
    actual += `</${tagName}>`;
  });

  parser.on('text', ({ text }) => {
    actual += text;
  });

  parser.on('comment', ({ text }) => {
    actual += `<!--${text}-->`;
  });

  const chunks = makeChunks(html);

  for (let i = 0; i < chunks.length - 1; i++) {
    parser.write(chunks[i]);
  }

  parser.end(chunks.at(-1) ?? '');
  await finished(parser);
  return actual;
}

function compactSaxEvent(kind, token) {
  const output = { kind };

  for (const key of ['tagName', 'attrs', 'selfClosing', 'text', 'name', 'publicId', 'systemId']) {
    if (key in token) {
      output[key] = token[key];
    }
  }

  output.sourceCodeLocation = compactLocation(token.sourceCodeLocation);
  return output;
}

async function collectSaxEvents(ParserClass, html, options = {}, chunks = null) {
  const parser = new ParserClass(options);
  const events = [];

  parser.on('doctype', (token) => events.push(compactSaxEvent('doctype', token)));
  parser.on('startTag', (token) => events.push(compactSaxEvent('startTag', token)));
  parser.on('endTag', (token) => events.push(compactSaxEvent('endTag', token)));
  parser.on('text', (token) => events.push(compactSaxEvent('text', token)));
  parser.on('comment', (token) => events.push(compactSaxEvent('comment', token)));

  const pieces = chunks ?? [html];

  for (let i = 0; i < pieces.length - 1; i++) {
    parser.write(pieces[i]);
  }

  parser.end(pieces.at(-1) ?? '');
  await finished(parser);
  return events;
}

function compactToken(token) {
  const output = { type: token.type };

  for (const key of ['tagName', 'attrs', 'selfClosing', 'chars', 'data', 'name', 'publicId', 'systemId', 'forceQuirks']) {
    if (key in token) {
      output[key] = token[key];
    }
  }

  output.location = compactLocation(token.location);
  return output;
}

function feedbackTokens(SimulatorClass, html) {
  const tokens = [];
  const handler = {
    onCharacter: (token) => tokens.push(compactToken(token)),
    onComment: (token) => tokens.push(compactToken(token)),
    onDoctype: (token) => tokens.push(compactToken(token)),
    onEndTag: (token) => tokens.push(compactToken(token)),
    onEof: (token) => tokens.push(compactToken(token)),
    onNullCharacter: (token) => tokens.push(compactToken(token)),
    onStartTag: (token) => tokens.push(compactToken(token)),
    onWhitespaceCharacter: (token) => tokens.push(compactToken(token)),
  };
  const simulator = new SimulatorClass({ sourceCodeLocationInfo: true }, handler);
  simulator.tokenizer.write(html, true);
  return tokens;
}

function reformattedAttrs(attrs) {
  return Object.fromEntries(attrs.map(({ name, value }) => [name, value]));
}

function runParserFeedbackFixture(SimulatorClass, testData) {
  const tokens = [];
  const errors = [];
  let sawEof = false;
  const handler = {
    addToken(token) {
      tokens.push(token);
    },
    onComment(token) {
      this.addToken(['Comment', token.data]);
    },
    onDoctype(token) {
      this.addToken(['DOCTYPE', token.name, token.publicId, token.systemId, !token.forceQuirks]);
    },
    onStartTag(token) {
      const entry = ['StartTag', token.tagName, reformattedAttrs(token.attrs)];

      if (token.selfClosing) {
        entry.push(true);
      }

      this.addToken(entry);
    },
    onEndTag(token) {
      this.addToken(['EndTag', token.tagName.toLowerCase()]);
    },
    onEof() {
      sawEof = true;
    },
    onCharacter(token) {
      const lastEntry = tokens[tokens.length - 1];

      if (lastEntry && lastEntry[0] === 'Character' && lastEntry[1] != null) {
        lastEntry[1] += token.chars;
      } else {
        tokens.push(['Character', token.chars]);
      }
    },
    onNullCharacter(token) {
      this.onCharacter(token);
    },
    onWhitespaceCharacter(token) {
      this.onCharacter(token);
    },
    onParseError(error) {
      errors.push({
        code: error.code,
        line: error.startLine,
        col: error.startCol,
      });
    },
  };
  const simulator = new SimulatorClass({}, handler);
  simulator.tokenizer.preprocessor.bufferWaterline = 8;
  simulator.tokenizer.state = testData.initialState;

  if (testData.lastStartTag) {
    simulator.tokenizer.lastStartTagName = testData.lastStartTag;
  }

  const chunks = makeChunks(testData.input);

  for (let i = 0; i < chunks.length; i++) {
    assert.equal(sawEof, false, `${testData.setName}.${testData.idx}: EOF before final chunk`);
    simulator.tokenizer.write(chunks[i], i === chunks.length - 1);
  }

  errors.sort((left, right) => left.line - right.line || left.col - right.col);

  assert.equal(sawEof, true, `${testData.setName}.${testData.idx}: EOF was not emitted`);
  assert.equal(simulator.tokenizer.active, false, `${testData.setName}.${testData.idx}: tokenizer still active`);
  assert.deepEqual(tokens, testData.expected, `${testData.setName}.${testData.idx}: ${testData.name}`);
  assert.deepEqual(errors, testData.expectedErrors, `${testData.setName}.${testData.idx}: parse errors`);
}

for (const data of loadSAXParserTestData(root)) {
  assert.equal(
    sanitizeForComparison(await saxSerializedOutput(PortSAXParser, data.src)),
    sanitizeForComparison(data.expected),
    `SAX fixture serialization failed for ${data.name}`
  );
}

const cases = [
  '<!doctype html><html><head><title>x</title></head><body><p id="a">hello<!--c--></p></body></html>',
  '<section><svg><foreignObject><p>x</p></foreignObject><textPath xlink:href="#x">y</textPath></svg></section>',
  '<math><mi definitionurl="x">n</mi><annotation-xml encoding="text/html"><p>html</p></annotation-xml></math>',
  '<textarea>\nhello</textarea><script>if (a < b) document.write("<p>x</p>")</script>',
  '<image src=x><pre>\ntrimmed</pre>\0',
];

for (const html of cases) {
  const events = await collectSaxEvents(PortSAXParser, html, { sourceCodeLocationInfo: true });
  assert.equal(events.length > 0, true, `expected SAX events for ${html}`);
  assert.equal(
    events.every((event) => event.sourceCodeLocation == null || typeof event.sourceCodeLocation.startOffset === 'number'),
    true,
    `expected compact locations for ${html}`
  );
}

assert.deepEqual(
  await collectSaxEvents(PortSAXParser, '', {}),
  []
);

const chunked = ['<!doctype html><title>', 'hello</title><body><p>', 'chunked</p>'];
assert.deepEqual(
  await collectSaxEvents(PortSAXParser, chunked.join(''), {}, chunked),
  await collectSaxEvents(PortSAXParser, chunked.join(''), {})
);

assert.throws(() => new PortSAXParser().write(Buffer.from('test')), TypeError);

const nullCharacters = await collectSaxEvents(PortSAXParser, '\0', {});
assert.equal(nullCharacters.some((event) => event.kind === 'text' && event.text === '\0'), true);

async function stopAfterTwoEvents(ParserClass) {
  const parser = new ParserClass();
  const events = [];
  parser.on('startTag', (token) => {
    events.push(token.tagName);

    if (events.length === 2) {
      parser.stop();
    }
  });

  parser.write('<div><span><em>x</em></span></div>');
  parser.end();
  await finished(parser);
  return events;
}

assert.deepEqual(await stopAfterTwoEvents(PortSAXParser), ['div', 'span']);

assert.deepEqual(
  feedbackTokens(PortParserFeedbackSimulator, '<p>hello</p>').map(({ type, tagName, chars }) => ({
    type,
    tagName,
    chars,
  })),
  [
    { type: 3, tagName: 'p', chars: undefined },
    { type: 0, tagName: undefined, chars: 'hello' },
    { type: 4, tagName: 'p', chars: undefined },
    { type: 7, tagName: undefined, chars: undefined },
  ]
);

const hugePage = path.join(root, 'fixtures/parse5/test/data/huge-page/huge-page.html');
const parser = new PortSAXParser();
const writable = new WritableStreamStub();
let handlerCallCount = 0;
const stopHandler = () => {
  handlerCallCount++;

  if (handlerCallCount === 10) {
    parser.stop();
  }
};

parser.on('startTag', stopHandler);
parser.on('endTag', stopHandler);
parser.on('doctype', stopHandler);
parser.on('comment', stopHandler);
parser.on('text', stopHandler);
fs.createReadStream(hugePage, 'utf8').pipe(parser).pipe(writable);
await finished(writable);
assert.equal(handlerCallCount, 10);
assert.equal(writable.writtenData, fs.readFileSync(hugePage, 'utf8'));

const devNullParser = new PortSAXParser();
fs.createReadStream(hugePage, 'utf8').pipe(devNullParser);
await finished(devNullParser);

for (const testData of loadParserFeedbackTokenizationTests(root)) {
  runParserFeedbackFixture(PortParserFeedbackSimulator, testData);
}

console.log('parse5-sax-parser tests passed.');
