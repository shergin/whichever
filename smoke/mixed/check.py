#!/usr/bin/env python3
"""Check that an older reader gives an honest result for a newer carrier.

Builds `smoke/mixed`, whose enum crate dispatches with the minor before this
checkout's a trait whose carrier this checkout wrote, and asserts that the
build either succeeds, the older reader reading a prefix it knows, or fails
with that reader's one sentence for a prefix it does not know; never with
a wrong sentence about the trait, and never with anything else. Needs the
registry.
"""

import subprocess
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
# The sentence of the readers up to 0.6, and the one from 0.7 on.
HONEST = (
    "unsupported `whichever` protocol version",
    "written by a newer `whichever` than the one dispatching it",
)
WRONG = "has no receiver"


def main():
    result = subprocess.run(
        ["cargo", "build", "-q", "--manifest-path", str(HERE / "Cargo.toml")],
        capture_output=True,
        text=True,
    )
    output = result.stdout + result.stderr
    if result.returncode == 0:
        print("ok: the older reader reads the carrier")
        return
    if WRONG in output:
        sys.exit(f"the older reader gave a wrong sentence:\n{output}")
    if not any(sentence in output for sentence in HONEST):
        sys.exit(f"the older reader did not report the newer carrier:\n{output}")
    print("ok: the older reader reports the newer carrier")


if __name__ == "__main__":
    main()
