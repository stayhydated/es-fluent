# cldr-es-fluent-lang

A Python script that generates language name translations from [Unicode CLDR](https://cldr.unicode.org/) data for [es-fluent-lang](../../crates/es-fluent-lang/README.md) and [es-fluent-lang-macro](../../crates/es-fluent-lang-macro/README.md).

## Purpose

This script downloads and processes CLDR (Common Locale Data Repository) JSON data to generate:

1. **`es-fluent-lang.ftl`** - A Fluent translation file containing language names (autonyms) for all supported locales.
1. **`supported_locales.rs`** - A Rust source file with a constant array of all supported language keys for compile-time validation in `es-fluent-lang-macro`.

## Requirements

- [uv](https://docs.astral.sh/uv/)

Install dependencies:

```sh
uv sync
```

## Usage

```sh
# Run with defaults (downloads CLDR, writes to crate locations)
uv run run.py

# Use a pre-downloaded CLDR archive
uv run run.py --cldr-zip /path/to/cldr-48.0.0-json-full.zip
```

## Output

The script generates entries like:

```ftl
es-fluent-lang-aa = Afar
es-fluent-lang-aa-DJ = Afar (Djibouti)
es-fluent-lang-aa-ER = Afar (Eritrea)
es-fluent-lang-aa-ET = Afar (Ethiopia)
es-fluent-lang-ab = Аԥсшәа
es-fluent-lang-ab-GE = Аԥсшәа (Georgia)
es-fluent-lang-ae = Avestan
es-fluent-lang-af = Afrikaans
es-fluent-lang-af-NA = Afrikaans (Namibia)
# ...
```

It also includes ISO 639-1 base language tags (two-letter codes like `en`, `fr`) so language-only folders are supported alongside region/script variants.

And a Rust file:

```rs
pub const SUPPORTED_LANGUAGE_KEYS: &[&str] = &[
    "aa",
    "aa-DJ",
    "aa-ER",
    "aa-ET",
    "ab",
    "ab-GE",
    "ae",
    "af",
    "af-NA",
    // ...
];
```
