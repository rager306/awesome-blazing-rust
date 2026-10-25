# Scenario: synthetic p99 regression (agent-skill experiment, ADR-005)

A single-binary Rust "service" (std-only, deterministic, runs in ~0.1 s) that
processes a 2M-event stream through a worker with a rolling history window and
prints latency quantiles plus the slowest events.

- Code: `scenario/src/main.rs` (agent-visible)
- Telemetry from a canonical run: `telemetry.txt` (agent-visible)
- Prebuilt binary: `telemetry-binary` (agents may run/modify/rebuild)

## Signal to diagnose

`p50 = 40 ns`, `p99 ~ 1.5 us` (≈ 38x p50), `p999 ~ 1.8 us`. The tail population
(~1.5% of events) is far too large to be scheduler preemption; the preemption
burst at the end of the slowest-list is a realistic distractor.

## Ground truth (for scoring only - never shown to trial agents)

Root cause: `history.insert(0, out)` in the sampled branch (`i % SAMPLE_EVERY == 0`)
is an O(n) shift of the 8192-element (64 KiB) window, executed every 64th event.
Slow-event spacing = 64; magnitude ~40x the fast path. Correct fix class: ring
buffer (`VecDeque`/`push_front`) or append-only + batch rebuild - must collapse
p99 without touching p50.

Telemetry v1 defect (found by trial subjects, fixed in v2): the slowest-events
list was built after `latencies.sort_unstable()`, so printed "event indices"
were sorted-array positions; the "decaying end cluster" was rank order plus one
host-noise burst. `telemetry.txt` = v1 (the stimulus all six trials saw);
`telemetry-v2.txt` = fixed capture on the same quiet host (true scattered
indices, identical checksums).

Scoring per trial: (1) mechanism identified (O(n) shifting insert on the
sampled history window), (2) correct fix class (ring/deque; not "remove
sampling"), (3) false leads (preemption blamed as the primary cause = fail,
scheduler noise acknowledged as secondary = ok), (4) period 64 recovered.
