# METHODOLOGY

How evidence in this catalog is judged. A number in a `Blazing:` line is necessary but not sufficient - comparability rules decide whether the number means anything for *your* workload.

## 1. Same work must be compared

Different project classes have different comparability requirements; a claim that skips these is tagged as such:

| Domain | Must match (or be stated) |
|---|---|
| Databases | durability, transaction guarantees, batching, value sizes, cache state |
| JSON / serialization | validation, numeric exactness, owned vs borrowed results, full vs selective parsing |
| Concurrent structures | operation semantics, read/write mix, key distribution, reclamation cost |
| Graph algorithms | graph representation, direction, density, identical output |
| ANN / vector search | retrieval quality vs speed trade-off (recall@k), index parameters, filtering |
| UI | identical task, app state, background processes, memory accounting method |

## 2. Preparation cost is part of the cost

Distinguish explicitly: index build vs query time; first run vs warm state; steady-state size vs peak build memory; one-shot operation vs long-running updates. A prepared-cache setup (see paramo1d in the README) is a different measurement than a cold start.

Write down three things a reader needs to use the number outside this repository:

- **The version measured** - a tag or commit, not a branch. A number attached to "main" has no meaning a month later.
- **The limits** - where the result stops applying: hardware class, dataset shape, thread count, cache state. Unknown fields are written as unknown, never guessed.
- **A correction is a new run**, never a silent edit of the old one. History is the value; a rewritten claim has none.

**Outcome versus process.** A number that describes the artifact is evidence; a number that describes the author's effort is not. "2.5 ms vs 1879 ms on 10M ids" is an outcome. "247 sites studied", "24 questions asked", "316 stars", "12 months of work" are process counts: the author can produce them alone in an afternoon, and no reader can use them to predict anything about their own workload. Process counts never qualify a card as `measured`.

## 3. More than one metric

No universal "fastest". Practical claims should say which of these they improve, and which they may worsen: throughput, median and tail latency, working vs peak memory, build/preparation time, update cost, correctness and portability limits. A structure whose win is memory (fitting a bigger working set) is a different claim than one whose win is lookup speed.

## 4. scope: micro vs component vs end-to-end

Every measured entry states its scope. Amdahl applies: a 2× win on a component that is 30% of runtime is ~1.18× end-to-end. Catalog numbers name their scope; extrapolating is the reader's move, but at least it is an informed one.

## 5. Who measured, what is reproducible, when

- **Who:** the project author, a third party, or this catalog's own runs. Every `measured` card states it in a `**Measured by:**` marker - `author`, `third-party` or `catalog` - because the three are different evidence and a reader must be able to tell them apart without following the link. A `catalog` claim must link its run under `runs/`; `abr check` enforces both.
- **What:** published result only / benchmark code and data available / independently replicated.
- **When:** measurement date. Reading this README is not a re-confirmation; old results remain useful if honestly tied to their historical versions.

## 6. Known limits of this catalog

- Author-reported numbers are taken at face value with methodology as stated. Where the claim is central, the catalog reruns the measurement itself (runs/: roaring-rs replicated 2026-10; the rest remain author-reported pending re-checks).
- Baselines are named because "5× faster" without "than what, on which machine, at which thread count" is marketing, not evidence.

## 7. Context signals are not evidence

`catalog/signals.json` records third-party context per repository - crates.io adoption, RUSTSEC advisories, maintenance state. It is deliberately **one-way**: a signal can hold or demote a card, never raise its `**Type:**`.

- Machine flags are facts only: `security` (an advisory inside a two-year window) and `unmaintained` (archived, or no push in 12 months).
- Anything requiring judgement is a human `verdict`, preserved across recollects; a `measured` card carrying a contested verdict fails `abr check`.
- Mention counts (HN, stars) are a loss function for anomaly detection. They are never a score and never appear on a card.
- Automatic dispute extraction was tried and rejected: 21 pointers across 94 repositories, zero of them a critique of the project's own numbers (runs/signals-2026-10/RESULTS.md).
