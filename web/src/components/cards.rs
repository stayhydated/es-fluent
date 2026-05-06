use dioxus::prelude::*;

#[component]
pub(crate) fn FeatureCard(title: String, body: String, style: String) -> Element {
    rsx! {
        article {
            class: "demo-card motion-reveal",
            style,
            h2 { "{title}" }
            p { class: "card-copy", "{body}" }
        }
    }
}
