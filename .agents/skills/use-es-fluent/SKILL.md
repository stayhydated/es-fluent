---
name: use-es-fluent
description: Use when adding, changing, documenting, or reviewing es-fluent localization in Rust applications or integrations. Covers choosing the correct public facade (`es-fluent`, CLI, embedded, Dioxus, Bevy, language enum, and public integration crates), deriving typed Fluent messages, wiring managers, running `cargo es-fluent`, and keeping examples/README/book usage guidance aligned.
---

# Use es-fluent

## Core Workflow

Start from the public facade, not an internal crate. Most application code uses `es-fluent` plus exactly one runtime manager:

1. Choose the manager: embedded for CLIs/TUIs/desktop/general Rust, Dioxus for Dioxus client or SSR, Bevy for ECS/assets, and `es-fluent-manager-core` only for custom runtime integrations.
2. Put localizable types in a library target (`src/lib.rs` or a library module). `cargo es-fluent generate` discovers library inventory; binary-only derives in `src/main.rs` are not discovered.
3. Put `define_i18n_module!()` in a library-reachable `src/i18n.rs`, and declare `pub mod i18n;` from `src/lib.rs`.
4. Derive `EsFluent` for messages. Use `EsFluentChoice` for selector fields, `EsFluentVariants` for field/variant labels, and `EsFluentLabel` for type-level labels.
5. Generate and validate FTL through the CLI: `cargo es-fluent generate`, then `cargo es-fluent status --all` or the narrower relevant command.
6. Localize through an explicit context: `i18n.localize_message(&message)` or `MyType::localize_label(&i18n)`. Avoid process-global or context-free lookup in application guidance.

When editing this repository's public usage workflow, keep `examples/readme`, affected READMEs, and matching `book/src/*.md` pages synchronized unless there is a documented reason not to.

## Reference Selection

Load only the reference needed for the task:

- `references/public-facades.md`: dependency and runtime choice, setup snippets, and which crate to use for embedded, Dioxus, Bevy, language enums, or custom integrations.
- `references/derive-and-ftl.md`: derive macro patterns, generated IDs/arguments, namespaces, choices, labels, variants, and FTL generation expectations.
- `references/cli-and-validation.md`: `cargo es-fluent` commands, `i18n.toml`, generated asset layout, doc sync, and validation commands.

Prefer current repo sources over memory when details matter. Good source files are `README.md`, `examples/readme`, `crates/*/README.md`, and `book/src/*.md`.

## Implementation Rules

Use repo-local dependency style when editing this workspace: put versions and paths in the root `Cargo.toml`, use `workspace = true` in member crates, and use explicit `path` dependencies only in examples.

Use `es-fluent-lang` for typed language pickers. Do not hand-write locale enums when `#[es_fluent_language]` can derive them from `i18n.toml` and the locale folders.

For manager macros that scan locale assets at compile time, add rebuild tracking when locale files or folders may change:

```rust
fn main() {
    es_fluent_build::track_i18n_assets();
}
```

with:

```toml
[build-dependencies]
es-fluent-build = "0.16"
```

When public API or behavior changes, update executable examples first when relevant, then mirror the example-first explanation into READMEs and mdBook pages.
