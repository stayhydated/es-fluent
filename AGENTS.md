# Project Overview

`es-fluent` is a comprehensive localization (i18n) ecosystem written in **Rust**, built on top of [Project Fluent](https://projectfluent.org/). It focuses on:

1. **Type Safety**: Ensuring at compile-time that your code and translation files are in sync.
1. **Ergonomics**: Providing simple macros (like `#[derive(EsFluent)]`) to make struct/enum fields localizable with minimal boilerplate.
1. **Developer Experience**: A robust CLI (`es-fluent-cli`) that auto-generates FTL files, manages keys, and ensures consistency.

## Architecture Documentation Index

| Crate                        | Link to Architecture Doc                                               | Purpose                                                                                     |
| ---------------------------- | ---------------------------------------------------------------------- | ------------------------------------------------------------------------------------------- |
| **Core**                     |                                                                        |                                                                                             |
| `es-fluent`                  | [Architecture](crates/es-fluent/docs/ARCHITECTURE.md)                  | Ecosystem facade, entry point, and registry types.                                          |
| `es-fluent-derive-core`      | [Architecture](crates/es-fluent-derive-core/docs/ARCHITECTURE.md)      | Build-time logic (options, validation, namer) for derive macros.                            |
| `es-fluent-derive`           | [Architecture](crates/es-fluent-derive/docs/ARCHITECTURE.md)           | Proc-macros for registration and trait implementation.                                      |
| `es-fluent-toml`             | [Architecture](crates/es-fluent-toml/docs/ARCHITECTURE.md)             | Configuration (`i18n.toml`) parsing and path resolution.                                    |
| **Language Support**         |                                                                        |                                                                                             |
| `es-fluent-lang`             | [Architecture](crates/es-fluent-lang/docs/ARCHITECTURE.md)             | Runtime language identification and embedded translations.                                  |
| `es-fluent-lang-macro`       | [Architecture](crates/es-fluent-lang-macro/docs/ARCHITECTURE.md)       | Generates type-safe language selection enums from asset folders.                            |
| **Managers**                 |                                                                        |                                                                                             |
| `es-fluent-manager-core`     | [Architecture](crates/es-fluent-manager-core/docs/ARCHITECTURE.md)     | Abstract traits for localization backends.                                                  |
| `es-fluent-manager-embedded` | [Architecture](crates/es-fluent-manager-embedded/docs/ARCHITECTURE.md) | Zero-setup backend for embedding FTL files in binary.                                       |
| `es-fluent-manager-bevy`     | [Architecture](crates/es-fluent-manager-bevy/docs/ARCHITECTURE.md)     | Backend integration for Bevy engine ECS and assets.                                         |
| `es-fluent-manager-macros`   | [Architecture](crates/es-fluent-manager-macros/docs/ARCHITECTURE.md)   | Macros for asset discovery and module registration.                                         |
| **CLI Tool**                 |                                                                        |                                                                                             |
| `es-fluent-cli`              | [Architecture](crates/es-fluent-cli/docs/ARCHITECTURE.md)              | Primary developer-facing CLI (`cargo es-fluent`) for validating and generating FTL files.   |
| **Tooling Internals**        |                                                                        |                                                                                             |
| `es-fluent-cli-helpers`      | [Architecture](crates/es-fluent-cli-helpers/docs/ARCHITECTURE.md)      | Runtime logic for checking/generating FTL files (runner crate).                             |
| `es-fluent-generate`         | [Architecture](crates/es-fluent-generate/docs/ARCHITECTURE.md)         | FTL AST manipulation, diffing, and formatting logic.                                        |
| **Scripts**                  |                                                                        |                                                                                             |
| `cldr-es-fluent-lang`        | [Architecture](scripts/cldr-es-fluent-lang/docs/ARCHITECTURE.md)       | Python script to generate language name data from Unicode CLDR.                             |
| **Web**                      |                                                                        |                                                                                             |
| `web`                        |                                                                        | Astro-based site for GitHub Pages. Hosts WASM-compiled examples (e.g., Bevy) as live demos. |

## Crate Descriptions

### Core Layers

- **`es-fluent`**: The user-facing library. Re-exports everything needed for general usage. Provides registry types (`FtlTypeInfo`, `FtlVariant`, `RegisteredFtlType`) for inventory collection. Connects the global `OnceLock` context to specific backend managers.
- **`es-fluent-derive-core`**: The shared logic library for derive macros. Contains `darling` attribute parsing, validation rules, and FTL key naming algorithms.
- **`es-fluent-derive`**: Provides the `#[derive(EsFluent)]` macro. transforming Rust types into inventory registrations and `FluentDisplay` implementations.
- **`es-fluent-toml`**: Centralizes configuration logic. Ensures that the CLI and proc-macros agree on where assets are located and what languages are available.

### Language & Locale

- **`es-fluent-lang`**: Provides the `I18nModule` for language names themselves (e.g. `lang-en = English`). Useful for UI language pickers.
- **`es-fluent-lang-macro`**: Scans your `assets/` directory to find available languages (e.g., `en/`, `fr/`) and generates an enum (e.g. `enum Languages { En, Fr }`) so you never hardcode language strings.

### Runtime Managers

- **`es-fluent-manager-core`**: Defines the `I18nModule` and `Localizer` traits. It allows the system to be agnostic about _how_ translations are loaded (disk vs embedded).
- **`es-fluent-manager-embedded`**: A "singleton" manager. Initializes a global manager with embedded assets. ideal for CLI tools or simple apps.
- **`es-fluent-manager-bevy`**: A "resource" manager. Hooks into Bevy's `AssetServer` for hot-reloading and ECS reactivity.

### CLI Tool

- **`es-fluent-cli`**: The primary developer-facing command-line tool, installed as `cargo es-fluent`. It provides commands like `check` and `generate` to validate FTL files against your Rust types and auto-generate missing translation keys. Under the hood, it compiles a temporary "runner crate" that links against your project to inspect registered types via inventory.

### Tooling Internals

- **`es-fluent-cli-helpers`**: The library code that runs _inside_ the temporary runner crate. It collects the inventory from the user's code and calls the generator.
- **`es-fluent-generate`**: A specialized FTL writer. It intelligently merges new keys into existing files without destroying manual comments or custom formatting.

### Scripts

- **`cldr-es-fluent-lang`**: A Python script that downloads Unicode CLDR data and generates `es-fluent-lang.ftl` (language autonyms like "English", "Français", "日本語") and `supported_locales.rs` (the list of valid language keys for compile-time validation in `es-fluent-lang-macro`).

### Web

- **`web`**: An Astro-based static site for GitHub Pages. Hosts WASM-compiled examples (e.g., the Bevy example) with live interactive demos. The site is built via the `gh-pages.yml` workflow which compiles Rust examples to WASM and deploys them.

## Development

- **Rust**: Use `cargo` for building, testing, and running Rust code.
- **JavaScript**: Use [bun](https://bun.com/) for dependency management.
- **Python**: Use [uv](https://docs.astral.sh/uv/) for dependency management and running Python scripts.
- **Testing**: Use [insta](https://insta.rs/) for snapshot tests where appropriate, rather than complex assertion-based unit tests.
