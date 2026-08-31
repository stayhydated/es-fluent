use super::{
    EsFluentChoiceExpansion, EsFluentExpansion, EsFluentLabelExpansion, EsFluentMessageVariant,
    EsFluentVariantsExpansion, ExpansionError, ValidatedDeriveInput,
};
use crate::expansion::DeriveFamily;
use es_fluent_shared::namespace::NamespaceRule;
use syn::parse_quote;

fn with_i18n_domains<T>(domains: &[&str], f: impl FnOnce() -> T) -> T {
    let temp_dir = tempfile::TempDir::new().expect("create temporary manifest directory");
    let domains = domains
        .iter()
        .map(|domain| format!("\"{domain}\""))
        .collect::<Vec<_>>()
        .join(", ");
    std::fs::write(
        temp_dir.path().join("i18n.toml"),
        format!(
            "fallback_language = \"en-US\"\nassets_dir = \"i18n\"\ndomains = [{domains}]\nnamespaces = [\"errors\", \"languages\"]\n"
        ),
    )
    .expect("write i18n.toml");

    temp_env::with_var("CARGO_MANIFEST_DIR", Some(temp_dir.path()), f)
}

#[test]
#[serial_test::serial(manifest)]
fn validated_input_covers_es_fluent_boundary() {
    let input: syn::DeriveInput = parse_quote! {
        #[fluent(namespace = "forms")]
        struct Login {
            name: String,
        }
    };

    let validated = ValidatedDeriveInput::for_es_fluent(&input).expect("input should validate");

    assert_eq!(validated.family(), DeriveFamily::EsFluent);
    assert_eq!(validated.input().ident, "Login");
    assert!(validated.envelope().is_none());
}

#[test]
#[serial_test::serial(manifest)]
fn validated_input_captures_label_parent_context() {
    let input: syn::DeriveInput = parse_quote! {
        #[fluent(domain = "shared", namespace = "labels")]
        enum Status {
            Active,
        }
    };

    let validated =
        ValidatedDeriveInput::for_es_fluent_label(&input).expect("input should validate");
    let envelope = validated.envelope().expect("label captures envelope");

    assert_eq!(validated.family(), DeriveFamily::EsFluentLabel);
    assert_eq!(envelope.source_ident(), "Status");
    assert_eq!(envelope.fluent_domain().expect("domain").as_str(), "shared");
    assert!(matches!(
        envelope.fluent_namespace().map(|namespace| namespace.rule()),
        Some(NamespaceRule::Literal(value)) if value == "labels"
    ));
}

#[test]
#[serial_test::serial(manifest)]
fn validated_input_captures_variants_parent_context() {
    let input: syn::DeriveInput = parse_quote! {
        #[fluent(namespace = "forms")]
        #[fluent_variants(keys = ["label"])]
        struct Profile {
            name: String,
        }
    };

    let validated =
        ValidatedDeriveInput::for_es_fluent_variants(&input).expect("input should validate");
    let envelope = validated.envelope().expect("variants captures envelope");

    assert_eq!(validated.family(), DeriveFamily::EsFluentVariants);
    assert_eq!(envelope.source_ident(), "Profile");
    assert!(matches!(
        envelope.fluent_namespace().map(|namespace| namespace.rule()),
        Some(NamespaceRule::Literal(value)) if value == "forms"
    ));
}

#[test]
#[serial_test::serial(manifest)]
fn validated_input_covers_choice_boundary() {
    let input: syn::DeriveInput = parse_quote! {
        enum Priority {
            VeryHigh,
        }
    };

    let validated =
        ValidatedDeriveInput::for_es_fluent_choice(&input).expect("input should validate");

    assert_eq!(validated.family(), DeriveFamily::EsFluentChoice);
    assert_eq!(validated.input().ident, "Priority");
    assert!(validated.envelope().is_none());
}

#[test]
#[serial_test::serial(manifest)]
fn choice_expansion_builds_validated_choice_model() {
    let input: syn::DeriveInput = parse_quote! {
        enum Priority<T>
        where
            T: Clone,
        {
            VeryHigh,
            Low,
        }
    };

    let expansion =
        EsFluentChoiceExpansion::from_derive_input(&input).expect("choice expansion should build");

    assert_eq!(expansion.ident().to_string(), "Priority");
    assert_eq!(
        expansion
            .generics()
            .type_params()
            .map(|param| param.ident.to_string())
            .collect::<Vec<_>>(),
        vec!["T"]
    );
    assert_eq!(
        expansion.choice().variants()[0].value().as_str(),
        "very-high"
    );
    assert_eq!(expansion.choice().variants()[1].value().as_str(), "low");
}

#[test]
#[serial_test::serial(manifest)]
fn choice_expansion_reports_darling_shape_errors() {
    let input: syn::DeriveInput = parse_quote! {
        struct NotAnEnum;
    };

    let err =
        EsFluentChoiceExpansion::from_derive_input(&input).expect_err("struct input should fail");

    assert!(matches!(err, ExpansionError::Darling(_)));
}

#[test]
#[serial_test::serial(manifest)]
fn choice_expansion_reports_core_attribute_errors_before_darling() {
    let input: syn::DeriveInput = parse_quote! {
        #[fluent_choice(rename_all = 123)]
        enum BadChoice {
            A,
        }
    };

    let err =
        EsFluentChoiceExpansion::from_derive_input(&input).expect_err("wrong shape should fail");

    assert!(matches!(err, ExpansionError::Core(_)));
}

#[test]
#[serial_test::serial(manifest)]
fn es_fluent_struct_expansion_builds_message_and_inventory_model() {
    let input: syn::DeriveInput = parse_quote! {
        #[fluent(namespace = "forms")]
        struct LoginForm {
            #[fluent(arg = "display_name")]
            name: String,
            attempts: u16,
        }
    };

    let EsFluentExpansion::Struct(expansion) =
        EsFluentExpansion::from_derive_input(&input).expect("struct expansion")
    else {
        panic!("expected struct expansion");
    };

    assert_eq!(expansion.ident().to_string(), "LoginForm");
    assert_eq!(
        expansion.message_entry().message_id().as_str(),
        "login_form"
    );
    assert_eq!(
        expansion
            .message_entry()
            .argument_names()
            .iter()
            .map(crate::semantic::ArgName::as_str)
            .collect::<Vec<_>>(),
        vec!["display_name", "attempts"]
    );
    assert!(matches!(
        expansion.message_model().namespace(),
        Some(NamespaceRule::Literal(value)) if value == "forms"
    ));
}

#[test]
#[serial_test::serial(manifest)]
fn es_fluent_enum_expansion_builds_localized_and_skipped_variants() {
    let input: syn::DeriveInput = parse_quote! {
        #[fluent(domain = "auth", namespace = "errors")]
        enum LoginError {
            Failed(
                #[fluent(arg = "display_name")]
                String,
                u16,
            ),
            #[fluent(skip)]
            Other(String),
        }
    };

    let expansion = with_i18n_domains(&["auth"], || {
        EsFluentExpansion::from_derive_input(&input).expect("enum expansion")
    });
    let EsFluentExpansion::Enum(expansion) = expansion else {
        panic!("expected enum expansion");
    };

    assert_eq!(expansion.ident().to_string(), "LoginError");
    assert_eq!(expansion.domain().expect("domain").as_str(), "auth");
    assert!(matches!(
        expansion.message_model().namespace(),
        Some(NamespaceRule::Literal(value)) if value == "errors"
    ));
    assert_eq!(expansion.variants().len(), 2);
    let EsFluentMessageVariant::Localized(localized) = &expansion.variants()[0] else {
        panic!("first variant should localize");
    };
    assert_eq!(
        localized.message_entry().message_id().as_str(),
        "login_error-Failed"
    );
    assert_eq!(
        localized
            .message_entry()
            .argument_names()
            .iter()
            .map(crate::semantic::ArgName::as_str)
            .collect::<Vec<_>>(),
        vec!["display_name", "f1"]
    );
    assert!(matches!(
        &expansion.variants()[1],
        EsFluentMessageVariant::Skipped(skipped) if skipped.ident() == "Other"
    ));
    assert_eq!(expansion.message_model().messages().len(), 1);
}

#[test]
#[serial_test::serial(manifest)]
fn label_expansion_builds_label_impl_and_inventory_model() {
    let input: syn::DeriveInput = parse_quote! {
        #[fluent(namespace = "ui")]
        struct LoginForm<T>(T);
    };

    let expansion =
        EsFluentLabelExpansion::from_derive_input(&input).expect("label expansion should build");
    let inventory = expansion.label_inventory();

    assert_eq!(expansion.ident().to_string(), "LoginForm");
    assert_eq!(expansion.ftl_key().as_str(), "login_form_label");
    assert_eq!(
        expansion
            .generics()
            .type_params()
            .map(|param| param.ident.to_string())
            .collect::<Vec<_>>(),
        vec!["T"]
    );
    assert!(matches!(
        inventory.namespace(),
        Some(NamespaceRule::Literal(value)) if value == "ui"
    ));
    assert_eq!(
        inventory
            .label()
            .expect("label entry")
            .message_id()
            .as_str(),
        "login_form_label"
    );
}

#[test]
#[serial_test::serial(manifest)]
fn explicit_domain_is_retained_for_generated_ftl() {
    let input: syn::DeriveInput = parse_quote! {
        #[fluent(domain = "ui")]
        enum UiMessage {
            Ready,
        }
    };
    let expansion = with_i18n_domains(&["ui"], || {
        EsFluentExpansion::from_derive_input(&input).expect("explicit domain should compile")
    });
    let EsFluentExpansion::Enum(expansion) = expansion else {
        panic!("expected enum expansion");
    };

    assert_eq!(expansion.domain().expect("domain").as_str(), "ui");
    assert_eq!(
        expansion
            .message_model()
            .domain()
            .expect("inventory domain")
            .as_str(),
        "ui"
    );
}

#[test]
#[serial_test::serial(manifest)]
fn label_expansion_accepts_no_label_attribute() {
    let input: syn::DeriveInput = parse_quote! {
        enum LabelOnly {
            A,
        }
    };

    let expansion = EsFluentLabelExpansion::from_derive_input(&input)
        .expect("label expansion should infer the type label");
    let inventory = expansion.label_inventory();

    assert!(inventory.label().is_some());
}

#[test]
#[serial_test::serial(manifest)]
fn label_expansion_rejects_legacy_origin_flag() {
    let input: syn::DeriveInput = parse_quote! {
        #[fluent_label(origin)]
        enum NoOrigin {
            A,
        }
    };

    let err =
        EsFluentLabelExpansion::from_derive_input(&input).expect_err("legacy origin should fail");

    assert!(matches!(err, ExpansionError::Core(_)));
    assert!(err.to_string().contains("#[fluent_label(origin)]"));
    assert!(err.to_string().contains("accepted key here is namespace"));
}

#[test]
#[serial_test::serial(manifest)]
fn label_expansion_rejects_conflicting_namespace_sources() {
    let input: syn::DeriveInput = parse_quote! {
        #[fluent(namespace = "parent")]
        #[fluent_label(namespace = "child")]
        struct NamespacedLabel;
    };

    let err = EsFluentLabelExpansion::from_derive_input(&input)
        .expect_err("conflicting namespaces should fail");

    assert!(matches!(err, ExpansionError::Core(_)));
    assert!(
        err.to_string()
            .contains("conflicting namespace declarations")
    );
}

#[test]
#[serial_test::serial(manifest)]
fn variants_expansion_builds_keyed_struct_targets() {
    let input: syn::DeriveInput = parse_quote! {
        #[fluent(namespace = "ui")]
        #[fluent_variants(keys = ["label", "placeholder"], derive(Debug))]
        struct LoginForm {
            username: String,
            #[fluent_variants(skip)]
            ignored: String,
        }
    };

    let expansion = EsFluentVariantsExpansion::from_derive_input(&input)
        .expect("variants expansion should build");

    assert_eq!(expansion.origin_ident().to_string(), "LoginForm");
    assert!(matches!(
        expansion.namespace(),
        Some(NamespaceRule::Literal(value)) if value == "ui"
    ));
    assert_eq!(expansion.targets().len(), 2);
    assert_eq!(
        expansion.targets()[0].ident().to_string(),
        "LoginFormLabelVariants"
    );
    assert_eq!(
        expansion.targets()[0]
            .key_name()
            .expect("key name")
            .as_str(),
        "label"
    );
    assert_eq!(
        expansion.targets()[0]
            .generated_model()
            .derives()
            .token_strings(),
        vec!["Clone", "Copy", "Debug", "Eq", "Hash", "PartialEq"]
    );
    assert_eq!(expansion.targets()[0].variants().len(), 1);
    assert_eq!(
        expansion.targets()[0].variants()[0]
            .message_entry()
            .message_id()
            .as_str(),
        "login_form_label_variants-username"
    );
}

#[test]
#[serial_test::serial(manifest)]
fn variants_expansion_rejects_explicit_keys_with_no_unskipped_targets() {
    let input: syn::DeriveInput = parse_quote! {
        #[fluent_variants(keys = ["label"])]
        struct LoginForm {
            #[fluent_variants(skip)]
            ignored: String,
        }
    };

    let err = EsFluentVariantsExpansion::from_derive_input(&input)
        .expect_err("explicit keys with no generated members should fail");

    assert!(matches!(err, ExpansionError::Core(_)));
    assert!(err.to_string().contains("fluent_variants(keys = ...)"));
    assert!(err.to_string().contains("at least one unskipped"));
}

#[test]
#[serial_test::serial(manifest)]
fn variants_expansion_builds_enum_label_key_and_domain() {
    let input: syn::DeriveInput = parse_quote! {
        #[fluent(
            domain = "es-fluent-lang",
            namespace = "languages"
        )]
        enum Language {
            English,
            French,
        }
    };

    let expansion = with_i18n_domains(&["es-fluent-lang"], || {
        EsFluentVariantsExpansion::from_derive_input(&input)
            .expect("variants expansion should build")
    });
    let target = expansion.targets().first().expect("target");

    assert_eq!(
        expansion.domain().expect("domain").as_str(),
        "es-fluent-lang"
    );
    assert_eq!(target.label_key().as_str(), "language_variants_label");
    assert_eq!(
        target.variants()[0].message_entry().message_id().as_str(),
        "language_variants-English"
    );
    assert_eq!(
        target.variants()[1].message_entry().message_id().as_str(),
        "language_variants-French"
    );
}

#[test]
#[serial_test::serial(manifest)]
fn variants_expansion_generates_label_key_without_label_derive() {
    let input: syn::DeriveInput = parse_quote! {
        struct LoginForm {
            username: String,
        }
    };

    let expansion = EsFluentVariantsExpansion::from_derive_input(&input)
        .expect("variants expansion should infer label output");
    let target = expansion.targets().first().expect("target");

    assert_eq!(target.label_key().as_str(), "login_form_variants_label");
}

#[test]
#[serial_test::serial(manifest)]
fn variants_expansion_rejects_conflicting_namespace_sources() {
    let input: syn::DeriveInput = parse_quote! {
        #[fluent(namespace = "parent_ns")]
        #[fluent_variants(namespace = "variant_ns")]
        #[fluent_label(namespace = "label_ns")]
        struct NamespaceHolder {
            field: String,
        }
    };

    let err = EsFluentVariantsExpansion::from_derive_input(&input)
        .expect_err("conflicting namespaces should fail");

    assert!(matches!(err, ExpansionError::Core(_)));
    assert!(
        err.to_string()
            .contains("conflicting namespace declarations")
    );
}
