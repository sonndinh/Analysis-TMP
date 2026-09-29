use clap::Parser;

/// Real-time workload benchmark ported from Cartographer's LocalTrajectoryBuilder2D.
///
/// Absolute timing numbers are host-dependent (CPU governor, turbo boost, other
/// load) -- use this for relative/comparative characterization and as input to
/// separate schedulability analysis, not as a certified WCET bound. "Deadline
/// miss" means this task's own execution time exceeded its own period-derived
/// deadline in isolation, not a multi-task scheduler outcome.
#[derive(Parser, Debug, Clone)]
#[command(name = "rt_slam_workload")]
pub struct Cli {
    /// Simulated sensor-time duration to run, in seconds.
    #[arg(long, default_value_t = 30.0)]
    pub duration_secs: f64,

    /// Synthetic LIDAR scan rate, in Hz.
    #[arg(long, default_value_t = 10.0)]
    pub scan_rate_hz: f64,

    /// Points per synthetic LIDAR scan.
    #[arg(long, default_value_t = 720)]
    pub points_per_scan: usize,

    /// Synthetic IMU sample rate, in Hz.
    #[arg(long, default_value_t = 100.0)]
    pub imu_rate_hz: f64,

    /// Number of scans accumulated before a release (matches
    /// `num_accumulated_range_data` in trajectory_builder_2d.lua, default 1).
    #[arg(long, default_value_t = 1)]
    pub num_accumulated_range_data: usize,

    /// Multiplier applied to the period to derive the deadline (deadline = period * scale).
    #[arg(long, default_value_t = 1.0)]
    pub deadline_scale: f64,

    /// Number of leading releases excluded from summary statistics.
    #[arg(long, default_value_t = 10)]
    pub warmup_releases: usize,

    /// Enable the real-time correlative scan matcher stage before refinement.
    #[arg(long, default_value_t = true)]
    pub enable_correlative_matching: bool,

    /// If set, sleep between releases to pace execution to the simulated period
    /// instead of running flat-out.
    #[arg(long, default_value_t = false)]
    pub real_time: bool,

    /// RNG seed, for reproducible runs.
    #[arg(long, default_value_t = 42)]
    pub seed: u64,

    /// Optional path to write a per-release CSV trace.
    #[arg(long)]
    pub output: Option<String>,

    // --- Parameters mirroring configuration_files/trajectory_builder_2d.lua ---
    #[arg(long, default_value_t = 0.025)]
    pub voxel_filter_size: f64,
    #[arg(long, default_value_t = 0.0)]
    pub min_range: f64,
    #[arg(long, default_value_t = 30.0)]
    pub max_range: f64,

    #[arg(long, default_value_t = 5.0)]
    pub motion_filter_max_time_seconds: f64,
    #[arg(long, default_value_t = 0.2)]
    pub motion_filter_max_distance_meters: f64,
    #[arg(long, default_value_t = 1.0_f64.to_radians())]
    pub motion_filter_max_angle_radians: f64,

    #[arg(long, default_value_t = 0.1)]
    pub correlative_linear_search_window: f64,
    #[arg(long, default_value_t = 20.0_f64.to_radians())]
    pub correlative_angular_search_window: f64,
    #[arg(long, default_value_t = 0.1)]
    pub correlative_translation_delta_cost_weight: f64,
    #[arg(long, default_value_t = 0.1)]
    pub correlative_rotation_delta_cost_weight: f64,

    #[arg(long, default_value_t = 1.0)]
    pub ceres_occupied_space_weight: f64,
    #[arg(long, default_value_t = 10.0)]
    pub ceres_translation_weight: f64,
    #[arg(long, default_value_t = 40.0)]
    pub ceres_rotation_weight: f64,
    #[arg(long, default_value_t = 20)]
    pub ceres_max_num_iterations: usize,

    #[arg(long, default_value_t = 0.5)]
    pub adaptive_voxel_max_length: f64,
    #[arg(long, default_value_t = 200)]
    pub adaptive_voxel_min_num_points: usize,
    #[arg(long, default_value_t = 50.0)]
    pub adaptive_voxel_max_range: f64,

    /// Number of insertions before a new submap is started (matches submaps.num_range_data).
    #[arg(long, default_value_t = 90)]
    pub submap_num_range_data: usize,
    #[arg(long, default_value_t = 0.05)]
    pub grid_resolution: f64,

    /// Pose queue duration for the constant-velocity extrapolator, in seconds.
    #[arg(long, default_value_t = 0.001)]
    pub pose_queue_duration: f64,
}

/// Plain runtime configuration for the workload pipeline, decoupled from CLI
/// parsing. This -- not `Cli` -- is what `WorkloadState`/`run` consume, so a
/// library caller (e.g. a real-time dispatcher) doesn't need to depend on
/// `clap` or build a `Cli` via argv parsing to drive the workload.
#[derive(Debug, Clone, PartialEq)]
pub struct WorkloadConfig {
    pub duration_secs: f64,
    pub scan_rate_hz: f64,
    pub points_per_scan: usize,
    pub imu_rate_hz: f64,
    pub num_accumulated_range_data: usize,
    pub deadline_scale: f64,
    pub warmup_releases: usize,
    pub enable_correlative_matching: bool,
    pub real_time: bool,
    pub seed: u64,
    pub voxel_filter_size: f64,
    pub min_range: f64,
    pub max_range: f64,
    pub motion_filter_max_time_seconds: f64,
    pub motion_filter_max_distance_meters: f64,
    pub motion_filter_max_angle_radians: f64,
    pub correlative_linear_search_window: f64,
    pub correlative_angular_search_window: f64,
    pub correlative_translation_delta_cost_weight: f64,
    pub correlative_rotation_delta_cost_weight: f64,
    pub ceres_occupied_space_weight: f64,
    pub ceres_translation_weight: f64,
    pub ceres_rotation_weight: f64,
    pub ceres_max_num_iterations: usize,
    pub adaptive_voxel_max_length: f64,
    pub adaptive_voxel_min_num_points: usize,
    pub adaptive_voxel_max_range: f64,
    pub submap_num_range_data: usize,
    pub grid_resolution: f64,
    pub pose_queue_duration: f64,
}

impl From<&Cli> for WorkloadConfig {
    fn from(cli: &Cli) -> Self {
        WorkloadConfig {
            duration_secs: cli.duration_secs,
            scan_rate_hz: cli.scan_rate_hz,
            points_per_scan: cli.points_per_scan,
            imu_rate_hz: cli.imu_rate_hz,
            num_accumulated_range_data: cli.num_accumulated_range_data,
            deadline_scale: cli.deadline_scale,
            warmup_releases: cli.warmup_releases,
            enable_correlative_matching: cli.enable_correlative_matching,
            real_time: cli.real_time,
            seed: cli.seed,
            voxel_filter_size: cli.voxel_filter_size,
            min_range: cli.min_range,
            max_range: cli.max_range,
            motion_filter_max_time_seconds: cli.motion_filter_max_time_seconds,
            motion_filter_max_distance_meters: cli.motion_filter_max_distance_meters,
            motion_filter_max_angle_radians: cli.motion_filter_max_angle_radians,
            correlative_linear_search_window: cli.correlative_linear_search_window,
            correlative_angular_search_window: cli.correlative_angular_search_window,
            correlative_translation_delta_cost_weight: cli.correlative_translation_delta_cost_weight,
            correlative_rotation_delta_cost_weight: cli.correlative_rotation_delta_cost_weight,
            ceres_occupied_space_weight: cli.ceres_occupied_space_weight,
            ceres_translation_weight: cli.ceres_translation_weight,
            ceres_rotation_weight: cli.ceres_rotation_weight,
            ceres_max_num_iterations: cli.ceres_max_num_iterations,
            adaptive_voxel_max_length: cli.adaptive_voxel_max_length,
            adaptive_voxel_min_num_points: cli.adaptive_voxel_min_num_points,
            adaptive_voxel_max_range: cli.adaptive_voxel_max_range,
            submap_num_range_data: cli.submap_num_range_data,
            grid_resolution: cli.grid_resolution,
            pose_queue_duration: cli.pose_queue_duration,
        }
    }
}

impl Default for WorkloadConfig {
    fn default() -> Self {
        // Keep in sync with `Cli`'s `#[arg(default_value_t = ...)]` values --
        // guarded by `workload_config_default_matches_cli_default` below.
        WorkloadConfig {
            duration_secs: 30.0,
            scan_rate_hz: 10.0,
            points_per_scan: 720,
            imu_rate_hz: 100.0,
            num_accumulated_range_data: 1,
            deadline_scale: 1.0,
            warmup_releases: 10,
            enable_correlative_matching: true,
            real_time: false,
            seed: 42,
            voxel_filter_size: 0.025,
            min_range: 0.0,
            max_range: 30.0,
            motion_filter_max_time_seconds: 5.0,
            motion_filter_max_distance_meters: 0.2,
            motion_filter_max_angle_radians: 1.0_f64.to_radians(),
            correlative_linear_search_window: 0.1,
            correlative_angular_search_window: 20.0_f64.to_radians(),
            correlative_translation_delta_cost_weight: 0.1,
            correlative_rotation_delta_cost_weight: 0.1,
            ceres_occupied_space_weight: 1.0,
            ceres_translation_weight: 10.0,
            ceres_rotation_weight: 40.0,
            ceres_max_num_iterations: 20,
            adaptive_voxel_max_length: 0.5,
            adaptive_voxel_min_num_points: 200,
            adaptive_voxel_max_range: 50.0,
            submap_num_range_data: 90,
            grid_resolution: 0.05,
            pose_queue_duration: 0.001,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn workload_config_default_matches_cli_default() {
        let cli = Cli::parse_from(["rt_slam_workload"]);
        assert_eq!(WorkloadConfig::from(&cli), WorkloadConfig::default());
    }
}
