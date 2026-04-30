use super::*;
use std::sync::{Mutex, RwLock, mpsc};
use std::time::Duration;

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

    let borrowed_bool = true;
    let borrowed_bool_value =
        FluentBorrowedArgumentValue::new(&borrowed_bool).into_fluent_argument_value(&mut localize);
    assert_string(borrowed_bool_value, "true");
}

#[test]
#[should_panic(expected = "ordinary arguments should not invoke nested localization")]
fn panic_lookup_reports_unexpected_nested_localization() {
    let _ = panic_lookup("domain", "id", None);
}

#[test]
fn argument_conversion_handles_optional_and_missing_values() {
    let mut localize = panic_lookup;
    let optional = Some("optional");
    let missing: Option<String> = None;
    let optional_number = Some(7i32);
    let optional_bool = Some(false);
    let missing_bool: Option<bool> = None;

    let optional_value = FluentOptionalArgumentValue::new(optional.as_ref())
        .into_fluent_argument_value(&mut localize);
    assert_string(optional_value, "optional");

    let missing_value = FluentOptionalArgumentValue::new(missing.as_ref())
        .into_fluent_argument_value(&mut localize);
    assert!(matches!(missing_value, FluentValue::None));

    let optional_number = FluentOptionalArgumentValue::new(optional_number.as_ref())
        .into_fluent_argument_value(&mut localize);
    assert_number(optional_number, 7.0);

    let optional_bool = FluentOptionalArgumentValue::new(optional_bool.as_ref())
        .into_fluent_argument_value(&mut localize);
    assert_string(optional_bool, "false");

    let missing_bool = FluentOptionalArgumentValue::new(missing_bool.as_ref())
        .into_fluent_argument_value(&mut localize);
    assert!(matches!(missing_bool, FluentValue::None));
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

#[test]
fn argument_conversion_localizes_optional_nested_messages_with_current_callback() {
    let mut localize = |domain: &str, id: &str, args: Option<&HashMap<&str, FluentValue<'_>>>| {
        assert_eq!(domain, "nested-domain");
        assert_eq!(id, "nested-id");
        assert!(args.is_none());
        "optional nested value".to_string()
    };

    let value =
        FluentArgumentValue::new(Some(NestedMessage)).into_fluent_argument_value(&mut localize);
    assert_string(value, "optional nested value");

    let missing = FluentArgumentValue::new(Option::<NestedMessage>::None)
        .into_fluent_argument_value(&mut localize);
    assert!(matches!(missing, FluentValue::None));
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

#[test]
fn localizer_extension_can_return_missing_typed_messages_without_id_fallback() {
    let localizer = StaticLocalizer { value: "Hello" };

    assert_eq!(
        localizer.try_localize_message(&NestedMessage),
        Some("Hello".to_string())
    );
    assert_eq!(localizer.try_localize_message(&MissingMessage), None);
}

struct BlockingSwitchLocalizer {
    selected: RwLock<&'static str>,
    child_seen: Mutex<mpsc::Sender<()>>,
    continue_child: Mutex<mpsc::Receiver<()>>,
}

impl BlockingSwitchLocalizer {
    fn new(child_seen: mpsc::Sender<()>, continue_child: mpsc::Receiver<()>) -> Self {
        Self {
            selected: RwLock::new("en"),
            child_seen: Mutex::new(child_seen),
            continue_child: Mutex::new(continue_child),
        }
    }

    fn select(&self, language: &'static str) {
        *self
            .selected
            .write()
            .expect("test language lock should not be poisoned") = language;
    }

    fn selected(&self) -> &'static str {
        *self
            .selected
            .read()
            .expect("test language lock should not be poisoned")
    }

    fn render_lookup(&self, language: &'static str, domain: &str, id: &str) -> Option<String> {
        if domain != "switch-domain" {
            return None;
        }

        if id == "child" {
            self.child_seen
                .lock()
                .expect("test child sender lock should not be poisoned")
                .send(())
                .expect("test should receive child lookup notification");
            self.continue_child
                .lock()
                .expect("test child receiver lock should not be poisoned")
                .recv()
                .expect("test should release child lookup");
        }

        matches!(id, "child" | "parent").then(|| format!("{language}-{id}"))
    }
}

impl FluentLocalizer for BlockingSwitchLocalizer {
    fn localize<'a>(
        &self,
        id: &str,
        _args: Option<&HashMap<&str, FluentValue<'a>>>,
    ) -> Option<String> {
        let language = self.selected();
        self.render_lookup(language, "switch-domain", id)
    }

    fn localize_in_domain<'a>(
        &self,
        domain: &str,
        id: &str,
        _args: Option<&HashMap<&str, FluentValue<'a>>>,
    ) -> Option<String> {
        let language = self.selected();
        self.render_lookup(language, domain, id)
    }

    fn with_lookup(
        &self,
        f: &mut dyn FnMut(
            &mut dyn for<'a> FnMut(
                &str,
                &str,
                Option<&HashMap<&str, FluentValue<'a>>>,
            ) -> Option<String>,
        ),
    ) {
        let selected = self
            .selected
            .read()
            .expect("test language lock should not be poisoned");
        let language = *selected;
        let mut lookup =
            |domain: &str, id: &str, _args: Option<&HashMap<&str, FluentValue<'_>>>| {
                self.render_lookup(language, domain, id)
            };

        f(&mut lookup);
    }
}

struct BlockingParent;

impl FluentMessage for BlockingParent {
    fn to_fluent_string_with(
        &self,
        localize: &mut dyn for<'a> FnMut(
            &str,
            &str,
            Option<&HashMap<&str, FluentValue<'a>>>,
        ) -> String,
    ) -> String {
        let child = localize("switch-domain", "child", None);
        let parent = localize("switch-domain", "parent", None);
        format!("{parent}:{child}")
    }
}

#[test]
fn localize_message_keeps_one_lookup_scope_during_concurrent_language_switch() {
    let (child_seen_tx, child_seen_rx) = mpsc::channel();
    let (continue_child_tx, continue_child_rx) = mpsc::channel();
    let localizer = Arc::new(BlockingSwitchLocalizer::new(
        child_seen_tx,
        continue_child_rx,
    ));

    let render_localizer = Arc::clone(&localizer);
    let render = std::thread::spawn(move || render_localizer.localize_message(&BlockingParent));

    child_seen_rx
        .recv_timeout(Duration::from_secs(1))
        .expect("render should reach the child lookup");

    let (switch_started_tx, switch_started_rx) = mpsc::channel();
    let (switch_done_tx, switch_done_rx) = mpsc::channel();
    let switch_localizer = Arc::clone(&localizer);
    let switch = std::thread::spawn(move || {
        switch_started_tx
            .send(())
            .expect("test should observe language switch start");
        switch_localizer.select("fr");
        switch_done_tx
            .send(())
            .expect("test should observe language switch completion");
    });

    switch_started_rx
        .recv_timeout(Duration::from_secs(1))
        .expect("language switch thread should start");
    assert!(
        switch_done_rx
            .recv_timeout(Duration::from_millis(50))
            .is_err(),
        "language switch completed while typed message render was still in progress"
    );

    continue_child_tx
        .send(())
        .expect("test should release the child lookup");

    let rendered = render
        .join()
        .expect("render thread should complete without panicking");
    switch_done_rx
        .recv_timeout(Duration::from_secs(1))
        .expect("language switch should complete after render");
    switch
        .join()
        .expect("language switch thread should complete without panicking");

    assert_eq!(rendered, "en-parent:en-child");
    assert_eq!(localizer.selected(), "fr");
}
