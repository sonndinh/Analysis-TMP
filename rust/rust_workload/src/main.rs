use clap::Parser;
use rt_slam_workload::config::{Cli, WorkloadConfig};
use rt_slam_workload::workload;

fn derive_scan_csv_path(path: &str) -> String {
    match path.strip_suffix(".csv") {
        Some(stripped) => format!("{}_scans.csv", stripped),
        None => format!("{}_scans.csv", path),
    }
}

fn main() {
    let cli = Cli::parse();
    let config = WorkloadConfig::from(&cli);
    let (scan_records, release_records, scan_summary, summary) = workload::run(config);

    if let Some(path) = &cli.output {
        match workload::write_release_csv(path, &release_records) {
            Ok(()) => println!("wrote {} release records to {}", release_records.len(), path),
            Err(e) => eprintln!("failed to write release CSV to {}: {}", path, e),
        }
        let scan_path = derive_scan_csv_path(path);
        match workload::write_scan_csv(&scan_path, &scan_records) {
            Ok(()) => println!("wrote {} scan records to {}", scan_records.len(), scan_path),
            Err(e) => eprintln!("failed to write scan CSV to {}: {}", scan_path, e),
        }
    }

    println!(
        "--- task 1: sync & accumulate ({} scans, {} produced a batch) ---",
        scan_summary.num_scans, scan_summary.batches_produced
    );
    println!(
        "wall exec time  mean/p95/max (ms):  {:.3} / {:.3} / {:.3}",
        scan_summary.mean_wall_secs * 1e3,
        scan_summary.p95_wall_secs * 1e3,
        scan_summary.max_wall_secs * 1e3
    );
    println!(
        "cpu  exec time  mean/p95/max (ms):  {:.3} / {:.3} / {:.3}",
        scan_summary.mean_cpu_secs * 1e3,
        scan_summary.p95_cpu_secs * 1e3,
        scan_summary.max_cpu_secs * 1e3
    );

    println!(
        "--- tasks 2-4: filter / scan match / motion filter + grid update ({} releases, {} warm-up excluded) ---",
        summary.num_releases, cli.warmup_releases
    );
    println!("mean period (s):                    {:.4}", summary.mean_period_secs);
    println!(
        "wall exec time  mean/p95/max (ms):  {:.3} / {:.3} / {:.3}",
        summary.mean_wall_secs * 1e3,
        summary.p95_wall_secs * 1e3,
        summary.max_wall_secs * 1e3
    );
    println!(
        "cpu  exec time  mean/p95/max (ms):  {:.3} / {:.3} / {:.3}",
        summary.mean_cpu_secs * 1e3,
        summary.p95_cpu_secs * 1e3,
        summary.max_cpu_secs * 1e3
    );
    println!("submap insertions:                  {}", summary.submap_insertions);
    println!(
        "grid growth events (post-warmup / lifetime): {} / {}",
        summary.grid_growth_events, summary.lifetime_grid_growth_events
    );
    println!("deadline misses:                    {}", summary.deadline_miss_count);
}
