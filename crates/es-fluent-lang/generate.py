from __future__ import annotations
import json
import tempfile
import zipfile
from collections import defaultdict
from collections.abc import Iterable
from dataclasses import dataclass
from pathlib import Path
from typing import Annotated

import requests
import typer
from tqdm import tqdm

CLDR_RELEASE = "47.0.0"
CLDR_ARCHIVE_NAME = f"cldr-{CLDR_RELEASE}-json-full.zip"
CLDR_URL = (
    "https://github.com/unicode-org/cldr-json/releases/download/"
    f"{CLDR_RELEASE}/{CLDR_ARCHIVE_NAME}"
)
SCRIPT_DIR = Path(__file__).resolve().parent
DEFAULT_OUTPUT = SCRIPT_DIR / "es-fluent-lang.ftl"

app = typer.Typer(
    help="Generate es-fluent-lang.ftl from CLDR data.",
    context_settings={"help_option_names": ["-h", "--help"]},
)


@dataclass(frozen=True, slots=True)
class Locale:
    language: str
    script: str | None = None
    region: str | None = None
    variants: tuple[str, ...] = ()

    @classmethod
    def parse(cls, tag: str) -> Locale:
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
        return cls(language, script, region, tuple(variants_list))

    def __str__(self) -> str:
        parts: list[str] = [self.language]
        if self.script:
            parts.append(self.script)
        if self.region:
            parts.append(self.region)
        parts.extend(self.variants)
        return "-".join(parts)


def download_file(url: str, destination: Path) -> None:
    destination.parent.mkdir(parents=True, exist_ok=True)
    try:
        with requests.get(url, stream=True) as response:
            response.raise_for_status()
            total_size = int(response.headers.get("content-length", 0))

            with (
                destination.open("wb") as output,
                tqdm(
                    desc=f"Downloading {destination.name}",
                    total=total_size,
                    unit="iB",
                    unit_scale=True,
                    unit_divisor=1024,
                ) as bar,
            ):
                for chunk in response.iter_content(chunk_size=8192):
                    size = output.write(chunk)
                    bar.update(size)

    except requests.RequestException as e:
        typer.secho(f"Error downloading file: {e}", fg=typer.colors.RED, err=True)
        raise typer.Exit(code=1)


def extract_archive(archive_path: Path, destination: Path) -> None:
    try:
        with zipfile.ZipFile(archive_path) as archive:
            member_list = archive.infolist()
            with tqdm(
                total=len(member_list),
                desc=f"Extracting {archive_path.name}",
                unit="file",
            ) as bar:
                for member in member_list:
                    archive.extract(member, destination)
                    bar.update(1)
    except zipfile.BadZipFile:
        typer.secho(
            f"Error: Failed to open zip file '{archive_path}'. It may be corrupted.",
            fg=typer.colors.RED,
            err=True,
        )
        raise typer.Exit(code=1)
    except Exception as e:
        typer.secho(f"Error extracting archive: {e}", fg=typer.colors.RED, err=True)
        raise typer.Exit(code=1)


def load_json(path: Path) -> dict:
    with path.open(encoding="utf-8") as handle:
        return json.load(handle)


def load_locale_entry(
    languages_root: Path, locale: str
) -> tuple[dict, dict[str, str]] | None:
    path = languages_root / locale / "languages.json"
    if not path.is_file():
        return None
    data = load_json(path)
    entry = data["main"][locale]
    identity = entry["identity"]
    names = entry["localeDisplayNames"]["languages"]
    return identity, names


def load_script_names(languages_root: Path, locale: str) -> dict[str, str]:
    data = load_json(languages_root / locale / "scripts.json")
    return data["main"][locale]["localeDisplayNames"]["scripts"]


def load_territory_names(languages_root: Path, locale: str) -> dict[str, str]:
    data = load_json(languages_root / locale / "territories.json")
    return data["main"][locale]["localeDisplayNames"]["territories"]


def expand_locale(tag: str, likely_subtags: dict[str, str]) -> Locale:
    canonical_tag = str(Locale.parse(tag))
    expanded_tag = likely_subtags.get(canonical_tag)
    if expanded_tag:
        return Locale.parse(expanded_tag)
    return Locale.parse(canonical_tag)


def fallback_chain(locale: str) -> list[str]:
    canonical_tag = str(Locale.parse(locale))
    parts = canonical_tag.split("-")
    chain = ["-".join(parts[:i]) for i in range(len(parts), 0, -1)]
    chain.append("root")
    return chain


def candidate_language_keys(
    original_locale: Locale, expanded_locale: Locale
) -> list[str]:
    keys: list[str] = []
    for key_tag in (
        str(original_locale),
        str(expanded_locale),
        # lang-script
        str(Locale(expanded_locale.language, expanded_locale.script, None, ())),
        # lang-region
        str(Locale(expanded_locale.language, None, expanded_locale.region, ())),
        expanded_locale.language,
    ):
        if key_tag and key_tag not in keys:
            keys.append(key_tag)
    return keys


def collapse_entries(entries: dict[str, str]) -> dict[str, str]:
    grouped: dict[Locale, dict[str, list[str]]] = defaultdict(lambda: defaultdict(list))
    locale_parts: dict[str, Locale] = {}
    for locale_tag, name in entries.items():
        parts = Locale.parse(locale_tag)
        locale_parts[locale_tag] = parts
        # Group by lang, script, and variants (ignoring region)
        key_locale = Locale(parts.language, parts.script, None, parts.variants)
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

    return str(Locale(language, script, region, variants))


def escape_fluent_value(value: str) -> str:
    return value.replace("{", "{{").replace("}", "}}")


def collect_entries(cldr_root: Path) -> dict[str, str]:
    likely_path = cldr_root / "cldr-core" / "supplemental" / "likelySubtags.json"
    available_path = cldr_root / "cldr-core" / "availableLocales.json"
    languages_root = cldr_root / "cldr-localenames-full" / "main"

    likely_subtags = load_json(likely_path)["supplemental"]["likelySubtags"]
    available_locales = load_json(available_path)["availableLocales"]["full"]

    english_entry = load_locale_entry(languages_root, "en")
    if not english_entry:
        typer.secho(
            "Error: English locale data missing from CLDR archive.",
            fg=typer.colors.RED,
            err=True,
        )
        raise typer.Exit(code=1)

    _, english_names = english_entry
    english_scripts = load_script_names(languages_root, "en")
    english_territories = load_territory_names(languages_root, "en")

    locale_cache: dict[str, tuple[dict, dict[str, str]] | None] = {"en": english_entry}

    def get_locale_entry(locale: str) -> tuple[dict, dict[str, str]] | None:
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


def write_ftl(output_path: Path, entries: Iterable[tuple[str, str]]) -> None:
    output_path.parent.mkdir(parents=True, exist_ok=True)
    lines = [
        f"es-fluent-lang-{locale} = {escape_fluent_value(name)}"
        for locale, name in entries
    ]
    _ = output_path.write_text("\n".join(lines) + "\n", encoding="utf-8")


def validate_i18n_dirs(
    valid_locales: Iterable[str], directories: Iterable[Path]
) -> None:
    valid = {str(Locale.parse(locale)) for locale in valid_locales}
    errors: list[str] = []

    for directory in directories:
        if not directory.exists():
            errors.append(f"I18n directory '{directory}' does not exist.")
            continue
        if not directory.is_dir():
            errors.append(f"I18n path '{directory}' is not a directory.")
            continue

        invalid_children = sorted(
            entry.name
            for entry in directory.iterdir()
            if entry.is_dir() and str(Locale.parse(entry.name)) not in valid
        )

        if invalid_children:
            formatted = ", ".join(invalid_children)
            errors.append(
                f"Invalid locale folder(s) in '{directory}': {formatted}. Update the folder name(s) to match a supported locale."
            )

    if errors:
        typer.secho(
            "Validation failed with the following errors:",
            fg=typer.colors.RED,
            err=True,
        )
        typer.secho("\n".join(errors), fg=typer.colors.RED, err=True)
        raise typer.Exit(code=1)


@app.command()
def main(
    output: Annotated[
        Path,
        typer.Option(
            "--output",
            "-o",
            help="Destination FTL file.",
            writable=True,
            resolve_path=True,
        ),
    ] = DEFAULT_OUTPUT,
    cldr_zip: Annotated[
        Path | None,
        typer.Option(
            "--cldr-zip",
            help="Path to an existing CLDR archive. If missing, the archive will be downloaded to a temporary path.",
            exists=True,
            file_okay=True,
            dir_okay=False,
            readable=True,
            resolve_path=True,
        ),
    ] = None,
) -> None:
    with tempfile.TemporaryDirectory() as tmp_dir:
        tmp_path = Path(tmp_dir)

        if cldr_zip:
            archive_path = cldr_zip
            typer.echo(f"Using existing CLDR archive: {archive_path}")
        else:
            archive_path = tmp_path / CLDR_ARCHIVE_NAME
            typer.echo(f"Downloading {CLDR_URL}...")
            download_file(CLDR_URL, archive_path)

        typer.echo(f"Extracting {archive_path.name}...")
        extract_dir = tmp_path / "cldr"
        extract_archive(archive_path, extract_dir)

        typer.echo("Collecting locale entries...")
        raw_entries = collect_entries(extract_dir)
        typer.echo(f"Collapsing {len(raw_entries)} entries...")
        collapsed_entries = collapse_entries(raw_entries)
        sorted_entries = sorted(collapsed_entries.items())

        typer.echo(f"Writing {len(sorted_entries)} locales to {output}...")
        write_ftl(output, sorted_entries)

    typer.secho(
        f"\nSuccessfully wrote {len(sorted_entries)} locales to {output}",
        fg=typer.colors.GREEN,
        bold=True,
    )


if __name__ == "__main__":
    app()
