# Public facades and setup

Use this reference to select dependencies, configure package ownership, or
establish a new es-fluent project. Read
[Runtime managers](managers.md) for framework-specific code.

## Select dependencies

| Need | Use |
| --- | --- |
| Typed messages, labels, variants, and choices | `es-fluent` |
| General Rust runtime | `es-fluent-manager-embedded` |
| Dioxus client or SSR | `es-fluent-manager-dioxus` with the relevant features |
| Bevy ECS and UI | `es-fluent-manager-bevy` |
| Typed language picker | `es-fluent-lang` |
| FTL generation and validation | `es-fluent-cli` as `cargo es-fluent` |
| Locale tracking and strict fallback catalog | `es-fluent-build` under `[build-dependencies]` |

Most applications should not depend directly on `es-fluent-derive`,
`es-fluent-lang-macro`, `es-fluent-manager-core`, or
`es-fluent-manager-macros`.

## Compatible release lines

- `es-fluent`, CLI, embedded manager, and language enum:
  `0.18.x`.
- Dioxus manager: `0.7.x` with Dioxus `0.7.x`.
- Bevy manager: `0.19.x` with Bevy `0.19.x`.

Use the versions already declared by the target repository rather than
rewriting manifests to these examples mechanically.

## Configure one owner package

Create `i18n.toml` next to the owner package's
`Cargo.toml`:

~~~toml
fallback_language = "en"
assets_dir = "assets/locales"

# Optional compilation features for derive inventory.
# fluent_feature = ["my-feature"]

# Optional literal namespace allowlist.
# namespaces = ["ui", "errors"]

# Optional package-local missing-message policy; strict is the default.
# missing_message_policy = "fallback-str"

# Optional additional package-local domains.
# domains = ["emails"]

# Optional fallback-copy warning policy.
# check_fallback_copies = false
~~~

Create the fallback locale directory and expose a manager module from the
library target:

~~~rust
// src/lib.rs
pub mod i18n;
~~~

The concrete manager reference supplies the matching
`define_i18n_module!()` invocation.

Locale names must be canonical BCP-47 tags. `assets_dir` stays inside
the package root; existing path components and locale targets must not be
symlinks.

## Track asset changes

Configured derives and manager macros consume locale data at compile time. Add
the helper to track changes and write the fallback-message catalog:

~~~toml
[build-dependencies]
es-fluent-build = "0.18"
~~~

~~~rust
// build.rs
fn main() {
    es_fluent_build::track_i18n_assets();
}
~~~

Run `cargo es-fluent doctor` to verify the build dependency, Cargo-selected
custom-build target, manager registration in the library target graph, fallback
locale, and catalog inputs. Treat warnings as requests for manual verification
when static inspection cannot prove the integration.

## Compose a workspace

Each package that declares localizable types owns:

- its `i18n.toml`;
- its fallback and translated FTL resources;
- its library-reachable manager module; and
- its build-script tracking.

The Cargo package name is the implicit default domain even when the library
target has a custom name or a host renames the dependency. Additional domains
remain package-local.

The host enables the selected manager feature on every owner, links those
crates, and creates one runtime manager. It does not copy dependency FTL.

Use workspace-root generation and validation for changes spanning owners:

~~~sh
cargo es-fluent generate --path .
cargo es-fluent status --path . --all-locales
cargo es-fluent check --path . --all-locales
~~~

Separate packages may reuse the same domain name and generated ID. Strict and
fallback-string packages may coexist in one workspace build.
