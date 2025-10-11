# es-fluent-build

The `es-fluent-build` crate provides a build script integration for automatically generating `.ftl` translation files from your Rust source code.

It is designed to be used as a `build-dependency` in your `Cargo.toml` and invoked from a `build.rs` script. The crate parses your source files for types that derive `EsFluent` and generates corresponding message keys in a Fluent resource file.

## Usage

1.  Add `es-fluent-build` as a build dependency in your `Cargo.toml`:

    ```toml
    [build-dependencies]
    es-fluent-build = { version = "...", path = "../es-fluent-build" } # Adjust path as needed
    ```

2.  Create a `build.rs` file in your crate's root with the following content:

    ```rs,no_run
    fn main() {
        es_fluent_build::FluentBuilder::new().build().unwrap();
    }
    ```

3.  Create an `i18n.toml` configuration file in your crate's root to specify the output directory and fallback language:

    ```toml
    fallback_language = "en"
    assets_dir = "i18n"
    ```

Now, every time you build your crate, the script will automatically scan your `src` directory and generate a `{crate_name}.ftl` file inside `i18n/en/`. This file will contain all the message keys extracted from your `EsFluent`-derived types, ready for you to add translations.
