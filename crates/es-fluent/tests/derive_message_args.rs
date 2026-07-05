#![cfg(feature = "derive")]

use es_fluent::registry::{StaticFluentDomain, StaticFluentEntryId};
use es_fluent::{
    EsFluent, EsFluentChoice as _, EsFluentVariants, FluentArgs, FluentMessage, FluentValue,
};
use std::collections::HashMap;

#[derive(EsFluent)]
struct DerivedBoolStruct {
    enabled: bool,
    maybe_enabled: Option<bool>,
}

#[derive(EsFluent)]
struct DerivedBorrowedBoolStruct<'a> {
    enabled: &'a bool,
    maybe_enabled: Option<&'a bool>,
}

#[derive(EsFluent)]
#[allow(dead_code)]
enum DerivedBoolEnum {
    Named {
        enabled: bool,
        maybe_enabled: Option<bool>,
    },
    Tuple(bool, Option<bool>),
}

#[derive(EsFluent)]
#[allow(dead_code)]
enum DerivedBorrowedBoolEnum<'a> {
    Named {
        enabled: &'a bool,
        maybe_enabled: Option<&'a bool>,
    },
    Tuple(&'a bool, Option<&'a bool>),
}

#[derive(EsFluent)]
enum DerivedTone {
    VeryFriendly,
    #[fluent(key = "serious")]
    Serious,
}

#[derive(EsFluent)]
struct OptionalSelector {
    #[fluent(selector)]
    tone: Option<DerivedTone>,
}

#[derive(EsFluentVariants)]
#[allow(dead_code)]
struct GeneratedChoiceForm {
    username: String,
    password: String,
}

#[derive(EsFluent)]
struct GeneratedVariantSelector {
    #[fluent(selector)]
    field: GeneratedChoiceFormVariants,
}

fn describe_arg(value: &FluentValue<'_>) -> String {
    match value {
        FluentValue::String(value) => value.as_ref().to_string(),
        FluentValue::None => "<none>".to_string(),
        other => format!("{other:?}"),
    }
}

fn render_args(message: &impl FluentMessage) -> HashMap<String, String> {
    let mut rendered = HashMap::new();
    {
        let mut localize = |_domain: StaticFluentDomain,
                            _id: StaticFluentEntryId,
                            args: Option<&FluentArgs<'_>>| {
            if let Some(args) = args {
                for (name, value) in args.as_raw() {
                    rendered.insert((*name).to_string(), describe_arg(value));
                }
            }

            "rendered".to_string()
        };

        message.to_fluent_string_with(&mut localize);
    }

    rendered
}

#[test]
fn derived_struct_bool_and_optional_bool_fields_compile_and_render() {
    let args = render_args(&DerivedBoolStruct {
        enabled: true,
        maybe_enabled: Some(false),
    });

    assert_eq!(args["enabled"], "true");
    assert_eq!(args["maybe_enabled"], "false");

    let missing = render_args(&DerivedBoolStruct {
        enabled: false,
        maybe_enabled: None,
    });

    assert_eq!(missing["enabled"], "false");
    assert_eq!(missing["maybe_enabled"], "<none>");
}

#[test]
fn derived_struct_borrowed_bool_and_optional_borrowed_bool_fields_compile_and_render() {
    let enabled = true;
    let maybe_enabled = false;
    let args = render_args(&DerivedBorrowedBoolStruct {
        enabled: &enabled,
        maybe_enabled: Some(&maybe_enabled),
    });

    assert_eq!(args["enabled"], "true");
    assert_eq!(args["maybe_enabled"], "false");

    let disabled = false;
    let missing = render_args(&DerivedBorrowedBoolStruct {
        enabled: &disabled,
        maybe_enabled: None,
    });

    assert_eq!(missing["enabled"], "false");
    assert_eq!(missing["maybe_enabled"], "<none>");
}

#[test]
fn derived_enum_named_bool_and_optional_bool_fields_compile_and_render() {
    let args = render_args(&DerivedBoolEnum::Named {
        enabled: true,
        maybe_enabled: Some(false),
    });

    assert_eq!(args["enabled"], "true");
    assert_eq!(args["maybe_enabled"], "false");
}

#[test]
fn derived_enum_named_borrowed_bool_and_optional_borrowed_bool_fields_compile_and_render() {
    let enabled = true;
    let maybe_enabled = false;
    let args = render_args(&DerivedBorrowedBoolEnum::Named {
        enabled: &enabled,
        maybe_enabled: Some(&maybe_enabled),
    });

    assert_eq!(args["enabled"], "true");
    assert_eq!(args["maybe_enabled"], "false");
}

#[test]
fn derived_enum_tuple_bool_and_optional_bool_fields_compile_and_render() {
    let args = render_args(&DerivedBoolEnum::Tuple(true, Some(false)));

    assert!(args.values().any(|value| value == "true"));
    assert!(args.values().any(|value| value == "false"));

    let missing = render_args(&DerivedBoolEnum::Tuple(false, None));

    assert!(missing.values().any(|value| value == "false"));
    assert!(missing.values().any(|value| value == "<none>"));
}

#[test]
fn derived_enum_tuple_borrowed_bool_and_optional_borrowed_bool_fields_compile_and_render() {
    let enabled = true;
    let maybe_enabled = false;
    let args = render_args(&DerivedBorrowedBoolEnum::Tuple(
        &enabled,
        Some(&maybe_enabled),
    ));

    assert!(args.values().any(|value| value == "true"));
    assert!(args.values().any(|value| value == "false"));

    let disabled = false;
    let missing = render_args(&DerivedBorrowedBoolEnum::Tuple(&disabled, None));

    assert!(missing.values().any(|value| value == "false"));
    assert!(missing.values().any(|value| value == "<none>"));
}

#[test]
fn es_fluent_unit_enum_infers_choice_and_optional_selector_renders() {
    assert_eq!(
        DerivedTone::VeryFriendly.as_fluent_choice().as_str(),
        "very-friendly"
    );
    assert_eq!(DerivedTone::Serious.as_fluent_choice().as_str(), "serious");

    let args = render_args(&OptionalSelector {
        tone: Some(DerivedTone::VeryFriendly),
    });
    assert_eq!(args["tone"], "very-friendly");

    let missing = render_args(&OptionalSelector { tone: None });
    assert_eq!(missing["tone"], "<none>");
}

#[test]
fn es_fluent_variants_generated_enums_infer_choice_and_render_as_selectors() {
    assert_eq!(
        GeneratedChoiceFormVariants::Username
            .as_fluent_choice()
            .as_str(),
        "username"
    );
    assert_eq!(
        GeneratedChoiceFormVariants::Password
            .as_fluent_choice()
            .as_str(),
        "password"
    );

    let args = render_args(&GeneratedVariantSelector {
        field: GeneratedChoiceFormVariants::Username,
    });
    assert_eq!(args["field"], "username");
}
