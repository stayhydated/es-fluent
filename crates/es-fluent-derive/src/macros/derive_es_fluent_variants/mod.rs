use darling::FromDeriveInput as _;
use es_fluent_derive_core::context::{ContainerContext, SpannedNamespaceRule};
use es_fluent_derive_core::error::AttrContext;
use es_fluent_derive_core::lowered::{GeneratedVariantsEnumModel, GeneratedVariantsStructModel};
use es_fluent_derive_core::options::GeneratedVariantsOptions;
use es_fluent_derive_core::options::r#enum::EnumVariantsOpts;
use es_fluent_derive_core::options::label::LabelOpts;
use es_fluent_derive_core::options::r#struct::StructVariantsOpts;
use es_fluent_derive_core::semantic::{
    FluentMessageId, GeneratedVariantMessageSeed, generated_label_message_value,
};
use es_fluent_derive_core::validation;
use es_fluent_shared::{namer, namespace::NamespaceRule};

use heck::ToPascalCase as _;
use proc_macro2::TokenStream;
use quote::quote;
use syn::{Data, DeriveInput, parse_macro_input};

use crate::macros::ir::{GeneratedUnitEnumVariant, MessageEntrySpec};
use crate::macros::utils::{
    CodegenContext, GeneratedUnitEnumInput, NamespaceSource, SpannedNamespaceRuleRef,
};

pub fn from(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let context = CodegenContext::resolve();
    expand_es_fluent_variants_with_context(input, &context).into()
}

#[cfg(test)]
fn expand_es_fluent_variants(input: DeriveInput) -> TokenStream {
    let context = CodegenContext::fallback();
    expand_es_fluent_variants_with_context(input, &context)
}

fn expand_es_fluent_variants_with_context(
    input: DeriveInput,
    context: &CodegenContext,
) -> TokenStream {
    if matches!(&input.data, Data::Union(_)) {
        return syn::Error::new(
            input.ident.span(),
            "EsFluentVariants can only be derived for structs and enums",
        )
        .to_compile_error();
    }

    if let Err(err) = validation::validate_es_fluent_variants_attribute_context(&input) {
        return crate::macros::utils::core_error_to_compile_error(err);
    }

    let label_opts = match LabelOpts::from_derive_input(&input) {
        Ok(opts) => opts,
        Err(err) => return err.write_errors(),
    };
    let container_context = match ContainerContext::from_derive_input(&input) {
        Ok(context) => context,
        Err(err) => return err.write_errors(),
    };

    match &input.data {
        Data::Struct(_) => {
            let opts = match StructVariantsOpts::from_derive_input(&input) {
                Ok(opts) => opts,
                Err(err) => return err.write_errors(),
            };

            process_struct(context, &container_context, &opts, Some(&label_opts))
        },
        Data::Enum(_) => {
            let opts = match EnumVariantsOpts::from_derive_input(&input) {
                Ok(opts) => opts,
                Err(err) => return err.write_errors(),
            };

            process_enum(context, &container_context, &opts, Some(&label_opts))
        },
        Data::Union(_) => syn::Error::new(
            input.ident.span(),
            "EsFluentVariants can only be derived for structs and enums",
        )
        .to_compile_error(),
    }
}

fn validate_namespace(
    namespace: Option<&NamespaceRule>,
    span: proc_macro2::Span,
) -> es_fluent_derive_core::error::EsFluentCoreResult<()> {
    if let Some(ns) = namespace
        && let Err(error) = validation::validate_namespace(ns, Some(span))
    {
        return Err(error);
    }

    Ok(())
}

fn resolved_variants_namespace<'a>(
    opts: &'a impl GeneratedVariantsOptions,
    label_opts: Option<&'a LabelOpts>,
    fluent_namespace: Option<SpannedNamespaceRuleRef<'a>>,
) -> es_fluent_derive_core::error::EsFluentCoreResult<Option<SpannedNamespaceRuleRef<'a>>> {
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

    crate::macros::utils::resolve_single_namespace_source([
        NamespaceSource::new(
            "#[fluent(namespace = ...)]",
            AttrContext::MessageContainer,
            fluent_namespace,
        ),
        NamespaceSource::new(
            "#[fluent_variants(namespace = ...)]",
            AttrContext::VariantsContainer,
            variants_namespace,
        ),
        NamespaceSource::new(
            "#[fluent_label(namespace = ...)]",
            AttrContext::LabelContainer,
            label_namespace,
        ),
    ])
}

pub fn process_struct(
    context: &CodegenContext,
    container_context: &ContainerContext,
    opts: &StructVariantsOpts,
    label_opts: Option<&LabelOpts>,
) -> TokenStream {
    let model = match GeneratedVariantsStructModel::from_options(opts) {
        Ok(model) => model,
        Err(error) => return crate::macros::utils::core_error_to_compile_error(error),
    };
    if let Err(error) = validation::validate_generated_variants_struct_model(&model) {
        return crate::macros::utils::core_error_to_compile_error(error);
    }
    let variant_seeds = match build_struct_variant_seeds(&model) {
        Ok(seeds) => seeds,
        Err(error) => return crate::macros::utils::core_error_to_compile_error(error),
    };
    emit_variants_output(context, container_context, opts, &variant_seeds, label_opts)
}

fn materialize_generated_variant(
    seed: &GeneratedVariantMessageSeed,
    base_key: &namer::FluentKey,
) -> es_fluent_derive_core::error::EsFluentCoreResult<GeneratedUnitEnumVariant> {
    let message = seed.materialize_message(base_key, AttrContext::VariantsContainer)?;

    Ok(GeneratedUnitEnumVariant {
        ident: seed.ident().clone(),
        doc_name: seed.doc_name().to_string(),
        message_entry: MessageEntrySpec::from_metadata(message, Vec::new()),
    })
}

fn variants_label_key(
    label_opts: Option<&LabelOpts>,
    base_key: &namer::FluentKey,
    span: proc_macro2::Span,
) -> es_fluent_derive_core::error::EsFluentCoreResult<Option<FluentMessageId>> {
    label_opts
        .filter(|opts| opts.attr_args().is_variants())
        .map(|_| generated_label_message_value(base_key, span, AttrContext::VariantsContainer))
        .transpose()
}

fn emit_variants_output(
    context: &CodegenContext,
    container_context: &ContainerContext,
    opts: &impl GeneratedVariantsOptions,
    variant_seeds: &[GeneratedVariantMessageSeed],
    label_opts: Option<&LabelOpts>,
) -> TokenStream {
    if variant_seeds.is_empty() {
        return quote! {};
    }

    let targets = crate::macros::utils::generated_variants_enum_targets(opts);
    let derives: Vec<syn::Path> = (*opts.variants_attr_args().derive()).to_vec();
    let namespace = match resolved_variants_namespace(
        opts,
        label_opts,
        container_context
            .fluent_namespace()
            .map(SpannedNamespaceRule::as_ref),
    ) {
        Ok(namespace) => namespace,
        Err(error) => return crate::macros::utils::core_error_to_compile_error(error),
    };
    let origin_ident = opts.variants_ident();
    if let Err(error) = validate_namespace(
        namespace.map(SpannedNamespaceRuleRef::rule),
        namespace
            .map(SpannedNamespaceRuleRef::span)
            .unwrap_or_else(|| origin_ident.span()),
    ) {
        return crate::macros::utils::core_error_to_compile_error(error);
    }
    let items = targets.iter().map(|target| {
        let ident = &target.ident;
        let key_name = target.key_name.map(|key| key.as_str());
        let base_key = namer::FluentKey::from(ident);
        let variant_entries = variant_seeds
            .iter()
            .map(|seed| materialize_generated_variant(seed, &base_key))
            .collect::<es_fluent_derive_core::error::EsFluentCoreResult<Vec<_>>>();
        let variant_entries = match variant_entries {
            Ok(entries) => entries,
            Err(error) => return crate::macros::utils::core_error_to_compile_error(error),
        };

        let label_key = match variants_label_key(label_opts, &base_key, origin_ident.span()) {
            Ok(label_key) => label_key,
            Err(error) => return crate::macros::utils::core_error_to_compile_error(error),
        };

        crate::macros::utils::emit_generated_unit_enum(
            context,
            GeneratedUnitEnumInput {
                ident,
                origin_ident,
                key_name,
                domain_override: container_context.fluent_domain(),
                derives: &derives,
                variants: &variant_entries,
                namespace: namespace.map(SpannedNamespaceRuleRef::rule),
                label_key,
            },
        )
    });

    quote! {
        #(#items)*
    }
}

fn build_struct_variant_seeds(
    model: &GeneratedVariantsStructModel<'_>,
) -> es_fluent_derive_core::error::EsFluentCoreResult<Vec<GeneratedVariantMessageSeed>> {
    model
        .fields()
        .iter()
        .map(|field| {
            let field_ident = field.ident;
            let original_field_name = namer::rust_ident_name(field_ident);
            let pascal_case_name = original_field_name.to_pascal_case();
            let variant_ident = syn::Ident::new(&pascal_case_name, field_ident.span());
            GeneratedVariantMessageSeed::new(
                variant_ident,
                original_field_name,
                namer::rust_ident_name(field_ident),
                field_ident.span(),
                AttrContext::VariantsField,
            )
        })
        .collect()
}

pub fn process_enum(
    context: &CodegenContext,
    container_context: &ContainerContext,
    opts: &EnumVariantsOpts,
    label_opts: Option<&LabelOpts>,
) -> TokenStream {
    let model = match GeneratedVariantsEnumModel::from_options(opts) {
        Ok(model) => model,
        Err(error) => return crate::macros::utils::core_error_to_compile_error(error),
    };
    if let Err(error) = validation::validate_generated_variants_enum_model(&model) {
        return crate::macros::utils::core_error_to_compile_error(error);
    }
    let variant_seeds = match build_enum_variant_seeds(&model) {
        Ok(seeds) => seeds,
        Err(error) => return crate::macros::utils::core_error_to_compile_error(error),
    };
    emit_variants_output(context, container_context, opts, &variant_seeds, label_opts)
}

fn build_enum_variant_seeds(
    model: &GeneratedVariantsEnumModel<'_>,
) -> es_fluent_derive_core::error::EsFluentCoreResult<Vec<GeneratedVariantMessageSeed>> {
    model
        .variants()
        .iter()
        .map(|variant| {
            let variant_ident = variant.ident;
            let variant_key = namer::rust_ident_name(variant_ident);
            GeneratedVariantMessageSeed::new(
                variant_ident.clone(),
                variant_key.clone(),
                variant_key,
                variant_ident.span(),
                AttrContext::VariantsVariant,
            )
        })
        .collect()
}

#[cfg(all(test, target_os = "linux"))]
mod tests {
    use crate::macros::ir::inventory_variant_tokens_for_model;
    use crate::macros::utils::CodegenContext;
    use darling::FromDeriveInput as _;
    use es_fluent_derive_core::context::ContainerContext;
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
        let context = CodegenContext::fallback();
        let container_context = ContainerContext::from_derive_input(&input).expect("context");

        let tokens = crate::snapshot_support::pretty_file_tokens(super::process_struct(
            &context,
            &container_context,
            &opts,
            label_opts.as_ref(),
        ));
        assert_snapshot!("process_struct_emits_keyed_generated_enums", tokens);
    }

    #[test]
    fn generated_variant_entry_drives_runtime_and_inventory_metadata() {
        let seed = es_fluent_derive_core::semantic::GeneratedVariantMessageSeed::new(
            syn::parse_quote!(Username),
            "username",
            "username",
            proc_macro2::Span::call_site(),
            es_fluent_derive_core::error::AttrContext::VariantsField,
        )
        .expect("seed");
        let entry = super::materialize_generated_variant(
            &seed,
            &es_fluent_shared::namer::FluentKey::from("login_form_label_variants"),
        )
        .expect("entry");

        assert_eq!(
            entry.message_entry.metadata.message_id().as_str(),
            "login_form_label_variants-username"
        );
        assert_eq!(
            entry.message_entry.metadata.argument_names(),
            Vec::<es_fluent_derive_core::semantic::ArgName>::new()
        );

        let context = CodegenContext::fallback();
        let runtime_tokens = entry.localize_with_match_arm(&context, None).to_string();
        let inventory_tokens =
            inventory_variant_tokens_for_model(&context, &entry.message_entry.metadata).to_string();

        assert!(runtime_tokens.contains("\"login_form_label_variants-username\""));
        assert!(inventory_tokens.contains(
            "StaticFluentEntryId :: new_unchecked (\"login_form_label_variants-username\")"
        ));
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
        let context = CodegenContext::fallback();
        let container_context = ContainerContext::from_derive_input(&input).expect("context");

        let tokens = crate::snapshot_support::pretty_file_tokens(super::process_enum(
            &context,
            &container_context,
            &opts,
            label_opts.as_ref(),
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
        let context = CodegenContext::fallback();
        let container_context = ContainerContext::from_derive_input(&input).expect("context");

        let tokens = crate::snapshot_support::pretty_file_tokens(super::process_enum(
            &context,
            &container_context,
            &opts,
            label_opts.as_ref(),
        ));

        assert!(tokens.contains("StaticFluentDomain"));
        assert!(tokens.contains("\"es-fluent-lang\""));
        assert!(tokens.contains("StaticFluentEntryId"));
        assert!(tokens.contains("\"language_variants-English\""));
        assert!(tokens.contains("\"language_variants-French\""));
        assert!(tokens.contains(
            "::es_fluent::__private::localize_label(\n            localizer,\n            \"es-fluent-lang\",\n            \"language_variants_label\","
        ));
        assert!(!tokens.contains("CARGO_PKG_NAME"));
    }

    #[test]
    fn process_variants_rejects_multiple_namespace_sources() {
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
        let context = CodegenContext::fallback();
        let container_context = ContainerContext::from_derive_input(&input).expect("context");

        let tokens = crate::snapshot_support::pretty_file_tokens(super::process_struct(
            &context,
            &container_context,
            &opts,
            label_opts.as_ref(),
        ));
        assert!(tokens.contains("compile_error"));
        assert!(tokens.contains("conflicting namespace declarations"));
        assert!(tokens.contains("#[fluent(namespace = ...)]"));
        assert!(tokens.contains("#[fluent_variants(namespace = ...)]"));
    }
}
