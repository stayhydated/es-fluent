#![doc = include_str!("../README.md")]

use proc_macro::TokenStream;
use quote::quote;
use syn::{ItemFn, parse_macro_input};

/// Subscribes a Dioxus component to locale changes while preserving direct
/// `message.to_fluent_string()` call sites.
///
/// This macro injects:
///
/// ```ignore
/// if let Err(error) = ::es_fluent_manager_dioxus::try_use_i18n_subscription() {
///     ::es_fluent_manager_dioxus::__log_i18n_subscription_error(&error);
/// }
/// ```
///
/// Missing providers remain optional, but failed providers or failed context
/// reads are logged by the runtime crate.
///
/// Put it before Dioxus' `#[component]` attribute so this macro runs first and
/// returns the component attribute unchanged.
#[proc_macro_attribute]
pub fn i18n_subscription(attr: TokenStream, item: TokenStream) -> TokenStream {
    let attr = proc_macro2::TokenStream::from(attr);
    if !attr.is_empty() {
        return syn::Error::new_spanned(attr, "i18n_subscription does not accept arguments")
            .to_compile_error()
            .into();
    }

    let item = parse_macro_input!(item as ItemFn);
    let attrs = item.attrs;
    let vis = item.vis;
    let sig = item.sig;
    let block = item.block;

    quote! {
        #(#attrs)*
        #vis #sig {
            if let Err(error) = ::es_fluent_manager_dioxus::try_use_i18n_subscription() {
                ::es_fluent_manager_dioxus::__log_i18n_subscription_error(&error);
            }
            #block
        }
    }
    .into()
}
