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

            r#struct::process_struct(&opts, data)
        },
        _ => panic!("Unsupported data type"),
    };

    tokens.into()
}
