use crate::error::{ErrorExt as _, EsFluentCoreError, EsFluentCoreResult};
use crate::options::r#enum::EnumOpts;
use crate::options::r#struct::StructOpts;
use syn::{DataEnum, DataStruct};

pub fn validate_enum(_opts: &EnumOpts, _data: &DataEnum) -> EsFluentCoreResult<()> {
    Ok(())
}

pub fn validate_struct(opts: &StructOpts, _data: &DataStruct) -> EsFluentCoreResult<()> {
    validate_struct_defaults(opts)?;
    Ok(())
}

fn validate_struct_defaults(opts: &StructOpts) -> EsFluentCoreResult<()> {
    let fields = opts.fields();
    let default_fields: Vec<_> = fields.iter().filter(|f| f.is_default()).collect();

    if default_fields.len() > 1
        && let Some(first_field_ident) = default_fields[0].ident().as_ref()
        && let Some(second_field_ident) = default_fields[1].ident().as_ref()
    {
        return Err(EsFluentCoreError::FieldError {
            message: "Struct cannot have multiple fields marked `#[fluent(default)]`.".to_string(),
            field_name: Some(second_field_ident.to_string()),
            span: Some(second_field_ident.span()),
        }
        .with_note(format!(
            "First `#[fluent(default)]` field found was `{}`.",
            first_field_ident
        )));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use darling::FromDeriveInput;
    use syn::{DataStruct, DeriveInput, parse_quote};

    fn create_struct_opts_with_multiple_defaults() -> crate::options::r#struct::StructOpts {
        let input: DeriveInput = parse_quote! {
            #[derive(EsFluent)]
            pub struct TestStruct {
                #[fluent(default)]
                field1: String,
                #[fluent(default)]
                field2: i32,
            }
        };

        crate::options::r#struct::StructOpts::from_derive_input(&input).unwrap()
    }

    fn create_struct_opts_with_single_default() -> crate::options::r#struct::StructOpts {
        let input: DeriveInput = parse_quote! {
            #[derive(EsFluent)]
            pub struct TestStruct {
                #[fluent(default)]
                field1: String,
                field2: i32,
            }
        };

        crate::options::r#struct::StructOpts::from_derive_input(&input).unwrap()
    }

    fn create_struct_opts_with_no_defaults() -> crate::options::r#struct::StructOpts {
        let input: DeriveInput = parse_quote! {
            #[derive(EsFluent)]
            pub struct TestStruct {
                field1: String,
                field2: i32,
            }
        };

        crate::options::r#struct::StructOpts::from_derive_input(&input).unwrap()
    }

    #[test]
    fn test_validate_struct_no_defaults_passes() {
        let opts = create_struct_opts_with_no_defaults();
        let data = DataStruct {
            struct_token: Default::default(),
            fields: syn::Fields::Named(parse_quote! { { field1: String, field2: i32 } }),
            semi_token: None,
        };
        let result = validate_struct(&opts, &data);
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_struct_single_default_passes() {
        let opts = create_struct_opts_with_single_default();
        let data = DataStruct {
            struct_token: Default::default(),
            fields: syn::Fields::Named(parse_quote! { { field1: String, field2: i32 } }),
            semi_token: None,
        };
        let result = validate_struct(&opts, &data);
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_struct_multiple_defaults_fails() {
        let opts = create_struct_opts_with_multiple_defaults();
        let data = DataStruct {
            struct_token: Default::default(),
            fields: syn::Fields::Named(parse_quote! { { field1: String, field2: i32 } }),
            semi_token: None,
        };
        let result = validate_struct(&opts, &data);
        assert!(result.is_err());
    }
}
