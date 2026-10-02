use es_fluent_manager_core::fallback::{
    FallbackChainAvailability, locale_candidates, resolve_fallback_chain_availability,
    resolve_fallback_language,
};
use proptest::prelude::*;
use unic_langid::LanguageIdentifier;

// Explicit candidate orders are independent of the resolver under test. Include
// CLDR parents that simple BCP-47 subtag truncation cannot produce.
const CHAINS: &[(&str, &[&str])] = &[
    ("en-US", &["en-US", "en"]),
    (
        "hi-Latn-IN",
        &["hi-Latn-IN", "hi-Latn", "en-IN", "en-001", "en"],
    ),
    ("de-DE-1901", &["de-DE-1901", "de-DE", "de-1901", "de"]),
];

fn language(value: &str) -> LanguageIdentifier {
    value.parse().expect("fixture locale must be valid")
}

fn candidates(chain: &[&str], indices: &[usize]) -> Vec<LanguageIdentifier> {
    indices
        .iter()
        .map(|&index| language(chain.get(index).copied().unwrap_or("fr")))
        .collect()
}

fn first_match(chain: &[&str], inputs: &[LanguageIdentifier]) -> Option<LanguageIdentifier> {
    chain
        .iter()
        .map(|value| language(value))
        .find(|candidate| inputs.contains(candidate))
}

proptest! {
    #![proptest_config(ProptestConfig { cases: 128, .. ProptestConfig::default() })]

    #[test]
    fn fallback_order_and_category_precedence_match_explicit_chains(
        fixture in 0usize..CHAINS.len(),
        ready in prop::collection::vec(0usize..7, 0..17),
        available in prop::collection::vec(0usize..7, 0..17),
        blocked in prop::collection::vec(0usize..7, 0..17),
    ) {
        let (requested, chain) = CHAINS[fixture];
        let requested = language(requested);
        let ready = candidates(chain, &ready);
        let available = candidates(chain, &available);
        let blocked = candidates(chain, &blocked);
        prop_assert_eq!(locale_candidates(&requested), chain.iter().map(|value| language(value)).collect::<Vec<_>>());
        prop_assert_eq!(resolve_fallback_language(&requested, &available), first_match(chain, &available));
        let expected = first_match(chain, &ready).map(FallbackChainAvailability::Ready)
            .or_else(|| first_match(chain, &available).map(FallbackChainAvailability::Available))
            .or_else(|| first_match(chain, &blocked).map(FallbackChainAvailability::Blocked))
            .unwrap_or(FallbackChainAvailability::Unavailable);
        let actual = resolve_fallback_chain_availability(&requested, &ready, &available, &blocked);
        prop_assert_eq!(&actual, &expected);

        let duplicate_and_reverse = |inputs: &[LanguageIdentifier]| inputs.iter().rev().chain(inputs).cloned().collect::<Vec<_>>();
        prop_assert_eq!(resolve_fallback_chain_availability(
            &requested,
            &duplicate_and_reverse(&ready),
            &duplicate_and_reverse(&available),
            &duplicate_and_reverse(&blocked),
        ), expected);
    }
}
