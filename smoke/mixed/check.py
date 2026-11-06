#!/usr/bin/env python3
"""Check that an older reader reports a newer carrier in one sentence.

Builds `smoke/mixed`, whose enum crate dispatches with the last published
`whichever` a trait whose carrier this checkout wrote, and asserts that the
build fails with that reader's sentence for a prefix it does not know, and
not with a wrong sentence about the trait. Needs the registry.
"""

import subprocess
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
EXPECTED = "unsupported `whichever` protocol version"
WRONG = "has no receiver"


def main():
    result = subprocess.run(
        ["cargo", "build", "-q", "--manifest-path", str(HERE / "Cargo.toml")],
        capture_output=True,
        text=True,
    )
    output = result.stdout + result.stderr
    if result.returncode == 0:
        sys.exit("the mixed-version build succeeded; the older reader read a newer carrier silently")
    if WRONG in output:
        sys.exit(f"the older reader gave a wrong sentence:\n{output}")
    if EXPECTED not in output:
        sys.exit(f"the older reader did not report the newer carrier:\n{output}")
    print(f"ok: the older reader says \"{EXPECTED}\"")


if __name__ == "__main__":
    main()
