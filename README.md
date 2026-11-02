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

See [`docs/architecture.md`](docs/architecture.md) for how the layers fit together and for current performance numbers (`npm run bench:wasm`).

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

Install the wasm target and wasm-bindgen CLI if needed. The CLI version has to match `wasm-bindgen` in `Cargo.lock` (`./scripts/check-wasm.sh` checks):

```sh
rustup target add wasm32-unknown-unknown
cargo install wasm-bindgen-cli --version "$(node scripts/wasm-bindgen-version.mjs)" --locked
```

## Build

Build the wasm package:

```sh
npm run build:wasm
```

This generates `packages/parse5-wasm/pkg/`, which is build output and is intentionally ignored by git.

## Test

Run the Rust gate (format, clippy, tests):

```sh
./scripts/check.sh
```

Run the wasm and npm gate (requires `wasm-bindgen-cli`, see Prerequisites):

```sh
./scripts/check-wasm.sh
```

`npm test` alone runs the offline package suite. It uses checked-in fixtures under `fixtures/parse5/test/data` and a built wasm package. It does not need `reference/`.

Optional comparison against upstream parse5 and the LWC checkout:

```sh
npm run setup:reference
npm run test:parity
```

`test:parity` skips, and says so, when those checkouts are missing.

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
