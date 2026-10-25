# Contributing

**The one rule: no numbers, no entry.** Every entry must link a public benchmark, measurement, or documented performance claim (the project's own README/blog counts if methodology is stated).

## Entry types

Each entry declares what it promises. Pick one:

| Type | Promise to the reader |
|---|---|
| **Measured project** | A concrete result with conditions and a source |
| **Perf/verification tool** | What the tool measures or verifies - and what it does not guarantee |
| **Architecture case study** | Mechanism, preconditions, cost, and limits of an approach |
| **Agent skill** | Application instructions for a specific tool (effectiveness assessed separately) |

**Candidate** is a *status*, not a type: any entry awaiting evidence stays in the Candidates section.

## Measurement passport (Measured projects)

The `Blazing:` line should, at minimum, carry: the claim, the baseline it was compared against, and a link to the source. When available, add: versions/revisions, workload (data size, operation, mix), environment (CPU/OS/threads/flags), and limits (where the result does not apply). Unknown fields are written as unknown - never guessed. Three things are tracked separately: **who** measured (author / third party / this catalog), **what** is reproducible (published result only / code+data / independent replication), **when** (measurement date; reading the README is not a re-confirmation).

`scope:` distinguishes microbenchmark vs component vs end-to-end. A 2× win on a 30%-of-runtime component is ~1.18× end-to-end - say which one the number is.

## Adding an entry

1. Pick the right section (or propose a new one).
2. Use this exact format:

   ```markdown
   - [name](url) — what it is, one line. **Blazing:** <the measurement with units and context>. **Arch:** `pattern-one`, `pattern-two`
   ```

3. `Blazing:` must contain *numbers with context* — "12× std BTreeMap at 96 threads", not "very fast".
4. `Arch:` tags must exist in [PATTERNS.md](PATTERNS.md) — if you coin a new one, add it to the index with a 2-3 line explanation.
5. One entry per project; cross-posting the same project into multiple sections is not allowed — pick the primary home.

## Fresh process

The **Fresh** section is fed weekly from our research pipeline. Entries land there first, then graduate into a permanent section after verification. Community finds are welcome through the normal PR route.

## Candidates

If a project is interesting but has no public numbers yet, add it to **Candidates** with a note on what evidence is needed. Maintainers and the community help chase the measurement.

## Measured Caveats

Counter-intuitive findings (compiler quirks, platform differences, misleading folk wisdom) are first-class content. PRs welcome — with the measurement, always.

## Repo tooling

This repository is kept consistent by its own zero-dependency Rust CLI: [`cli/`](cli/) (`abr`).

```
cargo run -p abr --release -- check    # validate cross-file consistency (CI runs this)
cargo run -p abr --release -- catalog  # regenerate catalog/projects.json from README
cargo run -p abr --release -- patterns # rebuild the PATTERNS.md index table
cargo run -p abr --release -- llms     # regenerate llms.txt (agent-facing index)
```

Paths are relative to the repository root, discovered by walking up from the current directory - `abr` runs from anywhere inside the repo. After editing entries in README.md run `abr catalog` (and `abr patterns` if you added tags), then `abr check` must pass. CI additionally proves projections are deterministic: regeneration must produce a byte-identical tree. A catalog about measured speed keeps itself consistent with measured-speed tooling: zero dependencies, single 7 KB binary.

## Checks

CI runs:

- `awesome-lint` — README structure and link format
- `lychee` — link checking (weekly; broken links auto-open an issue)
