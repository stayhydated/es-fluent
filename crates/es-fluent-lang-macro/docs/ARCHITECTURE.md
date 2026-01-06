# es-fluent-lang-macro Architecture

This document details the architecture of the `es-fluent-lang-macro` crate, which automates the generation of a type-safe language selection enum.

## Overview

The `#[es_fluent_language]` attribute macro generates a Rust enum representing the supported languages found in the project's configuration (`i18n.toml`) and file structure.

## Architecture

```mermaid
flowchart TD
    TOML[i18n.toml]
    ASSETS[assets_dir]
    DIRS["Language Folders<br/>(en, fr, etc.)"]

    subgraph INPUT["Input Code"]
        ENUM["#[es_fluent_language]<br/>enum Languages {}"]
    end

    subgraph MACRO["Proc Macro"]
        SCAN["Scan Configuration"]
        GEN["Generate Enum Variants"]
    end

    subgraph OUTPUT["Expanded Code"]
        EXP_ENUM["enum Languages {<br/>  En,<br/>  Fr,<br/>}"]
        IMPL["impl FromStr/Display"]
    end

    TOML -->|defines| ASSETS
    ASSETS -->|contains| DIRS
    DIRS -->|scanned by| SCAN
    ENUM --> GEN
    SCAN --> GEN
    GEN --> EXP_ENUM
    GEN --> IMPL
```

## Macro Process

The macro automates language discovery based on the project structure:

```
workspace_root/
├── i18n.toml          # Specifies: `assets_dir = "assets/locales"`
└── assets/
    └── locales/       # The configured assets directory
        ├── en/        # Found -> Enum Variant: `En`
        ├── fr/        # Found -> Enum Variant: `Fr`
        └── zh-CN/     # Found -> Enum Variant: `ZhCn`
```

The detailed steps are:

1.  **Read Configuration**: The macro reads `i18n.toml` from the workspace root (using `es-fluent-toml`) to determine the `assets_dir`.
2.  **Scan Assets**: It scans the configured assets directory for subdirectories.
3.  **Identify Languages**: Each subdirectory name is parsed as a BCP-47 language code (e.g., `en`, `zh-CN`).
4.  **Generate Enum**:
    *   The user provides an **empty enum**.
    *   The macro populates it with variants corresponding to the discovered language codes.
    *   It implements helper traits for converting between the enum and string representations.
    *   It implements `Default` based on the fallback language defined in `i18n.toml`.

## Modes

### Default Mode
By default, the macro links the generated enum to the embedded translations provided by the `es-fluent-lang` crate (e.g., providing "English", "Français" names automatically).

It generates:
```rust
#[es_fluent_language]
pub enum Languages {}
// Becomes ->
#[derive(EsFluent, ...)]
#[fluent(resource = "es-fluent-lang", skip_inventory)]
pub enum Languages {
    En,
    Fr,
    ZhCn, // PascalCase from language code
}
```
*   **Expansion**: The empty enum is populated with variants derived from the folder names (converted to PascalCase).
*   `resource = "es-fluent-lang"`: Links to the bundled FTL resource.
*   `skip_inventory`: Skips registration because these are generic language names, not user-app content.

### Custom Mode (`custom = true`)
If you want to provide your own translations for language names, use the `custom` argument:

```rust
#[es_fluent_language(custom)]
enum Languages {}
```

In this mode:
1.  **No Resource Link**: It does *not* add `resource = "es-fluent-lang"`.
2.  **Inventory Registration**: It does *not* skip inventory. The enum is registered like any other localized type.
3.  **Skeleton Generation**: The CLI will detect this registration and generate skeleton Fluent keys (e.g., `languages-en`, `languages-fr`) in your project's FTL files, which you can then translate manually.
