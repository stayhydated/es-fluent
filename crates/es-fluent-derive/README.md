# es-fluent-derive

[![CI](https://github.com/stayhydated/es-fluent/actions/workflows/ci.yml/badge.svg)](https://github.com/stayhydated/es-fluent/actions/workflows/ci.yml)
[![Codecov](https://codecov.io/gh/stayhydated/es-fluent/branch/master/graph/badge.svg)](https://codecov.io/gh/stayhydated/es-fluent)
[![Book](https://img.shields.io/badge/book-online-blue)](https://stayhydated.github.io/es-fluent/book/)
[![Crates.io](https://img.shields.io/crates/v/es-fluent-derive.svg)](https://crates.io/crates/es-fluent-derive)

Procedural macros behind the `es-fluent` typed-message facade.
Applications should import these derives from
[`es-fluent`](../es-fluent/README.md) rather than depending on this
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
