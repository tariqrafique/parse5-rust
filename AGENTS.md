# Repository Notes

This repository is for a Rust/wasm port of `parse5`, with upstream JavaScript projects checked out locally for reference and compatibility testing.

## Vendored Fixtures

The self-contained test fixtures live under `fixtures/parse5/test/data`. These are checked-in copies of upstream parse5 test data and are the source of truth for normal `cargo test` and npm package tests.

Do not make tests depend on `reference/parse5/test/data`; `reference/parse5` is for reading upstream source and deliberately refreshing the vendored fixture copy.

## Reference Sources

Reference repositories live under `reference/` and are intentionally treated as external source checkouts, not code to edit as part of this port:

- `reference/parse5`: upstream `parse5` source from `https://github.com/inikulin/parse5.git`
- `reference/lwc`: upstream LWC source from `https://github.com/salesforce/lwc.git`

Set them up with:

```sh
./scripts/setup-reference.sh
```

The setup script clones missing repositories only. If a reference checkout already exists, it prints the current branch and commit and leaves it alone. It also initializes the parse5 html5lib fixture submodules so vendored fixtures can be refreshed deliberately when needed.

To use a different branch or tag for a fresh clone:

```sh
PARSE5_REF=master LWC_REF=master ./scripts/setup-reference.sh
```

## Compatibility Target

The first compatibility target is `parse5@8.0.1`, especially the API surface used by `@lwc/template-compiler`:

- `parseFragment`
- default tree adapter node shapes and type guards
- `sourceCodeLocationInfo`
- `onParseError` with parse5-compatible error codes and locations

Current LWC keeps `parse5` and `@parse5/tools` in `devDependencies` and bundles their relevant code into `@lwc/template-compiler/dist`. For final validation, rebuild or alias LWC source against the wasm-backed parse5 package rather than only swapping a runtime dependency in the published package.

See `docs/lwc-parse5-requirements.md` for the detailed must-port API surface and LWC call sites.

## Working Rules

- Do not edit files under `reference/` unless explicitly asked.
- Prefer adding focused Rust/wasm implementation and tests in this repo root.
- Use checked-in fixtures for normal parse5 test runs. Use the upstream reference checkout and LWC template compiler fixtures as external compatibility checks.
