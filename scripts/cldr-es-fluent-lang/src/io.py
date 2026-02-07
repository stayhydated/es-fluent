"""File I/O helpers for downloading and extracting archives."""

from __future__ import annotations

import json
import zipfile
from pathlib import Path

import requests
from tqdm import tqdm

DEFAULT_USER_AGENT = "es-fluent-cldr-updater/1.0"
DEFAULT_TIMEOUT_SECONDS = 60


class DownloadError(Exception):
    """Raised when a file download fails."""


class ExtractionError(Exception):
    """Raised when archive extraction fails."""


def download_file(url: str, destination: Path) -> None:
    """Download a file from URL with progress bar.

    Args:
        url: URL to download from.
        destination: Local path to save the file.

    Raises:
        DownloadError: If the download fails.
    """
    destination.parent.mkdir(parents=True, exist_ok=True)
    try:
        with requests.get(
            url,
            stream=True,
            timeout=DEFAULT_TIMEOUT_SECONDS,
            headers={"User-Agent": DEFAULT_USER_AGENT},
        ) as response:
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
        raise DownloadError(f"Error downloading file: {e}") from e


def extract_archive(archive_path: Path, destination: Path) -> None:
    """Extract a ZIP archive with progress bar.

    Args:
        archive_path: Path to the ZIP archive.
        destination: Directory to extract to.

    Raises:
        ExtractionError: If extraction fails.
    """
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
    except zipfile.BadZipFile as e:
        raise ExtractionError(
            f"Failed to open zip file '{archive_path}'. It may be corrupted."
        ) from e
    except Exception as e:
        raise ExtractionError(f"Error extracting archive: {e}") from e


def load_json(path: Path) -> dict:
    """Load JSON file from disk."""
    with path.open(encoding="utf-8") as handle:
        return json.load(handle)
