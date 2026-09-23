from __future__ import annotations

import pytest

import install_cli


@pytest.mark.parametrize(
    ("version_input", "action_ref", "expected"),
    [
        ("", "", None),
        ("", "master", None),
        ("", "abc1234", None),
        ("", "es-fluent-cli-v", None),
        ("", "es-fluent-cli-v0.20.3", "0.20.3"),
        ("0.20.3", "", "0.20.3"),
        ("0.20.3", "master", "0.20.3"),
        ("latest", "", "latest"),
        (" latest ", "master", "latest"),
    ],
)
def test_resolve_version(version_input, action_ref, expected):
    assert install_cli.resolve_version(version_input, action_ref) == expected


@pytest.mark.parametrize(
    ("version", "has_binstall", "expected"),
    [
        (
            None,
            True,
            ["cargo", "install", "--path", "/action", "--locked"],
        ),
        (
            None,
            False,
            ["cargo", "install", "--path", "/action", "--locked"],
        ),
        (
            "latest",
            True,
            ["cargo", "binstall", "es-fluent-cli", "--locked", "--no-confirm"],
        ),
        (
            "latest",
            False,
            ["cargo", "install", "es-fluent-cli", "--locked"],
        ),
        (
            "0.20.3",
            True,
            [
                "cargo",
                "binstall",
                "es-fluent-cli",
                "--locked",
                "--no-confirm",
                "--version",
                "0.20.3",
            ],
        ),
        (
            "0.20.3",
            False,
            ["cargo", "install", "es-fluent-cli", "--locked", "--version", "0.20.3"],
        ),
    ],
)
def test_install_command(version, has_binstall, expected):
    assert (
        install_cli.install_command(
            version, has_binstall=has_binstall, action_path="/action"
        )
        == expected
    )
