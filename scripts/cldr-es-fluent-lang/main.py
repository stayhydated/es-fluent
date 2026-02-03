#!/usr/bin/env python3
"""CLI entrypoint for generating es-fluent-lang.ftl from CLDR data."""

from __future__ import annotations

import tempfile
from pathlib import Path
from typing import Annotated

import typer

from src.io import DownloadError, ExtractionError, download_file, extract_archive
from src.models import Locale
from src.processing import collapse_entries, collect_entries
from src.writers import write_ftl, write_supported_locales

CLDR_RELEASE = "48.0.0"
CLDR_ARCHIVE_NAME = f"cldr-{CLDR_RELEASE}-json-full.zip"
CLDR_URL = (
    "https://github.com/unicode-org/cldr-json/releases/download/"
    f"{CLDR_RELEASE}/{CLDR_ARCHIVE_NAME}"
)

__SCRIPT_DIR = Path(__file__).resolve().parent
__WORKSPACE_DIR = __SCRIPT_DIR.parent
CRATES_DIR = __WORKSPACE_DIR.parent / "crates"
CACHE_DIR = __SCRIPT_DIR / "cldr"
CLDR_CACHE_PATH = CACHE_DIR / CLDR_ARCHIVE_NAME

FTL_FILE_OUTPUT = CRATES_DIR / "es-fluent-lang" / "es-fluent-lang.ftl"
I18N_DIR = CRATES_DIR / "es-fluent-lang" / "i18n"
I18N_RESOURCE_NAME = "es-fluent-lang.ftl"
SUPPORTED_RS_OUTPUT = (
    CRATES_DIR / "es-fluent-lang-macro" / "src" / "supported_locales.rs"
)

app = typer.Typer(
    help="Generate es-fluent-lang language-name files from CLDR data.",
    add_completion=False,
)


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
    ] = FTL_FILE_OUTPUT,
    supported_output: Annotated[
        Path,
        typer.Option(
            "--supported-output",
            help="Destination Rust file containing supported language keys.",
            writable=True,
            resolve_path=True,
        ),
    ] = SUPPORTED_RS_OUTPUT,
    i18n_dir: Annotated[
        Path,
        typer.Option(
            "--i18n-dir",
            help="Destination directory for per-locale i18n files.",
            writable=True,
            resolve_path=True,
            file_okay=False,
            dir_okay=True,
        ),
    ] = I18N_DIR,
    all_locales: Annotated[
        bool,
        typer.Option(
            "--all-locales/--no-all-locales",
            help="Generate per-locale i18n files for every CLDR locale.",
        ),
    ] = True,
    display_locale: Annotated[
        str | None,
        typer.Option(
            "--display-locale",
            help="Locale used to translate language names (e.g., en, fr-CA). Defaults to autonyms when omitted.",
        ),
    ] = None,
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
    """Generate es-fluent-lang outputs from CLDR data."""
    written_locales = 0
    with tempfile.TemporaryDirectory() as tmp_dir:
        tmp_path = Path(tmp_dir)

        if cldr_zip:
            archive_path = cldr_zip
            typer.echo(f"Using existing CLDR archive: {archive_path}")
        else:
            if CLDR_CACHE_PATH.is_file():
                archive_path = CLDR_CACHE_PATH
                typer.echo(f"Using cached CLDR archive: {archive_path}")
            else:
                archive_path = CLDR_CACHE_PATH
                typer.echo(f"Downloading {CLDR_URL}...")
                try:
                    download_file(CLDR_URL, archive_path)
                except DownloadError as e:
                    typer.secho(str(e), fg=typer.colors.RED, err=True)
                    raise typer.Exit(code=1)

        typer.echo(f"Extracting {archive_path.name}...")
        extract_dir = tmp_path / "cldr"
        try:
            extract_archive(archive_path, extract_dir)
        except ExtractionError as e:
            typer.secho(str(e), fg=typer.colors.RED, err=True)
            raise typer.Exit(code=1)

        localenames_root = extract_dir / "cldr-localenames-full" / "main"
        if not localenames_root.is_dir():
            typer.secho(
                "Error: CLDR localenames data missing from archive.",
                fg=typer.colors.RED,
                err=True,
            )
            raise typer.Exit(code=1)
        localename_locales = sorted(
            {
                entry.name
                for entry in localenames_root.iterdir()
                if entry.is_dir() and (entry / "languages.json").is_file()
            }
        )

        if display_locale:
            typer.echo(
                f"Collecting locale entries (display locale: {display_locale})..."
            )
        else:
            typer.echo("Collecting locale entries (autonyms)...")
        try:
            raw_entries = collect_entries(extract_dir, display_locale=display_locale)
        except ValueError as e:
            typer.secho(f"Error: {e}", fg=typer.colors.RED, err=True)
            raise typer.Exit(code=1)
        typer.echo(f"Collapsing {len(raw_entries)} entries...")
        collapsed_entries = collapse_entries(raw_entries)
        sorted_entries = sorted(collapsed_entries.items())

        typer.echo(f"Writing {len(sorted_entries)} locales to {output}...")
        write_ftl(output, sorted_entries)
        supported_locales = [locale for locale, _ in sorted_entries]
        typer.echo(
            f"Writing {len(supported_locales)} supported locales to {supported_output}..."
        )
        write_supported_locales(supported_output, supported_locales)

        if display_locale:
            display_locales = [display_locale]
        elif all_locales:
            display_locales = localename_locales
        else:
            display_locales = []

        if display_locales:
            typer.echo(f"Writing per-locale i18n files to {i18n_dir}...")
            seen_locales: set[str] = set()
            for locale in display_locales:
                canonical_locale = str(Locale.parse(locale))
                if canonical_locale == "root" or canonical_locale in seen_locales:
                    continue
                seen_locales.add(canonical_locale)
                typer.echo(
                    f"Collecting locale entries (display locale: {canonical_locale})..."
                )
                try:
                    localized_entries = collect_entries(
                        extract_dir, display_locale=canonical_locale
                    )
                except ValueError as e:
                    typer.secho(f"Error: {e}", fg=typer.colors.RED, err=True)
                    raise typer.Exit(code=1)
                collapsed_localized = collapse_entries(localized_entries)
                sorted_localized = sorted(collapsed_localized.items())
                output_path = i18n_dir / canonical_locale / I18N_RESOURCE_NAME
                typer.echo(
                    f"Writing {len(sorted_localized)} locales to {output_path}..."
                )
                write_ftl(output_path, sorted_localized)
                written_locales += 1

    success_message = f"\nSuccessfully wrote {len(sorted_entries)} locales to {output}"
    if written_locales:
        success_message += f" and {written_locales} i18n locales to {i18n_dir}"
    typer.secho(success_message, fg=typer.colors.GREEN, bold=True)


if __name__ == "__main__":
    app()
