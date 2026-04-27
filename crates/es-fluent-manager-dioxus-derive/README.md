[![Docs](https://docs.rs/es-fluent-manager-dioxus-derive/badge.svg)](https://docs.rs/es-fluent-manager-dioxus-derive/)
[![Crates.io](https://img.shields.io/crates/v/es-fluent-manager-dioxus-derive.svg)](https://crates.io/crates/es-fluent-manager-dioxus-derive)

# es-fluent-manager-dioxus-derive

Attribute macros for [`es-fluent-manager-dioxus`](../es-fluent-manager-dioxus/README.md).

Most Dioxus apps should depend on `es-fluent-manager-dioxus` for the runtime. Add this crate when components should keep direct `message.to_fluent_string()` calls while still subscribing to Dioxus locale changes.

```toml
[dependencies]
es-fluent-manager-dioxus = { version = "0.7", features = ["client"] }
es-fluent-manager-dioxus-derive = "0.7"
```

## `#[i18n_subscription]`

Use this attribute before Dioxus' `#[component]` attribute:

```rs
use dioxus::prelude::*;
use es_fluent::ToFluentString as _;
use es_fluent_manager_dioxus_derive::i18n_subscription;

#[i18n_subscription]
#[component]
fn Header() -> Element {
    rsx! {
        h1 { "{SiteMessage::Title.to_fluent_string()}" }
    }
}
```

The macro inserts an optional subscription and logs failed subscription attempts:

```rs
if let Err(error) = ::es_fluent_manager_dioxus::try_use_i18n_subscription() {
    ::es_fluent_manager_dioxus::__log_i18n_subscription_error(&error);
}
```

Missing providers remain optional. Failed providers or failed context reads are logged by `es-fluent-manager-dioxus`. The macro does not add `#[component]`, require a provider, change return types, or render error UI.
