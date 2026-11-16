#!/usr/bin/env python3
"""Prints the GitHub release notes for a version: the opening paragraph of its
changelog entry, the one the checklist asks every entry to begin with, and a
link to the full entry at the release's tag.

    scripts/release_notes.py 0.8.0            # the notes
    scripts/release_notes.py 0.8.0 --whole    # the whole entry instead

Exits 1 when the changelog has no entry for the version, so a tag without an
entry cannot become a release. Needs only Python 3.
"""

import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
CHANGELOG = ROOT / "CHANGELOG.md"
REPOSITORY = "https://github.com/shergin/whichever"


def entry(version: str) -> tuple[str, str]:
    """The heading and the body of the version's changelog entry."""
    text = CHANGELOG.read_text()
    match = re.search(rf"^## {re.escape(version)}(?: - [0-9-]+)?$", text, re.M)
    if match is None:
        sys.exit(f"CHANGELOG.md has no entry for {version}")
    rest = text[match.end():]
    following = re.search(r"^## ", rest, re.M)
    body = rest[: following.start()] if following else rest
    return match.group(0), body.strip()


def anchor(heading: str) -> str:
    """The fragment GitHub gives a heading: lowercased, punctuation dropped,
    spaces as hyphens."""
    title = heading.lstrip("#").strip().lower()
    return re.sub(r"[^a-z0-9 -]", "", title).replace(" ", "-")


def main() -> None:
    arguments = [a for a in sys.argv[1:] if not a.startswith("--")]
    if len(arguments) != 1:
        sys.exit(__doc__.strip())
    version = arguments[0].lstrip("v")
    heading, body = entry(version)
    if "--whole" in sys.argv:
        print(body)
        return
    opening = re.split(r"\n\s*\n", body, maxsplit=1)[0]
    link = f"{REPOSITORY}/blob/v{version}/CHANGELOG.md#{anchor(heading)}"
    print(f"{opening}\n\nThe full entry is in [CHANGELOG.md]({link}).")


if __name__ == "__main__":
    main()
