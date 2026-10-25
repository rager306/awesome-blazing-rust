//! Fixed version + end-cluster probe: for every event > 5000 ns, print
//! event index and wall-clock offset from loop start. Plus loop total time.

use std::collections::VecDeque;
use std::time::Instant;

const TOTAL_EVENTS: u32 = 2_000_000;
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
    let mut slow_marks: Vec<(u32, u64)> = Vec::new(); // (i, ns since loop start)

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
            slow_marks.push((i, loop_start.elapsed().as_nanos() as u64));
        }
    }
    let total_loop = loop_start.elapsed();

    let drift: u64 = history.iter().fold(0, |acc, &x| acc.rotate_left(3) ^ x);
    latencies.sort_unstable();
    let q = |p: f64| latencies[((p * (TOTAL_EVENTS - 1) as f64) as usize) as usize];
    println!("checksum: {checksum:016x}");
    println!("drift-audit: {drift:016x}");
    println!("p50: {} ns", q(0.50));
    println!("p99: {} ns", q(0.99));
    println!("p999: {} ns", q(0.999));
    println!("loop total: {total_loop:?}");
    println!("events >5us: {} first few marks (event_i, us_since_loop_start):", slow_marks.len());
    for m in slow_marks.iter().take(6) {
        println!("  event {} @ {} us", m.0, m.1 / 1000);
    }
    let last6: Vec<(u32, u64)> = slow_marks.iter().rev().take(6).rev().copied().collect();
    println!("last marks:");
    for m in last6 {
        println!("  event {} @ {} us", m.0, m.1 / 1000);
    }
}
