use crate::{I18nBundle, I18nDomainBundles, I18nResource};
use bevy::ecs::system::SystemParam;
use bevy::prelude::*;
use es_fluent::{FluentLocalizer, FluentLocalizerExt, FluentMessage, FluentValue};
use std::collections::HashMap;
use unic_langid::LanguageIdentifier;

/// Bevy-native localization context for systems.
///
/// Use this `SystemParam` when a system needs to localize a typed
/// `FluentMessage` directly instead of attaching a [`crate::FluentText`]
/// component for automatic UI updates.
#[derive(SystemParam)]
pub struct BevyI18n<'w> {
    i18n_resource: Res<'w, I18nResource>,
    i18n_bundle: Res<'w, I18nBundle>,
    i18n_domain_bundles: Res<'w, I18nDomainBundles>,
}

impl<'w> BevyI18n<'w> {
    /// Returns the currently published active language.
    pub fn active_language(&self) -> &LanguageIdentifier {
        self.i18n_resource.active_language()
    }

    /// Returns the resolved fallback language used for loaded Bevy bundles.
    pub fn resolved_language(&self) -> &LanguageIdentifier {
        self.i18n_resource.resolved_language()
    }

    /// Returns whether the unscoped or domain bundle cache changed this tick.
    pub fn is_bundle_changed(&self) -> bool {
        self.i18n_bundle.is_changed() || self.i18n_domain_bundles.is_changed()
    }

    /// Renders a typed Fluent message through this Bevy context.
    pub fn localize_message<T>(&self, message: &T) -> String
    where
        T: FluentMessage + ?Sized,
    {
        FluentLocalizerExt::localize_message(self, message)
    }
}

impl<'w> FluentLocalizer for BevyI18n<'w> {
    fn localize<'a>(
        &self,
        id: &str,
        args: Option<&HashMap<&str, FluentValue<'a>>>,
    ) -> Option<String> {
        self.i18n_resource.localize(id, args, &self.i18n_bundle)
    }

    fn localize_in_domain<'a>(
        &self,
        domain: &str,
        id: &str,
        args: Option<&HashMap<&str, FluentValue<'a>>>,
    ) -> Option<String> {
        self.i18n_resource
            .localize_in_domain(&self.i18n_domain_bundles, domain, id, args)
    }
}
