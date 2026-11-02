// Runs the Rust serializer and the JS serializer over the same vendored
// serialization fixture. Uses fixtures/ only.

import assert from 'node:assert/strict';
import fs from 'node:fs';
import path from 'node:path';
import { pathToFileURL } from 'node:url';

const root = path.resolve(import.meta.dirname, '..');
const fixturePath = path.join(root, 'fixtures/parse5/test/data/serialization/tests.json');
const wasmEntry = path.join(root, 'packages/parse5-wasm/pkg/parse5_wasm.js');
const portEntry = path.join(root, 'packages/parse5/src/index.js');

assert.equal(
  fs.existsSync(wasmEntry),
  true,
  'packages/parse5-wasm/pkg is missing; run npm run build:wasm'
);

const tests = JSON.parse(fs.readFileSync(fixturePath, 'utf8'));
const wasm = await import(pathToFileURL(wasmEntry).href);
const port = await import(pathToFileURL(portEntry).href);
await port.initParse5Wasm?.();

const failures = [];

for (const [idx, test] of tests.entries()) {
  const scriptingEnabled = test.options?.scriptingEnabled ?? true;
  const options = { scriptingEnabled };
  const rustHtml = wasm.parseAndSerialize(test.input, scriptingEnabled);
  const jsHtml = port.serialize(port.parse(test.input, options), options);

  if (rustHtml !== test.expected || jsHtml !== test.expected || rustHtml !== jsHtml) {
    failures.push({
      idx,
      name: test.name,
      rustMatches: rustHtml === test.expected,
      jsMatches: jsHtml === test.expected,
      serializersMatch: rustHtml === jsHtml,
    });
  }
}

if (failures.length > 0) {
  console.error(JSON.stringify(failures.slice(0, 10), null, 2));
  throw new Error(
    `${failures.length}/${tests.length} serialization fixtures disagree between the Rust serializer, the JS serializer, and tests.json`
  );
}

console.log(`serializer parity passed: ${tests.length} fixtures`);
