# Provenance of claim numbers: first run

Run: 2026-10-28, `abr verify`, 22 `measured` cards, 7 seconds.

The check is mechanical: take the numbers a card states, fetch the page its
claim links to, and ask whether those numbers appear in the text. It answers
"was this figure invented or transposed", never "does the source support the
claim" - the two jobs are separate, and only the first is deterministic.

## Result

| | |
|---|---|
| measured cards | 22 |
| claim cites no URL at all | **9** |
| checked against a source | 13 |
| every number present | 1 |
| some numbers missing | 12 |

## Finding 1 - nine measured cards have no linkable methodology

`ripgrep`, `blake3`, `crc32fast`, `bbtree`, `redb`, `sonic-rs`,
`differential-dataflow`, `pingora`, `paramo1d` state a number in `Blazing:` and
name no URL. The evidence is implied ("per project benchmarks", "documented in
its bench suite") and lives outside the card.

This is the catalog's own rule, unmet: CONTRIBUTING asks a measured card to
carry a baseline *and a link to the source*. Nine of twenty-two do not. The
check did not exist before, so nobody knew the number.

## Finding 2 - the misses are mostly placement, not invention

Sample inspection of the twelve `MISS` rows:

- `jaq` 0/20: the claim links `bench.sh`, the script; the numbers live in the
  README table it produces.
- `ntex`, `statico`, `taffy`, `flashraw`: several figures come from tables that
  the fetched page renders as images, or from a different page of the same
  project than the one linked.
- `datafusion` "missing 13099": an issue number inside the URL, not a claim.
- `oxc` "missing 1": a weak token - `1.0` normalises to `1` - now filtered out.

So a miss is a pointer to look, not a verdict. That is why the command is
advisory and exits zero.

## What this changes

- The gap is now measured rather than assumed: 41% of measured cards cannot be
  traced from the card itself.
- Two cheapest fixes follow directly: require a URL in every `measured` claim
  (an `abr check` rule once the backlog is cleared), and make `MISS` a signal
  in `catalog/signals.json` rather than a one-off report.

## Limits

Text matching cannot see charts, PDFs, values written in prose, or numbers
rendered client-side. It also cannot tell a correctly quoted figure from a
misread one: that is entailment, and it needs a reader.
