---
name: tail-latency-triage
description: Diagnose p99 latency regressions in Rust services from telemetry output and source. Use when p50 is healthy but p99/p99.9 spikes, or when slow events cluster periodically.
---

# Tail Latency Triage

Skill for tail-regression diagnosis: from symptom shape to root cause without a profiler.

## Step 1 - classify the spike shape

Look at the slowest-event distribution in telemetry output:

- **Periodic clustering** (slow events at regular index intervals, e.g. every 512th/1024th): the cost is amortized work - something big runs on a period. Dividing slow-event indices by the spacing gives the period.
- **Random scatter**: contention (locks, allocator stalls, page faults) or I/O interleaved into the hot path.
- **Monotonic drift upward over time**: unbounded growth - leak, unbounded queue, or growing per-op cost.
- **Decoy — top-N slowest list dominated by environmental bursts**: scheduler/steal stalls (often near process exit, wall ≫ CPU) can occupy the entire "slowest events" list while the true p99 mass is a large population of *uniformly* slow periodic events, none of which individually enters the top N. Cross-check: the count of events above p99 (~1% of total) vs TOTAL/period — if they match, a periodic population sets p99 and the listed outliers are a decoy. Confirm by instrumenting the suspected path and comparing per-class medians (`i % period`), and by wall-vs-CPU time on the outliers (wall ≫ CPU = stolen time, environmental; wall ≈ CPU = real compute).

## Step 2 - periodic shape: hunt amortized O(n)

When spikes are periodic, look for work whose cost is O(state) executed on a period:

- `Vec::insert(0, x)` / `remove(0)` on a long buffer - O(n) memmove per call; a 1M-element buffer shifts megabytes in nanoseconds-scale hot loops.
- Periodic rehash/regrow of maps sized by accumulated state.
- Histogram/rotation jobs: compacting, snapshotting, flushing at counts or intervals.
- `truncate`/`drain` of accumulators that only run when a cap is hit.

One common shape (verify, don't assume): when the slow path's cost scales with accumulated state, spike spacing times per-event fast-path cost roughly equals the spike magnitude - e.g. a 1M-element `insert(0)` every 512 events can produce spikes ~2^10 times the fast path. Spacing and per-event cost can also drift independently, so confirm the suspected path by instrumenting it, not from arithmetic alone.

## Step 3 - fix classes

- Ring buffer (`VecDeque`, fixed capacity, `push_back` + `pop_front`) instead of shifting insert - O(1) both sides.
- Defer compaction: append-only + occasional batch rebuild, amortized outside the hot path.
- Move the periodic work to a separate thread/task if the data allows.

## Verify

Re-run the same workload; the fix must collapse p99 to within a small factor of p50, and p50 must stay within run-to-run noise. If p50 moved beyond noise, explain it before accepting the fix: either it touched the fast path (intended? measure the new cost) or the environment drifted (re-run the baseline).
