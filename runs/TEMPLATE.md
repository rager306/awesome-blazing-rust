# Run template

Copy the section below to `runs/<name>-<date>/RESULTS.md` and replace the
placeholders with observed facts. Follow [METHODOLOGY.md](../METHODOLOGY.md)
for evidence and comparison rules. Record `unknown` or `not applicable` rather
than guessing. This template is a starting point, not a new evidence tier or a
benchmark harness.

---

# <Run title>

- **Date:** <YYYY-MM-DD>
- **Measured by:** <catalog / author / third-party>
- **Scope:** <micro / component / end-to-end; operation and outcome>
- **Baseline:** <name and version; what work is compared>
- **Limits:** <important differences, missing measurements, applicability>

## Inputs and environment

- Source revisions: <exact commits/tags, including benchmark code and baseline>
- Workload and data: <description, dataset revision or checksum, generation commands and seeds>
- Binaries and tools: <versions, build commands and binary hashes; identify hash algorithm>
- Machine: <CPU, memory, OS, thread count and relevant resource limits>
- Configuration: <arguments, relevant environment variables, cache state and preparation costs>

## Reproduce

```sh
<commands to obtain inputs, build, prepare, and run from a stated directory>
```

- Untimed preflight: <command and result checking inputs, baseline/output equivalence and exit status>
- Measurement: <tool and version, exact command, warmups, sample count and timed region>
- Optional metrics: <for example peak RSS or retired instructions; method and units, or unavailable>
- If using hardware counters: <probe command and result before the measurement matrix; report unavailable counters rather than treating them as zero>

## Results and artifacts

- Raw samples: <path or durable link to every sample and tool export, including units and exit statuses>
- Summary: <per workload and operation, statistic and units; do not average unlike workloads together>
- Correctness: <reference and ordered output/diagnostic comparison where applicable, including exit statuses; record differences>
- Interpretation: <result, trade-offs, and limits; distinguish preparation from measured work>

Keep the inputs, commands, preflight result and raw data together or link durable
locations and checksums here. If a report is generated, record its renderer
revision/hash and command, then check a fresh render against the published
artifact; note any differences or why this cannot be done. If a correction is
needed, publish a new run and link the run it supersedes.
