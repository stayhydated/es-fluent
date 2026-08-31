use super::localizer::LoadedDioxusI18nAssetModule;

use dioxus::prelude::Asset;

use es_fluent_manager_core::{
    I18nModuleDescriptor, I18nModuleRegistration, LocaleRelativeFtlPath, ModuleData,
    ModuleRegistrationKind, ModuleResourceSpec, ResourceKey, ResourceLoadError,
    parse_fluent_resource_bytes,
};

use std::{
    collections::HashMap,
    sync::{Arc, OnceLock},
};
use unic_langid::LanguageIdentifier;

pub struct DioxusI18nAssetResource {
    language: LanguageIdentifier,
    key: &'static str,
    locale_relative_path: &'static str,
    required: bool,
    pub(super) asset: Asset,
}

impl DioxusI18nAssetResource {
    pub const fn new(
        language: LanguageIdentifier,
        key: &'static str,
        locale_relative_path: &'static str,
        required: bool,
        asset: Asset,
    ) -> Self {
        Self {
            language,
            key,
            locale_relative_path,
            required,
            asset,
        }
    }

    pub(super) fn spec(&self) -> ModuleResourceSpec {
        ModuleResourceSpec::new(
            ResourceKey::from_static_path(self.key),
            LocaleRelativeFtlPath::from_static_path(self.locale_relative_path),
            self.required,
        )
    }
}

pub struct DioxusI18nAssetModule {
    data: &'static ModuleData,
    pub(super) resources: &'static [DioxusI18nAssetResource],
}

impl DioxusI18nAssetModule {
    pub const fn new(
        data: &'static ModuleData,
        resources: &'static [DioxusI18nAssetResource],
    ) -> Self {
        Self { data, resources }
    }

    pub(super) async fn load_with_cache_bust(
        &'static self,
        cache_bust: Option<u64>,
    ) -> LoadedDioxusI18nAssetModule {
        let mut loaded_resources = HashMap::new();
        let mut load_errors = HashMap::new();
        let mut resource_specs_by_language: HashMap<LanguageIdentifier, Vec<ModuleResourceSpec>> =
            HashMap::new();

        for resource in self.resources {
            let lang = resource.language.clone();
            let spec = resource.spec();
            resource_specs_by_language
                .entry(lang.clone())
                .or_default()
                .push(spec.clone());

            match read_dioxus_asset_bytes(&resource.asset, cache_bust).await {
                Ok(bytes) => match parse_fluent_resource_bytes(&spec, &bytes) {
                    Ok(parsed) => {
                        loaded_resources.insert((lang, spec.key.clone()), parsed);
                    },
                    Err(error) => {
                        load_errors.insert((lang, error.key().clone()), error);
                    },
                },
                Err(error) => {
                    let load_error = ResourceLoadError::load(&spec, error.to_string());
                    load_errors.insert((lang, spec.key.clone()), load_error);
                },
            }
        }

        for specs in resource_specs_by_language.values_mut() {
            specs.sort_by(|left, right| left.key.cmp(&right.key));
        }

        LoadedDioxusI18nAssetModule {
            data: self.data,
            loaded_resources: Arc::new(loaded_resources),
            load_errors: Arc::new(load_errors),
            resource_specs_by_language: Arc::new(resource_specs_by_language),
        }
    }
}

impl I18nModuleDescriptor for DioxusI18nAssetModule {
    fn data(&self) -> &'static ModuleData {
        self.data
    }
}

impl I18nModuleRegistration for DioxusI18nAssetModule {
    fn registration_kind(&self) -> ModuleRegistrationKind {
        ModuleRegistrationKind::MetadataOnly
    }

    fn resource_plan_for_language(
        &self,
        lang: &LanguageIdentifier,
    ) -> Option<Vec<ModuleResourceSpec>> {
        let mut resources = self
            .resources
            .iter()
            .filter(|resource| &resource.language == lang)
            .map(DioxusI18nAssetResource::spec)
            .collect::<Vec<_>>();

        resources.sort_by(|left, right| left.key.cmp(&right.key));
        (!resources.is_empty()).then_some(resources)
    }
}

inventory::collect!(&'static DioxusI18nAssetModule);

async fn read_dioxus_asset_bytes(
    asset: &Asset,
    cache_bust: Option<u64>,
) -> Result<Vec<u8>, String> {
    #[cfg(not(target_arch = "wasm32"))]
    {
        let _ = cache_bust;
        let path = asset.resolve();
        std::fs::read(&path)
            .map_err(|error| format!("failed to read Dioxus asset '{}': {error}", path.display()))
    }

    #[cfg(target_arch = "wasm32")]
    {
        let asset = asset.to_string();
        let asset = cache_bust
            .map(|revision| cache_busted_asset_path(&asset, revision))
            .unwrap_or(asset);

        dioxus::asset_resolver::read_asset_bytes(asset)
            .await
            .map_err(|error| error.to_string())
    }
}

#[cfg(any(target_arch = "wasm32", test))]
pub(super) fn cache_busted_asset_path(path: &str, revision: u64) -> String {
    let separator = if path.contains('?') { '&' } else { '?' };
    format!("{path}{separator}dx_i18n_reload={revision}")
}

#[derive(Clone, Copy)]
enum DioxusI18nAssetModuleSource {
    Static(&'static [&'static DioxusI18nAssetModule]),
    Discovered,
}

#[derive(Clone, Copy)]
pub struct DioxusI18nAssetModules {
    source: DioxusI18nAssetModuleSource,
}

impl DioxusI18nAssetModules {
    pub const fn new(modules: &'static [&'static DioxusI18nAssetModule]) -> Self {
        Self {
            source: DioxusI18nAssetModuleSource::Static(modules),
        }
    }

    pub const fn discovered() -> Self {
        Self {
            source: DioxusI18nAssetModuleSource::Discovered,
        }
    }

    pub fn as_slice(self) -> &'static [&'static DioxusI18nAssetModule] {
        match self.source {
            DioxusI18nAssetModuleSource::Static(modules) => modules,
            DioxusI18nAssetModuleSource::Discovered => discovered_dioxus_i18n_asset_modules(),
        }
    }
}

impl Default for DioxusI18nAssetModules {
    fn default() -> Self {
        Self::discovered()
    }
}

impl PartialEq for DioxusI18nAssetModules {
    fn eq(&self, other: &Self) -> bool {
        let left = self.as_slice();
        let right = other.as_slice();
        left.len() == right.len() && std::ptr::eq(left.as_ptr(), right.as_ptr())
    }
}

impl Eq for DioxusI18nAssetModules {}

impl std::fmt::Debug for DioxusI18nAssetModules {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DioxusI18nAssetModules")
            .field("len", &self.as_slice().len())
            .finish()
    }
}

fn discovered_dioxus_i18n_asset_modules() -> &'static [&'static DioxusI18nAssetModule] {
    static MODULES: OnceLock<Box<[&'static DioxusI18nAssetModule]>> = OnceLock::new();

    MODULES
        .get_or_init(|| {
            inventory::iter::<&'static DioxusI18nAssetModule>()
                .copied()
                .collect::<Vec<_>>()
                .into_boxed_slice()
        })
        .as_ref()
}
