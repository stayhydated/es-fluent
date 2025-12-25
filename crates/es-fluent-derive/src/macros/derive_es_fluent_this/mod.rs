use darling::FromDeriveInput as _;
use es_fluent_core::{namer, options::this::ThisOpts};
use quote::quote;
use syn::{DeriveInput, parse_macro_input};

pub fn from(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    let opts = match ThisOpts::from_derive_input(&input) {
        Ok(opts) => opts,
        Err(err) => return err.write_errors().into(),
    };

    let original_ident = opts.ident();
    let generics = opts.generics();
    let ftl_key = if opts.attr_args().is_origin() {
        let this_ident = quote::format_ident!("{}_this", original_ident);
        Some(namer::FluentKey::new(&this_ident, "").to_string())
    } else {
        None
    };

    let this_ftl_impl =
        crate::macros::utils::generate_this_ftl_impl(original_ident, generics, ftl_key.as_deref());

    let tokens = quote! {
        #this_ftl_impl
    };

    tokens.into()
}
