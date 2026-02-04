"""Locale processing logic for CLDR data."""

from __future__ import annotations

from collections.abc import Callable
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


def merge_names_for_locale(
    languages_root: Path,
    locale: str,
    loader: Callable[
        [Path, str], dict[str, str] | tuple[LocaleIdentity, dict[str, str]] | None
    ],
) -> dict[str, str]:
    """Merge display names for a locale using its fallback chain."""
    merged: dict[str, str] = {}
    for fallback in fallback_chain(locale):
        try:
            data = loader(languages_root, fallback)
        except FileNotFoundError:
            continue
        if not data:
            continue
        if isinstance(data, tuple):
            _, names = data
        else:
            names = data
        for key, value in names.items():
            if key not in merged:
                merged[key] = value
    return merged


def collapse_entries(entries: dict[str, str]) -> dict[str, str]:
    """Preserve all locale entries without collapsing region variants.

    Previously this function would deduplicate entries where multiple region
    variants shared the same name (e.g., en-US, en-GB -> en). Now it preserves
    all entries to ensure region-specific locales like en-US are available.
    """
    # Return entries as-is without collapsing
    return dict(entries)


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


def collect_entries(
    cldr_root: Path, display_locale: str | None = None
) -> dict[str, str]:
    """Collect all locale entries from CLDR data.

    When display_locale is provided, language names are localized to that locale.
    Otherwise, autonyms (self-names) are used.
    """
    languages_root = cldr_root / "cldr-localenames-full" / "main"

    likely_subtags = load_likely_subtags(cldr_root)
    available_locales = load_available_locales(cldr_root)

    english_entry = load_locale_entry(languages_root, "en")
    if not english_entry:
        raise ValueError("English locale data missing from CLDR archive.")

    _, english_names = english_entry
    english_scripts = load_script_names(languages_root, "en")
    english_territories = load_territory_names(languages_root, "en")

    display_names: dict[str, str] | None = None
    display_scripts: dict[str, str] | None = None
    display_territories: dict[str, str] | None = None

    if display_locale:
        display_locale_tag = str(Locale.parse(display_locale))
        display_names = merge_names_for_locale(
            languages_root, display_locale_tag, load_locale_entry
        )
        if not display_names:
            raise ValueError(
                f"Display locale '{display_locale}' missing from CLDR archive."
            )
        display_scripts = merge_names_for_locale(
            languages_root, display_locale_tag, load_script_names
        )
        display_territories = merge_names_for_locale(
            languages_root, display_locale_tag, load_territory_names
        )

    script_names = display_scripts or english_scripts
    territory_names = display_territories or english_territories

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

    def build_entry(
        locale_tag: str, *, force_language_only: bool = False
    ) -> tuple[str, str, Locale, Locale]:
        original_locale = Locale.parse(locale_tag)
        expanded_locale = expand_locale(locale_tag, likely_subtags)

        keys = candidate_language_keys(original_locale, expanded_locale)
        autonym: str | None = None

        if display_names is not None:
            for key in keys:
                value = display_names.get(key)
                if value:
                    autonym = value
                    break
        else:
            for fallback in fallback_chain(locale_tag):
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
            if expanded_locale.script and expanded_locale.script in script_names:
                qualifiers.append(script_names[expanded_locale.script])
            if expanded_locale.region and expanded_locale.region in territory_names:
                qualifiers.append(territory_names[expanded_locale.region])
            autonym = (
                f"{base_name} ({', '.join(qualifiers)})" if qualifiers else base_name
            )

        if force_language_only:
            normalized_locale_tag = original_locale.language
        else:
            normalized_locale_tag = format_locale(
                expanded_locale,
                original_locale,
                likely_subtags,
            )

        return normalized_locale_tag, autonym, expanded_locale, original_locale

    # First pass: collect base language names (without region, or with region "001")
    base_language_names: dict[str, str] = {}

    entries: dict[str, str] = {}

    for locale in sorted(available_locales, key=lambda loc: str(Locale.parse(loc))):
        normalized_locale_tag, autonym, expanded_locale, original_locale = build_entry(
            locale
        )

        # Track base language names:
        # - Locales without region and without script
        # - Locales with region "001" (World) which serve as the base for that language
        is_base_locale = (
            original_locale.region is None and original_locale.script is None
        ) or original_locale.region == "001"
        if is_base_locale and expanded_locale.language not in base_language_names:
            base_language_names[expanded_locale.language] = autonym

        if normalized_locale_tag not in entries:
            entries[normalized_locale_tag] = autonym

    # Add ISO 639-1 base language tags (two-letter codes like "en").
    iso_639_1_languages: set[str] = set()
    for tag in english_names.keys():
        language = Locale.parse(tag).language
        if len(language) == 2 and language.isalpha():
            iso_639_1_languages.add(language)
    for language in sorted(iso_639_1_languages):
        if language in entries:
            continue
        normalized_locale_tag, autonym, expanded_locale, _ = build_entry(
            language, force_language_only=True
        )
        if normalized_locale_tag not in entries:
            entries[normalized_locale_tag] = autonym
        if expanded_locale.language not in base_language_names:
            base_language_names[expanded_locale.language] = autonym

    # Second pass: add region qualifiers and filter out unhelpful entries
    qualified_entries: dict[str, str] = {}
    for locale_tag, name in entries.items():
        parsed = Locale.parse(locale_tag)

        # Skip numeric region codes (UN M.49) that don't have distinct names
        # These are macro-regions like 001 (World), 150 (Europe), 419 (Latin America)
        # Keep them only if they have a distinct name from the base language
        if parsed.region and parsed.region.isdigit():
            base_name = base_language_names.get(parsed.language)
            if base_name and name == base_name:
                # Skip this entry - it's a numeric region with no distinct name
                continue

        # If this locale has a region and its name matches the base language name,
        # add a region qualifier
        if parsed.region and not parsed.region.isdigit():
            base_name = base_language_names.get(parsed.language)
            if base_name and name == base_name:
                territory_name = territory_names.get(parsed.region)
                if territory_name:
                    name = f"{name} ({territory_name})"

        qualified_entries[locale_tag] = name

    return qualified_entries
