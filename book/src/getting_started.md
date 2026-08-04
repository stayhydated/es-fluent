# Getting started

This tutorial creates a Rust binary that generates a fallback Fluent resource
and prints a typed localized message. Run the commands from a Cargo package
with both `src/lib.rs` and `src/main.rs`; the CLI discovers
localizable types through library targets.

## Install dependencies

Add the facade, embedded manager, and locale identifier crate:

~~~toml
[dependencies]
es-fluent = "0.18"
es-fluent-manager-embedded = "0.18"
unic-langid = "0.9"
~~~

Install the Cargo subcommand:

~~~sh
cargo install es-fluent-cli --locked
~~~

## Configure locale assets

Create `i18n.toml` next to `Cargo.toml`:

~~~toml
fallback_language = "en"
assets_dir = "assets/locales"
~~~

Create the fallback locale directory:

~~~sh
mkdir -p assets/locales/en
~~~

See [Configure a project](configuration.md) for feature-gated derives,
namespace allowlists, additional domains, and validation settings.

## Define the runtime module

Create a library-reachable manager module:

~~~rust
// src/i18n.rs
pub use es_fluent_manager_embedded::{
    EmbeddedI18n as I18n, EmbeddedInitError, LocalizationError,
};

es_fluent_manager_embedded::define_i18n_module!();
~~~

Define a typed message in the library target:

~~~rust
// src/lib.rs
pub mod i18n;

use es_fluent::EsFluent;

#[derive(EsFluent)]
pub struct Greeting<'a> {
    pub name: &'a str,
}
~~~

## Generate fallback FTL

Run:

~~~sh
cargo es-fluent generate
~~~

For a package named `my-crate`, generation creates
`assets/locales/en/my-crate.ftl` with an entry like:

~~~ftl
## Greeting

greeting = Greeting { $name }
~~~

Replace the generated value with fallback-language copy:

~~~ftl
## Greeting

greeting = Hello, { $name }!
~~~

Conservative generation preserves edited values on later runs.

## Localize the message

A package named `my-crate` is imported as `my_crate` from
its binary target:

~~~rust
// src/main.rs
use my_crate::{Greeting, i18n::I18n};
use unic_langid::langid;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let i18n = I18n::try_new_with_language(langid!("en"))?;
    println!("{}", i18n.localize_message(&Greeting { name: "Ada" }));
    Ok(())
}
~~~

Run the program:

~~~sh
cargo run
~~~

It prints:

~~~text
Hello, Ada!
~~~

## Continue the workflow

After adding or changing localizable types, use:

~~~sh
cargo es-fluent generate
cargo es-fluent status --all-locales
~~~

Add a translated locale with:

~~~sh
cargo es-fluent add-locale fr-FR
~~~

Then edit the seeded FTL and run
`cargo es-fluent check --all-locales`. See
[CLI reference](cli.md) for command behavior and
[Runtime managers](managers.md) for Dioxus and Bevy setup.
