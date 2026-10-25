//! Instrumented copy of main.rs for tail diagnosis.
//! The timed region is IDENTICAL to the original; all extra bookkeeping
//! happens after `t0.elapsed()` has been taken.

use std::collections::VecDeque;
use std::time::Instant;

const TOTAL_EVENTS: u32 = 2_000_000;
const SAMPLE_EVERY: u32 = 64;
const HISTORY_CAP: usize = 8_192;

fn process_event(payload: u64) -> u64 {
    payload.wrapping_mul(0x9E37_79B9).rotate_left(17) ^ (payload >> 7)
}

fn pct(v: &mut Vec<u32>, p: f64) -> u32 {
    v.sort_unstable();
    v[((p * (v.len() - 1) as f64) as usize)]
}

fn main() {
    let mut history: Vec<u64> = Vec::with_capacity(HISTORY_CAP);
    let mut queue: VecDeque<u64> = VecDeque::with_capacity(1024);
    let mut latencies: Vec<u32> = Vec::with_capacity(TOTAL_EVENTS as usize);
    let mut checksum: u64 = 0;

    // ---- instrumentation state (never inside the timed window) ----
    let mut big: Vec<(u32, u32)> = Vec::new(); // (ns, i) for ns > 1000
    let mut sampled_early: Vec<u32> = Vec::new(); // sampled-event lat while history not yet full
    let mut sampled_late: Vec<u32> = Vec::new(); // sampled-event lat once history at cap
    let mut cap_changes: Vec<(u32, usize)> = Vec::new();
    let mut max_queue_len: usize = 0;
    let mut prev_cap = HISTORY_CAP;
    let mut big_sampled = 0usize;
    let mut big_unsampled = 0usize;

    for i in 0..TOTAL_EVENTS {
        queue.push_back(i as u64);
        if queue.len() > max_queue_len {
            max_queue_len = queue.len();
        }
        let payload = queue.pop_front().expect("queue never empty");

        let t0 = Instant::now();
        let out = process_event(payload);

        // rolling history window for drift analysis (sampled) -- UNCHANGED
        if i % SAMPLE_EVERY == 0 {
            history.insert(0, out);
            if history.len() > HISTORY_CAP {
                history.truncate(HISTORY_CAP);
            }
        }

        checksum ^= out;
        let lat = t0.elapsed().as_nanos() as u32;
        latencies.push(lat);

        // ---- instrumentation, AFTER the timed window ----
        if i % SAMPLE_EVERY == 0 {
            let cap = history.capacity();
            if cap != prev_cap {
                cap_changes.push((i, cap));
                prev_cap = cap;
            }
            if history.len() >= HISTORY_CAP {
                sampled_late.push(lat);
            } else {
                sampled_early.push(lat);
            }
        }
        if lat > 1000 {
            if i % SAMPLE_EVERY == 0 {
                big_sampled += 1;
            } else {
                big_unsampled += 1;
            }
            big.push((lat, i));
        }
    }

    let drift: u64 = history.iter().fold(0, |acc, &x| acc.rotate_left(3) ^ x);
    latencies.sort_unstable();
    let q = |p: f64| latencies[((p * (TOTAL_EVENTS - 1) as f64) as usize) as usize];

    println!("checksum: {checksum:016x}");
    println!("drift-audit: {drift:016x}");
    println!("p50: {} ns", q(0.50));
    println!("p90: {} ns", q(0.90));
    println!("p99: {} ns", q(0.99));
    println!("p999: {} ns", q(0.999));

    println!("--- diagnosis ---");
    println!("max queue len: {max_queue_len}");
    println!("history capacity changes (i, new_cap): {cap_changes:?}");
    println!("events > 1000 ns: {} (sampled i%64==0: {big_sampled}, unsampled: {big_unsampled})", big.len());

    let (e_n, e_min, e_p50, e_mean, e_max) = {
        let n = sampled_early.len();
        let min = *sampled_early.iter().min().unwrap_or(&0);
        let max = *sampled_early.iter().max().unwrap_or(&0);
        let mean = if n == 0 { 0 } else { sampled_early.iter().map(|&x| x as u64).sum::<u64>() / n as u64 };
        (n, min, pct(&mut sampled_early, 0.5), mean, max)
    };
    println!("sampled-event latency while history FILLING (n={e_n}): min={e_min} p50={e_p50} mean={e_mean} max={e_max}");

    let (l_n, l_min, l_p50, l_mean, l_max) = {
        let n = sampled_late.len();
        let min = *sampled_late.iter().min().unwrap_or(&0);
        let max = *sampled_late.iter().max().unwrap_or(&0);
        let mean = if n == 0 { 0 } else { sampled_late.iter().map(|&x| x as u64).sum::<u64>() / n as u64 };
        (n, min, pct(&mut sampled_late, 0.5), mean, max)
    };
    println!("sampled-event latency while history FULL (n={l_n}): min={l_min} p50={l_p50} mean={l_mean} max={l_max}");

    // ramp: mean sampled latency in 4 quarters of the filling phase
    let n = sampled_early.len();
    for (k, chunk) in sampled_early.chunks(n.div_ceil(4)).enumerate() {
        let m = chunk.iter().map(|&x| x as u64).sum::<u64>() / chunk.len() as u64;
        println!("  fill quarter {}: mean sampled lat = {} ns (history len ~ {})", k + 1, m, (k as usize + 1) * HISTORY_CAP / 4);
    }

    big.sort_unstable_by(|a, b| b.0.cmp(&a.0));
    println!("top 50 slowest (ns, i, sampled?):");
    for &(v, i) in big.iter().take(50) {
        println!("  {:>7} ns @ event {} ({})", v, i, if i % SAMPLE_EVERY == 0 { "sampled" } else { "UNSAMPLED" });
    }
    if !big.is_empty() {
        let first_uns = big.iter().filter(|&&(_, i)| i % SAMPLE_EVERY != 0).map(|&(_, i)| i).min();
        let last_uns = big.iter().filter(|&&(_, i)| i % SAMPLE_EVERY != 0).map(|&(_, i)| i).max();
        println!("unsampled >1us events: first={first_uns:?} last={last_uns:?} count={big_unsampled}");
    }
}
