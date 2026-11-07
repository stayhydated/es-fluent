# Incremental builds

Configured crates discover locale assets and validate derived messages against
the fallback locale at compile time. Cargo also needs explicit asset tracking so
locale changes, additions, renames, and deletions trigger that work again.

The `es-fluent-build` helper emits the rebuild directives and writes a build
manifest with `i18n.toml`, the discovered locales and resources, and a catalog
of resolvable fallback messages. Derive, language, and manager macros expand
from that manifest, and derive output uses its catalog to make a missing
fallback message value a compile-time error. Macros reference the manifest's
content hash, so compiler caches such as sccache or kache re-expand them when
the manifest changes.

## Setup

Add `es-fluent-build` to your **build dependencies**:

```toml
[build-dependencies]
es-fluent-build = "0.22"
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
Cargo reruns the build script when configured locale files or directories
change, including additions, removals, and renames.

With crate-root assets (`assets_dir = "."`), Cargo watches `i18n.toml` and each
existing locale directory rather than the crate root, whose `target/` directory
changes on every build. After adding a locale directory at the crate root, save
`i18n.toml` so Cargo reruns the helper. Run `cargo es-fluent doctor` to verify the helper
through Cargo's selected target and its local module graph. A warning means
static inspection could not prove the integration and requires manual
verification.
