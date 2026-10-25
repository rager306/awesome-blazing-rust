//! this catalog's own reproducible run: roaring-rs vs std HashSet/BTreeSet
//!
//! Workload: two ID sets (uniform u32), B = 50% of A + 50% fresh keys.
//! Metrics: build time, 1M random contains-probes (~50% hit), full
//! intersection and union. Median of 7 runs for the hot ops.
//! Deterministic xorshift64* generator - no external RNG, reproducible.

use std::collections::{BTreeSet, HashSet};
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

fn median(v: &mut [f64]) -> f64 {
    v.sort_by(|a, b| a.partial_cmp(b).unwrap());
    v[v.len() / 2]
}

fn ms(t: Instant) -> f64 {
    t.elapsed().as_secs_f64() * 1000.0
}

fn gen_a(n: u64, rng: &mut Rng, universe: u64) -> Vec<u32> {
    (0..n).map(|_| rng.below(universe) as u32).collect()
}

fn gen_b(a: &[u32], n: u64, rng: &mut Rng, universe: u64) -> Vec<u32> {
    let half = n / 2;
    let mut b: Vec<u32> = (0..half).map(|i| a[rng.below(a.len() as u64) as usize]).collect();
    b.extend((0..n - half).map(|_| rng.below(universe) as u32));
    b
}

fn run_size(n: u64, seed: u64, universe: u64, label: &str) {
    println!("== N = {}, {} (universe 2^{:.0}) ==", n, label, (universe as f64).log2());
    let mut rng = Rng(seed);
    let a = gen_a(n, &mut rng, universe);
    let b = gen_b(&a, n, &mut rng, universe);

    // --- build (single pass each) ---
    let t = Instant::now();
    let mut rb_a = roaring::RoaringBitmap::new();
    for &x in &a {
        rb_a.insert(x);
    }
    let rb_build = ms(t);
    let t = Instant::now();
    let mut hs_a: HashSet<u32> = HashSet::with_capacity(a.len());
    for &x in &a {
        hs_a.insert(x);
    }
    let hs_build = ms(t);
    let t = Instant::now();
    let mut bs_a: BTreeSet<u32> = BTreeSet::new();
    for &x in &a {
        bs_a.insert(x);
    }
    let bs_build = ms(t);

    // --- contains: 1M random probes ---
    let probes: Vec<u32> = (0..1_000_000).map(|_| rng.below(universe) as u32).collect();
    let mut rb_hits = [0f64; 7];
    let mut hs_hits = [0f64; 7];
    let mut bs_hits = [0f64; 7];
    for i in 0..7 {
        let t = Instant::now();
        let mut hit = 0u64;
        for &p in &probes {
            hit += rb_a.contains(p) as u64;
        }
        rb_hits[i] = ms(t);
        assert!(hit > 0);
        let t = Instant::now();
        let mut hit = 0u64;
        for &p in &probes {
            hit += hs_a.contains(&p) as u64;
        }
        hs_hits[i] = ms(t);
        assert!(hit > 0);
        let t = Instant::now();
        let mut hit = 0u64;
        for &p in &probes {
            hit += bs_a.contains(&p) as u64;
        }
        bs_hits[i] = ms(t);
        assert!(hit > 0);
    }

    // --- intersection + union ---
    let t = Instant::now();
    let mut rb_b = roaring::RoaringBitmap::new();
    for &x in &b {
        rb_b.insert(x);
    }
    let rb_b_build = ms(t);
    let t = Instant::now();
    let inter = &rb_a & &rb_b;
    let rb_inter = ms(t);
    let rb_inter_len = inter.len();
    let t = Instant::now();
    let uni = &rb_a | &rb_b;
    let rb_union = ms(t);
    let rb_union_len = uni.len();

    let t = Instant::now();
    let mut hs_b: HashSet<u32> = HashSet::with_capacity(b.len());
    for &x in &b {
        hs_b.insert(x);
    }
    let hs_b_build = ms(t);
    let t = Instant::now();
    let hs_inter: usize = hs_a.intersection(&hs_b).count();
    let hs_inter_t = ms(t);
    let t = Instant::now();
    let hs_union: usize = hs_a.union(&hs_b).count();
    let hs_union_t = ms(t);

    let t = Instant::now();
    let mut bs_b: BTreeSet<u32> = BTreeSet::new();
    for &x in &b {
        bs_b.insert(x);
    }
    let bs_b_build = ms(t);
    let t = Instant::now();
    let bs_inter: usize = bs_a.intersection(&bs_b).count();
    let bs_inter_t = ms(t);
    let t = Instant::now();
    let bs_union: usize = bs_a.union(&bs_b).count();
    let bs_union_t = ms(t);

    assert_eq!(rb_inter_len as usize, hs_inter);
    assert_eq!(hs_inter, bs_inter);
    assert_eq!(rb_union_len as usize, hs_union);

    println!("build       roaring {:>8.1} ms | hashset {:>8.1} ms | btreeset {:>8.1} ms", rb_build, hs_build, bs_build);
    println!("build_B     roaring {:>8.1} ms | hashset {:>8.1} ms | btreeset {:>8.1} ms", rb_b_build, hs_b_build, bs_b_build);
    println!(
        "contains 1M roaring {:>8.2} ms | hashset {:>8.2} ms | btreeset {:>8.2} ms",
        median(&mut rb_hits[..]),
        median(&mut hs_hits[..]),
        median(&mut bs_hits[..])
    );
    println!(
        "intersect   roaring {:>8.2} ms ({} keys) | hashset {:>8.2} ms | btreeset {:>8.2} ms",
        rb_inter, rb_inter_len, hs_inter_t, bs_inter_t
    );
    println!(
        "union       roaring {:>8.2} ms ({} keys) | hashset {:>8.2} ms | btreeset {:>8.2} ms",
        rb_union, rb_union_len, hs_union_t, bs_union_t
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
