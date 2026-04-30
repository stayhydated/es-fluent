use es_fluent::{EsFluent, FluentLocalizer, FluentLocalizerExt, FluentValue};
use std::collections::HashMap;

#[derive(EsFluent)]
struct Child;

#[derive(EsFluent)]
struct Parent {
    child: Child,
}

#[derive(EsFluent)]
struct OptionalParent {
    child: Option<Child>,
}

#[derive(EsFluent)]
struct BorrowedName<'a> {
    name: &'a str,
}

#[derive(EsFluent)]
struct GenericParent<T: es_fluent::FluentMessage> {
    child: T,
}

struct TestLocalizer {
    child_value: &'static str,
}

impl FluentLocalizer for TestLocalizer {
    fn localize<'a>(
        &self,
        _id: &str,
        _args: Option<&HashMap<&str, FluentValue<'a>>>,
    ) -> Option<String> {
        None
    }

    fn localize_in_domain<'a>(
        &self,
        _domain: &str,
        id: &str,
        args: Option<&HashMap<&str, FluentValue<'a>>>,
    ) -> Option<String> {
        match id {
            "child" => Some(self.child_value.to_string()),
            "parent" => {
                let FluentValue::String(child) = args?.get("child")? else {
                    return None;
                };
                Some(format!("parent:{}", child.as_ref()))
            },
            "optional_parent" => match args?.get("child")? {
                FluentValue::String(child) => Some(format!("optional:{}", child.as_ref())),
                FluentValue::None => Some("optional:missing".to_string()),
                _ => None,
            },
            "borrowed_name" => {
                let FluentValue::String(name) = args?.get("name")? else {
                    return None;
                };
                Some(format!("name:{}", name.as_ref()))
            },
            "generic_parent" => {
                let FluentValue::String(child) = args?.get("child")? else {
                    return None;
                };
                Some(format!("generic:{}", child.as_ref()))
            },
            _ => None,
        }
    }
}

#[test]
fn derived_nested_message_field_does_not_need_clone() {
    let en = TestLocalizer {
        child_value: "child-en",
    };
    let fr = TestLocalizer {
        child_value: "child-fr",
    };
    let parent = Parent { child: Child };

    assert_eq!(en.localize_message(&parent), "parent:child-en");
    assert_eq!(fr.localize_message(&parent), "parent:child-fr");
}

#[test]
fn derived_optional_nested_message_field_uses_current_localizer() {
    let localizer = TestLocalizer {
        child_value: "child",
    };

    assert_eq!(
        localizer.localize_message(&OptionalParent { child: Some(Child) }),
        "optional:child"
    );
    assert_eq!(
        localizer.localize_message(&OptionalParent { child: None }),
        "optional:missing"
    );
}

#[test]
fn derived_borrowed_string_field_renders_as_argument() {
    let localizer = TestLocalizer {
        child_value: "child",
    };

    assert_eq!(
        localizer.localize_message(&BorrowedName { name: "Ada" }),
        "name:Ada"
    );
}

#[test]
fn derived_generic_message_field_uses_current_localizer() {
    let localizer = TestLocalizer {
        child_value: "child",
    };
    let parent = GenericParent { child: Child };

    assert_eq!(localizer.localize_message(&parent), "generic:child");
}
