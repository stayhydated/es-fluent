use dioxus::prelude::*;
use stayhydated_dioxus::{
    NavigationTarget, ShaderBackground, StayhydatedProjectPortalShell, page_entry_reveal_style,
};

use crate::site::{
    constants::{PROJECT, VERSION},
    routing::{AppRoute, PageKind},
};

#[component]
fn DemoCardContents(title: &'static str, shader_id: &'static str, time_offset: f32) -> Element {
    rsx! {
        ShaderBackground {
            canvas_id: shader_id,
            extra_class: "demo-card-shader",
            time_offset,
        }
        span { class: "demo-card-tint", aria_hidden: "true" }
        h2 { class: "demo-card-title", "{title}" }
    }
}

#[component]
fn DemoCardLink(
    route: AppRoute,
    title: &'static str,
    shader_id: &'static str,
    time_offset: f32,
) -> Element {
    let aria_label = format!("Open {title} demo");

    if try_router().is_some() {
        rsx! {
            Link {
                class: "demo-card",
                to: route,
                aria_label,
                DemoCardContents { title, shader_id, time_offset }
            }
        }
    } else {
        rsx! {
            a {
                class: "demo-card",
                href: route.to_string(),
                aria_label,
                DemoCardContents { title, shader_id, time_offset }
            }
        }
    }
}

#[component]
pub(crate) fn DemosPage() -> Element {
    let demos_style = page_entry_reveal_style().into_string();

    rsx! {
        StayhydatedProjectPortalShell {
            project: PROJECT,
            version: VERSION,
            home: NavigationTarget::Internal(crate::site::routing::app_route(PageKind::Home)),
            div { class: "demo-page demo-gallery",
                section {
                    class: "grid columns-3 demo-example-cards motion-reveal",
                    style: demos_style,
                    DemoCardLink {
                        route: crate::site::routing::app_route(PageKind::Dioxus),
                        title: "Dioxus",
                        shader_id: "dioxus-demo-card-shader",
                        time_offset: 0.0,
                    }
                    DemoCardLink {
                        route: crate::site::routing::app_route(PageKind::Bevy),
                        title: "Bevy",
                        shader_id: "bevy-demo-card-shader",
                        time_offset: 13.0,
                    }
                    DemoCardLink {
                        route: crate::site::routing::app_route(PageKind::Gpui),
                        title: "GPUI",
                        shader_id: "gpui-demo-card-shader",
                        time_offset: 26.0,
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn demos_page_renders_shader_backed_cards() {
        let html = dioxus::ssr::render_element(rsx! { DemosPage {} });

        assert_eq!(html.matches("class=\"demo-card\"").count(), 3);
        assert_eq!(html.matches("class=\"demo-card-title\"").count(), 3);
        assert_eq!(html.matches("class=\"demo-card-tint\"").count(), 3);
        assert_eq!(
            html.matches("data-shader-background=\"loading\"").count(),
            3
        );
        assert!(html.contains("id=\"dioxus-demo-card-shader\""));
        assert!(html.contains("id=\"bevy-demo-card-shader\""));
        assert!(html.contains("id=\"gpui-demo-card-shader\""));
    }
}
