# es-fluent-build Architecture

`es-fluent-build` is the build-script helper crate for the `es-fluent`
workspace.

## Responsibilities

1. Read `i18n.toml` from `CARGO_MANIFEST_DIR`.
2. Resolve the configured locale asset directory through `es-fluent-toml`.
3. Emit `cargo:rerun-if-changed` directives for `i18n.toml` and the resolved
   asset directory.

The crate intentionally keeps this API outside the `es-fluent` facade so normal
runtime users do not pull `i18n.toml` parsing into their dependency graph.
