//! Shared Fluent identifier and domain newtypes.

use std::fmt;

/// Error returned when a Fluent identifier-like value is invalid.
#[derive(Clone, Debug, Eq, thiserror::Error, PartialEq)]
#[error("{label} {reason}")]
pub struct FluentIdentifierError {
    label: &'static str,
    reason: String,
}

impl FluentIdentifierError {
    fn new(label: &'static str, reason: impl Into<String>) -> Self {
        Self {
            label,
            reason: reason.into(),
        }
    }

    /// Human-readable label for the value that failed validation.
    pub fn label(&self) -> &'static str {
        self.label
    }

    /// Human-readable reason validation failed.
    pub fn reason(&self) -> &str {
        &self.reason
    }
}

macro_rules! fluent_string_type {
    ($name:ident, $label:literal) => {
        #[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
        pub struct $name(String);

        impl $name {
            /// Validates and creates the newtype.
            pub fn try_new(value: impl Into<String>) -> Result<Self, FluentIdentifierError> {
                let value = value.into();
                validate_identifier_like(&value, $label)?;
                Ok(Self(value))
            }

            /// Returns the value as a string slice.
            pub fn as_str(&self) -> &str {
                &self.0
            }

            /// Returns the owned string.
            pub fn into_string(self) -> String {
                self.0
            }
        }

        impl AsRef<str> for $name {
            fn as_ref(&self) -> &str {
                self.as_str()
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str(self.as_str())
            }
        }
    };
}

fluent_string_type!(FluentArgumentName, "Fluent argument name");
fluent_string_type!(FluentVariantKey, "Fluent variant key");
fluent_string_type!(FluentDomain, "Fluent domain");
fluent_string_type!(FluentMessageId, "Fluent message id");

/// A Fluent entry identifier, covering both message IDs and term IDs.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct FluentEntryId(String);

impl FluentEntryId {
    /// Validates and creates the entry identifier.
    pub fn try_new(value: impl Into<String>) -> Result<Self, FluentIdentifierError> {
        let value = value.into();
        if let Some(term_name) = value.strip_prefix('-') {
            validate_identifier_like(term_name, "Fluent entry id")?;
        } else {
            validate_identifier_like(&value, "Fluent entry id")?;
        }
        Ok(Self(value))
    }

    /// Returns the entry identifier as a string slice.
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Returns the owned string.
    pub fn into_string(self) -> String {
        self.0
    }
}

impl AsRef<str> for FluentEntryId {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl fmt::Display for FluentEntryId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

fn validate_identifier_like(value: &str, label: &'static str) -> Result<(), FluentIdentifierError> {
    if value.is_empty() {
        return Err(FluentIdentifierError::new(label, "must not be empty"));
    }

    let mut chars = value.chars();
    let first = chars.next().expect("checked non-empty");
    if !first.is_ascii_alphabetic() {
        return Err(FluentIdentifierError::new(
            label,
            "must start with an ASCII letter",
        ));
    }

    if let Some(invalid) = chars.find(|ch| !(ch.is_ascii_alphanumeric() || matches!(ch, '_' | '-')))
    {
        return Err(FluentIdentifierError::new(
            label,
            format!(
                "contains invalid character '{invalid}'; use ASCII letters, digits, '_' or '-'"
            ),
        ));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fluent_identifier_newtypes_accept_current_generated_shapes() {
        assert_eq!(
            FluentMessageId::try_new("status-Ready")
                .expect("message id")
                .as_str(),
            "status-Ready"
        );
        assert_eq!(
            FluentArgumentName::try_new("display_name")
                .expect("argument")
                .as_str(),
            "display_name"
        );
        assert_eq!(
            FluentVariantKey::try_new("custom-key")
                .expect("variant key")
                .as_str(),
            "custom-key"
        );
        assert_eq!(
            FluentDomain::try_new("es-fluent-lang")
                .expect("domain")
                .as_str(),
            "es-fluent-lang"
        );
        assert_eq!(
            FluentEntryId::try_new("-shared-term")
                .expect("term")
                .as_str(),
            "-shared-term"
        );
    }

    #[test]
    fn fluent_identifier_newtypes_reject_invalid_values() {
        let empty = FluentArgumentName::try_new("").expect_err("empty value should fail");
        assert_eq!(empty.to_string(), "Fluent argument name must not be empty");

        assert!(FluentArgumentName::try_new("1value").is_err());
        assert!(FluentArgumentName::try_new("display name").is_err());
        assert!(FluentMessageId::try_new("_message").is_err());
        assert!(FluentEntryId::try_new("-").is_err());
        assert!(FluentEntryId::try_new("-_term").is_err());
    }
}
