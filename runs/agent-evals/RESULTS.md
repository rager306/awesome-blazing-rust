# Agent evals: first full run

Run: 2026-10-28, five fixtures, `runs/agent-evals/run.sh`, model `glm-5.3-flash`.
Harness: `run.sh`; design and outcome semantics: `SCENARIO.md`.

## Result

| Fixture | Expected | Outcome | What happened |
|---|---|---|---|
| `astral-sh/uv` | card | **PASS** | markers, links and the scratch `abr check` all held |
| `nushell/nushell` | card | **INCONCLUSIVE** | the session answered without a parsable card object |
| `serde-rs/serde` | card | **INCONCLUSIVE** | same |
| `bodil/im-rs` | skip | **PASS** | refused: "archived (archived:true, last push 2024-08-19, im 15.1.0)" |
| `facebook/react` | skip | **PASS** | refused: "not a Rust project (GitHub primary language: JavaScript)" |

**3 pass, 0 fail, 2 inconclusive.** No card failed a check: among the sessions
that produced a card, no invented link, no missing marker, no `abr check`
rejection. The two inconclusive runs produced no card object at all.

## What the run actually demonstrates

1. **Both branches work.** The refusals are not luck: the agent read the
   archive flag and the language from the GitHub API and said why. That is the
   freshness gate and the Rust-only scope operating as designed.
2. **A produced card integrates.** The uv card passed the marker checks, the
   link resolution and `abr check` inside a scratch copy of the catalog - the
   same gauntlet the intake pipeline applies.
3. **The inconclusive rate is the finding.** Two of five sessions (40%)
   answered without the JSON object. Earlier in the day two more sessions came
   up with an empty deferred-tool registry and produced nothing at all. A binary
   pass/fail would have logged all four as quality regressions; they are not.

## Honest limits

- The harness cannot yet tell "the model declined to format" from "the model
  had no tools". Both surface as no parsable JSON. Separating them means
  capturing the tool-use trace (`stream-json`) and checking whether any tool
  ran - the next improvement, not a claim of this run.
- Five fixtures are a smoke test, not a measurement of card quality. They check
  integration, not whether the card is *good*: a plausible-looking claim with a
  wrong number would pass, because no check compares the number to its source.
- Costs real model time (~2 minutes per fixture). This is an on-demand
  instrument, deliberately not a CI gate.

## Fixes this run forced, before it could pass

- Fixtures were cataloged projects, so the scratch check fired
  `DUPLICATE_ENTRY` - correct behaviour, wrong test.
- The scratch check did not regenerate `PATTERNS.md`, rejecting legitimate new
  `Arch` tags that the pipeline indexes.
- Card extraction assumed the JSON started a line and produced false
  INCONCLUSIVE verdicts when the agent wrapped it in prose.


## A/B: catalog index in the prompt vs without

Run: 2026-10-31, five fixtures, one run per cell, harness `ab-index.sh`.
Baseline: the policy file only. Treatment: the policy file with `llms.txt`
appended. The hypothesis came from Vercel's 2026 eval, where discretionary
retrieval failed silently in the majority of cases.

| Fixture | Expected | Baseline | With index |
|---|---|---|---|
| `astral-sh/uv` | card | PASS | PASS |
| `nushell/nushell` | card | PASS | PASS |
| `serde-rs/serde` | card | PASS | PASS |
| `bodil/im-rs` (archived) | skip | PASS | PASS |
| `facebook/react` (not Rust) | card -> refuse | **FAIL** | PASS |

The one divergence is the informative cell: without the index, the agent drafted
a card for `facebook/react` - a JavaScript project - which is exactly the
silent scope failure the catalog exists to prevent. With the index, it refused.

Directional, not statistical: five fixtures, one run per cell, and the INCONCLUSIVE
rate means a missing row is not evidence of failure. The harness records this as
`runs/agent-evals/ab-index.sh`; rerun it before drawing conclusions from a single
sweep.

Process notes from the run: the first sweep was unusable because the table
recorded raw JSON instead of verdicts (`verdict_of` was defined but never
called), and the second stopped after one fixture because the model call
consumed the harness's stdin. Both are the kind of failure a fixed fixture set
catches immediately, which is the argument for keeping this harness at all.
