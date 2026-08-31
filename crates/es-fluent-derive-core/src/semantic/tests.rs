use super::*;

#[test]
fn typed_names_accept_current_generated_shapes() {
    let span = Span::call_site();

    assert_eq!(
        parse_fluent_message_id_in_context("status-Ready", span, AttrContext::MessageContainer)
            .expect("message id")
            .as_str(),
        "status-Ready"
    );
    assert_eq!(
        parse_arg_name("display_name", span)
            .expect("argument")
            .as_str(),
        "display_name"
    );
    assert_eq!(
        parse_variant_key_in_context("custom-key", span, AttrContext::EnumVariant)
            .expect("variant key")
            .as_str(),
        "custom-key"
    );
    assert_eq!(
        parse_domain_name_in_context("es-fluent-lang", span, AttrContext::MessageContainer)
            .expect("domain")
            .as_str(),
        "es-fluent-lang"
    );
}

#[test]
fn typed_names_reject_empty_leading_digit_and_invalid_characters() {
    let span = Span::call_site();

    let err = parse_arg_name_in_context("", span, AttrContext::MessageField)
        .expect_err("empty arg should fail");
    assert_eq!(
        err.to_string(),
        "Attribute error in message field: Fluent argument name must not be empty"
    );

    assert!(parse_arg_name("1value", span).is_err());
    assert!(parse_arg_name("display name", span).is_err());
    assert!(
        parse_fluent_message_id_in_context("_message", span, AttrContext::MessageContainer)
            .is_err()
    );
}

#[test]
fn generated_message_id_helpers_return_typed_spanned_values() {
    let span = Span::call_site();
    let login_form: syn::Ident = syn::parse_quote!(LoginForm);
    let login_error: syn::Ident = syn::parse_quote!(LoginError);
    let failed: syn::Ident = syn::parse_quote!(Failed);
    let username: syn::Ident = syn::parse_quote!(Username);

    assert_eq!(
        message_id_for_ident(&login_form, AttrContext::MessageContainer)
            .expect("struct message id")
            .value()
            .as_str(),
        "login_form"
    );
    assert_eq!(
        label_message_id_for_ident(&login_form, AttrContext::LabelContainer)
            .expect("label message id")
            .value()
            .as_str(),
        "login_form_label"
    );

    let base = message_id_for_ident(&login_error, AttrContext::MessageContainer)
        .expect("enum base")
        .into_value();
    assert_eq!(
        variant_message_id(&base, &failed, None, AttrContext::EnumVariant)
            .expect("variant message id")
            .value()
            .as_str(),
        "login_error-Failed"
    );

    let override_key = parse_variant_key_in_context("custom-key", span, AttrContext::EnumVariant)
        .expect("override key");
    assert_eq!(
        variant_message_id(
            &base,
            &failed,
            Some(&override_key),
            AttrContext::EnumVariant
        )
        .expect("overridden variant message id")
        .value()
        .as_str(),
        "login_error-custom-key"
    );

    let generated_base = namer::FluentKey::from("login_form_label_variants");
    assert_eq!(
        generated_variant_message_id(
            &generated_base,
            "username",
            username.span(),
            AttrContext::VariantsContainer,
        )
        .expect("generated variant id")
        .value()
        .as_str(),
        "login_form_label_variants-username"
    );
    assert_eq!(
        generated_label_message_id(&generated_base, span, AttrContext::VariantsContainer,)
            .expect("generated label id")
            .value()
            .as_str(),
        "login_form_label_variants_label"
    );
}

#[test]
fn message_entry_model_returns_inventory_argument_names_from_arguments() {
    let span = Span::call_site();
    let entry = MessageEntryModel::new(
        RustSourceName::new("Ready", span),
        SpannedValue::new(
            parse_fluent_message_id_in_context("status-Ready", span, AttrContext::MessageContainer)
                .expect("message id"),
            span,
        ),
        vec![
            ArgumentModel::new(SpannedValue::new(
                parse_arg_name("first", span).expect("arg"),
                span,
            )),
            ArgumentModel::new_with_value_strategy(
                SpannedValue::new(parse_arg_name("second", span).expect("arg"), span),
                ArgumentValueStrategy::Choice {
                    span,
                    ty: Box::new(syn::parse_quote!(Status)),
                },
            ),
        ],
        SourceLocation::new(span),
    );

    assert_eq!(entry.source_name(), "Ready");
    assert_eq!(entry.message_id().as_str(), "status-Ready");
    let _span = entry.source_location().span();
    assert_eq!(
        entry
            .argument_names()
            .iter()
            .map(ArgName::as_str)
            .collect::<Vec<_>>(),
        vec!["first", "second"]
    );
    assert!(matches!(
        entry.arguments()[1].value_strategy(),
        ArgumentValueStrategy::Choice { .. }
    ));
}

#[test]
fn message_model_groups_entries() {
    let span = Span::call_site();
    let entry = MessageEntryModel::new(
        RustSourceName::new("Ready", span),
        SpannedValue::new(
            parse_fluent_message_id_in_context("status-Ready", span, AttrContext::MessageContainer)
                .expect("message id"),
            span,
        ),
        Vec::new(),
        SourceLocation::new(span),
    );
    let model = MessageModel::new(
        RustTypeName::new("Status", proc_macro2::Span::call_site()),
        TypeKind::Enum,
        None,
        None,
        vec![entry.clone()],
        None,
    );

    assert_eq!(model.source_type(), "Status");
    assert!(matches!(model.type_kind(), TypeKind::Enum));
    assert_eq!(model.messages()[0].message_id().as_str(), "status-Ready");

    let label = MessageEntryModel::new(
        RustSourceName::new("StatusFtl", span),
        SpannedValue::new(
            parse_fluent_message_id_in_context(
                "status_ftl_label",
                span,
                AttrContext::VariantsContainer,
            )
            .expect("label id"),
            span,
        ),
        Vec::new(),
        SourceLocation::new(span),
    );
    let generated = GeneratedEnumModel::new(
        RustTypeName::new("StatusFtl", proc_macro2::Span::call_site()),
        RustTypeName::new("Status", proc_macro2::Span::call_site()),
        DerivePathList::from_paths(
            vec![syn::parse_quote!(Debug)],
            AttrContext::VariantsContainer,
        )
        .expect("derive paths"),
        vec![entry],
        Some(label),
        None,
        None,
    );

    assert_eq!(generated.ident(), "StatusFtl");
    assert_eq!(generated.origin_ident(), "Status");
    assert_eq!(
        generated.derives().token_strings(),
        vec!["Debug".to_string()]
    );
    assert_eq!(generated.messages()[0].source_name(), "Ready");
}

#[test]
fn generated_variant_derives_include_defaults_and_deduplicate_explicit_paths() {
    let derives = DerivePathList::for_generated_variants(
        vec![
            syn::parse_quote!(Debug),
            syn::parse_quote!(::core::clone::Clone),
            syn::parse_quote!(serde::Serialize),
        ],
        AttrContext::VariantsContainer,
    )
    .expect("derive paths");

    assert_eq!(
        derives.token_strings(),
        vec![
            "Clone",
            "Copy",
            "Debug",
            "Eq",
            "Hash",
            "PartialEq",
            "serde :: Serialize",
        ]
    );
}

#[test]
fn generated_variant_derives_reject_manual_es_fluent_choice() {
    let err = DerivePathList::for_generated_variants(
        vec![syn::parse_quote!(es_fluent::EsFluentChoice)],
        AttrContext::VariantsContainer,
    )
    .expect_err("generated variants infer EsFluentChoice");

    assert!(
        err.to_string()
            .contains("generated variant enums implement EsFluentChoice automatically")
    );
    assert!(
        err.to_string()
            .contains("remove EsFluentChoice from #[fluent_variants(derive(...))]")
    );
}

#[test]
fn choice_model_applies_rename_all_once() {
    let choice_ident: syn::Ident = syn::parse_quote!(SeverityChoice);
    let high_ident: syn::Ident = syn::parse_quote!(VeryHigh);
    let low_ident: syn::Ident = syn::parse_quote!(Low);

    let model = ChoiceModel::from_variant_idents(
        &choice_ident,
        [&high_ident, &low_ident],
        Some(CaseStyle::SnakeCase),
    )
    .expect("choice model");

    assert_eq!(model.ident().to_string(), "SeverityChoice");
    assert_eq!(model.variants()[0].ident().to_string(), "VeryHigh");
    assert_eq!(model.variants()[0].value().as_str(), "very_high");
    assert_eq!(model.variants()[1].value().as_str(), "low");
}

#[test]
fn choice_model_defaults_to_kebab_case() {
    let choice_ident: syn::Ident = syn::parse_quote!(SeverityChoice);
    let high_ident: syn::Ident = syn::parse_quote!(VeryHigh);

    let model =
        ChoiceModel::from_variant_idents(&choice_ident, [&high_ident], None).expect("choice model");

    assert_eq!(model.variants()[0].value().as_str(), "very-high");
}

#[test]
fn choice_model_rejects_invalid_generated_selector_values() {
    let choice_ident: syn::Ident = syn::parse_quote!(SeverityChoice);
    let high_ident: syn::Ident = syn::parse_quote!(VeryHigh);

    let err =
        ChoiceModel::from_variant_idents(&choice_ident, [&high_ident], Some(CaseStyle::TitleCase))
            .expect_err("title case generates a selector value containing a space");

    let message = err.to_string();
    assert!(message.contains("choice container"), "{message}");
}
