# Introduction

`es-fluent` is a localization (i18n) ecosystem for Rust built on top of [Project Fluent](https://projectfluent.org/). It provides type-safe, ergonomic derive macros to link your Rust types directly to Fluent `.ftl` translation files.

The core philosophy:

- **Type Safety**: Your code and translation files stay in sync — mismatches are caught at compile time.
- **Ergonomics**: A single `#[derive(EsFluent)]` on a struct or enum is all you need.
- **Developer Experience**: A CLI generates FTL file skeletons, validates keys, and keeps everything consistent.

## What This Book Covers

1. [**Workspace Crates**](workspace_map.md) — Which crates you depend on directly and which support crates you usually won't need directly.
2. [**Getting Started**](getting_started.md) — Installation, configuration, and a working end-to-end example.
3. [**Deriving Messages**](deriving_messages.md) — Mapping structs and enums to FTL message keys using `EsFluent`, `EsFluentChoice`, `EsFluentVariants`, and `EsFluentThis`.
4. [**Namespaces & File Splitting**](namespaces.md) — Organizing translations into multiple FTL files.
5. [**Language Enum**](language_enum.md) — Auto-generating a type-safe `Languages` enum from your locale folders.
6. [**Runtime Managers**](managers.md) — Loading and resolving translations at runtime with the embedded, Dioxus, or Bevy manager.
7. [**CLI Tooling**](cli.md) — Generating, validating, syncing, cleaning, formatting, and inspecting FTL files from the command line.
8. [**Incremental Builds**](incremental_builds.md) — Ensuring Cargo rebuilds when locale files change.
