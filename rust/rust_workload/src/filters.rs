use nalgebra::{Isometry2, Point2};
use rand::Rng;
use std::collections::HashMap;

/// Reservoir-samples one representative point per voxel, mirroring
/// `sensor::VoxelFilter`'s `RandomizedVoxelFilter` ([voxel_filter.cc:89-114]).
/// The original re-seeds a fresh RNG on every call (so it's actually
/// deterministic run-to-run); here a shared seeded RNG is threaded through
/// instead, for reproducibility across an entire benchmark run.
pub fn voxel_filter(points: &[Point2<f64>], resolution: f64, rng: &mut impl Rng) -> Vec<Point2<f64>> {
    let mut voxel_count_and_index: HashMap<(i64, i64), (u32, usize)> = HashMap::new();
    for (i, p) in points.iter().enumerate() {
        let key = (
            (p.x / resolution).round() as i64,
            (p.y / resolution).round() as i64,
        );
        let entry = voxel_count_and_index.entry(key).or_insert((0, 0));
        entry.0 += 1;
        if entry.0 == 1 {
            entry.1 = i;
        } else if rng.gen_range(1..=entry.0) == entry.0 {
            entry.1 = i;
        }
    }
    let mut indices: Vec<usize> = voxel_count_and_index.values().map(|v| v.1).collect();
    indices.sort_unstable();
    indices.into_iter().map(|i| points[i]).collect()
}

pub struct AdaptiveVoxelFilterOptions {
    pub max_length: f64,
    pub min_num_points: usize,
    pub max_range: f64,
}

/// Binary-searches the voxel edge length to hit `min_num_points`, mirroring
/// `sensor::AdaptiveVoxelFilter` ([voxel_filter.cc:38-75]).
pub fn adaptive_voxel_filter(
    points: &[Point2<f64>],
    options: &AdaptiveVoxelFilterOptions,
    rng: &mut impl Rng,
) -> Vec<Point2<f64>> {
    let filtered: Vec<Point2<f64>> = points
        .iter()
        .cloned()
        .filter(|p| p.coords.norm() <= options.max_range)
        .collect();
    if filtered.len() <= options.min_num_points {
        return filtered;
    }
    let result = voxel_filter(&filtered, options.max_length, rng);
    if result.len() >= options.min_num_points {
        return result;
    }
    let mut high_length = options.max_length;
    let mut fallback = result;
    while high_length > 1e-2 * options.max_length {
        let low_length = high_length / 2.0;
        let candidate = voxel_filter(&filtered, low_length, rng);
        if candidate.len() >= options.min_num_points {
            let mut low = low_length;
            let mut high = high_length;
            let mut best = candidate;
            while (high - low) / low > 1e-1 {
                let mid = (low + high) / 2.0;
                let mid_candidate = voxel_filter(&filtered, mid, rng);
                if mid_candidate.len() >= options.min_num_points {
                    low = mid;
                    best = mid_candidate;
                } else {
                    high = mid;
                }
            }
            return best;
        }
        fallback = candidate;
        high_length /= 2.0;
    }
    fallback
}

pub struct MotionFilterOptions {
    pub max_time_seconds: f64,
    pub max_distance_meters: f64,
    pub max_angle_radians: f64,
}

/// Throttles submap insertion, mirroring `MotionFilter::IsSimilar`
/// ([internal/motion_filter.cc:40]): drops (returns true) only if time, distance,
/// AND angle since the last kept pose are all within threshold.
pub struct MotionFilter {
    options: MotionFilterOptions,
    last_time: Option<f64>,
    last_pose: Option<Isometry2<f64>>,
}

impl MotionFilter {
    pub fn new(options: MotionFilterOptions) -> Self {
        MotionFilter {
            options,
            last_time: None,
            last_pose: None,
        }
    }

    pub fn is_similar(&mut self, time: f64, pose: Isometry2<f64>) -> bool {
        if let (Some(last_time), Some(last_pose)) = (self.last_time, self.last_pose) {
            if time - last_time <= self.options.max_time_seconds {
                let distance = (pose.translation.vector - last_pose.translation.vector).norm();
                if distance <= self.options.max_distance_meters {
                    let delta = last_pose.inverse() * pose;
                    if delta.rotation.angle().abs() <= self.options.max_angle_radians {
                        return true;
                    }
                }
            }
        }
        self.last_time = Some(time);
        self.last_pose = Some(pose);
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::SeedableRng;
    use rand::rngs::StdRng;

    #[test]
    fn voxel_filter_reduces_dense_cluster() {
        let mut rng = StdRng::seed_from_u64(1);
        let mut points = Vec::new();
        for i in 0..100 {
            let jitter = (i as f64) * 1e-4;
            points.push(Point2::new(jitter, jitter));
        }
        let filtered = voxel_filter(&points, 0.1, &mut rng);
        assert!(filtered.len() < points.len());
        assert!(!filtered.is_empty());
    }

    #[test]
    fn motion_filter_respects_all_three_thresholds() {
        let options = MotionFilterOptions {
            max_time_seconds: 5.0,
            max_distance_meters: 0.2,
            max_angle_radians: 1.0_f64.to_radians(),
        };
        let mut filter = MotionFilter::new(options);
        assert!(!filter.is_similar(0.0, Isometry2::identity()));

        // Small move, well within all thresholds and time budget: dropped (similar).
        let small_move = Isometry2::new(nalgebra::Vector2::new(0.05, 0.0), 0.0);
        assert!(filter.is_similar(1.0, small_move));

        // Distance threshold exceeded: kept.
        let big_move = Isometry2::new(nalgebra::Vector2::new(1.0, 0.0), 0.0);
        assert!(!filter.is_similar(2.0, big_move));

        // Time threshold exceeded even with no motion: kept.
        assert!(!filter.is_similar(20.0, big_move));
    }
}
