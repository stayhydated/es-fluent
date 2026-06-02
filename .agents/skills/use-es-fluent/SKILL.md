---
name: use-es-fluent
description: 'Use when adding, changing, documenting, or reviewing user-facing es-fluent i18n/localization in Rust applications. Scope is public application workflow only: choosing `es-fluent`, using the `cargo es-fluent` CLI, embedded/Dioxus/Bevy managers, typed language enums, deriving Fluent messages, generated FTL, and explicit manager contexts. Do not use for es-fluent repository internals, maintenance tooling, crate architecture, release automation, or contributor-only implementation work.'
---

# Use es-fluent

## Scope Boundary

Treat this skill as a hosted public-usage guide for es-fluent consumers. Even when working inside an es-fluent checkout, use this skill only for user-facing workflows: application setup, public crate usage, public CLI behavior, generated FTL expectations, and docs or examples intended for application developers.

Do not use this skill as a contributor guide for es-fluent repository internals. For workspace maintenance, internal crate implementation, architecture docs, CI, releases, or maintainer-only workflows, read the repository source, `AGENTS.md`, and the relevant crate documentation directly.

## Core Workflow

Start from the user-facing facade. Most application code uses `es-fluent` plus exactly one runtime manager:

1. Choose the manager: embedded for CLIs/TUIs/desktop/general Rust, Dioxus for Dioxus client or SSR, and Bevy for ECS/assets.
2. Put localizable types in a library target (`src/lib.rs` or a library module). `cargo es-fluent generate` discovers library inventory; binary-only derives in `src/main.rs` are not discovered.
3. Put `define_i18n_module!()` in a library-reachable `src/i18n.rs`, and declare `pub mod i18n;` from `src/lib.rs`.
4. Derive `EsFluent` for messages. Use `EsFluentChoice` for selector fields, `EsFluentVariants` for field/variant labels, and `EsFluentLabel` for type-level labels.
5. Generate and inspect FTL through the es-fluent CLI: `cargo es-fluent generate`, then `cargo es-fluent status --all` or the narrower relevant command.
6. Localize through an explicit context: `i18n.localize_message(&message)` or `MyType::localize_label(&i18n)`.

## Reference Selection

Load only the reference needed for the task:

- `references/public-facades.md`: dependency and runtime choice, setup snippets, and which crate to use for embedded, Dioxus, Bevy, or language enums.
- `references/derive-and-ftl.md`: derive macro patterns, generated IDs/arguments, namespaces, choices, labels, variants, and FTL generation expectations.
- `references/cli-workflow.md`: `cargo es-fluent` commands, `i18n.toml`, and generated asset layout.

Prefer current public docs or source examples over memory when details matter.

## Implementation Rules

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
