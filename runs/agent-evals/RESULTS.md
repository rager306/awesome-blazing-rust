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

Reported run: 2026-10-31, model `glm-5.3-flash`, five fixtures, one run
per cell, harness `ab-index.sh` as recorded in commit `1840b8e`. Baseline:
the policy file only. Treatment: the policy file with `llms.txt` appended.

The checked-in [A/B results](ab-results.md) and the original version of this
report disagree about the `facebook/react` cells. Both were introduced in commit
`1840b8e`; no raw model responses or tool traces for this run were found in
`runs/agent-evals/`, and that harness version writes verdicts, not responses,
to its output file. The actual outcomes for those two cells are **unknown**; neither
account establishes which configuration refused or drafted a card.

| Fixture | Expected | Baseline | With index |
|---|---|---|---|
| `astral-sh/uv` | card | PASS (both reports) | PASS (both reports) |
| `nushell/nushell` | card | PASS (both reports) | PASS (both reports) |
| `serde-rs/serde` | card | PASS (both reports) | PASS (both reports) |
| `bodil/im-rs` (archived) | skip | PASS (both reports) | PASS (both reports) |
| `facebook/react` (not Rust) | skip | unknown: reports FAIL / PASS | unknown: reports PASS / FAIL |

For `facebook/react`, the two values in each unknown cell are, respectively,
the original version of this report and `ab-results.md`. Even if a FAIL were
confirmed, that harness version's skip check only establishes that the parsed output
did not contain a nonempty skip reason and an empty card; it does not by itself
establish that a card was drafted. For a card fixture, that version checked only
that the parsed card field was nonempty, even if a skip reason was also present.
Unlike `run.sh` at the time, that version of `ab-index.sh` did not check card
markers, links, or `abr check` for its PASS verdicts.

The conflicting record cannot support a conclusion about whether the index helped
or hurt. One run per cell is also too small to establish an effect. Future runs
need retained per-cell responses, tool traces, and verdicts to resolve discrepancies;
INCONCLUSIVE means the harness found no parsable flat JSON object with a `card`
key, not proof of either an environment failure or a model failure.

Process notes from the run: the first sweep was unusable because the table
recorded raw JSON instead of verdicts (`verdict_of` was defined but never
called), and the second stopped after one fixture because the model call
consumed the harness's stdin. Both are the kind of failure a fixed fixture set
catches immediately, which is the argument for keeping this harness at all.
