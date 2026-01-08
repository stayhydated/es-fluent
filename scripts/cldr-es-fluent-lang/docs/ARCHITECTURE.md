# cldr-es-fluent-lang Architecture

This document details the architecture of the `cldr-es-fluent-lang` script, which generates language name data for the es-fluent ecosystem.

## Overview

The script processes Unicode CLDR (Common Locale Data Repository) JSON data to extract language autonyms (self-names) and generates both Fluent translation files and Rust source files for compile-time validation.

## Package Structure

```
scripts/cldr-es-fluent-lang/
├── run.py              # CLI entrypoint
├── src/
│   ├── __init__.py     # Package exports
│   ├── models.py       # Pydantic models for CLDR JSON structures
│   ├── io.py           # File I/O helpers (download, extract, load)
│   ├── loaders.py      # CLDR data loaders with Pydantic validation
│   ├── processing.py   # Locale processing logic
│   └── writers.py      # Output file writers
└── docs/
    ├── ARCHITECTURE.md # This file
    └── README.md       # Usage guide
```

## Architecture

```mermaid
flowchart TD
    subgraph INPUT["Input"]
        CLDR["CLDR JSON Archive<br/>(cldr-48.0.0-json-full.zip)"]
        URL["GitHub Release URL"]
    end

    subgraph IO["src/io.py"]
        DL["download_file()"]
        EXT["extract_archive()"]
        LOAD["load_json()"]
    end

    subgraph LOADERS["src/loaders.py"]
        LIKELY["load_likely_subtags()"]
        AVAIL["load_available_locales()"]
        LOCALE["load_locale_entry()"]
        SCRIPT["load_script_names()"]
        TERR["load_territory_names()"]
    end

    subgraph PROCESSING["src/processing.py"]
        COLLECT["collect_entries()"]
        COLLAPSE["collapse_entries()"]
        FORMAT["format_locale()"]
        EXPAND["expand_locale()"]
        FALLBACK["fallback_chain()"]
    end

    subgraph WRITERS["src/writers.py"]
        WFTL["write_ftl()"]
        WRS["write_supported_locales()"]
    end

    subgraph OUTPUT["Output"]
        FTL["es-fluent-lang.ftl"]
        RS["supported_locales.rs"]
    end

    URL -->|download| DL
    CLDR -->|or use existing| EXT
    DL --> EXT
    EXT --> LOAD

    LOAD --> LIKELY & AVAIL & LOCALE & SCRIPT & TERR

    LIKELY --> EXPAND
    AVAIL --> COLLECT
    LOCALE & SCRIPT & TERR --> COLLECT
    EXPAND & FALLBACK --> COLLECT
    COLLECT --> COLLAPSE
    COLLAPSE --> FORMAT
    FORMAT --> WFTL & WRS
    WFTL --> FTL
    WRS --> RS
```

## Module Responsibilities

### run.py (CLI Entrypoint)

Thin CLI wrapper using Typer. Handles:

- Command-line argument parsing
- Orchestrating the pipeline (download -> extract -> process -> write)
- User-facing output and progress messages

### src/models.py (Pydantic Models)

Defines typed models for CLDR JSON structures:

| Model | Purpose |
|-------|---------|
| `Locale` | BCP-47 locale representation with parsing |
| `LikelySubtagsData` | Parses `likelySubtags.json` |
| `AvailableLocalesData` | Parses `availableLocales.json` |
| `LanguagesJsonMain` / `LocaleEntry` | Parses `languages.json` |
| `ScriptsJsonMain` | Parses `scripts.json` |
| `TerritoriesJsonMain` | Parses `territories.json` |

The `Locale` model parses BCP-47 language tags into components:

```python
class Locale(BaseModel, frozen=True):
    language: str          # e.g., "en", "zh"
    script: str | None     # e.g., "Hans", "Latn"
    region: str | None     # e.g., "US", "CN"
    variants: tuple[str, ...]  # e.g., ("valencia",)
```

### src/io.py (File I/O)

Low-level file operations:

| Function | Purpose |
|----------|---------|
| `download_file()` | Downloads CLDR archive with progress bar |
| `extract_archive()` | Extracts ZIP with progress bar |
| `load_json()` | Loads and parses JSON files |

### src/loaders.py (CLDR Data Loaders)

CLDR-specific data loading with Pydantic validation:

| Function | Purpose |
|----------|---------|
| `load_likely_subtags()` | Loads locale expansion mappings |
| `load_available_locales()` | Loads list of available locales |
| `load_locale_entry()` | Loads language names for a locale |
| `load_script_names()` | Loads script display names |
| `load_territory_names()` | Loads territory display names |

### src/processing.py (Locale Processing)

Core business logic:

| Function | Purpose |
|----------|---------|
| `expand_locale()` | Expands minimal tags using likelySubtags |
| `fallback_chain()` | Generates locale fallback sequence |
| `candidate_language_keys()` | Generates lookup keys for autonym search |
| `collapse_entries()` | Deduplicates entries with identical names |
| `format_locale()` | Normalizes output locale tags |
| `collect_entries()` | Main processing loop for all locales |

### src/writers.py (Output Writers)

File generation:

| Function | Purpose |
|----------|---------|
| `escape_fluent_value()` | Escapes curly braces for Fluent format |
| `write_ftl()` | Writes Fluent translation file |
| `write_supported_locales()` | Writes Rust constant array |

## Data Flow

### 1. CLDR Data Acquisition

The script uses CLDR release 48.0.0 by default. It can either:

- Download the archive from GitHub releases
- Use a pre-existing local archive (via `--cldr-zip`)

### 2. Entry Collection

For each available locale in CLDR:

1. **Expand locale** using `likelySubtags.json` (e.g., `zh` -> `zh-Hans-CN`)
1. **Generate candidate keys** for lookup (full tag, lang-script, lang-region, base language)
1. **Fallback chain** lookup for autonym:
   - Try the locale's own `languages.json`
   - Fall back through parent locales (e.g., `en-US` -> `en` -> `root`)
   - Fall back to English names
1. **Construct display name** from components if no autonym found

### 3. Entry Collapsing

The `collapse_entries()` function deduplicates entries where multiple region variants share the same name:

| Before | After |
|--------|-------|
| `en-US = English`, `en-GB = English`, `en-AU = English` | `en = English` |
| `zh-Hans-CN = 简体中文`, `zh-Hans-SG = 简体中文` | `zh-Hans = 简体中文` |

This reduces file size while preserving distinct names for locales that differ.

### 4. Locale Formatting

The `format_locale()` function normalizes output tags:

- Drops implicit scripts (e.g., `en-Latn` -> `en` since Latin is default for English)
- Drops `001` (World) region when implicit
- Preserves scripts when they differ from the likely default

## Output Files

### es-fluent-lang.ftl

Located at `crates/es-fluent-lang/es-fluent-lang.ftl`:

```ftl
es-fluent-lang-aa = Afar
es-fluent-lang-ab-GE = Аԥсшәа
es-fluent-lang-af = Afrikaans
es-fluent-lang-agq-CM = Aghem
es-fluent-lang-ak-GH = Akan
es-fluent-lang-am-ET = አማርኛ
es-fluent-lang-an-ES = aragonés
es-fluent-lang-ann-NG = Obolo
# ...
```

Keys are prefixed with `es-fluent-lang-` to namespace them within the Fluent ecosystem.

### supported_locales.rs

Located at `crates/es-fluent-lang-macro/src/supported_locales.rs`:

```rs
pub const SUPPORTED_LANGUAGE_KEYS: &[&str] = &[
    "aa",
    "ab-GE",
    "af",
    "agq-CM",
    "ak-GH",
    "am-ET",
    "an-ES",
    "ann-NG",
    // ...
];
```

This constant is used by `es-fluent-lang-macro` to validate language directories at compile time.
