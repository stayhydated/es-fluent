"""CLDR data loaders with Pydantic validation."""

from __future__ import annotations

from pathlib import Path

from .io import load_json
from .models import (
    AvailableLocalesData,
    LanguagesJsonMain,
    LikelySubtagsData,
    LocaleIdentity,
    ScriptsJsonMain,
    TerritoriesJsonMain,
)


def load_likely_subtags(cldr_root: Path) -> dict[str, str]:
    """Load and parse likelySubtags.json."""
    path = cldr_root / "cldr-core" / "supplemental" / "likelySubtags.json"
    data = LikelySubtagsData.model_validate(load_json(path))
    return data.likely_subtags


def load_available_locales(cldr_root: Path) -> list[str]:
    """Load and parse availableLocales.json."""
    path = cldr_root / "cldr-core" / "availableLocales.json"
    data = AvailableLocalesData.model_validate(load_json(path))
    return data.full


def load_locale_entry(
    languages_root: Path, locale: str
) -> tuple[LocaleIdentity, dict[str, str]] | None:
    """Load and parse a locale's languages.json."""
    path = languages_root / locale / "languages.json"
    if not path.is_file():
        return None
    data = LanguagesJsonMain.model_validate(load_json(path))
    entry = data.main[locale]
    return entry.identity, entry.locale_display_names.languages


def load_script_names(languages_root: Path, locale: str) -> dict[str, str]:
    """Load and parse a locale's scripts.json."""
    path = languages_root / locale / "scripts.json"
    data = ScriptsJsonMain.model_validate(load_json(path))
    return data.main[locale].locale_display_names.scripts


def load_territory_names(languages_root: Path, locale: str) -> dict[str, str]:
    """Load and parse a locale's territories.json."""
    path = languages_root / locale / "territories.json"
    data = TerritoriesJsonMain.model_validate(load_json(path))
    return data.main[locale].locale_display_names.territories
