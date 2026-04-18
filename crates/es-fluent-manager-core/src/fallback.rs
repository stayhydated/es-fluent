use fluent_fallback::env::LocalesProvider;
use icu_locale::{Locale, fallback::LocaleFallbacker};
use unic_langid::LanguageIdentifier;

#[derive(Clone, Debug, Eq, PartialEq)]
#[doc(hidden)]
pub enum FallbackChainAvailability {
    Ready(LanguageIdentifier),
    Available(LanguageIdentifier),
    Blocked(LanguageIdentifier),
    Unavailable,
}

fn sorted_languages(languages: &[LanguageIdentifier]) -> Vec<LanguageIdentifier> {
    let mut languages = languages.to_vec();
    languages.sort_by_key(|lang| lang.to_string());
    languages.dedup();
    languages
}

/// Returns language candidates in fallback order for the requested language.
///
/// This uses ICU4X locale fallback data to produce a CLDR-backed parent chain
/// independent of the currently available locales.
pub fn locale_candidates(requested: &LanguageIdentifier) -> Vec<LanguageIdentifier> {
    let mut locales = Vec::new();
    let mut push = |candidate: LanguageIdentifier| {
        if !locales.iter().any(|lang| lang == &candidate) {
            locales.push(candidate);
        }
    };

    push(requested.clone());

    let Ok(locale) = requested.to_string().parse::<Locale>() else {
        return locales;
    };

    let fallbacker = LocaleFallbacker::new();
    let mut iterator = fallbacker
        .for_config(Default::default())
        .fallback_for(locale.into());

    loop {
        let current = iterator.get();
        if current.is_unknown() {
            break;
        }

        if let Ok(candidate) = current.to_string().parse::<LanguageIdentifier>() {
            push(candidate);
        }

        iterator.step();
    }

    locales
}

/// Returns a CLDR-backed fallback locale list for the requested language.
pub fn fallback_locales(requested: &LanguageIdentifier) -> impl LocalesProvider {
    locale_candidates(requested)
}

/// Picks the first available language from the fallback chain.
pub fn resolve_fallback_language(
    requested: &LanguageIdentifier,
    available: &[LanguageIdentifier],
) -> Option<LanguageIdentifier> {
    let available = sorted_languages(available);

    locale_candidates(requested)
        .into_iter()
        .find(|candidate| available.iter().any(|lang| lang == candidate))
}

/// Resolves the first matching locale in the fallback chain by availability category.
///
/// Category precedence is global across the entire fallback chain:
/// 1. First locale present in `ready`
/// 2. Otherwise first locale present in `available`
/// 3. Otherwise first locale present in `blocked`
#[doc(hidden)]
pub fn resolve_fallback_chain_availability(
    requested: &LanguageIdentifier,
    ready: &[LanguageIdentifier],
    available: &[LanguageIdentifier],
    blocked: &[LanguageIdentifier],
) -> FallbackChainAvailability {
    let ready = sorted_languages(ready);
    let available = sorted_languages(available);
    let blocked = sorted_languages(blocked);

    if let Some(candidate) = locale_candidates(requested)
        .into_iter()
        .find(|candidate| ready.iter().any(|lang| lang == candidate))
    {
        return FallbackChainAvailability::Ready(candidate);
    }

    if let Some(candidate) = locale_candidates(requested)
        .into_iter()
        .find(|candidate| available.iter().any(|lang| lang == candidate))
    {
        return FallbackChainAvailability::Available(candidate);
    }

    if let Some(candidate) = locale_candidates(requested)
        .into_iter()
        .find(|candidate| blocked.iter().any(|lang| lang == candidate))
    {
        return FallbackChainAvailability::Blocked(candidate);
    }

    FallbackChainAvailability::Unavailable
}

/// Picks the best locale for active use, preferring ready locales over merely available locales.
///
/// Resolution order:
/// 1. First fallback candidate present in `ready`.
/// 2. If no ready match exists, first fallback candidate present in `available`.
pub fn resolve_ready_locale(
    requested: &LanguageIdentifier,
    ready: &[LanguageIdentifier],
    available: &[LanguageIdentifier],
) -> Option<LanguageIdentifier> {
    match resolve_fallback_chain_availability(requested, ready, available, &[]) {
        FallbackChainAvailability::Ready(candidate)
        | FallbackChainAvailability::Available(candidate) => Some(candidate),
        FallbackChainAvailability::Blocked(_) | FallbackChainAvailability::Unavailable => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
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
    fn resolve_ready_locale_prefers_ready_candidates() {
        let requested = langid!("en-US");
        let ready = vec![langid!("en")];
        let available = vec![langid!("en-US"), langid!("en")];

        assert_eq!(
            resolve_ready_locale(&requested, &ready, &available),
            Some(langid!("en"))
        );
    }

    #[test]
    fn resolve_ready_locale_falls_back_to_available_when_not_ready() {
        let requested = langid!("en-US");
        let ready = vec![langid!("fr")];
        let available = vec![langid!("en-US"), langid!("en")];

        assert_eq!(
            resolve_ready_locale(&requested, &ready, &available),
            Some(langid!("en-US"))
        );
    }

    #[test]
    fn resolve_fallback_chain_availability_prefers_ready_over_blocked_exact_match() {
        let requested = langid!("en-US");
        let ready = vec![langid!("en")];
        let blocked = vec![langid!("en-US")];

        assert_eq!(
            resolve_fallback_chain_availability(&requested, &ready, &[], &blocked),
            FallbackChainAvailability::Ready(langid!("en"))
        );
    }

    #[test]
    fn resolve_fallback_chain_availability_returns_available_when_not_ready() {
        let requested = langid!("en-US");
        let available = vec![langid!("en-US"), langid!("en")];

        assert_eq!(
            resolve_fallback_chain_availability(&requested, &[], &available, &[]),
            FallbackChainAvailability::Available(langid!("en-US"))
        );
    }

    #[test]
    fn resolve_fallback_chain_availability_returns_blocked_when_only_blocked_match_exists() {
        let requested = langid!("en-US");
        let blocked = vec![langid!("en-US"), langid!("en")];

        assert_eq!(
            resolve_fallback_chain_availability(&requested, &[], &[], &blocked),
            FallbackChainAvailability::Blocked(langid!("en-US"))
        );
    }

    #[test]
    fn resolve_fallback_chain_availability_returns_unavailable_when_missing() {
        let requested = langid!("en-US");

        assert_eq!(
            resolve_fallback_chain_availability(&requested, &[], &[], &[]),
            FallbackChainAvailability::Unavailable
        );
    }

    #[test]
    fn resolve_fallback_returns_none_for_generic_request_without_parent_match() {
        let requested = langid!("en");
        let available = vec![langid!("en-US"), langid!("en-CA")];

        assert_eq!(resolve_fallback_language(&requested, &available), None);
    }

    #[test]
    fn locale_candidates_include_cldr_parents() {
        let requested = langid!("hi-Latn-IN");
        let locales = locale_candidates(&requested);

        assert_eq!(
            locales,
            vec![
                langid!("hi-Latn-IN"),
                langid!("hi-Latn"),
                langid!("en-IN"),
                langid!("en-001"),
                langid!("en"),
            ]
        );
    }

    #[test]
    fn locale_candidates_include_variant_and_variantless_parents() {
        let requested = langid!("de-DE-1901");
        let locales = locale_candidates(&requested);

        assert_eq!(
            locales,
            vec![
                langid!("de-DE-1901"),
                langid!("de-DE"),
                langid!("de-1901"),
                langid!("de"),
            ]
        );
    }

    #[test]
    fn resolve_fallback_uses_documented_candidate_order() {
        let requested = langid!("hi-Latn-IN");
        let available = vec![langid!("en"), langid!("en-001")];

        assert_eq!(
            resolve_fallback_language(&requested, &available),
            Some(langid!("en-001"))
        );
    }

    #[test]
    fn resolve_fallback_normalizes_unsorted_duplicate_available_languages() {
        let requested = langid!("en-US");
        let available = vec![langid!("fr"), langid!("en"), langid!("fr"), langid!("en")];

        assert_eq!(
            resolve_fallback_language(&requested, &available),
            Some(langid!("en"))
        );
    }
}
