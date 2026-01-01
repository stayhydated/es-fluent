# es-fluent-cli

A command-line tool for managing Fluent translations in `es-fluent` projects.

## Usage

Run the tool from your project root or a sub-crate.

### Commands

#### `generate`

Scans your Rust source code for `#[derive(EsFluent)]` and related macros, and updates or creates your FTL files.

```sh
es-fluent generate
```

- **Default Behavior**: Adds new keys and updates existing ones. Preserves existing translations.
- **Options**:
  - `--mode aggressive`: Completely overwrites the file (WARNING: this deletes custom translations).

#### `watch`

Watches your `src/` directory for changes and automatically runs the generator. This is ideal for development.

```sh
es-fluent watch
```

- **Default Behavior**: Adds new keys and updates existing ones. Preserves existing translations.
- **Options**:
  - `--mode aggressive`: Completely overwrites the file (WARNING: this deletes custom translations).

#### `clean`

Removes orphan keys and groups—translations that are defined in your FTL file but no longer found in your Rust code.

```sh
es-fluent clean
```

- **Options**:
  - `--all`: Clean all locales, not just the fallback language.

#### `format`

Formats FTL files by sorting message keys alphabetically (A-Z). Group comments are preserved and sorted with their associated messages.

```sh
es-fluent format
```

- **Options**:
  - `--all`: Format all locales, not just the fallback language.
  - `--dry-run`: Show what would be formatted without making changes.

#### `check`

Checks FTL files for missing keys and omitted variables. Reports errors for missing translation keys and warnings for translations that omit variables present in the fallback language.

```sh
es-fluent check
```

- **Options**:
  - `--all`: Check all locales against the fallback language.

#### `sync`

Synchronizes missing translations from the fallback language to other locales. This copies untranslated keys from your fallback (e.g., English) to target locales, preserving existing translations.

```sh
es-fluent sync --all
```

- **Options**:
  - `--all`: Sync to all locales (excluding the fallback language).
  - `-l, --locale <LOCALE>`: Specific locale(s) to sync to.
  - `--dry-run`: Show what would be synced without making changes.

## Configuration

The CLI relies on an `i18n.toml` file in your crate root:

```toml
assets_dir = "i18n"         # Directory for FTL files
fallback_language = "en"    # Default language
```

### Optional: Feature-Gated Translations

If your es-fluent derives are behind a feature flag:

```toml
assets_dir = "i18n"
fallback_language = "en"
fluent_feature = "i18n"     # Single feature
# Or multiple features:
# fluent_feature = ["i18n", "translations"]
```
