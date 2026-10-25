# ADR-004: memory-bounded-streaming

**Status:** proven (1 project, extreme constraint)
**Origin:** [paramo1d](https://github.com/ignotusnemo/paramo1d)
**Adopted by:** paramo1d

## Context (the problem)

Verification-heavy systems (recursive proofs, big indexes, ML artifacts) traditionally assume the artifact fits in memory. On a $100 phone with 4 GB RAM that assumption dies: loading proof matrices or state caches kills the process before the first block is checked.

## Technique

- Compressed matrices live **on disk** and stream through **bounded buffers**; peak memory becomes a design constant, not an accident.
- Cache rows are authenticated by compiled-in SHA-256 pins - the trust surface is documented honestly ("relies on SHA-256 second-preimage resistance and process-private cache memory").
- Supply chain is part of the memory story: `vendor/upstream.json` (hash inventory) + `client-overrides.json` (mobile adapters) + `check-upstream.py` verifying both - so the code doing the streaming is exactly the code you audited.

## Evidence

- Full mainnet sync from scratch: **48.43 s** on Redmi 15C (4 GB RAM, 8 cores, Termux, 4G+VPN).
- Recursive state proof from genesis: **10.78 s**; incremental block: 11.94-13.20 s.
- Peak process memory: **192.2 MiB** - the whole node.
- Header chain of 249,885 blocks verified in **7.44 s**.

## Trade-offs / when NOT to use

- Streaming adds complexity to every consumer: random access becomes sequential-shaped; latency-sensitive random reads suffer.
- Disk I/O replaces memory I/O - on fast NVMe the win is memory headroom; on slow flash the bottleneck moves.
- If the artifact genuinely fits in RAM on target hardware, bounded buffers are pure overhead - measure first.

## References

- [paramo1d repo](https://github.com/ignotusnemo/paramo1d) - README numbers, `vendor/upstream.json`, `check-upstream.py`.
