# Three pages ship in the crate docs

Status: accepted, 2026-11-17. Serves no rule of the
[vision](../vision.md); it settles where the documents are read. Reopen
when a fourth page is wanted on docs.rs, which is the moment to ask
whether every page under `docs/` ships; if docs.rs comes to render a
package's Markdown pages itself; or if keeping two targets for every link
costs more than the pages earn.

## Context

docs.rs renders rustdoc and nothing else, from the published package. The
tutorial, the glossary and the troubleshooting page were Markdown files in
the repository, and `/docs` was left out of the package. A reader on
docs.rs had the crate docs, which linked to those pages on GitHub at
`main`, so the pages could describe a later version than the one the
reader depends on. The owner asked for the three pages on docs.rs.

## Decision

- `src/docs.rs` declares three empty modules, `docs::tutorial`,
  `docs::glossary` and `docs::troubleshooting`. Each is documented with
  `include_str!` of its page, so GitHub and docs.rs read one file.
- The modules are compiled only when rustdoc builds documentation,
  `#[cfg(all(doc, not(doctest)))]`. No code can name them, so they are no
  part of the API. Their code blocks are not collected as doctests: they
  are pieces of `examples/tour.rs` and of `tests/ui` cases, which are
  checked where they live.
- The three files ship in the package. The rest of `docs/` stays out.
- A page's links to files are reference-style. The page defines each label
  for GitHub, by relative path. `src/docs.rs` defines the same label
  before the page is included, and Markdown keeps the first definition, so
  in the crate docs the label goes to the matching section of the crate
  documentation, to another of the three modules, or to the repository.
- `scripts/check_doc_links.py` reads the rendered docs and fails on a link
  that reaches no file or no heading. It catches a label that lacks its
  second definition, and a heading of the crate docs that was renamed.

## Evidence

- Probed on 2026-11-17, on stable 1.100: a definition written above the
  include wins over the page's own, and `crate#options` renders as a link
  to that section of the crate page. Under `cfg(doc)` alone,
  `cargo test --doc` collected a page's snippet and failed on it; with
  `not(doctest)` it collects none.
- `cargo package --list` shows the three pages and no other file of
  `docs/`.
- Every in-page link of the three pages reached a heading in the rendered
  HTML, so rustdoc's heading ids agree with GitHub's anchors for them.

## Not chosen

- Links to the repository only, as before: the pages are read at `main`,
  not at the version.
- A copy of each page under `src/`, generated with its links rewritten:
  two files a page to keep in step, and a generator to maintain.
- Modules that are always compiled: public names with nothing behind them,
  and a promise to keep them.
- Every page under `docs/`: the crate docs already carry what the
  reference pages say, with runnable examples, and a second copy on the
  same site would have to be kept in step with the first.
- Marking each snippet `ignore`: rustdoc then flags every one of them as
  untested.
- A book on a site of its own: another build to run, and its version
  pinned by hand.
