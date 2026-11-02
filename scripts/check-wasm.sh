#!/usr/bin/env bash
# Wasm + offline npm gate. Requires wasm-bindgen-cli at the Cargo.lock version.
set -euo pipefail
cd "$(dirname "$0")/.."

lock_version="$(node scripts/wasm-bindgen-version.mjs)"

if ! command -v wasm-bindgen >/dev/null 2>&1; then
  echo "wasm-bindgen-cli is not installed."
  echo "Install the Cargo.lock version with:"
  echo "  cargo install wasm-bindgen-cli --version ${lock_version} --locked"
  exit 1
fi

installed="$(wasm-bindgen --version | awk '{print $2}')"
if [[ "${installed}" != "${lock_version}" ]]; then
  echo "wasm-bindgen-cli ${installed} does not match Cargo.lock ${lock_version}."
  echo "Install the matching version with:"
  echo "  cargo install wasm-bindgen-cli --version ${lock_version} --locked"
  exit 1
fi

npm run build:wasm
node scripts/doctor-parse5-wasm.mjs
npm test
