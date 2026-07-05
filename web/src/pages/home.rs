use crate::components::{FooterPanel, PageHeader};
use crate::site::routing::PageKind;
use dioxus::prelude::*;
use stayhydated_dioxus::{
    FeatureCard, HeroListPanel, HeroPanelItem, LinkTarget, ProjectHero, ProjectHeroActions,
    ProjectHomeShell, ProjectSurfaceSection, feature_card_reveal_style, hero_reveal_style,
};

#[component]
pub(crate) fn HomePage() -> Element {
    let hero_style = hero_reveal_style();

    rsx! {
        ProjectHomeShell {
            header: rsx!(PageHeader { current_page: PageKind::Home }),
            footer: rsx!(FooterPanel {}),
            ProjectHero {
                eyebrow: "Type-safe Project Fluent for Rust",
                title: "Localize Rust apps with typed messages",
                body: "Define messages in Rust. Generate Fluent files. Use them in embedded, Bevy, and Dioxus apps.",
                style: hero_style,
                side: Some(rsx! {
                    HeroListPanel {
                        label: "Runtime managers",
                        class: "hero-panel",
                        list_class: "hero-list",
                        body_class: "feature-copy",
                        label_heading: true,
                        items: vec![
                            HeroPanelItem::new("Embedded manager", "Ship translations in your binary."),
                            HeroPanelItem::new("Bevy manager", "Localize ECS systems and assets."),
                            HeroPanelItem::new("Dioxus manager", "Localize client and SSR views."),
                        ],
                    }
                }),
                actions: Some(rsx! {
                    ProjectHeroActions::<crate::site::routing::AppRoute> {
                        book: crate::site::routing::book_href().as_str(),
                        demos: LinkTarget::route(crate::site::routing::app_route(PageKind::Demos)),
                        primary_label: "Read the book",
                        secondary_label: "View demos",
                    }
                }),
            }

            ProjectSurfaceSection {
                title: "es-fluent workflow",
                lead: "For more detail, read the book.",
                FeatureCard {
                    label: "Rust derives",
                    title: "Define messages",
                    body: "Derive message keys and arguments from Rust types.",
                    style: feature_card_reveal_style(0),
                }
                FeatureCard {
                    label: "CLI",
                    title: "Check FTL files",
                    body: "Run `cargo es-fluent` to generate, check, and sync FTL.",
                    style: feature_card_reveal_style(1),
                }
                FeatureCard {
                    label: "Runtime managers",
                    title: "Use one manager",
                    body: "Reuse one message model across supported runtimes.",
                    style: feature_card_reveal_style(2),
                }
            }
        }
    }
}
