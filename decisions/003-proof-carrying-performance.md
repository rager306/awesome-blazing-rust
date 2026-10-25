# ADR-003: proof-carrying-performance

**Status:** proven (1 flagship + growing ecosystem)
**Origin:** [bbtree](https://github.com/XiangpengHao/bbtree) CI ladder; [verus](https://github.com/verus-lang/verus)
**Adopted by:** bbtree

## Context (the problem)

Unsafe, lock-free, SIMD-heavy code is where performance lives and where bugs thrive. Benchmarks tell you it is fast; nothing tells you it is *correct* - until production. Traditional answer ("write it safe and slow") forfeits the speed. Verification usually lives in papers, not in CI.

## Technique

Put the correctness ladder **in CI, next to the benchmarks** - three guards, each catching a different class:

1. **Proofs (Verus)** - machine-checked invariants per PR. bbtree ships `src/proof/` with propositions like "inline keys are ordered as the keys themselves", checked on every PR.
2. **Dynamic testing (Miri + sanitizers + fuzzing)** - interpreters and sanitizers catch UB and memory errors the type system cannot.
3. **Dynamic scheduling testing (Shuttle + ffs)** - thousands of thread schedules per PR; fork-based DST (ffs) found a race Shuttle missed (bbtree issue #9) - run both.

## Evidence

- bbtree: 627 M ops/s at 96 threads *with* the full ladder on every PR - the ladder did not cost the headline numbers; it protects them.
- ffs vs Shuttle: a real data race found by fork-based exploration after Shuttle passed (bbtree #9).
- Concurrency bug cost asymmetry: one shipped race outweighs years of CI minutes.

## Trade-offs / when NOT to use

- Proof maintenance is real work: every layout change touches the spec. Budget it.
- Verus-style proofs fit data-structure invariants well; whole-system properties (distributed, I/O-timing) are still research territory.
- For prototype-stage code the ladder slows iteration - introduce it when the design freezes, and let Shuttle/Miri lead before that.

## References

- [bbtree CI workflows](https://github.com/XiangpengHao/bbtree/tree/main/.github/workflows) - verus.yml, shuttle.yml, miri.yml, sanitizer.yml, fuzz.yml.
- [verus](https://github.com/verus-lang/verus), [shuttle](https://github.com/awslabs/shuttle), [miri](https://github.com/rust-lang/miri).
