# fmt, clippy -D warnings, cargo test, reference/ guard. About 10s warm.
check:
    ./scripts/check.sh

# Release wasm build and offline npm tests. About 16s after the first wasm build.
check-wasm:
    ./scripts/check-wasm.sh
