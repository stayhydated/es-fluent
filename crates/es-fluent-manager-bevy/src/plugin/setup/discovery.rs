use es_fluent_manager_core::{
    I18nModuleRegistration, ModuleDiscoveryError, ModuleRegistrationKind,
};
use std::collections::HashSet;
use unic_langid::LanguageIdentifier;

pub(in crate::plugin) struct ModuleDiscovery {
    pub(in crate::plugin) modules: Vec<&'static dyn I18nModuleRegistration>,
    pub(in crate::plugin) domains: HashSet<(&'static str, &'static str)>,
    pub(in crate::plugin) asset_languages: HashSet<LanguageIdentifier>,
    pub(in crate::plugin) all_languages: HashSet<LanguageIdentifier>,
}

pub(in crate::plugin) fn discover_modules() -> Result<ModuleDiscovery, Vec<ModuleDiscoveryError>> {
    let discovered = inventory::iter::<&'static dyn I18nModuleRegistration>()
        .copied()
        .collect::<Vec<_>>();
    let modules = es_fluent_manager_core::try_filter_module_registry(discovered)?;
    let mut domains = HashSet::new();
    let mut asset_languages = HashSet::new();
    let mut all_languages = HashSet::new();

    for module in &modules {
        let data = module.data();
        for domain in data.domains {
            domains.insert((data.owner.as_str(), domain.domain.as_str()));
        }
        for lang in data.supported_languages {
            all_languages.insert(lang.clone());
            if module.registration_kind() == ModuleRegistrationKind::MetadataOnly {
                asset_languages.insert(lang.clone());
            }
        }

        bevy::log::info!(
            "Discovered i18n module: {} with owner: {}, domains: {:?}",
            data.name,
            data.owner,
            data.domains,
        );
    }

    Ok(ModuleDiscovery {
        modules,
        domains,
        asset_languages,
        all_languages,
    })
}
