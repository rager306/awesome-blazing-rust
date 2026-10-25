# Own run: roaring-rs vs std HashSet/BTreeSet (2026-10-25)

**Who:** this catalog's own reproducible run. **Status:** replicated (independent of upstream benchmarks).

## Environment

**Host:** KVM VPS, single NUMA node.

| | |
|---|---|
| CPU | AMD EPYC (with IBPB), 12 vCPU / 1 thread per core, 2.0 GHz |
| AVX | AVX2 + BMI2 (no AVX-512) |
| Cache | L3 8 MiB (shared, 1 instance) |
| RAM | 48 GB |
| Kernel / OS | Ubuntu 24.04, 6.8.0-147-generic, x86_64 |
| Toolchain | rustc 1.99.1 (2026-10-13), `--release` = opt-level 3 + LTO |
| Crates | roaring 0.10.12; baselines from `std::collections` |
| Parallelism | workload single-threaded (12 vCPU available) |

Note: this is a shared-tenant VM at 2.0 GHz - absolute numbers are modest by design; the *ratios* between structures are the claim. Numbers from 2026-10-25.

## Method

- Two uniform-random u32 sets per shape: A (N keys), B = 50% keys drawn from A + 50% fresh.
- Deterministic xorshift64* seeding (fixed seeds in source) - reproducible byte-for-byte.
- Hot ops = median of 7 runs; contains = 1M random probes; intersection/union = full op with result materialized; counts cross-checked across all three structures (asserted in code).
- Two data shapes: **sparse** (universe 2^32, density ~0.02%) and **dense** (universe 8N, density 12.5%).

## Results (median; ms)

| Shape | N | Op | roaring | HashSet | BTreeSet |
|---|---|---|---|---|---|
| sparse | 1M | build A | 1317.0 | 149.5 | 465.0 |
| sparse | 1M | contains 1M | 663.1 | 52.1 | 544.9 |
| sparse | 1M | intersect | 54.4 | 118.2 | 62.9 |
| sparse | 1M | union | 42.6 | 131.1 | 73.7 |
| dense | 1M | build A | 106.4 | 153.7 | 355.7 |
| dense | 1M | contains 1M | 28.0 | 92.8 | 512.3 |
| dense | 1M | intersect | **1.9** | 148.2 | 61.8 |
| dense | 1M | union | **0.3** | 127.7 | 65.1 |
| sparse | 8M | build A | 8279.2 | 2001.5 | 8017.0 |
| sparse | 8M | contains 1M | 1125.5 | 131.8 | 1215.4 |
| sparse | 8M | intersect | 169.5 | 1646.0 | 1114.9 |
| sparse | 8M | union | 202.6 | 1761.3 | 990.4 |
| dense | 8M | build A | 2925.6 | 1897.0 | 6740.6 |
| dense | 8M | contains 1M | 150.7 | 149.1 | 1039.4 |
| dense | 8M | intersect | **17.9** | 1419.7 | 620.6 |
| dense | 8M | union | **3.9** | 1598.1 | 769.2 |

## Reading

- **Density decides.** Sparse uniform keys (0.02% density) put every roaring container in array mode: random `contains` is **8-13x slower than HashSet**, and builds are 4-9x slower. Dense keys (12.5%) flip containers to bitmap mode: roaring matches HashSet on lookup and **wins set operations by 76-413x**.
- Roaring is a **set-operation accelerator**, not a drop-in faster hashset. If your workload is point-lookups over sparse ids, HashSet wins. If you intersect/union large id collections (postings, filters, graph node sets), roaring wins catastrophically - on dense data by two orders of magnitude.
- Upstream's benchmark harness exists but does not publish results; this run fills that gap for one concrete workload class.

## Reproduce

```
cd runs/roaring-2026-10 && cargo run --release
```

Seeds are fixed; numbers should be stable on any single-socket machine up to run-to-run jitter.
