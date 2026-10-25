# Architecture Patterns Index

The second axis of this list: **how** the speed is achieved. Each pattern below maps to entries in the README. When you add an entry, tag it with the patterns it uses — if a pattern is missing here, add it with a 2-3 line explanation.

| Pattern | Used by |
|---|---|
| [`cost-model-driven`](decisions/001-cost-model-driven.md) | bbtree |
| `simd` | bbtree, memchr, blake3, xxhash-rust, polars, nalgebra, rapier, hashbrown |
| `versioned-locks` | bbtree |
| `inline-keys` | bbtree |
| `prefix-keys` | bbtree |
| `cache-conscious-layout` | bbtree, hashbrown |
| `epoch-reclamation` | crossbeam, papaya |
| `lock-free` | crossbeam |
| `sharding` | dashmap, papaya, glommio |
| `thread-per-core` | glommio, monoio |
| `io-uring` | io-uring, glommio, monoio |
| `batching` | io-uring |
| `mmap` | redb |
| `lsm` | fjall, SlateDB, sled |
| `columnar` | polars, arrow-rs, datafusion, tantivy |
| `zero-copy` | rkyv, arrow-rs, winnow, ripgrep, pingora |
| `stringview` | arrow-rs, datafusion |
| `query-optimization` | polars, datafusion |
| `branchless` | memchr |
| `automata-compilation` | regex, aho-corasick |
| `prefilter` | regex |
| `work-stealing` | tokio, rayon |
| [`findings-as-verdicts`](decisions/002-findings-as-verdicts.md) | uringscope |
| `kernel-matrix-in-ci` | uringscope |
| `btf-probe` | uringscope |
| `recursive-proofs` | paramo1d |
| [`memory-bounded-streaming`](decisions/004-memory-bounded-streaming.md) | paramo1d |
| `vendor-pinning` | paramo1d |
| [`skills-in-repo`](decisions/005-skills-in-repo.md) | uringscope |
| `poseidon2-gf2` | paramo1d |
| [`proof-carrying-performance`](decisions/003-proof-carrying-performance.md) | verus, bbtree |
| `miri-in-ci` | miri, bbtree |
| `dst-in-ci` | shuttle, bbtree |
| `gpu-rendered-ui` | zed, gpui-kit |
| `elm-architecture` / `typed-effects` | crux |
| `structural-sharing` / `hamt` | im |
| `pure-rust-port` | lz4_flex, redb |
| `merkle-tree` / `multi-core-hash` | blake3 |
| `statistical-rigor` | hyperfine, criterion |

---

## Pattern Notes

Patterns with a link have a full write-up in [decisions/](decisions/) - problem, technique, evidence, trade-offs.

Short versions. Each entry's `Blazing:` line in the README carries the evidence; this is the "how".

- **`cost-model-driven`** — decide the data layout from a cost model (cache miss ≈ 100 ns ≫ instruction ≈ 0.5 ns ≫ coherence traffic) *before* writing code. bbtree's three principles are the canonical example.
- **`versioned-locks`** — readers validate a version counter with retry instead of taking an RwLock: a read-lock increment is a cache-line write, so "lock-free reads" via RwLock are a myth at scale.
- **`inline-keys` / `prefix-keys`** — store fixed-size key prefixes inline (dense integer comparisons instead of pointer chasing); derive shared prefixes from parent separators for free (prefix B-trees, 1977).
- **`findings-as-verdicts`** — the profiler's output is a named problem + suggested fix + evidence, not a dashboard. Reports end with verdicts, not charts.
- **`memory-bounded-streaming`** — huge artifacts (proof matrices, indexes) live on disk and stream through bounded buffers; peak memory becomes a design constant, not an accident.
- **`proof-carrying-performance`** — formal proofs ride in CI next to the benchmarks: optimizations that break correctness cannot merge. Verus/Miri/Shuttle are the three guards.
- **`kernel-matrix-in-ci`** — the portability claim is executed, not written: the list of tested kernels lives in one file consumed by both docs and CI.
- **`vendor-pinning`** — third-party code is vendored with a hash inventory (`upstream.json`) and a checker script; trust surface is documented, not implied.

*(Patterns grow as entries grow. Propose new ones via PR.)*
