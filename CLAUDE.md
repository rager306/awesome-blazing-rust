# CLAUDE.md

Catalog of measurably fast Rust projects. The Rust CLI in `cli/` (abr) generates every derived artifact from `catalog/projects.json`.

## Non-negotiable rules

- Evidence ladder: `Blazing` (claimed) < `Measured` (public methodology) < `Replicated` (we re-ran it). Never invent numbers. A `Measured` claim needs a linkable methodology; `Replicated` needs a run in `runs/`.
- You propose, gates decide: never treat your own output as verified. The deterministic verifiers are `awesome-lint` and `abr check` (CI runs both).
- Never push, never commit, never edit files unless a task explicitly says otherwise. Read-only by default — the pipeline mediates every write.
- Public repo: all repo content in English.

## Where things live

- Cards: `README.md` (house format: dense one-liner, `**Blazing:**` evidence with numbers and methodology link, `**Type:**`, `**Arch:**` tags). Draft zone: `## Candidates`.
- Catalog source of truth: `catalog/projects.json`, synced from README with `abr catalog`. Derived: `PATTERNS.md`, `llms.txt` — regenerate with the abr CLI, never hand-edit.
- Upstream scan: `catalog/upstream.json` (latest tags) + `catalog/upstream-state.json` (consecutive-failure ledger; 3 strikes = removal candidate under phase-2 policy; >3% unreachable in one run = incident, state frozen).
- Methodology history: `runs/RESULTS.md`.

## House style for tasks

- Report facts with file paths and numbers you actually saw; say "not found" instead of guessing.
- Verify claims against primary sources (repo, docs, benchmarks), not aggregators.
- Card drafts: follow the existing card format exactly, link the benchmark methodology, list license and maintenance status, end with a `verdict:` line — `fits` / `fits-with-caveats` / `does-not-fit` + one sentence why.
