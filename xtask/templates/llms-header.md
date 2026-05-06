# es-fluent

> es-fluent is a Rust localization ecosystem built on Project Fluent focused on type safety, ergonomics, and developer experience. It uses derive macros and tooling to keep Rust types and FTL messages in sync.

Key features:

- Derive macros (`EsFluent`, `EsFluentChoice`, `EsFluentVariants`, `EsFluentLabel`) for strongly typed message keys and arguments
- `cargo es-fluent` CLI support for checking and generating FTL skeletons
- Compile-time namespace and argument validation aligned with `i18n.toml`
- Language enum generation from locale assets via `es-fluent-lang-macro`
- Runtime integrations via `es-fluent-manager-embedded` and `es-fluent-manager-bevy`
