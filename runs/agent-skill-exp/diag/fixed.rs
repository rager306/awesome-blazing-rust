//! Fixed: rolling history window uses a VecDeque ring buffer instead of
//! Vec::insert(0, ...) (O(n) memmove every SAMPLE_EVERY events).
//! Semantics identical: newest element at front, capped at HISTORY_CAP,
//! iteration order front->back unchanged (drift-audit hash must be identical).

use std::collections::VecDeque;
use std::time::Instant;

const TOTAL_EVENTS: u32 = 2_000_000;
const SAMPLE_EVERY: u32 = 64;
const HISTORY_CAP: usize = 8_192;

fn process_event(payload: u64) -> u64 {
    // real work: a small mix of arithmetic so the optimizer cannot fold it away
    payload.wrapping_mul(0x9E37_79B9).rotate_left(17) ^ (payload >> 7)
}

fn main() {
    let mut history: VecDeque<u64> = VecDeque::with_capacity(HISTORY_CAP);
    let mut queue: VecDeque<u64> = VecDeque::with_capacity(1024);
    let mut latencies: Vec<u32> = Vec::with_capacity(TOTAL_EVENTS as usize);
    let mut checksum: u64 = 0;

    for i in 0..TOTAL_EVENTS {
        queue.push_back(i as u64);
        let payload = queue.pop_front().expect("queue never empty");

        let t0 = Instant::now();
        let out = process_event(payload);

        // rolling history window for drift analysis (sampled)
        if i % SAMPLE_EVERY == 0 {
            history.push_front(out);
            if history.len() > HISTORY_CAP {
                history.pop_back();
            }
        }

        checksum ^= out;
        latencies.push(t0.elapsed().as_nanos() as u32);
    }

    // drift audit: fold the whole window so telemetry covers history state too
    let drift: u64 = history.iter().fold(0, |acc, &x| acc.rotate_left(3) ^ x);

    latencies.sort_unstable();
    let q = |p: f64| latencies[((p * (TOTAL_EVENTS - 1) as f64) as usize) as usize];
    println!("checksum: {checksum:016x}");
    println!("drift-audit: {drift:016x}");
    println!("p50: {} ns", q(0.50));
    println!("p90: {} ns", q(0.90));
    println!("p99: {} ns", q(0.99));
    println!("p999: {} ns", q(0.999));

    let mut ranked: Vec<(u32, u32)> = latencies.iter().copied().enumerate().map(|(i, v)| (v, i as u32)).collect();
    ranked.sort_unstable_by(|a, b| b.0.cmp(&a.0));
    println!("slowest events (ns, event-index):");
    for (v, i) in ranked.iter().take(45) {
        println!("  {v} ns @ event {i}");
    }
}
