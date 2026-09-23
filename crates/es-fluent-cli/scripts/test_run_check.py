from __future__ import annotations

import pytest

import run_check


@pytest.mark.parametrize(
    ("value", "expected"),
    [("true", True), ("false", False), ("", False)],
)
def test_parse_bool(value, expected):
    assert run_check.parse_bool("force_run", value) is expected


@pytest.mark.parametrize("value", ["True", "yes", "1", " true", "false "])
def test_parse_bool_rejects(value):
    with pytest.raises(run_check.ActionInputError, match="must be 'true' or 'false'"):
        run_check.parse_bool("force_run", value)


@pytest.mark.parametrize(
    ("value", "expected"),
    [
        ("", ""),
        ("  ", ""),
        ("a", "a"),
        ("a,b", "a,b"),
        ("a\nb", "a,b"),
        (" a , b \n\n c ", "a,b,c"),
        (",a,,b,", "a,b"),
    ],
)
def test_normalize_ignore(value, expected):
    assert run_check.normalize_ignore(value) == expected


def test_check_args_minimal():
    assert run_check.check_args(
        path=".",
        package="",
        all_locales="false",
        ignore="",
        no_fallback_copy_check="false",
        force_run="false",
    ) == ["--path", "."]


def test_check_args_full():
    assert run_check.check_args(
        path=".",
        package="pkg",
        all_locales="true",
        ignore=" a ,\n b ",
        no_fallback_copy_check="true",
        force_run="true",
    ) == [
        "--path",
        ".",
        "--package",
        "pkg",
        "--all-locales",
        "--ignore",
        "a,b",
        "--no-fallback-copy-check",
        "--force-run",
    ]


def test_check_args_omits_empty_values():
    assert (
        run_check.check_args(
            path="",
            package="",
            all_locales="",
            ignore="",
            no_fallback_copy_check="",
            force_run="",
        )
        == []
    )


def test_check_args_rejects_invalid_bool():
    with pytest.raises(run_check.ActionInputError):
        run_check.check_args(
            path="",
            package="",
            all_locales="maybe",
            ignore="",
            no_fallback_copy_check="false",
            force_run="false",
        )
