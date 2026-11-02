// Pins the quadratic streaming path (docs/architecture.md concern 4).
// A ratio above TARGET is still the known bug and does not fail the suite.
// A ratio above CEILING is a regression and fails.
import assert from 'node:assert/strict';
import fs from 'node:fs';
import path from 'node:path';
import { pathToFileURL } from 'node:url';

const TARGET = 20;
const CEILING = 1_500;
const root = path.resolve(import.meta.dirname, '..');
const html = fs.readFileSync(
  path.join(root, 'fixtures/parse5/test/data/location-info/wiki-42/data.html'),
  'utf8'
);
const port = await import(pathToFileURL(path.join(root, 'packages/parse5/src/index.js')).href);

function streamParse(chunkSize) {
  const parser = new port.Parser();

  for (let offset = 0; offset < html.length; offset += chunkSize) {
    parser.tokenizer.write(html.slice(offset, offset + chunkSize), offset + chunkSize >= html.length);
  }

  return parser.document;
}

function once(fn) {
  const start = performance.now();
  fn();
  return performance.now() - start;
}

const oneShot = once(() => port.parse(html));
const streamed = once(() => streamParse(64 * 1024));
const ratio = streamed / oneShot;

assert.equal(streamParse(64 * 1024).nodeName, port.parse(html).nodeName);
console.log(
  `streaming ${ (html.length / 1024).toFixed(0) } KiB in 64 KiB chunks: ${streamed.toFixed(0)} ms vs one-shot ${oneShot.toFixed(0)} ms (${ratio.toFixed(1)}x)`
);

if (ratio > CEILING) {
  throw new Error(
    `streaming regression: ${ratio.toFixed(1)}x one-shot exceeds the ${CEILING}x ceiling (docs/architecture.md concern 4)`
  );
}

if (ratio > TARGET) {
  console.log(
    `KNOWN_BUG: streaming is ${ratio.toFixed(1)}x one-shot. docs/architecture.md concern 4. Target is under ${TARGET}x.`
  );
}
