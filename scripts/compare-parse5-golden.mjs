import fs from 'node:fs';
import path from 'node:path';
import { pathToFileURL } from 'node:url';

const root = path.resolve(import.meta.dirname, '..');
const args = new Set(process.argv.slice(2));

const serializationFixture = path.join(
  root,
  'fixtures',
  'parse5',
  'test',
  'data',
  'serialization',
  'tests.json'
);

const treeConstructionDir = path.join(
  root,
  'fixtures',
  'parse5',
  'test',
  'data',
  'parser-feedback'
);

function assert(condition, message) {
  if (!condition) {
    throw new Error(message);
  }
}

function readJson(file) {
  return JSON.parse(fs.readFileSync(file, 'utf8'));
}

assert(fs.existsSync(serializationFixture), 'parse5 serialization fixture is missing');
assert(fs.existsSync(treeConstructionDir), 'parse5 parser-feedback fixture directory is missing');

const serializationTests = readJson(serializationFixture);
const parserFeedbackTests = fs
  .readdirSync(treeConstructionDir)
  .filter((file) => file.endsWith('.test'))
  .sort();

if (args.has('--check-fixtures-only')) {
  console.log('parse5 fixture audit passed.');
  console.log(
    JSON.stringify(
      {
        serializationTests: serializationTests.length,
        parserFeedbackFiles: parserFeedbackTests.length,
      },
      null,
      2
    )
  );
  process.exit(0);
}

const portEntry = process.env.PORT_PARSE5_ENTRY
  ? path.resolve(process.env.PORT_PARSE5_ENTRY)
  : path.join(root, 'packages', 'parse5', 'src', 'index.js');

assert(
  fs.existsSync(portEntry),
  `port package entry is missing at ${portEntry}; set PORT_PARSE5_ENTRY if it moved`
);

const port = await import(pathToFileURL(portEntry).href);

await port.initParse5Wasm?.();

const failures = [];

for (const [idx, test] of serializationTests.entries()) {
  const portHtml = port.serialize(port.parse(test.input, test.options), test.options);

  if (portHtml !== test.expected) {
    failures.push({
      idx,
      name: test.name,
      expected: test.expected,
      portHtml,
    });
  }
}

if (failures.length > 0) {
  console.error(JSON.stringify(failures.slice(0, 10), null, 2));
  throw new Error(`${failures.length} parse5 serialization golden tests failed`);
}

console.log(`parse5 serialization golden tests passed: ${serializationTests.length}`);
