"""Build and run ``cargo es-fluent check`` for the es-fluent-cli GitHub Action.

The action passes its inputs through the environment. This module validates
boolean inputs, normalizes the ignore list, and runs the CLI.
"""

from __future__ import annotations

import os
import re
import subprocess
import sys

IGNORE_SEPARATOR = re.compile(r"[,\n]")


class ActionInputError(ValueError):
    """A boolean action input was not ``true`` or ``false``."""


def parse_bool(name: str, value: str) -> bool:
    """Return the boolean value of an action input, rejecting anything else."""
    if value == "true":
        return True
    if value in ("", "false"):
        return False
    raise ActionInputError(f"input '{name}' must be 'true' or 'false' (got '{value}')")


def normalize_ignore(value: str) -> str:
    """Return the comma-joined, trimmed, non-empty entries of ``value``."""
    entries = (entry.strip() for entry in IGNORE_SEPARATOR.split(value))
    return ",".join(entry for entry in entries if entry)


def check_args(
    *,
    path: str,
    package: str,
    all_locales: str,
    ignore: str,
    no_fallback_copy_check: str,
    force_run: str,
) -> list[str]:
    """Build the ``cargo es-fluent check`` arguments from action inputs."""
    args: list[str] = []
    if path:
        args += ["--path", path]
    if package:
        args += ["--package", package]
    if parse_bool("all_locales", all_locales):
        args.append("--all-locales")
    ignore_value = normalize_ignore(ignore)
    if ignore_value:
        args += ["--ignore", ignore_value]
    if parse_bool("no_fallback_copy_check", no_fallback_copy_check):
        args.append("--no-fallback-copy-check")
    if parse_bool("force_run", force_run):
        args.append("--force-run")
    return args


def main() -> int:
    """Validate inputs, then run ``cargo es-fluent check``."""
    try:
        args = check_args(
            path=os.environ.get("ES_FLUENT_PATH", ""),
            package=os.environ.get("ES_FLUENT_PACKAGE", ""),
            all_locales=os.environ.get("ES_FLUENT_ALL_LOCALES", ""),
            ignore=os.environ.get("ES_FLUENT_IGNORE", ""),
            no_fallback_copy_check=os.environ.get(
                "ES_FLUENT_NO_FALLBACK_COPY_CHECK", ""
            ),
            force_run=os.environ.get("ES_FLUENT_FORCE_RUN", ""),
        )
    except ActionInputError as error:
        print(f"::error::{error}", file=sys.stderr)
        return 1
    command = ["cargo", "es-fluent", "check", *args]
    print("+ " + " ".join(command), flush=True)
    return subprocess.call(command)


if __name__ == "__main__":
    raise SystemExit(main())
