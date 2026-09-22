# es-fluent-derive

[![Codecov: es-fluent-derive][codecov-badge]][codecov]
[![crates.io: es-fluent-derive][crate-badge]][crate]

Procedural macros behind the `es-fluent` typed-message facade.
Applications should import these derives from
[`es-fluent`][es-fluent] rather than depending on this
crate directly.

The crate implements:

- `EsFluent` for structs and enum messages;
- `EsFluentChoice` for standalone selector enums;
- `EsFluentVariants` for field or variant metadata; and
- `EsFluentLabel` for type-level labels.

~~~rust,ignore
use es_fluent::{EsFluent, EsFluentLabel};

#[derive(EsFluent, EsFluentLabel)]
pub enum AccountMessage<'a> {
    Welcome { name: &'a str },
    SignedOut,
}
~~~

Derives support argument transforms, selectors, explicit keys, package-local
domains, and namespace-based file splitting.

[es-fluent]: https://github.com/stayhydated/es-fluent/blob/master/crates/es-fluent/README.md
[codecov-badge]: https://codecov.io/gh/stayhydated/es-fluent/branch/master/graph/badge.svg?component=es-fluent-derive
[codecov]: https://codecov.io/gh/stayhydated/es-fluent
[crate-badge]: https://img.shields.io/crates/v/es-fluent-derive.svg?label=es-fluent-derive
[crate]: https://crates.io/crates/es-fluent-derive
