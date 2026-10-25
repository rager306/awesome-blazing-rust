# ADR-002: findings-as-verdicts

**Status:** proven (1 project, research-grade)
**Origin:** [uringscope](https://github.com/rch0wdhury/uringscope)
**Adopted by:** uringscope

## Context (the problem)

Profilers and tracers answer "what happened" and stop. The human still has to turn 400 flamegraph rows and a wall of counters into a decision. Under time pressure that step is where misdiagnosis happens - and it is not reproducible.

## Technique

End the tool's output with **named verdicts**, not dashboards: each finding = (named problem, suggested fix, typed evidence). In uringscope: a findings engine (64 typed slots, `US_EV_*` evidence keys) consumes reconstructed io_uring request lifecycles and emits conclusions like:

- "BATCH: you pay a syscall per request - the thing io_uring exists to avoid."
- "PUNT: this is where your p99 lives - O_DIRECT + registered buffers."

Output formats: human report + JSON + Perfetto trace, all from the same evidence store.

## Evidence

- Kernel coverage 5.15-6.17+ via BTF-probe choosing per-kernel tracepoint variants.
- Findings engine: `src/findings.c` (typed evidence `US_EV_U64/DBL/STR`, leak detection by age, buffer-race hazard sampling).
- Reproducibility: `bench/` ships fio workloads (smallbatch, sqpoll, randread-cold) + `collect.sh` + `plot.py` - every chart in the paper regenerates.

## Trade-offs / when NOT to use

- Verdicts encode an opinion - a wrong heuristic becomes a confident wrong answer. Keep the raw evidence one click away (uringscope does: evidence travels with the verdict).
- Maintenance scales with kernel surface area; uringscope pays for it with a kernel matrix in CI (`test/kernels.txt` is the single source for docs and vmtest).
- For exploratory profiling (no hypothesis yet), dashboards are still the right first tool; verdicts are for the triage phase.

## References

- [uringscope repo](https://github.com/rch0wdhury/uringscope) - Zenodo DOI, paper PDF, `src/findings.c`, `skills/uringscope/SKILL.md`.
