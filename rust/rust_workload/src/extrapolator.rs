use nalgebra::{Isometry2, Vector2};
use std::collections::VecDeque;

/// Simplified 2D stand-in for `PoseExtrapolator`. The original tracks full 3D
/// orientation via `ImuTracker` to separate gravity/pitch/roll from heading; since
/// this benchmark's synthetic world is flat by construction (see plan's
/// "Assumptions & explicit simplifications"), IMU integration here only tracks
/// yaw rate, which still exercises the same O(k)-in-buffered-samples cost shape
/// as the original's `AdvanceImuTracker` ([pose_extrapolator.cc:196]).
#[derive(Clone, Copy)]
pub struct ImuSample {
    pub time: f64,
    pub angular_velocity_z: f64,
}

pub struct PoseExtrapolator {
    pose_queue_duration: f64,
    poses: VecDeque<(f64, Isometry2<f64>)>,
    imu_queue: VecDeque<ImuSample>,
    linear_velocity: Vector2<f64>,
    angular_velocity_z: f64,
}

impl PoseExtrapolator {
    pub fn new(pose_queue_duration: f64) -> Self {
        PoseExtrapolator {
            pose_queue_duration,
            poses: VecDeque::new(),
            imu_queue: VecDeque::new(),
            linear_velocity: Vector2::zeros(),
            angular_velocity_z: 0.0,
        }
    }

    pub fn has_pose(&self) -> bool {
        !self.poses.is_empty()
    }

    pub fn last_pose(&self) -> Isometry2<f64> {
        self.poses.back().expect("no pose yet").1
    }

    pub fn last_pose_time(&self) -> f64 {
        self.poses.back().expect("no pose yet").0
    }

    /// Adds an observed pose, updates the constant-velocity estimate from the
    /// pose queue's oldest/newest entries (mirrors `UpdateVelocitiesFromPoses`,
    /// [pose_extrapolator.cc:156]), and drops now-stale buffered IMU samples.
    pub fn add_pose(&mut self, time: f64, pose: Isometry2<f64>) {
        self.poses.push_back((time, pose));
        while self.poses.len() > 1 && time - self.poses.front().unwrap().0 > self.pose_queue_duration {
            self.poses.pop_front();
        }
        if self.poses.len() >= 2 {
            let (t0, p0) = self.poses[0];
            let (t1, p1) = *self.poses.back().unwrap();
            let dt = t1 - t0;
            if dt > 0.0 {
                self.linear_velocity = (p1.translation.vector - p0.translation.vector) / dt;
                let delta = p0.inverse() * p1;
                self.angular_velocity_z = delta.rotation.angle() / dt;
            }
        }
        while let Some(front) = self.imu_queue.front() {
            if front.time <= time {
                self.imu_queue.pop_front();
            } else {
                break;
            }
        }
    }

    pub fn add_imu(&mut self, sample: ImuSample) {
        self.imu_queue.push_back(sample);
    }

    /// Extrapolates the pose at `time`: constant-velocity translation, plus a
    /// yaw delta integrated over any buffered IMU samples since the last known
    /// pose (mirrors `ExtrapolatePose`, [pose_extrapolator.cc:134-196]).
    pub fn extrapolate_pose(&self, time: f64) -> Isometry2<f64> {
        let (last_time, last_pose) = *self.poses.back().expect("no pose yet");
        let dt = time - last_time;
        let translation = last_pose.translation.vector + self.linear_velocity * dt;

        let mut angle = last_pose.rotation.angle();
        let mut cursor = last_time;
        let mut last_rate = self.angular_velocity_z;
        for sample in self.imu_queue.iter() {
            if sample.time <= last_time {
                continue;
            }
            if sample.time > time {
                break;
            }
            angle += last_rate * (sample.time - cursor);
            cursor = sample.time;
            last_rate = sample.angular_velocity_z;
        }
        angle += last_rate * (time - cursor);

        Isometry2::new(translation, angle)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn constant_velocity_extrapolation() {
        let mut ex = PoseExtrapolator::new(1.0);
        ex.add_pose(0.0, Isometry2::identity());
        ex.add_pose(1.0, Isometry2::new(Vector2::new(1.0, 0.0), 0.0));
        let p = ex.extrapolate_pose(2.0);
        assert!((p.translation.vector - Vector2::new(2.0, 0.0)).norm() < 1e-9);
    }

    #[test]
    fn imu_integration_advances_yaw() {
        let mut ex = PoseExtrapolator::new(1.0);
        ex.add_pose(0.0, Isometry2::identity());
        ex.add_imu(ImuSample {
            time: 0.5,
            angular_velocity_z: std::f64::consts::PI,
        });
        let p = ex.extrapolate_pose(1.0);
        // From t=0 to 0.5 no imu sample yet applies (falls back to 0 base rate),
        // from 0.5 to 1.0 integrates pi rad/s over 0.5s = pi/2.
        assert!((p.rotation.angle() - std::f64::consts::FRAC_PI_2).abs() < 1e-9);
    }
}
