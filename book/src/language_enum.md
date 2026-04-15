# Language Enum

Most apps need to know which languages are available — for a settings screen, a language picker, or to select the right locale at startup. The `#[es_fluent_language]` macro generates a strongly-typed enum from the folders in your `assets_dir`, so you never hardcode locale strings.

## Setup

Add the `es-fluent-lang` crate:

```toml
[dependencies]
es-fluent-lang = "*"
```

## Usage

Define an empty enum and annotate it with `#[es_fluent_language]`:

```rust
use es_fluent_lang::es_fluent_language;
use es_fluent::EsFluent;
use strum::EnumIter;

#[es_fluent_language]
#[derive(Debug, Clone, Copy, PartialEq, EsFluent, EnumIter)]
pub enum Languages {}
```

If your `assets_dir` contains `en`, `fr`, and `de` folders, the macro expands this into:

```rust
pub enum Languages {
    De,
    En,
    Fr,
}
```

The macro also generates these trait implementations:

| Trait                          | Description                                                           |
| ------------------------------ | --------------------------------------------------------------------- |
| `Default`                      | Returns the variant matching `fallback_language` from `i18n.toml`     |
| `FromStr`                      | Parses `"en-US"`, `"fr"`, or `"de-DE-1901"` into the matching variant |
| `TryFrom<&LanguageIdentifier>` | Converts from a `unic-langid` identifier                              |
| `Into<LanguageIdentifier>`     | Converts back to a `unic-langid` identifier                           |

## Using with Managers

The `Languages` enum plugs directly into manager initialization:

```rust
use es_fluent_manager_embedded as manager;

manager::init_with_language(Languages::En);
```

Since it implements `Into<LanguageIdentifier>`, you can pass variants anywhere a `LanguageIdentifier` is expected.

## Language Name Labels

By deriving `EsFluent` alongside `#[es_fluent_language]`, you get `to_fluent_string()` on each variant. The crate ships with a bundled `es-fluent-lang.ftl` file containing autonyms (language names in their own script: "English", "Français", "Deutsch"), so a language picker works out of the box:

```rust
use es_fluent::ToFluentString;

// Prints the language name in its native script
println!("{}", Languages::Fr.to_fluent_string()); // → "Français"
```

## Custom Mode

By default, the macro links to the bundled autonym translations and skips inventory registration. If you want to provide your own translations for language names (e.g., localized names like "French" in English or "Francés" in Spanish), use **custom mode**:

```rust
#[es_fluent_language(custom)]
#[derive(Debug, Clone, Copy, EsFluent, EnumIter)]
pub enum Languages {}
```

In custom mode:

- The enum **is** registered with inventory, so `cargo es-fluent generate` will create keys for it in your FTL files.
- You provide your own translations instead of using the bundled autonyms.
