#!/usr/bin/env bash
# Runs the whole suite and writes benchmarks/RESULTS.md: the criterion
# benchmarks, the generated-code comparison and the report. Arguments go to
# criterion, so `scripts/run.sh --quick` is the fast version.
set -euo pipefail
cd "$(dirname "$0")/../.."
cargo bench -p whichever-benchmarks -- "$@"
python3 benchmarks/scripts/codegen.py
python3 benchmarks/scripts/report.py
