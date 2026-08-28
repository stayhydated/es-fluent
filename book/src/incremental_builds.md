# Incremental builds

Configured crates discover locale assets and validate derived messages against
the fallback locale at compile time. Cargo also needs explicit asset tracking so
locale changes, additions, renames, and deletions trigger that work again.

The `es-fluent-build` helper emits the rebuild directives and writes a catalog
of resolvable fallback messages. Derive output uses that catalog to make a
missing fallback message value a compile-time error.

## Setup

Add `es-fluent-build` to your **build dependencies**:

```toml
[build-dependencies]
es-fluent-build = "0.18"
```

Call the tracking helper from Cargo's selected custom-build target. The default
path is `build.rs`:

```rust,no_run
// build.rs
fn main() {
    es_fluent_build::track_i18n_assets();
}
```

A custom `[package] build = "support/i18n.rs"` path uses the same helper call.
This guarantees your project recompiles whenever locale files or folders are
added, removed, or renamed. Run `cargo es-fluent doctor` to verify the helper
through Cargo's selected target and its local module graph. A warning means
static inspection could not prove the integration and requires manual
verification.
