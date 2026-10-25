# ADR-001: cost-model-driven

**Status:** proven (1 flagship + spillover effects across its ecosystem swaps)
**Origin:** [bbtree](https://github.com/XiangpengHao/bbtree) (Xiangpeng Hao, Bf-Tree / VLDB 2024)
**Adopted by:** bbtree

## Context (the problem)

Most data structures are built by tradition: B-trees look like 1970s papers, hash maps like 1990s CLRS. Nobody asks what the actual hardware bill is. Meanwhile three costs dominate modern CPUs and differ by 200×: cache miss ≈ 100 ns, instruction ≈ 0.5 ns, coherence traffic ≈ 100 ns+ per bounced cache line. Intuition tuned for 1990s hardware optimizes the wrong term.

## Technique

1. Write the cost model down first: what does one lookup pay for in cache misses, instructions, and coherence traffic?
2. Derive the layout from the model, not from folklore: nodes of 16-32 elements (linear search beats binary below ~32 thanks to the prefetcher), power-of-two allocations (no fragmentation), key encodings chosen per probe cost.
3. Only then write code, and only where the model says it pays: manual SIMD on the hot path, integer comparisons instead of memcmp, versioned locks instead of RwLock (a read-lock increment *is* a cache-line write).

## Evidence

- get u64: 109 ns vs std `BTreeMap` 264 ns vs crossbeam SkipMap 997 ns.
- 96 threads: **12× vs crossbeam-skiplist** (the upstream headline comparison); 627 M ops/s at 96 threads. Single-thread: **5× vs std `BTreeMap`** (upstream benchmark). p99 5× lower.
- Memory: 271 MB vs 315 (std) / 435 (skiplist) at 10 M keys.
- End-to-end (map swap only) in fjall / SlateDB / Tonbo / Turso / Lance: 1.16-6.83×.
- Cross-architecture (Zen 5, Rocket Lake, M1 Pro, Xeon, Graviton5): 2.0-2.8× vs std.

## Trade-offs / when NOT to use

- Requires a workload stable enough to model. If access patterns change monthly, the model rots.
- Hand-tuned paths are per-ISA (AVX-512 vs NEON need separate care).
- For cold, latency-tolerant storage the whole exercise is wasted - model first, and the model may say "boring is fine".

## References

- [bbtree design post](https://bbtree.xiangpeng.systems/post) - full methodology, machines and allocator stated.
- [bbtree `design/03-decisions.md`](https://github.com/XiangpengHao/bbtree/blob/main/design/03-decisions.md) - the author's own decision log (D56-style).
