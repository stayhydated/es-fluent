use es_fluent_derive_core::{
    attribute::AttributeLocation, error::EsFluentCoreError, macro_input::ValidatedMacroInput,
    macro_support::ResolvedCratePath,
};

pub(crate) fn embedded_manager_path() -> ResolvedCratePath {
    ResolvedCratePath::resolve("es-fluent-manager-embedded", "es_fluent_manager_embedded")
}

pub(crate) fn bevy_manager_path() -> ResolvedCratePath {
    ResolvedCratePath::resolve("es-fluent-manager-bevy", "es_fluent_manager_bevy")
}

pub(crate) fn dioxus_manager_path() -> ResolvedCratePath {
    ResolvedCratePath::resolve("es-fluent-manager-dioxus", "es_fluent_manager_dioxus")
}

pub(crate) fn core_error_to_compile_error(error: EsFluentCoreError) -> proc_macro2::TokenStream {
    es_fluent_derive_core::macro_support::core_error_to_compile_error(error)
}

pub(crate) fn validate_locale_marker(
    attr: &syn::Attribute,
    location: AttributeLocation,
) -> Result<bool, EsFluentCoreError> {
    ValidatedMacroInput::locale_marker(attr, location)
}
