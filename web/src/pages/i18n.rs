use es_fluent::{EsFluent, unic_langid::LanguageIdentifier};

es_fluent_manager_dioxus::define_i18n_module!();

/// The demo's locale directories, labeled in their own language.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(crate) enum DemoLanguage {
    #[default]
    EnUs,
    FrFr,
    ZhCn,
}

impl DemoLanguage {
    pub(crate) const ALL: [Self; 3] = [Self::EnUs, Self::FrFr, Self::ZhCn];

    pub(crate) const fn code(self) -> &'static str {
        match self {
            Self::EnUs => "en-US",
            Self::FrFr => "fr-FR",
            Self::ZhCn => "zh-CN",
        }
    }

    const fn autonym(self) -> &'static str {
        match self {
            Self::EnUs => "English (United States)",
            Self::FrFr => "français (France)",
            Self::ZhCn => "中文（中国）",
        }
    }
}

impl From<DemoLanguage> for LanguageIdentifier {
    fn from(language: DemoLanguage) -> Self {
        language.code().parse().expect("demo locale tags are valid")
    }
}

impl TryFrom<&LanguageIdentifier> for DemoLanguage {
    type Error = ();

    fn try_from(value: &LanguageIdentifier) -> Result<Self, Self::Error> {
        Self::ALL
            .into_iter()
            .find(|language| LanguageIdentifier::from(*language) == *value)
            .ok_or(())
    }
}

/// Autonyms read the same in every UI language, so labels need no lookup.
impl es_fluent::FluentMessage for DemoLanguage {
    fn to_fluent_string_with(&self, _: &mut es_fluent::FluentMessageLookup<'_>) -> String {
        self.autonym().to_owned()
    }
}

#[derive(Clone, Copy, Debug, EsFluent)]
pub(crate) enum EmberEchoCopy {
    RestaurantName,
    Kicker,
    Title,
    Manifesto,
    SignatureName,
    SignatureDescription,
    RivalryClaim,
    SelectLabel,
    ImageAlt,
    DoughTitle,
    DoughBody,
    FireTitle,
    FireBody,
    FinishTitle,
    FinishBody,
}

#[derive(Debug, EsFluent)]
pub(crate) struct CourtyardLocaleStatus<'a> {
    pub(crate) locale: &'a str,
}

#[derive(Debug, EsFluent)]
pub(crate) struct CourtyardWelcome<'a> {
    pub(crate) guest: &'a str,
}

#[derive(Debug, EsFluent)]
pub(crate) struct PiesAtDusk {
    pub(crate) count: i32,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn demo_languages_match_the_locale_directories_and_fallback() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
        let mut directories = std::fs::read_dir(root.join("assets/locales"))
            .expect("read locale directories")
            .map(|entry| entry.expect("locale entry").file_name().into_string().unwrap())
            .collect::<Vec<_>>();
        directories.sort();
        let mut codes = DemoLanguage::ALL.map(DemoLanguage::code).to_vec();
        codes.sort();
        assert_eq!(directories, codes);

        let config = std::fs::read_to_string(root.join("i18n.toml")).expect("read i18n.toml");
        let fallback = format!("fallback_language = \"{}\"", DemoLanguage::default().code());
        assert!(config.lines().any(|line| line.trim() == fallback));
        for language in DemoLanguage::ALL {
            assert_eq!(
                DemoLanguage::try_from(&LanguageIdentifier::from(language)),
                Ok(language)
            );
        }
    }
}
