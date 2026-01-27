[![Docs](https://docs.rs/es-fluent-cli-helpers/badge.svg)](https://docs.rs/es-fluent-cli-helpers/)
[![Crates.io](https://img.shields.io/crates/v/es-fluent-cli-helpers.svg)](https://crates.io/crates/es-fluent-cli-helpers)

# es-fluent-cli-helpers

**Internal Crate**: Runtime library for the `es-fluent` runner crate.

The `es-fluent` CLI works by generating a temporary "runner crate" to inspect your project's types. This library is injected into that runner crate to perform the actual extraction and generation work, including namespace validation when configured in `i18n.toml`.
