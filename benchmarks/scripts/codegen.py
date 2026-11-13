#!/usr/bin/env python3
"""Compare the machine code each dispatch strategy produces.

Compiles `examples/codegen.rs` to assembly, cuts out the exported probe
functions (one per subject, family, variant count and method, each calling
the subject's method once from a crate that is not the enum's own) and
compares every subject's function with the hand-written one after renaming
labels and symbols. Two functions that differ only in label numbers and in
which `<Payload as Trait>::method` symbols they branch to are reported as
identical.

Also notes the shape of each function: whether the generated method was
inlined into the caller or left as an out-of-line tail call, and whether the
dispatch itself is a compare chain, a jump table or an indirect call through
a vtable.

    scripts/codegen.py              # compile, compare, write target/codegen/
    scripts/codegen.py --asm FILE   # compare an assembly file already emitted

Writes `results.json` and one normalised `.s` file per probe to the output
directory, and prints the comparison table. Needs only Python 3 and cargo.
"""

import argparse
import json
import re
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
    "dynamic",
    "mono",
]
FAMILIES = ["trivial", "opaque"]
VARIANTS = [2, 8, 32]
METHODS = ["area", "scale"]

PROBE = re.compile(
    r"^_?(?P<subject>[a-z_]+?)__(?P<family>trivial|opaque)__(?P<variants>\d+)__(?P<method>area|scale)$"
)
# A non-local label defines a function (or data); local labels start with `L`
# on macOS and `.L` on Linux.
LABEL = re.compile(r"^([^\s.][^\s:]*|\.L[^\s:]*):$")
LOCAL = re.compile(r"^\.?L[A-Za-z0-9_$.]+$")
# LLVM writes a merged function's other names as `name = target`; the LLVM of
# Rust 1.88 writes `.set name, target`.
ALIAS = re.compile(r"^(\S+)\s*=\s*(\S+)$")
SET_ALIAS = re.compile(r"^\.set\s+(\S+),\s*(\S+)$")
TOKEN = re.compile(r"[A-Za-z_.$][A-Za-z0-9_.$@]*")
MANGLED = re.compile(r"^_*(RNv|ZN)")
# A tail call straight into a `payloads::V*` method is no dispatch at all: the
# callee was known at compile time.
PAYLOAD_METHOD = re.compile(r"8payloads[0-9]*V[0-9]")
# AArch64 and x86-64 spellings of the branch kinds the summary counts. On
# x86-64 Linux a call into another crate goes through the GOT,
# `jmpq *sym@GOTPCREL(%rip)`: a fixed target, so a direct call, not a dispatch.
GOT = r"\*[A-Za-z_.$][^\s]*@GOTPCREL\(%rip\)"
DIRECT_CALL = re.compile(r"^(bl|callq?)\s+(?:[^*%]|" + GOT + ")")
TAIL = re.compile(r"^(b|jmpq?)\s+(?:[^*%L.]|" + GOT + ")")
INDIRECT = re.compile(r"^(blr|br|callq?\s+\*|jmpq?\s+\*|notrack\s+jmp)\b(?!.*@GOTPCREL)")
CONDITIONAL = re.compile(r"^(b\.[a-z]+|cbn?z|tbn?z|j(?!mp)[a-z]+)\b")
JUMP_TABLE = re.compile(r"JTI")


def target_directory():
    """Returns the target directory cargo builds into, `CARGO_TARGET_DIR` included."""
    command = ["cargo", "metadata", "-q", "--no-deps", "--format-version", "1"]
    output = subprocess.run(command, cwd=ROOT, check=True, capture_output=True, text=True).stdout
    return Path(json.loads(output)["target_directory"])


def emit_assembly():
    """Compiles the probe example to one assembly file and returns its path."""
    # The assembly goes to a path of our own: where cargo keeps an unnamed
    # `--emit` output changed in 1.100, from `release/examples/codegen-<hash>.s`
    # to a per-unit build directory. Cargo hashes these arguments into the
    # example's fingerprint, so when it finds the example fresh, the file was
    # written by this same command from the same sources.
    path = target_directory() / "codegen-asm" / "codegen.s"
    path.parent.mkdir(parents=True, exist_ok=True)
    command = [
        "cargo", "rustc", "-q", "-p", "whichever-benchmarks", "--release", "--example", "codegen",
        "--", f"--emit=asm={path}", "-C", "codegen-units=1", "-C", "debuginfo=0",
    ]
    subprocess.run(command, cwd=ROOT, check=True)
    if not path.is_file():
        sys.exit(f"no {path}; run `cargo clean -p whichever-benchmarks --release` and retry")
    return path


def parse(asm_text):
    """Returns (functions, aliases): label -> instruction lines, alias -> target."""
    functions = {}
    aliases = {}
    current = None
    for raw in asm_text.splitlines():
        line = raw.strip()
        alias = SET_ALIAS.match(line) or (None if line.startswith(".") else ALIAS.match(line))
        if alias:
            aliases[alias.group(1)] = alias.group(2)
            continue
        label = LABEL.match(line)
        if label:
            name = label.group(1)
            if LOCAL.match(name):
                if current is not None:
                    functions[current].append(name + ":")
            else:
                current = name
                functions[current] = []
            continue
        if current is None:
            continue
        if line.startswith(".cfi_endproc"):
            current = None
            continue
        if not line or line.startswith(".") or line.startswith(("#", ";", "//")):
            continue
        line = re.split(r"\s(?:;|//|#(?!\d))", line, maxsplit=1)[0].rstrip()
        functions[current].append(line)
    return functions, aliases


def resolve(name, aliases):
    seen = set()
    while name in aliases and name not in seen:
        seen.add(name)
        name = aliases[name]
    return name


def normalise(body, defined):
    """Renames local labels and global symbols in order of first appearance."""
    labels = {}
    symbols = {}

    def rename(token):
        # `label@PAGE`, `symbol@PLT`: the relocation suffix stays, the name is renamed.
        bare, at, suffix = token.partition("@")
        if LOCAL.match(bare):
            return labels.setdefault(bare, f"L{len(labels)}") + at + suffix
        if bare in defined or bare.lstrip("_") in defined or MANGLED.match(bare):
            return symbols.setdefault(bare, f"S{len(symbols)}") + at + suffix
        return token

    out = []
    for line in body:
        if line.endswith(":"):
            out.append(rename(line[:-1]) + ":")
            continue
        parts = line.split(None, 1)
        if len(parts) == 1:
            out.append(line)
            continue
        mnemonic, operands = parts
        out.append(f"{mnemonic}\t{TOKEN.sub(lambda m: rename(m.group(0)), operands)}")
    return out


def summarise(body):
    instructions = [line for line in body if not line.endswith(":")]
    counts = {
        "instructions": len(instructions),
        "direct_calls": sum(1 for i in instructions if DIRECT_CALL.match(i)),
        "tail_calls": sum(1 for i in instructions if TAIL.match(i)),
        "indirect": sum(1 for i in instructions if INDIRECT.match(i)),
        "conditional": sum(1 for i in instructions if CONDITIONAL.match(i)),
        "jump_table": any(JUMP_TABLE.search(i) for i in instructions),
    }
    if counts["instructions"] == 1 and counts["tail_calls"] == 1:
        target = instructions[0].split()[-1]
        counts["shape"] = "direct call" if PAYLOAD_METHOD.search(target) else "out-of-line call"
    elif counts["indirect"] and not counts["jump_table"]:
        counts["shape"] = "indirect call"
    elif counts["jump_table"]:
        counts["shape"] = "jump table"
    elif counts["conditional"]:
        counts["shape"] = "compare chain"
    else:
        counts["shape"] = "straight line"
    return counts


def main():
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("--asm", type=Path, help="an assembly file already emitted, instead of compiling")
    parser.add_argument("--out", type=Path, default=ROOT / "target/codegen", help="output directory")
    args = parser.parse_args()

    asm_path = args.asm or emit_assembly()
    functions, aliases = parse(asm_path.read_text())
    defined = set(functions) | set(aliases)

    probes = {}
    for name in list(functions) + list(aliases):
        match = PROBE.match(name)
        if match:
            probes[name] = match.groupdict()
    if not probes:
        sys.exit(f"no probe functions found in {asm_path}")

    args.out.mkdir(parents=True, exist_ok=True)
    normalised = {}
    results = {}
    for name, info in probes.items():
        target = resolve(name, aliases)
        body = functions.get(target)
        if body is None:
            continue
        norm = normalise(body, defined)
        normalised[name] = norm
        key = (info["family"], int(info["variants"]), info["method"])
        entry = summarise(body)
        entry.update(subject=info["subject"], family=info["family"], variants=int(info["variants"]), method=info["method"])
        entry["alias_of"] = None if target == name else PROBE.match(target).group("subject")
        results.setdefault(key, {})[info["subject"]] = entry
        (args.out / f"{name.lstrip('_')}.s").write_text("\n".join(norm) + "\n")

    for key, by_subject in results.items():
        hand = normalised.get(next(n for n, i in probes.items() if (i["family"], int(i["variants"]), i["method"]) == key and i["subject"] == "hand"))
        for subject, entry in by_subject.items():
            name = next(n for n, i in probes.items() if (i["family"], int(i["variants"]), i["method"]) == key and i["subject"] == subject)
            entry["identical_to_hand"] = normalised[name] == hand

    flat = [entry for by_subject in results.values() for entry in by_subject.values()]
    (args.out / "results.json").write_text(json.dumps({"asm": str(asm_path), "probes": flat}, indent=1))

    width = max(len(s) for s in SUBJECTS)
    for method in METHODS:
        for family in FAMILIES:
            print(f"\n{method}, {family} payloads: instructions / shape (* = identical to hand-written)")
            print(f"{'subject':<{width}}  " + "  ".join(f"{v:>26}" for v in VARIANTS))
            for subject in SUBJECTS:
                cells = []
                for variants in VARIANTS:
                    entry = results.get((family, variants, method), {}).get(subject)
                    if entry is None:
                        cells.append(f"{'-':>26}")
                        continue
                    mark = "*" if entry["identical_to_hand"] else " "
                    cells.append(f"{entry['instructions']:>4}{mark} {entry['shape']:<20}")
                print(f"{subject:<{width}}  " + "  ".join(cells))
    print(f"\nwrote {args.out / 'results.json'} and one normalised .s per probe")


if __name__ == "__main__":
    main()
