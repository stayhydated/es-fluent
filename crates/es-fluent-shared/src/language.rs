//! Shared language-identifier parsing helpers.

use icu::locale::{Locale, LocaleCanonicalizer};
use thiserror::Error;
use unic_langid::{LanguageIdentifier, LanguageIdentifierError};

/// Errors returned when parsing a language identifier that must already be canonicalized.
#[derive(Debug, Error)]
pub enum CanonicalLanguageIdentifierError {
    /// The identifier could not be parsed as a BCP-47 language identifier.
    #[error("Invalid language identifier '{name}'")]
    Invalid {
        /// The invalid identifier.
        name: String,
        /// The parsing error produced by `unic-langid`.
        #[source]
        source: LanguageIdentifierError,
    },
    /// The identifier parsed as a `unic-langid` identifier but could not be converted to ICU.
    #[error("Language identifier '{name}' could not be parsed as an ICU locale: {details}")]
    IcuInvalid {
        /// The invalid identifier.
        name: String,
        /// The ICU parsing error.
        details: String,
    },
    /// The identifier parsed successfully but was not written in canonical form.
    #[error("Locale directory '{name}' must use canonical BCP-47 form '{canonical}'")]
    NonCanonical {
        /// The original identifier.
        name: String,
        /// The canonical identifier.
        canonical: String,
    },
}

/// Parses a language identifier and rejects non-canonical locale forms.
pub fn parse_canonical_language_identifier(
    name: &str,
) -> Result<LanguageIdentifier, CanonicalLanguageIdentifierError> {
    let lang = name.parse::<LanguageIdentifier>().map_err(|source| {
        CanonicalLanguageIdentifierError::Invalid {
            name: name.to_string(),
            source,
        }
    })?;
    let mut locale =
        name.parse::<Locale>()
            .map_err(|source| CanonicalLanguageIdentifierError::IcuInvalid {
                name: name.to_string(),
                details: source.to_string(),
            })?;
    LocaleCanonicalizer::new_extended().canonicalize(&mut locale);

    let canonical = locale.to_string();
    if canonical != name {
        return Err(CanonicalLanguageIdentifierError::NonCanonical {
            name: name.to_string(),
            canonical,
        });
    }

    Ok(lang)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_canonical_language_identifier() {
        let lang = parse_canonical_language_identifier("de-DE-1901")
            .expect("canonical locale should parse");
        assert_eq!(lang.to_string(), "de-DE-1901");
    }

    #[test]
    fn rejects_invalid_identifier() {
        let err = parse_canonical_language_identifier("not-a-lang!")
            .expect_err("invalid locale should fail");
        assert!(matches!(
            err,
            CanonicalLanguageIdentifierError::Invalid { name, .. } if name == "not-a-lang!"
        ));
    }

    #[test]
    fn rejects_noncanonical_identifier() {
        let err = parse_canonical_language_identifier("en-us")
            .expect_err("noncanonical locale should fail");
        assert!(matches!(
            err,
            CanonicalLanguageIdentifierError::NonCanonical { name, canonical }
                if name == "en-us" && canonical == "en-US"
        ));
    }

    #[test]
    fn rejects_noncanonical_language_identifiers() {
        let err = parse_canonical_language_identifier("iw")
            .expect_err("noncanonical identifiers should fail");
        assert!(matches!(
            err,
            CanonicalLanguageIdentifierError::NonCanonical { name, canonical }
                if name == "iw" && canonical == "he"
        ));
    }
}
