# Incremental Builds

If your crate uses the embedded, Dioxus, or Bevy manager macros, they discover locales at compile time by scanning your `assets_dir`. By default, Cargo doesn't know about these files, so changes like renaming a locale folder (e.g., `fr` → `fr-FR`) won't trigger a rebuild.

The `es-fluent-build` crate provides a `build.rs` helper that emits `cargo:rerun-if-changed` directives for your locale assets, ensuring Cargo rebuilds when translations change. Crates that only use the derive macros do not need this setup.

## Setup

Add `es-fluent-build` to your **build dependencies**:

```toml
[build-dependencies]
es-fluent-build = "0.16"
```

Call the tracking helper from `build.rs`:

```rust,no_run
// build.rs
fn main() {
    es_fluent_build::track_i18n_assets();
}
```

This guarantees your project recompiles whenever locale files or folders are added, removed, or renamed.
