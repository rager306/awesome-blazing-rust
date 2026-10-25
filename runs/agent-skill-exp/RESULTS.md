# Own run: does a skill file improve agent diagnosis? (ADR-005 validation, 2026-10-26)

**Claim under test** ([ADR-005](../../decisions/005-skills-in-repo.md)): a triage skill shipped inside the repo measurably helps an agent diagnose a performance regression.

**Design:** blind parallel trials, 3 with the skill / 3 without. Identical task text, identical scenario, identical quiet host (2-vCPU KVM, the same machine as the build-disk-hygiene replication). Group A was given one extra sentence: the path to `skill/SKILL.md`. Model identical across all trials (glm-5.3-flash via Hermes subagents). Scoring rubric: mechanism identified, period recovered, correct fix class, verified fix (p99 collapse with p50 unchanged and bit-identical checksums), false leads.

## Scenario

`scenario/src/main.rs`: 2M-event pipeline, sampled drift-history window, quantile telemetry. Hidden defect: `Vec::insert(0, ..)` memmoves the 64 KiB window every 64th event - periodic tail, p50 30 ns vs p99 ~890 ns. Ground truth in [SCENARIO.md](SCENARIO.md).

## Results

| Trial | Skill | Mechanism | Period 64 | Fix class | Verified fix | API calls | Wall |
|---|---|---|---|---|---|---|---|
| A1 | yes | yes | yes | yes (VecDeque push_front) | yes, bit-identical | 19 | 974 s |
| A2 | yes | yes | yes | yes | yes, bit-identical | 6 | 288 s |
| A3 | yes | yes | yes | yes | yes, bit-identical | 22 | 881 s |
| B1 | no | yes | yes | yes | yes, bit-identical | 13 | 406 s |
| B2 | no | yes | yes | yes | yes, bit-identical | 23 | 873 s |
| B3 | no | yes | yes | yes | yes, bit-identical | 11 | 446 s |

**6/6 correct in both groups.** Aggregate cost: 47 API calls in each group (coincidence), mean wall 714 s (A) vs 575 s (B) - within shared-host noise, no claim either way.

## Findings

1. **No measurable skill advantage at this scale.** The scenario was below the model's difficulty floor: every trial nailed mechanism, period, and fix regardless of the skill. A ceiling effect, not a negative result about skills in general.
2. **The harness works.** Six independent agents ran, instrumented, fixed, and verified to bit-identical `checksum`/`drift-audit` outputs on a remote host - the trial protocol itself is the reusable artifact here.
3. **Qualitative skill effect is real.** A1 explicitly executed the skill's step ladder; the population-check refinement that A3 added to the skill afterwards (kept, see `skill/SKILL.md` Step 1 "Decoy") is exactly the kind of improvement the repo wants flowing back into skills.
4. **Unintended second defect found by the subjects.** v1 telemetry printed `slowest events` after `latencies.sort_unstable()`, so the "event indices" were sorted-array positions - the "decaying end cluster" was rank order plus one host-noise burst. Three trials (A2, B1, B2) independently flagged it; the strongest evidence was wall-vs-CPU on the outliers. Telemetry fixed in v2: indices captured before sorting, slowest list now shows true scattered indices (`telemetry-v2.txt`); `telemetry.txt` is preserved as the v1 stimulus all trials actually saw.
5. **Protocol lessons.** Six trials sharing one remote sandbox mutated each other's working copies (later trials detected and isolated; B3 ran in a clean private copy). A 6-wide fan-out hit provider rate limits once (one re-dispatch). Next iterations: one sandbox per trial, harder scenario (multi-cause, non-obvious period), and a weaker model tier where the ceiling is not instant.

## Verdict for ADR-005

**Not validated at n=6 / this model / this difficulty - and not falsified.** The skills-in-repo mechanism now has a working measurement harness; the claim stays open until a scenario separates the groups. The honest state of this repo's own claims remains: every number has a source - including ours.

## Reproduce

Scenario rebuild + any agent runtime; rubric in [SCENARIO.md](SCENARIO.md). Run the binary on a quiet 2-vCPU host for comparable numbers.
