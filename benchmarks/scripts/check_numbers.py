#!/usr/bin/env python3
"""Checks that every number the docs quote is one the last benchmark run
measured.

The README and the guide quote nanoseconds per element and ratios from
`benchmarks/RESULTS.md` by hand. This script reads the tables of that
file and fails when a quoted number is not among them, so a re-run that
moves a number cannot leave a stale one in the prose. CI runs it without
running a benchmark; `report.py` regenerates `RESULTS.md`.
"""
import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
RESULTS = ROOT / "benchmarks" / "RESULTS.md"
QUOTING = [ROOT / "README.md", ROOT / "docs" / "when-to-use-enum-dispatch.md"]

CELL = re.compile(r"(\d+\.\d+)(?: \((\d+\.\d+)x\))?")
NANOSECONDS = re.compile(r"(\d+(?:\.\d+)?) ns")
RATIO = re.compile(r"(\d+(?:\.\d+)?)x\b")
# Another crate's claim the guide quotes and argues with, not a measurement.
QUOTED_CLAIMS = {"10"}


def measured(text: str) -> tuple[set[str], set[str]]:
    """The values and the ratios of every table cell, as the docs round them."""
    values: set[str] = set()
    ratios: set[str] = set()
    for line in text.splitlines():
        if not line.startswith("|"):
            continue
        for value, ratio in CELL.findall(line):
            values.add(f"{float(value):.1f}")
            if ratio:
                ratios.add(f"{float(ratio):.1f}")
                ratios.add(f"{int(float(ratio))}")
                ratios.add(f"{round(float(ratio))}")
    return values, ratios


def quoted(text: str) -> tuple[list[str], list[str]]:
    """The nanoseconds and the ratios the prose quotes, outside code blocks."""
    prose = re.sub(r"```.*?```", "", text, flags=re.DOTALL)
    return NANOSECONDS.findall(prose), RATIO.findall(prose)


def main() -> int:
    values, ratios = measured(RESULTS.read_text())
    stale = []
    for path in QUOTING:
        nanoseconds, quoted_ratios = quoted(path.read_text())
        for number in nanoseconds:
            if f"{float(number):.1f}" not in values:
                stale.append(f"{path.name}: {number} ns is not in RESULTS.md")
        for number in quoted_ratios:
            if number in QUOTED_CLAIMS:
                continue
            if number not in ratios and f"{float(number):.1f}" not in ratios:
                stale.append(f"{path.name}: {number}x is not in RESULTS.md")
    if stale:
        print("\n".join(stale))
        print(f"{len(stale)} quoted numbers are not in {RESULTS.relative_to(ROOT)}")
        return 1
    print("every quoted number is in RESULTS.md")
    return 0


if __name__ == "__main__":
    sys.exit(main())
