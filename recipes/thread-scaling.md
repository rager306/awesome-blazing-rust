# Recipe: Throughput does not scale with threads

**Symptom:** "I added threads and throughput barely moved" (or dropped).

## Measure first

1. Per-thread CPU: are all threads busy, or is one at 100% (serialization point)?
2. `perf c2c` / `perf lock` for contention and false sharing; `offcputime` for time parked on locks.
3. Allocation rate: a global allocator is a shared cache line with a queue attached.

## Usual causes, in hit order

1. **A "read lock" that is a write.** `RwLock::read()` increments a reader count - a cache-line write. Under load the line ping-pongs between sockets and the "shared" lock serializes everyone. See the `versioned-locks` pattern ([bbtree](../README.md#concurrency--lock-free): readers validate a version counter with retry instead).
2. **One structure, all threads.** Shard it - by key range (ordered maps) or hash (concurrent maps). [papaya](../README.md#concurrency--lock-free) is the read-optimized counterpoint: it bets on read throughput with incremental resizing instead of coarse sharding.
3. **Allocation contention.** Count allocations per operation; switch to per-thread arenas or a scalable allocator before blaming your algorithm.
4. **Work too fine.** Steal granularity: scheduling overhead per item can exceed the item's work. Batch.
5. **The workload is latency-shaped.** For I/O fan-out, a work-stealing async runtime ([tokio](../README.md#async-runtimes--io)) beats a thread pool; for CPU-bound maps, the reverse.

## Where it still will not scale

- A single hot key: sharding does not split one cache line. Redesign the data layout, not the locking.
- Reads that mutate shared counters (metrics!) - aggregate per thread, merge at read.
- NUMA: past ~2 sockets, "shared memory" is a polite fiction; partition by socket first.

## Checkpoint

Scaling stop at ~N threads with all CPUs idle-pointed at one line in `perf` means one shared resource. Find the line before adding more threads.
