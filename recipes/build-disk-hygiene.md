# Recipe: Build artifacts eating your disk

**Symptom:** "`target/` is 10+ GB", "the disk fills up after a week of development", "clone of a small crate built 2 GB of artifacts".

## The chain, cheapest-first

1. **Stop generating debuginfo by default.** In the workspace `Cargo.toml`:

```toml
[profile.dev]
debug = "line-tables-only"   # meaningful backtraces, most of the savings
incremental = false          # saves incremental/; slows rebuilds of changed crates
```

`debug = 0` saves even more but costs you debugger sessions and symbolized backtraces. `[profile.test]` inherits from `[profile.dev]` - do not duplicate it. Panic messages and `assert!` checks are untouched by any of this.

2. **Build only what the task needs.** `cargo check -p <pkg> --lib --tests` verifies compilation without codegen; `cargo test -p <pkg> --lib --tests` runs those tests. `--workspace --all-targets` (and doctests) belong to the pre-merge gate, not to every iteration.

3. **One target dir per project, outside throwaway worktrees:**

```bash
export CARGO_TARGET_DIR="$HOME/.cache/cargo-target/my-project"
```

This reuses artifacts across worktrees but does not cap anything - watch it with `du -sh "$CARGO_TARGET_DIR"` and clean with `cargo clean -p <pkg>` or [`cargo-sweep`](https://github.com/romac/cargo-sweep) (timestamp-based).

4. **Bring debuginfo back per-run, not per-config:**

```bash
CARGO_PROFILE_TEST_DEBUG=2 CARGO_PROFILE_TEST_STRIP=none cargo test -p <pkg>
```

Note: `STRIP=false` does **not** return information the compiler never generated (`debug = 0`). For backtraces only, `line-tables-only` is usually enough.

5. **A small final executable is a different goal** (release profile, `opt-level = "s"`, `lto`, `strip = "symbols"`, optionally `panic = "abort"` - a behavior change). Measure: `"z"` does not guarantee smaller than `"s"` or `3`, and LTO/codegen-units=1 cost build time.

## Measured (this catalog's own run, 2026-10-25)

Workload: a zero-dependency crate (this repo's `cli/`), cold `cargo build`, dev profile.

| Config | EPYC 12c (2.0 GHz) | 2-vCPU KVM (independent host) |
| --- | --- | --- |
| default dev | 24 MB | 12.2 MB / 1.32 s |
| debug = 0 + incremental = false | **0.8 MB** | **0.7 MB / 0.31 s** |
| line-tables-only | 5.0 MB / 1.21 s | 9.4 MB / 0.60 s |

Two hosts, same direction: debuginfo is the bulk of the savings. Absolute sizes vary with dependency count - the zero-dep case is the floor, a 300-dependency workspace scales it up, ratios hold. Reproduce: any crate, `CARGO_PROFILE_DEV_DEBUG=0 CARGO_TARGET_DIR=$(mktemp -d) cargo build`, then `du -sh`.

## Where it bites back

- `debug = 0`: debugger breakpoints lose variable inspection; crash reports become bare addresses. `line-tables-only` keeps function+line backtraces.
- `incremental = false`: cold builds get faster, rebuilds of *changed* crates get slower - measure on your edit-compile-test loop before committing to it.
- A shared `CARGO_TARGET_DIR` mixes artifact variants: keep toolchain, features and `RUSTFLAGS` stable per project, or the dir bloats anyway.
- Third-party caches (sccache and friends) need their own retention policy; disk frees only when the last link is deleted (`unlink(2)` semantics).
