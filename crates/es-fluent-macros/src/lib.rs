use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, LitStr};
use std::fs;

#[proc_macro]
pub fn define_i18n_module(input: TokenStream) -> TokenStream {
    let path = parse_macro_input!(input as LitStr);
    let crate_name = std::env::var("CARGO_PKG_NAME").expect("CARGO_PKG_NAME must be set");
    let static_data_name = syn::Ident::new(
        &format!(
            "{}_I18N_MODULE_DATA",
            &crate_name.to_uppercase().replace('-', "_")
        ),
        proc_macro2::Span::call_site(),
    );

    let i18n_root_path = std::path::Path::new(&std::env::var("CARGO_MANIFEST_DIR").unwrap()).join(path.value());

    let mut resources = Vec::new();
    let entries = fs::read_dir(&i18n_root_path)
        .unwrap_or_else(|e| panic!("Failed to read i18n directory at {:?}: {}", i18n_root_path, e));

    for entry in entries {
        let entry = entry.expect("Failed to read directory entry");
        let path = entry.path();
        if path.is_dir() {
            if let Some(lang_code) = path.file_name().and_then(|s| s.to_str()) {
                let ftl_file_name = format!("{}.ftl", crate_name);
                let ftl_path = path.join(ftl_file_name);

                if ftl_path.exists() {
                    let content = fs::read_to_string(&ftl_path)
                        .unwrap_or_else(|e| panic!("Failed to read FTL file at {:?}: {}", ftl_path, e));
                    resources.push((lang_code.to_string(), content));
                }
            }
        }
    }

    let resource_tuples = resources.iter().map(|(lang, content)| {
        quote! { (unic_langid::langid!(#lang), #content) }
    });

    let expanded = quote! {
        static #static_data_name: es_fluent::StaticModuleData = es_fluent::StaticModuleData {
            name: #crate_name,
            resources: &[
                #(#resource_tuples),*
            ],
        };

        inventory::submit!(
            &es_fluent::StaticI18nModule::new(&#static_data_name)
            as &dyn es_fluent::I18nModule
        );
    };

    TokenStream::from(expanded)
}
