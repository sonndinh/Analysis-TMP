use nalgebra::{Isometry2, Point2, Vector2};
use rand::Rng;
use rand_distr::{Distribution, Normal};

pub struct Segment {
    pub a: Point2<f64>,
    pub b: Point2<f64>,
}

/// A simple bounded square-room scene, ray-cast to produce synthetic LIDAR
/// returns. Deliberately simple (see plan's "synthetic environment only"
/// simplification) -- what matters is that it's large enough, and the
/// trajectory long enough, for the grid to need to grow at least a few times.
pub struct World {
    pub segments: Vec<Segment>,
}

impl World {
    pub fn square_room(half_size: f64) -> Self {
        let s = half_size;
        let corners = [
            Point2::new(-s, -s),
            Point2::new(s, -s),
            Point2::new(s, s),
            Point2::new(-s, s),
        ];
        let mut segments = Vec::new();
        for i in 0..4 {
            segments.push(Segment {
                a: corners[i],
                b: corners[(i + 1) % 4],
            });
        }
        World { segments }
    }

    /// Casts a ray from `origin` at `angle` (world frame, radians) and returns
    /// the distance to the nearest wall within `max_range`, if any.
    pub fn cast_ray(&self, origin: Point2<f64>, angle: f64, max_range: f64) -> Option<f64> {
        let dir = Vector2::new(angle.cos(), angle.sin());
        let mut best: Option<f64> = None;
        for seg in &self.segments {
            if let Some(t) = ray_segment_intersection(origin, dir, seg.a, seg.b) {
                if t >= 0.0 && t <= max_range {
                    best = Some(best.map_or(t, |b: f64| b.min(t)));
                }
            }
        }
        best
    }
}

fn ray_segment_intersection(
    o: Point2<f64>,
    d: Vector2<f64>,
    a: Point2<f64>,
    b: Point2<f64>,
) -> Option<f64> {
    let v1 = o - a;
    let v2 = b - a;
    let v3 = Vector2::new(-d.y, d.x);
    let dot = v2.dot(&v3);
    if dot.abs() < 1e-9 {
        return None;
    }
    let t1 = (v2.x * v1.y - v2.y * v1.x) / dot;
    let t2 = v1.dot(&v3) / dot;
    if t1 >= 0.0 && (0.0..=1.0).contains(&t2) {
        Some(t1)
    } else {
        None
    }
}

/// The ground-truth robot motion driving the synthetic sensors: a slow
/// constant-speed circle inside the room. Calibrated (see plan's "Synthetic
/// environment & trajectory design") so the `MotionFilter`'s distance/angle
/// thresholds are crossed every few releases, and so the trajectory eventually
/// reaches map area outside the grid's initial bounds, triggering growth.
pub struct Trajectory {
    pub circle_radius: f64,
    pub angular_speed: f64,
}

impl Trajectory {
    pub fn pose_at(&self, t: f64) -> Isometry2<f64> {
        let angle = self.angular_speed * t;
        let x = self.circle_radius * angle.cos();
        let y = self.circle_radius * angle.sin();
        let heading = angle + std::f64::consts::FRAC_PI_2;
        Isometry2::new(Vector2::new(x, y), heading)
    }

    pub fn angular_velocity_z(&self) -> f64 {
        self.angular_speed
    }
}

/// Generates one synthetic LIDAR scan in the SENSOR-LOCAL frame (relative to
/// `robot_pose`), matching `TimedPointCloudData`'s per-point sensor-frame
/// convention.
pub fn generate_scan(
    world: &World,
    robot_pose: Isometry2<f64>,
    num_points: usize,
    min_range: f64,
    max_range: f64,
    rng: &mut impl Rng,
    noise_std: f64,
) -> Vec<Point2<f64>> {
    let normal = Normal::new(0.0, noise_std.max(1e-6)).unwrap();
    let mut points = Vec::with_capacity(num_points);
    for i in 0..num_points {
        let local_angle =
            -std::f64::consts::PI + 2.0 * std::f64::consts::PI * (i as f64) / (num_points as f64);
        let world_angle = robot_pose.rotation.angle() + local_angle;
        let origin = Point2::from(robot_pose.translation.vector);
        if let Some(range) = world.cast_ray(origin, world_angle, max_range) {
            let noisy_range = (range + normal.sample(rng)).max(0.0);
            // Mirrors the `range >= options_.min_range()` drop in `AddRangeData`
            // ([local_trajectory_builder_2d.cc:175]).
            if noisy_range < min_range {
                continue;
            }
            points.push(Point2::new(
                local_angle.cos() * noisy_range,
                local_angle.sin() * noisy_range,
            ));
        }
    }
    points
}

pub fn generate_imu_sample(true_angular_velocity_z: f64, rng: &mut impl Rng, noise_std: f64) -> f64 {
    let normal = Normal::new(0.0, noise_std.max(1e-9)).unwrap();
    true_angular_velocity_z + normal.sample(rng)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cast_ray_hits_wall() {
        let world = World::square_room(5.0);
        let hit = world.cast_ray(Point2::new(0.0, 0.0), 0.0, 30.0);
        assert!((hit.unwrap() - 5.0).abs() < 1e-9);
    }

    #[test]
    fn cast_ray_respects_max_range() {
        let world = World::square_room(5.0);
        let hit = world.cast_ray(Point2::new(0.0, 0.0), 0.0, 1.0);
        assert!(hit.is_none());
    }
}
