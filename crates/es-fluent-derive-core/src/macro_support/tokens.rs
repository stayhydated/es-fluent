use super::*;

pub fn static_domain_tokens(
    facade_path: &TokenStream,
    domain_override: Option<&FluentDomain>,
) -> TokenStream {
    match domain_override {
        Some(domain) => {
            let domain = domain.as_str();
            quote! { #facade_path::registry::__macro::static_domain(#domain) }
        },
        None => quote! {
            #facade_path::registry::StaticFluentDomain::from_package_name(env!("CARGO_PKG_NAME"))
        },
    }
}

pub fn static_entry_id_tokens(
    facade_path: &TokenStream,
    entry_id: &FluentMessageId,
) -> TokenStream {
    let entry_id = entry_id.as_str();
    quote! {
        #facade_path::registry::__macro::static_entry_id(#entry_id)
    }
}

pub fn static_message_key_tokens(
    facade_path: &TokenStream,
    domain_override: Option<&FluentDomain>,
    entry_id: &FluentMessageId,
    fallback: Option<&str>,
) -> TokenStream {
    let domain = static_domain_tokens(facade_path, domain_override);
    let entry_id = static_entry_id_tokens(facade_path, entry_id);
    match fallback {
        Some(fallback) => quote! {
            #facade_path::registry::__macro::static_message_key_with_fallback(
                env!("CARGO_PKG_NAME"),
                #domain,
                #entry_id,
                #fallback,
            )
        },
        None => quote! {
            #facade_path::registry::__macro::static_message_key(
                env!("CARGO_PKG_NAME"),
                #domain,
                #entry_id,
            )
        },
    }
}

pub fn static_argument_name_tokens(
    facade_path: &TokenStream,
    argument_name: &FluentArgumentName,
) -> TokenStream {
    let argument_name = argument_name.as_str();
    quote! {
        #facade_path::registry::__macro::static_argument_name(#argument_name)
    }
}

pub fn static_variant_key_tokens(
    facade_path: &TokenStream,
    variant_key: &FluentVariantKey,
) -> TokenStream {
    let variant_key = variant_key.as_str();
    quote! {
        #facade_path::registry::__macro::static_variant_key(#variant_key)
    }
}

pub fn core_error_to_compile_error(error: EsFluentCoreError) -> TokenStream {
    if let EsFluentCoreError::StructuredAttributeErrors(errors) = error {
        let errors = errors.into_iter().map(|error| {
            let message = error.to_string();
            match error.span {
                Some(span) => quote_spanned! { span=> compile_error!(#message); },
                None => quote! { compile_error!(#message); },
            }
        });
        return quote! { #(#errors)* };
    }

    let message = error.to_string();
    match error.span() {
        Some(span) => quote_spanned! { span=> compile_error!(#message); },
        None => quote! { compile_error!(#message); },
    }
}
