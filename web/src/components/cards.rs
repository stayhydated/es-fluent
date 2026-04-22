use dioxus_core::Element;
use dioxus_core_macro::{Props, component, rsx};
#[allow(unused_imports)]
use dioxus_html as dioxus_elements;

#[component]
pub(crate) fn FeatureCard(title: String, body: String) -> Element {
    rsx! {
        article { class: "demo-card",
            h2 { "{title}" }
            p { class: "card-copy", "{body}" }
        }
    }
}

#[component]
pub(crate) fn DemoCard(
    label: String,
    title: String,
    body: String,
    action: String,
    href: String,
    external: bool,
) -> Element {
    rsx! {
        a {
            class: "demo-card",
            href,
            target: external.then_some("_blank"),
            rel: external.then_some("noreferrer"),
            div { class: "card-label", "{label}" }
            h2 { "{title}" }
            p { class: "card-copy", "{body}" }
            span { class: "card-link", "{action}" }
        }
    }
}
