use bon::Builder;
use darling::{FromDeriveInput, FromField, FromMeta};
use getset::Getters;
use quote::format_ident;

use crate::error::EsFluentCoreResult;
use crate::namer;

/// Options for a struct field.
#[derive(Clone, Debug, FromField, Getters)]
#[darling(attributes(fluent))]
pub struct StructFieldOpts {
    /// The identifier of the field.
    #[getset(get = "pub")]
    ident: Option<syn::Ident>,
    /// The type of the field.
    #[getset(get = "pub")]
    ty: syn::Type,
    /// Whether to skip this field.
    #[darling(default)]
    skip: Option<bool>,
    /// Whether this field is a default.
    #[darling(default)]
    default: Option<bool>,
    /// Whether this field is a choice.
    #[darling(default)]
    choice: Option<bool>,
    /// A value transformation expression.
    #[darling(default)]
    value: Option<super::ValueAttr>,
    /// Optional argument name override.
    #[darling(default)]
    arg_name: Option<syn::LitStr>,
}

impl StructFieldOpts {
    /// Returns `true` if the field should be skipped.
    pub fn is_skipped(&self) -> bool {
        self.skip.unwrap_or(false)
    }

    /// Returns `true` if the field is a default.
    pub fn is_default(&self) -> bool {
        self.default.unwrap_or(false)
    }

    /// Returns `true` if the field is a choice.
    pub fn is_choice(&self) -> bool {
        self.choice.unwrap_or(false)
    }

    /// Returns the Fluent argument name for this field.
    pub fn fluent_arg_name(&self, index: usize) -> String {
        if let Some(arg_name) = self.arg_name() {
            arg_name
        } else {
            self.ident
                .as_ref()
                .map(|ident| ident.to_string())
                .unwrap_or_else(|| namer::UnnamedItem::from(index).to_string())
        }
    }

    /// Returns the value expression if present.
    pub fn value(&self) -> Option<&syn::Expr> {
        self.value.as_ref().map(|v| &v.0)
    }

    /// Returns explicit field argument name if provided.
    pub fn arg_name(&self) -> Option<String> {
        self.arg_name.as_ref().map(syn::LitStr::value)
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
    attr_args: StructFluentAttributeArgs,
}

impl StructOpts {
    /// Returns the fields of the struct that are not skipped.
    pub fn fields(&self) -> Vec<&StructFieldOpts> {
        match &self.data {
            darling::ast::Data::Struct(fields) => fields
                .fields
                .iter()
                .filter(|field| !field.is_skipped())
                .collect(),
            _ => vec![],
        }
    }

    /// Returns the fields of the struct paired with their declaration index.
    pub fn indexed_fields(&self) -> Vec<(usize, &StructFieldOpts)> {
        match &self.data {
            darling::ast::Data::Struct(fields) => fields
                .fields
                .iter()
                .enumerate()
                .filter(|(_, field)| !field.is_skipped())
                .collect(),
            _ => vec![],
        }
    }

    /// Returns all fields (including skipped) paired with their declaration index.
    pub fn all_indexed_fields(&self) -> Vec<(usize, &StructFieldOpts)> {
        match &self.data {
            darling::ast::Data::Struct(fields) => fields.fields.iter().enumerate().collect(),
            _ => vec![],
        }
    }
}

/// Attribute arguments for a struct.
#[derive(Builder, Clone, Debug, Default, FromMeta, Getters)]
pub struct StructFluentAttributeArgs {
    /// The traits to derive on the FTL enum.
    #[getset(get = "pub")]
    #[darling(default)]
    derive: darling::util::PathList,
    /// Optional namespace for FTL file generation.
    /// - `namespace = "name"` - writes to `{lang}/{crate}/{name}.ftl`
    /// - `namespace = file` - writes to `{lang}/{crate}/{source_file_stem}.ftl`
    /// - `namespace(file(relative))` - writes to `{lang}/{crate}/{relative_file_path}.ftl`
    /// - `namespace = folder` - writes to `{lang}/{crate}/{source_parent_folder}.ftl`
    /// - `namespace(folder(relative))` - writes to `{lang}/{crate}/{relative_parent_folder_path}.ftl`
    #[darling(default)]
    namespace: Option<super::namespace::NamespaceValue>,
}
impl StructFluentAttributeArgs {
    /// Returns the namespace value if provided.
    pub fn namespace(&self) -> Option<&super::namespace::NamespaceValue> {
        self.namespace.as_ref()
    }
}

/// Options for a struct field.
#[derive(Clone, Debug, FromField, Getters)]
#[darling(attributes(fluent_variants))]
pub struct StructVariantsFieldOpts {
    /// The identifier of the field.
    #[getset(get = "pub")]
    ident: Option<syn::Ident>,
    /// The type of the field.
    #[getset(get = "pub")]
    ty: syn::Type,
    /// Whether to skip this field.
    #[darling(default)]
    skip: Option<bool>,
}

impl StructVariantsFieldOpts {
    /// Returns `true` if the field should be skipped.
    pub fn is_skipped(&self) -> bool {
        self.skip.unwrap_or(false)
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
    data: darling::ast::Data<darling::util::Ignored, StructVariantsFieldOpts>,
    #[darling(flatten)]
    attr_args: StructVariantsFluentAttributeArgs,
}

impl StructVariantsOpts {
    const FTL_ENUM_IDENT: &str = "Variants";

    /// Returns the identifier of the FTL enum.
    pub fn ftl_enum_ident(&self) -> syn::Ident {
        format_ident!("{}{}", &self.ident, Self::FTL_ENUM_IDENT)
    }

    /// Returns the identifiers of the keyed FTL enums.
    pub fn keyed_idents(&self) -> EsFluentCoreResult<Vec<syn::Ident>> {
        self.attr_args.clone().keys.map_or_else(
            || Ok(Vec::new()),
            |keys| {
                keys.into_iter()
                    .map(|key| {
                        let pascal_key = super::validate_snake_case_key(&key)?;
                        Ok(format_ident!(
                            "{}{}{}",
                            &self.ident,
                            pascal_key,
                            Self::FTL_ENUM_IDENT
                        ))
                    })
                    .collect()
            },
        )
    }

    /// Returns the identifiers used to build base FTL keys (without suffixes).
    pub fn keyed_base_idents(&self) -> EsFluentCoreResult<Vec<syn::Ident>> {
        self.attr_args.clone().keys.map_or_else(
            || Ok(Vec::new()),
            |keys| {
                keys.into_iter()
                    .map(|key| {
                        let pascal_key = super::validate_snake_case_key(&key)?;
                        Ok(format_ident!("{}{}", &self.ident, pascal_key))
                    })
                    .collect()
            },
        )
    }

    /// Returns the fields of the struct that are not skipped.
    pub fn fields(&self) -> Vec<&StructVariantsFieldOpts> {
        match &self.data {
            darling::ast::Data::Struct(fields) => fields
                .fields
                .iter()
                .filter(|field| !field.is_skipped())
                .collect(),
            _ => vec![],
        }
    }
}

/// Attribute arguments for a struct.
#[derive(Builder, Clone, Debug, Default, FromMeta, Getters)]
pub struct StructVariantsFluentAttributeArgs {
    #[darling(default)]
    keys: Option<Vec<syn::LitStr>>,
    /// The traits to derive on the FTL enum.
    #[getset(get = "pub")]
    #[darling(default)]
    derive: darling::util::PathList,
    /// Optional namespace for FTL file generation.
    /// - `namespace = "name"` - writes to `{lang}/{crate}/{name}.ftl`
    /// - `namespace = file` - writes to `{lang}/{crate}/{source_file_stem}.ftl`
    /// - `namespace(file(relative))` - writes to `{lang}/{crate}/{relative_path}.ftl`
    /// - `namespace = folder` - writes to `{lang}/{crate}/{source_parent_folder}.ftl`
    /// - `namespace(folder(relative))` - writes to `{lang}/{crate}/{relative_parent_folder_path}.ftl`
    #[darling(default)]
    namespace: Option<super::namespace::NamespaceValue>,
}

impl StructVariantsFluentAttributeArgs {
    /// Returns the namespace value if provided.
    pub fn namespace(&self) -> Option<&super::namespace::NamespaceValue> {
        self.namespace.as_ref()
    }

    /// Returns the raw key strings if provided.
    pub fn key_strings(&self) -> Option<Vec<String>> {
        self.keys
            .as_ref()
            .map(|keys| keys.iter().map(|k| k.value()).collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::options::namespace::NamespaceValue;
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
            Some(NamespaceValue::Literal(value)) if value == "forms"
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
            Some(NamespaceValue::Literal(value)) if value == "ui"
        ));

        let fields = opts.fields();
        assert_eq!(fields.len(), 1);
        assert_eq!(
            fields[0].ident().as_ref().expect("named field").to_string(),
            "user"
        );
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
