mod r#enum;
mod r#struct;

use darling::FromDeriveInput as _;
use es_fluent_derive_core::{
    options::{r#enum::EnumOpts, r#struct::StructOpts},
    validation,
};
use syn::{Data, DeriveInput, parse_macro_input};

pub fn from(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    let tokens = match &input.data {
        Data::Enum(data) => {
            let opts = match EnumOpts::from_derive_input(&input) {
                Ok(opts) => opts,
                Err(err) => return err.write_errors().into(),
            };

            // Validate namespace if provided
            if let Some(ns) = opts.attr_args().namespace()
                && let Err(err) = validation::validate_namespace(ns, Some(opts.ident().span()))
            {
                err.abort();
            }

            // Validate each variant's key individually in strict mode for better error reporting
            let base_key = opts.base_key();
            for variant_opt in opts.variants().iter().filter(|v| !v.is_skipped()) {
                let variant_key_suffix = variant_opt
                    .key()
                    .map(|key| key.to_string())
                    .unwrap_or_else(|| variant_opt.ident().to_string());
                let ftl_key = es_fluent_derive_core::namer::FluentKey::from(base_key.as_str())
                    .join(&variant_key_suffix)
                    .to_string();

                // Validate this specific key with the variant's span
                if let Err(err) = validation::validate_keys_in_strict_mode(
                    &[ftl_key],
                    Some(variant_opt.ident().span()),
                ) {
                    err.abort();
                }
            }

            r#enum::process_enum(&opts, data)
        },
        Data::Struct(data) => {
            let opts = match StructOpts::from_derive_input(&input) {
                Ok(opts) => opts,
                Err(err) => return err.write_errors().into(),
            };

            if let Err(err) = validation::validate_struct(&opts) {
                err.abort();
            }

            // Validate namespace if provided
            if let Some(ns) = opts.attr_args().namespace()
                && let Err(err) = validation::validate_namespace(ns, Some(opts.ident().span()))
            {
                err.abort();
            }

            // Collect the FTL key for strict mode validation
            let ftl_key = es_fluent_derive_core::namer::FluentKey::from(opts.ident()).to_string();
            let keys = vec![ftl_key];

            // Validate keys in strict mode
            if let Err(err) =
                validation::validate_keys_in_strict_mode(&keys, Some(opts.ident().span()))
            {
                err.abort();
            }

            r#struct::process_struct(&opts, data)
        },
        _ => panic!("Unsupported data type"),
    };

    tokens.into()
}
