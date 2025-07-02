Build.rs script for generating Fluent localization files from Rust source code.

## Parse Modes

The `FluentBuilder` supports two modes:

### Aggressive Mode
Warning : Flushes and rewrites all entries.

```rust
// build.rs
es_fluent_build::FluentBuilder::new()
    .mode(es_fluent_build::FluentParseMode::Aggressive)
    .build()
```

### Conservative Mode (default)
Adds new entries while preserving all existing ones. Useful when you want to avoid losing existing work when things move around.

```rust
// build.rs
es_fluent_build::FluentBuilder::new()
    .mode(es_fluent_build::FluentParseMode::Conservative)
    .build()
```

## Note
- the parser will be aware of the `#[strum_discriminants(...)]` attributes, and will generate entries for them.
