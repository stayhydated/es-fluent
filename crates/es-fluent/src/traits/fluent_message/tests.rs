use super::*;

fn panic_lookup<'a>(
    _domain: &str,
    _id: &str,
    _args: Option<&HashMap<&str, FluentValue<'a>>>,
) -> String {
    panic!("ordinary arguments should not invoke nested localization")
}

fn assert_string(value: FluentValue<'_>, expected: &str) {
    match value {
        FluentValue::String(value) => assert_eq!(value.as_ref(), expected),
        other => panic!("expected string FluentValue, got {other:?}"),
    }
}

fn assert_number(value: FluentValue<'_>, expected: f64) {
    match value {
        FluentValue::Number(value) => assert_eq!(value.value, expected),
        other => panic!("expected number FluentValue, got {other:?}"),
    }
}

#[test]
fn argument_conversion_handles_primitive_values() {
    let mut localize = panic_lookup;

    let string_value =
        FluentArgumentValue::new("borrowed").into_fluent_argument_value(&mut localize);
    assert_string(string_value, "borrowed");

    let number_value = FluentArgumentValue::new(42i32).into_fluent_argument_value(&mut localize);
    assert_number(number_value, 42.0);

    let bool_value = FluentArgumentValue::new(true).into_fluent_argument_value(&mut localize);
    assert_string(bool_value, "true");

    let false_value = FluentArgumentValue::new(false).into_fluent_argument_value(&mut localize);
    assert_string(false_value, "false");
}

#[test]
#[should_panic(expected = "ordinary arguments should not invoke nested localization")]
fn panic_lookup_reports_unexpected_nested_localization() {
    let _ = panic_lookup("domain", "id", None);
}

#[test]
fn argument_conversion_handles_optional_and_missing_values() {
    let mut localize = panic_lookup;

    let optional_value =
        FluentArgumentValue::new(Some("optional")).into_fluent_argument_value(&mut localize);
    assert_string(optional_value, "optional");

    let missing_value =
        FluentArgumentValue::new(Option::<String>::None).into_fluent_argument_value(&mut localize);
    assert!(matches!(missing_value, FluentValue::None));

    let optional_number =
        FluentArgumentValue::new(Some(7i32)).into_fluent_argument_value(&mut localize);
    assert_number(optional_number, 7.0);
}

#[test]
fn argument_conversion_handles_borrowed_and_owned_values() {
    let mut localize = panic_lookup;
    let borrowed = String::from("borrowed string");

    let borrowed_value =
        FluentArgumentValue::new(&borrowed).into_fluent_argument_value(&mut localize);
    assert_string(borrowed_value, "borrowed string");

    let owned_value = FluentArgumentValue::new(String::from("owned string"))
        .into_fluent_argument_value(&mut localize);
    assert_string(owned_value, "owned string");
}

#[derive(Clone)]
struct NestedMessage;

impl FluentMessage for NestedMessage {
    fn to_fluent_string_with(
        &self,
        localize: &mut dyn for<'a> FnMut(
            &str,
            &str,
            Option<&HashMap<&str, FluentValue<'a>>>,
        ) -> String,
    ) -> String {
        localize("nested-domain", "nested-id", None)
    }
}

#[test]
fn argument_conversion_localizes_nested_messages_with_current_callback() {
    let mut localize = |domain: &str, id: &str, args: Option<&HashMap<&str, FluentValue<'_>>>| {
        assert_eq!(domain, "nested-domain");
        assert_eq!(id, "nested-id");
        assert!(args.is_none());
        "nested value".to_string()
    };

    let value = FluentArgumentValue::new(NestedMessage).into_fluent_argument_value(&mut localize);
    assert_string(value, "nested value");
}

struct StaticLocalizer {
    value: &'static str,
}

impl FluentLocalizer for StaticLocalizer {
    fn localize<'a>(
        &self,
        id: &str,
        _args: Option<&HashMap<&str, FluentValue<'a>>>,
    ) -> Option<String> {
        if id == "nested-id" {
            Some(self.value.to_string())
        } else {
            None
        }
    }

    fn localize_in_domain<'a>(
        &self,
        domain: &str,
        id: &str,
        args: Option<&HashMap<&str, FluentValue<'a>>>,
    ) -> Option<String> {
        if domain == "nested-domain" {
            self.localize(id, args)
        } else {
            None
        }
    }
}

#[test]
fn localize_message_uses_the_explicit_localizer() {
    let en = StaticLocalizer { value: "Hello" };
    let fr = StaticLocalizer { value: "Bonjour" };

    assert_eq!(en.localize_message(&NestedMessage), "Hello");
    assert_eq!(fr.localize_message(&NestedMessage), "Bonjour");
    assert_eq!(en.localize_message(&NestedMessage), "Hello");
}

struct MissingMessage;

impl FluentMessage for MissingMessage {
    fn to_fluent_string_with(
        &self,
        localize: &mut dyn for<'a> FnMut(
            &str,
            &str,
            Option<&HashMap<&str, FluentValue<'a>>>,
        ) -> String,
    ) -> String {
        localize("missing-domain", "missing-id", None)
    }
}

#[test]
fn fluent_message_reference_impl_delegates_to_inner_message() {
    let message = NestedMessage;
    let message_ref = &message;
    let mut localize = |domain: &str, id: &str, _args: Option<&HashMap<&str, FluentValue<'_>>>| {
        format!("{domain}:{id}")
    };

    assert_eq!(
        FluentMessage::to_fluent_string_with(&message_ref, &mut localize),
        "nested-domain:nested-id"
    );
}

#[test]
fn fluent_localizer_reference_and_arc_impls_delegate_to_inner_localizer() {
    let localizer = StaticLocalizer { value: "Hello" };
    let localizer_ref = &localizer;
    let localizer_arc = Arc::new(StaticLocalizer { value: "Bonjour" });

    assert_eq!(localizer_ref.localize_message(&NestedMessage), "Hello");
    assert_eq!(localizer_arc.localize_message(&NestedMessage), "Bonjour");
    assert_eq!(
        FluentLocalizer::localize(&localizer_ref, "nested-id", None),
        Some("Hello".to_string())
    );
    assert_eq!(
        FluentLocalizer::localize_in_domain(&localizer_ref, "nested-domain", "nested-id", None,),
        Some("Hello".to_string())
    );
    assert_eq!(
        FluentLocalizer::localize_in_domain(&localizer_arc, "nested-domain", "nested-id", None,),
        Some("Bonjour".to_string())
    );
}

#[test]
fn localizer_extension_localizes_typed_messages_with_id_fallback() {
    let localizer = StaticLocalizer { value: "Hello" };

    assert_eq!(
        FluentLocalizer::localize(&localizer, "nested-id", None),
        Some("Hello".to_string())
    );
    assert_eq!(
        FluentLocalizer::localize_in_domain(&localizer, "nested-domain", "nested-id", None),
        Some("Hello".to_string())
    );
    assert_eq!(localizer.localize_message(&MissingMessage), "missing-id");
}
