# Signals: does third-party context add signal, or just noise?

Run: 2026-10-2(7), catalog at 96 entries / 94 unique GitHub repositories.
Collector: `abr signals` (the repository's zero-dependency CLI). Data: `catalog/signals.json`.

## What was measured

| Metric | Result |
|---|---|
| repositories covered | **94** (96 entries; two pairs share a repository) |
| mapped to a crates.io crate | **68 / 94** (72%) - identity verified by the crate's own `repository` field |
| repositories with any RUSTSEC advisory | 12 |
| repositories with an advisory inside the 2-year window | **4** - bytecodealliance/wasmtime, pseitz/lz4_flex, rkyv/rkyv, tokio-rs/tokio |
| `unmaintained` (archived or no push in 12 months) | **5** - BurntSushi/fst, Stoeoef/rstar, bodil/im-rs, cozodb/cozo, nnethercote/dhat-rs |
| repositories flagged at all | **9 / 94** (9.6%) |
| dispute pointers found automatically | 021, of which real: **0** |

## Finding 1 - the advisory window is the whole difference

RUSTSEC keeps every advisory forever. Flagging on presence would have marked
12 of 94 repositories "security" on the strength of CVEs from 2016-2022;
wasmtime alone carries 65 advisories. Restricting the flag to advisories from
the last two years (the year is encoded in the advisory id) brings it to 4 -
projects with a *current* exposure. Both numbers are in the data; only the
second one is actionable.

## Finding 2 - keyword dispute extraction is worthless, and that is a result

Searching HN comments for a project name plus a critique marker ("misleading",
"slower than", "cherry-pick", "does not scale") produced 21 pointers across 94
repositories. Manual review of a sample:

| Pointer | What it actually was |
|---|---|
| memchr / RUSTSEC-free thread | ripgrep's author discussing SIMD prefilters - not a critique of memchr |
| crossbeam / "cherry-pick" | feedback on a language's homepage benchmarks |
| crux / "misleading" | a discussion about LLM benchmark categories |
| hyperfine / "slower than" | a thread about jq's startup time |
| regex / "slower than" | a thread about hash maps in Rust |

A stricter rule - the comment must link the repository *and* carry the
criticism in the same sentence as the project name - matched **0 of 21**.

So the automatic dispute layer is removed. The `disputes` field stays in the
schema as a **human/agent input**: a maintainer or the intake agent records a
real critique with its URL, and only then can it become a `verdict`.

## Finding 3 - a small typed-decision model, not a big one, does the judgement

Rejecting keyword extraction did not remove the need for judgement; it moved
it. The split that remains:

| Step | Cost | Job |
|---|---|---|
| keyword prefilter | free | recall, deliberately over-inclusive (21 hits / 94 repos) |
| typed decision (`adapters/subq.rs`, SubQ Snap) | $0.04 per 1M input tokens, output free | precision: one noul question per candidate, bucketed at 0.65 / 0.35 |
| human verdict | attention | authority: only a person turns a pointer into a `contested` flag |

A yes/no probability is the right shape here: no prose to parse, no chat
session, and the cost of triaging the whole catalog's candidates is fractions
of a cent. The model is asked exactly one falsifiable question - "does this
comment criticise *this project's own* numbers" - with explicit criteria for
yes and no.

**Measured on the fixture (27 items).** Hand-labelling turned out to be the
weak link, not the model. The first review called all 21 pointers noise; two
were genuine contradictions. The first full run with a live key then produced
15 accepted pointers, and a spot check found two of them were **name
collisions**: `fst` appears as a variable in a Python snippet, `blaze` as a
different C++ library. That is what a bare crate name buys you.

The fix was a second judgement, not a bigger model: every candidate now gets
`subject` ("is this about *this* project, not a namesake") and `critique`
("does it dispute the numbers"), in one request, and the two probabilities
multiply. A name collision cannot be rescued by a confident critique score.

| Item | p (subject x critique) | Truth |
|---|---|---|
| ripgrep ("loses ~3x to ugrep in your benchmark?") | 0.97 | critique |
| polars ("benchmarking is always so unserious and biased") | 0.96 | critique |
| datafusion ("much slower than duckdb or OOMing") | 0.96 | critique |
| ntex ("those framework benchmarks are a pissing contest") | 0.85 | critique |
| tantivy (competitor: "pg_search is much slower") | 0.82 | critique |
| ruff ("Ruff is fast, sure, but no ecosystem...") | 0.95 | **noise - known false positive** |
| fst (Python variable named `fst`) | 0.04 | noise |
| blaze (C++ library, different project) | 0.34 | noise |
| 19 more | 0.00 - 0.42 | noise |

At the accept cutoff 0.65: **26 of 27 correct, 1 false positive, 0 lost
critiques**. The false positive is kept in the fixture on purpose: the eval
prints it and exits non-zero, so the number stays visible instead of being
tuned away. The product absorbs it - a pointer is a lead for a human, and only
a human `verdict` can ever demote a card.

The cutoffs are provisional: five positives are a calibration, not a
distribution. `abr signals --eval <fixture>` reproduces this table from the
shipped binary.

**On the live catalog (94 repositories):** 27 pointers survive - 17 accepted,
10 in review, 9 repositories flagged (4 `security`, 5 `unmaintained`). Roughly
a quarter of the catalog carries a benchmark dispute somewhere on HN, which is
what a speed-focused list should expect; the review band keeps the uncertain
ones out of the machine bucket.

## Finding 4 - hype detection is not measurable yet

`hype-suspect` needs a mention baseline: 7-day volume against a 90-day median
per repository. This is the first collection, so there is no baseline and the
flag stays off. What this run produced is the series start: `attention`
(stories, comments) is recorded per repository and is explicitly *never* shown
on a card and *never* used as a score.

## What the layer looks like after the experiment

Kept, because it is factual and cheap:

- `crate` - adoption (downloads, recent downloads, versions, yanks), identity
  verified against the repository URL so a name collision cannot attach
  someone else's numbers
- `github` - archived, pushed_at, stars, license
- `advisories` + the two machine flags

Rejected: automatic disputes. Deferred: hype/astroturf detection.

## Gate behaviour

`abr check` stays green (96 entries, 120 tags) with the new file. The added
invariants are `SIGNALS_UNKNOWN_FLAG` (a flag outside the vocabulary) and
`MEASURED_CONTESTED` (a `measured` card whose repository carries a contested
verdict). Neither fires today because no maintainer verdict exists yet - which
is what an empty verdict field is supposed to mean.

Unit tests: 30 pass, including the parser contract and the asymmetry rule
(a contested signal blocks a measured card and does not block a case-study).
