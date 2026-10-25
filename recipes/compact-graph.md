# Recipe: A graph that fits in memory and traverses fast

**Symptom:** "My graph occupies too much RAM" or "traversal is slow and the profiler says cache misses".

## Measure first

1. RSS of the loaded graph vs the raw input size. A 10x blow-up from strings + pointer-heavy nodes is typical, not exceptional.
2. `perf stat -e cache-misses,LLC-load-misses` during a full traversal. If misses dominate, you have a layout problem, not an algorithm problem.
3. Edge traversal rate (edges/s) per thread - the single number to compare against after each step below.

## The chain that usually fixes it

Each step shrinks memory; the *speed* comes from the shrunken data fitting in cache. This is why compactness and speed are the same optimization here.

1. **String dictionary.** Node labels/URIs repeat; intern them once. A finite state transducer or a sorted compact structure maps strings to dense `u32` ids: [fst](../README.md#search--text-processing) (157 MB for 16M titles, 2015 measurement), [sux](../README.md#algorithms--data-structures) (rank/select, Elias-Fano, MPHFs - queries run on the compressed form).
2. **Dense ids everywhere.** After interning, edges are pairs of `u32`. A pointer-chasing `HashMap<String, Vec<String>>` becomes a flat CSR (offsets + packed neighbors).
3. **Bitmap filters on top.** Node predicates (types, marks, visibility) as Roaring bitmaps compose with `and`/`or` at cache-line speed: [roaring-rs](https://github.com/rager306/awesome-blazing-rust/blob/main/runs/roaring-2026-10/RESULTS.md) (catalog own run v2: dense union materialize 755x vs HashSet, sparse materialize only 4-16x, sparse contains 6-11x slower - density and op semantics decide; verify on your density).
4. **Traversal order.** Dense CSR + BFS/DFS in id order gives near-sequential reads; prefetcher does the rest.
5. **Hot subgraph caching.** If queries repeatedly touch a small working set, an admission-policy cache (moka, see [Caching](../README.md#caching--working-set)) beats generic LRU - hit ratio is the metric, not get latency.
6. **If the graph updates constantly.** Recompute-nothing: [differential-dataflow](../README.md#incremental-computation) answers changes in microseconds on graphs where a rebuild takes seconds.

## Where each choice loses

- Succinct structures: queries on compressed data are fast but *updates* are expensive - static dictionaries are for slowly-changing node sets.
- CSR: edge insertions force a rebuild (or a delta structure - complexity).
- Bitmap filters: pay off at moderate predicate density; on very sparse or very dense sets simpler masks win.
- Dense `u32` ids: you give up direct string addressing - every access goes through the dictionary (one extra indirection you must keep hot).

## Sanity check (Amdahl)

If traversal is 30% of your query pipeline, a 2x traversal win is ~1.18x end-to-end. Fix layout because the graph does not fit, not because a microbenchmark said so.
