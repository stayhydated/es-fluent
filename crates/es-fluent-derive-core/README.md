# es-fluent-derive-core

[![Codecov: es-fluent-derive-core][codecov-badge]][codecov]
[![crates.io: es-fluent-derive-core][crate-badge]][crate]

Build-time parsing, validation, and code-generation support for
`es-fluent-derive`. It is separated from the proc-macro crate so the
same derive rules can be reused without exposing proc-macro entry points.

Applications should depend on [`es-fluent`][es-fluent].
Proc-macro integrations should normally use
[`es-fluent-derive`][es-fluent-derive].

[es-fluent]: https://github.com/stayhydated/es-fluent/blob/master/crates/es-fluent/README.md
[es-fluent-derive]: https://github.com/stayhydated/es-fluent/blob/master/crates/es-fluent-derive/README.md
[codecov-badge]: https://codecov.io/gh/stayhydated/es-fluent/branch/master/graph/badge.svg?component=es-fluent-derive-core
[codecov]: https://codecov.io/gh/stayhydated/es-fluent
[crate-badge]: https://img.shields.io/crates/v/es-fluent-derive-core.svg?label=es-fluent-derive-core
[crate]: https://crates.io/crates/es-fluent-derive-core
