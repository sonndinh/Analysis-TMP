use crate::dispatcher::EDFTask;
use crate::qpa::Task;
use rt_slam_workload::workload::LocalSlamChain;
use typenum::{P50, P100};

/// Wraps the ported `LocalTrajectoryBuilder2D` real-time workload as a `Task`.
///
/// `Wcet`/`Deadline`/`Period` below are PLACEHOLDERS, not calibrated values --
/// deriving a safe compile-time bound from an actual benchmark run of
/// `rt_slam_workload` is deliberately deferred to a later step. This type
/// still models the *whole* local-SLAM chain as a single `Task` (one
/// `do_work` call = one release, i.e. Tasks 2-4 of the 4-task decomposition,
/// preceded by however many Task-1 scan ticks it takes to produce a batch) --
/// splitting it into 4 separate `Task` impls, one per decomposed stage, is
/// the natural next step, not done here.
pub struct LocalTrajectoryBuilderTask;

impl Task for LocalTrajectoryBuilderTask {
    // Placeholder period (ms): matches the ported workload's default
    // (num_accumulated_range_data=1 / scan_rate_hz=10 -> 100ms). Wcet/Deadline
    // are placeholders too, pending calibration against a real benchmark run.
    type Wcet = P50;
    type Deadline = P100;
    type Period = P100;
    type State = LocalSlamChain;

    fn do_work(state: &mut Self::State) {
        // Tick Task 1 until it produces a batch and the rest of the chain
        // runs -- one do_work call is still exactly one release.
        loop {
            let (_scan_record, release_record) = state.tick();
            if release_record.is_some() {
                break;
            }
        }
    }
}

impl EDFTask for LocalTrajectoryBuilderTask {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dispatcher::{Dispatcher, EDF, QPATest};
    use crate::qpa::{Nulltask, Tasklist};

    #[test]
    fn do_work_runs_repeated_releases_without_panicking() {
        let mut state = <LocalTrajectoryBuilderTask as Task>::State::default();
        for _ in 0..3 {
            LocalTrajectoryBuilderTask::do_work(&mut state);
        }
    }

    #[test]
    fn dispatches_through_the_full_edf_pipeline() {
        // Exercises the real dispatcher path (thread spawn, park/unpark, the
        // per-job-release do_work loop, join) with this task, not just direct
        // do_work calls -- proves the end-to-end wiring, not only the pipeline
        // logic in isolation.
        type Taskset = Tasklist<LocalTrajectoryBuilderTask, Nulltask>;
        Dispatcher::<Taskset, EDF, QPATest>::dispatch(3);
    }
}
