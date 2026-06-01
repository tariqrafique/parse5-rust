Vendored parse5 upstream test fixtures.

The files under `test/data` were copied from the upstream parse5 repository's
`test/data` directory so this Rust port can build and run its parity tests
without depending on a sibling `reference/parse5` checkout.

The `reference/parse5` checkout remains useful for reading upstream source and
refreshing fixtures deliberately, but normal cargo/npm test runs should use this
checked-in fixture copy.
