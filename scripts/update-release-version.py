from __future__ import annotations

import re
import tomllib
from pathlib import Path


def update_versions(root: Path) -> str:
    """Copy Rooster's new changelog version into the workspace manifest."""
    heading = re.search(
        r"^## (\d+\.\d+\.\d+)\s*$", (root / "CHANGELOG.md").read_text(), re.M
    )
    if heading is None:
        raise ValueError("Missing stable release version in CHANGELOG.md")
    version = heading[1]
    manifest = root / "Cargo.toml"
    content = manifest.read_text()
    old = tomllib.loads(content)["workspace"]["package"]["version"]
    content, count = re.subn(
        rf'^version = "{re.escape(old)}"$',
        f'version = "{version}"',
        content,
        count=1,
        flags=re.M,
    )
    if count != 1:
        raise ValueError("Could not update version in Cargo.toml")
    manifest.write_text(content)
    return version


if __name__ == "__main__":
    print(update_versions(Path(__file__).resolve().parent.parent))
