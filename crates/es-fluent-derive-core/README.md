[![Docs](https://docs.rs/es-fluent-derive-core/badge.svg)](https://docs.rs/es-fluent-derive-core/)
[![Crates.io](https://img.shields.io/crates/v/es-fluent-derive-core.svg)](https://crates.io/crates/es-fluent-derive-core)

# es-fluent-derive-core

**Internal Crate**: Build-time logic for `es-fluent`.

This crate contains the logic required by `es-fluent-derive` to parse attributes (including namespace options), validate structures, and generate naming conventions. It is separated from the proc-macro crate to keep compile times minimal.
