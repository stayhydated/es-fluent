use super::super::state::update_global_bundle;
use crate::{I18nAssets, I18nBundle, I18nResource};
use bevy::prelude::*;
use bevy::window::RequestRedraw;

#[doc(hidden)]
pub(crate) fn sync_global_state(
    i18n_bundle: Res<I18nBundle>,
    i18n_assets: Res<I18nAssets>,
    i18n_resource: Res<I18nResource>,
    mut redraw_events: MessageWriter<RequestRedraw>,
) {
    if i18n_bundle.is_changed() {
        update_global_bundle((*i18n_bundle).clone());

        if i18n_assets.is_language_loaded(i18n_resource.current_language()) {
            let lang = i18n_resource.current_language().clone();
            debug!("I18n bundle ready for current language: {}", lang);
            // Request a redraw so that UI updates even when using WinitSettings::desktop_app()
            redraw_events.write(RequestRedraw);
        }
    }
}
