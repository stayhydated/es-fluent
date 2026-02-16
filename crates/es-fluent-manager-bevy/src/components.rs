use bevy::prelude::*;
use es_fluent::ToFluentString;
use std::fmt::Display;

/// A Bevy component that holds localized text content.
///
/// `FluentText` is a generic component that wraps any type implementing `ToFluentString`
/// and `Clone`. It's designed to work with the `es-fluent` localization system in Bevy
/// applications.
///
/// When used with Bevy's text rendering systems, `FluentText` automatically handles
/// localization and text updates when the application locale changes.
///
/// # Examples
///
/// ```ignore
/// use bevy::prelude::*;
/// use es_fluent_manager_bevy::FluentText;
/// use es_fluent::EsFluent;
///
/// // The message key is derived from the struct name: "simple-message"
/// #[derive(Clone, EsFluent)]
/// struct SimpleMessage {
///     name: String,
/// }
///
/// fn setup_text(mut commands: Commands) {
///     let content = SimpleMessage { name: "World".to_string() };
///     commands.spawn(FluentText::new(content));
/// }
/// ```
#[derive(Clone, Component)]
pub struct FluentText<T: ToFluentString + Clone> {
    /// The localized text content.
    pub value: T,
}

impl<T: ToFluentString + Clone> FluentText<T> {
    /// Creates a new `FluentText` component with the given value.
    ///
    /// # Arguments
    ///
    /// * `value` - The text content that implements `ToFluentString` and `Clone`
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

impl<T: ToFluentString + Clone> Display for FluentText<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.value.to_fluent_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Clone)]
    struct FakeMessage(&'static str);

    impl ToFluentString for FakeMessage {
        fn to_fluent_string(&self) -> String {
            self.0.to_string()
        }
    }

    #[test]
    fn fluent_text_new_and_display_use_inner_to_fluent_string() {
        let component = FluentText::new(FakeMessage("hello"));
        assert_eq!(component.value.to_fluent_string(), "hello");
        assert_eq!(component.to_string(), "hello");
    }
}
