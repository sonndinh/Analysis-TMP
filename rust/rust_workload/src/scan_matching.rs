use crate::grid::ProbabilityGrid;
use nalgebra::{DMatrix, DVector, Isometry2, Matrix3, Point2, Vector2};

pub struct CorrelativeOptions {
    pub linear_search_window: f64,
    pub angular_search_window: f64,
    pub translation_delta_cost_weight: f64,
    pub rotation_delta_cost_weight: f64,
    pub resolution: f64,
}

/// Brute-force search over a discretized (dx, dy, dtheta) window, scoring mean
/// occupancy probability per candidate against the grid, mirroring
/// `RealTimeCorrelativeScanMatcher2D::Match` ([real_time_correlative_scan_matcher_2d.cc:117]).
/// This is the dominant cost of the pipeline: candidate count grows with both
/// search-window size and grid resolution.
pub fn correlative_match(
    pose_prediction: Isometry2<f64>,
    points: &[Point2<f64>],
    grid: &ProbabilityGrid,
    options: &CorrelativeOptions,
) -> (Isometry2<f64>, f64) {
    let max_scan_range = points
        .iter()
        .map(|p| p.coords.norm())
        .fold(0.0_f64, f64::max)
        .max(1e-3);
    let angular_step = (options.resolution / max_scan_range).max(1e-4);
    let num_angular_steps = (options.angular_search_window / angular_step).ceil() as i64;
    let num_linear_steps = (options.linear_search_window / options.resolution).ceil() as i64;

    let base_angle = pose_prediction.rotation.angle();
    let base_translation = pose_prediction.translation.vector;

    let mut best_score = f64::NEG_INFINITY;
    let mut best_pose = pose_prediction;

    for a in -num_angular_steps..=num_angular_steps {
        let dtheta = a as f64 * angular_step;
        let rotation = nalgebra::UnitComplex::new(base_angle + dtheta);
        let rotated_points: Vec<Point2<f64>> = points.iter().map(|p| rotation * p).collect();
        for ix in -num_linear_steps..=num_linear_steps {
            let dx = ix as f64 * options.resolution;
            for iy in -num_linear_steps..=num_linear_steps {
                let dy = iy as f64 * options.resolution;
                let translation = base_translation + Vector2::new(dx, dy);
                let mut sum = 0.0;
                for p in &rotated_points {
                    sum += grid.probability_at(Point2::from(translation) + p.coords);
                }
                let mean_score = sum / (rotated_points.len().max(1) as f64);
                let penalty = options.translation_delta_cost_weight * (dx * dx + dy * dy).sqrt()
                    + options.rotation_delta_cost_weight * dtheta.abs();
                let score = mean_score - penalty;
                if score > best_score {
                    best_score = score;
                    best_pose = Isometry2::new(translation, base_angle + dtheta);
                }
            }
        }
    }
    (best_pose, best_score)
}

pub struct RefinementOptions {
    pub occupied_space_weight: f64,
    pub translation_weight: f64,
    pub rotation_weight: f64,
    pub max_num_iterations: usize,
}

fn wrap_angle(a: f64) -> f64 {
    a.sin().atan2(a.cos())
}

fn compute_residuals(
    x: f64,
    y: f64,
    theta: f64,
    target: Isometry2<f64>,
    points: &[Point2<f64>],
    grid: &ProbabilityGrid,
    options: &RefinementOptions,
) -> DVector<f64> {
    let pose = Isometry2::new(Vector2::new(x, y), theta);
    let mut residuals = Vec::with_capacity(points.len() + 3);
    for p in points {
        let world = pose * p;
        let prob = grid.probability_at(world);
        residuals.push(options.occupied_space_weight * (1.0 - prob));
    }
    residuals.push(options.translation_weight * (x - target.translation.vector.x));
    residuals.push(options.translation_weight * (y - target.translation.vector.y));
    residuals.push(
        options.rotation_weight * wrap_angle(theta - target.rotation.angle()),
    );
    DVector::from_vec(residuals)
}

/// Fixed-iteration Gauss-Newton refinement with finite-difference Jacobians,
/// standing in for `CeresScanMatcher2D::Match` ([ceres_scan_matcher_2d.cc:63]).
/// Same three cost terms (occupied-space per point, translation, rotation) and
/// the same default iteration budget, but no autodiff/bicubic interpolation --
/// see the plan's "Assumptions & explicit simplifications" for what's approximated.
pub fn refine_pose(
    initial_pose: Isometry2<f64>,
    target: Isometry2<f64>,
    points: &[Point2<f64>],
    grid: &ProbabilityGrid,
    options: &RefinementOptions,
) -> Isometry2<f64> {
    let mut x = initial_pose.translation.vector.x;
    let mut y = initial_pose.translation.vector.y;
    let mut theta = initial_pose.rotation.angle();
    let eps = 1e-6;
    let damping = 1e-6;

    for _ in 0..options.max_num_iterations {
        let r0 = compute_residuals(x, y, theta, target, points, grid, options);
        let n = r0.len();
        let mut jac = DMatrix::<f64>::zeros(n, 3);
        let deltas = [(eps, 0.0, 0.0), (0.0, eps, 0.0), (0.0, 0.0, eps)];
        for (col, (ddx, ddy, ddt)) in deltas.iter().enumerate() {
            let r = compute_residuals(x + ddx, y + ddy, theta + ddt, target, points, grid, options);
            for i in 0..n {
                jac[(i, col)] = (r[i] - r0[i]) / eps;
            }
        }
        let jt = jac.transpose();
        let jtj = &jt * &jac + Matrix3::identity() * damping;
        let jtr = &jt * &r0;
        match jtj.try_inverse() {
            Some(inv) => {
                let step = -(inv * jtr);
                x += step[0];
                y += step[1];
                theta += step[2];
                if step.norm() < 1e-9 {
                    break;
                }
            }
            None => break,
        }
    }
    Isometry2::new(Vector2::new(x, y), theta)
}

#[cfg(test)]
mod tests {
    use super::*;

    // An "L"-shaped corner (points along both x and y) rather than a single
    // near-collinear cluster: a collinear cluster makes small rotations and
    // small lateral translations nearly indistinguishable (the classic
    // aperture problem), which would make this test flaky for reasons that
    // have nothing to do with a matcher bug.
    fn corner_points() -> Vec<Point2<f64>> {
        vec![
            Point2::new(1.0, 0.0),
            Point2::new(0.95, 0.0),
            Point2::new(0.0, 1.0),
            Point2::new(0.0, 0.95),
        ]
    }

    fn build_test_grid() -> ProbabilityGrid {
        let mut grid = ProbabilityGrid::new(0.05);
        let origin = Point2::new(0.0, 0.0);
        for hit in corner_points() {
            for _ in 0..5 {
                grid.insert_ray(origin, hit);
            }
        }
        grid
    }

    #[test]
    fn correlative_match_improves_on_offset_prediction() {
        let grid = build_test_grid();
        let points = corner_points();
        let options = CorrelativeOptions {
            linear_search_window: 0.1,
            angular_search_window: 20.0_f64.to_radians(),
            translation_delta_cost_weight: 0.1,
            rotation_delta_cost_weight: 0.1,
            resolution: 0.05,
        };
        let true_pose = Isometry2::identity();
        let (_, true_score) = correlative_match(true_pose, &points, &grid, &options);

        let offset_prediction = Isometry2::new(Vector2::new(0.05, 0.05), 0.0);
        let (recovered_pose, _) = correlative_match(offset_prediction, &points, &grid, &options);
        let recovered_score_at_true_eval = {
            // Re-score the recovered pose directly against the grid to compare.
            let mut sum = 0.0;
            for p in &points {
                sum += grid.probability_at(recovered_pose * p);
            }
            sum / points.len() as f64
        };
        assert!(recovered_score_at_true_eval >= true_score - 0.2);
        assert!((recovered_pose.translation.vector - Vector2::new(0.0, 0.0)).norm() < 0.1);
    }

    #[test]
    fn refine_pose_pulls_toward_occupied_cells() {
        let grid = build_test_grid();
        let points = corner_points();
        let options = RefinementOptions {
            occupied_space_weight: 1.0,
            translation_weight: 10.0,
            rotation_weight: 40.0,
            max_num_iterations: 20,
        };
        let initial = Isometry2::new(Vector2::new(0.03, 0.03), 0.0);
        let target = Isometry2::identity();
        let refined = refine_pose(initial, target, &points, &grid, &options);
        let dist_before = (initial.translation.vector - Vector2::new(0.0, 0.0)).norm();
        let dist_after = (refined.translation.vector - Vector2::new(0.0, 0.0)).norm();
        assert!(dist_after <= dist_before);
    }
}
