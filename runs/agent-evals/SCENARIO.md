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

`ab-index.sh` runs each fixture twice: the policy file only (baseline), and
that policy with `llms.txt` appended (treatment). Both cells run with the
repository root as their working directory. The baseline can still read
`llms.txt` using the permitted tools, so this compares prompt inclusion, not
access to the index. Both cells also have the same repository, tool and CLI
configuration, which can affect results.

This A/B script is separate from the card-quality checks above: it checks only
whether the output matches the fixture's expected card or skip. The model's
result must be a JSON object with a string `card`, a null or string `skip`, and
a string `notes`. For a card fixture, a nonempty `card` and empty or null
`skip` pass; for a skip fixture, a nonempty `skip` and empty `card` pass.
Whitespace-only values count as empty. Process errors, timeouts, model errors
and unparseable, missing-field or wrongly typed responses are INCONCLUSIVE. A
parsed response that does not match the expectation fails.

Run `runs/agent-evals/ab-index.sh [output-parent-directory]`; the default parent
is `/tmp`. It needs Python 3, `claude`, credentials, and access to the permitted
Claude tools. The permitted `Bash(gh api *)` tool is not a read-only sandbox:
`gh api` can make write requests, and the agent can read local files. Run only
in a trusted, suitably isolated environment with least-privilege credentials.
Each invocation creates a new private `ab-index-*` directory and
prints its path. `report.md` links to each cell's `evidence.json`, which records
the expectation, verdict, reason, process status and parsed response. Each cell
also contains captured CLI stdout (`stdout.json`, including the model response
and any metadata returned by the CLI) and stderr (`stderr.txt`). The report
identifies the requested `ANTHROPIC_MODEL`, or `default`; the actual model is
available only if the CLI includes it in its response. A timeout is 420 seconds
per cell. The CLI's `--max-turns` is 20.

Scoring uses the original response in memory. Captured output is redacted for
credential-like environment values and common API token formats before being
written, so it is not byte-for-byte raw. This is
best effort, not a guarantee: model output or errors may contain other sensitive
information. Inspect artifacts before sharing them, and do not use sensitive
fixtures or credentials in prompts. No environment dump or full prompt is
saved.

One run per cell is directional, not proof of an index effect. Multiple runs
and a report of the INCONCLUSIVE rate are needed; inconclusive results can have
several causes and must not be counted as pass or fail.
