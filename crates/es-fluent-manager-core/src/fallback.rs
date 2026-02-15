use fluent_fallback::env::LocalesProvider;
use unic_langid::LanguageIdentifier;

/// Returns language candidates in fallback order for the requested language.
///
/// The order is:
/// 1. Full canonical form.
/// 2. Canonical form without variants.
/// 3. Without region (if present).
/// 4. Without script (if present).
/// 5. Primary language subtag only.
pub fn locale_candidates(requested: &LanguageIdentifier) -> Vec<LanguageIdentifier> {
    let mut locales = Vec::new();
    let mut push = |candidate: LanguageIdentifier| {
        if !locales.iter().any(|lang| lang == &candidate) {
            locales.push(candidate);
        }
    };

    push(requested.clone());

    let mut without_variants = requested.clone();
    without_variants.clear_variants();
    push(without_variants.clone());

    if without_variants.region.is_some() {
        let mut no_region = without_variants.clone();
        no_region.region = None;
        push(no_region);
    }

    if without_variants.script.is_some() {
        let mut no_script = without_variants.clone();
        no_script.script = None;
        push(no_script);
    }

    if let Ok(primary) = without_variants
        .language
        .as_str()
        .parse::<LanguageIdentifier>()
    {
        push(primary);
    }

    locales
}

/// Returns a Fluent-style fallback locale list for the requested language.
///
/// This yields the requested locale first, then falls back to the primary
/// language subtag when region/script/variant subtags are present.
pub fn fallback_locales(requested: &LanguageIdentifier) -> impl LocalesProvider {
    locale_candidates(requested)
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

    #[test]
    fn locale_candidates_include_script_and_region_fallbacks() {
        let requested = langid!("sr-Cyrl-RS");
        let locales = locale_candidates(&requested);

        assert_eq!(
            locales,
            vec![
                langid!("sr-Cyrl-RS"),
                langid!("sr-Cyrl"),
                langid!("sr-RS"),
                langid!("sr")
            ]
        );
    }

    #[test]
    fn locale_candidates_include_variantless_form() {
        let requested = langid!("de-DE-1901");
        let locales = locale_candidates(&requested);

        assert_eq!(
            locales,
            vec![langid!("de-DE-1901"), langid!("de-DE"), langid!("de")]
        );
    }
}
