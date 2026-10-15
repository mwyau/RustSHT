use std::hint::black_box;

use rustsht_fft::RealFftPlan;

fn main() {
    let mut args = std::env::args().skip(1);
    let n = args
        .next()
        .expect("transform length is required")
        .parse::<usize>()
        .expect("transform length must be an integer");
    let workers = args
        .next()
        .expect("worker count is required")
        .parse::<usize>()
        .expect("worker count must be an integer");
    assert!(args.next().is_none(), "unexpected extra arguments");
    assert!(workers > 0, "worker count must be positive");

    let plan = RealFftPlan::<f64>::new(n).expect("real FFT plan");
    let workspaces: Vec<_> = (0..workers)
        .map(|_| plan.workspace().expect("real FFT workspace"))
        .collect();
    let workspace_bytes = workspaces[0]
        .scratch_bytes()
        .expect("workspace size fits usize");
    assert!(
        workspaces
            .iter()
            .all(|workspace| { workspace.scratch_bytes() == Some(workspace_bytes) })
    );
    let total_workspace_bytes = workspace_bytes
        .checked_mul(workers)
        .expect("total workspace size fits usize");
    black_box(&workspaces);
    let peak_rss_kib =
        peak_rss_kib().map_or_else(|| String::from("unavailable"), |value| value.to_string());
    println!("n,workers,workspace_bytes_per_worker,workspace_bytes_total,peak_rss_kib");
    println!("{n},{workers},{workspace_bytes},{total_workspace_bytes},{peak_rss_kib}");
}

#[cfg(target_os = "linux")]
fn peak_rss_kib() -> Option<usize> {
    let status = std::fs::read_to_string("/proc/self/status").ok()?;
    status.lines().find_map(|line| {
        let value = line.strip_prefix("VmHWM:")?;
        value.split_whitespace().next()?.parse().ok()
    })
}

#[cfg(not(target_os = "linux"))]
fn peak_rss_kib() -> Option<usize> {
    None
}
