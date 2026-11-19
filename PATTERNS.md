# Architecture Patterns Index

The second axis of this list: **how** the speed is achieved. Patterns with a link have a full write-up in [decisions/](decisions/) - problem, technique, evidence, trade-offs. New entries must only use tags from this table (add a row via PR).

| Pattern | Used by | Removes |
| --- | --- | --- |
| `aes-ni` | ahash, fastdedup |  |
| `alloc-tracking` | divan, dhat, hotpath-rs |  |
| `arrangement-indexing` | differential-dataflow |  |
| `async-native` | quinn, hyper, ntex, plumbrs, orion, redistill |  |
| `automata-compilation` | regex, aho-corasick, alyze, wordchipper |  |
| `backend-trait` | candle, burn |  |
| `batching` | io-uring, SurrealKV, apache/iggy, lionturtle, emdb, redistill |  |
| `bitmap-index` | roaring-rs, fast-grep-rust |  |
| `branchless` | memchr, foldhash |  |
| `broad-phase-narrow-phase` | rapier |  |
| `btf-probe` | uringscope |  |
| `bulk-loading` | rstar |  |
| `c-binding` | zstd-safe, lionturtle, NumKong, ForkUnion |  |
| `cache-conscious-layout` | hashbrown, taffy, eg-strider, sparx, radixox, rust-tango, ForkUnion |  |
| `classical-algorithms` | num-bigint, rust-bio |  |
| `code-layout` | cargo-pgo |  |
| `columnar` | tantivy, polars, apache-arrow-rs, datafusion, fastdedup, vortex |  |
| `columnar-layout-optional` | ndarray |  |
| `combinators` | winnow, ntex |  |
| `compact-index` | indexmap |  |
| `compact-serialization` | fst |  |
| `component-layer` | gpui-kit |  |
| `connection-coalescing` | pingora |  |
| `const-generics` | nalgebra |  |
| `copy-on-write` | redb |  |
| [`cost-model-driven`](decisions/001-cost-model-driven.md) | bbtree |  |
| `data-parallel` | differential-dataflow, flashraw, gitoxide, oxc, mzstd, molar, wordchipper, gzippy, flock, ForkUnion |  |
| `datalog` | cozo |  |
| `declarative-dsl` | slint |  |
| `density-dependent` | roaring-rs |  |
| `dst-in-ci` | shuttle |  |
| `elm-architecture` | crux |  |
| `embedded` | cozo, SurrealKV, canopydb, cesiumdb, mace |  |
| `embedded-first` | slint |  |
| `epoch-reclamation` | crossbeam |  |
| `extensible` | datafusion, vortex |  |
| `feature-gated-safety` | lz4_flex |  |
| [`findings-as-verdicts`](decisions/002-findings-as-verdicts.md) | uringscope, uringscope SKILL.md |  |
| `finite-fields` | arkworks, flock |  |
| `finite-state-transducer` | fst |  |
| `generic-graph` | petgraph, graphrs, sparx |  |
| `generic-vertices` | pathfinding |  |
| `gpu-rendered-ui` | zed, gpui-kit, dbdelve |  |
| `hamt` | im |  |
| `hit-ratio-first` | moka, foyer |  |
| `hnsw` | qdrant |  |
| `hot-patch` | blaze |  |
| `hyaline-reclamation` | papaya |  |
| `hybrid-cache` | foyer |  |
| `immediate-mode` | egui |  |
| `incremental-view-maintenance` | differential-dataflow, eg-strider, unumsearch |  |
| `inline-keys` | bbtree |  |
| `inline-storage` | smallvec |  |
| `invariants-as-tests` | nachalnik |  |
| `inverted-index` | tantivy, fast-grep-rust, unumsearch |  |
| `io-driver` | tokio |  |
| `io-uring` | glommio, monoio, io-uring, statico, plumbrs, apache/iggy, radixox, spargio |  |
| `jit` | wasmtime, blaze |  |
| `join-split` | rayon |  |
| `kernel-fusion` | burn, NumKong |  |
| `kernel-matrix-in-ci` | uringscope |  |
| `lock-free` | crossbeam, zero-pool, rust-tango, ForkUnion, emdb |  |
| `lock-free-parallel-walk` | ripgrep, ffind |  |
| `lsm` | fjall, SlateDB, SurrealKV, cesiumdb |  |
| `lsm-plus-btree` | sled, mace |  |
| [`memory-bounded-streaming`](decisions/004-memory-bounded-streaming.md) | paramo1d |  |
| `merkle-tree` | blake3 |  |
| `miri-in-ci` | miri |  |
| `multi-core-hash` | blake3 |  |
| `no-python` | candle |  |
| `no-std-friendly` | arkworks, mzstd, wordchipper, rust-tango |  |
| `object-storage` | SlateDB |  |
| `own-renderer` | zed |  |
| `parallel-algos` | rustworkx, graphrs, sparx |  |
| `partitioned` | fjall |  |
| `perf-integration` | cargo-flamegraph |  |
| `pgo` | cargo-pgo |  |
| `poseidon2-gf2` | paramo1d |  |
| `prefilter` | regex, unumsearch |  |
| `prefix-keys` | bbtree, radixox |  |
| [`proof-carrying-performance`](decisions/003-proof-carrying-performance.md) | verus |  |
| `pure-rust-port` | lz4_flex, gitoxide, jaq, mzstd, solar, rust-tango |  |
| `python-bridge` | rustworkx, molar |  |
| `quality-vs-speed` | qdrant, foldhash, pgvectorscale |  |
| `quantization` | qdrant, pgvectorscale |  |
| `query-optimization` | polars, datafusion |  |
| `rank-select` | sux |  |
| `read-optimized` | papaya |  |
| `recursive-proofs` | paramo1d, flock |  |
| `relative-pointers` | rkyv |  |
| `runtime-dispatch` | faer, pulp, NumKong |  |
| `sandboxing` | wasmtime, nachalnik |  |
| `sharding` | dashmap, glommio, apache/iggy, spargio, emdb, redistill |  |
| `shuttle-in-ci` | bbtree |  |
| `simd` | ripgrep, memchr, regex, hashbrown, blake3, xxhash-rust, crc32fast, bbtree, aho-corasick, sonic-rs, simd-json, polars, faer, nalgebra, rapier, pulp, flashraw, mzstd, fast-grep-rust, ffind, NumKong, flock |  |
| [`skills-in-repo`](decisions/005-skills-in-repo.md) | uringscope SKILL.md |  |
| `sparql-engine` | oxigraph |  |
| `speculative-parallel-decode` | flashraw |  |
| `statistical-rigor` | hyperfine, criterion, solar |  |
| `storage-backend-flag` | oxigraph |  |
| `stringview` | apache-arrow-rs, datafusion |  |
| `structural-sharing` | im |  |
| `succinct` | sux |  |
| `table-driven` | crc32fast |  |
| `tape-representation` | simd-json |  |
| `thread-per-core` | glommio, monoio, statico, orion, apache/iggy, radixox |  |
| `tls-13` | quinn |  |
| `typed-effects` | crux |  |
| `typed-parse-direct` | sonic-rs |  |
| `typegen` | crux |  |
| `valgrind-based` | gungraun |  |
| `vendor-pinning` | paramo1d |  |
| `versioned-locks` | bbtree |  |
| `verus-in-ci` | bbtree |  |
| `views` | ndarray |  |
| `wakeup-contract` | strict-future-contract |  |
| `window-tinylfu` | moka |  |
| `work-stealing` | tokio, rayon, pingora, ruff, graphrs, may_minihttp, ffind, spargio |  |
| `zero-copy` | ripgrep, sux, itertools, winnow, apache-arrow-rs, rkyv, gitoxide, oxc, statico, ruff, orion, fast-grep-rust, sparx, gzippy, rust-tango, vortex, unumsearch |  |
| `zero-macro-ergonomics` | axum |  |

---

## Pattern Notes

Short versions. Each entry's `Blazing:` line in the README carries the evidence; this is the "how".

- **`cost-model-driven`** - decide the data layout from a cost model (cache miss ≈ 100 ns ≫ instruction ≈ 0.5 ns ≫ coherence traffic) *before* writing code. bbtree's three principles are the canonical example. [Full ADR](decisions/001-cost-model-driven.md)
- **`versioned-locks`** - readers validate a version counter with retry instead of taking an RwLock: a read-lock increment is a cache-line write, so "lock-free reads" via RwLock are a myth at scale.
- **`inline-keys` / `prefix-keys`** - store fixed-size key prefixes inline (dense integer comparisons instead of pointer chasing); derive shared prefixes from parent separators for free (prefix B-trees, 1977).
- **`findings-as-verdicts`** - the profiler's output is a named problem + suggested fix + evidence, not a dashboard. [Full ADR](decisions/002-findings-as-verdicts.md)
- **`memory-bounded-streaming`** - huge artifacts live on disk and stream through bounded buffers; peak memory becomes a design constant. [Full ADR](decisions/004-memory-bounded-streaming.md)
- **`proof-carrying-performance`** - formal proofs ride in CI next to the benchmarks. [Full ADR](decisions/003-proof-carrying-performance.md)
- **`skills-in-repo`** - the agent-readable skill travels with the binary. [Full ADR](decisions/005-skills-in-repo.md)
- **`kernel-matrix-in-ci`** - the portability claim is executed, not written: the kernel list lives in one file consumed by both docs and CI.
- **`vendor-pinning`** - third-party code is vendored with a hash inventory and a checker script; trust surface is documented, not implied.

*(Patterns grow as entries grow. Propose new ones via PR - with the causal story: what cost it removes, when it works, what it costs, when it loses.)*
