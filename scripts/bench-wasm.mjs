// Compares the wasm-backed parse5 facade against upstream parse5 across the
// API paths that cross the wasm boundary differently.
//
//   npm run build:wasm && node scripts/bench-wasm.mjs [path/to/input.html]
//
// Upstream numbers need a built reference checkout (see README); without it
// only the port is measured. Numbers are machine-dependent: compare ratios.

import fs from 'node:fs';
import path from 'node:path';
import { pathToFileURL } from 'node:url';

const root = path.resolve(import.meta.dirname, '..');
const inputPath =
  process.argv[2] ?? path.join(root, 'fixtures/parse5/test/data/location-info/wiki-42/data.html');
const html = fs.readFileSync(inputPath, 'utf8');

const port = await import(pathToFileURL(path.join(root, 'packages/parse5-wasm/src/index.js')).href);
await port.initParse5Wasm();
const raw = await import(pathToFileURL(path.join(root, 'packages/parse5-wasm/pkg/parse5_wasm.js')).href);

const referencePath = path.join(root, 'reference/parse5/packages/parse5/dist/index.js');
const upstream = fs.existsSync(referencePath) ? await import(pathToFileURL(referencePath).href) : null;

function time(fn, { warmup = 5, iterations = 30 } = {}) {
  for (let i = 0; i < warmup; i++) fn();
  const start = performance.now();
  for (let i = 0; i < iterations; i++) fn();
  return (performance.now() - start) / iterations;
}

function streamParse(lib, chunkSize) {
  const parser = new lib.Parser();
  for (let offset = 0; offset < html.length; offset += chunkSize) {
    parser.tokenizer.write(html.slice(offset, offset + chunkSize), offset + chunkSize >= html.length);
  }
  return parser.document;
}

function tokenize(lib) {
  let count = 0;
  const handler = {};
  for (const name of [
    'onCharacter',
    'onNullCharacter',
    'onWhitespaceCharacter',
    'onStartTag',
    'onEndTag',
    'onComment',
    'onDoctype',
    'onEof',
  ]) {
    handler[name] = () => count++;
  }
  new lib.Tokenizer({}, handler).write(html, true);
  return count;
}

// Shaped like an LWC component template: LWC compiles many small templates,
// so fixed per-call overhead matters as much as throughput.
const smallTemplate = `<template>
  <div class="card" data-id={recordId}>
    <h2 class="title">{title}</h2>
    <template lwc:if={hasItems}>
      <ul>
        <template for:each={items} for:item="item">
          <li key={item.id} onclick={handleClick}>{item.label} &amp; more</li>
        </template>
      </ul>
    </template>
    <template lwc:else><p>No items</p></template>
    <c-child record-id={recordId} onselect={handleSelect}></c-child>
    <slot name="footer"></slot>
  </div>
</template>`;
const lwcOptions = { sourceCodeLocationInfo: true, onParseError: () => {} };

const scenarios = [
  [
    'LWC-shaped: 1 KiB parseFragment',
    (lib) => lib.parseFragment(smallTemplate, lwcOptions),
    { warmup: 200, iterations: 2000 },
  ],
  ['parse', (lib) => lib.parse(html)],
  ['parse + sourceCodeLocationInfo', (lib) => lib.parse(html, { sourceCodeLocationInfo: true })],
  ['parseFragment', (lib) => lib.parseFragment(html)],
  ['Parser, 64 KiB chunks', (lib) => streamParse(lib, 64 * 1024)],
  ['Parser, 4 KiB chunks', (lib) => streamParse(lib, 4 * 1024), { iterations: 5 }],
  ['Tokenizer (SAX path)', tokenize, { iterations: 5 }],
];

const serializeInputs = { port: port.parse(html), upstream: upstream?.parse(html) };
scenarios.push(['serialize', (lib) => lib.serialize(lib === port ? serializeInputs.port : serializeInputs.upstream)]);

console.log(`input: ${path.relative(root, inputPath)} (${(html.length / 1024).toFixed(0)} KiB)\n`);
console.log('scenario                          upstream      wasm port    ratio');

for (const [name, fn, opts] of scenarios) {
  const portMs = time(() => fn(port), opts);
  const upstreamMs = upstream ? time(() => fn(upstream), opts) : NaN;
  console.log(
    `${name.padEnd(32)} ${upstream ? upstreamMs.toFixed(2).padStart(8) + ' ms' : '       n/a'}  ` +
      `${portMs.toFixed(2).padStart(8)} ms  ${upstream ? (portMs / upstreamMs).toFixed(1).padStart(5) + 'x' : ''}`
  );
}

// Where the time goes on the default parse path.
const json = JSON.stringify(raw.parse(html, {}));
const jsonLoc = JSON.stringify(raw.parse(html, { sourceCodeLocationInfo: true }));

console.log('\nbreakdown of parse (wasm port)');
console.log(`  Rust parse + serialize in wasm, no tree transfer  ${time(() => raw.parseFragmentAndSerialize(html)).toFixed(2)} ms`);
console.log(`  raw wasm.parse (parse + JSON + JSON.parse)        ${time(() => raw.parse(html, {})).toFixed(2)} ms`);
console.log(`  JSON.parse alone (${(json.length / 1e6).toFixed(1)} MB)                         ${time(() => JSON.parse(json)).toFixed(2)} ms`);
console.log(`  raw wasm.parse with locations                     ${time(() => raw.parse(html, { sourceCodeLocationInfo: true })).toFixed(2)} ms`);
console.log(`  JSON.parse alone with locations (${(jsonLoc.length / 1e6).toFixed(1)} MB)          ${time(() => JSON.parse(jsonLoc)).toFixed(2)} ms`);
