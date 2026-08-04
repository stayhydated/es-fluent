# es-fluent-derive

[![Docs](https://docs.rs/es-fluent-derive/badge.svg)](https://docs.rs/es-fluent-derive/)
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

~~~rust
use es_fluent::{EsFluent, EsFluentLabel};

#[derive(EsFluent, EsFluentLabel)]
pub enum AccountMessage<'a> {
    Welcome { name: &'a str },
    SignedOut,
}
~~~

Derives support argument transforms, selectors, explicit keys, package-local
domains, and namespace-based file splitting. See the
[derive guide](https://stayhydated.github.io/es-fluent/book/deriving_messages.html)
for the public attribute and generated-FTL contract.
