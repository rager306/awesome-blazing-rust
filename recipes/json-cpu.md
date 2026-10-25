# Recipe: JSON parsing is the top CPU consumer

**Symptom:** "The profiler says parsing dominates my pipeline."

## Measure first

1. Scope the number: parsing % of end-to-end. If parsing is 30% of runtime, a 2x parser win is ~1.18x end-to-end - decide whether that is worth an API migration before starting one.
2. Split parse time by operation: full DOM build vs typed struct extraction vs validation. They are different costs (see [METHODOLOGY](../METHODOLOGY.md), same-work rule for JSON).

## Options, cheapest-first

1. **Don't parse what you don't need.** Selective/partial extraction; skip fields without materializing them.
2. **SIMD parsing into typed structs.** [sonic-rs](../README.md#data--analytics) parses straight into serde-compatible structs (no tape re-parse); mind its documented caveat: the `sanitize` feature costs ~30% in serialization.
3. **SIMD tape.** [simd-json](../README.md#data--analytics) builds an intermediate tape - wins when you genuinely traverse the document rather than map it to one struct; best numbers need mimalloc/jemalloc.
4. **Skip parsing entirely for stable payloads.** [rkyv](../README.md#data--analytics) archives once and reads zero-copy afterwards - the right answer when the same data is read many times and the producer/consumer can share a schema.
5. **Prefilter with `memchr`-class scans.** [memchr](../README.md#hall-of-fame) accelerators shrink the bytes that reach the parser.

## Limits

- Small documents: SIMD setup overhead can erase the win - measure at *your* sizes.
- Validation is not free: zero-copy access to an *untrusted* document requires a validation pass (rkyv) - cost it honestly.
- Owned vs borrowed results change your lifetime story, not just your speed.

## Checkpoint

Re-profile after the change. The parser share should drop; if end-to-end barely moved, the scope was never there - see Amdahl above.
