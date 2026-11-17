#!/usr/bin/env python3
"""Checks that every relative link in the Markdown docs points at a file
and a heading that exist.

The README and the pages under `docs/` link to each other by path and by
heading anchor, and a renamed page or heading breaks such a link without
any build noticing. This script reads every Markdown file git tracks or
would track, resolves each relative link and image against the linking
file, and checks a `#fragment` against the anchors GitHub derives from the
target's headings. Absolute URLs are not fetched, so CI does not depend on
the network. With paths as arguments it checks those files only.
"""
import re
import subprocess
import sys
from pathlib import Path
from urllib.parse import unquote

ROOT = Path(__file__).resolve().parents[1]

FENCE = re.compile(r"^ {0,3}(`{3,}|~{3,})")
CODE_SPAN = re.compile(r"(`+).+?\1")
HEADING = re.compile(r"^ {0,3}#{1,6}\s+(.*?)(?:\s+#+)?\s*$")
HTML_ANCHOR = re.compile(r"<a\s+(?:name|id)=\"([^\"]+)\"")
INLINE = re.compile(r"\]\(\s*<?([^)\s>]*)>?(?:\s+\"[^\"]*\")?\s*\)")
DEFINITION = re.compile(r"^ {0,3}\[([^\]]+)\]:\s*<?(\S+?)>?(?:\s.*)?$")
FULL_REFERENCE = re.compile(r"\]\[([^\]]*)\]")
SCHEME = re.compile(r"^[a-zA-Z][a-zA-Z0-9+.-]*:")


def prose_lines(text: str, keep_code: bool = False) -> list[tuple[int, str]]:
    """The numbered lines outside fenced code blocks, inline code removed."""
    lines = []
    fence = ""
    for number, line in enumerate(text.splitlines(), 1):
        opening = FENCE.match(line)
        if fence:
            # A fence closes only on a run of its own character at least as long.
            if opening and opening.group(1)[0] == fence[0] and len(opening.group(1)) >= len(fence):
                if not line.strip().strip(fence[0]):
                    fence = ""
            continue
        if opening:
            fence = opening.group(1)
            continue
        lines.append((number, line if keep_code else CODE_SPAN.sub("", line)))
    return lines


def slug(heading: str) -> str:
    """The anchor GitHub gives a heading, before duplicates are numbered."""
    # A code span's text is kept verbatim, so `Box<Self>` is not taken for a tag.
    parts = re.split(r"(`+)(.+?)\1", heading)
    text = ""
    for index in range(0, len(parts), 3):
        prose = re.sub(r"!?\[([^\]]*)\]\([^)]*\)", r"\1", parts[index])
        prose = re.sub(r"<[^>]+>", "", prose).replace("*", "")
        # An underscore is emphasis only at a word boundary; `snake_case` keeps it.
        text += re.sub(r"(?<!\w)_+|_+(?!\w)", "", prose)
        text += parts[index + 2] if index + 2 < len(parts) else ""
    text = "".join(c for c in text.lower() if c.isalnum() or c in " -_")
    return text.replace(" ", "-")


def anchors(path: Path) -> set[str]:
    """Every anchor a page offers, numbering repeated headings as GitHub does."""
    found: set[str] = set()
    seen: dict[str, int] = {}
    # A heading's code spans are part of its text, so they stay here.
    for _, line in prose_lines(path.read_text(), keep_code=True):
        found.update(HTML_ANCHOR.findall(line))
        heading = HEADING.match(line)
        if not heading:
            continue
        base = slug(heading.group(1))
        count = seen.get(base, 0)
        seen[base] = count + 1
        found.add(base if count == 0 else f"{base}-{count}")
    return found


def targets(lines: list[tuple[int, str]]) -> list[tuple[int, str]]:
    """The inline and reference-definition targets, with their line numbers."""
    found = []
    for number, line in lines:
        definition = DEFINITION.match(line)
        if definition:
            found.append((number, definition.group(2)))
            continue
        found.extend((number, target) for target in INLINE.findall(line))
    return found


def check_target(path: Path, target: str, cache: dict[Path, set[str]]) -> str | None:
    """Why a link target does not resolve, or `None` when it does."""
    if not target or SCHEME.match(target):
        return None
    location, _, fragment = target.partition("#")
    location = unquote(location.partition("?")[0])
    if location.startswith("/"):
        resolved = ROOT / location.lstrip("/")
    else:
        resolved = (path.parent / location).resolve() if location else path
    if not resolved.exists():
        return f"{target}: {location} does not exist"
    if not fragment or resolved.suffix != ".md" or resolved.is_dir():
        return None
    if resolved not in cache:
        cache[resolved] = anchors(resolved)
    if unquote(fragment) not in cache[resolved]:
        return f"{target}: no heading with anchor #{fragment} in {display(resolved)}"
    return None


def check_references(lines: list[tuple[int, str]]) -> list[tuple[int, str]]:
    """Full or collapsed references to undefined labels, and unused definitions."""
    defined = {m.group(1).lower(): n for n, line in lines if (m := DEFINITION.match(line))}
    prose = [(n, line) for n, line in lines if not DEFINITION.match(line)]
    used = {label.lower() for _, line in prose for label in re.findall(r"\[([^\]]+)\]", line)}
    problems = [
        (n, f"reference [{label}] is not defined")
        for n, line in prose for label in FULL_REFERENCE.findall(line)
        if label and label.lower() not in defined
    ]
    problems += [(n, f"reference [{label}] is defined but not used")
                 for label, n in defined.items() if label not in used]
    return problems


def display(path: Path) -> str:
    """A path relative to the repository root when it lies inside it."""
    try:
        return str(path.resolve().relative_to(ROOT))
    except ValueError:
        return str(path)


def markdown_files() -> list[Path]:
    """Every Markdown file git tracks or would track, so new pages count."""
    listed = subprocess.run(
        ["git", "ls-files", "--cached", "--others", "--exclude-standard", "*.md"],
        cwd=ROOT, capture_output=True, text=True, check=True,
    ).stdout.splitlines()
    return [ROOT / name for name in sorted(set(listed)) if (ROOT / name).exists()]


def main() -> int:
    paths = [Path(arg).resolve() for arg in sys.argv[1:]] or markdown_files()
    cache: dict[Path, set[str]] = {}
    broken = []
    for path in paths:
        lines = prose_lines(path.read_text())
        problems = [(n, check_target(path, target, cache)) for n, target in targets(lines)]
        problems += check_references(lines)
        broken += [f"{display(path)}:{n}: {problem}" for n, problem in problems if problem]
    if broken:
        print("\n".join(broken))
        print(f"{len(broken)} link problems in {len(paths)} Markdown files")
        return 1
    print(f"every link in {len(paths)} Markdown files resolves")
    return 0


if __name__ == "__main__":
    sys.exit(main())
