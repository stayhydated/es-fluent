# es-fluent-build

The `es-fluent-build` crate provides a build script integration for automatically generating `.ftl` translation files from your Rust source code.

It is designed to be used as a `build-dependency` in your `Cargo.toml` and invoked from a `build.rs` script. The crate parses your source files for types that derive `EsFluent` and generates corresponding message keys in a Fluent resource file.

## Usage

1. Add `es-fluent-build` as a build dependency in your `Cargo.toml`:

   ```toml
   [build-dependencies]
   es-fluent-build = { version = "*" }
   ```

1. Create a `build.rs` file in your crate's root with the following content:

   ```rs
   pub fn main() {
       if let Err(e) = es_fluent_build::FluentBuilder::new()
           .mode(es_fluent_build::FluentParseMode::Conservative)
           .build()
       {
           log::error!("Error building FTL files: {}", e);
       }
   }
   ```

1. Create an `i18n.toml` configuration file in your crate's root to specify the output directory and fallback language:

   ```toml
   fallback_language = "en"
   assets_dir = "i18n"
   ```

Now, every time you build your crate, the script will automatically scan your `src` directory and generate a `{crate_name}.ftl` file inside `i18n/en/`. This file will contain all the message keys extracted from your `EsFluent`-derived types, ready for you to add translations.

## Skipping FTL Generation

in envs like publishing, you can set the `ES_FLUENT_SKIP_BUILD` environment variable. This will prevent the build script from running and thus avoid generating the FTL files.

such as:

```sh
ES_FLUENT_SKIP_BUILD=true cargo publish --workspace --dry-run
```
