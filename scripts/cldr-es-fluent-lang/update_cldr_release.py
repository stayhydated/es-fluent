#!/usr/bin/env python3
"""Check CLDR releases, bump `main.py` CLDR_RELEASE, and run `main.py`."""

from __future__ import annotations

import json
import re
import subprocess
import sys
import urllib.error
import urllib.request
from pathlib import Path
from typing import Annotated

import typer

CLDR_RELEASES_API_URL = (
    "https://api.github.com/repos/unicode-org/cldr-json/releases/latest"
)
CLDR_RELEASE_PATTERN = re.compile(
    r'^(?P<prefix>\s*CLDR_RELEASE\s*=\s*")(?P<version>[^"]+)(".*)$',
    re.MULTILINE,
)
SCRIPT_DIR = Path(__file__).resolve().parent
DEFAULT_MAIN_PY = SCRIPT_DIR / "main.py"
DEFAULT_REPO_ROOT = SCRIPT_DIR.parent.parent
DEFAULT_COMMIT_PATHS = (
    "scripts/cldr-es-fluent-lang/main.py",
    "crates/es-fluent-lang/es-fluent-lang.ftl",
    "crates/es-fluent-lang/i18n",
    "crates/es-fluent-lang-macro/src/supported_locales.rs",
)

app = typer.Typer(
    help=(
        "Check unicode-org/cldr-json for new releases and optionally update "
        "CLDR_RELEASE in scripts/cldr-es-fluent-lang/main.py, then run "
        "scripts/cldr-es-fluent-lang/main.py to regenerate outputs."
    ),
    add_completion=False,
)


def fetch_latest_release(timeout_seconds: int = 30) -> str:
    request = urllib.request.Request(
        CLDR_RELEASES_API_URL,
        headers={
            "Accept": "application/vnd.github+json",
            "User-Agent": "es-fluent-cldr-updater",
        },
    )
    try:
        with urllib.request.urlopen(request, timeout=timeout_seconds) as response:
            payload = json.loads(response.read().decode("utf-8"))
    except (urllib.error.URLError, TimeoutError) as error:
        raise RuntimeError(
            f"Failed to fetch CLDR releases from {CLDR_RELEASES_API_URL}: {error}"
        ) from error

    tag_name = payload.get("tag_name")
    if not isinstance(tag_name, str) or not tag_name.strip():
        raise RuntimeError(
            "GitHub releases API response did not include a valid 'tag_name'."
        )
    return tag_name.strip()


def read_current_release(main_py_path: Path) -> str:
    content = main_py_path.read_text(encoding="utf-8")
    match = CLDR_RELEASE_PATTERN.search(content)
    if not match:
        raise RuntimeError(f"Could not find CLDR_RELEASE in {main_py_path}.")
    return match.group("version")


def update_release_in_file(main_py_path: Path, new_release: str) -> bool:
    content = main_py_path.read_text(encoding="utf-8")
    match = CLDR_RELEASE_PATTERN.search(content)
    if not match:
        raise RuntimeError(f"Could not find CLDR_RELEASE in {main_py_path}.")

    current_release = match.group("version")
    if current_release == new_release:
        return False

    updated_content = (
        content[: match.start("version")]
        + new_release
        + content[match.end("version") :]
    )
    main_py_path.write_text(updated_content, encoding="utf-8")
    return True


def run_generator(main_py_path: Path, repo_root: Path) -> None:
    command = [sys.executable, str(main_py_path)]
    subprocess.run(command, cwd=repo_root, check=True)


def run_git_command(repo_root: Path, *args: str) -> subprocess.CompletedProcess[str]:
    return subprocess.run(
        ["git", *args],
        cwd=repo_root,
        check=True,
        text=True,
        capture_output=True,
    )


def commit_generated_files(
    repo_root: Path,
    main_py_path: Path,
    latest_release: str,
    commit_message: str | None,
) -> bool:
    try:
        main_py_relative = str(main_py_path.resolve().relative_to(repo_root.resolve()))
    except ValueError as error:
        raise RuntimeError(
            "--main-py must be inside --repo-root when using --commit."
        ) from error

    candidate_paths = [main_py_relative, *DEFAULT_COMMIT_PATHS]
    existing_paths = [
        path for path in dict.fromkeys(candidate_paths) if (repo_root / path).exists()
    ]
    if not existing_paths:
        raise RuntimeError("No CLDR output paths were found to stage for commit.")

    try:
        run_git_command(repo_root, "add", "--", *existing_paths)
    except subprocess.CalledProcessError as error:
        stderr = (error.stderr or "").strip()
        raise RuntimeError(
            f"Failed to stage CLDR files. {stderr or 'Verify repository paths.'}"
        ) from error

    diff_result = subprocess.run(
        ["git", "diff", "--cached", "--quiet", "--exit-code"],
        cwd=repo_root,
        check=False,
    )
    if diff_result.returncode == 0:
        return False
    if diff_result.returncode != 1:
        raise RuntimeError("Failed to inspect staged changes.")

    message = commit_message or f"chore(cldr): update CLDR to {latest_release}"
    try:
        run_git_command(repo_root, "commit", "-m", message)
    except subprocess.CalledProcessError as error:
        stderr = (error.stderr or "").strip()
        raise RuntimeError(
            f"Failed to create git commit. {stderr or 'Check git user.name/user.email configuration.'}"
        ) from error
    return True


def write_github_outputs(path: Path, outputs: dict[str, str]) -> None:
    lines = [f"{key}={value}" for key, value in outputs.items()]
    with path.open("a", encoding="utf-8") as file:
        file.write("\n".join(lines))
        file.write("\n")


@app.command()
def main(
    main_py: Annotated[
        Path,
        typer.Option(
            "--main-py",
            help="Path to scripts/cldr-es-fluent-lang/main.py",
            exists=True,
            file_okay=True,
            dir_okay=False,
            readable=True,
            resolve_path=True,
        ),
    ] = DEFAULT_MAIN_PY,
    repo_root: Annotated[
        Path,
        typer.Option(
            "--repo-root",
            help="Repository root used as working directory for generator execution.",
            exists=True,
            file_okay=False,
            dir_okay=True,
            readable=True,
            resolve_path=True,
        ),
    ] = DEFAULT_REPO_ROOT,
    apply: Annotated[
        bool,
        typer.Option(
            "--apply",
            help="Apply the version update and run scripts/cldr-es-fluent-lang/main.py when a new release is found.",
        ),
    ] = False,
    skip_generate: Annotated[
        bool,
        typer.Option(
            "--skip-generate",
            help="With --apply, update CLDR_RELEASE but skip running scripts/cldr-es-fluent-lang/main.py.",
        ),
    ] = False,
    commit: Annotated[
        bool,
        typer.Option(
            "--commit",
            help="With --apply, create a git commit for updated CLDR files.",
        ),
    ] = False,
    commit_message: Annotated[
        str | None,
        typer.Option(
            "--commit-message",
            help="Custom commit message used with --commit.",
        ),
    ] = None,
    github_output: Annotated[
        Path | None,
        typer.Option(
            "--github-output",
            help="Optional GitHub Actions output file path (e.g., $GITHUB_OUTPUT).",
            writable=True,
            resolve_path=True,
        ),
    ] = None,
) -> None:
    """Check for a new CLDR release and optionally update via `main.py`."""
    try:
        main_py_path = main_py.resolve()
        repo_root_path = repo_root.resolve()

        current_release = read_current_release(main_py_path)
        latest_release = fetch_latest_release()
        update_available = latest_release != current_release
        updated = False
        committed = False

        if commit and not apply:
            raise RuntimeError("--commit requires --apply.")

        typer.echo(f"Current CLDR release: {current_release}")
        typer.echo(f"Latest CLDR release:  {latest_release}")

        if update_available:
            if apply:
                typer.echo(f"Updating CLDR_RELEASE in {main_py_path}...")
                updated = update_release_in_file(main_py_path, latest_release)
                if updated and not skip_generate:
                    typer.echo(f"Running {main_py_path}...")
                    run_generator(main_py_path, repo_root_path)
                if updated and commit:
                    typer.echo("Committing CLDR changes...")
                    committed = commit_generated_files(
                        repo_root_path,
                        main_py_path,
                        latest_release,
                        commit_message,
                    )
                    if committed:
                        typer.echo("Commit complete.")
                    else:
                        typer.echo("No staged changes to commit.")
                if updated:
                    typer.echo("Update complete.")
            else:
                typer.echo(
                    "A new CLDR release is available. Re-run with --apply to update."
                )
        else:
            typer.echo("CLDR is already up to date.")

        if github_output:
            write_github_outputs(
                github_output.resolve(),
                {
                    "current_release": current_release,
                    "latest_release": latest_release,
                    "update_available": str(update_available).lower(),
                    "updated": str(updated).lower(),
                    "committed": str(committed).lower(),
                },
            )
    except RuntimeError as error:
        typer.secho(f"Error: {error}", fg=typer.colors.RED, err=True)
        raise typer.Exit(code=1) from error


if __name__ == "__main__":
    app()
