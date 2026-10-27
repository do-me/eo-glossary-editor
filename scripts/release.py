"""Validate immutable release versions and the complete desktop package matrix."""
import json
import re
import sys
import tomllib
from pathlib import Path


def validate(tag: str) -> None:
    if not re.fullmatch(r"v\d+\.\d+\.\d+", tag):
        raise SystemExit("Run the release workflow from a vMAJOR.MINOR.PATCH tag.")
    versions = {
        json.loads(Path("package.json").read_text())["version"],
        json.loads(Path("src-tauri/tauri.conf.json").read_text())["version"],
        tomllib.loads(Path("Cargo.toml").read_text())["workspace"]["package"]["version"],
    }
    if versions != {tag[1:]}:
        raise SystemExit(f"Tag {tag} disagrees with application versions: {versions}")
    if not Path(f"docs/releases/{tag}.md").is_file():
        raise SystemExit("The release needs a saved release note.")
    print(f"Validated release version {tag}.")


def assets(directory: str) -> None:
    packages = Path(directory)
    required = [
        "*_macos_aarch64.dmg", "*_macos_x64.dmg",
        "*_windows_x64-setup.exe", "*_windows_x64.msi",
        "*_linux_*.deb", "*_linux_*.rpm", "*_linux_*.AppImage",
    ]
    for pattern in required:
        matches = list(packages.glob(pattern))
        if len(matches) != 1 or matches[0].stat().st_size < 100_000:
            raise SystemExit(f"Missing, ambiguous, or empty release package: {pattern}")
    print(f"Verified {len(required)} required installers across all desktop platforms.")


if __name__ == "__main__":
    if len(sys.argv) != 3 or sys.argv[1] not in {"validate", "assets"}:
        raise SystemExit("Usage: release.py validate TAG | assets DIRECTORY")
    {"validate": validate, "assets": assets}[sys.argv[1]](sys.argv[2])
