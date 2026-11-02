import { execFileSync } from 'node:child_process';
import fs from 'node:fs';
import path from 'node:path';

const root = path.resolve(import.meta.dirname, '..');
const wasm = path.join(root, 'packages/parse5-wasm/pkg/parse5_wasm_bg.wasm');

function fail(message) {
  console.error(message);
  process.exit(1);
}

if (!fs.existsSync(wasm)) {
  fail('packages/parse5-wasm/pkg/parse5_wasm_bg.wasm is missing. Run npm run build:wasm.');
}

const wasmTime = fs.statSync(wasm).mtimeMs;
const crateRoot = path.join(root, 'crates');

function newestUnder(dir) {
  let newest = 0;
  for (const entry of fs.readdirSync(dir, { withFileTypes: true })) {
    if (entry.name === 'target') {
      continue;
    }
    const abs = path.join(dir, entry.name);
    if (entry.isDirectory()) {
      newest = Math.max(newest, newestUnder(abs));
    } else if (entry.name.endsWith('.rs') || entry.name === 'Cargo.toml') {
      newest = Math.max(newest, fs.statSync(abs).mtimeMs);
    }
  }
  return newest;
}

const crateTime = newestUnder(crateRoot);
if (wasmTime + 1 < crateTime) {
  fail('parse5_wasm_bg.wasm is older than crates/. Run npm run build:wasm.');
}

const expectedVersion = execFileSync(process.execPath, ['scripts/wasm-bindgen-version.mjs'], {
  cwd: root,
  encoding: 'utf8',
}).trim();

let installed = '';
try {
  installed = execFileSync('wasm-bindgen', ['--version'], { encoding: 'utf8' }).trim().split(/\s+/).at(-1);
} catch {
  fail(
    `wasm-bindgen-cli is not installed. Install ${expectedVersion} with: cargo install wasm-bindgen-cli --version ${expectedVersion} --locked`
  );
}

if (installed !== expectedVersion) {
  fail(
    `wasm-bindgen-cli ${installed} does not match Cargo.lock ${expectedVersion}. Install with: cargo install wasm-bindgen-cli --version ${expectedVersion} --locked`
  );
}

console.log(`doctor ok: wasm bundle is current, wasm-bindgen-cli ${installed}`);
