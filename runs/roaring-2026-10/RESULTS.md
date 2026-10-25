# Own run: roaring-rs vs std collections (v2, 2026-10-26)

**Who:** this catalog's own reproducible run. **Status:** replicated (independent of upstream benchmarks).

**Supersedes the 2026-10-25 v1 run.** v1 compared roaring's *materializing* `&`/`|` against std's *counting* `.intersection().count()` as one coefficient - different op semantics averaged into one number. v2 runs both scenarios explicitly for every contender, repeats every published operation (v1 repeated only `contains`), and prints all samples so the raw data ships with the run.

## Environment (canonical run)

**Host:** KVM VPS, quiet, single NUMA node.

| | |
|---|---|
| CPU | AMD EPYC 9754-class, 2 vCPU / 1 thread per core |
| RAM | 50 GB |
| OS | Linux 6.x, x86_64 |
| Toolchain | rustc 1.99.1 (2026-10-13), `--release` = opt-level 3 + LTO |
| Crates | roaring 0.10.12; baselines from `std::collections` |
| Parallelism | workload single-threaded |

Second host (shared 12-vCPU, noisier) agrees in direction on all four shapes (dense 1M union materialize: 0.15 ms vs 415 ms = 2768x); ratios, not absolute values, are the claim.

## Method

- Two uniform-random u32 sets per shape: `A` (N draws), `B` = 50% drawn from A + 50% fresh. Population honesty: draws with duplicates are not unique keys - the binary prints actual unique counts (e.g. 8M draws -> 7.99M unique at sparse).
- **contains**: 1M probes drawn from the whole universe; hit rates are measured, not assumed: 0.02% sparse (N=1M), 0.2% (N=8M), 11.7% dense.
- **count scenario** - result size without a container: roaring iterates the smaller set probing the larger; std uses iterator `.count()`.
- **materialize scenario** - build the result container: roaring `&` / `|`; std `.collect()`.
- Every scenario: 7 runs, median reported, all 7 samples printed. Builds: declared single-run.
- Cross-checks outside the timed region: hit counts and result sizes equal across all three structures; FNV fingerprint over sorted keys proves identical intersection/union contents.
- Deterministic xorshift64*, fixed seeds, zero deps besides roaring.

## Results (medians, ms; quiet host)

| Shape | Op | roaring | HashSet | BTreeSet |
|---|---|---|---|---|
| sparse 1M | contains 1M | 278.8 | **25.2** | 286.8 |
| sparse 1M | intersect count | 78.0 | 49.5 | **17.3** |
| sparse 1M | union count | 103.7 | 55.4 | **15.7** |
| sparse 1M | intersect materialize | 25.8 | 101.0 | 19.7 |
| sparse 1M | union materialize | 26.1 | 157.8 | 25.8 |
| dense 1M | contains 1M | **10.9** | 36.9 | 242.4 |
| dense 1M | intersect count | **8.3** | 55.5 | 15.9 |
| dense 1M | union count | 11.5 | 53.6 | **14.3** |
| dense 1M | intersect materialize | **1.5** | 100.7 | 18.3 |
| dense 1M | union materialize | **0.14** | 152.0 | 23.7 |
| sparse 8M | contains 1M | 385.0 | **67.8** | 525.0 |
| sparse 8M | intersect count | 742.8 | 699.6 | **136.6** |
| sparse 8M | union count | 824.3 | 692.6 | **125.2** |
| sparse 8M | intersect materialize | **109.8** | 1134.4 | 155.4 |
| sparse 8M | union materialize | **126.5** | 2020.2 | 225.9 |
| dense 8M | contains 1M | **52.5** | 75.5 | 510.8 |
| dense 8M | intersect count | **97.9** | 681.6 | 126.9 |
| dense 8M | union count | 128.7 | 675.7 | **114.7** |
| dense 8M | intersect materialize | **12.6** | 1175.6 | 145.7 |
| dense 8M | union materialize | **2.5** | 1879.2 | 227.6 |

Raw output with every sample: `telemetry-v2-quiet.txt` (canonical), `telemetry-v2-epyc.txt` (shared-host spot-check).

## Reading

- **Density decides - and so does the op.** Roaring's headline wins are *materialize* wins: dense union materialize is 755-1086x faster than HashSet (2.5 ms vs 1879 ms at 8M keys), dense intersect materialize 69-93x. Against BTreeSet the dense materialize margin is 91-169x.
- **Sparse materialize is a modest win**: 4-16x vs HashSet, parity-to-1.4x vs BTreeSet. At 0.02-0.2% density the result-assembly cost dominates the bitmap advantage.
- **Count-only scans**: BTreeSet's sorted merge wins every sparse shape; roaring's probe-scan loses 2-7x. If you only need `|A & B|`, the sorted structure is both the wrong baseline to praise roaring against - and a real winner against it.
- **contains**: roaring wins only in dense (3.4x at 1M, 1.4x at 8M vs HashSet); at sparse it is 6-11x *slower* than HashSet. "Bitmap lookup is free" holds at density, not sparsity.
- Builds (single-run) are roaring's slowest showing: 4.3 s vs 0.8 s HashSet at 8M sparse - cold-start relevant, part of the honest cost.

## Reproduce

```sh
cd runs/roaring-2026-10 && cargo run --release
```

Seeds are fixed; numbers should be stable on any single-socket machine up to run-to-run jitter.
