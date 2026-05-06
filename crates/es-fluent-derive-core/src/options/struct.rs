use darling::{FromDeriveInput, FromField};
use getset::Getters;

use crate::options::{FluentField, GeneratedVariantsOptions, StructDataOptions};

/// Options for a struct field.
#[derive(Clone, Debug, FromField)]
#[darling(attributes(fluent))]
pub struct StructFieldOpts {
    /// The identifier of the field.
    ident: Option<syn::Ident>,
    /// The type of the field.
    ty: syn::Type,
    #[darling(flatten)]
    attr_args: super::FluentFieldAttributeArgs,
    /// Whether this field is a default.
    #[darling(default)]
    default: Option<bool>,
}

impl StructFieldOpts {
    pub fn ident(&self) -> Option<&syn::Ident> {
        self.ident.as_ref()
    }

    pub fn ty(&self) -> &syn::Type {
        &self.ty
    }

    /// Returns `true` if the field is a default.
    pub fn is_default(&self) -> bool {
        self.default.unwrap_or(false)
    }
}

impl FluentField for StructFieldOpts {
    fn ident(&self) -> Option<&syn::Ident> {
        self.ident.as_ref()
    }

    fn ty(&self) -> &syn::Type {
        &self.ty
    }

    fn field_attr_args(&self) -> &super::FluentFieldAttributeArgs {
        &self.attr_args
    }
}

/// Options for a struct.
#[derive(Clone, Debug, FromDeriveInput, Getters)]
#[darling(supports(struct_named, struct_tuple, struct_unit), attributes(fluent))]
#[getset(get = "pub")]
pub struct StructOpts {
    /// The identifier of the struct.
    ident: syn::Ident,
    /// The generics of the struct.
    generics: syn::Generics,
    data: darling::ast::Data<darling::util::Ignored, StructFieldOpts>,
    #[darling(flatten)]
    attr_args: super::DerivedNamespacedAttributeArgs,
}

impl StructDataOptions for StructOpts {
    type Field = StructFieldOpts;

    fn struct_data(&self) -> &darling::ast::Data<darling::util::Ignored, Self::Field> {
        &self.data
    }
}

/// Options for a struct.
#[derive(Clone, Debug, FromDeriveInput, Getters)]
#[darling(supports(struct_named, struct_unit), attributes(fluent_variants))]
#[getset(get = "pub")]
pub struct StructVariantsOpts {
    /// The identifier of the struct.
    ident: syn::Ident,
    /// The generics of the struct.
    generics: syn::Generics,
    data: darling::ast::Data<darling::util::Ignored, super::SkippableFieldOpts>,
    #[darling(flatten)]
    attr_args: super::VariantsFluentAttributeArgs,
}

impl StructDataOptions for StructVariantsOpts {
    type Field = super::SkippableFieldOpts;

    fn struct_data(&self) -> &darling::ast::Data<darling::util::Ignored, Self::Field> {
        &self.data
    }
}

impl GeneratedVariantsOptions for StructVariantsOpts {
    fn variants_ident(&self) -> &syn::Ident {
        &self.ident
    }

    fn variants_attr_args(&self) -> &super::VariantsFluentAttributeArgs {
        &self.attr_args
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use es_fluent_shared::namespace::NamespaceRule;
    use quote::quote;
    use syn::{DeriveInput, parse_quote};

    #[test]
    fn struct_opts_cover_field_helpers_indexing_and_value_expressions() {
        let input: DeriveInput = parse_quote! {
            #[derive(EsFluent)]
            #[fluent(namespace = "forms")]
            struct LoginForm {
                #[fluent(default)]
                username: String,
                #[fluent(choice, value = "|v: &String| v.len()")]
                role: String,
                #[fluent(skip)]
                hidden: bool,
            }
        };

        let opts = StructOpts::from_derive_input(&input).expect("StructOpts should parse");
        assert!(matches!(
            opts.attr_args().namespace(),
            Some(NamespaceRule::Literal(value)) if value == "forms"
        ));

        let fields = opts.fields();
        assert_eq!(fields.len(), 2);
        assert_eq!(fields[0].fluent_arg_name(0), "username");
        assert!(fields[0].is_default());
        assert!(!fields[0].is_choice());

        assert_eq!(fields[1].fluent_arg_name(1), "role");
        assert!(fields[1].is_choice());
        let value_expr = fields[1]
            .value()
            .expect("value expression should be present");
        assert_eq!(
            quote!(#value_expr).to_string(),
            "| v : & String | v . len ()"
        );

        let indexed = opts.indexed_fields();
        assert_eq!(indexed.len(), 2);
        assert_eq!(indexed[0].0, 0);
        assert_eq!(indexed[1].0, 1);

        let all_indexed = opts.all_indexed_fields();
        assert_eq!(all_indexed.len(), 3);

        let tuple_input: DeriveInput = parse_quote! {
            #[derive(EsFluent)]
            struct TupleLogin(#[fluent(skip)] u8, String, bool);
        };
        let tuple_opts = StructOpts::from_derive_input(&tuple_input).expect("tuple struct parse");
        let tuple_fields = tuple_opts.fields();
        assert_eq!(tuple_fields.len(), 2);
        assert_eq!(tuple_fields[0].fluent_arg_name(1), "f1");
        assert_eq!(tuple_fields[1].fluent_arg_name(2), "f2");
    }

    #[test]
    fn struct_field_arg_name_overrides_work_for_named_and_tuple() {
        let named_input: DeriveInput = parse_quote! {
            #[derive(EsFluent)]
            struct Named {
                #[fluent(arg_name = "display_name")]
                name: String,
                value: String,
            }
        };
        let named_opts = StructOpts::from_derive_input(&named_input).expect("named parse");
        let named_fields = named_opts.fields();
        assert_eq!(named_fields[0].fluent_arg_name(0), "display_name");
        assert_eq!(named_fields[1].fluent_arg_name(1), "value");

        let tuple_input: DeriveInput = parse_quote! {
            #[derive(EsFluent)]
            struct Tuple(String, #[fluent(arg_name = "f1")] String, String);
        };
        let tuple_opts = StructOpts::from_derive_input(&tuple_input).expect("tuple parse");
        let tuple_fields = tuple_opts.fields();
        assert_eq!(tuple_fields[0].fluent_arg_name(0), "f0");
        assert_eq!(tuple_fields[1].fluent_arg_name(1), "f1");
        assert_eq!(tuple_fields[2].fluent_arg_name(2), "f2");
    }

    #[test]
    fn struct_variants_opts_cover_key_generation_and_field_filtering() {
        let input: DeriveInput = parse_quote! {
            #[derive(EsFluentVariants)]
            #[fluent_variants(keys = ["label_text", "placeholder_text"], namespace = "ui")]
            struct ProfileForm {
                user: String,
                #[fluent_variants(skip)]
                internal: bool,
            }
        };

        let opts =
            StructVariantsOpts::from_derive_input(&input).expect("StructVariantsOpts should parse");
        assert_eq!(opts.ftl_enum_ident().to_string(), "ProfileFormVariants");

        let keyed_idents: Vec<String> = opts
            .keyed_idents()
            .expect("keyed idents should parse")
            .into_iter()
            .map(|ident| ident.to_string())
            .collect();
        assert_eq!(
            keyed_idents,
            vec![
                "ProfileFormLabelTextVariants",
                "ProfileFormPlaceholderTextVariants",
            ]
        );

        let keyed_base_idents: Vec<String> = opts
            .keyed_base_idents()
            .expect("keyed base idents should parse")
            .into_iter()
            .map(|ident| ident.to_string())
            .collect();
        assert_eq!(
            keyed_base_idents,
            vec!["ProfileFormLabelText", "ProfileFormPlaceholderText"]
        );

        assert_eq!(
            opts.attr_args().key_strings(),
            Some(vec![
                "label_text".to_string(),
                "placeholder_text".to_string(),
            ])
        );
        assert!(matches!(
            opts.attr_args().namespace(),
            Some(NamespaceRule::Literal(value)) if value == "ui"
        ));

        let fields = opts.fields();
        assert_eq!(fields.len(), 1);
        assert_eq!(fields[0].ident().expect("named field").to_string(), "user");
    }

    #[test]
    fn struct_variants_key_methods_cover_empty_and_invalid_keys() {
        let no_key_input: DeriveInput = parse_quote! {
            #[derive(EsFluentVariants)]
            struct NoKeys {
                value: i32
            }
        };
        let no_key_opts =
            StructVariantsOpts::from_derive_input(&no_key_input).expect("parse without keys");
        assert!(no_key_opts.keyed_idents().expect("keyed_idents").is_empty());
        assert!(
            no_key_opts
                .keyed_base_idents()
                .expect("keyed_base_idents")
                .is_empty()
        );

        let invalid_key_input: DeriveInput = parse_quote! {
            #[derive(EsFluentVariants)]
            #[fluent_variants(keys = ["NotSnake"])]
            struct Invalid {
                value: i32
            }
        };
        let invalid_opts =
            StructVariantsOpts::from_derive_input(&invalid_key_input).expect("input should parse");

        let idents_err = invalid_opts
            .keyed_idents()
            .expect_err("invalid key should fail");
        assert!(idents_err.to_string().contains("lowercase snake_case"));

        let base_err = invalid_opts
            .keyed_base_idents()
            .expect_err("invalid key should fail");
        assert!(base_err.to_string().contains("lowercase snake_case"));
    }

    #[test]
    fn struct_methods_return_empty_on_unexpected_internal_shapes() {
        let struct_input: DeriveInput = parse_quote! {
            struct InternalShape {
                value: i32
            }
        };
        let mut struct_opts = StructOpts::from_derive_input(&struct_input).expect("StructOpts");
        struct_opts.data = darling::ast::Data::Enum(Vec::<darling::util::Ignored>::new());
        assert!(struct_opts.fields().is_empty());
        assert!(struct_opts.indexed_fields().is_empty());
        assert!(struct_opts.all_indexed_fields().is_empty());

        let variants_input: DeriveInput = parse_quote! {
            #[derive(EsFluentVariants)]
            struct InternalVariantsShape {
                value: i32
            }
        };
        let mut variants_opts =
            StructVariantsOpts::from_derive_input(&variants_input).expect("StructVariantsOpts");
        variants_opts.data = darling::ast::Data::Enum(Vec::<darling::util::Ignored>::new());
        assert!(variants_opts.fields().is_empty());
    }
}
