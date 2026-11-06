use es_fluent::{EsFluent, EsFluentLabel, EsFluentVariants};
use es_fluent::unic_langid::LanguageIdentifier;

#[cfg(feature = "bevy")]
use bevy::prelude::*;

#[cfg(feature = "bevy")]
es_fluent_manager_bevy::define_i18n_module!();

#[cfg(any(feature = "dioxus", feature = "embedded"))]
es_fluent_manager_embedded::define_i18n_module!();

#[derive(Clone, Copy, Debug, Default, Eq, EsFluent, PartialEq)]
#[cfg_attr(
    feature = "bevy",
    derive(Component, es_fluent_manager_bevy::BevyFluentText)
)]
pub enum ButtonState {
    #[default]
    Normal,
    Hovered,
    Pressed,
}

/// The example's locale directories, labeled in their own language for a picker.
///
/// For a generated enum with ICU4X locale names, see the `iso-types` crate's
/// `#[iso_locale]`.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
pub enum Languages {
    #[default]
    En,
    FrFr,
    ZhCn,
}

impl Languages {
    pub const ALL: [Self; 3] = [Self::En, Self::FrFr, Self::ZhCn];

    /// The locale directory name and BCP 47 tag.
    pub const fn code(self) -> &'static str {
        match self {
            Self::En => "en",
            Self::FrFr => "fr-FR",
            Self::ZhCn => "zh-CN",
        }
    }

    /// The language's name in itself, so each reader can find their own.
    pub const fn autonym(self) -> &'static str {
        match self {
            Self::En => "English",
            Self::FrFr => "français (France)",
            Self::ZhCn => "中文（中国）",
        }
    }

    pub fn next(self) -> Self {
        Self::ALL[(self as usize + 1) % Self::ALL.len()]
    }
}

impl From<Languages> for LanguageIdentifier {
    fn from(language: Languages) -> Self {
        language
            .code()
            .parse()
            .expect("configured locale tags are valid")
    }
}

impl TryFrom<&LanguageIdentifier> for Languages {
    type Error = UnsupportedLanguage;

    fn try_from(value: &LanguageIdentifier) -> Result<Self, Self::Error> {
        Self::ALL
            .into_iter()
            .find(|language| LanguageIdentifier::from(*language) == *value)
            .ok_or_else(|| UnsupportedLanguage(value.clone()))
    }
}

/// Autonyms read the same in every UI language, so labels need no lookup.
impl es_fluent::FluentMessage for Languages {
    fn to_fluent_string_with(&self, _: &mut es_fluent::FluentMessageLookup<'_>) -> String {
        self.autonym().to_owned()
    }
}

/// A locale outside [`Languages`].
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct UnsupportedLanguage(pub LanguageIdentifier);

impl std::fmt::Display for UnsupportedLanguage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "unsupported example language: {}", self.0)
    }
}

impl std::error::Error for UnsupportedLanguage {}

pub struct CurrentLanguage(pub Languages);

#[derive(Clone, Debug, Default, EsFluent, EsFluentLabel)]
pub struct EmptyStruct;

#[derive(Clone, Debug, Default, EsFluentLabel, EsFluentVariants)]
pub struct EmptyStructVariants;

#[derive(Clone, Copy, Debug, EsFluent, EsFluentLabel)]
pub enum EmptyEnum {}

#[derive(Clone, EsFluent, EsFluentLabel, EsFluentVariants)]
#[fluent_variants(keys = ["description", "label"])]
pub enum Country {
    USA(USAState),
    Canada(CanadaProvince),
}

#[derive(Clone, EsFluent, EsFluentLabel)]
pub enum USAState {
    A,
}

#[derive(Clone, EsFluent, EsFluentLabel)]
pub enum CanadaProvince {
    A,
    B,
}

#[derive(EsFluent, EsFluentLabel, EsFluentVariants)]
#[fluent_variants(keys = ["description", "label"])]
pub struct SplitVariants {
    pub country: Country,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn languages_match_the_locale_directories_and_fallback() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
        let mut directories = std::fs::read_dir(root.join("i18n"))
            .expect("read locale directories")
            .map(|entry| entry.expect("locale entry").file_name().into_string().unwrap())
            .collect::<Vec<_>>();
        directories.sort();
        let mut codes = Languages::ALL.map(Languages::code).to_vec();
        codes.sort();
        assert_eq!(directories, codes);

        let config = std::fs::read_to_string(root.join("i18n.toml")).expect("read i18n.toml");
        let fallback = format!("fallback_language = \"{}\"", Languages::default().code());
        assert!(config.lines().any(|line| line.trim() == fallback));
    }

    #[test]
    fn languages_round_trip_through_language_identifiers() {
        for language in Languages::ALL {
            let id = LanguageIdentifier::from(language);
            assert_eq!(Languages::try_from(&id), Ok(language));
        }
        let unsupported: LanguageIdentifier = "de".parse().unwrap();
        assert_eq!(
            Languages::try_from(&unsupported),
            Err(UnsupportedLanguage(unsupported))
        );
        assert_eq!(Languages::ZhCn.next(), Languages::En);
    }
}
