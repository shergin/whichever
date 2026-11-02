#!/usr/bin/env python3
"""Turn the criterion output and the codegen comparison into RESULTS.md.

Reads every `benchmark.json` and `estimates.json` under `target/criterion`,
groups them by benchmark group and lays each group out as a table: one row per
subject, one column per parameter (variant count, method or element count),
each cell the median time per element and its ratio to the hand-written row.
Appends the codegen comparison when `target/codegen/results.json` exists.

    scripts/report.py                       # write benchmarks/RESULTS.md
    scripts/report.py --out -               # print to stdout

Needs only Python 3.
"""

import argparse
import datetime
import json
import platform
import subprocess
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
ROOT = HERE.parent.parent
SUBJECTS = [
    "hand",
    "whichever",
    "enum_dispatch",
    "ambassador",
    "enum_delegate",
    "delegation",
    "delegate",
    "box_dyn",
    "monomorphic",
]
LABELS = {
    "hand": "hand-written `match`",
    "whichever": "**whichever**",
    "enum_dispatch": "enum_dispatch",
    "ambassador": "ambassador",
    "enum_delegate": "enum_delegate",
    "delegation": "delegation",
    "delegate": "delegate",
    "box_dyn": "`Box<dyn Trait>`",
    "monomorphic": "no dispatch (one type)",
}
CODEGEN_SUBJECTS = {"dynamic": "box_dyn", "mono": "monomorphic"}


def run(command):
    try:
        return subprocess.run(command, capture_output=True, text=True, check=True).stdout.strip()
    except (OSError, subprocess.CalledProcessError):
        return ""


def machine():
    system = platform.system()
    if system == "Darwin":
        cpu = run(["sysctl", "-n", "machdep.cpu.brand_string"])
        version = "macOS " + run(["sw_vers", "-productVersion"])
    else:
        cpu = ""
        try:
            for line in Path("/proc/cpuinfo").read_text().splitlines():
                if line.startswith("model name"):
                    cpu = line.split(":", 1)[1].strip()
                    break
        except OSError:
            pass
        version = f"{system} {platform.release()}"
    return cpu or platform.processor() or platform.machine(), version


def load_criterion(directory):
    """Returns {group_id: {function_id: {value_str: measurement}}}."""
    groups = {}
    for benchmark in directory.glob("**/new/benchmark.json"):
        meta = json.loads(benchmark.read_text())
        estimates = json.loads((benchmark.parent / "estimates.json").read_text())
        median = estimates["median"]
        elements = (meta.get("throughput") or {}).get("Elements", 1)
        point = median["point_estimate"] / elements
        lower = median["confidence_interval"]["lower_bound"] / elements
        upper = median["confidence_interval"]["upper_bound"] / elements
        value = meta.get("value_str") or ""
        groups.setdefault(meta["group_id"], {}).setdefault(meta["function_id"], {})[value] = {
            "ns": point,
            "ci": max(point - lower, upper - point) / point if point else 0.0,
        }
    return groups


def numeric(value):
    try:
        return (0, int(value))
    except ValueError:
        return (1, value)


def human_count(value):
    try:
        n = int(value)
    except ValueError:
        return value
    for unit, size in (("Mi", 1 << 20), ("Ki", 1 << 10)):
        if n >= size and n % size == 0:
            return f"{n // size} {unit}"
    return str(n)


def table(group, columns, column_label, rows=SUBJECTS):
    """One markdown table: subjects down, parameters across, ns per element."""
    present = [s for s in rows if s in group]
    columns = [c for c in columns if any(c in group[s] for s in present)]
    header = "| | " + " | ".join(column_label(c) for c in columns) + " |"
    rule = "| --- |" + " ---: |" * len(columns)
    lines = [header, rule]
    worst_ci = 0.0
    for subject in present:
        cells = []
        for column in columns:
            cell = group[subject].get(column)
            if cell is None:
                cells.append("")
                continue
            if cell["ns"] < 0.005:
                # The optimiser removed the loop: nothing left to measure.
                cells.append("<0.01")
                continue
            worst_ci = max(worst_ci, cell["ci"])
            text = f"{cell['ns']:.2f}"
            reference = group.get("hand", {}).get(column)
            if reference and subject != "hand":
                text += f" ({cell['ns'] / reference['ns']:.2f}x)"
            cells.append(text)
        lines.append(f"| {LABELS.get(subject, subject)} | " + " | ".join(cells) + " |")
    return "\n".join(lines), worst_ci


def deviation(group, subject="whichever", reference="hand"):
    """The largest relative difference between two rows over all columns."""
    worst = None
    for column, cell in group.get(subject, {}).items():
        base = group.get(reference, {}).get(column)
        if base:
            ratio = cell["ns"] / base["ns"] - 1
            if worst is None or abs(ratio) > abs(worst):
                worst = ratio
    return worst


def dispatch_section(groups):
    out = []
    worst_overall = None
    for family in ("trivial", "opaque"):
        for pattern in ("uniform", "round_robin", "shuffled"):
            group = groups.get(f"dispatch/{family}/{pattern}")
            if not group:
                continue
            columns = sorted({c for s in group.values() for c in s}, key=numeric)
            body, ci = table(group, columns, lambda c: f"{c} variants")
            out.append(f"#### `{family}` bodies, `{pattern}` layout\n\n{body}\n")
            out.append(f"95% confidence interval within ±{ci * 100:.1f}% of the median in every cell.\n")
            worst = deviation(group)
            if worst is not None and (worst_overall is None or abs(worst) > abs(worst_overall)):
                worst_overall = worst
    if worst_overall is not None:
        out.insert(0, f"Across every cell below, `whichever` is within {abs(worst_overall) * 100:.1f}% of the hand-written `match`.\n")
    return "\n".join(out)


def methods_section(groups):
    merged = {}
    for method in ("area", "scale", "name", "scaled"):
        group = groups.get(f"methods/{method}")
        if not group:
            continue
        for subject, values in group.items():
            merged.setdefault(subject, {})[method] = next(iter(values.values()))
    if not merged:
        return ""
    labels = {
        "area": "`area(&self) -> u64`",
        "scale": "`scale(&mut self, u64)`",
        "name": "`name(&self)`, default body",
        "scaled": "`scaled(&self, u64) -> Self`",
    }
    body, ci = table(merged, list(labels), labels.get)
    return f"{body}\n\n95% confidence interval within ±{ci * 100:.1f}% of the median in every cell.\n"


def memory_section(groups):
    out = []
    scattered = groups.get("memory/scattered")
    if scattered:
        columns = sorted({c for s in scattered.values() for c in s}, key=numeric)
        body, ci = table(scattered, columns, lambda c: f"{human_count(c)} elements")
        out.append(f"#### Scattered collections\n\n{body}\n\n95% confidence interval within ±{ci * 100:.1f}% of the median in every cell.\n")
    build = groups.get("memory/build_and_drop")
    if build:
        columns = sorted({c for s in build.values() for c in s}, key=numeric)
        body, ci = table(build, columns, lambda c: f"build and drop {human_count(c)} elements")
        out.append(f"#### Building a collection\n\n{body}\n\n95% confidence interval within ±{ci * 100:.1f}% of the median in every cell.\n")
    return "\n".join(out)


def codegen_section(path):
    if not path.exists():
        return ""
    data = json.loads(path.read_text())
    by_key = {}
    for probe in data["probes"]:
        subject = CODEGEN_SUBJECTS.get(probe["subject"], probe["subject"])
        by_key.setdefault((probe["method"], probe["family"]), {}).setdefault(subject, {})[probe["variants"]] = probe
    out = []
    for method in ("area", "scale"):
        for family in ("trivial", "opaque"):
            group = by_key.get((method, family))
            if not group:
                continue
            variants = sorted({v for s in group.values() for v in s})
            lines = ["| | " + " | ".join(f"{v} variants" for v in variants) + " |", "| --- |" + " --- |" * len(variants)]
            for subject in SUBJECTS:
                if subject not in group:
                    continue
                cells = []
                for v in variants:
                    probe = group[subject].get(v)
                    if probe is None:
                        cells.append("")
                        continue
                    if subject == "hand":
                        cells.append(f"{probe['instructions']} instr., {probe['shape']}")
                    elif probe["identical_to_hand"]:
                        cells.append("identical")
                    else:
                        cells.append(f"{probe['instructions']} instr., {probe['shape']}")
                lines.append(f"| {LABELS.get(subject, subject)} | " + " | ".join(cells) + " |")
            out.append(f"#### `{method}`, `{family}` bodies\n\n" + "\n".join(lines) + "\n")
    return "\n".join(out)


def main():
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("--criterion", type=Path, default=ROOT / "target/criterion")
    parser.add_argument("--codegen", type=Path, default=ROOT / "target/codegen/results.json")
    parser.add_argument("--out", type=Path, default=ROOT / "benchmarks/RESULTS.md", help="'-' for stdout")
    args = parser.parse_args()

    groups = load_criterion(args.criterion)
    if not groups:
        sys.exit(f"no criterion results under {args.criterion}; run `cargo bench -p whichever-benchmarks` first")

    cpu, os_version = machine()
    rustc = run(["rustc", "--version"]) or "rustc"
    sections = [
        "# Benchmark results",
        "",
        "Generated by `benchmarks/scripts/report.py`; the methodology is in [README.md](README.md).",
        "",
        f"- Date: {datetime.date.today().isoformat()}",
        f"- Machine: {cpu}, {os_version}",
        f"- Toolchain: {rustc}, default `bench` profile (no LTO)",
        "- Numbers: median nanoseconds per element, lower is better; in parentheses, the ratio to the hand-written `match` row.",
        "",
    ]
    dispatch = dispatch_section(groups)
    if dispatch:
        sections += ["## One `&self` call per element", "", dispatch]
    methods = methods_section(groups)
    if methods:
        sections += ["## Method kinds, 8 variants, random layout", "", methods]
    memory = memory_section(groups)
    if memory:
        sections += ["## Memory", "", memory]
    codegen = codegen_section(args.codegen)
    if codegen:
        sections += [
            "## Generated code, compared with the hand-written `match`",
            "",
            "From `scripts/codegen.py`: each cell is the exported probe function that calls the subject's method once from another crate. \"identical\" means the same instructions as the hand-written probe once labels and symbol names are renamed.",
            "",
            codegen,
        ]
    text = "\n".join(sections).rstrip() + "\n"
    if str(args.out) == "-":
        sys.stdout.write(text)
    else:
        args.out.write_text(text)
        print(f"wrote {args.out}")


if __name__ == "__main__":
    main()
