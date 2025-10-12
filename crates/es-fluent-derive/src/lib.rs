#![doc = include_str!("../README.md")]

use proc_macro_error2::proc_macro_error;

mod macros;

#[proc_macro_derive(EsFluent, attributes(fluent))]
#[proc_macro_error]
pub fn derive_es_fluent(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    macros::derive_es_fluent::from(input)
}

#[proc_macro_derive(EsFluentKv, attributes(fluent_kv))]
#[proc_macro_error]
pub fn derive_es_fluent_kv(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    macros::derive_es_fluent_kv::from(input)
}

#[proc_macro_derive(EsFluentChoice, attributes(fluent_choice))]
#[proc_macro_error]
pub fn derive_fluent_choice(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    macros::derive_fluent_choice::from(input)
}
