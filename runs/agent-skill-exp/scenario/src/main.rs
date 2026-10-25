//! Event pipeline service - synthetic load generator with latency telemetry.
//!
//! Processes a stream of events through a worker, keeping a rolling history
//! window for drift analysis, and prints latency quantiles plus the slowest
//! individual events (indices captured before sorting, v2 telemetry).

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
    let mut history: Vec<u64> = Vec::with_capacity(HISTORY_CAP);
    let mut queue: VecDeque<u64> = VecDeque::with_capacity(1024);
    let mut samples: Vec<(u32, u32)> = Vec::with_capacity(TOTAL_EVENTS as usize); // (ns, event index)
    let mut checksum: u64 = 0;

    for i in 0..TOTAL_EVENTS {
        queue.push_back(i as u64);
        let payload = queue.pop_front().expect("queue never empty");

        let t0 = Instant::now();
        let out = process_event(payload);

        // rolling history window for drift analysis (sampled)
        if i % SAMPLE_EVERY == 0 {
            history.insert(0, out);
            if history.len() > HISTORY_CAP {
                history.truncate(HISTORY_CAP);
            }
        }

        checksum ^= out;
        samples.push((t0.elapsed().as_nanos() as u32, i));
    }

    // drift audit: fold the whole window so telemetry covers history state too
    let drift: u64 = history.iter().fold(0, |acc, &x| acc.rotate_left(3) ^ x);

    let mut times: Vec<u32> = samples.iter().map(|&(v, _)| v).collect();
    times.sort_unstable();
    let q = |p: f64| times[((p * (TOTAL_EVENTS - 1) as f64) as usize) as usize];
    println!("checksum: {checksum:016x}");
    println!("drift-audit: {drift:016x}");
    println!("p50: {} ns", q(0.50));
    println!("p90: {} ns", q(0.90));
    println!("p99: {} ns", q(0.99));
    println!("p999: {} ns", q(0.999));

    let mut ranked: Vec<(u32, u32)> = samples.clone();
    ranked.sort_unstable_by(|a, b| b.0.cmp(&a.0));
    println!("slowest events (ns, event-index):");
    for (v, i) in ranked.iter().take(45) {
        println!("  {v} ns @ event {i}");
    }
}
