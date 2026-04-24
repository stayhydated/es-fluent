use dioxus_core::Element;
use dioxus_core_macro::{Props, component, rsx};
#[allow(unused_imports)]
use dioxus_html as dioxus_elements;

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
