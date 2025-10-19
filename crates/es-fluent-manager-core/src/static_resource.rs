use fluent_bundle::FluentResource;
use std::sync::Arc;
use unic_langid::LanguageIdentifier;

/// A static Fluent resource that can be injected directly into localization bundles.
pub trait StaticI18nResource: Send + Sync {
    /// Domain under which the resource should be registered.
    fn domain(&self) -> &'static str;

    /// Returns the resource content.
    fn resource(&self) -> Arc<FluentResource>;

    /// Returns `true` when this resource should be included for the provided language.
    fn matches_language(&self, _lang: &LanguageIdentifier) -> bool {
        true
    }
}

inventory::collect!(&'static dyn StaticI18nResource);
