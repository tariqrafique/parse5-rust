// Fails when a default test script or Rust integration test mentions reference/.
// Upstream comparison belongs in `npm run test:parity`, which is allowed to.

import fs from 'node:fs';
import path from 'node:path';

const root = path.resolve(import.meta.dirname, '..');
const packageJson = JSON.parse(fs.readFileSync(path.join(root, 'package.json'), 'utf8'));

function assert(condition, message) {
  if (!condition) {
    console.error(message);
    process.exit(1);
  }
}

function filesReachedBy(scriptName, seen = new Set()) {
  if (seen.has(scriptName)) {
    return [];
  }
  seen.add(scriptName);
  const body = packageJson.scripts[scriptName];
  assert(body, `package.json is missing script ${scriptName}`);
  const files = [];
  for (const match of body.matchAll(/node\s+(scripts\/\S+)/g)) {
    files.push(match[1]);
  }
  for (const match of body.matchAll(/npm run ([^\s&]+)/g)) {
    files.push(...filesReachedBy(match[1], seen));
  }
  return files;
}

function walk(dir, out = []) {
  if (!fs.existsSync(dir)) {
    return out;
  }
  for (const entry of fs.readdirSync(dir, { withFileTypes: true })) {
    const abs = path.join(dir, entry.name);
    if (entry.isDirectory()) {
      walk(abs, out);
    } else if (entry.name.endsWith('.rs')) {
      out.push(abs);
    }
  }
  return out;
}

const offenders = [];

for (const relative of filesReachedBy('test')) {
  if (relative === 'scripts/check-no-reference.mjs') {
    continue;
  }
  const abs = path.join(root, relative);
  const text = fs.readFileSync(abs, 'utf8');
  if (text.includes('reference/')) {
    offenders.push(relative);
  }
}

for (const crateTests of walk(path.join(root, 'crates'))) {
  if (!crateTests.includes(`${path.sep}tests${path.sep}`)) {
    continue;
  }
  const text = fs.readFileSync(crateTests, 'utf8');
  if (text.includes('reference/')) {
    offenders.push(path.relative(root, crateTests));
  }
}

if (offenders.length > 0) {
  console.error('Default tests must not mention reference/. Move the check to npm run test:parity.');
  for (const file of offenders) {
    console.error(`  ${file}`);
  }
  process.exit(1);
}

console.log('reference/ guard passed for npm test and crates/*/tests');
