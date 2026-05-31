use darling::FromDeriveInput as _;
use es_fluent_derive_core::error::AttrContext;
use es_fluent_derive_core::options::r#enum::EnumVariantsOpts;
use es_fluent_derive_core::options::label::LabelOpts;
use es_fluent_derive_core::options::r#struct::StructVariantsOpts;
use es_fluent_derive_core::options::{
    FilteredEnumDataOptions as _, GeneratedVariantsOptions, StructDataOptions as _,
};
use es_fluent_derive_core::semantic::{DomainName, FluentMessageId};
use es_fluent_derive_core::validation;
use es_fluent_shared::{namer, namespace::NamespaceRule};

use heck::ToPascalCase as _;
use proc_macro2::TokenStream;
use quote::quote;
use syn::{Data, DeriveInput, parse_macro_input};

use crate::macros::ir::{GeneratedUnitEnumVariant, MessageEntrySpec};
use crate::macros::utils::{GeneratedUnitEnumInput, SpannedNamespaceRule, SpannedNamespaceRuleRef};

pub fn from(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    expand_es_fluent_variants(input).into()
}

fn expand_es_fluent_variants(input: DeriveInput) -> TokenStream {
    if matches!(&input.data, Data::Union(_)) {
        proc_macro_error2::abort!(
            input.ident.span(),
            "EsFluentVariants can only be derived for structs and enums"
        );
    }

    let label_opts = match LabelOpts::from_derive_input(&input) {
        Ok(opts) => opts,
        Err(err) => return err.write_errors(),
    };
    let fluent_namespace = match crate::macros::utils::inherited_fluent_namespace_with_span(&input)
    {
        Ok(namespace) => namespace,
        Err(err) => return err.write_errors(),
    };
    let fluent_domain = match crate::macros::utils::inherited_fluent_domain(&input) {
        Ok(domain) => domain,
        Err(err) => return err.write_errors(),
    };

    match &input.data {
        Data::Struct(_) => {
            let opts = match StructVariantsOpts::from_derive_input(&input) {
                Ok(opts) => opts,
                Err(err) => return err.write_errors(),
            };

            process_struct(
                &opts,
                Some(&label_opts),
                fluent_namespace.as_ref().map(SpannedNamespaceRule::as_ref),
                fluent_domain.as_ref(),
            )
        },
        Data::Enum(_) => {
            let opts = match EnumVariantsOpts::from_derive_input(&input) {
                Ok(opts) => opts,
                Err(err) => return err.write_errors(),
            };

            process_enum(
                &opts,
                Some(&label_opts),
                fluent_namespace.as_ref().map(SpannedNamespaceRule::as_ref),
                fluent_domain.as_ref(),
            )
        },
        Data::Union(_) => syn::Error::new(
            input.ident.span(),
            "EsFluentVariants can only be derived for structs and enums",
        )
        .to_compile_error(),
    }
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
    label_opts: Option<&'a LabelOpts>,
    fluent_namespace: Option<SpannedNamespaceRuleRef<'a>>,
) -> Option<SpannedNamespaceRuleRef<'a>> {
    let variants_namespace = opts.variants_attr_args().namespace().map(|namespace| {
        SpannedNamespaceRuleRef::new(
            namespace,
            opts.variants_attr_args()
                .namespace_span()
                .unwrap_or_else(|| opts.variants_ident().span()),
        )
    });
    let label_namespace = label_opts.and_then(|opts| {
        opts.attr_args().namespace().map(|namespace| {
            SpannedNamespaceRuleRef::new(
                namespace,
                opts.attr_args()
                    .namespace_span()
                    .unwrap_or_else(|| opts.ident().span()),
            )
        })
    });

    crate::macros::utils::preferred_spanned_namespace([
        fluent_namespace,
        variants_namespace,
        label_namespace,
    ])
}

pub fn process_struct(
    opts: &StructVariantsOpts,
    label_opts: Option<&LabelOpts>,
    fluent_namespace: Option<SpannedNamespaceRuleRef<'_>>,
    fluent_domain: Option<&DomainName>,
) -> TokenStream {
    let variant_seeds = build_struct_variant_seeds(opts);
    emit_variants_output(
        opts,
        &variant_seeds,
        label_opts,
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

struct NamedVariantFieldSeed<'a> {
    ident: &'a syn::Ident,
}

impl<'a> NamedVariantFieldSeed<'a> {
    fn from_field(field: &'a es_fluent_derive_core::options::SkippableFieldOpts) -> Self {
        let Some(ident) = field.ident() else {
            proc_macro_error2::abort!(
                proc_macro2::Span::call_site(),
                "internal error: generated struct variant field is missing an identifier"
            );
        };
        Self { ident }
    }
}

impl GeneratedVariantSeed {
    fn materialize(&self, base_key: &namer::FluentKey) -> GeneratedUnitEnumVariant {
        let ftl_key = base_key.join(&self.key_fragment).to_string();
        let message_id = crate::macros::utils::message_id_or_abort(
            ftl_key,
            self.ident.span(),
            AttrContext::VariantsContainer,
        );
        GeneratedUnitEnumVariant {
            ident: self.ident.clone(),
            doc_name: self.doc_name.clone(),
            message_entry: MessageEntrySpec::new(
                namer::rust_ident_name(&self.ident),
                message_id,
                Vec::new(),
            ),
        }
    }
}

fn variants_label_key(
    label_opts: Option<&LabelOpts>,
    base_key: &namer::FluentKey,
    span: proc_macro2::Span,
) -> Option<FluentMessageId> {
    label_opts
        .filter(|opts| opts.attr_args().is_variants())
        .map(|_| {
            crate::macros::utils::message_id_or_abort(
                format!("{}{}", base_key, namer::FluentKey::LABEL_SUFFIX),
                span,
                AttrContext::VariantsContainer,
            )
            .into_value()
        })
}

fn emit_variants_output(
    opts: &impl GeneratedVariantsOptions,
    variant_seeds: &[GeneratedVariantSeed],
    label_opts: Option<&LabelOpts>,
    fluent_namespace: Option<SpannedNamespaceRuleRef<'_>>,
    fluent_domain: Option<&DomainName>,
) -> TokenStream {
    if variant_seeds.is_empty() {
        return quote! {};
    }

    let keys = crate::macros::utils::keyed_variant_idents_or_abort(opts);
    let key_strings = opts.variants_attr_args().key_strings().unwrap_or_default();
    let derives: Vec<syn::Path> = (*opts.variants_attr_args().derive()).to_vec();
    let namespace = resolved_variants_namespace(opts, label_opts, fluent_namespace);
    let origin_ident = opts.variants_ident();
    validate_namespace(
        namespace.map(SpannedNamespaceRuleRef::rule),
        namespace
            .map(SpannedNamespaceRuleRef::span)
            .unwrap_or_else(|| origin_ident.span()),
    );
    let ftl_enum_ident = opts.ftl_enum_ident();

    crate::macros::utils::emit_default_or_keyed_items(
        &ftl_enum_ident,
        &keys,
        &key_strings,
        |ident, key_name| {
            let base_key = namer::FluentKey::from(ident);
            let variant_entries: Vec<_> = variant_seeds
                .iter()
                .map(|seed| seed.materialize(&base_key))
                .collect();

            crate::macros::utils::emit_generated_unit_enum(GeneratedUnitEnumInput {
                ident,
                origin_ident,
                key_name,
                domain_override: fluent_domain,
                derives: &derives,
                variants: &variant_entries,
                namespace: namespace.map(SpannedNamespaceRuleRef::rule),
                label_key: variants_label_key(label_opts, &base_key, origin_ident.span()),
            })
        },
    )
}

fn build_struct_variant_seeds(opts: &StructVariantsOpts) -> Vec<GeneratedVariantSeed> {
    opts.fields()
        .iter()
        .map(|field_opt| {
            let field = NamedVariantFieldSeed::from_field(field_opt);
            let field_ident = field.ident;
            let original_field_name = namer::rust_ident_name(field_ident);
            let pascal_case_name = original_field_name.to_pascal_case();
            let variant_ident = syn::Ident::new(&pascal_case_name, field_ident.span());
            GeneratedVariantSeed {
                ident: variant_ident,
                doc_name: original_field_name,
                key_fragment: namer::rust_ident_name(field_ident),
            }
        })
        .collect()
}

pub fn process_enum(
    opts: &EnumVariantsOpts,
    label_opts: Option<&LabelOpts>,
    fluent_namespace: Option<SpannedNamespaceRuleRef<'_>>,
    fluent_domain: Option<&DomainName>,
) -> TokenStream {
    let variant_seeds = build_enum_variant_seeds(opts);
    emit_variants_output(
        opts,
        &variant_seeds,
        label_opts,
        fluent_namespace,
        fluent_domain,
    )
}

fn build_enum_variant_seeds(opts: &EnumVariantsOpts) -> Vec<GeneratedVariantSeed> {
    opts.variants()
        .iter()
        .map(|variant_opt| {
            let variant_ident = variant_opt.ident();
            let variant_key = namer::rust_ident_name(variant_ident);
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
    use crate::macros::ir::inventory_variant_tokens_for_model;
    use darling::FromDeriveInput as _;
    use es_fluent_derive_core::options::{
        r#enum::EnumVariantsOpts, label::LabelOpts, r#struct::StructVariantsOpts,
    };
    use insta::assert_snapshot;
    use syn::parse_quote;

    #[test]
    fn expand_es_fluent_variants_reports_invalid_fluent_label_attribute() {
        let input: syn::DeriveInput = parse_quote! {
            #[fluent_label(variantz)]
            struct LoginForm {
                username: String,
            }
        };

        let tokens = super::expand_es_fluent_variants(input).to_string();

        assert!(tokens.contains("compile_error"));
        assert!(tokens.contains("variantz"));
    }

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
        let label_opts = LabelOpts::from_derive_input(&input).ok();
        let fluent_namespace = crate::macros::utils::inherited_fluent_namespace_with_span(&input)
            .expect("parent namespace");

        let tokens = crate::snapshot_support::pretty_file_tokens(super::process_struct(
            &opts,
            label_opts.as_ref(),
            fluent_namespace
                .as_ref()
                .map(crate::macros::utils::SpannedNamespaceRule::as_ref),
            None,
        ));
        assert_snapshot!("process_struct_emits_keyed_generated_enums", tokens);
    }

    #[test]
    fn generated_variant_entry_drives_runtime_and_inventory_metadata() {
        let seed = super::GeneratedVariantSeed {
            ident: syn::parse_quote!(Username),
            doc_name: "username".to_string(),
            key_fragment: "username".to_string(),
        };
        let entry = seed.materialize(&es_fluent_shared::namer::FluentKey::from(
            "login_form_label_variants",
        ));

        assert_eq!(
            entry.message_entry.metadata.message_id().as_str(),
            "login_form_label_variants-username"
        );
        assert_eq!(
            entry.message_entry.metadata.argument_names(),
            Vec::<es_fluent_derive_core::semantic::ArgName>::new()
        );

        let runtime_tokens = entry.localize_with_match_arm(None).to_string();
        let inventory_tokens =
            inventory_variant_tokens_for_model(&entry.message_entry.metadata).to_string();

        assert!(runtime_tokens.contains("\"login_form_label_variants-username\""));
        assert!(inventory_tokens.contains("ftl_key : \"login_form_label_variants-username\""));
        assert!(inventory_tokens.contains("name : \"Username\""));
        assert!(inventory_tokens.contains("args : & []"));
    }

    #[test]
    fn process_enum_emits_variants_label_registration() {
        let input: syn::DeriveInput = parse_quote! {
            #[fluent(namespace = "ui")]
            #[fluent_label(variants = true)]
            enum Status {
                Ready,
                Failed,
            }
        };

        let opts = EnumVariantsOpts::from_derive_input(&input).expect("EnumVariantsOpts");
        let label_opts = LabelOpts::from_derive_input(&input).ok();
        let fluent_namespace = crate::macros::utils::inherited_fluent_namespace_with_span(&input)
            .expect("parent namespace");

        let tokens = crate::snapshot_support::pretty_file_tokens(super::process_enum(
            &opts,
            label_opts.as_ref(),
            fluent_namespace
                .as_ref()
                .map(crate::macros::utils::SpannedNamespaceRule::as_ref),
            None,
        ));
        assert_snapshot!("process_enum_emits_variants_label_registration", tokens);
    }

    #[test]
    fn process_enum_uses_parent_domain_for_generated_variants_and_label() {
        let input: syn::DeriveInput = parse_quote! {
            #[fluent(domain = "es-fluent-lang", resource = "es-fluent-lang", namespace = "languages")]
            #[fluent_label(variants = true)]
            enum Language {
                English,
                French,
            }
        };

        let opts = EnumVariantsOpts::from_derive_input(&input).expect("EnumVariantsOpts");
        let label_opts = LabelOpts::from_derive_input(&input).ok();
        let fluent_namespace = crate::macros::utils::inherited_fluent_namespace_with_span(&input)
            .expect("parent namespace");
        let fluent_domain =
            crate::macros::utils::inherited_fluent_domain(&input).expect("parent domain");

        let tokens = crate::snapshot_support::pretty_file_tokens(super::process_enum(
            &opts,
            label_opts.as_ref(),
            fluent_namespace
                .as_ref()
                .map(crate::macros::utils::SpannedNamespaceRule::as_ref),
            fluent_domain.as_ref(),
        ));

        assert!(
            tokens.contains("localize(\"es-fluent-lang\", \"language_variants-English\", None)")
        );
        assert!(
            tokens.contains("localize(\"es-fluent-lang\", \"language_variants-French\", None)")
        );
        assert!(tokens.contains(
            "::es_fluent::__private::localize_label(\n            localizer,\n            \"es-fluent-lang\",\n            \"language_variants_label\","
        ));
        assert!(
            !tokens.contains("localize(env!(\"CARGO_PKG_NAME\"), \"language_variants-English\"")
        );
    }

    #[test]
    fn process_variants_prefers_parent_namespace_over_variants_and_label_namespaces() {
        let input: syn::DeriveInput = parse_quote! {
            #[fluent(namespace = "parent_ns")]
            #[fluent_variants(namespace = "variant_ns")]
            #[fluent_label(variants = true, namespace = "label_ns")]
            struct NamespaceHolder {
                field: String,
            }
        };

        let opts = StructVariantsOpts::from_derive_input(&input).expect("StructVariantsOpts");
        let label_opts = LabelOpts::from_derive_input(&input).ok();
        let fluent_namespace = crate::macros::utils::inherited_fluent_namespace_with_span(&input)
            .expect("parent namespace");

        let tokens = crate::snapshot_support::pretty_file_tokens(super::process_struct(
            &opts,
            label_opts.as_ref(),
            fluent_namespace
                .as_ref()
                .map(crate::macros::utils::SpannedNamespaceRule::as_ref),
            None,
        ));
        assert_snapshot!(
            "process_variants_prefers_parent_namespace_over_variants_and_label_namespaces",
            tokens
        );
    }
}
