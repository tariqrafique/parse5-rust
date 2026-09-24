# parse5-rust

A Rust and WebAssembly port of the [`parse5`](https://github.com/inikulin/parse5) HTML parser ecosystem.

The core parser, tokenizer, serializer, and default tree adapter live in Rust under `crates/parse5`. The npm packages under `packages/` provide parse5-compatible JavaScript entrypoints backed by the Rust implementation compiled to wasm.

## Status

This repository is an active port, currently focused on compatibility with parse5 8.x and the API surface used by LWC's `@lwc/template-compiler`.

For LWC, the direct must-port JavaScript surface is:

- `parse5`
- `@parse5/tools`

LWC does not import `parse5-parser-stream`, `parse5-sax-parser`, `parse5-html-rewriting-stream`, `parse5-plain-text-conversion-stream`, or `parse5-htmlparser2-tree-adapter`. Those sibling package surfaces are included for broader parse5 ecosystem parity and upstream fixture coverage, not because LWC requires them.

Implemented package surfaces currently include:

- `parse5`
- `parse5-wasm`
- `parse5-parser-stream`
- `parse5-sax-parser`
- `parse5-plain-text-conversion-stream`
- `parse5-html-rewriting-stream`
- `parse5-htmlparser2-tree-adapter`
- `@parse5/tools`

## Layout

- `crates/parse5` - core Rust parser, tokenizer, serializer, tree adapter, and upstream fixture tests.
- `crates/parse5-wasm` - wasm-bindgen bindings over the Rust crate.
- `packages/parse5` - parse5-compatible npm facade that initializes and re-exports the wasm-backed API.
- `packages/parse5-wasm` - JavaScript compatibility bridge around the generated wasm module.
- `packages/parse5-*` - sibling parse5 package facades and stream packages.
- `fixtures/parse5/test/data` - vendored upstream parse5 fixture data used by normal tests.
- `reference/` - optional local upstream checkouts, ignored by git, for source reading and external compatibility checks.

## Prerequisites

- Rust with the `wasm32-unknown-unknown` target.
- Node.js and npm.
- `wasm-bindgen` CLI available on `PATH`.

If Rust was just installed, reload your shell environment first:

```sh
. "$HOME/.cargo/env"
```

Install the wasm target and wasm-bindgen CLI if needed:

```sh
rustup target add wasm32-unknown-unknown
cargo install wasm-bindgen-cli
```

## Build

Build the wasm package:

```sh
npm run build:wasm
```

This generates `packages/parse5-wasm/pkg/`, which is build output and is intentionally ignored by git.

## Test

Run the Rust test suite:

```sh
cargo test
```

Run the npm package suite (requires a built wasm package, see Build):

```sh
npm test
```

Run lint checks:

```sh
cargo clippy --all-targets --all-features -- -D warnings
```

These tests use checked-in fixtures under `fixtures/parse5/test/data`. The one exception is `npm run test:wasm-runtime` (the first step of `npm test`), which compares the port's exports and output against a built upstream parse5 at `reference/parse5/packages/parse5/dist`. To prepare it for the `parse5@8.0.1` compatibility target:

```sh
git clone --depth 1 --branch v8.0.1 https://github.com/inikulin/parse5.git reference/parse5
(cd reference/parse5 && npm ci --ignore-scripts && npx tsc --build packages/parse5)
```

## Reference Checkouts

Reference checkouts are optional and live under `reference/`, which is ignored by git.

```sh
npm run setup:reference
```

Use these checkouts for reading upstream source, refreshing vendored fixtures deliberately, and running downstream integration checks. Do not make normal build or test commands depend on `reference/parse5`.

## LWC Integration Check

The final downstream check for this port is LWC's template compiler with local `parse5` and `@parse5/tools` swapped in.

After setting up `reference/lwc` and its dependencies, run:

```sh
cd reference/lwc
corepack yarn vitest run packages/@lwc/template-compiler
```

This check is intentionally downstream validation. Upstream parse5 fixture parity should guide port completeness. Passing this check does not imply LWC uses the sibling parse5 packages; the sibling packages are tested separately by this repo's own npm suite.

## Licensing

This port is licensed under the MIT License. See `LICENSE`.

Vendored parse5 fixture data keeps its upstream license notice in `fixtures/parse5/LICENSE`. The html5lib fixture directories also include their own license files.
