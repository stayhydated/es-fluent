use dioxus::prelude::*;
use stayhydated_dioxus::{NavigationTarget, StayhydatedProjectPortalShell};

use crate::site::{
    constants::{PROJECT, VERSION},
    routing::PageKind,
};

#[derive(Clone, Copy)]
struct BrowserDemo {
    framework: &'static str,
    library: &'static str,
    directory: &'static str,
    title: &'static str,
}

const BROWSER_DEMOS: [BrowserDemo; 4] = [
    BrowserDemo {
        framework: "TypeScript",
        library: "@es-fluent/core",
        directory: "typescript-demo",
        title: "TypeScript localization demo",
    },
    BrowserDemo {
        framework: "Solid 2",
        library: "@es-fluent/solid",
        directory: "solid-demo",
        title: "Solid 2 localization demo",
    },
    BrowserDemo {
        framework: "React",
        library: "@es-fluent/react",
        directory: "react-demo",
        title: "React localization demo",
    },
    BrowserDemo {
        framework: "Expo",
        library: "@es-fluent/expo",
        directory: "expo-demo",
        title: "Expo localization demo",
    },
];

#[component]
pub(crate) fn TypeScriptDemosPage() -> Element {
    let page_output_dir = crate::site::routing::output_dir(PageKind::TypeScript);
    let site_root = crate::site::routing::site_root_prefix(&page_output_dir);

    rsx! {
        StayhydatedProjectPortalShell {
            project: PROJECT,
            version: VERSION,
            home: NavigationTarget::Internal(crate::site::routing::app_route(PageKind::Home)),
            div { class: "demos-list-page",
                for demo in BROWSER_DEMOS {
                    section { class: "demo-list-item", key: "{demo.directory}",
                        header { class: "demo-list-heading",
                            h1 { "{demo.framework}" }
                            code { class: "demo-list-library", "{demo.library}" }
                        }
                        div { class: "demo-list-surface",
                            iframe {
                                class: "demo-list-frame",
                                src: format!("{site_root}{}/", demo.directory),
                                title: demo.title,
                            }
                        }
                    }
                }
            }
        }
    }
}
