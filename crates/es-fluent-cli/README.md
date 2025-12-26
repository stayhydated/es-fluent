# es-fluent-cli

A command-line tool for managing Fluent translations in `es-fluent` projects.

## Installation

```sh
cargo install es-fluent-cli
```

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

#### `clean`

Removes orphan keys and groups—translations that are defined in your FTL file but no longer found in your Rust code.

```sh
es-fluent clean
```

> **Note**: This will remove comments associated with orphan keys, and even entire groups if they become empty.

## Configuration

The CLI relies on an `i18n.toml` file in your crate root:

```toml
assets_dir = "i18n"         # Directory for FTL files
fallback_language = "en"    # Default language
```
