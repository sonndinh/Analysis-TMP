use clap::Parser;
use rt_slam_workload::config::{Cli, WorkloadConfig};
use rt_slam_workload::workload;

fn main() {
    let cli = Cli::parse();
    let config = WorkloadConfig::from(&cli);
    let (records, summary) = workload::run(config);

    if let Some(path) = &cli.output {
        match workload::write_csv(path, &records) {
            Ok(()) => println!("wrote {} release records to {}", records.len(), path),
            Err(e) => eprintln!("failed to write CSV to {}: {}", path, e),
        }
    }

    println!(
        "--- summary ({} releases, {} warm-up excluded) ---",
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
