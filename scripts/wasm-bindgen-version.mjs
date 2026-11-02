import fs from 'node:fs';
import path from 'node:path';

const lockPath = path.join(import.meta.dirname, '..', 'Cargo.lock');
const text = fs.readFileSync(lockPath, 'utf8');
const match = text.match(/name = "wasm-bindgen"\nversion = "([^"]+)"/);

if (!match) {
  console.error('wasm-bindgen version not found in Cargo.lock');
  process.exit(1);
}

process.stdout.write(match[1]);
