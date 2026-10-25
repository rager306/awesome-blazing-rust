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

- [ripgrep](https://github.com/BurntSushi/ripgrep) - recursive grep with regex support. **Blazing:** repeatedly benchmarks 10×+ faster than GNU grep on large trees (see the project's own fgrep/grep comparisons). **Arch:** `simd`, `lock-free-parallel-walk`, `zero-copy`.
- [memchr](https://github.com/BurntSushi/memchr) - substring & multi-substring search. **Blazing:** SIMD-accelerated; the backbone under `regex` and `ripgrep`. **Arch:** `simd`, `branchless`.
- [regex](https://github.com/rust-lang/regex) - the regex engine. **Blazing:** literal optimizations + SIMD-powered DFA; its docs document multi-x speedups vs naive engines. **Arch:** `automata-compilation`, `simd`, `prefilter`.
- [tokio](https://github.com/tokio-rs/tokio) - the async runtime most of this list runs on. **Blazing:** work-stealing scheduler; powers production at AWS, Microsoft, Discord. **Arch:** `work-stealing`, `io-driver`.
- [hyperfine](https://github.com/sharkdp/hyperfine) - command-line benchmarking tool. **Blazing:** statistical warm-up/outlier analysis in a CLI; the tool we benchmark this list with. **Arch:** `statistical-rigor`.

## Algorithms & Data Structures

- [sux](https://github.com/vigna/sux-rs) - succinct structures: Elias-Fano, rank/select, static functions, minimal perfect hashing. **Blazing:** scales to trillions of keys with very fast queries executed directly on the compressed representation. **Arch:** `succinct`, `rank-select`, `zero-copy`.

Well-known algorithms, implemented properly in Rust.

### Collections

- [hashbrown](https://github.com/rust-lang/hashbrown) - SwissTable hash map; the implementation inside Rust's std since 1.36. **Blazing:** open addressing with SIMD control-byte probing. **Arch:** `simd`, `cache-conscious-layout`.
- [indexmap](https://github.com/indexmap-rs/indexmap) - insertion-ordered hash map + set, O(1) amortized with order. **Blazing:** std-level speed with deterministic iteration. **Arch:** `compact-index`.
- [im](https://github.com/bodil/im-rs) - immutable/persistent collections with structural sharing (archived 2026 - historical entry). **Blazing:** O(1) clones - cheap snapshots for concurrent and functional workloads. **Arch:** `structural-sharing`, `hamt`.
- [smallvec](https://github.com/servo/rust-smallvec) - stack-allocated small vectors. **Blazing:** eliminates heap allocation for the common short case. **Arch:** `inline-storage`.
- [itertools](https://github.com/rust-itertools/itertools) - the algorithm toolbox on iterators. **Blazing:** zero-cost abstractions, fused adapters. **Arch:** `zero-copy`.

### Hashing & Checksums

- [blake3](https://github.com/BLAKE3-team/BLAKE3) - cryptographic hash. **Blazing:** ~GB/s on modern x86 (AVX2/AVX-512, per project benchmarks); orders above SHA-2/SHA-3. **Arch:** `simd`, `merkle-tree`, `multi-core-hash`.
- [xxhash-rust](https://github.com/cescou/xxhash-rust) - xxHash (XXH3). **Blazing:** non-crypto hashing at memory-bandwidth speeds. **Arch:** `simd`.
- [ahash](https://github.com/tkaitchuck/aHash) - DoS-resistant fast hash map hasher. **Blazing:** documented multi-fold wins over SipHash on hash-map workloads. **Arch:** `aes-ni`.
- [crc32fast](https://github.com/srijs/rust-crc32fast) - CRC-32. **Blazing:** 15× faster than naive, via slicing-by-8 and SIMD switching. **Arch:** `simd`, `table-driven`.

### Pathfinding & Spatial

- [pathfinding](https://github.com/samueltardieu/pathfinding) - BFS, Dijkstra, A*, K-means, CSPs and friends. **Blazing:** the Swiss-army graph-algorithms crate. **Arch:** `generic-vertices`.
- [rstar](https://github.com/Stoeoef/rstar) - R*-tree spatial index. **Blazing:** bulk-loaded queries at interactive rates. **Arch:** `bulk-loading`.

### Math & Parallel

- [num-bigint](https://github.com/rust-num/num-bigint) - arbitrary-precision integers. **Blazing:** Karatsuba multiplication in the num-rational stack. **Arch:** `classical-algorithms`.
- [rayon](https://github.com/rayon-rs/rayon) - data parallelism. **Blazing:** near-linear scaling on parallel sort/map in typical workloads. **Arch:** `work-stealing`, `join-split`.

### Compression

- [lz4_flex](https://github.com/pseitz/lz4_flex) - pure-Rust LZ4. **Blazing:** no C dependency; the safe configuration avoids `unsafe` and is benchmarked separately from the unsafe-feature build. **Arch:** `pure-rust-port`, `feature-gated-safety`.
- [zstd-safe](https://github.com/gyscos/zstd-rs) - zstd bindings. **Blazing:** the ratio/speed king of general compression, one call away. **Arch:** `c-binding`.

## Concurrency & Lock-free

- [bbtree](https://github.com/XiangpengHao/bbtree) - concurrent ordered map built from a cost model. **Blazing:** 5× std `BTreeMap` at 1 thread; 12× crossbeam-skiplist at 96 threads; map-only swap into fjall/SlateDB/Tonbo/Turso/Lance: 1.16-6.83× end-to-end; p99 5× lower. **Arch:** `cost-model-driven`, `simd`, `versioned-locks`, `inline-keys`, `prefix-keys`, `verus-in-ci`, `shuttle-in-ci`.
- [crossbeam](https://github.com/crossbeam-rs/crossbeam) - epoch-based memory reclamation + concurrent queues/deques. **Blazing:** the substrate half of modern lock-free Rust runs on. **Arch:** `epoch-reclamation`, `lock-free`.
- [dashmap](https://github.com/xacrimon/dashmap) - concurrent hash map. **Blazing:** sharded locking beats global `RwLock<HashMap>` by design. **Arch:** `sharding`.
- [papaya](https://github.com/ibraheemdev/papaya) - concurrent hash map, read-optimized counterpoint to sharded designs. **Blazing:** built around read throughput with incremental resizing. **Arch:** `read-optimized`, `hyaline-reclamation`.

## Async Runtimes & I/O

- [glommio](https://github.com/DataDog/glommio) - thread-per-core io_uring runtime. **Blazing:** no cross-thread scheduling, direct io_uring. **Arch:** `thread-per-core`, `io-uring`, `sharding`.
- [monoio](https://github.com/bytedance/monoio) - ByteDance's thread-per-core runtime. **Blazing:** io_uring-first, production at scale. **Arch:** `thread-per-core`, `io-uring`.
- [io-uring](https://github.com/tokio-rs/io-uring) - raw io_uring bindings. **Blazing:** one syscall for many ops - the kernel interface built to be ignored by no one. **Arch:** `io-uring`, `batching`.

## Storage & Embedded Databases

- [redb](https://github.com/cberner/redb) - pure-Rust ACID B-tree database. **Blazing:** LMDB-class numbers documented in its bench suite; the File backend sits within ~2× of the removed (unsound-in-practice) mmap backend per the 0.14.0 changelog. **Arch:** `copy-on-write`.
- [fjall](https://github.com/fjall-rs/fjall) - LSM storage engine. **Blazing:** benchmarked against rocksdb/sled with open methodology; bbtree swap-in partner. **Arch:** `lsm`, `partitioned`.
- [SlateDB](https://github.com/slatedb/slatedb) - LSM on object storage (S3/R2). **Blazing:** cloud-native LSM; bbtree e2e partner. **Arch:** `lsm`, `object-storage`.
- [sled](https://github.com/spacejam/sled) - the famous beta embedded DB. **Blazing:** the project that set the tone for "blazing" claims - treat numbers as historical. **Arch:** `lsm-plus-btree`.

## Graph & Knowledge Graphs

- [qdrant](https://github.com/qdrant/qdrant) - vector database in Rust: filtered ANN search, quantization. **Blazing:** publishes an open benchmark suite that compares engines at comparable recall@k - speed claims are quality-adjusted, not raw. **Arch:** `hnsw`, `quantization`, `quality-vs-speed`.

- [petgraph](https://github.com/petgraph/petgraph) - THE graph data-structure crate. **Blazing:** the default choice; algorithmic core used across the ecosystem. **Arch:** `generic-graph`.
- [rustworkx](https://github.com/Qiskit/rustworkx) - IBM's graph library (Python API requires adaptation, not drop-in NetworkX; `rustworkx-core` is the pure-Rust crate). **Blazing:** documented multi-x wins over pure-Python NetworkX at Qiskit scale. **Arch:** `python-bridge`, `parallel-algos`.
- [oxigraph](https://github.com/oxigraph/oxigraph) - RDF/SPARQL store. **Blazing:** embedded+server SPARQL with active perf discipline; persistence is RocksDB-backed by default (feature-gated in-memory alternative). **Arch:** `sparql-engine`, `storage-backend-flag`.
- [cozo](https://github.com/cozodb/cozo) - embedded Datalog graph DB. **Blazing:** recursive queries as a first-class, stored-procedure-free primitive. **Arch:** `datalog`, `embedded`.

## Search & Text Processing

- [fst](https://github.com/BurntSushi/fst) - finite state transducers for large sets and maps with streaming construction. **Blazing:** ~16,000,000 Wikipedia article titles (384 MB of strings) compress to a 157 MB fst with fast prefix/fuzzy queries (author's 2015 measurement - historical, tied to that setup). **Arch:** `finite-state-transducer`, `compact-serialization`.

- [tantivy](https://github.com/quickwit-oss/tantivy) - full-text search engine. **Blazing:** benchmarked against Lucene, frequently ahead on ingest. **Arch:** `inverted-index`, `columnar`.
- [aho-corasick](https://github.com/BurntSushi/aho-corasick) - multi-pattern matching. **Blazing:** the prefilter behind regex/ripgrep multi-literal search. **Arch:** `automata-compilation`, `simd`.
- [winnow](https://github.com/winnow-rs/winnow) - parser combinators. **Blazing:** zero-copy parsing as the default path. **Arch:** `zero-copy`, `combinators`.

## Data & Analytics

- [sonic-rs](https://github.com/cloudwego/sonic-rs) - SIMD JSON parsing straight into serde-compatible structs (no intermediate tape re-parse). **Blazing:** published benchmarks vs serde_json and simd-json; the `sanitize` feature needed for LLVM sanitizers costs ~30% in serialization - production builds keep it off. **Arch:** `simd`, `typed-parse-direct`.
- [simd-json](https://github.com/simd-lite/simd-json) - SIMD JSON parsing into a tape, following the simdjson design in Rust. **Blazing:** performance tracks the C++ simdjson implementation; best numbers require a custom allocator (mimalloc/jemalloc). **Arch:** `simd`, `tape-representation`.

- [polars](https://github.com/pola-rs/polars) - DataFrames. **Blazing:** documented multi-x wins over pandas on groupby/join; query-optimized, lazy, parallel. **Arch:** `columnar`, `query-optimization`, `simd`.
- [apache-arrow-rs](https://github.com/apache/arrow-rs) - Arrow columnar format. **Blazing:** zero-copy interchange - the "no-serialization" tax advantage. **Arch:** `columnar`, `zero-copy`, `stringview`.
- [datafusion](https://github.com/apache/datafusion) - SQL query engine. **Blazing:** the engine behind many "faster than X" blog posts; StringView and bbtree-grade extensibility. **Arch:** `query-optimization`, `columnar`, `extensible`.

## Caching & Working Set

The win here is avoiding expensive misses - judge these by hit ratio on realistic admission policies, not by single-get latency.

- [moka](https://github.com/moka-rs/moka) - concurrent cache, Rust port of Caffeine ideas: admission/eviction policies, sync and async APIs. **Blazing:** the leverage is Window-TinyLFU admission keeping hot keys resident; hit-ratio studies are published in the wiki. **Arch:** `window-tinylfu`, `hit-ratio-first`.
- [foyer](https://github.com/foyer-rs/foyer) - hybrid RAM + disk cache for data-intensive systems. **Blazing:** eviction-policy benchmark harness (hit-ratio benches) lives in the repo - policy choice is measured, not vibes. **Arch:** `hybrid-cache`, `hit-ratio-first`.

## Incremental Computation

Recompute nothing you can update: define the computation once, respond to changes.

- [differential-dataflow](https://github.com/TimelyDataflow/differential-dataflow) - incremental data-parallel computation on timely dataflow. **Blazing:** in the author's graph example (10M nodes, 50M edges), a 15 s initial computation answers input changes in ~230 us - about 100,000x faster than recomputing; batched updates average ~5 us each; two workers cut the build from 15 s to ~8 s. **Arch:** `incremental-view-maintenance`, `arrangement-indexing`, `data-parallel`.

## Networking & Protocols

- [quinn](https://github.com/quinn-rs/quinn) - QUIC implementation. **Blazing:** 0-RTT, multiplexed streams without head-of-line blocking. **Arch:** `async-native`, `tls-13`.
- [hyper](https://github.com/hyperium/hyper) - HTTP implementation. **Blazing:** the substrate of axum/reqwest; TechEmpower regular. **Arch:** `async-native`.
- [pingora](https://github.com/cloudflare/pingora) - Cloudflare's proxy framework. **Blazing:** in Cloudflare's migration report, ~70% less CPU and ~67% less memory than the previous proxy at the same traffic. **Arch:** `connection-coalescing`, `work-stealing`.

## Web & Edge

- [axum](https://github.com/tokio-rs/axum) - ergonomic HTTP framework. **Blazing:** TechEmpower top-tier via hyper. **Arch:** `zero-macro-ergonomics`.
- [wasmtime](https://github.com/bytecodealliance/wasmtime) - WebAssembly runtime. **Blazing:** Cranelift JIT + Spectre-mitigating design; runs in Firefox, Fastly, Docker. **Arch:** `jit`, `sandboxing`.
- [rkyv](https://github.com/rkyv/rkyv) - zero-copy serialization. **Blazing:** zero-copy *access* to the archived representation; validation and full reconstruction are separate, individually costed steps. **Arch:** `zero-copy`, `relative-pointers`.

## Science & Computational

- [faer](https://github.com/sarah-quinones/faer-rs) - linear algebra foundation: dense matrices, decompositions, eigenproblems. **Blazing:** published benchmark plots against BLAS/LAPACK backends; kernels use runtime SIMD dispatch via pulp. **Arch:** `simd`, `runtime-dispatch`.

- [ndarray](https://github.com/rust-ndarray/ndarray) - N-dimensional arrays. **Blazing:** the substrate scientific Rust computes on. **Arch:** `columnar-layout-optional`, `views`.
- [nalgebra](https://github.com/dimforge/nalgebra) - linear algebra, statically sized. **Blazing:** compile-time dims → unrolled SIMD-friendly kernels. **Arch:** `const-generics`, `simd`.
- [candle](https://github.com/huggingface/candle) - Hugging Face's tensor library. **Blazing:** minimal-deps CPU inference that made HF blogs measurable. **Arch:** `backend-trait`, `no-python`.
- [burn](https://github.com/tracel-ai/burn) - deep learning framework. **Blazing:** backend-agnostic; documented wins from kernel fusion on some models. **Arch:** `backend-trait`, `kernel-fusion`.
- [rapier](https://github.com/dimforge/rapier) - 2D/3D physics. **Blazing:** real-time rigid-body simulation benchmarks published with each release. **Arch:** `simd`, `broad-phase-narrow-phase`.
- [rust-bio](https://github.com/rust-bio/rust-bio) - bioinformatics algorithms. **Blazing:** pairwise alignment, BWT, variant calling - the algorithms section of computational biology. **Arch:** `classical-algorithms`.

## Crypto & Proof Systems

- [paramo1d](https://github.com/ignotusnemo/paramo1d) - mobile blockchain verification client with recursive proofs. **Blazing:** mainnet sync in 48.43 s on a $100 Redmi 15C (4 GB RAM; prepared matrix caches in place, not a cold start); state proof from genesis 10.78 s; 192.2 MiB peak for the verification client process; 249,885-header chain in 7.44 s. **Arch:** `memory-bounded-streaming`, `recursive-proofs`, `vendor-pinning`, `poseidon2-gf2`.
- [arkworks](https://github.com/arkworks-rs) - zk-SNARK ecosystem. **Blazing:** the research-grade finite-field/curve stack most new proof systems benchmark against. **Arch:** `finite-fields`, `no-std-friendly`.

## Verification & Correctness

Performance you can trust - proofs and dynamic testing as CI gates.

- [verus](https://github.com/verus-lang/verus) - verified Rust. **Blazing (for trust):** SMT-backed proofs checked per PR - bbtree ships proofs in-repo as a performance guard. **Arch:** `proof-carrying-performance`.
- [miri](https://github.com/rust-lang/miri) - interpreter catching UB. **Blazing (for trust):** runs bbtree and friends' test suites to keep the unsafe honest. **Arch:** `miri-in-ci`.
- [shuttle](https://github.com/awslabs/shuttle) - concurrent testing as a library. **Blazing (for trust):** thousands of schedules per PR; pairs with fork-based DST (ffs) that caught what Shuttle missed (bbtree issue #9). **Arch:** `dst-in-ci`.

## Observability & Profiling

- [uringscope](https://github.com/rch0wdhury/uringscope) - eBPF tracer for io_uring (C+BPF companion for Rust io_uring apps). **Blazing:** named findings with fixes - "BATCH: you pay a syscall per request" and "PUNT: this is where your p99 lives". **Arch:** `findings-as-verdicts`, `btf-probe`, `kernel-matrix-in-ci`.
- [cargo-flamegraph](https://github.com/flamegraph-rs/flamegraph) - flamegraphs from cargo. **Blazing:** the first tool you reach for when "blazing" needs evidence. **Arch:** `perf-integration`.
- [criterion](https://github.com/bheisler/criterion.rs) - statistics-driven benchmarking. **Blazing:** regression detection with outlier resistance. **Arch:** `statistical-rigor`.
- [divan](https://github.com/nvzqz/divan) - fast, smart benchmarking. **Blazing:** allocation & wall-time profiling with less boilerplate. **Arch:** `alloc-tracking`.

## Agent Skills

Projects that ship their tribal knowledge as an agent-readable `SKILL.md` - because in 2026 the first user is often an AI agent. Entry links point at the skill file itself.

- [uringscope SKILL.md](https://github.com/rch0wdhury/uringscope/blob/main/skills/uringscope/SKILL.md) - for the eBPF io_uring tracer: when-to-reach-for triggers phrased as symptoms (p99 regressions, iou-wrk-* thread storms, syscall-per-op drift). **Blazing:** turns profiling output into named verdicts. **Arch:** `skills-in-repo`, `findings-as-verdicts`. See [ADR-005](decisions/005-skills-in-repo.md).

## UI & Apps

- [zed](https://github.com/zed-industries/zed) - the editor built on GPUI. **Blazing:** reported ~128-222 MB RSS vs multi-GB VS Code in published comparisons ([zed.dev/compare/vscode](https://zed.dev/compare/vscode)). **Arch:** `gpu-rendered-ui`, `own-renderer`.
- [gpui-kit](https://github.com/longbridge/gpui-kit) - component library for GPUI ("shadcn for GPUI"). **Blazing:** brings widget completeness to raw GPUI (which critics note lacks text input). **Arch:** `gpu-rendered-ui`, `component-layer`.
- [crux](https://github.com/redbadger/crux) - shared Rust core, native shells (Elm architecture). **Blazing:** one core, UIs in SwiftUI/Kotlin/React - typegen proves every effect is handled. **Arch:** `elm-architecture`, `typed-effects`, `typegen`.
- [egui](https://github.com/emilk/egui) - immediate-mode GUI. **Blazing:** the pragmatic winner of the 2026 Rust GUI survey. **Arch:** `immediate-mode`.
- [slint](https://github.com/slint-ui/slint) - declarative UI. **Blazing:** embedded-friendly; the other 2026 survey winner. **Arch:** `declarative-dsl`, `embedded-first`.

## Benchmarking & Toolchain

- [pulp](https://github.com/sarah-quinones/pulp) - write SIMD kernels once, dispatch to AVX2/AVX-512/NEON at run time. **Blazing:** removes the compile-time SIMD fork dilemma; the scalar path remains the baseline for measurement. **Arch:** `runtime-dispatch`, `simd`.
- [cargo-pgo](https://github.com/Kobzol/cargo-pgo) - PGO and BOLT for Rust binaries from cargo. **Blazing:** profile-guided inlining and code layout on real workloads; validate on a workload different from the profiling one. **Arch:** `pgo`, `code-layout`.

- [gungraun](https://github.com/gungraun/gungraun) - former iai-callgrind; Valgrind-based instruction-count benchmarking. **Blazing:** deterministic instruction and cache-metric counts for CI regression gates (specific metrics, not a blanket "no noise" claim). **Arch:** `valgrind-based`.
- [dhat](https://github.com/nnethercote/dhat-rs) - heap profiling. **Blazing:** the tool behind most "we cut allocations 10×" posts. **Arch:** `alloc-tracking`.

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

## Measured Caveats

|Caveat                                                                                                                                                                                                                                                                                                                                                               |Evidence                                                                               |
|---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------|---------------------------------------------------------------------------------------|
|The compiler is not always your friend: at small N, hand-written SIMD beats auto-vectorization (NEON count 1678 to 407 instructions); at tiny N, branchless scalar beats both. Linear search beats binary search up to ~16-32 elements: the prefetcher changes the rules. RwLock can be slower than a lock: read-lock increments are writes in cache-coherence terms.|[bbtree design post](https://bbtree.xiangpeng.systems/post)                            |
|macOS numbers are not Windows numbers: WebView2 is Chromium-class, a macOS RAM win can vanish on Windows.                                                                                                                                                                                                                                                            |[Tauri vs Electron memory measurements](https://www.gethopp.app/blog/tauri-vs-electron)|
|Raw GPUI is not GPUI Component: no built-in text input; the component layer is the usable part.                                                                                                                                                                                                                                                                      |[Rust GUI Survey 2026](https://blog.wybxc.cc/blog/rust-gui-survey-2026/)               |

## Fresh

This week's finds from our research pipeline. New entries land here first, graduate into sections once verified.

Entries land here first from the weekly research pipeline, then graduate into permanent sections. Community finds are welcome via PR.

## Candidates

- [roaring-rs](https://github.com/RoaringBitmap/roaring-rs) - Roaring compressed bitmaps for IDs, postings and set operations. In-repo bench harness and real datasets exist, but no published Rust-specific result yet - numbers from the Java or C implementations do not transfer automatically.

Measured-numbers pending. Promote with a PR that includes evidence.

- [mimalloc](https://github.com/microsoft/mimalloc) - allocator with documented wins vs jemalloc/tcmalloc on server workloads. *Needs: a Rust-context benchmark citation.*.
- [kutha-graph](https://github.com/rager306/kutha-graph) - greenfield graph engine (ours). *Needs: first public bench run.*.

## See also

- [awesome-rust](https://github.com/rust-unofficial/awesome-rust) - The encyclopedic Rust list (everything, not just measured speed).
- [This Week in Rust](https://this-week-in-rust.org) - The weekly newsletter.

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md). Short version: **evidence first** - link the benchmark, name the architecture pattern, tag the entry. PRs adding entries without numbers will be asked to move them to Candidates.

[![CC0-1.0 License](https://img.shields.io/badge/license-CC0--1.0-blue.svg)](LICENSE)

