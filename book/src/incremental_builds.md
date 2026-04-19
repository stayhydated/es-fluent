# Incremental Builds

If your crate uses the embedded or Bevy manager macros, they discover locales at compile time by scanning your `assets_dir`. By default, Cargo doesn't know about these files, so changes like renaming a locale folder (e.g., `fr` → `fr-FR`) won't trigger a rebuild.

The `build` feature adds a `build.rs` helper that emits `cargo:rerun-if-changed` directives for your locale assets, ensuring Cargo rebuilds when translations change. Crates that only use the derive macros do not need this setup.

## Setup

Add `es-fluent` to your **build dependencies**:

```toml
[build-dependencies]
es-fluent = { version = "*", features = ["build"] }
```

Call the tracking helper from `build.rs`:

```rust
// build.rs
fn main() {
    es_fluent::build::track_i18n_assets();
}
```

This guarantees your project recompiles whenever locale files or folders are added, removed, or renamed.
