use std::hint::black_box;
use std::time::Instant;
fn main() {
    const N: usize = 1_000_000;
    let total = Instant::now();
    let mut sum = 0_u128;
    for _ in 0..N {
        let start = Instant::now();
        black_box(1_u64);
        sum += start.elapsed().as_nanos();
    }
    let elapsed = total.elapsed().as_nanos();
    println!("n={N} sum_elapsed_ns={sum} wall_ns={elapsed} avg_measured_ns={:.2} avg_wall_ns={:.2}", sum as f64 / N as f64, elapsed as f64 / N as f64);
}
