use crate::config::WorkloadConfig;
use crate::extrapolator::{ImuSample, PoseExtrapolator};
use crate::filters::{
    adaptive_voxel_filter, voxel_filter, AdaptiveVoxelFilterOptions, MotionFilter,
    MotionFilterOptions,
};
use crate::grid::{ActiveSubmaps, ProbabilityGrid};
use crate::scan_matching::{correlative_match, refine_pose, CorrelativeOptions, RefinementOptions};
use crate::sensor::{generate_imu_sample, generate_scan, Trajectory, World};
use nalgebra::{Isometry2, Point2};
use rand::rngs::StdRng;
use rand::SeedableRng;
use std::time::{Duration, Instant};

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

// ---------------------------------------------------------------------------
// Inter-task handoff types -- the data contract between consecutive stages.
// ---------------------------------------------------------------------------

/// Output of `SyncAccumulateTask`: accumulated points in map frame, plus the
/// release's sensor timestamp (the last accumulated scan's time, matching
/// the original's convention of using the last point's time for a release).
pub struct AccumulatedBatch {
    pub points: Vec<Point2<f64>>,
    pub release_time: f64,
}

/// Output of `FilterTask`: the scan-matcher-ready point cloud, recentered
/// relative to the extrapolator's last pose.
pub struct FilteredScan {
    pub points: Vec<Point2<f64>>,
    pub release_time: f64,
}

/// Output of `ScanMatchingTask`: the refined pose, plus the same point cloud
/// it matched against (`MotionFilterGridUpdateTask` needs it again to
/// ray-cast into the grid).
pub struct MatchedPose {
    pub pose: Isometry2<f64>,
    pub points: Vec<Point2<f64>>,
    pub release_time: f64,
}

// ---------------------------------------------------------------------------
// Timing records -- two streams, since Task 1 has a different period than
// Tasks 2-4 (it fires once per raw scan; a release only happens every
// `num_accumulated_range_data` scans).
// ---------------------------------------------------------------------------

pub struct ScanRecord {
    pub scan_index: usize,
    pub sensor_time: f64,
    pub wall_exec_secs: f64,
    pub cpu_exec_secs: f64,
    pub produced_batch: bool,
}

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

pub struct ScanSummary {
    pub num_scans: usize,
    pub mean_wall_secs: f64,
    pub p95_wall_secs: f64,
    pub max_wall_secs: f64,
    pub mean_cpu_secs: f64,
    pub p95_cpu_secs: f64,
    pub max_cpu_secs: f64,
    pub batches_produced: usize,
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

// ---------------------------------------------------------------------------
// Task 1: Sync & Accumulate. Released once per raw scan -- a genuinely
// different (shorter) period than Tasks 2-4. Mirrors `AddRangeData`'s
// per-point extrapolation, range gating, and accumulation
// ([local_trajectory_builder_2d.cc:104-209]).
// ---------------------------------------------------------------------------

pub struct SyncAccumulateTask {
    rng: StdRng,
    world: World,
    trajectory: Trajectory,
    points_per_scan: usize,
    min_range: f64,
    max_range: f64,
    num_accumulated_range_data: usize,
    scan_period: f64,
    imu_period: f64,
    num_imu_per_scan: usize,
    accumulated: Vec<Point2<f64>>,
    accumulated_scans: usize,
    t: f64,
}

impl SyncAccumulateTask {
    pub fn new(config: &WorkloadConfig, seed: u64) -> Self {
        let scan_period = 1.0 / config.scan_rate_hz;
        let imu_period = 1.0 / config.imu_rate_hz;
        let num_imu_per_scan = ((scan_period / imu_period).round() as usize).max(1);
        SyncAccumulateTask {
            rng: StdRng::seed_from_u64(seed),
            world: World::square_room(5.0),
            trajectory: Trajectory {
                circle_radius: 3.0,
                angular_speed: 0.05,
            },
            points_per_scan: config.points_per_scan,
            min_range: config.min_range,
            max_range: config.max_range,
            num_accumulated_range_data: config.num_accumulated_range_data,
            scan_period,
            imu_period,
            num_imu_per_scan,
            accumulated: Vec::new(),
            accumulated_scans: 0,
            t: 0.0,
        }
    }

    /// This scan's own sensor timestamp, before `step` advances it.
    pub fn current_time(&self) -> f64 {
        self.t
    }

    /// Generates synthetic IMU + LIDAR data for one raw scan, feeds the IMU
    /// samples to the shared extrapolator, and accumulates this scan's
    /// points (in map frame) using the extrapolator's current prediction.
    /// Returns the accumulated batch once `num_accumulated_range_data` scans
    /// have been absorbed ([local_trajectory_builder_2d.cc:188]), else
    /// `None`.
    pub fn step(&mut self, extrapolator: &mut PoseExtrapolator) -> Option<AccumulatedBatch> {
        for k in 0..self.num_imu_per_scan {
            let imu_t = self.t + (k as f64) * self.imu_period;
            let true_omega = self.trajectory.angular_velocity_z();
            let noisy_omega = generate_imu_sample(true_omega, &mut self.rng, 0.01);
            extrapolator.add_imu(ImuSample {
                time: imu_t,
                angular_velocity_z: noisy_omega,
            });
        }

        let true_pose = self.trajectory.pose_at(self.t);
        let scan_points_local = generate_scan(
            &self.world,
            true_pose,
            self.points_per_scan,
            self.min_range,
            self.max_range,
            &mut self.rng,
            0.01,
        );

        // Simplification: all points in a scan share timestamp `t` rather
        // than each point's own within-sweep timestamp (see plan's
        // assumptions).
        let predicted_pose_now = extrapolator.extrapolate_pose(self.t);
        for p in &scan_points_local {
            self.accumulated.push(predicted_pose_now * p);
        }

        let scan_t = self.t;
        self.t += self.scan_period;
        self.accumulated_scans += 1;

        if self.accumulated_scans == self.num_accumulated_range_data {
            self.accumulated_scans = 0;
            Some(AccumulatedBatch {
                points: std::mem::take(&mut self.accumulated),
                release_time: scan_t,
            })
        } else {
            None
        }
    }
}

// ---------------------------------------------------------------------------
// Task 2: Filtering (Voxel Filter + Adaptive Voxel Filter). Mirrors
// `TransformToGravityAlignedFrameAndFilter` + `AdaptiveVoxelFilter`
// ([local_trajectory_builder_2d.cc:51-63,228-233]).
// ---------------------------------------------------------------------------

pub struct FilterTask {
    rng: StdRng,
    voxel_filter_size: f64,
    adaptive_options: AdaptiveVoxelFilterOptions,
}

impl FilterTask {
    pub fn new(config: &WorkloadConfig, seed: u64) -> Self {
        FilterTask {
            rng: StdRng::seed_from_u64(seed),
            voxel_filter_size: config.voxel_filter_size,
            adaptive_options: AdaptiveVoxelFilterOptions {
                max_length: config.adaptive_voxel_max_length,
                min_num_points: config.adaptive_voxel_min_num_points,
                max_range: config.adaptive_voxel_max_range,
            },
        }
    }

    /// Recenters the batch relative to the extrapolator's last pose, then
    /// runs the plain voxel filter followed by the adaptive voxel filter.
    pub fn step(&mut self, batch: AccumulatedBatch, extrapolator: &PoseExtrapolator) -> FilteredScan {
        let last_pose = extrapolator.last_pose();
        let local_points: Vec<Point2<f64>> = batch
            .points
            .iter()
            .map(|p| last_pose.inverse() * p)
            .collect();
        let coarsely_filtered = voxel_filter(&local_points, self.voxel_filter_size, &mut self.rng);
        let filtered = adaptive_voxel_filter(&coarsely_filtered, &self.adaptive_options, &mut self.rng);
        FilteredScan {
            points: filtered,
            release_time: batch.release_time,
        }
    }
}

// ---------------------------------------------------------------------------
// Task 3: Scan Matching (correlative + refinement), plus the PoseExtrapolator
// write-back -- folded in here since its own cost is negligible. Mirrors
// `ScanMatch` + the `AddPose` call immediately after it
// ([local_trajectory_builder_2d.cc:65-102,244]).
// ---------------------------------------------------------------------------

pub struct ScanMatchingTask {
    enable_correlative_matching: bool,
    correlative_options: CorrelativeOptions,
    refine_options: RefinementOptions,
}

impl ScanMatchingTask {
    pub fn new(config: &WorkloadConfig) -> Self {
        ScanMatchingTask {
            enable_correlative_matching: config.enable_correlative_matching,
            correlative_options: CorrelativeOptions {
                linear_search_window: config.correlative_linear_search_window,
                angular_search_window: config.correlative_angular_search_window,
                translation_delta_cost_weight: config.correlative_translation_delta_cost_weight,
                rotation_delta_cost_weight: config.correlative_rotation_delta_cost_weight,
                resolution: config.grid_resolution,
            },
            refine_options: RefinementOptions {
                occupied_space_weight: config.ceres_occupied_space_weight,
                translation_weight: config.ceres_translation_weight,
                rotation_weight: config.ceres_rotation_weight,
                max_num_iterations: config.ceres_max_num_iterations,
            },
        }
    }

    /// Predicts a pose from the extrapolator, optionally refines it with the
    /// correlative matcher, runs Gauss-Newton refinement against the current
    /// matching submap grid, and writes the result back to the extrapolator.
    /// Returns the result plus per-stage timing for the correlative and
    /// refine phases.
    pub fn step(
        &mut self,
        scan: FilteredScan,
        extrapolator: &mut PoseExtrapolator,
        matching_grid: &ProbabilityGrid,
    ) -> (MatchedPose, Duration, Duration) {
        let t = scan.release_time;
        let pose_prediction = extrapolator.extrapolate_pose(t);
        let mut matching_pose = pose_prediction;
        let mut stage_corr = Duration::ZERO;
        if self.enable_correlative_matching {
            let corr_start = Instant::now();
            let (p, _score) = correlative_match(
                pose_prediction,
                &scan.points,
                matching_grid,
                &self.correlative_options,
            );
            matching_pose = p;
            stage_corr = corr_start.elapsed();
        }

        let refine_start = Instant::now();
        let refined_pose = refine_pose(
            matching_pose,
            pose_prediction,
            &scan.points,
            matching_grid,
            &self.refine_options,
        );
        let stage_refine = refine_start.elapsed();

        extrapolator.add_pose(t, refined_pose);

        (
            MatchedPose {
                pose: refined_pose,
                points: scan.points,
                release_time: t,
            },
            stage_corr,
            stage_refine,
        )
    }
}

// ---------------------------------------------------------------------------
// Task 4: Motion Filter + Voxel Grid Update. Motion Filter is folded in here
// since it's cheap and exists mainly as the gate for the grid update. Mirrors
// `InsertIntoSubmap`/`ActiveSubmaps2D::InsertRangeData`
// ([local_trajectory_builder_2d.cc:279-300]).
// ---------------------------------------------------------------------------

pub struct MotionFilterGridUpdateTask {
    motion_filter: MotionFilter,
    active_submaps: ActiveSubmaps,
}

impl MotionFilterGridUpdateTask {
    pub fn new(config: &WorkloadConfig) -> Self {
        MotionFilterGridUpdateTask {
            motion_filter: MotionFilter::new(MotionFilterOptions {
                max_time_seconds: config.motion_filter_max_time_seconds,
                max_distance_meters: config.motion_filter_max_distance_meters,
                max_angle_radians: config.motion_filter_max_angle_radians,
            }),
            active_submaps: ActiveSubmaps::new(config.submap_num_range_data, config.grid_resolution),
        }
    }

    /// The submap grid Task 3 matches against -- Task 3's read of Task 4's
    /// state, one of the two cross-task feedback edges in this chain.
    pub fn matching_grid(&self) -> &ProbabilityGrid {
        self.active_submaps.matching_grid()
    }

    /// Decides whether to keep the pose (`MotionFilter::IsSimilar`) and, if
    /// so, ray-casts the matched points into the active submap(s). Returns
    /// (inserted, grid_grew, insert_stage_duration).
    pub fn step(&mut self, matched: MatchedPose) -> (bool, bool, Duration) {
        if self.motion_filter.is_similar(matched.release_time, matched.pose) {
            return (false, false, Duration::ZERO);
        }
        let insert_start = Instant::now();
        let hits_map: Vec<Point2<f64>> = matched.points.iter().map(|p| matched.pose * p).collect();
        let origin_map = Point2::from(matched.pose.translation.vector);
        let grew = self.active_submaps.insert(origin_map, &hits_map);
        let stage_insert = insert_start.elapsed();
        (true, grew, stage_insert)
    }
}

// ---------------------------------------------------------------------------
// Orchestrator: owns the 4 tasks plus the one other cross-task store
// (PoseExtrapolator), and drives them. This is where the explicit borrows
// documenting each cross-task dependency live; the natural seam for a real
// dispatcher to later replace with channels/locks between actual threads.
// ---------------------------------------------------------------------------

pub struct LocalSlamChain {
    config: WorkloadConfig,
    extrapolator: PoseExtrapolator,
    task1: SyncAccumulateTask,
    task2: FilterTask,
    task3: ScanMatchingTask,
    task4: MotionFilterGridUpdateTask,
    scan_index: usize,
    release_index: usize,
    last_release_time: Option<f64>,
}

impl LocalSlamChain {
    pub fn new(config: WorkloadConfig) -> Self {
        let mut extrapolator = PoseExtrapolator::new(config.pose_queue_duration);
        extrapolator.add_pose(0.0, Isometry2::identity());

        let task1 = SyncAccumulateTask::new(&config, config.seed);
        let task2 = FilterTask::new(&config, config.seed.wrapping_add(1));
        let task3 = ScanMatchingTask::new(&config);
        let task4 = MotionFilterGridUpdateTask::new(&config);

        LocalSlamChain {
            config,
            extrapolator,
            task1,
            task2,
            task3,
            task4,
            scan_index: 0,
            release_index: 0,
            last_release_time: None,
        }
    }

    /// Advances by one raw scan. Always runs and times Task 1; if that
    /// produces a batch, also runs Tasks 2-4 as one release and times them
    /// together.
    pub fn tick(&mut self) -> (ScanRecord, Option<ReleaseRecord>) {
        let scan_t = self.task1.current_time();
        let wall_start = Instant::now();
        let cpu_start = cpu_time_now();
        let batch = self.task1.step(&mut self.extrapolator);
        let wall_elapsed = wall_start.elapsed().as_secs_f64();
        let cpu_elapsed = cpu_time_now() - cpu_start;

        let scan_record = ScanRecord {
            scan_index: self.scan_index,
            sensor_time: scan_t,
            wall_exec_secs: wall_elapsed,
            cpu_exec_secs: cpu_elapsed,
            produced_batch: batch.is_some(),
        };
        self.scan_index += 1;

        let release_record = batch.map(|b| self.run_release(b));
        (scan_record, release_record)
    }

    fn run_release(&mut self, batch: AccumulatedBatch) -> ReleaseRecord {
        let t = batch.release_time;
        let wall_start = Instant::now();
        let cpu_start = cpu_time_now();

        let filter_start = Instant::now();
        let scan = self.task2.step(batch, &self.extrapolator);
        let stage_filter = filter_start.elapsed();

        let grid = self.task4.matching_grid();
        let (matched, stage_corr, stage_refine) = self.task3.step(scan, &mut self.extrapolator, grid);

        let (inserted, grew, stage_insert) = self.task4.step(matched);

        let wall_elapsed = wall_start.elapsed().as_secs_f64();
        let cpu_elapsed = cpu_time_now() - cpu_start;

        let base_period = (1.0 / self.config.scan_rate_hz) * self.config.num_accumulated_range_data as f64;
        let period = self
            .last_release_time
            .map(|lt| t - lt)
            .unwrap_or(base_period);
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

        if self.config.real_time {
            std::thread::sleep(Duration::from_secs_f64(period.max(0.0)));
        }

        record
    }
}

impl Default for LocalSlamChain {
    fn default() -> Self {
        LocalSlamChain::new(WorkloadConfig::default())
    }
}

/// Runs `config.duration_secs` of simulated sensor time, tick by tick.
/// Release-level results match the original single-shot benchmark's
/// semantics exactly (same release count, same deadline/warm-up logic);
/// exact timing values differ slightly from any earlier single-RNG-stream
/// version since each task now draws from its own independent RNG stream.
pub fn run(config: WorkloadConfig) -> (Vec<ScanRecord>, Vec<ReleaseRecord>, ScanSummary, Summary) {
    let scan_period = 1.0 / config.scan_rate_hz;
    let num_scans = (config.duration_secs / scan_period).floor() as usize;
    let warmup_releases = config.warmup_releases;
    let warmup_scans = warmup_releases * config.num_accumulated_range_data;

    let mut chain = LocalSlamChain::new(config);
    let mut scan_records = Vec::with_capacity(num_scans);
    let mut release_records = Vec::new();
    for _ in 0..num_scans {
        let (scan_record, release_record) = chain.tick();
        scan_records.push(scan_record);
        if let Some(r) = release_record {
            release_records.push(r);
        }
    }

    let scan_summary = compute_scan_summary(&scan_records, warmup_scans);
    let summary = compute_summary(&release_records, warmup_releases);
    (scan_records, release_records, scan_summary, summary)
}

fn percentile(sorted: &[f64], p: f64) -> f64 {
    if sorted.is_empty() {
        return 0.0;
    }
    let idx = ((sorted.len() as f64 - 1.0) * p).round() as usize;
    sorted[idx.min(sorted.len() - 1)]
}

fn compute_scan_summary(records: &[ScanRecord], warmup: usize) -> ScanSummary {
    let data: Vec<&ScanRecord> = records.iter().skip(warmup).collect();
    let mut wall: Vec<f64> = data.iter().map(|r| r.wall_exec_secs).collect();
    let mut cpu: Vec<f64> = data.iter().map(|r| r.cpu_exec_secs).collect();
    wall.sort_by(|a, b| a.partial_cmp(b).unwrap());
    cpu.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let n = (data.len().max(1)) as f64;
    ScanSummary {
        num_scans: data.len(),
        mean_wall_secs: data.iter().map(|r| r.wall_exec_secs).sum::<f64>() / n,
        p95_wall_secs: percentile(&wall, 0.95),
        max_wall_secs: wall.last().copied().unwrap_or(0.0),
        mean_cpu_secs: data.iter().map(|r| r.cpu_exec_secs).sum::<f64>() / n,
        p95_cpu_secs: percentile(&cpu, 0.95),
        max_cpu_secs: cpu.last().copied().unwrap_or(0.0),
        batches_produced: data.iter().filter(|r| r.produced_batch).count(),
    }
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

pub fn write_release_csv(path: &str, records: &[ReleaseRecord]) -> std::io::Result<()> {
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

pub fn write_scan_csv(path: &str, records: &[ScanRecord]) -> std::io::Result<()> {
    use std::io::Write;
    let mut f = std::fs::File::create(path)?;
    writeln!(f, "scan_index,sensor_time,wall_exec_secs,cpu_exec_secs,produced_batch")?;
    for r in records {
        writeln!(
            f,
            "{},{:.6},{:.6},{:.6},{}",
            r.scan_index, r.sensor_time, r.wall_exec_secs, r.cpu_exec_secs, r.produced_batch
        )?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chain_produces_expected_release_count() {
        let mut config = WorkloadConfig::default();
        config.duration_secs = 5.0;
        config.num_accumulated_range_data = 1;
        let (scan_records, release_records, _, _) = run(config.clone());
        let expected_scans = (config.duration_secs * config.scan_rate_hz).floor() as usize;
        assert_eq!(scan_records.len(), expected_scans);
        assert_eq!(release_records.len(), expected_scans / config.num_accumulated_range_data);
    }

    #[test]
    fn accumulation_task_fires_more_often_than_releases() {
        let mut config = WorkloadConfig::default();
        config.duration_secs = 5.0;
        config.num_accumulated_range_data = 5;
        let (scan_records, release_records, _, _) = run(config.clone());
        assert_eq!(scan_records.len(), release_records.len() * config.num_accumulated_range_data);
    }
}
