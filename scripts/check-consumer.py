#!/usr/bin/env python3
"""Compile a standalone consumer using the workspace's locked dependency graph."""
import argparse
import json
import os
from pathlib import Path
import subprocess
import tempfile
import tomllib

parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument("package")
parser.add_argument("--features", default="")
parser.add_argument("--no-default-features", action="store_true")
args = parser.parse_args()
root = Path(__file__).resolve().parents[1]
package_path = root / "crates" / args.package
if not (package_path / "Cargo.toml").is_file():
    parser.error(f"unknown library package: {args.package}")
features = [feature for feature in args.features.split(",") if feature]
with tempfile.TemporaryDirectory(prefix="ci-consumer-") as temporary:
    consumer = Path(temporary)
    dependency = (
        f'path = {json.dumps(str(package_path))}, '
        f'default-features = {str(not args.no_default_features).lower()}, '
        f'features = {json.dumps(features)}'
    )
    (consumer / "Cargo.toml").write_text(
        '[package]\nname = "ci-feature-consumer"\nversion = "0.0.0"\n'
        'edition = "2024"\npublish = false\n\n[workspace]\n\n[dependencies]\n'
        f'{args.package} = {{ {dependency} }}\n'
    )
    # Seed the workspace versions, then let Cargo prune unused packages and
    # adjust dependency edges for this independent consumer. Refuse any new
    # version/source/checksum before compiling with the resulting lockfile.
    lock_text = (root / "Cargo.lock").read_text()
    (consumer / "Cargo.lock").write_text(lock_text)
    (consumer / "src").mkdir()
    (consumer / "src" / "lib.rs").write_text(
        f'pub use {args.package.replace("-", "_")}::*;\n'
    )
    environment = os.environ.copy()
    environment.setdefault("CARGO_TARGET_DIR", str(root / "target" / "ci-consumers"))
    subprocess.run(
        ["cargo", "metadata", "--manifest-path", str(consumer / "Cargo.toml"),
         "--format-version", "1"],
        env=environment,
        stdout=subprocess.DEVNULL,
        check=True,
    )
    def identity(package):
        return tuple(package.get(key) for key in ("name", "version", "source", "checksum"))

    locked = {identity(package) for package in tomllib.loads(lock_text)["package"]}
    selected = tomllib.loads((consumer / "Cargo.lock").read_text())["package"]
    changed = [identity(package) for package in selected
               if package["name"] != "ci-feature-consumer" and identity(package) not in locked]
    if changed:
        raise SystemExit(f"consumer resolution changed workspace dependency pins: {changed}")
    subprocess.run(
        ["cargo", "check", "--manifest-path", str(consumer / "Cargo.toml"), "--locked"],
        env=environment,
        check=True,
    )
