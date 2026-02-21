use crate::assets::{I18nAssets, current_crate_name, module_data_static_tokens};
use heck::ToPascalCase as _;
use proc_macro::TokenStream;
use quote::quote;

struct ManagerPaths {
    manager_core_path: proc_macro2::TokenStream,
    langid_path: proc_macro2::TokenStream,
    inventory_path: proc_macro2::TokenStream,
    rust_embed_path: proc_macro2::TokenStream,
    rust_embed_attr_path: &'static str,
    module_data_suffix: &'static str,
}

impl ManagerPaths {
    fn embedded() -> Self {
        Self {
            manager_core_path: quote! { ::es_fluent_manager_embedded::__manager_core },
            langid_path: quote! { ::es_fluent_manager_embedded::__unic_langid },
            inventory_path: quote! { ::es_fluent_manager_embedded::__inventory },
            rust_embed_path: quote! { ::es_fluent_manager_embedded::__rust_embed },
            rust_embed_attr_path: "::es_fluent_manager_embedded::__rust_embed",
            module_data_suffix: "EMBEDDED_I18N_MODULE_DATA",
        }
    }

    fn bevy() -> Self {
        Self {
            manager_core_path: quote! { ::es_fluent_manager_bevy::__manager_core },
            langid_path: quote! { ::es_fluent_manager_bevy::__unic_langid },
            inventory_path: quote! { ::es_fluent_manager_bevy::__inventory },
            rust_embed_path: quote! { ::es_fluent_manager_bevy::__rust_embed },
            rust_embed_attr_path: "::es_fluent_manager_bevy::__rust_embed",
            module_data_suffix: "BEVY_I18N_MODULE_DATA",
        }
    }
}

type ModuleTokenGenerator =
    fn(String, I18nAssets, syn::Ident, proc_macro2::TokenStream, &ManagerPaths) -> TokenStream;

fn expand_define_i18n_module(
    manager_paths: ManagerPaths,
    generate_tokens: ModuleTokenGenerator,
) -> TokenStream {
    let crate_name = match current_crate_name() {
        Ok(name) => name,
        Err(err) => return TokenStream::from(err.to_compile_error()),
    };

    let assets = match I18nAssets::load(&crate_name) {
        Ok(assets) => assets,
        Err(err) => return TokenStream::from(err.to_compile_error()),
    };

    let language_identifiers = assets.language_identifier_tokens(&manager_paths.langid_path);
    let namespace_strings = assets.namespace_tokens();

    let module_data_name = syn::Ident::new(
        &format!(
            "{}_{}",
            &crate_name.to_uppercase().replace('-', "_"),
            manager_paths.module_data_suffix
        ),
        proc_macro2::Span::call_site(),
    );

    let module_data_static = module_data_static_tokens(
        &manager_paths.manager_core_path,
        &module_data_name,
        &crate_name,
        &language_identifiers,
        &namespace_strings,
    );

    generate_tokens(
        crate_name,
        assets,
        module_data_name,
        module_data_static,
        &manager_paths,
    )
}

pub(crate) fn define_embedded_i18n_module(_input: TokenStream) -> TokenStream {
    expand_define_i18n_module(ManagerPaths::embedded(), generate_embedded_tokens)
}

pub(crate) fn define_bevy_i18n_module(_input: TokenStream) -> TokenStream {
    expand_define_i18n_module(ManagerPaths::bevy(), generate_bevy_tokens)
}

fn generate_embedded_tokens(
    crate_name: String,
    assets: I18nAssets,
    module_data_name: syn::Ident,
    module_data_static: proc_macro2::TokenStream,
    manager_paths: &ManagerPaths,
) -> TokenStream {
    let assets_struct_name = syn::Ident::new(
        &format!(
            "{}I18nAssets",
            &crate_name.replace('-', "_").to_pascal_case()
        ),
        proc_macro2::Span::call_site(),
    );

    let module_instance_name = syn::Ident::new(
        &format!(
            "{}_I18N_MODULE",
            &crate_name.to_uppercase().replace('-', "_")
        ),
        proc_macro2::Span::call_site(),
    );

    let i18n_root_str = assets.root_path.to_string_lossy();
    let rust_embed_path = &manager_paths.rust_embed_path;
    let rust_embed_attr_path = syn::LitStr::new(
        manager_paths.rust_embed_attr_path,
        proc_macro2::Span::call_site(),
    );
    let manager_core_path = &manager_paths.manager_core_path;
    let inventory_path = &manager_paths.inventory_path;

    let expanded = quote! {
        #[derive(#rust_embed_path::RustEmbed)]
        #[crate_path = #rust_embed_attr_path]
        #[folder = #i18n_root_str]
        struct #assets_struct_name;

        impl #manager_core_path::EmbeddedAssets for #assets_struct_name {
            fn domain() -> &'static str {
                #crate_name
            }
        }

        #module_data_static

        static #module_instance_name:
            #manager_core_path::EmbeddedI18nModule<#assets_struct_name> =
            #manager_core_path::EmbeddedI18nModule::<#assets_struct_name>::new(&#module_data_name);

        #inventory_path::submit!(
            &#module_instance_name
            as &dyn #manager_core_path::I18nModuleRegistration
        );
    };

    TokenStream::from(expanded)
}

fn generate_bevy_tokens(
    crate_name: String,
    assets: I18nAssets,
    module_data_name: syn::Ident,
    module_data_static: proc_macro2::TokenStream,
    manager_paths: &ManagerPaths,
) -> TokenStream {
    let registration_struct_name = syn::Ident::new(
        &format!(
            "{}I18nRegistration",
            &crate_name.replace('-', "_").to_pascal_case()
        ),
        proc_macro2::Span::call_site(),
    );

    let registration_instance_name = syn::Ident::new(
        &format!(
            "{}_I18N_REGISTRATION_INSTANCE",
            &crate_name.to_uppercase().replace('-', "_")
        ),
        proc_macro2::Span::call_site(),
    );

    let manifest_match_arms = assets
        .resource_plan_match_arms(&manager_paths.manager_core_path, &manager_paths.langid_path);
    let manager_core_path = &manager_paths.manager_core_path;
    let langid_path = &manager_paths.langid_path;
    let inventory_path = &manager_paths.inventory_path;

    let expanded = quote! {
        #module_data_static

        struct #registration_struct_name;

        impl #manager_core_path::I18nModuleDescriptor for #registration_struct_name {
            fn data(&self) -> &'static #manager_core_path::ModuleData {
                &#module_data_name
            }
        }

        impl #manager_core_path::I18nModuleRegistration for #registration_struct_name {
            fn resource_plan_for_language(
                &self,
                lang: &#langid_path::LanguageIdentifier,
            ) -> Option<Vec<#manager_core_path::ModuleResourceSpec>> {
                match lang {
                    #(#manifest_match_arms,)*
                    _ => None,
                }
            }
        }

        static #registration_instance_name: #registration_struct_name = #registration_struct_name;

        #inventory_path::submit!(
            &#registration_instance_name as &dyn #manager_core_path::I18nModuleRegistration
        );
    };

    TokenStream::from(expanded)
}
