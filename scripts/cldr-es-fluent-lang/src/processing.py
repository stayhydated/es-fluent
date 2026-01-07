"""Locale processing logic for CLDR data."""

from __future__ import annotations

from collections import defaultdict
from pathlib import Path

from .loaders import (
    load_available_locales,
    load_likely_subtags,
    load_locale_entry,
    load_script_names,
    load_territory_names,
)
from .models import Locale, LocaleIdentity


def expand_locale(tag: str, likely_subtags: dict[str, str]) -> Locale:
    """Expand a minimal locale tag using likelySubtags."""
    canonical_tag = str(Locale.parse(tag))
    expanded_tag = likely_subtags.get(canonical_tag)
    if expanded_tag:
        return Locale.parse(expanded_tag)
    return Locale.parse(canonical_tag)


def fallback_chain(locale: str) -> list[str]:
    """Generate the fallback chain for a locale."""
    canonical_tag = str(Locale.parse(locale))
    parts = canonical_tag.split("-")
    chain = ["-".join(parts[:i]) for i in range(len(parts), 0, -1)]
    chain.append("root")
    return chain


def candidate_language_keys(
    original_locale: Locale, expanded_locale: Locale
) -> list[str]:
    """Generate candidate keys for autonym lookup."""
    keys: list[str] = []
    for key_tag in (
        str(original_locale),
        str(expanded_locale),
        # lang-script
        str(Locale(language=expanded_locale.language, script=expanded_locale.script)),
        # lang-region
        str(Locale(language=expanded_locale.language, region=expanded_locale.region)),
        expanded_locale.language,
    ):
        if key_tag and key_tag not in keys:
            keys.append(key_tag)
    return keys


def collapse_entries(entries: dict[str, str]) -> dict[str, str]:
    """Deduplicate entries where multiple region variants share the same name."""
    grouped: dict[Locale, dict[str, list[str]]] = defaultdict(lambda: defaultdict(list))
    for locale_tag, name in entries.items():
        parts = Locale.parse(locale_tag)
        # Group by lang, script, and variants (ignoring region)
        key_locale = Locale(
            language=parts.language, script=parts.script, variants=parts.variants
        )
        grouped[key_locale][name].append(locale_tag)

    collapsed: dict[str, str] = {}
    canonical_sources: dict[str, list[str]] = {}

    for key_locale, name_map in grouped.items():
        for name, locales in name_map.items():
            locales = sorted(locales)
            if len(locales) == 1:
                collapsed[locales[0]] = name
                continue

            canonical_tag = str(key_locale)
            existing = collapsed.get(canonical_tag)

            if existing is None:
                collapsed[canonical_tag] = name
                canonical_sources[canonical_tag] = locales
                continue

            if existing == name:
                continue

            previous_locales = canonical_sources.pop(canonical_tag, [])
            collapsed.pop(canonical_tag, None)
            for previous in previous_locales:
                collapsed[previous] = existing
            for locale in locales:
                collapsed[locale] = name

    return collapsed


def format_locale(
    expanded_locale: Locale,
    original_locale: Locale,
    likely_subtags: dict[str, str],
) -> str:
    """Normalize output locale tags."""
    expanded_tag = str(expanded_locale)
    original_tag = str(original_locale)
    drop_script = False

    language = expanded_locale.language
    script = expanded_locale.script
    region = expanded_locale.region
    variants = expanded_locale.variants

    if original_locale.script is None:
        if region and likely_subtags.get(original_tag) == expanded_tag:
            drop_script = True
        elif script == "Latn":
            drop_script = True

    if drop_script:
        script = None

    if region == "001" and original_locale.region is None:
        region = None

    return str(
        Locale(language=language, script=script, region=region, variants=variants)
    )


def collect_entries(cldr_root: Path) -> dict[str, str]:
    """Collect all locale entries from CLDR data."""
    languages_root = cldr_root / "cldr-localenames-full" / "main"

    likely_subtags = load_likely_subtags(cldr_root)
    available_locales = load_available_locales(cldr_root)

    english_entry = load_locale_entry(languages_root, "en")
    if not english_entry:
        raise ValueError("English locale data missing from CLDR archive.")

    _, english_names = english_entry
    english_scripts = load_script_names(languages_root, "en")
    english_territories = load_territory_names(languages_root, "en")

    locale_cache: dict[str, tuple[LocaleIdentity, dict[str, str]] | None] = {
        "en": english_entry
    }

    def get_locale_entry(
        locale: str,
    ) -> tuple[LocaleIdentity, dict[str, str]] | None:
        canonical_tag = str(Locale.parse(locale))
        if canonical_tag not in locale_cache:
            locale_cache[canonical_tag] = load_locale_entry(
                languages_root, canonical_tag
            )
        return locale_cache[canonical_tag]

    entries: dict[str, str] = {}

    for locale in sorted(available_locales, key=lambda loc: str(Locale.parse(loc))):
        original_locale = Locale.parse(locale)
        expanded_locale = expand_locale(locale, likely_subtags)

        keys = candidate_language_keys(original_locale, expanded_locale)
        autonym: str | None = None

        for fallback in fallback_chain(locale):
            entry = get_locale_entry(fallback)
            if not entry:
                continue
            _, names = entry
            for key in keys:
                value = names.get(key)
                if value:
                    autonym = value
                    break
            if autonym:
                break

        if not autonym:
            for key in keys:
                value = english_names.get(key)
                if value:
                    autonym = value
                    break

        if not autonym:
            base_name = english_names.get(
                expanded_locale.language, expanded_locale.language
            )
            qualifiers: list[str] = []
            if expanded_locale.script and expanded_locale.script in english_scripts:
                qualifiers.append(english_scripts[expanded_locale.script])
            if expanded_locale.region and expanded_locale.region in english_territories:
                qualifiers.append(english_territories[expanded_locale.region])
            autonym = (
                f"{base_name} ({', '.join(qualifiers)})" if qualifiers else base_name
            )

        normalized_locale_tag = format_locale(
            expanded_locale,
            original_locale,
            likely_subtags,
        )
        if normalized_locale_tag not in entries:
            entries[normalized_locale_tag] = autonym

    return entries
