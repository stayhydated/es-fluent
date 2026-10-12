# es-fluent-derive

[![Codecov: es-fluent-derive][codecov-badge]][codecov]
[![crates.io: es-fluent-derive][crate-badge]][crate]

Procedural macros behind the [`es-fluent`][facade] typed-message facade.
Applications use the facade's default `derive` feature and its re-exports;
a direct dependency serves integrations that need the proc-macro entry points.

Requires Rust 1.99 or newer.

## Overview

- `EsFluent` derives struct and enum messages, including selector values for unit-only enums.
- `EsFluentChoice` derives selector-only enums.
- `EsFluentVariants` derives field or variant messages.
- `EsFluentLabel` derives type labels.

Derives support argument transforms, explicit keys, package-local domains, and
namespace-based file splitting.

[facade]: https://github.com/stayhydated/es-fluent/blob/master/crates/es-fluent/README.md
[codecov-badge]: https://codecov.io/gh/stayhydated/es-fluent/branch/master/graph/badge.svg?component=es-fluent-derive
[codecov]: https://codecov.io/gh/stayhydated/es-fluent
[crate-badge]: https://img.shields.io/crates/v/es-fluent-derive.svg?label=es-fluent-derive
[crate]: https://crates.io/crates/es-fluent-derive
