# Workspace Crates

Start with the high-level crates. A typical application needs the facade crate,
one runtime manager, and optionally the language-enum helper.

```toml
[dependencies]
es-fluent = { version = "*", features = ["derive"] }
es-fluent-manager-embedded = "*"
es-fluent-lang = "*"
```

Install the CLI separately:

```sh
cargo install es-fluent-cli --locked
```

Swap `es-fluent-manager-embedded` for `es-fluent-manager-dioxus` in Dioxus
apps, or for `es-fluent-manager-bevy` in Bevy apps.

## Crates You Usually Use

| Crate                        | Use it for                                                                    | Covered in this book                                                                                                           |
| ---------------------------- | ----------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------ |
| `es-fluent`                  | Derives, traits, and the public localization facade                           | [Getting Started](getting_started.md), [Deriving Messages](deriving_messages.md), [Namespaces & File Splitting](namespaces.md) |
| `es-fluent-manager-embedded` | Embedded-runtime apps, CLIs, TUIs, desktop apps                               | [Runtime Managers](managers.md)                                                                                                |
| `es-fluent-manager-dioxus`   | Dioxus apps using hook-based locale state on web/desktop/mobile plus SSR      | [Runtime Managers](managers.md)                                                                                                |
| `es-fluent-manager-bevy`     | Bevy integration, reactive localized UI, asset loading                        | [Runtime Managers](managers.md)                                                                                                |
| `es-fluent-lang`             | Type-safe locale enum generation and localized language names                 | [Language Enum](language_enum.md)                                                                                              |
| `es-fluent-cli`              | Generating, checking, cleaning, syncing, formatting, and inspecting FTL files | [CLI Tooling](cli.md)                                                                                                          |

## Public Support Crates

| Crate                      | Role                                                         |
| -------------------------- | ------------------------------------------------------------ |
| `es-fluent-derive`         | Proc-macro implementation re-exported by `es-fluent`         |
| `es-fluent-lang-macro`     | Implementation crate behind `#[es_fluent_language]`          |
| `es-fluent-manager-core`   | Shared runtime traits, module registration, fallback logic   |
| `es-fluent-manager-macros` | Compile-time module registration and `BevyFluentText` derive |

## Internal Workspace Crates

| Crate                   | Responsibility                                                                  |
| ----------------------- | ------------------------------------------------------------------------------- |
| `es-fluent-shared`      | Runtime-safe metadata, naming, namespace, and path helpers                      |
| `es-fluent-derive-core` | Build-time option parsing and validation for derives                            |
| `es-fluent-toml`        | `i18n.toml` parsing, path resolution, and build helpers                         |
| `es-fluent-generate`    | FTL AST generation, merging, cleaning, and formatting                           |
| `es-fluent-cli-helpers` | Runtime logic executed inside the generated runner binary                       |
| `es-fluent-runner`      | Shared runner protocol types and `.es-fluent/metadata` path helpers             |
| `xtask`                 | Repository maintenance tasks such as rebuilding the book and language-name data |
