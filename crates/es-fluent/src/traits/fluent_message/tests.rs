use super::*;
use std::collections::HashMap;
use std::sync::{Mutex, RwLock, mpsc};
use std::time::Duration;

fn static_key(domain: &'static str, id: &'static str) -> StaticFluentMessageKey {
    crate::registry::__macro::static_message_key(
        "test-owner",
        crate::registry::__macro::static_domain(domain),
        crate::registry::__macro::static_entry_id(id),
    )
}

fn fallback_key(
    domain: &'static str,
    id: &'static str,
    fallback: &'static str,
) -> StaticFluentMessageKey {
    crate::registry::__macro::static_message_key_with_fallback(
        "test-owner",
        crate::registry::__macro::static_domain(domain),
        crate::registry::__macro::static_entry_id(id),
        fallback,
    )
}

fn panic_lookup<'a>(_key: StaticFluentMessageKey, _args: Option<&FluentArgs<'a>>) -> String {
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

    let choice_value = FluentArgumentValue::new(
        StaticFluentVariantKey::try_new("selected").expect("valid choice"),
    )
    .into_fluent_argument_value(&mut localize);
    assert_string(choice_value, "selected");

    let borrowed_bool = true;
    let borrowed_bool_value =
        FluentBorrowedArgumentValue::new(&borrowed_bool).into_fluent_argument_value(&mut localize);
    assert_string(borrowed_bool_value, "true");
}

#[test]
#[should_panic(expected = "ordinary arguments should not invoke nested localization")]
fn panic_lookup_reports_unexpected_nested_localization() {
    let _ = panic_lookup(static_key("domain", "id"), None);
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
    fn to_fluent_string_with(&self, localize: &mut FluentMessageLookup<'_>) -> String {
        localize(static_key("nested-domain", "nested-id"), None)
    }
}

#[test]
fn argument_conversion_localizes_nested_messages_with_current_callback() {
    let mut localize = |key: StaticFluentMessageKey, args: Option<&FluentArgs<'_>>| {
        assert_eq!(key.owner().as_str(), "test-owner");
        assert_eq!(key.domain().as_str(), "nested-domain");
        assert_eq!(key.id().as_str(), "nested-id");
        assert!(args.is_none());
        "nested value".to_string()
    };

    let value = FluentArgumentValue::new(NestedMessage).into_fluent_argument_value(&mut localize);
    assert_string(value, "nested value");
}

#[test]
fn argument_conversion_localizes_optional_nested_messages_with_current_callback() {
    let mut localize = |key: StaticFluentMessageKey, args: Option<&FluentArgs<'_>>| {
        assert_eq!(key.owner().as_str(), "test-owner");
        assert_eq!(key.domain().as_str(), "nested-domain");
        assert_eq!(key.id().as_str(), "nested-id");
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
        key: StaticFluentMessageKey,
        _args: Option<&FluentArgs<'a>>,
    ) -> Option<String> {
        if key.owner() == "test-owner" && key.domain() == "nested-domain" && key.id() == "nested-id"
        {
            Some(self.value.to_string())
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
    fn to_fluent_string_with(&self, localize: &mut FluentMessageLookup<'_>) -> String {
        localize(static_key("missing-domain", "missing-id"), None)
    }
}

struct FallbackMessage;

impl FluentMessage for FallbackMessage {
    fn to_fluent_string_with(&self, localize: &mut FluentMessageLookup<'_>) -> String {
        localize(
            fallback_key("missing-domain", "fallback-id", "fallback_message"),
            None,
        )
    }
}

struct MapLocalizer(HashMap<StaticFluentMessageKey, &'static str>);

impl FluentLocalizer for MapLocalizer {
    fn localize<'a>(
        &self,
        key: StaticFluentMessageKey,
        _args: Option<&FluentArgs<'a>>,
    ) -> Option<String> {
        self.0.get(&key).map(|value| (*value).to_string())
    }
}

#[test]
fn fallback_message_key_matches_custom_localizer_map_entry() {
    let localizer = MapLocalizer(HashMap::from([(
        static_key("missing-domain", "fallback-id"),
        "Translated fallback",
    )]));

    assert_eq!(
        localizer.localize_message(&FallbackMessage),
        "Translated fallback"
    );
}

struct CallbackOnlyMessage;

impl FluentMessage for CallbackOnlyMessage {
    fn to_fluent_string_with(&self, localize: &mut FluentMessageLookup<'_>) -> String {
        localize(static_key("callback-domain", "callback-id"), None)
    }
}

#[test]
fn fluent_message_reference_impl_delegates_to_inner_message() {
    let message = NestedMessage;
    let message_ref = &message;
    let mut localize = |key: StaticFluentMessageKey, _args: Option<&FluentArgs<'_>>| {
        format!("{}:{}", key.domain().as_str(), key.id().as_str())
    };

    assert_eq!(
        FluentMessage::to_fluent_string_with(&message_ref, &mut localize),
        "nested-domain:nested-id"
    );
}

#[test]
fn manual_fluent_message_uses_supplied_callback_for_lookup() {
    let mut called = false;
    let mut localize = |key: StaticFluentMessageKey, args: Option<&FluentArgs<'_>>| {
        called = true;
        assert_eq!(key.owner().as_str(), "test-owner");
        assert_eq!(key.domain().as_str(), "callback-domain");
        assert_eq!(key.id().as_str(), "callback-id");
        assert!(args.is_none());
        "callback result".to_string()
    };

    assert_eq!(
        CallbackOnlyMessage.to_fluent_string_with(&mut localize),
        "callback result"
    );
    assert!(called);
}

#[test]
fn fluent_localizer_reference_and_arc_impls_delegate_to_inner_localizer() {
    let localizer = StaticLocalizer { value: "Hello" };
    let localizer_ref = &localizer;
    let localizer_arc = Arc::new(StaticLocalizer { value: "Bonjour" });

    assert_eq!(localizer_ref.localize_message(&NestedMessage), "Hello");
    assert_eq!(localizer_arc.localize_message(&NestedMessage), "Bonjour");
    assert_eq!(
        FluentLocalizer::localize(
            &localizer_ref,
            static_key("nested-domain", "nested-id"),
            None
        ),
        Some("Hello".to_string())
    );
    assert_eq!(
        FluentLocalizer::localize(
            &localizer_ref,
            static_key("nested-domain", "nested-id"),
            None,
        ),
        Some("Hello".to_string())
    );
    assert_eq!(
        FluentLocalizer::localize(
            &localizer_arc,
            static_key("nested-domain", "nested-id"),
            None,
        ),
        Some("Bonjour".to_string())
    );
}

#[test]
fn localizer_extension_localizes_typed_messages() {
    let localizer = StaticLocalizer { value: "Hello" };

    assert_eq!(
        FluentLocalizer::localize(&localizer, static_key("nested-domain", "nested-id"), None),
        Some("Hello".to_string())
    );
    assert_eq!(
        FluentLocalizer::localize(&localizer, static_key("nested-domain", "nested-id"), None,),
        Some("Hello".to_string())
    );
}

#[test]
#[should_panic(expected = "missing Fluent message `missing-id` in domain `missing-domain`")]
fn localizer_extension_panics_when_a_typed_message_is_missing() {
    let localizer = StaticLocalizer { value: "Hello" };
    let _ = localizer.localize_message(&MissingMessage);
}

#[test]
fn localizer_extension_can_return_missing_typed_messages_without_id_fallback() {
    let localizer = StaticLocalizer { value: "Hello" };

    assert_eq!(
        localizer.try_localize_message(&NestedMessage),
        Some("Hello".to_string())
    );
    assert_eq!(localizer.try_localize_message(&MissingMessage), None);
    assert_eq!(
        localizer.localize_message(&FallbackMessage),
        "fallback_message"
    );
    assert_eq!(localizer.try_localize_message(&FallbackMessage), None);
}

struct MinimalScopedLocalizer;

impl MinimalScopedLocalizer {
    fn lookup(
        &self,
        key: StaticFluentMessageKey,
        _args: Option<&FluentArgs<'_>>,
    ) -> Option<String> {
        Some(format!("{}:{}", key.domain(), key.id()))
    }
}

impl FluentLocalizer for MinimalScopedLocalizer {
    fn localize<'a>(
        &self,
        key: StaticFluentMessageKey,
        args: Option<&FluentArgs<'a>>,
    ) -> Option<String> {
        self.lookup(key, args)
    }

    fn with_lookup(&self, f: &mut dyn FnMut(&mut FluentLocalizerLookup<'_>)) {
        let mut lookup =
            |key: StaticFluentMessageKey, args: Option<&FluentArgs<'_>>| self.localize(key, args);
        f(&mut lookup);
    }
}

struct ScopedMessage;

impl FluentMessage for ScopedMessage {
    fn to_fluent_string_with(&self, localize: &mut FluentMessageLookup<'_>) -> String {
        localize(static_key("custom-domain", "scoped-message"), None)
    }
}

#[test]
fn custom_localizer_with_lookup_invokes_callback_and_renders_typed_message() {
    assert_eq!(
        MinimalScopedLocalizer.localize_message(&ScopedMessage),
        "custom-domain:scoped-message"
    );
}

struct SkippingCallbackLocalizer;

impl FluentLocalizer for SkippingCallbackLocalizer {
    fn localize<'a>(
        &self,
        _key: StaticFluentMessageKey,
        _args: Option<&FluentArgs<'a>>,
    ) -> Option<String> {
        None
    }

    fn with_lookup(&self, _f: &mut dyn FnMut(&mut FluentLocalizerLookup<'_>)) {}
}

struct DoubleCallbackLocalizer;

impl FluentLocalizer for DoubleCallbackLocalizer {
    fn localize<'a>(
        &self,
        key: StaticFluentMessageKey,
        _args: Option<&FluentArgs<'a>>,
    ) -> Option<String> {
        Some(key.id().as_str().to_string())
    }

    fn with_lookup(&self, f: &mut dyn FnMut(&mut FluentLocalizerLookup<'_>)) {
        let mut lookup = |key: StaticFluentMessageKey, _args: Option<&FluentArgs<'_>>| {
            Some(key.id().as_str().to_string())
        };
        f(&mut lookup);
        f(&mut lookup);
    }
}

#[test]
#[should_panic(expected = "FluentLocalizer::with_lookup must invoke its callback exactly once")]
fn localize_message_panics_when_with_lookup_skips_callback() {
    SkippingCallbackLocalizer.localize_message(&NestedMessage);
}

#[test]
#[should_panic(expected = "FluentLocalizer::with_lookup must invoke its callback exactly once")]
fn try_localize_message_panics_when_with_lookup_invokes_callback_twice() {
    let _ = DoubleCallbackLocalizer.try_localize_message(&NestedMessage);
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
        key: StaticFluentMessageKey,
        _args: Option<&FluentArgs<'a>>,
    ) -> Option<String> {
        let language = self.selected();
        self.render_lookup(language, key.domain().as_str(), key.id().as_str())
    }

    fn with_lookup(&self, f: &mut dyn FnMut(&mut FluentLocalizerLookup<'_>)) {
        let selected = self
            .selected
            .read()
            .expect("test language lock should not be poisoned");
        let language = *selected;
        let mut lookup = |key: StaticFluentMessageKey, _args: Option<&FluentArgs<'_>>| {
            self.render_lookup(language, key.domain().as_str(), key.id().as_str())
        };

        f(&mut lookup);
    }
}

struct BlockingParent;

impl FluentMessage for BlockingParent {
    fn to_fluent_string_with(&self, localize: &mut FluentMessageLookup<'_>) -> String {
        let child = localize(static_key("switch-domain", "child"), None);
        let parent = localize(static_key("switch-domain", "parent"), None);
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
