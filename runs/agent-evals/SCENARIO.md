# Agent evals: three outcomes, not two

The intake and discovery prompts are non-deterministic. The same prompt, model
and tools can produce a card, a refusal, or nothing at all - and "nothing at
all" is not the same failure as "a bad card". On 2026-10-27 a model session came
up with an empty deferred-tool registry and answered without touching a single
file; a binary pass/fail would have reported that as a quality regression.

Following AgentAssay (arXiv 2603.02601, 2026) and OpenAI's skill-eval practice
(prompt -> captured run -> checks -> score), the harness reports:

| Outcome | Meaning |
|---|---|
| **PASS** | every check held |
| **FAIL** | the agent produced a card and a property was wrong |
| **INCONCLUSIVE** | no parsable output at all: environment or model flakiness, not a quality signal |

## Checks applied to a produced card

1. Required markers present: `**Blazing:**`, `**Type:**`, `**Arch:**`.
2. `**Measured by:**` present whenever the type is `measured` - the invariant
   `abr check` enforces.
3. Every URL in the card returns 2xx or 3xx. A hallucinated link is the failure
   mode that matters most: it survives review because it looks like evidence.
4. The card inserted into a scratch copy of the catalog passes `abr check`.

## Cost and cadence

Five fixtures, minutes each. This is not a CI gate - it runs on demand when a
prompt, model or tool changes, and the result is written next to this file.


## A/B: does the catalog index in the prompt change outcomes?

Measured by `ab-index.sh`: each fixture runs twice — the policy file only
(baseline) and the policy file with `llms.txt` appended (treatment). One run per
cell, five fixtures, the verdict recorded verbatim.

Question it answers: does an inlined catalog index (passive context) change
whether the agent produces a card at all — Vercel's 2026 finding was that
discretionary retrieval fails silently in the majority of cases.

Limits: one run per cell, so the numbers are directional. A real claim needs
several runs per cell and a report of the abstention (INCONCLUSIVE) rate — an
INCONCLUSIVE cell is environment noise, not an outcome, and must not be counted
as pass or fail.
