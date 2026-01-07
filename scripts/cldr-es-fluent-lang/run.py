#!/usr/bin/env python3
"""CLI entrypoint for generating es-fluent-lang.ftl from CLDR data."""

from __future__ import annotations

import tempfile
from pathlib import Path
from typing import Annotated

import typer
from src.io import DownloadError, ExtractionError, download_file, extract_archive
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

FTL_FILE_OUTPUT = CRATES_DIR / "es-fluent-lang" / "es-fluent-lang.ftl"
SUPPORTED_RS_OUTPUT = (
    CRATES_DIR / "es-fluent-lang-macro" / "src" / "supported_locales.rs"
)

app = typer.Typer(
    help="Generate es-fluent-lang.ftl from CLDR data.",
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
    """Generate es-fluent-lang.ftl and supported_locales.rs from CLDR data."""
    with tempfile.TemporaryDirectory() as tmp_dir:
        tmp_path = Path(tmp_dir)

        if cldr_zip:
            archive_path = cldr_zip
            typer.echo(f"Using existing CLDR archive: {archive_path}")
        else:
            archive_path = tmp_path / CLDR_ARCHIVE_NAME
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

        typer.echo("Collecting locale entries...")
        try:
            raw_entries = collect_entries(extract_dir)
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

    typer.secho(
        f"\nSuccessfully wrote {len(sorted_entries)} locales to {output}",
        fg=typer.colors.GREEN,
        bold=True,
    )


if __name__ == "__main__":
    app()
