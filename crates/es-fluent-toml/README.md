# es-fluent-toml

The `es-fluent-toml` crate provides the data structures and parsing logic for the `i18n.toml` configuration file used by the `es-fluent` ecosystem.

## Configuration

The `i18n.toml` file defines the basic settings required for the build tools to locate and process your translation files.

-   `fallback_language`: The primary language for your application. The build tools will generate the initial `.ftl` file in this language's directory.
-   `assets_dir`: The path to the directory where your translation files are stored. The tools expect a structure of `{assets_dir}/{language_code}/...`.

### Example `i18n.toml`

```toml
fallback_language = "en"
assets_dir = "i18n"
```
