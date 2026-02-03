use fluent_fallback::env::LocalesProvider;
use unic_langid::LanguageIdentifier;

/// Returns a Fluent-style fallback locale list for the requested language.
///
/// This yields the requested locale first, then falls back to the primary
/// language subtag when region/script/variant subtags are present.
pub fn fallback_locales(requested: &LanguageIdentifier) -> impl LocalesProvider {
    let mut locales = Vec::new();
    locales.push(requested.clone());

    let needs_primary_fallback = requested.script.is_some()
        || requested.region.is_some()
        || requested.variants().next().is_some();

    if needs_primary_fallback
        && let Ok(primary) = requested.language.as_str().parse::<LanguageIdentifier>()
        && !locales.iter().any(|lang| lang == &primary)
    {
        locales.push(primary);
    }

    locales
}

/// Picks the first available language from the fallback chain.
pub fn resolve_fallback_language(
    requested: &LanguageIdentifier,
    available: &[LanguageIdentifier],
) -> Option<LanguageIdentifier> {
    fallback_locales(requested)
        .locales()
        .find(|candidate| available.iter().any(|lang| lang == candidate))
}

#[cfg(test)]
mod tests {
    use super::*;
    use fluent_fallback::env::LocalesProvider;
    use unic_langid::langid;

    #[test]
    fn fallback_locales_includes_primary_language() {
        let requested = langid!("en-US");
        let locales: Vec<_> = fallback_locales(&requested).locales().collect();

        assert_eq!(locales, vec![langid!("en-US"), langid!("en")]);
    }

    #[test]
    fn resolve_fallback_prefers_exact_match() {
        let requested = langid!("en-US");
        let available = vec![langid!("en-US"), langid!("en")];

        assert_eq!(
            resolve_fallback_language(&requested, &available),
            Some(langid!("en-US"))
        );
    }

    #[test]
    fn resolve_fallback_uses_primary_language() {
        let requested = langid!("en-US");
        let available = vec![langid!("en"), langid!("fr")];

        assert_eq!(
            resolve_fallback_language(&requested, &available),
            Some(langid!("en"))
        );
    }

    #[test]
    fn resolve_fallback_returns_none_when_missing() {
        let requested = langid!("en-US");
        let available = vec![langid!("fr")];

        assert_eq!(resolve_fallback_language(&requested, &available), None);
    }
}
