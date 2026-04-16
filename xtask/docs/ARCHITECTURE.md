# Architecture: xtask

## Purpose

`xtask` provides workspace maintenance tasks for this project.

## CLI commands

- `generate-lang-names`: Generates language-name resources and `supported_locales.rs` from ICU4X data
- `build-book`: builds mdBook documentation to `web/public/book`.
- `build-llms-txt`: concatenates mdBook sources into `web/public/llms.txt` for LLM consumption.
- `build-wasm-examples`: builds wasm examples declared in `web/wasm-examples.json`.
- `generate-wasm-examples-schema`: writes `web/wasm-examples.schema.json` from Rust manifest types.
- `verify-wasm-examples`: verifies that declared wasm example outputs still embed their required markers.

### generate-lang-names

#### Responsibilities

`generate-lang-names` performs three coordinated outputs:

1. Generates autonyms into `crates/es-fluent-lang/es-fluent-lang.ftl`.
1. Generates localized language-name files into
   `crates/es-fluent-lang/i18n/<locale>/es-fluent-lang.ftl`.
1. Generates compile-time locale keys into
   `crates/es-fluent-lang-macro/src/supported_locales.rs`.

#### Data Flow

```mermaid
flowchart TD
    ICU["ICU4X compiled data (display names)"]
    XT["xtask generate-lang-names"]
    AUT["es-fluent-lang.ftl (autonyms)"]
    I18N["i18n/<locale>/es-fluent-lang.ftl"]
    SUP["supported_locales.rs"]
    LANG["es-fluent-lang"]
    MACRO["es-fluent-lang-macro"]

    ICU --> XT
    XT --> AUT
    XT --> I18N
    XT --> SUP
    AUT --> LANG
    I18N --> LANG
    SUP --> MACRO
```

#### Notes

- Locale discovery is based on ICU4X markers shared across language/locale/region/script/variant display-name datasets.
- Output locales are filtered to locales with usable formatter data.
- Locale-name fallback favors exact match, then parent locale, then English, then first available locale.

### build-book

- `xtask/src/commands/build_book.rs`: builds mdBook via the `mdbook` crate API with output to `web/public/book`, adds `.gitignore` to exclude built files from version control.

### build-llms-txt

- `xtask/src/commands/build_llms_txt.rs`: loads the mdBook, skips draft chapters, writes a linked chapter index to `llms.txt`, and writes the expanded chapter content to `llms-full.txt`.

### build-wasm-examples

- `xtask/src/commands/build_wasm_examples.rs`: loads `web/wasm-examples.json`, runs `wasm-pack` for each declared example, and copies any declared asset directories into the declared output paths.

### generate-wasm-examples-schema

- `xtask/src/commands/generate_wasm_examples_schema.rs`: generates `web/wasm-examples.schema.json` from the Rust manifest types in `xtask/src/wasm_examples.rs`.

### verify-wasm-examples

- `xtask/src/commands/verify_wasm_examples.rs`: loads `web/wasm-examples.json`, asserts each declared module and wasm output exists, and checks the declared marker strings inside the wasm bytes.
