# Contributing

**The one rule: no numbers, no entry.** Every entry must link a public benchmark, measurement, or documented performance claim (the project's own README/blog counts if methodology is stated).

## Adding an entry

1. Pick the right section (or propose a new one).
2. Use this exact format:

   ```markdown
   - [name](url) — what it is, one line. **Blazing:** <the measurement with units and context>. **Arch:** `pattern-one`, `pattern-two`
   ```

3. `Blazing:` must contain *numbers with context* — "12× std BTreeMap at 96 threads", not "very fast".
4. `Arch:` tags must exist in [PATTERNS.md](PATTERNS.md) — if you coin a new one, add it to the index with a 2-3 line explanation.
5. One entry per project; cross-posting the same project into multiple sections is not allowed — pick the primary home.

## Fresh process

The **Fresh** section is fed weekly from our research pipeline. Entries land there first, then graduate into a permanent section after verification. Community finds are welcome through the normal PR route.

## Candidates

If a project is interesting but has no public numbers yet, add it to **Candidates** with a note on what evidence is needed. Maintainers and the community help chase the measurement.

## Measured Caveats

Counter-intuitive findings (compiler quirks, platform differences, misleading folk wisdom) are first-class content. PRs welcome — with the measurement, always.

## Checks

CI runs:

- `awesome-lint` — README structure and link format
- `lychee` — link checking (weekly; broken links auto-open an issue)
