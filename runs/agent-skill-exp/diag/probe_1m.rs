//! End-cluster discriminator: fixed mechanism, TOTAL_EVENTS=1_000_000.
//! Marks every event > 5000 ns with (index, wall-offset from loop start).

use std::collections::VecDeque;
use std::time::Instant;

const TOTAL_EVENTS: u32 = 1_000_000;
const SAMPLE_EVERY: u32 = 64;
const HISTORY_CAP: usize = 8_192;

fn process_event(payload: u64) -> u64 {
    payload.wrapping_mul(0x9E37_79B9).rotate_left(17) ^ (payload >> 7)
}

fn main() {
    let mut history: VecDeque<u64> = VecDeque::with_capacity(HISTORY_CAP);
    let mut queue: VecDeque<u64> = VecDeque::with_capacity(1024);
    let mut latencies: Vec<u32> = Vec::with_capacity(TOTAL_EVENTS as usize);
    let mut checksum: u64 = 0;
    let loop_start = Instant::now();
    let mut slow: Vec<(u32, u64)> = Vec::new();

    for i in 0..TOTAL_EVENTS {
        queue.push_back(i as u64);
        let payload = queue.pop_front().expect("queue never empty");
        let t0 = Instant::now();
        let out = process_event(payload);
        if i % SAMPLE_EVERY == 0 {
            history.push_front(out);
            if history.len() > HISTORY_CAP {
                history.pop_back();
            }
        }
        checksum ^= out;
        let lat = t0.elapsed().as_nanos() as u32;
        latencies.push(lat);
        if lat > 5000 {
            slow.push((i, loop_start.elapsed().as_nanos() as u64));
        }
    }
    let total = loop_start.elapsed();

    latencies.sort_unstable();
    let q = |p: f64| latencies[((p * (TOTAL_EVENTS - 1) as f64) as usize) as usize];
    println!("checksum: {checksum:016x}  p50: {}  p99: {}  loop: {total:?}", q(0.50), q(0.99));
    println!("events >5us: {}", slow.len());
    let tail: Vec<(u32, u64)> = slow.iter().rev().take(8).rev().copied().collect();
    print!("last marks (event_i @ us):");
    for m in tail {
        print!("  {} @{}", m.0, m.1 / 1000);
    }
    println!();
    let near_end = slow.iter().filter(|&&(i, _)| i >= TOTAL_EVENTS - 45).count();
    println!("marks within last 45 events: {near_end}");
}
