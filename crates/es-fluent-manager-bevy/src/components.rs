use bevy::prelude::*;
use es_fluent::FluentMessage;

/// A Bevy component that holds localized text content.
///
/// `FluentText` is a generic component that wraps any type implementing
/// `FluentMessage` and `Clone`. It's designed to work with the `es-fluent`
/// localization system in Bevy applications.
///
/// When its message type is registered through `#[derive(BevyFluentText)]` or
/// manual registration, `FluentText` updates Bevy `Text` components when the
/// application locale changes.
///
/// # Examples
///
/// ```ignore
/// use bevy::prelude::*;
/// use es_fluent::EsFluent;
/// use es_fluent_manager_bevy::{BevyFluentText, FluentText};
///
/// // The message key is derived from the struct name: "simple-message"
/// #[derive(BevyFluentText, Clone, EsFluent)]
/// struct SimpleMessage {
///     name: String,
/// }
///
/// fn setup_text(mut commands: Commands) {
///     let content = SimpleMessage { name: "World".to_string() };
///     commands.spawn((FluentText::new(content), Text::new("")));
/// }
/// ```
#[derive(Clone, Component)]
pub struct FluentText<T: FluentMessage + Clone> {
    /// The localized text content.
    pub value: T,
}

impl<T: FluentMessage + Clone> FluentText<T> {
    /// Creates a new `FluentText` component with the given value.
    ///
    /// # Arguments
    ///
    /// * `value` - The text content that implements `FluentMessage` and `Clone`
    ///
    /// # Examples
    ///
    /// Create a FluentText component with a simple string message:
    ///
    /// ```ignore
    /// use es_fluent_manager_bevy::FluentText;
    /// use es_fluent::EsFluent;
    ///
    /// // The message key is derived from the struct name: "message"
    /// #[derive(Clone, EsFluent)]
    /// struct Message {
    ///     content: String,
    /// }
    ///
    /// let text = FluentText::new(Message { content: "Hello".to_string() });
    /// ```
    pub fn new(value: T) -> Self {
        Self { value }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Clone)]
    struct FakeMessage(&'static str);

    impl FluentMessage for FakeMessage {
        fn to_fluent_string_with(
            &self,
            _localize: &mut dyn for<'a> FnMut(
                &str,
                &str,
                Option<&std::collections::HashMap<&str, es_fluent::FluentValue<'a>>>,
            ) -> String,
        ) -> String {
            self.0.to_string()
        }
    }

    #[test]
    fn fluent_text_new_stores_inner_value() {
        let component = FluentText::new(FakeMessage("hello"));
        assert_eq!(component.value.0, "hello");
    }

    #[test]
    fn fluent_text_clone_clones_inner_value() {
        let component = FluentText::new(FakeMessage("hello"));
        let cloned = component.clone();

        assert_eq!(cloned.value.0, "hello");
    }

    #[test]
    fn fluent_text_can_be_inserted_as_bevy_component() {
        let mut world = World::new();
        let entity = world.spawn(FluentText::new(FakeMessage("hello"))).id();

        let component = world
            .get::<FluentText<FakeMessage>>(entity)
            .expect("component should be present");
        assert_eq!(component.value.0, "hello");
    }

    #[test]
    fn fluent_text_value_can_render_through_fluent_message_trait() {
        let component = FluentText::new(FakeMessage("hello"));
        let mut localize = |_domain: &str,
                            _id: &str,
                            _args: Option<
            &std::collections::HashMap<&str, es_fluent::FluentValue<'_>>,
        >| { "unused".to_string() };

        assert_eq!(
            component.value.to_fluent_string_with(&mut localize),
            "hello"
        );
    }
}
