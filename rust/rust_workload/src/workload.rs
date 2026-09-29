use crate::config::WorkloadConfig;
use crate::extrapolator::{ImuSample, PoseExtrapolator};
use crate::filters::{
    adaptive_voxel_filter, voxel_filter, AdaptiveVoxelFilterOptions, MotionFilter,
    MotionFilterOptions,
};
use crate::grid::ActiveSubmaps;
use crate::scan_matching::{correlative_match, refine_pose, CorrelativeOptions, RefinementOptions};
use crate::sensor::{generate_imu_sample, generate_scan, Trajectory, World};
use nalgebra::{Isometry2, Point2};
use rand::rngs::StdRng;
use rand::SeedableRng;
use std::time::{Duration, Instant};

pub struct ReleaseRecord {
    pub release_index: usize,
    pub sensor_time: f64,
    pub period: f64,
    pub wall_exec_secs: f64,
    pub cpu_exec_secs: f64,
    pub stage_filter_secs: f64,
    pub stage_correlative_secs: f64,
    pub stage_refine_secs: f64,
    pub stage_insert_secs: f64,
    pub submap_inserted: bool,
    pub grid_grew: bool,
    pub deadline_secs: f64,
    pub deadline_met: bool,
}

pub struct Summary {
    pub num_releases: usize,
    pub mean_wall_secs: f64,
    pub p95_wall_secs: f64,
    pub max_wall_secs: f64,
    pub mean_cpu_secs: f64,
    pub p95_cpu_secs: f64,
    pub max_cpu_secs: f64,
    pub mean_period_secs: f64,
    pub deadline_miss_count: usize,
    pub grid_growth_events: usize,
    pub submap_insertions: usize,
    /// Grid-growth events across the *entire* run, warm-up included -- growth
    /// is a first-touch reallocation cost and tends to cluster right at the
    /// start (the robot begins outside the grid's initial bounds), which is
    /// exactly the kind of cold-start effect warm-up exclusion exists for.
    pub lifetime_grid_growth_events: usize,
}

#[cfg(unix)]
fn cpu_time_now() -> f64 {
    unsafe {
        let mut ts: libc::timespec = std::mem::zeroed();
        libc::clock_gettime(libc::CLOCK_THREAD_CPUTIME_ID, &mut ts);
        ts.tv_sec as f64 + ts.tv_nsec as f64 * 1e-9
    }
}

#[cfg(not(unix))]
fn cpu_time_now() -> f64 {
    0.0
}

/// Persistent state for the release-by-release pipeline -- everything that
/// used to be a loop-local inside `run` now lives here so it can survive
/// across repeated calls. One call to `step()` performs exactly one release:
/// `config.num_accumulated_range_data` synthetic scans are generated and
/// accumulated, then gravity-alignment/filtering, scan matching, and
/// (conditionally, per `MotionFilter`) submap insertion run and are timed --
/// mirroring one call to `AddAccumulatedRangeData`
/// ([local_trajectory_builder_2d.cc:104-277]).
///
/// This is the type meant to be driven from a real-time dispatcher's periodic
/// task loop: construct once (e.g. via `Default`, at task setup), then call
/// `step()` once per job release.
pub struct WorkloadState {
    config: WorkloadConfig,
    rng: StdRng,
    world: World,
    trajectory: Trajectory,
    extrapolator: PoseExtrapolator,
    motion_filter: MotionFilter,
    active_submaps: ActiveSubmaps,
    accumulated: Vec<Point2<f64>>,
    last_release_time: Option<f64>,
    release_index: usize,
    t: f64,
    scan_period: f64,
    imu_period: f64,
    num_imu_per_scan: usize,
}

impl WorkloadState {
    pub fn new(config: WorkloadConfig) -> Self {
        let rng = StdRng::seed_from_u64(config.seed);
        let world = World::square_room(5.0);
        let trajectory = Trajectory {
            circle_radius: 3.0,
            angular_speed: 0.05,
        };

        let mut extrapolator = PoseExtrapolator::new(config.pose_queue_duration);
        extrapolator.add_pose(0.0, Isometry2::identity());

        let motion_filter = MotionFilter::new(MotionFilterOptions {
            max_time_seconds: config.motion_filter_max_time_seconds,
            max_distance_meters: config.motion_filter_max_distance_meters,
            max_angle_radians: config.motion_filter_max_angle_radians,
        });

        let active_submaps = ActiveSubmaps::new(config.submap_num_range_data, config.grid_resolution);

        let scan_period = 1.0 / config.scan_rate_hz;
        let imu_period = 1.0 / config.imu_rate_hz;
        let num_imu_per_scan = ((scan_period / imu_period).round() as usize).max(1);

        WorkloadState {
            config,
            rng,
            world,
            trajectory,
            extrapolator,
            motion_filter,
            active_submaps,
            accumulated: Vec::new(),
            last_release_time: None,
            release_index: 0,
            t: 0.0,
            scan_period,
            imu_period,
            num_imu_per_scan,
        }
    }

    /// Runs one release and returns its `ReleaseRecord`.
    pub fn step(&mut self) -> ReleaseRecord {
        // Accumulate `num_accumulated_range_data` scans. `last_scan_t` captures
        // the timestamp of the last scan in this batch (before it's advanced
        // for the *next* scan), matching the original's convention of using
        // the last accumulated point's time for the release.
        let mut last_scan_t = self.t;
        for _ in 0..self.config.num_accumulated_range_data {
            for k in 0..self.num_imu_per_scan {
                let imu_t = self.t + (k as f64) * self.imu_period;
                let true_omega = self.trajectory.angular_velocity_z();
                let noisy_omega = generate_imu_sample(true_omega, &mut self.rng, 0.01);
                self.extrapolator.add_imu(ImuSample {
                    time: imu_t,
                    angular_velocity_z: noisy_omega,
                });
            }

            let true_pose = self.trajectory.pose_at(self.t);
            let scan_points_local = generate_scan(
                &self.world,
                true_pose,
                self.config.points_per_scan,
                self.config.min_range,
                self.config.max_range,
                &mut self.rng,
                0.01,
            );

            // Simplification: all points in a scan share timestamp `t` rather
            // than each point's own within-sweep timestamp (see plan's
            // assumptions).
            let predicted_pose_now = self.extrapolator.extrapolate_pose(self.t);
            for p in &scan_points_local {
                self.accumulated.push(predicted_pose_now * p);
            }

            last_scan_t = self.t;
            self.t += self.scan_period;
        }
        let t = last_scan_t;

        let wall_start = Instant::now();
        let cpu_start = cpu_time_now();

        let last_pose = self.extrapolator.last_pose();
        let filter_start = Instant::now();
        let local_points: Vec<Point2<f64>> = self
            .accumulated
            .iter()
            .map(|p| last_pose.inverse() * p)
            .collect();
        // Mirrors the original's two-stage filtering: a plain voxel filter
        // over the gravity-aligned accumulated points ([voxel_filter_size],
        // `TransformToGravityAlignedFrameAndFilter`), then the adaptive
        // voxel filter that produces the scan-matcher's input point cloud.
        let coarsely_filtered = voxel_filter(&local_points, self.config.voxel_filter_size, &mut self.rng);
        let adaptive_options = AdaptiveVoxelFilterOptions {
            max_length: self.config.adaptive_voxel_max_length,
            min_num_points: self.config.adaptive_voxel_min_num_points,
            max_range: self.config.adaptive_voxel_max_range,
        };
        let filtered_local =
            adaptive_voxel_filter(&coarsely_filtered, &adaptive_options, &mut self.rng);
        let stage_filter = filter_start.elapsed();

        let pose_prediction = self.extrapolator.extrapolate_pose(t);
        let mut matching_pose = pose_prediction;
        let mut stage_corr = Duration::ZERO;
        if self.config.enable_correlative_matching {
            let corr_start = Instant::now();
            let corr_options = CorrelativeOptions {
                linear_search_window: self.config.correlative_linear_search_window,
                angular_search_window: self.config.correlative_angular_search_window,
                translation_delta_cost_weight: self.config.correlative_translation_delta_cost_weight,
                rotation_delta_cost_weight: self.config.correlative_rotation_delta_cost_weight,
                resolution: self.config.grid_resolution,
            };
            let (p, _score) = correlative_match(
                pose_prediction,
                &filtered_local,
                self.active_submaps.matching_grid(),
                &corr_options,
            );
            matching_pose = p;
            stage_corr = corr_start.elapsed();
        }

        let refine_start = Instant::now();
        let refine_options = RefinementOptions {
            occupied_space_weight: self.config.ceres_occupied_space_weight,
            translation_weight: self.config.ceres_translation_weight,
            rotation_weight: self.config.ceres_rotation_weight,
            max_num_iterations: self.config.ceres_max_num_iterations,
        };
        let refined_pose = refine_pose(
            matching_pose,
            pose_prediction,
            &filtered_local,
            self.active_submaps.matching_grid(),
            &refine_options,
        );
        let stage_refine = refine_start.elapsed();

        self.extrapolator.add_pose(t, refined_pose);

        let mut inserted = false;
        let mut grew = false;
        let mut stage_insert = Duration::ZERO;
        if !self.motion_filter.is_similar(t, refined_pose) {
            let insert_start = Instant::now();
            let hits_map: Vec<Point2<f64>> = filtered_local.iter().map(|p| refined_pose * p).collect();
            let origin_map = Point2::from(refined_pose.translation.vector);
            grew = self.active_submaps.insert(origin_map, &hits_map);
            inserted = true;
            stage_insert = insert_start.elapsed();
        }

        let wall_elapsed = wall_start.elapsed().as_secs_f64();
        let cpu_elapsed = cpu_time_now() - cpu_start;

        let period = self
            .last_release_time
            .map(|lt| t - lt)
            .unwrap_or(self.scan_period * self.config.num_accumulated_range_data as f64);
        self.last_release_time = Some(t);
        let deadline = period * self.config.deadline_scale;

        let record = ReleaseRecord {
            release_index: self.release_index,
            sensor_time: t,
            period,
            wall_exec_secs: wall_elapsed,
            cpu_exec_secs: cpu_elapsed,
            stage_filter_secs: stage_filter.as_secs_f64(),
            stage_correlative_secs: stage_corr.as_secs_f64(),
            stage_refine_secs: stage_refine.as_secs_f64(),
            stage_insert_secs: stage_insert.as_secs_f64(),
            submap_inserted: inserted,
            grid_grew: grew,
            deadline_secs: deadline,
            deadline_met: wall_elapsed <= deadline,
        };
        self.release_index += 1;
        self.accumulated.clear();

        if self.config.real_time {
            std::thread::sleep(Duration::from_secs_f64(period.max(0.0)));
        }

        record
    }
}

impl Default for WorkloadState {
    fn default() -> Self {
        WorkloadState::new(WorkloadConfig::default())
    }
}

/// Runs `config.duration_secs` of simulated sensor time via repeated `step()`
/// calls -- matches the original single-shot benchmark's behavior exactly.
pub fn run(config: WorkloadConfig) -> (Vec<ReleaseRecord>, Summary) {
    let scan_period = 1.0 / config.scan_rate_hz;
    let num_scans = (config.duration_secs / scan_period).floor() as usize;
    let num_releases = num_scans / config.num_accumulated_range_data;
    let warmup_releases = config.warmup_releases;

    let mut state = WorkloadState::new(config);
    let mut records = Vec::with_capacity(num_releases);
    for _ in 0..num_releases {
        records.push(state.step());
    }

    let summary = compute_summary(&records, warmup_releases);
    (records, summary)
}

fn percentile(sorted: &[f64], p: f64) -> f64 {
    if sorted.is_empty() {
        return 0.0;
    }
    let idx = ((sorted.len() as f64 - 1.0) * p).round() as usize;
    sorted[idx.min(sorted.len() - 1)]
}

fn compute_summary(records: &[ReleaseRecord], warmup: usize) -> Summary {
    let data: Vec<&ReleaseRecord> = records.iter().skip(warmup).collect();
    let mut wall: Vec<f64> = data.iter().map(|r| r.wall_exec_secs).collect();
    let mut cpu: Vec<f64> = data.iter().map(|r| r.cpu_exec_secs).collect();
    wall.sort_by(|a, b| a.partial_cmp(b).unwrap());
    cpu.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let n = (data.len().max(1)) as f64;
    Summary {
        num_releases: data.len(),
        mean_wall_secs: data.iter().map(|r| r.wall_exec_secs).sum::<f64>() / n,
        p95_wall_secs: percentile(&wall, 0.95),
        max_wall_secs: wall.last().copied().unwrap_or(0.0),
        mean_cpu_secs: data.iter().map(|r| r.cpu_exec_secs).sum::<f64>() / n,
        p95_cpu_secs: percentile(&cpu, 0.95),
        max_cpu_secs: cpu.last().copied().unwrap_or(0.0),
        mean_period_secs: data.iter().map(|r| r.period).sum::<f64>() / n,
        deadline_miss_count: data.iter().filter(|r| !r.deadline_met).count(),
        grid_growth_events: data.iter().filter(|r| r.grid_grew).count(),
        submap_insertions: data.iter().filter(|r| r.submap_inserted).count(),
        lifetime_grid_growth_events: records.iter().filter(|r| r.grid_grew).count(),
    }
}

pub fn write_csv(path: &str, records: &[ReleaseRecord]) -> std::io::Result<()> {
    use std::io::Write;
    let mut f = std::fs::File::create(path)?;
    writeln!(
        f,
        "release_index,sensor_time,period,wall_exec_secs,cpu_exec_secs,stage_filter_secs,stage_correlative_secs,stage_refine_secs,stage_insert_secs,submap_inserted,grid_grew,deadline_secs,deadline_met"
    )?;
    for r in records {
        writeln!(
            f,
            "{},{:.6},{:.6},{:.6},{:.6},{:.6},{:.6},{:.6},{:.6},{},{},{:.6},{}",
            r.release_index,
            r.sensor_time,
            r.period,
            r.wall_exec_secs,
            r.cpu_exec_secs,
            r.stage_filter_secs,
            r.stage_correlative_secs,
            r.stage_refine_secs,
            r.stage_insert_secs,
            r.submap_inserted,
            r.grid_grew,
            r.deadline_secs,
            r.deadline_met
        )?;
    }
    Ok(())
}
