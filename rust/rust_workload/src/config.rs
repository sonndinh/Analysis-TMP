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
