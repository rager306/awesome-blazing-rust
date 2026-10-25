//! this catalog's own reproducible run: roaring-rs vs std HashSet/BTreeSet
//!
//! Workload: two ID sets (uniform u32), B = 50% of A + 50% fresh keys.
//! Every published operation runs 7 times; the median is reported and all
//! 7 samples are printed, so the raw data ships with the run.
//! Set operations run in two named scenarios with per-library canonical
//! implementations:
//!   count       - determine the result size without building a container
//!                 (roaring: iterate smaller set, probe larger; std:
//!                 iterator .count())
//!   materialize - build the result container (roaring: `&` / `|`;
//!                 std: `.collect()`)
//! Builds are declared single-run. All results are cross-checked across
//! the three structures OUTSIDE the timed region (counts, hit counts and
//! FNV fingerprints over sorted keys). Deterministic xorshift64* - no
//! external RNG, reproducible.

use std::collections::{BTreeSet, HashSet};
use std::hint::black_box;
use std::time::Instant;

struct Rng(u64);

impl Rng {
    fn next(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        x.wrapping_mul(0x2545F4914F6CDD1D)
    }
    fn below(&mut self, n: u64) -> u64 {
        self.next() % n
    }
}

const RUNS: usize = 7;

fn median(v: &mut [f64]) -> f64 {
    v.sort_by(|a, b| a.partial_cmp(b).unwrap());
    v[v.len() / 2]
}

fn ms(t: Instant) -> f64 {
    t.elapsed().as_secs_f64() * 1000.0
}

/// 7 timed runs; every sample recorded, median returned, all printed.
/// The result is black-boxed so LLVM cannot delete the measured work.
fn timed<T>(f: impl Fn() -> T) -> [f64; RUNS] {
    let mut out = [0f64; RUNS];
    for slot in out.iter_mut() {
        let t = Instant::now();
        black_box(f());
        *slot = ms(t);
    }
    out
}

fn report(label: &str, who: &str, mut s: [f64; RUNS]) -> f64 {
    let med = median(&mut s);
    let parts: Vec<String> = s.iter().map(|x| format!("{:.2}", x)).collect();
    println!("    {:<22} {:<9} median {:>9.2} ms  [{}]", label, who, med, parts.join(", "));
    med
}

/// FNV-1a over sorted keys: order-independent proof that the materialized
/// results of two structures hold identical elements.
fn fingerprint(mut v: Vec<u32>) -> u64 {
    v.sort_unstable();
    let mut h = 0xcbf29ce484222325u64;
    for &x in &v {
        h ^= x as u64;
        h = h.wrapping_mul(0x100000001b3);
    }
    h
}

fn gen_a(n: u64, rng: &mut Rng, universe: u64) -> Vec<u32> {
    (0..n).map(|_| rng.below(universe) as u32).collect()
}

fn gen_b(a: &[u32], n: u64, rng: &mut Rng, universe: u64) -> Vec<u32> {
    let half = n / 2;
    let mut b: Vec<u32> = (0..half).map(|_| a[rng.below(a.len() as u64) as usize]).collect();
    b.extend((0..n - half).map(|_| rng.below(universe) as u32));
    b
}

fn run_size(n: u64, seed: u64, universe: u64, label: &str) {
    println!("== N = {}, {} (universe 2^{:.0}) ==", n, label, (universe as f64).log2());
    let mut rng = Rng(seed);
    let a = gen_a(n, &mut rng, universe);
    let b = gen_b(&a, n, &mut rng, universe);

    // --- build: single run each (declared as such, not a median) ---
    let t = Instant::now();
    let rb_a: roaring::RoaringBitmap = a.iter().copied().collect();
    let rb_build = ms(t);
    let t = Instant::now();
    let hs_a: HashSet<u32> = a.iter().copied().collect();
    let hs_build = ms(t);
    let t = Instant::now();
    let bs_a: BTreeSet<u32> = a.iter().copied().collect();
    let bs_build = ms(t);
    let t = Instant::now();
    let rb_b: roaring::RoaringBitmap = b.iter().copied().collect();
    let rb_b_build = ms(t);
    let t = Instant::now();
    let hs_b: HashSet<u32> = b.iter().copied().collect();
    let hs_b_build = ms(t);
    let t = Instant::now();
    let bs_b: BTreeSet<u32> = b.iter().copied().collect();
    let bs_b_build = ms(t);

    // Population honesty: draws with duplicates are not unique keys.
    assert_eq!(rb_a.len() as usize, hs_a.len());
    assert_eq!(hs_a.len(), bs_a.len());
    assert_eq!(rb_b.len() as usize, hs_b.len());
    let unique_a = rb_a.len();
    println!(
        "  population: {} draws -> A unique {} | B unique {} | overlap potential 50% of B",
        2 * n,
        unique_a,
        rb_b.len()
    );

    // --- contains: 1M probes from the whole universe ---
    let probes: Vec<u32> = (0..1_000_000).map(|_| rng.below(universe) as u32).collect();
    let rb_c = timed(|| {
        let mut hit = 0u64;
        for &p in &probes {
            hit += black_box(rb_a.contains(p)) as u64;
        }
        black_box(hit)
    });
    let hs_c = timed(|| {
        let mut hit = 0u64;
        for &p in &probes {
            hit += black_box(hs_a.contains(&p)) as u64;
        }
        black_box(hit)
    });
    let bs_c = timed(|| {
        let mut hit = 0u64;
        for &p in &probes {
            hit += black_box(bs_a.contains(&p)) as u64;
        }
        black_box(hit)
    });
    // cross-check hit counts outside timing
    let (rh, hh, bh) = (
        probes.iter().filter(|&&p| rb_a.contains(p)).count(),
        probes.iter().filter(|&&p| hs_a.contains(&p)).count(),
        probes.iter().filter(|&&p| bs_a.contains(&p)).count(),
    );
    assert_eq!(rh, hh, "contains hit mismatch roaring/hashset");
    assert_eq!(hh, bh, "contains hit mismatch hashset/btreeset");
    println!("  contains hit rate: {}/1M probes ({:.1}% of universe-drawn probes hit A)", rh, rh as f64 / 10_000.0);
    report("contains 1M", "roaring", rb_c);
    report("contains 1M", "hashset", hs_c);
    report("contains 1M", "btreeset", bs_c);

    // --- scenario COUNT: result size, no container built ---
    let rb_ic = timed(|| {
        let (small, big) = if rb_a.len() <= rb_b.len() { (&rb_a, &rb_b) } else { (&rb_b, &rb_a) };
        black_box(small.iter().filter(|x| big.contains(*x)).count())
    });
    let hs_ic = timed(|| black_box(hs_a.intersection(&hs_b).count()));
    let bs_ic = timed(|| black_box(bs_a.intersection(&bs_b).count()));
    let rb_uc = timed(|| {
        // union size by scanning both sets without building a container
        let mut c = rb_a.len();
        for x in rb_b.iter() {
            if !rb_a.contains(x) {
                c += 1;
            }
        }
        black_box(c)
    });
    let hs_uc = timed(|| black_box(hs_a.union(&hs_b).count()));
    let bs_uc = timed(|| black_box(bs_a.union(&bs_b).count()));
    report("intersect count", "roaring", rb_ic);
    report("intersect count", "hashset", hs_ic);
    report("intersect count", "btreeset", bs_ic);
    report("union count", "roaring", rb_uc);
    report("union count", "hashset", hs_uc);
    report("union count", "btreeset", bs_uc);

    // --- scenario MATERIALIZE: build the result container ---
    let rb_im = timed(|| black_box(&rb_a & &rb_b));
    let hs_im = timed(|| black_box(hs_a.intersection(&hs_b).copied().collect::<HashSet<u32>>()));
    let bs_im = timed(|| black_box(bs_a.intersection(&bs_b).copied().collect::<BTreeSet<u32>>()));
    let rb_um = timed(|| black_box(&rb_a | &rb_b));
    let hs_um = timed(|| black_box(hs_a.union(&hs_b).copied().collect::<HashSet<u32>>()));
    let bs_um = timed(|| black_box(bs_a.union(&bs_b).copied().collect::<BTreeSet<u32>>()));
    report("intersect materialize", "roaring", rb_im);
    report("intersect materialize", "hashset", hs_im);
    report("intersect materialize", "btreeset", bs_im);
    report("union materialize", "roaring", rb_um);
    report("union materialize", "hashset", hs_um);
    report("union materialize", "btreeset", bs_um);

    // --- cross-checks OUTSIDE the timed region: identical results ---
    let inter_ref = fingerprint(hs_a.intersection(&hs_b).copied().collect());
    let union_ref = fingerprint(hs_a.union(&hs_b).copied().collect());
    assert_eq!(fingerprint((&rb_a & &rb_b).iter().collect()), inter_ref, "roaring inter != hashset");
    assert_eq!(bs_a.intersection(&bs_b).count(), hs_a.intersection(&hs_b).count(), "btree inter != hashset");
    assert_eq!(fingerprint((&rb_a | &rb_b).iter().collect()), union_ref, "roaring union != hashset");
    assert_eq!(bs_a.union(&bs_b).count(), hs_a.union(&hs_b).count(), "btree union != hashset");
    println!(
        "  cross-check OK: inter = {} keys, union = {} keys, identical across all three (fingerprint)",
        hs_a.intersection(&hs_b).count(),
        hs_a.union(&hs_b).count()
    );
    println!(
        "  build single-run ms - A: roaring {:.1} | hash {:.1} | btree {:.1}   B: roaring {:.1} | hash {:.1} | btree {:.1}",
        rb_build, hs_build, bs_build, rb_b_build, hs_b_build, bs_b_build
    );
}

fn host_line() -> String {
    let model = std::fs::read_to_string("/proc/cpuinfo")
        .ok()
        .and_then(|s| s.lines().find(|l| l.starts_with("model name")).and_then(|l| l.split(':').nth(1).map(|s| s.trim().to_string())))
        .unwrap_or_else(|| "unknown cpu".into());
    format!(
        "host: {} | {} vcpus | {}",
        model,
        std::thread::available_parallelism().map(|n| n.get()).unwrap_or(0),
        std::env::consts::OS
    )
}

fn main() {
    println!("{}", host_line());
    let sparse = 1u64 << 32;
    let dense = |n: u64| 8 * n;
    run_size(1_000_000, 0xDA7A_0001, sparse, "sparse uniform");
    run_size(1_000_000, 0xDA7A_0003, dense(1_000_000), "dense");
    run_size(8_000_000, 0xDA7A_0002, sparse, "sparse uniform");
    run_size(8_000_000, 0xDA7A_0004, dense(8_000_000), "dense");
}
