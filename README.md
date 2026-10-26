# Awesome Blazing Rust [![Awesome](https://awesome.re/badge.svg)](https://awesome.re)

> Rust projects that **prove** their speed. Every entry ships measured numbers - and the architectural tricks behind them are named.

**The rule: no numbers, no entry.** Projects without public benchmarks live in the Candidates section until they show evidence. Each entry carries a `Blazing:` line (the measurement) and an `Arch:` line (the patterns - indexed in [PATTERNS.md](PATTERNS.md)).

---

## Contents

- [Hall of Fame](#hall-of-fame).
- [Algorithms & Data Structures](#algorithms--data-structures).
- [Concurrency & Lock-free](#concurrency--lock-free).
- [Async Runtimes & I/O](#async-runtimes--io).
- [Storage & Embedded Databases](#storage--embedded-databases).
- [Graph & Knowledge Graphs](#graph--knowledge-graphs).
- [Search & Text Processing](#search--text-processing).
- [Data & Analytics](#data--analytics)
- [Caching & Working Set](#caching--working-set)
- [Incremental Computation](#incremental-computation).
- [Networking & Protocols](#networking--protocols).
- [Web & Edge](#web--edge).
- [Science & Computational](#science--computational).
- [Crypto & Proof Systems](#crypto--proof-systems).
- [Verification & Correctness](#verification--correctness).
- [Observability & Profiling](#observability--profiling)
- [Agent Skills](#agent-skills).
- [UI & Apps](#ui--apps).
- [Benchmarking & Toolchain](#benchmarking--toolchain).
- [Wow & Humor](#wow--humor).
- [Recipes](#recipes).
- [Measured Caveats](#measured-caveats).
- [Fresh](#fresh)
- [Candidates](#candidates)
- [See also](#see-also).

---

## Hall of Fame

The canonical blazing projects - the ones that made "Rust is fast" a meme with receipts.

- [ripgrep](https://github.com/BurntSushi/ripgrep) - recursive grep with regex support. **Blazing:** repeatedly benchmarks 10×+ faster than GNU grep on large trees (see the project's own fgrep/grep comparisons). **Type:** measured. **Arch:** `simd`, `lock-free-parallel-walk`, `zero-copy`.
- [memchr](https://github.com/BurntSushi/memchr) - substring & multi-substring search. **Blazing:** SIMD-accelerated; the backbone under `regex` and `ripgrep`. **Type:** case-study. **Arch:** `simd`, `branchless`.
- [regex](https://github.com/rust-lang/regex) - the regex engine. **Blazing:** literal optimizations + SIMD-powered DFA; its docs document multi-x speedups vs naive engines. **Type:** case-study. **Arch:** `automata-compilation`, `simd`, `prefilter`.
- [tokio](https://github.com/tokio-rs/tokio) - the async runtime most of this list runs on. **Blazing:** work-stealing scheduler; powers production at AWS, Microsoft, Discord. **Type:** case-study. **Arch:** `work-stealing`, `io-driver`.
- [hyperfine](https://github.com/sharkdp/hyperfine) - command-line benchmarking tool. **Blazing:** statistical warm-up/outlier analysis in a CLI; the tool we benchmark this list with. **Type:** tool. **Arch:** `statistical-rigor`.

## Algorithms & Data Structures

- [sux](https://github.com/vigna/sux-rs) - succinct structures: Elias-Fano, rank/select, static functions, minimal perfect hashing. **Blazing:** scales to trillions of keys with very fast queries executed directly on the compressed representation. **Type:** case-study. **Arch:** `succinct`, `rank-select`, `zero-copy`.
- [roaring-rs](https://github.com/RoaringBitmap/roaring-rs) - Roaring compressed bitmaps for IDs, postings and set operations. **Blazing:** own run, v2 methodology (2026-10, quiet 2-vCPU, median of 7): dense union materialize 2.5 ms vs 1879 ms for HashSet (755x) and intersect 12.6 ms vs 1176 ms (93x); at sparse 0.02-0.2% density materialize wins only 4-16x, contains is 6-11x slower than HashSet, and BTreeSet wins count-only scans - density and op semantics decide ([full run](https://github.com/rager306/awesome-blazing-rust/blob/main/runs/roaring-2026-10/RESULTS.md), scope: component). **Type:** measured. **Arch:** `bitmap-index`, `density-dependent`.

Well-known algorithms, implemented properly in Rust.

### Collections

- [hashbrown](https://github.com/rust-lang/hashbrown) - SwissTable hash map; the implementation inside Rust's std since 1.36. **Blazing:** open addressing with SIMD control-byte probing. **Type:** case-study. **Arch:** `simd`, `cache-conscious-layout`.
- [indexmap](https://github.com/indexmap-rs/indexmap) - insertion-ordered hash map + set, O(1) amortized with order. **Blazing:** std-level speed with deterministic iteration. **Type:** case-study. **Arch:** `compact-index`.
- [im](https://github.com/bodil/im-rs) - immutable/persistent collections with structural sharing (archived 2026 - historical entry). **Blazing:** O(1) clones - cheap snapshots for concurrent and functional workloads. **Type:** case-study. **Arch:** `structural-sharing`, `hamt`.
- [smallvec](https://github.com/servo/rust-smallvec) - stack-allocated small vectors. **Blazing:** eliminates heap allocation for the common short case. **Type:** case-study. **Arch:** `inline-storage`.
- [itertools](https://github.com/rust-itertools/itertools) - the algorithm toolbox on iterators. **Blazing:** zero-cost abstractions, fused adapters. **Type:** tool. **Arch:** `zero-copy`.

### Hashing & Checksums

- [blake3](https://github.com/BLAKE3-team/BLAKE3) - cryptographic hash. **Blazing:** ~GB/s on modern x86 (AVX2/AVX-512, per project benchmarks); orders above SHA-2/SHA-3. **Type:** measured. **Arch:** `simd`, `merkle-tree`, `multi-core-hash`.
- [xxhash-rust](https://github.com/DoumanAsh/xxhash-rust) - xxHash (XXH3). **Blazing:** non-crypto hashing at memory-bandwidth speeds. **Type:** case-study. **Arch:** `simd`.
- [ahash](https://github.com/tkaitchuck/aHash) - DoS-resistant fast hash map hasher. **Blazing:** documented multi-fold wins over SipHash on hash-map workloads. **Type:** case-study. **Arch:** `aes-ni`.
- [crc32fast](https://github.com/srijs/rust-crc32fast) - CRC-32. **Blazing:** 15× faster than naive, via slicing-by-8 and SIMD switching. **Type:** measured. **Arch:** `simd`, `table-driven`.

### Pathfinding & Spatial

- [pathfinding](https://github.com/samueltardieu/pathfinding) - BFS, Dijkstra, A*, K-means, CSPs and friends. **Blazing:** the Swiss-army graph-algorithms crate. **Type:** tool. **Arch:** `generic-vertices`.
- [rstar](https://github.com/Stoeoef/rstar) - R*-tree spatial index. **Blazing:** bulk-loaded queries at interactive rates. **Type:** case-study. **Arch:** `bulk-loading`.

### Math & Parallel

- [num-bigint](https://github.com/rust-num/num-bigint) - arbitrary-precision integers. **Blazing:** Karatsuba multiplication in the num-rational stack. **Type:** case-study. **Arch:** `classical-algorithms`.
- [rayon](https://github.com/rayon-rs/rayon) - data parallelism. **Blazing:** near-linear scaling on parallel sort/map in typical workloads. **Type:** case-study. **Arch:** `work-stealing`, `join-split`.

### Compression

- [lz4_flex](https://github.com/pseitz/lz4_flex) - pure-Rust LZ4. **Blazing:** no C dependency; the safe configuration avoids `unsafe` and is benchmarked separately from the unsafe-feature build. **Type:** case-study. **Arch:** `pure-rust-port`, `feature-gated-safety`.
- [zstd-safe](https://github.com/gyscos/zstd-rs) - zstd bindings. **Blazing:** the ratio/speed king of general compression, one call away. **Type:** case-study. **Arch:** `c-binding`.

## Concurrency & Lock-free

- [bbtree](https://github.com/XiangpengHao/bbtree) - concurrent ordered map built from a cost model. **Blazing:** 5× vs std `BTreeMap` at 1 thread; 12× vs crossbeam-skiplist at 96 threads; map-only swap into fjall/SlateDB/Tonbo/Turso/Lance: 1.16-6.83× end-to-end; p99 5× lower. **Type:** measured. **Arch:** `cost-model-driven`, `simd`, `versioned-locks`, `inline-keys`, `prefix-keys`, `verus-in-ci`, `shuttle-in-ci`.
- [crossbeam](https://github.com/crossbeam-rs/crossbeam) - epoch-based memory reclamation + concurrent queues/deques. **Blazing:** the substrate half of modern lock-free Rust runs on. **Type:** case-study. **Arch:** `epoch-reclamation`, `lock-free`.
- [dashmap](https://github.com/xacrimon/dashmap) - concurrent hash map. **Blazing:** sharded locking beats global `RwLock<HashMap>` by design. **Type:** case-study. **Arch:** `sharding`.
- [papaya](https://github.com/ibraheemdev/papaya) - concurrent hash map, read-optimized counterpoint to sharded designs. **Blazing:** built around read throughput with incremental resizing. **Type:** case-study. **Arch:** `read-optimized`, `hyaline-reclamation`.

## Async Runtimes & I/O

- [glommio](https://github.com/DataDog/glommio) - thread-per-core io_uring runtime. **Blazing:** no cross-thread scheduling, direct io_uring. **Type:** case-study. **Arch:** `thread-per-core`, `io-uring`, `sharding`.
- [monoio](https://github.com/bytedance/monoio) - ByteDance's thread-per-core runtime. **Blazing:** io_uring-first, production at scale. **Type:** case-study. **Arch:** `thread-per-core`, `io-uring`.
- [io-uring](https://github.com/tokio-rs/io-uring) - raw io_uring bindings. **Blazing:** one syscall for many ops - the kernel interface built to be ignored by no one. **Type:** case-study. **Arch:** `io-uring`, `batching`.

## Storage & Embedded Databases

- [redb](https://github.com/cberner/redb) - pure-Rust ACID B-tree database. **Blazing:** LMDB-class numbers documented in its bench suite; the File backend stays within a ~2× margin against the removed (unsound-in-practice) mmap backend per the 0.14.0 changelog. **Type:** measured. **Arch:** `copy-on-write`.
- [fjall](https://github.com/fjall-rs/fjall) - LSM storage engine. **Blazing:** benchmarked against rocksdb/sled with open methodology; bbtree swap-in partner. **Type:** case-study. **Arch:** `lsm`, `partitioned`.
- [SlateDB](https://github.com/slatedb/slatedb) - LSM on object storage (S3/R2). **Blazing:** cloud-native LSM; bbtree e2e partner. **Type:** case-study. **Arch:** `lsm`, `object-storage`.
- [sled](https://github.com/spacejam/sled) - the famous beta embedded DB. **Blazing:** the project that set the tone for "blazing" claims - treat numbers as historical. **Type:** case-study. **Arch:** `lsm-plus-btree`.

## Graph & Knowledge Graphs

- [qdrant](https://github.com/qdrant/qdrant) - vector database in Rust: filtered ANN search, quantization. **Blazing:** publishes an open benchmark suite that compares engines at comparable recall@k - speed claims are quality-adjusted, not raw. **Type:** case-study. **Arch:** `hnsw`, `quantization`, `quality-vs-speed`.

- [petgraph](https://github.com/petgraph/petgraph) - THE graph data-structure crate. **Blazing:** the default choice; algorithmic core used across the ecosystem. **Type:** case-study. **Arch:** `generic-graph`.
- [rustworkx](https://github.com/Qiskit/rustworkx) - IBM's graph library (Python API requires adaptation, not drop-in NetworkX; `rustworkx-core` is the pure-Rust crate). **Blazing:** documented multi-x wins over pure-Python NetworkX at Qiskit scale. **Type:** case-study. **Arch:** `python-bridge`, `parallel-algos`.
- [oxigraph](https://github.com/oxigraph/oxigraph) - RDF/SPARQL store. **Blazing:** embedded+server SPARQL with active perf discipline; persistence is RocksDB-backed by default (feature-gated in-memory alternative). **Type:** case-study. **Arch:** `sparql-engine`, `storage-backend-flag`.
- [cozo](https://github.com/cozodb/cozo) - embedded Datalog graph DB. **Blazing:** recursive queries as a first-class, stored-procedure-free primitive. **Type:** case-study. **Arch:** `datalog`, `embedded`.

## Search & Text Processing

- [fst](https://github.com/BurntSushi/fst) - finite state transducers for large sets and maps with streaming construction. **Blazing:** ~16,000,000 Wikipedia article titles (384 MB of strings) compress to a 157 MB fst with fast prefix/fuzzy queries (author's 2015 measurement - historical, tied to that setup). **Type:** case-study. **Arch:** `finite-state-transducer`, `compact-serialization`.

- [tantivy](https://github.com/quickwit-oss/tantivy) - full-text search engine. **Blazing:** benchmarked against Lucene, frequently ahead on ingest. **Type:** case-study. **Arch:** `inverted-index`, `columnar`.
- [aho-corasick](https://github.com/BurntSushi/aho-corasick) - multi-pattern matching. **Blazing:** the prefilter behind regex/ripgrep multi-literal search. **Type:** case-study. **Arch:** `automata-compilation`, `simd`.
- [winnow](https://github.com/winnow-rs/winnow) - parser combinators. **Blazing:** zero-copy parsing as the default path. **Type:** case-study. **Arch:** `zero-copy`, `combinators`.

## Data & Analytics

- [sonic-rs](https://github.com/cloudwego/sonic-rs) - SIMD JSON parsing straight into serde-compatible structs (no intermediate tape re-parse). **Blazing:** published benchmarks vs serde_json and simd-json; the `sanitize` feature needed for LLVM sanitizers costs ~30% in serialization - production builds keep it off. **Type:** measured. **Arch:** `simd`, `typed-parse-direct`.
- [simd-json](https://github.com/simd-lite/simd-json) - SIMD JSON parsing into a tape, following the simdjson design in Rust. **Blazing:** performance tracks the C++ simdjson implementation; best numbers require a custom allocator (mimalloc/jemalloc). **Type:** case-study. **Arch:** `simd`, `tape-representation`.

- [polars](https://github.com/pola-rs/polars) - DataFrames. **Blazing:** documented multi-x wins over pandas on groupby/join; query-optimized, lazy, parallel. **Type:** case-study. **Arch:** `columnar`, `query-optimization`, `simd`.
- [apache-arrow-rs](https://github.com/apache/arrow-rs) - Arrow columnar format. **Blazing:** zero-copy interchange - the "no-serialization" tax advantage. **Type:** case-study. **Arch:** `columnar`, `zero-copy`, `stringview`.
- [datafusion](https://github.com/apache/datafusion) - SQL query engine. **Blazing:** the engine behind many "faster than X" blog posts; StringView and bbtree-grade extensibility. **Type:** case-study. **Arch:** `query-optimization`, `columnar`, `extensible`.

## Caching & Working Set

The win here is avoiding expensive misses - judge these by hit ratio on realistic admission policies, not by single-get latency.

- [moka](https://github.com/moka-rs/moka) - concurrent cache, Rust port of Caffeine ideas: admission/eviction policies, sync and async APIs. **Blazing:** the leverage is Window-TinyLFU admission keeping hot keys resident; hit-ratio studies are published in the wiki. **Type:** case-study. **Arch:** `window-tinylfu`, `hit-ratio-first`.
- [foyer](https://github.com/foyer-rs/foyer) - hybrid RAM + disk cache for data-intensive systems. **Blazing:** eviction-policy benchmark harness (hit-ratio benches) lives in the repo - policy choice is measured, not vibes. **Type:** case-study. **Arch:** `hybrid-cache`, `hit-ratio-first`.

## Incremental Computation

Recompute nothing you can update: define the computation once, respond to changes.

- [differential-dataflow](https://github.com/TimelyDataflow/differential-dataflow) - incremental data-parallel computation on timely dataflow. **Blazing:** in the author's graph example (10M nodes, 50M edges), a 15 s initial computation answers input changes in ~230 us - about 100,000x faster than recomputing; batched updates average ~5 us each; two workers cut the build from 15 s to ~8 s. **Type:** measured. **Arch:** `incremental-view-maintenance`, `arrangement-indexing`, `data-parallel`.

## Networking & Protocols

- [quinn](https://github.com/quinn-rs/quinn) - QUIC implementation. **Blazing:** 0-RTT, multiplexed streams without head-of-line blocking. **Type:** case-study. **Arch:** `async-native`, `tls-13`.
- [hyper](https://github.com/hyperium/hyper) - HTTP implementation. **Blazing:** the substrate of axum/reqwest; TechEmpower regular. **Type:** case-study. **Arch:** `async-native`.
- [pingora](https://github.com/cloudflare/pingora) - Cloudflare's proxy framework. **Blazing:** in Cloudflare's migration report, ~70% less CPU and ~67% less memory than the previous proxy at the same traffic. **Type:** measured. **Arch:** `connection-coalescing`, `work-stealing`.

## Web & Edge

- [axum](https://github.com/tokio-rs/axum) - ergonomic HTTP framework. **Blazing:** TechEmpower top-tier via hyper. **Type:** case-study. **Arch:** `zero-macro-ergonomics`.
- [wasmtime](https://github.com/bytecodealliance/wasmtime) - WebAssembly runtime. **Blazing:** Cranelift JIT + Spectre-mitigating design; runs in Firefox, Fastly, Docker. **Type:** case-study. **Arch:** `jit`, `sandboxing`.
- [rkyv](https://github.com/rkyv/rkyv) - zero-copy serialization. **Blazing:** zero-copy *access* to the archived representation; validation and full reconstruction are separate, individually costed steps. **Type:** case-study. **Arch:** `zero-copy`, `relative-pointers`.

## Science & Computational

- [faer](https://github.com/sarah-quinones/faer-rs) - linear algebra foundation: dense matrices, decompositions, eigenproblems. **Blazing:** published benchmark plots against BLAS/LAPACK backends; kernels use runtime SIMD dispatch via pulp. **Type:** case-study. **Arch:** `simd`, `runtime-dispatch`.

- [ndarray](https://github.com/rust-ndarray/ndarray) - N-dimensional arrays. **Blazing:** the substrate scientific Rust computes on. **Type:** case-study. **Arch:** `columnar-layout-optional`, `views`.
- [nalgebra](https://github.com/dimforge/nalgebra) - linear algebra, statically sized. **Blazing:** compile-time dims → unrolled SIMD-friendly kernels. **Type:** case-study. **Arch:** `const-generics`, `simd`.
- [candle](https://github.com/huggingface/candle) - Hugging Face's tensor library. **Blazing:** minimal-deps CPU inference that made HF blogs measurable. **Type:** case-study. **Arch:** `backend-trait`, `no-python`.
- [burn](https://github.com/tracel-ai/burn) - deep learning framework. **Blazing:** backend-agnostic; documented wins from kernel fusion on some models. **Type:** case-study. **Arch:** `backend-trait`, `kernel-fusion`.
- [rapier](https://github.com/dimforge/rapier) - 2D/3D physics. **Blazing:** real-time rigid-body simulation benchmarks published with each release. **Type:** case-study. **Arch:** `simd`, `broad-phase-narrow-phase`.
- [rust-bio](https://github.com/rust-bio/rust-bio) - bioinformatics algorithms. **Blazing:** pairwise alignment, BWT, variant calling - the algorithms section of computational biology. **Type:** case-study. **Arch:** `classical-algorithms`.

## Crypto & Proof Systems

- [paramo1d](https://github.com/ignotusnemo/paramo1d) - mobile blockchain verification client with recursive proofs. **Blazing:** mainnet sync in 48.43 s on a $100 Redmi 15C against its own cold start (4 GB RAM; prepared matrix caches in place); state proof from genesis 10.78 s; 192.2 MiB peak for the verification client process; 249,885-header chain in 7.44 s. **Type:** measured. **Arch:** `memory-bounded-streaming`, `recursive-proofs`, `vendor-pinning`, `poseidon2-gf2`.
- [arkworks](https://github.com/arkworks-rs) - zk-SNARK ecosystem. **Blazing:** the research-grade finite-field/curve stack most new proof systems benchmark against. **Type:** case-study. **Arch:** `finite-fields`, `no-std-friendly`.

## Verification & Correctness

Performance you can trust - proofs and dynamic testing as CI gates.

- [verus](https://github.com/verus-lang/verus) - verified Rust. **Blazing (for trust):** SMT-backed proofs checked per PR - bbtree ships proofs in-repo as a performance guard. **Type:** tool. **Arch:** `proof-carrying-performance`.
- [miri](https://github.com/rust-lang/miri) - interpreter catching UB. **Blazing (for trust):** runs bbtree and friends' test suites to keep the unsafe honest. **Type:** tool. **Arch:** `miri-in-ci`.
- [shuttle](https://github.com/awslabs/shuttle) - concurrent testing as a library. **Blazing (for trust):** thousands of schedules per PR; pairs with fork-based DST (ffs) that caught what Shuttle missed (bbtree issue #9). **Type:** tool. **Arch:** `dst-in-ci`.

## Observability & Profiling

- [uringscope](https://github.com/rch0wdhury/uringscope) - eBPF tracer for io_uring (C+BPF companion for Rust io_uring apps). **Blazing:** named findings with fixes - "BATCH: you pay a syscall per request" and "PUNT: this is where your p99 lives". **Type:** case-study. **Arch:** `findings-as-verdicts`, `btf-probe`, `kernel-matrix-in-ci`.
- [cargo-flamegraph](https://github.com/flamegraph-rs/flamegraph) - flamegraphs from cargo. **Blazing:** the first tool you reach for when "blazing" needs evidence. **Type:** tool. **Arch:** `perf-integration`.
- [criterion](https://github.com/bheisler/criterion.rs) - statistics-driven benchmarking. **Blazing:** regression detection with outlier resistance. **Type:** tool. **Arch:** `statistical-rigor`.
- [divan](https://github.com/nvzqz/divan) - fast, smart benchmarking. **Blazing:** allocation & wall-time profiling with less boilerplate. **Type:** tool. **Arch:** `alloc-tracking`.

## Agent Skills

Projects that ship their tribal knowledge as an agent-readable `SKILL.md` - because in 2026 the first user is often an AI agent. Entry links point at the skill file itself.

- [uringscope SKILL.md](https://github.com/rch0wdhury/uringscope/blob/main/skills/uringscope/SKILL.md) - for the eBPF io_uring tracer: when-to-reach-for triggers phrased as symptoms (p99 regressions, iou-wrk-* thread storms, syscall-per-op drift). **Blazing:** turns profiling output into named verdicts. **Type:** agent-skill. **Arch:** `skills-in-repo`, `findings-as-verdicts`. See [ADR-005](decisions/005-skills-in-repo.md).

## UI & Apps

- [zed](https://github.com/zed-industries/zed) - the editor built on GPUI. **Blazing:** reported ~128-222 MB RSS vs multi-GB VS Code in published comparisons ([zed.dev/compare/vscode](https://zed.dev/compare/vscode)). **Type:** measured. **Arch:** `gpu-rendered-ui`, `own-renderer`.
- [gpui-kit](https://github.com/longbridge/gpui-kit) - component library for GPUI ("shadcn for GPUI"). **Blazing:** brings widget completeness to raw GPUI (which critics note lacks text input). **Type:** case-study. **Arch:** `gpu-rendered-ui`, `component-layer`.
- [crux](https://github.com/redbadger/crux) - shared Rust core, native shells (Elm architecture). **Blazing:** one core, UIs in SwiftUI/Kotlin/React - typegen proves every effect is handled. **Type:** case-study. **Arch:** `elm-architecture`, `typed-effects`, `typegen`.
- [egui](https://github.com/emilk/egui) - immediate-mode GUI. **Blazing:** the pragmatic winner of the 2026 Rust GUI survey. **Type:** case-study. **Arch:** `immediate-mode`.
- [slint](https://github.com/slint-ui/slint) - declarative UI. **Blazing:** embedded-friendly; the other 2026 survey winner. **Type:** case-study. **Arch:** `declarative-dsl`, `embedded-first`.

## Benchmarking & Toolchain

- [pulp](https://github.com/sarah-quinones/pulp) - write SIMD kernels once, dispatch to AVX2/AVX-512/NEON at run time. **Blazing:** removes the compile-time SIMD fork dilemma; the scalar path remains the baseline for measurement. **Type:** case-study. **Arch:** `runtime-dispatch`, `simd`.
- [cargo-pgo](https://github.com/Kobzol/cargo-pgo) - PGO and BOLT for Rust binaries from cargo. **Blazing:** profile-guided inlining and code layout on real workloads; validate on a workload different from the profiling one. **Type:** case-study. **Arch:** `pgo`, `code-layout`.

- [gungraun](https://github.com/gungraun/gungraun) - former iai-callgrind; Valgrind-based instruction-count benchmarking. **Blazing:** deterministic instruction and cache-metric counts for CI regression gates (specific metrics, not a blanket "no noise" claim). **Type:** tool. **Arch:** `valgrind-based`.
- [dhat](https://github.com/nnethercote/dhat-rs) - heap profiling. **Blazing:** the tool behind most "we cut allocations 10×" posts. **Type:** tool. **Arch:** `alloc-tracking`.

## Wow & Humor

- [uutils/coreutils](https://github.com/uutils/coreutils) - GNU coreutils rewritten in Rust. **Wow:** ships as the default in Ubuntu 25.10. History, running.
- [fish-shell](https://github.com/fish-shell/fish-shell) - fish 4.0 rewrote from C++ to Rust. **Wow:** a decade-old codebase migrated, got faster and safer.
- [Servo](https://github.com/servo/servo) - the browser engine that came back from the dead. **Wow:** again upstream-active, embedded experiments (visionOS prototype included).
- [COSMIC](https://github.com/pop-os/cosmic-epoch) - System76's desktop environment in Rust. **Wow:** a full DE, iced-based, shipping on real hardware.
- [Redox](https://github.com/redox-os/redox) - a complete OS in Rust. **Wow:** drivers, GUI, shell - everything.
- [human-panic](https://github.com/rust-cli/human-panic) - crash reports humans can read, with the famous "Well, this is embarrassing" banner. **Humor:** empathy as a feature.
- [ferris-says](https://github.com/mgattozzi/ferris-says) - the crab says things. **Humor:** the official mascot as a CLI.

## Recipes

Symptom-driven guides: from a developer's problem to the patterns, projects and limits. Contributions welcome - a recipe must end in a verifiable experiment, not a vibe.

- [A graph that fits in memory and traverses fast](https://github.com/rager306/awesome-blazing-rust/blob/main/recipes/compact-graph.md) - String dictionary, dense ids, CSR, bitmap filters, hot-set caching.
- [Throughput does not scale with threads](https://github.com/rager306/awesome-blazing-rust/blob/main/recipes/thread-scaling.md) - Read locks that write, sharding, allocation contention.
- [JSON parsing is the top CPU consumer](https://github.com/rager306/awesome-blazing-rust/blob/main/recipes/json-cpu.md) - Scope first (Amdahl), then selective parse, SIMD parse, tape, or skip parsing entirely.
- [Build artifacts eating your disk](https://github.com/rager306/awesome-blazing-rust/blob/main/recipes/build-disk-hygiene.md) - Debuginfo is the bulk: 24 MB to 0.8 MB on a zero-dep crate, replicated on a second host.

## Measured Caveats

|Caveat                                                                                                                                                                                                                                                                                                                                                               |Evidence                                                                               |
|---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------|---------------------------------------------------------------------------------------|
|The compiler is not always your friend: at small N, hand-written SIMD beats auto-vectorization (NEON count 1678 to 407 instructions); at tiny N, branchless scalar beats both. Linear search beats binary search up to ~16-32 elements: the prefetcher changes the rules. RwLock can be slower than a lock: read-lock increments are writes in cache-coherence terms.|[bbtree design post](https://bbtree.xiangpeng.systems/post)                            |
|macOS numbers are not Windows numbers: WebView2 is Chromium-class, a macOS RAM win can vanish on Windows.                                                                                                                                                                                                                                                            |[Tauri vs Electron memory measurements](https://www.gethopp.app/blog/tauri-vs-electron)|
|Raw GPUI is not GPUI Component: no built-in text input; the component layer is the usable part.                                                                                                                                                                                                                                                                      |[Rust GUI Survey 2026](https://blog.wybxc.cc/blog/rust-gui-survey-2026/)               |

## Fresh

This section is the intake of a weekly research pipeline. Lifecycle of an entry:

**Spotted → Sources verified → Measurement formatted → Accepted → Due for re-check.**

|Stage                |Meaning                                                                                                                                           |
|---------------------|--------------------------------------------------------------------------------------------------------------------------------------------------|
|Spotted              |Found in release notes, papers, or our research digest.                                                                                           |
|Sources verified     |Every number traced to a primary source - retold READMEs do not count.                                                                            |
|Measurement formatted|The `Blazing:` line carries claim, baseline, and scope ([METHODOLOGY](https://github.com/rager306/awesome-blazing-rust/blob/main/METHODOLOGY.md)).|
|Accepted             |The entry moves from Fresh into a permanent section.                                                                                              |
|Due for re-check     |Old results stay useful when honestly dated; versions drift.                                                                                      |
|Replicated (top tier)|The catalog ran its own benchmark ([runs/](https://github.com/rager306/awesome-blazing-rust/tree/main/runs)), independent of upstream numbers.    |

Last full review: 2026-10-25. Community finds are welcome via PR - same bar applies.

- [blaze](https://github.com/indexable-inc/blaze) - incremental rebuilds beside cargo: keeps the project compiled in memory, recompiles only changed functions (Cranelift) and patches the program cargo built. **Blazing:** author benchmarks, edit-rebuild on real projects, Linux x86-64: ripgrep 508 -> 56 ms, helix 2.15 s -> 116 ms, bun (1.25M lines) 9.11 s -> 451 ms vs cargo build; ~20 ms save-to-run with live edits ([method and per-platform numbers](https://github.com/indexable-inc/blaze/blob/main/docs/benchmarks.md)). **Type:** measured. **Arch:** `jit`, `hot-patch`.
- [dbdelve](https://github.com/ShayanAbbas1/dbdelve) - native multi-engine database client on GPUI (Postgres, MySQL, SQLite, DuckDB, MongoDB, Redis and more), no Electron/JVM. **Blazing:** author benchmark vs DBeaver 25.3.1 (M1 Pro, 2026-10, median of 5, alternating runs): launch 1.1 s vs 5.2 s, idle memory 133 MB vs 606 MB, 1M-row grid 56 s vs 208 s at 4.9 GB vs 17 GB ([method](https://github.com/ShayanAbbas1/dbdelve/blob/main/BENCHMARKS.md)). **Type:** measured. **Arch:** `gpu-rendered-ui`.
- [strict-future-contract](https://github.com/oconnor663/rfcs/blob/future_contract_squashed/text/0000-strict-future-contract.md) - RFC draft (by the blake3 author): a pending future that requests a wakeup must be **polled promptly or dropped** - cancel anytime, never snooze. **Blazing:** mechanism claim, no benchmark: snoozing hidden in common patterns (select! by reference through the `Pin<&mut _>` impl, `StreamExt::next`) withholds wakeups and surfaces as futurelock deadlocks, missed wakeups, timeouts and UI stutter ([Futurelock case study](https://rfd.shared.oxide.computer/rfd/0609)). **Type:** case-study. **Arch:** `wakeup-contract`.
- [nachalnik](https://github.com/ljedrz/nachalnik) - transparent agent runtime (context, tools, permissions and requests as explicit user-editable state, append-only event log, no `unsafe`, no sockets in the core) plus `kamchatka`, a terminal agent with a kernel-sandboxed shell (Landlock + seccomp, asked when a command *actually tries* the network), per-item context control with undo, and byte-for-byte reproducible releases. **Blazing:** mechanism claims, no benchmark: every documented invariant has a red test, `cargo mutants` survivors get a test or a written excuse it can't be caught, model-introspection eval makes "why do you think that?" falsifiable, and build-profile decisions in Cargo.toml carry their own measurements (fat LTO 10.8 MB vs 14.3 default; `opt-level="s"` saves 3.5 MB for 15-18% slower screen draws). **Type:** case-study. **Arch:** `sandboxing`, `invariants-as-tests`.
- [taffy](https://github.com/DioxusLabs/taffy) - flexbox and CSS Grid layout engine in Rust powering Servo, Bevy, Zed (GPUI), Slint and Lapce; per-node fixed-size measure cache with bit-packed keys (two `Option<f32>` + available space in one `u64`, requested-axis bits hidden in f32 sign bits), spec-conformance tests generated from CSS Working Group fixtures by the `gentest` tooling. **Blazing:** author benchmarks vs Yoga 0.4.0 (M1 Pro, criterion, commits pinned): deep 100k-node hierarchies 63.8 vs 76.8 ms and super-deep 472 µs vs 555 µs (-58..-71% vs its own 0.2 after cache rework), but wide 100k-node trees 247 vs 136 ms - ~1.8x slower there; regressions published alongside wins in-tree ([method and full tables](https://github.com/DioxusLabs/taffy/blob/main/benches/results-2023-02-08.md)). **Type:** measured. **Arch:** `cache-conscious-layout`.
- [flashraw](https://github.com/ybouane/flashraw) - memory-safe camera RAW decoder in Rust: ~40 formats, 1,980 of 2,016 raw.pixls.us corpus files decode, multithreaded even single-stream codecs (lossless JPEG, Nikon, Pentax) via speculative parallel decoding, explicit SIMD kernels (SSSE3/SSE4.1, NEON, WASM SIMD128), fused single-pass RAW-to-RGB on CPU or GPU (WGSL for Metal/Vulkan/DX12/WebGPU), bounds-checked parsing with pre-checked allocation limits and 15 in-tree fuzz targets; Rust/C/C++/Python (NumPy zero-copy)/WASM bindings plus a LibRaw-compatible C API subset. **Blazing:** author benchmark vs LibRaw 0.22, RawSpeed, rawler, rawpy, dcraw (62 popular + 159-file corpus, median of 7 runs in a fresh process per (file, library, mode, threads), every build flag documented, raw data and charts in-tree): RAW-to-RGB 8.8x vs LibRaw on Ryzen 9700X and 15.4x on M1 Ultra all-cores (59/59 files), 2.1x/1.8x single-threaded; RawSpeed is faster single-threaded on Apple Silicon (median 0.83x) - a "where FlashRaw is not the fastest" section is part of the docs ([method](https://github.com/ybouane/flashraw/blob/main/docs/benchmarks.md)). **Type:** measured. **Arch:** `simd`, `data-parallel`, `speculative-parallel-decode`.

## Candidates


Measured-numbers pending. Promote with a PR that includes evidence.

- [mimalloc](https://github.com/microsoft/mimalloc) - allocator with documented wins vs jemalloc/tcmalloc on server workloads. *Needs: a Rust-context benchmark citation.*.
- [kutha-graph](https://github.com/rager306/kutha-graph) - greenfield graph engine (ours). *Needs: first public bench run.*.
- [respan-gateway](https://github.com/respanai/respan-gateway) - single-binary AI gateway in Rust: any client format (OpenAI Chat/Responses, Anthropic Messages, Gemini) to any model with streaming/tools/reasoning translation, cross-vendor realtime voice, MCP tools inside chat, hedged requests, plugins compiled in on a four-hook trait. **Blazing:** author claim 0.07 ms added latency, 56k RPS, 31 MB, "~3× the next fastest gateway" - no published methodology. *Needs: benchmark method (hardware, competitor versions, request profile) or a catalog own-run.*.

## See also

- [awesome-rust](https://github.com/rust-unofficial/awesome-rust) - The encyclopedic Rust list (everything, not just measured speed).
- [This Week in Rust](https://this-week-in-rust.org) - The weekly newsletter.
- [maze: B-trees, Pages and Denis Villeneuve's Prisoners](https://piyushyaps.in/posts/b-trees-pages-and-denis-villeneuves-prisoners) - Build-log of a first storage engine in Rust: slotted pages, B+tree fanout math (4 KiB page, 290 routing cells, depth 3 for 40M rows), SQLite page-type codes and the honest open questions ([code](https://github.com/piyushnotvibing/maze)).

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md). Short version: **evidence first** - link the benchmark, name the architecture pattern, tag the entry. PRs adding entries without numbers will be asked to move them to Candidates.

[![CC0-1.0 License](https://img.shields.io/badge/license-CC0--1.0-blue.svg)](LICENSE)

