# Project Overview

`es-fluent` is a comprehensive localization (i18n) ecosystem for Rust, built on top of [Project Fluent](https://projectfluent.org/). It focuses on:

1. **Type Safety**: Ensuring at compile-time that your code and translation files are in sync.
1. **Ergonomics**: Providing simple macros (`#[derive(EsFluent)]`) to make struct/enum fields localizable with minimal boilerplate.
1. **Developer Experience**: A robust CLI (`es-fluent-cli`) that auto-generates FTL files, manages keys, and ensures consistency.

## Architecture Documentation Index

| Crate | Link to Architecture Doc | Purpose |
|-------|-------------------|---------|
| **Core** | | |
| `es-fluent` | [Architecture](crates/es-fluent/docs/ARCHITECTURE.md) | Ecosystem facade and entry point. |
| `es-fluent-core` | [Architecture](crates/es-fluent-core/docs/ARCHITECTURE.md) | Fundamental types, registry traits, and memory layout. |
| `es-fluent-derive` | [Architecture](crates/es-fluent-derive/docs/ARCHITECTURE.md) | Proc-macros for registration and trait implementation. |
| `es-fluent-toml` | [Architecture](crates/es-fluent-toml/docs/ARCHITECTURE.md) | Configuration (`i18n.toml`) parsing and path resolution. |
| **Language Support** | | |
| `es-fluent-lang` | [Architecture](crates/es-fluent-lang/docs/ARCHITECTURE.md) | Runtime language identification and embedded translations. |
| `es-fluent-lang-macro` | [Architecture](crates/es-fluent-lang-macro/docs/ARCHITECTURE.md) | Generates type-safe language selection enums from asset folders. |
| **Managers** | | |
| `es-fluent-manager-core` | [Architecture](crates/es-fluent-manager-core/docs/ARCHITECTURE.md) | Abstract traits for localization backends. |
| `es-fluent-manager-embedded`| [Architecture](crates/es-fluent-manager-embedded/docs/ARCHITECTURE.md)| Zero-setup backend for embedding FTL files in binary. |
| `es-fluent-manager-bevy` | [Architecture](crates/es-fluent-manager-bevy/docs/ARCHITECTURE.md) | Backend integration for Bevy engine ECS and assets. |
| `es-fluent-manager-macros` | [Architecture](crates/es-fluent-manager-macros/docs/ARCHITECTURE.md)| Macros for asset discovery and module registration. |
| **Tooling** | | |
| `es-fluent-cli` | [Architecture](crates/es-fluent-cli/docs/ARCHITECTURE.md) | The `cargo es-fluent` command-line tool. |
| `es-fluent-cli-helpers` | [Architecture](crates/es-fluent-cli-helpers/docs/ARCHITECTURE.md) | Runtime logic for checking/generating FTL files (runner crate). |
| `es-fluent-generate` | [Architecture](crates/es-fluent-generate/docs/ARCHITECTURE.md) | FTL AST manipulation, diffing, and formatting logic. |

## Crate Descriptions

### Core Layers

- **`es-fluent`**: The user-facing library. Re-exports everything needed for general usage. Connects the global `OnceLock` context to specific backend managers.
- **`es-fluent-core`**: The "glue" crate. Defines `FtlTypeInfo` (inventory payload) and registration mechanisms. It's a common dependency for almost everything.
- **`es-fluent-derive`**: Provides the `#[derive(EsFluent)]` macro. transforming Rust types into inventory registrations and `FluentDisplay` implementations.
- **`es-fluent-toml`**: Centralizes configuration logic. Ensures that the CLI and proc-macros agree on where assets are located and what languages are available.

### Language & Locale

- **`es-fluent-lang`**: Provides the `I18nModule` for language names themselves (e.g. `lang-en = English`). Useful for UI language pickers.
- **`es-fluent-lang-macro`**: Scans your `assets/` directory to find available languages (e.g., `en/`, `fr/`) and generates an enum (e.g. `enum Languages { En, Fr }`) so you never hardcode language strings.

### Runtime Managers

- **`es-fluent-manager-core`**: Defines the `I18nModule` and `Localizer` traits. It allows the system to be agnostic about *how* translations are loaded (disk vs embedded).
- **`es-fluent-manager-embedded`**: A "singleton" manager. Initializes a global manager with embedded assets. ideal for CLI tools or simple apps.
- **`es-fluent-manager-bevy`**: A "resource" manager. Hooks into Bevy's `AssetServer` for hot-reloading and ECS reactivity.

### Tooling Internals

- **`es-fluent-cli`**: The binary installed by users. It compiles a "runner crate" to inspect the user's project codebase.
- **`es-fluent-cli-helpers`**: The library code that runs *inside* that temporary runner crate. It collects the inventory from the user's code and calls the generator.
- **`es-fluent-generate`**: A specialized FTL writer. It intelligently merges new keys into existing files without destroying manual comments or custom formatting.
