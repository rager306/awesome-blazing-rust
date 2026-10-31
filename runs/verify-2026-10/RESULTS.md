# Provenance and entailment: 23 measured cards

Run: 2026-10-31, `abr verify --entail` over the whole catalog.
Two questions, deliberately separate:

- **provenance** (mechanical): does the number a card states appear in the page
  the card links? Deterministic, free, can be a gate.
- **entailment** (a judgement): does that page support the claim - same metric,
  same baseline, same units? Non-deterministic, costs a request per card, can
  never be a gate.

## Result

| | |
|---|---|
| measured cards | 23 |
| claim cites no URL | **9** |
| checked against a source | 14 |
| every number present | 1 |
| some numbers missing | 13 |
| low entailment (support < 0.5) | **2** |

## Finding 1 - nine cards still name no source

`ripgrep`, `blake3`, `crc32fast`, `bbtree`, `redb`, `sonic-rs`,
`differential-dataflow`, `pingora`, `paramo1d`. Unchanged from the first run:
the evidence is implied and lives outside the card. This is the catalog's own
rule unmet, and it is the cheapest thing to fix.

## Finding 2 - a miss usually means the numbers are elsewhere, and entailment says so

The two layers disagree in a useful way. Cards where the numbers are missing but
the source plainly supports the claim:

| Card | numbers found | support |
|---|---|---|
| `taffy` | 7/16 | 0.98 |
| `jaq` | 0/20 | 0.83 |
| `ntex` | 8/13 | 0.99 |
| `statico` | 9/11 | 0.99 |
| `gitoxide` | 6/10 | 0.99 |

The pattern is placement, not invention: numbers that live in a table rendered
as an image, in a linked gist, or in the README while the card links the script
that produces them (`jaq` links `bench.sh`, whose output is the table).

## Finding 3 - two cards the entailment layer does not believe

| Card | numbers found | support | What to do |
|---|---|---|---|
| `zed` | 0/2 | **0.05** | the linked page does not support the memory figures |
| `eg-strider` | 2/9 | **0.14** | same: the numbers are not on the linked page |

Both are the case the two-layer design exists for: mechanically they look like
the other misses, but the semantic layer says the source does not back the
claim at all. Neither is proven wrong - both need a human to open the link - but
they are the only two that deserve it first.

## What the numbers do not say

- A high `support` with few numbers found is not a pass: the model may be
  agreeing with the claim's shape while the specific figures sit elsewhere.
- A low `support` is not proof of a false claim; it can mean the linked page is
  a landing page, an index, or a stub.
- Nine unlinked cards were never judged at all - the largest known gap is still
  that they carry no link to check.

## Next

1. Require a URL in every `measured` claim, and clear the nine-card backlog.
2. Promote `verify` from a report to a signal: write `numbers_found` and
   `support` into `catalog/signals.json` so the gap is visible per card over
   time, not only in a one-off run.
