use es_fluent_manager_embedded as i18n_manager;
use owner_a_api::{OwnerAGreeting, SharedUiGreeting as OwnerAUiGreeting};
use renamed_owner::{OwnerBGreeting, SharedUiGreeting as OwnerBUiGreeting};
use unic_langid::langid;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let language = langid!("en");
    let i18n = i18n_manager::EmbeddedI18n::try_new_with_language(language.clone())?;

    let owner_a = without_isolates(i18n.localize_message(&OwnerAGreeting { name: "Ada" }));
    let owner_b =
        without_isolates(i18n.localize_message(&OwnerBGreeting::Greeting { name: "Grace" }));
    let owner_a_ui = without_isolates(i18n.localize_message(&OwnerAUiGreeting { name: "Lin" }));
    let owner_b_ui = without_isolates(i18n.localize_message(&OwnerBUiGreeting { name: "Mira" }));

    assert_eq!(owner_a, "Owner A greets Ada");
    assert_eq!(owner_b, "Owner B greets Grace");
    assert_eq!(owner_a_ui, "Owner A UI greets Lin");
    assert_eq!(owner_b_ui, "Owner B UI greets Mira");

    let mut module_plans = i18n_manager::__inventory::iter::<
        &'static dyn i18n_manager::__manager_core::I18nModuleRegistration,
    >
        .into_iter()
        .map(|registration| {
            let registration = *registration;
            let owner = registration.data().owner.as_str();
            let resources = registration
                .resource_plan_for_language(&language)
                .expect("generated registration should expose its manifest resource plan");
            let resource_domains = resources
                .iter()
                .map(|resource| resource.key.domain())
                .collect::<Vec<_>>();
            if owner == "owner-a" {
                assert_eq!(resource_domains, ["owner-a", "ui"]);
            } else {
                assert_eq!(resource_domains, ["owner-b", "ui"]);
            }
            (owner.to_string(), resources.len())
        })
        .collect::<Vec<_>>();
    module_plans.sort();

    assert_eq!(
        module_plans,
        vec![("owner-a".to_string(), 2), ("owner-b".to_string(), 2)]
    );
    println!("owner-a: {owner_a}");
    println!("owner-b: {owner_b}");
    println!("owner-a-ui: {owner_a_ui}");
    println!("owner-b-ui: {owner_b_ui}");
    println!("modules: owner-a,owner-b");

    Ok(())
}

fn without_isolates(value: String) -> String {
    value.replace(['\u{2068}', '\u{2069}'], "")
}
