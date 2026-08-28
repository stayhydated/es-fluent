# es-fluent-manager-embedded

[![Docs](https://docs.rs/es-fluent-manager-embedded/badge.svg)](https://docs.rs/es-fluent-manager-embedded/)
[![Crates.io](https://img.shields.io/crates/v/es-fluent-manager-embedded.svg)](https://crates.io/crates/es-fluent-manager-embedded)

Embedded localization for general Rust applications, including CLIs, TUIs,
desktop apps, and services. Configured FTL resources are compiled into the
binary and resolved through an explicit, cloneable `EmbeddedI18n`
handle.

~~~toml
[dependencies]
es-fluent = "*"
es-fluent-manager-embedded = "*"
unic-langid = "0.9"
~~~

Register resources from a library-reachable module:

~~~rust,ignore
es_fluent_manager_embedded::define_i18n_module!();
~~~

Initialize a selected locale and localize typed messages:

~~~rust,ignore
use es_fluent_manager_embedded::EmbeddedI18n;
use unic_langid::langid;

let i18n = EmbeddedI18n::try_new_with_language(langid!("en"))?;
let text = i18n.localize_message(&message);
~~~

Use strict initialization or selection only when every linked application
module must support the requested locale. Clones share locale state; construct
a separate manager for independent state.

Configured packages call `es_fluent_build::track_i18n_assets()` from Cargo's
selected custom-build target. Derived fallback-locale messages are compile-time
checked by default. Set `missing_message_policy = "fallback-str"` in the owning
package's `i18n.toml` to return snake_case field, variant, or type names from
normal typed lookup after locale fallback is exhausted; fallible lookup still
returns `None`.

See the [embedded manager guide](https://stayhydated.github.io/es-fluent/book/manager_embedded.html)
for workspace composition, fallible lookup, locale switching, and debug asset
embedding.
