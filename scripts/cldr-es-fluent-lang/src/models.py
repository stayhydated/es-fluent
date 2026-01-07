"""Pydantic models for CLDR JSON structures and locale representation."""

from __future__ import annotations

from pydantic import BaseModel, Field


class Locale(BaseModel, frozen=True):
    """BCP-47 locale representation."""

    language: str
    script: str | None = None
    region: str | None = None
    variants: tuple[str, ...] = ()

    @classmethod
    def parse(cls, tag: str) -> Locale:
        """Parse a BCP-47 language tag into a Locale."""
        subtags = tag.replace("_", "-").split("-")
        language = subtags[0].lower()
        script: str | None = None
        region: str | None = None
        variants_list: list[str] = []
        for subtag in subtags[1:]:
            if len(subtag) == 4 and subtag.isalpha():
                script = subtag.title()
            elif (len(subtag) == 2 and subtag.isalpha()) or (
                len(subtag) == 3 and subtag.isdigit()
            ):
                region = subtag.upper()
            elif subtag:
                variants_list.append(subtag.lower())
        return cls(
            language=language,
            script=script,
            region=region,
            variants=tuple(variants_list),
        )

    def __str__(self) -> str:
        parts: list[str] = [self.language]
        if self.script:
            parts.append(self.script)
        if self.region:
            parts.append(self.region)
        parts.extend(self.variants)
        return "-".join(parts)

    def __hash__(self) -> int:
        return hash((self.language, self.script, self.region, self.variants))


class LikelySubtagsData(BaseModel):
    """Model for likelySubtags.json."""

    supplemental: dict[str, dict[str, str]]

    @property
    def likely_subtags(self) -> dict[str, str]:
        return self.supplemental["likelySubtags"]


class AvailableLocalesData(BaseModel):
    """Model for availableLocales.json."""

    available_locales: dict[str, list[str]] = Field(alias="availableLocales")

    @property
    def full(self) -> list[str]:
        return self.available_locales["full"]


class LocaleIdentity(BaseModel):
    """Identity block in locale data."""

    language: str
    script: str | None = None
    territory: str | None = None


class LocaleDisplayNames(BaseModel):
    """Locale display names block."""

    languages: dict[str, str]


class LocaleEntry(BaseModel):
    """Entry for a single locale in languages.json."""

    identity: LocaleIdentity
    locale_display_names: LocaleDisplayNames = Field(alias="localeDisplayNames")


class LanguagesJsonMain(BaseModel):
    """Main block in languages.json."""

    main: dict[str, LocaleEntry]


class ScriptDisplayNames(BaseModel):
    """Script display names block."""

    scripts: dict[str, str]


class ScriptsLocaleEntry(BaseModel):
    """Entry for a single locale in scripts.json."""

    locale_display_names: ScriptDisplayNames = Field(alias="localeDisplayNames")


class ScriptsJsonMain(BaseModel):
    """Main block in scripts.json."""

    main: dict[str, ScriptsLocaleEntry]


class TerritoryDisplayNames(BaseModel):
    """Territory display names block."""

    territories: dict[str, str]


class TerritoriesLocaleEntry(BaseModel):
    """Entry for a single locale in territories.json."""

    locale_display_names: TerritoryDisplayNames = Field(alias="localeDisplayNames")


class TerritoriesJsonMain(BaseModel):
    """Main block in territories.json."""

    main: dict[str, TerritoriesLocaleEntry]
