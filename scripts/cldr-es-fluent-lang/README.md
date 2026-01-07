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
es-fluent-lang-ab-GE = Аԥсшәа
es-fluent-lang-af = Afrikaans
es-fluent-lang-agq-CM = Aghem
es-fluent-lang-ak-GH = Akan
es-fluent-lang-am-ET = አማርኛ
es-fluent-lang-an-ES = aragonés
es-fluent-lang-ann-NG = Obolo
# ...
```

And a Rust file:

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
