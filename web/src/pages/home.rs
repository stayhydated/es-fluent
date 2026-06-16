use crate::components::{FooterPanel, PageHeader};
use crate::site::i18n::{HomeHeroMessage, HomeWorkflowMessage, SiteLanguage};
use crate::site::routing::PageKind;
use dioxus::prelude::*;
use stayhydated_dioxus::{
    FeatureCardItem, HeroListPanel, HeroPanelItem, LinkTarget, Project, ProjectHero,
    ProjectHeroActions, ProjectHomeShell, SkillFeatureSection, hero_reveal_style,
};

#[component]
pub(crate) fn HomePage(locale: SiteLanguage) -> Element {
    let hero_style = hero_reveal_style();
    let i18n = match es_fluent_manager_dioxus::use_i18n() {
        Ok(i18n) => i18n,
        Err(error) => return rsx! { div { class: "page-shell", "failed: {error}" } },
    };
    let hero_eyebrow = i18n.localize_message(&HomeHeroMessage::Eyebrow);
    let hero_title = i18n.localize_message(&HomeHeroMessage::Title);
    let hero_body = i18n.localize_message(&HomeHeroMessage::Body);
    let hero_primary_action = i18n.localize_message(&HomeHeroMessage::PrimaryAction);
    let hero_secondary_action = i18n.localize_message(&HomeHeroMessage::SecondaryAction);
    let hero_panel_label = i18n.localize_message(&HomeHeroMessage::PanelLabel);
    let hero_panel_one_title = i18n.localize_message(&HomeHeroMessage::PanelOneTitle);
    let hero_panel_one_body = i18n.localize_message(&HomeHeroMessage::PanelOneBody);
    let hero_panel_two_title = i18n.localize_message(&HomeHeroMessage::PanelTwoTitle);
    let hero_panel_two_body = i18n.localize_message(&HomeHeroMessage::PanelTwoBody);
    let hero_panel_three_title = i18n.localize_message(&HomeHeroMessage::PanelThreeTitle);
    let hero_panel_three_body = i18n.localize_message(&HomeHeroMessage::PanelThreeBody);
    let workflow_title = i18n.localize_message(&HomeWorkflowMessage::Title);
    let workflow_lead = i18n.localize_message(&HomeWorkflowMessage::Lead);
    let workflow_one_label = i18n.localize_message(&HomeWorkflowMessage::OneLabel);
    let workflow_one_title = i18n.localize_message(&HomeWorkflowMessage::OneTitle);
    let workflow_one_body = i18n.localize_message(&HomeWorkflowMessage::OneBody);
    let workflow_two_label = i18n.localize_message(&HomeWorkflowMessage::TwoLabel);
    let workflow_two_title = i18n.localize_message(&HomeWorkflowMessage::TwoTitle);
    let workflow_two_body = i18n.localize_message(&HomeWorkflowMessage::TwoBody);
    let workflow_three_label = i18n.localize_message(&HomeWorkflowMessage::ThreeLabel);
    let workflow_three_title = i18n.localize_message(&HomeWorkflowMessage::ThreeTitle);
    let workflow_three_body = i18n.localize_message(&HomeWorkflowMessage::ThreeBody);

    rsx! {
        ProjectHomeShell {
            header: rsx!(PageHeader { locale, current_page: PageKind::Home }),
            footer: rsx!(FooterPanel {}),
            ProjectHero {
                eyebrow: hero_eyebrow,
                title: hero_title,
                body: hero_body,
                style: hero_style,
                side: Some(rsx! {
                    HeroListPanel {
                        label: hero_panel_label,
                        class: "hero-panel",
                        list_class: "hero-list",
                        body_class: "feature-copy",
                        label_heading: true,
                        items: vec![
                            HeroPanelItem::new(hero_panel_one_title, hero_panel_one_body),
                            HeroPanelItem::new(hero_panel_two_title, hero_panel_two_body),
                            HeroPanelItem::new(hero_panel_three_title, hero_panel_three_body),
                        ],
                    }
                }),
                actions: Some(rsx! {
                    ProjectHeroActions::<crate::site::routing::AppRoute> {
                        book: crate::site::routing::book_href().as_str(),
                        demos: LinkTarget::route(crate::site::routing::app_route(locale, PageKind::Demos)),
                        primary_label: hero_primary_action,
                        secondary_label: hero_secondary_action,
                    }
                }),
            }

            SkillFeatureSection {
                title: workflow_title,
                lead: workflow_lead,
                repo: Project::EsFluent,
                items: vec![
                    FeatureCardItem::new(workflow_one_label, workflow_one_title, workflow_one_body),
                    FeatureCardItem::new(workflow_two_label, workflow_two_title, workflow_two_body),
                    FeatureCardItem::new(
                        workflow_three_label,
                        workflow_three_title,
                        workflow_three_body,
                    ),
                ],
            }
        }
    }
}
