# es-fluent-derive-core

[![Docs](https://docs.rs/es-fluent-derive-core/badge.svg)](https://docs.rs/es-fluent-derive-core/)
[![Crates.io](https://img.shields.io/crates/v/es-fluent-derive-core.svg)](https://crates.io/crates/es-fluent-derive-core)

Build-time parsing, validation, and code-generation support for
`es-fluent-derive`. It is separated from the proc-macro crate so the
same derive rules can be reused without exposing proc-macro entry points.

Applications should depend on [`es-fluent`](../es-fluent/README.md).
Proc-macro integrations should normally use
[`es-fluent-derive`](../es-fluent-derive/README.md).
