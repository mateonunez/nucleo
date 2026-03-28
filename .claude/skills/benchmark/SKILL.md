---
name: benchmark
description: Run nucleo benchmarks
---

# /benchmark

Run the nucleo benchmark suite to measure token consumption and execution speed.

## Instructions

1. Ensure nucleo is built: `cargo build --release`
2. Run `./benchmarks/run.sh --quick` for a quick smoke test
3. For full results: `./benchmarks/run.sh`
4. For format comparison: `./benchmarks/run.sh --formats`
5. Results are saved to `benchmarks/results/`
