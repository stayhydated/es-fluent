use darling::FromDeriveInput as _;
use es_fluent_derive_core::options::r#enum::EnumVariantsOpts;
use es_fluent_derive_core::options::r#struct::StructVariantsOpts;
use es_fluent_derive_core::options::this::ThisOpts;
use es_fluent_derive_core::options::{
    FilteredEnumDataOptions as _, GeneratedVariantsOptions, StructDataOptions as _,
};
use es_fluent_derive_core::validation;
use es_fluent_shared::{namer, namespace::NamespaceRule};

use heck::ToPascalCase as _;
use proc_macro2::TokenStream;
use quote::quote;
use syn::{Data, DeriveInput, parse_macro_input};

use crate::macros::ir::GeneratedUnitEnumVariant;
use crate::macros::utils::{
    GeneratedUnitEnumInput, emit_default_or_keyed_items, emit_generated_unit_enum,
    inherited_fluent_domain, inherited_fluent_namespace, keyed_variant_idents_or_abort,
    namespace_rule_tokens, preferred_namespace,
};

pub fn from(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    let this_opts = ThisOpts::from_derive_input(&input).ok();
    let fluent_namespace = match inherited_fluent_namespace(&input) {
        Ok(namespace) => namespace,
        Err(err) => return err.write_errors().into(),
    };
    let fluent_domain = match inherited_fluent_domain(&input) {
        Ok(domain) => domain,
        Err(err) => return err.write_errors().into(),
    };

    let tokens = match &input.data {
        Data::Struct(_) => {
            let opts = match StructVariantsOpts::from_derive_input(&input) {
                Ok(opts) => opts,
                Err(err) => return err.write_errors().into(),
            };

            process_struct(
                &opts,
                this_opts.as_ref(),
                fluent_namespace.as_ref(),
                fluent_domain.as_deref(),
            )
        },
        Data::Enum(_) => {
            let opts = match EnumVariantsOpts::from_derive_input(&input) {
                Ok(opts) => opts,
                Err(err) => return err.write_errors().into(),
            };

            process_enum(
                &opts,
                this_opts.as_ref(),
                fluent_namespace.as_ref(),
                fluent_domain.as_deref(),
            )
        },
        Data::Union(_) => proc_macro_error2::abort!(
            input.ident.span(),
            "EsFluentVariants can only be derived for structs and enums"
        ),
    };

    tokens.into()
}

fn validate_namespace(namespace: Option<&NamespaceRule>, span: proc_macro2::Span) {
    if let Some(ns) = namespace
        && let Err(err) = validation::validate_namespace(ns, Some(span))
    {
        err.abort();
    }
}

fn resolved_variants_namespace<'a>(
    opts: &'a impl GeneratedVariantsOptions,
    this_opts: Option<&'a ThisOpts>,
    fluent_namespace: Option<&'a NamespaceRule>,
) -> Option<&'a NamespaceRule> {
    preferred_namespace([
        fluent_namespace,
        opts.variants_attr_args().namespace(),
        this_opts.and_then(|opts| opts.attr_args().namespace()),
    ])
}

pub fn process_struct(
    opts: &StructVariantsOpts,
    this_opts: Option<&ThisOpts>,
    fluent_namespace: Option<&NamespaceRule>,
    fluent_domain: Option<&str>,
) -> TokenStream {
    let variant_seeds = build_struct_variant_seeds(opts);
    emit_variants_output(
        opts,
        &variant_seeds,
        this_opts,
        fluent_namespace,
        fluent_domain,
    )
}

#[derive(Clone)]
struct GeneratedVariantSeed {
    ident: syn::Ident,
    doc_name: String,
    key_fragment: String,
}

impl GeneratedVariantSeed {
    fn materialize(&self, base_key: &namer::FluentKey) -> GeneratedUnitEnumVariant {
        GeneratedUnitEnumVariant {
            ident: self.ident.clone(),
            doc_name: self.doc_name.clone(),
            ftl_key: base_key.join(&self.key_fragment).to_string(),
        }
    }
}

fn variants_this_key(this_opts: Option<&ThisOpts>, base_key: &namer::FluentKey) -> Option<String> {
    this_opts
        .filter(|opts| opts.attr_args().is_variants())
        .map(|_| format!("{}{}", base_key, namer::FluentKey::THIS_SUFFIX))
}

fn emit_variants_output(
    opts: &impl GeneratedVariantsOptions,
    variant_seeds: &[GeneratedVariantSeed],
    this_opts: Option<&ThisOpts>,
    fluent_namespace: Option<&NamespaceRule>,
    fluent_domain: Option<&str>,
) -> TokenStream {
    if variant_seeds.is_empty() {
        return quote! {};
    }

    let keys = keyed_variant_idents_or_abort(opts);
    let key_strings = opts.variants_attr_args().key_strings().unwrap_or_default();
    let derives: Vec<syn::Path> = (*opts.variants_attr_args().derive()).to_vec();
    let namespace = resolved_variants_namespace(opts, this_opts, fluent_namespace);
    let origin_ident = opts.variants_ident();
    validate_namespace(namespace, origin_ident.span());
    let namespace_expr = namespace_rule_tokens(namespace);
    let ftl_enum_ident = opts.ftl_enum_ident();

    emit_default_or_keyed_items(&ftl_enum_ident, &keys, &key_strings, |ident, key_name| {
        let base_key = namer::FluentKey::from(ident);
        let variant_entries: Vec<_> = variant_seeds
            .iter()
            .map(|seed| seed.materialize(&base_key))
            .collect();

        emit_generated_unit_enum(GeneratedUnitEnumInput {
            ident,
            origin_ident,
            key_name,
            domain_override: fluent_domain,
            derives: &derives,
            variants: &variant_entries,
            namespace_expr: namespace_expr.clone(),
            this_key: variants_this_key(this_opts, &base_key),
        })
    })
}

fn build_struct_variant_seeds(opts: &StructVariantsOpts) -> Vec<GeneratedVariantSeed> {
    opts.fields()
        .iter()
        .map(|field_opt| {
            let field_ident = field_opt.ident().expect("named field");
            let original_field_name = field_ident.to_string();
            let pascal_case_name = original_field_name.to_pascal_case();
            let variant_ident = syn::Ident::new(&pascal_case_name, field_ident.span());
            GeneratedVariantSeed {
                ident: variant_ident,
                doc_name: original_field_name,
                key_fragment: field_ident.to_string(),
            }
        })
        .collect()
}

pub fn process_enum(
    opts: &EnumVariantsOpts,
    this_opts: Option<&ThisOpts>,
    fluent_namespace: Option<&NamespaceRule>,
    fluent_domain: Option<&str>,
) -> TokenStream {
    let variant_seeds = build_enum_variant_seeds(opts);
    emit_variants_output(
        opts,
        &variant_seeds,
        this_opts,
        fluent_namespace,
        fluent_domain,
    )
}

fn build_enum_variant_seeds(opts: &EnumVariantsOpts) -> Vec<GeneratedVariantSeed> {
    opts.variants()
        .iter()
        .map(|variant_opt| {
            let variant_ident = variant_opt.ident();
            let variant_key = variant_ident.to_string();
            GeneratedVariantSeed {
                ident: variant_ident.clone(),
                doc_name: variant_key.clone(),
                key_fragment: variant_key,
            }
        })
        .collect()
}

#[cfg(all(test, target_os = "linux"))]
mod tests {
    use super::{process_enum, process_struct};
    use crate::macros::utils::{inherited_fluent_domain, inherited_fluent_namespace};
    use crate::snapshot_support::pretty_file_tokens;
    use darling::FromDeriveInput as _;
    use es_fluent_derive_core::options::{
        r#enum::EnumVariantsOpts, r#struct::StructVariantsOpts, this::ThisOpts,
    };
    use insta::assert_snapshot;
    use syn::parse_quote;

    #[test]
    fn process_struct_emits_keyed_generated_enums() {
        let input: syn::DeriveInput = parse_quote! {
            #[fluent(namespace = "ui")]
            #[fluent_variants(keys = ["label", "placeholder"], derive(Debug))]
            struct LoginForm {
                username: String,
                password: String,
            }
        };

        let opts = StructVariantsOpts::from_derive_input(&input).expect("StructVariantsOpts");
        let this_opts = ThisOpts::from_derive_input(&input).ok();
        let fluent_namespace = inherited_fluent_namespace(&input).expect("parent namespace");

        let tokens = pretty_file_tokens(process_struct(
            &opts,
            this_opts.as_ref(),
            fluent_namespace.as_ref(),
            None,
        ));
        assert_snapshot!("process_struct_emits_keyed_generated_enums", tokens);
    }

    #[test]
    fn process_enum_emits_variants_this_registration() {
        let input: syn::DeriveInput = parse_quote! {
            #[fluent(namespace = "ui")]
            #[fluent_this(variants)]
            enum Status {
                Ready,
                Failed,
            }
        };

        let opts = EnumVariantsOpts::from_derive_input(&input).expect("EnumVariantsOpts");
        let this_opts = ThisOpts::from_derive_input(&input).ok();
        let fluent_namespace = inherited_fluent_namespace(&input).expect("parent namespace");

        let tokens = pretty_file_tokens(process_enum(
            &opts,
            this_opts.as_ref(),
            fluent_namespace.as_ref(),
            None,
        ));
        assert_snapshot!("process_enum_emits_variants_this_registration", tokens);
    }

    #[test]
    fn process_enum_uses_parent_domain_for_generated_variants_and_this() {
        let input: syn::DeriveInput = parse_quote! {
            #[fluent(domain = "es-fluent-lang", resource = "es-fluent-lang", namespace = "languages")]
            #[fluent_this(variants)]
            enum Language {
                English,
                French,
            }
        };

        let opts = EnumVariantsOpts::from_derive_input(&input).expect("EnumVariantsOpts");
        let this_opts = ThisOpts::from_derive_input(&input).ok();
        let fluent_namespace = inherited_fluent_namespace(&input).expect("parent namespace");
        let fluent_domain = inherited_fluent_domain(&input).expect("parent domain");

        let tokens = pretty_file_tokens(process_enum(
            &opts,
            this_opts.as_ref(),
            fluent_namespace.as_ref(),
            fluent_domain.as_deref(),
        ));

        assert!(
            tokens.contains("localize(\"es-fluent-lang\", \"language_variants-English\", None)")
        );
        assert!(
            tokens.contains("localize(\"es-fluent-lang\", \"language_variants-French\", None)")
        );
        assert!(tokens.contains(
            "::es_fluent::__private::localize_this(\n            localizer,\n            \"es-fluent-lang\",\n            \"language_variants_this\","
        ));
        assert!(
            !tokens.contains("localize(env!(\"CARGO_PKG_NAME\"), \"language_variants-English\"")
        );
    }

    #[test]
    fn process_variants_prefers_parent_namespace_over_variants_and_this_namespaces() {
        let input: syn::DeriveInput = parse_quote! {
            #[fluent(namespace = "parent_ns")]
            #[fluent_variants(namespace = "variant_ns")]
            #[fluent_this(variants, namespace = "this_ns")]
            struct NamespaceHolder {
                field: String,
            }
        };

        let opts = StructVariantsOpts::from_derive_input(&input).expect("StructVariantsOpts");
        let this_opts = ThisOpts::from_derive_input(&input).ok();
        let fluent_namespace = inherited_fluent_namespace(&input).expect("parent namespace");

        let tokens = pretty_file_tokens(process_struct(
            &opts,
            this_opts.as_ref(),
            fluent_namespace.as_ref(),
            None,
        ));
        assert_snapshot!(
            "process_variants_prefers_parent_namespace_over_variants_and_this_namespaces",
            tokens
        );
    }
}
