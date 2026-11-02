# Agent runbook

Rust/wasm port of parse5. Compatibility target is `parse5@8.0.1`, especially `parseFragment`, default tree nodes, `sourceCodeLocationInfo`, and `onParseError` for `@lwc/template-compiler`. Details: `docs/lwc-parse5-requirements.md` and `docs/architecture.md`.

## Commands (repo root)

| Command | What it does | Warm time |
|---|---|---|
| `cargo test` | Rust suite against `fixtures/` | ~3s |
| `cargo test --test html5lib_tree_construction` | One integration suite | ~1s |
| `cargo test --test attr_location_spans` | Attribute span cases | <1s |
| `./scripts/check.sh` or `just check` | `cargo fmt --check`, `clippy -D warnings`, `cargo test`, reference guard | ~10s |
| `./scripts/check-wasm.sh` or `just check-wasm` | wasm-bindgen version, `npm run build:wasm`, `npm test` | ~16s after the first wasm build |
| `npm test` | Offline package suite. Needs a built wasm package. Does not need `reference/` | ~4s |
| `npm run setup:reference && npm run test:parity` | Optional upstream and LWC comparison | network + a built reference checkout |

`wasm-bindgen-cli` must match the `wasm-bindgen` version in `Cargo.lock`. `./scripts/check-wasm.sh` prints the install command when it does not.

## Where to look

- Upstream file map: the table in `docs/architecture.md` (`parser/mod.rs` ↔ `parser/index.ts`, and the rest).
- Do not read or grep `fixtures/` wholesale. It is about 7.8 MB. `namedEntities.test` is 1.1 MB and vendored twice (the copies are byte-identical; `npm run test:fixtures` checks that). Use `rg -g '!fixtures'`.
- Offsets are UTF-16 code units everywhere (errors and `sourceCodeLocation`).
- The parser does not use the `TreeAdapter` trait. Call the free functions in `tree_adapters::default`.
- Tag, namespace, foreign-content, and error-code tables exist in both `crates/parse5/src/common/*.rs` and `packages/parse5-wasm/src/index.js`. Change both. `cargo test --test js_table_parity` fails if they diverge.
- The Rust serializer (`crates/parse5/src/serializer`) and the JS serializer (`packages/parse5-wasm/src/index.js`) are both checked against `fixtures/parse5/test/data/serialization/tests.json` by `npm run test:serializer-parity`.

## Adding a behavior fix

1. Take the expected value from parse5 8.0.1. `scripts/attr-span-reference.mjs` reads `PARSE5_PATH` or `reference/parse5/packages/parse5`.
2. Add a failing case under `crates/parse5/tests/`, commit, then fix.

## Rules and what enforces them

| Rule | Enforced by |
|---|---|
| Default tests do not mention `reference/` | `scripts/check-no-reference.mjs` in `./scripts/check.sh`, `npm test`, and CI |
| fmt and clippy are clean | `./scripts/check.sh` and `.github/workflows/ci.yml`; `[workspace.lints]` denies unused Rust lints and `clippy::all` |
| Rust and JS tag, namespace, foreign-content, and error-code tables match | `cargo test --test js_table_parity` (`npm run test:tables-parity`) |
| Rust and JS serializers agree on the serialization fixture | `npm run test:serializer-parity` |
| `namedEntities.test` copies stay byte-identical | `npm run test:fixtures` |
| Do not edit `reference/` | `.gitignore` (`reference/` is untracked) |
| CI runs the same commands as local | `.github/workflows/ci.yml` calls `./scripts/check.sh` and `./scripts/check-wasm.sh` |

Do not edit files under `reference/` unless explicitly asked. Normal tests use `fixtures/parse5/test/data`.
