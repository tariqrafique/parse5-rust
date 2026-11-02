---
name: verify-parse5-wasm
description: Build and drive the wasm-backed parse5 package the way LWC does (parseFragment, source locations, onParseError). Use after parser, tokenizer, tree, or packages/parse5-wasm changes, and before claiming the package surface works.
---

# Verify parse5-wasm

Drive the package surface from the repo root. Do not grep `fixtures/` wholesale.

## Launch

```sh
./scripts/check-wasm.sh
```

That checks `wasm-bindgen-cli` against `Cargo.lock`, runs `npm run build:wasm`, then `npm test`.

For a Rust-only change, `./scripts/check.sh` is enough until the wasm boundary is involved.

## Doctor

```sh
node scripts/doctor-parse5-wasm.mjs
```

The doctor fails when `packages/parse5-wasm/pkg/parse5_wasm_bg.wasm` is missing, when it is older than `crates/`, or when `wasm-bindgen --version` disagrees with `Cargo.lock`.

## Drive

```sh
node scripts/drive-parse5-wasm.mjs
```

The script imports `packages/parse5` and calls `parseFragment` with `sourceCodeLocationInfo: true` and `onParseError` on an LWC-shaped template, then checks location fields and a parse-error code. `npm test` runs this as `test:drive`.

Upstream diff: `npm run setup:reference && npm run test:parity`. That step skips, with a message, when `reference/` is absent. Do not make `npm test` depend on it.

## Feature map

| Surface | Where |
|---|---|
| `parseFragment` + locations + `onParseError` | `packages/parse5` → `packages/parse5-wasm/src/index.js` |
| Error code strings | `crates/parse5/src/common/error_codes.rs` and `ErrorCodes` in `index.js` (parity: `cargo test --test js_table_parity`) |
| `serialize` / `serializeOuter` | JS in `index.js`; Rust in `crates/parse5/src/serializer`. `npm run test:serializer-parity` |
| `Parser` streaming | `parse5-parser-stream`, `parse5-plain-text-conversion-stream` |
| `Tokenizer` / SAX | `parse5-sax-parser`, `parse5-html-rewriting-stream` |
