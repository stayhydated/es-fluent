# Introduction

`es-fluent` is a localization (i18n) ecosystem for Rust built on top of [Project Fluent](https://projectfluent.org/). It provides type-safe, ergonomic derive macros to link your Rust types directly to Fluent `.ftl` translation files.

The core philosophy:

- **Type Safety**: Your code and translation files stay in sync — mismatches are caught at compile time.
- **Ergonomics**: A single `#[derive(EsFluent)]` on a struct or enum is all you need.
- **Developer Experience**: A CLI generates FTL file skeletons, validates keys, and keeps everything consistent.

## What This Book Covers

1. [**Getting Started**](getting_started.md) — Installation, configuration, and a working end-to-end example.
2. [**Deriving Messages**](deriving_messages.md) — Mapping structs and enums to FTL message keys using `EsFluent`, `EsFluentChoice`, `EsFluentVariants`, and `EsFluentThis`.
3. [**Namespaces & File Splitting**](namespaces.md) — Organizing translations into multiple FTL files.
4. [**Language Enum**](language_enum.md) — Auto-generating a type-safe `Languages` enum from your locale folders.
5. [**Runtime Managers**](managers.md) — Loading and resolving translations at runtime with the embedded or Bevy manager.
6. [**CLI Tooling**](cli.md) — Generating, validating, syncing, and cleaning FTL files from the command line.
7. [**Incremental Builds**](incremental_builds.md) — Ensuring Cargo rebuilds when locale files change.
