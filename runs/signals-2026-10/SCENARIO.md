# Signals: does third-party context add signal, or just noise?

**Question.** The catalog's only currency is reproducible numbers. Can we add a
second layer - adoption, maintenance, advisories, critique pointers - without
either (a) letting popularity become evidence or (b) drowning maintainers in
noise?

**Hypothesis.** Context is useful exactly where it is *falsifiable*: a RUSTSEC
advisory exists or it does not, a repository is archived or it is not. It is
noise where it is *quantitative opinion*: stars, mentions, "X is everywhere".
So the collector records facts, stores opinions as unverified pointers, and
never computes a score.

**Design under test.**

1. `scripts/signals/collect.py` - stdlib only, four sources (crates.io, HN
   Algolia, RUSTSEC advisory-db, GitHub via `gh`), output `catalog/signals.json`.
2. Machine flags are limited to two objective facts: `security` (an advisory
   exists) and `unmaintained` (archived, or no push in 12 months).
3. Anything requiring judgement is an **unverified dispute pointer** with a URL;
   only a human promotes it to a `verdict`, and verdicts survive recollects.
4. `abr check` enforces the asymmetry: a `measured` card carrying `contested`
   fails the build. Signals can hold or demote, never promote.

**Metrics.**

| Metric | Why it matters |
|---|---|
| crate coverage | how many of the catalog's repos are actual crates |
| advisory hits | does RUSTSEC actually intersect this catalog |
| dispute pointers, and how many survive review | precision, not recall |
| measured cards left untouched | if signals start flagging everything, the layer is useless |

**What this run cannot measure.** Hype detection. `hype-suspect` needs a
mention baseline (7-day volume against a 90-day median), and this is the first
collection - there is no history. The honest position is to record the series
now and compute the anomaly later; the flag stays off until then.

**Falsifier.** If the dispute pointers are mostly noise on manual review, or if
crate coverage is low enough that the layer describes a minority of entries,
the design is wrong and should be narrowed to advisories only.
