# ADR-005: skills-in-repo

**Status:** emerging (small but growing set of projects)
**Origin:** [uringscope](https://github.com/rch0wdhury/uringscope) (`skills/uringscope/SKILL.md`)
**Adopted by:** uringscope

## Context (the problem)

In 2026 a large share of first contact with a tool happens through an AI agent, not a human reading docs. Agents need the same tribal knowledge that used to live in one maintainer's head: when to reach for the tool, what its failure signatures look like, which flags matter. Without it, agents misapply the tool and humans get blamed.

## Technique

Ship a **SKILL.md inside the repository** - next to the code it documents:

- **When-to-reach-for triggers**, phrased as observable symptoms: for uringscope - "p99 regressions on io_uring paths", "hundreds of iou-wrk-* threads", "syscall-per-op ratio climbing", "silent buffer races", "hung completions".
- Failure signatures and what they mean, not just feature lists.
- The file travels with the binary (same release, same commit) - docs and code cannot drift, the same discipline as `test/kernels.txt`.

## Evidence

- uringscope ships `skills/uringscope/SKILL.md` in-repo with symptom-triggered entry points; it is the fourth observed instance of the "tool + agent skill in one artifact" pattern across the projects we track in 2026.
- Effect is measured indirectly but consistently in agent-first workflows: tools with skill files get picked correctly on the first try; tools without them get hallucinated flags.

## Trade-offs / when NOT to use

- A stale SKILL.md is worse than none: wrong triggers actively misroute agents. Keep it adjacent to the code changes that invalidate it (same PR culture).
- Not every tool deserves a skill: if the entry point is "run `tool --help`", the README is enough. Skills earn their place when *judgment* is the scarce resource (profilers, verifiers, storage engines).
- Don't duplicate the man page - the skill encodes decision knowledge, not flags.

## References

- [uringscope `skills/uringscope/SKILL.md`](https://github.com/rch0wdhury/uringscope/blob/main/skills/uringscope/SKILL.md).
- Anthropic agent-skills format (`SKILL.md` with YAML frontmatter) - the de-facto standard this pattern rides on.
