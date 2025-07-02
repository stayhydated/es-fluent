Cli (TUI) for generating Fluent localization files from Rust source code.

The cli will watch over each crate that contains a `i18n.toml` file.

## Parse Modes

`es-fluent-cli` supports two modes:

### Aggressive Mode
Warning : Flushes and rewrites all entries.

```sh
es-fluent-cli --mode aggressive
```

### Conservative Mode (default)
Adds new entries while preserving all existing ones. Useful when you want to avoid losing existing work when things move around.

```sh
es-fluent-cli --mode conservative
```

## Note
- the parser will be aware of the `#[strum_discriminants(...)]` attributes, and will generate entries for them.
