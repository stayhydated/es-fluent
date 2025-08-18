# es-fluent-toml

A configuration parser for es-fluent internationalization projects. This crate provides standardized TOML configuration parsing to eliminate hardcoded paths and centralize i18n settings across build scripts, proc macros, CLI tools, and runtime managers.

## Overview

The `es-fluent-toml` crate reads and parses `i18n.toml` configuration files, providing a unified way to configure internationalization settings across the entire es-fluent ecosystem. This eliminates the need for:

- Hardcoded asset paths in proc macros
- Duplicate configuration across different tools
- Manual path management in build scripts
- Inconsistent configuration formats

## Installation

Add this to your `Cargo.toml`:

```toml
[dependencies]
es-fluent-toml = "0.1"
```

For build scripts:

```toml
[build-dependencies]
es-fluent-toml = "0.1"
```

## Configuration Format

Create an `i18n.toml` file in your project root:

```toml
# (Required) The language identifier used as fallback
# This should match your source code language
fallback_language = "en"

# Fluent localization system configuration
[fluent]
# (Required) Path to translation assets directory
# Expected structure: {assets_dir}/{language}/{domain}.ftl
assets_dir = "assets/i18n"
```

### Example Configurations

**Basic setup:**
```toml
fallback_language = "en"

[fluent]
assets_dir = "assets/i18n"
```

**Custom asset location:**
```toml
fallback_language = "en"

[fluent]
assets_dir = "../shared/locales"
```

**Multi-language project:**
```toml
fallback_language = "en-US"

[fluent]
assets_dir = "translations"
```

## Usage

### Basic Configuration Reading

```rust
use es_fluent_toml::I18nConfig;

// Read configuration with automatic fallback to defaults
let config = I18nConfig::read_or_default();
println!("Fallback language: {}", config.fallback_language);
println!("Assets directory: {}", config.fluent.assets_dir);
```

### Error Handling

```rust
use es_fluent_toml::{I18nConfig, ConfigError};

match I18nConfig::read() {
    Ok(config) => {
        println!("Config loaded: {:?}", config);
    },
    Err(ConfigError::NotFound) => {
        println!("No i18n.toml found, using defaults");
        let config = I18nConfig::default();
    },
    Err(ConfigError::ParseError(e)) => {
        eprintln!("Invalid TOML format: {}", e);
    },
    Err(ConfigError::ReadError(e)) => {
        eprintln!("Failed to read file: {}", e);
    },
}
```

### Build Scripts and Proc Macros

```rust
// In build.rs or proc macro
use es_fluent_toml::I18nConfig;

let config = I18nConfig::read_from_manifest_dir()?;
let assets_path = config.assets_dir_from_manifest()?;

// Use assets_path for file operations
println!("cargo:rerun-if-changed={}", assets_path.display());
```

### Path Operations

```rust
use es_fluent_toml::I18nConfig;

let config = I18nConfig::read_or_default();

// Get assets directory as PathBuf
let assets_path = config.assets_dir_path();

// Get absolute path from manifest directory
let absolute_path = config.assets_dir_from_manifest()?;

// Validate directory exists
config.validate_assets_dir()?;
```

### Search and Discovery

```rust
use es_fluent_toml::I18nConfig;

// Search up directory tree for i18n.toml
let config = I18nConfig::find_and_read()?;

// Search from specific directory
let config = I18nConfig::find_and_read_from_dir("/path/to/project")?;
```

## API Reference

### `I18nConfig`

The main configuration structure containing all i18n settings.

#### Fields

- `fallback_language: String` - The primary language identifier
- `fluent: FluentConfig` - Fluent-specific configuration

#### Methods

- `read() -> Result<Self, ConfigError>` - Read from `./i18n.toml`
- `read_from_path<P: AsRef<Path>>(path: P) -> Result<Self, ConfigError>` - Read from specific path
- `read_from_manifest_dir() -> Result<Self, ConfigError>` - Read from `$CARGO_MANIFEST_DIR/i18n.toml`
- `find_and_read() -> Result<Self, ConfigError>` - Search up directory tree
- `read_or_default() -> Self` - Read with fallback to defaults (never fails)
- `assets_dir_path() -> PathBuf` - Get assets directory as path
- `assets_dir_from_manifest() -> Result<PathBuf, ConfigError>` - Get absolute assets path
- `validate_assets_dir() -> Result<(), ConfigError>` - Validate directory exists
- `fallback_language_id() -> &str` - Get fallback language identifier

### `FluentConfig`

Configuration specific to the Fluent localization system.

#### Fields

- `assets_dir: String` - Path to translation assets directory

### `ConfigError`

Error types for configuration operations.

#### Variants

- `NotFound` - Configuration file not found
- `ReadError(std::io::Error)` - File system read error
- `ParseError(toml::de::Error)` - TOML parsing error

## Integration Examples

### With Proc Macros

```rust
// In your proc macro
use es_fluent_toml::I18nConfig;

#[proc_macro]
pub fn define_i18n_module(_input: TokenStream) -> TokenStream {
    let config = I18nConfig::read_from_manifest_dir()
        .unwrap_or_default();
    
    let assets_dir = &config.fluent.assets_dir;
    let fallback_lang = &config.fallback_language;
    
    // Use config values instead of hardcoded paths
    let expanded = quote! {
        // Generate code using config...
    };
    
    TokenStream::from(expanded)
}
```

### With Build Scripts

```rust
// In build.rs
use es_fluent_toml::I18nConfig;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = I18nConfig::read_from_manifest_dir()?;
    
    // Watch for changes in assets directory
    let assets_path = config.assets_dir_from_manifest()?;
    println!("cargo:rerun-if-changed={}", assets_path.display());
    
    // Validate configuration
    config.validate_assets_dir()?;
    
    Ok(())
}
```

### With CLI Tools

```rust
// In CLI application
use es_fluent_toml::I18nConfig;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = I18nConfig::find_and_read()
        .unwrap_or_else(|_| {
            eprintln!("Warning: No i18n.toml found, using defaults");
            I18nConfig::default()
        });
    
    println!("Using assets directory: {}", config.fluent.assets_dir);
    
    // Perform CLI operations...
    Ok(())
}
```

## Directory Structure

The configuration expects translation files to be organized as:

```
{assets_dir}/
├── en/
│   ├── domain1.ftl
│   └── domain2.ftl
├── fr/
│   ├── domain1.ftl
│   └── domain2.ftl
└── es/
    ├── domain1.ftl
    └── domain2.ftl
```

Where:
- `{assets_dir}` is the path specified in `i18n.toml`
- `en`, `fr`, `es` are language directories
- `domain1.ftl`, `domain2.ftl` are Fluent translation files

## Default Values

When no configuration is found or values are missing:

- `fallback_language`: `"en"`
- `assets_dir`: `"assets/i18n"`

## Error Handling Best Practices

1. **Use `read_or_default()`** for most applications to avoid crashes
2. **Handle `ConfigError::NotFound`** gracefully with fallback behavior
3. **Validate assets directory** before performing file operations
4. **Provide helpful error messages** when configuration is invalid

## Migration from Hardcoded Paths

If you're migrating from hardcoded paths in macros:

1. Create an `i18n.toml` configuration file
2. Replace hardcoded paths with `I18nConfig::read_from_manifest_dir()`
3. Update macro calls to remove path parameters
4. Test with different asset directory configurations

## Examples

Run the included example:

```bash
cargo run --example config_example
```

This demonstrates various configuration reading scenarios and error handling.

## License

Licensed under either of Apache License, Version 2.0 or MIT license at your option.