"""Install ``es-fluent-cli`` for the ``es-fluent-cli`` GitHub Action.

The action passes its inputs through the environment. This module resolves the
requested version and builds the install command, preferring a prebuilt
``cargo-es-fluent`` binary from cargo-binstall over compiling from source.
"""

from __future__ import annotations

import os
import shutil
import subprocess

PACKAGE = "es-fluent-cli"
TAG_PREFIX = "es-fluent-cli-v"


def resolve_version(version_input: str, action_ref: str) -> str | None:
    """Resolve the version to install.

    Returns ``None`` when the CLI should be built from the pinned action ref.

    An explicit ``version`` input wins. Otherwise a release-tag action ref such
    as ``es-fluent-cli-v0.20.3`` selects that version. Any other ref (branch or
    commit SHA) builds from source.
    """
    version = version_input.strip()
    if version:
        return version
    ref = action_ref.strip()
    if ref.startswith(TAG_PREFIX):
        pinned = ref[len(TAG_PREFIX) :]
        if pinned:
            return pinned
    return None


def install_command(
    version: str | None, *, has_binstall: bool, action_path: str
) -> list[str]:
    """Build the command that installs ``es-fluent-cli``."""
    if version is None:
        return ["cargo", "install", "--path", action_path, "--locked"]
    if has_binstall:
        command = ["cargo", "binstall", PACKAGE, "--locked", "--no-confirm"]
    else:
        command = ["cargo", "install", PACKAGE, "--locked"]
    if version != "latest":
        command += ["--version", version]
    return command


def main() -> int:
    """Run the install command selected from the environment."""
    version = resolve_version(
        os.environ.get("ES_FLUENT_CLI_VERSION", ""),
        os.environ.get("ES_FLUENT_ACTION_REF", ""),
    )
    command = install_command(
        version,
        has_binstall=shutil.which("cargo-binstall") is not None,
        action_path=os.environ.get("GITHUB_ACTION_PATH") or os.getcwd(),
    )
    print("+ " + " ".join(command), flush=True)
    return subprocess.call(command)


if __name__ == "__main__":
    raise SystemExit(main())
