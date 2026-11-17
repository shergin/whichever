#!/usr/bin/env python3
"""Checks that every link in the rendered crate docs reaches a page and an
anchor that exist.

The pages under `docs/` are pulled into the crate docs by `src/docs.rs`,
which gives each of their link labels a definition that works there. A
label without one keeps its GitHub path, such as `options.md`, and rustdoc
renders it as a dead link without a warning. rustdoc also checks that an
intra-doc link names an item, but never the `#fragment` after it, so a
renamed heading strands links such as `crate#options`. This script reads
the output of `cargo doc --no-deps -p whichever -p whichever-macros` and,
for every link in the documentation text of the two crates' pages, checks
that a relative target is a file of the output and that a fragment names
an `id` of the target page. Absolute URLs are not fetched, so CI does not
depend on the network. It builds nothing; a doc directory other than
`target/doc/` may be given as the argument.
"""
import sys
from html.parser import HTMLParser
from pathlib import Path
from urllib.parse import unquote, urlsplit

ROOT = Path(__file__).resolve().parents[1]
CRATES = ("whichever", "whichever_macros")


class Page(HTMLParser):
    """The ids of a page and the hrefs in its documentation text."""

    def __init__(self) -> None:
        super().__init__()
        self.ids: set[str] = set()
        self.hrefs: list[str] = []
        # Only `div` and `dd` open a scope, so only their nesting is counted;
        # counting every tag would need to know which ones HTML leaves open.
        self.depth = {"div": 0, "dd": 0}
        self.scope: tuple[str, int] | None = None
        self.in_item_table = False

    def handle_starttag(self, tag: str, attrs: list[tuple[str, str | None]]) -> None:
        """Records ids everywhere, and hrefs inside a scope."""
        values = {name: value or "" for name, value in attrs}
        if "id" in values:
            self.ids.add(unquote(values["id"]))
        classes = values.get("class", "").split()
        if tag == "dl":
            self.in_item_table = "item-table" in classes
        if tag in self.depth:
            self.depth[tag] += 1
            # A `docblock` holds an item's docs; a `dd` of a module's item
            # table holds the summary line of an item's docs. The rest of the
            # page, sidebar and table of contents included, is rustdoc's own.
            opens = "docblock" in classes or "docblock-short" in classes
            opens = opens or (tag == "dd" and self.in_item_table)
            if self.scope is None and opens:
                self.scope = (tag, self.depth[tag])
        if tag == "a" and self.scope is not None and "href" in values:
            self.hrefs.append(values["href"])

    def handle_endtag(self, tag: str) -> None:
        """Closes the scope with the element that opened it."""
        if tag == "dl":
            self.in_item_table = False
        if tag not in self.depth:
            return
        if self.scope == (tag, self.depth[tag]):
            self.scope = None
        self.depth[tag] -= 1


def read(path: Path, cache: dict[Path, Page]) -> Page:
    """The parsed page, parsed once."""
    if path not in cache:
        page = Page()
        page.feed(path.read_text())
        page.close()
        cache[path] = page
    return cache[path]


def check_href(doc: Path, path: Path, href: str, cache: dict[Path, Page]) -> str | None:
    """Why a link does not resolve, or `None` when it does."""
    parts = urlsplit(href)
    # A scheme covers `https:`, `mailto:` and `javascript:`. The standard
    # library's docs are not in a `--no-deps` output, so rustdoc links them
    # by absolute URL and this skips them too.
    if parts.scheme or parts.netloc:
        return None
    location = unquote(parts.path)
    target = (path.parent / location).resolve() if location else path
    # A path that climbs out of the output, such as `../examples/tour.rs`
    # from a page, may still name a file of the repository; docs.rs has none.
    if not target.is_relative_to(doc) or not target.is_file():
        return f"{href}: {display(doc, target)} does not exist in the docs"
    if not parts.fragment:
        return None
    inside = target.relative_to(doc).parts
    # A link into the source view carries a line range such as `#12-34`,
    # which the view's script resolves; the page has no such id.
    if inside[0] not in CRATES:
        return None
    fragment = unquote(parts.fragment)
    if fragment not in read(target, cache).ids:
        return f"{href}: no id {fragment!r} in {display(doc, target)}"
    return None


def display(doc: Path, path: Path) -> str:
    """A path relative to the doc directory when it lies inside it."""
    try:
        return str(path.resolve().relative_to(doc))
    except ValueError:
        return str(path)


def main() -> int:
    doc = (Path(sys.argv[1]) if len(sys.argv) > 1 else ROOT / "target" / "doc").resolve()
    if not (doc / "whichever" / "index.html").is_file():
        print(f"{doc / 'whichever' / 'index.html'} is missing; build the docs with")
        print("  cargo doc --no-deps -p whichever -p whichever-macros")
        return 1
    pages = sorted(p for crate in CRATES for p in (doc / crate).rglob("*.html"))
    cache: dict[Path, Page] = {}
    broken = []
    for path in pages:
        for href in read(path, cache).hrefs:
            problem = check_href(doc, path, href, cache)
            if problem:
                broken.append(f"{display(doc, path)}: {problem}")
    if broken:
        print("\n".join(broken))
        print(f"{len(broken)} link problems in {len(pages)} rendered pages")
        return 1
    print(f"every documentation link in {len(pages)} rendered pages resolves")
    return 0


if __name__ == "__main__":
    sys.exit(main())
