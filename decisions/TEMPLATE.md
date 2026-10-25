# ADR template

Each file in `decisions/` documents one architectural pattern from [PATTERNS.md](../PATTERNS.md) in depth.

Format:

```markdown
# ADR-NNN: <pattern-name>

**Status:** proposed | proven (N+ projects) | contested
**Origin:** <project / author who popularized it>
**Adopted by:** <list of entries>

## Context (the problem)

## Technique

## Evidence

## Trade-offs / when NOT to use

## References
```

Rules:
- One pattern per file, named after its PATTERNS.md tag.
- `Evidence` must cite measured numbers with their source.
- `Trade-offs / when NOT to use` is mandatory — a pattern without known limits is a pattern nobody has stressed yet.
