# Agent policy files

The system prompts of the repository's own agents. The workflow appends
`llms.txt` to each one, so an agent starts with the catalog's index in context
instead of deciding whether to go looking for it: Vercel's 2026 eval found
agents never invoked an available skill in 56% of cases, and a passive index
in the prompt scored 100% where on-demand retrieval scored 53%.

| File | Used by |
|---|---|
| `intake.md` | the intake workflow, one suggested repository -> one card |
| `discovery.md` | the daily discovery workflow, search -> candidates |
| `curator.md` | the nightly digest over the upstream scan |

## These are written by a person, not generated

SkillsBench (arXiv 2602.12670, 2026) measured 7,308 trajectories: curated
human-written skills raised pass rate by 16.2 points, self-generated ones
degraded it by 1.3. A rule an agent wrote for itself is not evidence that the
rule works.

So: edit these files in a pull request, with a reason. Do not have an agent
append to them from a run's outcome.
