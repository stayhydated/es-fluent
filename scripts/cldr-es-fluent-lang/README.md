# cldr-es-fluent-lang

A Python script that generates language name translations from [Unicode CLDR](https://cldr.unicode.org/) data for [es-fluent-lang](../../crates/es-fluent-lang/README.md) and [es-fluent-lang-macro](../../crates/es-fluent-lang-macro/README.md).

## Purpose

This script downloads and processes CLDR (Common Locale Data Repository) JSON data to generate:

1. **`es-fluent-lang.ftl`** - A Fluent translation file containing language names (autonyms) for all supported locales.
1. **`supported_locales.rs`** - A Rust source file with a constant array of all supported language keys for compile-time validation in `es-fluent-lang-macro`.
1. **Per-locale i18n files** - Localized language-name files for every CLDR locale in `crates/es-fluent-lang/i18n/<locale>/es-fluent-lang.ftl`.

By default, the script generates per-locale i18n files for every locale present in CLDR’s `cldr-localenames-full` dataset under `crates/es-fluent-lang/i18n`. You can disable that with `--no-all-locales`, or generate a single localized language-name file for a specific display locale using `--display-locale`.

## Requirements

- [uv](https://docs.astral.sh/uv/)

Install dependencies:

```sh
uv sync
```

## Usage

```sh
# Run with defaults (downloads CLDR, writes to crate locations)
uv run scripts/cldr-es-fluent-lang/main.py

# Use a pre-downloaded CLDR archive
uv run scripts/cldr-es-fluent-lang/main.py --cldr-zip /path/to/cldr-48.0.0-json-full.zip

# Generate localized language names (English) into i18n/en
uv run scripts/cldr-es-fluent-lang/main.py --display-locale en

# Skip per-locale i18n generation and only write the single output file
uv run scripts/cldr-es-fluent-lang/main.py --no-all-locales --display-locale en --output /path/to/es-fluent-lang.en.ftl

# Customize where per-locale i18n files are written
uv run scripts/cldr-es-fluent-lang/main.py --i18n-dir /path/to/i18n
```

The CLDR zip is cached under `.es-fluent/cldr/` in the repository root. The script re-downloads automatically when the CLDR version changes.

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

## Automated release updates

The repository includes `scripts/cldr-es-fluent-lang/update_cldr_release.py`, used by `.github/workflows/update-cldr-lang.yml`.

```sh
# Check current vs latest CLDR release
uv run scripts/cldr-es-fluent-lang/update_cldr_release.py

# Update CLDR_RELEASE in main.py and regenerate outputs when a new release exists
uv run scripts/cldr-es-fluent-lang/update_cldr_release.py --apply

# Also create a git commit for the update
uv run scripts/cldr-es-fluent-lang/update_cldr_release.py --apply --commit
```
