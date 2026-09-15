# es-fluent-derive-core

[![CI](https://github.com/stayhydated/es-fluent/actions/workflows/ci.yml/badge.svg)](https://github.com/stayhydated/es-fluent/actions/workflows/ci.yml)
[![Codecov](https://codecov.io/gh/stayhydated/es-fluent/branch/master/graph/badge.svg)](https://codecov.io/gh/stayhydated/es-fluent)
[![Book](https://img.shields.io/badge/book-online-blue)](https://stayhydated.github.io/es-fluent/book/)
[![Crates.io](https://img.shields.io/crates/v/es-fluent-derive-core.svg)](https://crates.io/crates/es-fluent-derive-core)

Build-time parsing, validation, and code-generation support for
`es-fluent-derive`. It is separated from the proc-macro crate so the
same derive rules can be reused without exposing proc-macro entry points.

Applications should depend on [`es-fluent`](../es-fluent/README.md).
Proc-macro integrations should normally use
[`es-fluent-derive`](../es-fluent-derive/README.md).
