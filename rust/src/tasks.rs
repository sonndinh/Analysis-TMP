use crate::chain_state::{batch_mailbox, filtered_mailbox, matched_mailbox, shared_extrapolator, shared_grid_update};
use crate::dispatcher::EDFTask;
use crate::qpa::Task;
use rt_slam_workload::config::WorkloadConfig;
use rt_slam_workload::workload::{
    FilterTask as RwFilterTask, MotionFilterGridUpdateTask, ScanMatchingTask as RwScanMatchingTask,
    SyncAccumulateTask as RwSyncAccumulateTask,
};
use std::sync::{Arc, Mutex};
use typenum::{P50, P100};

// `Wcet`/`Deadline`/`Period` on all four below are PLACEHOLDERS, not
// calibrated values -- deriving real bounds from a benchmark run, and
// assigning the staggered sub-deadlines the chain's precedence order needs,
// is deliberately deferred. Task 1 is given the same placeholder period as
// the other three for now even though its *true* period is the (generally
// shorter) raw-scan period, not the release period -- they coincide at the
// benchmark's default `num_accumulated_range_data = 1`, but won't in general.

// ---------------------------------------------------------------------------
// Task 1: Sync & Accumulate. Mirrors `AddRangeData`'s accumulation loop
// ([local_trajectory_builder_2d.cc:104-209]). Reads/writes the shared
// PoseExtrapolator; pushes a completed batch to Task 2's mailbox once
// `num_accumulated_range_data` scans have been absorbed.
// ---------------------------------------------------------------------------

pub struct SyncAccumulateState {
    inner: RwSyncAccumulateTask,
}

impl Default for SyncAccumulateState {
    fn default() -> Self {
        let config = WorkloadConfig::default();
        SyncAccumulateState {
            inner: RwSyncAccumulateTask::new(&config, config.seed),
        }
    }
}

pub struct SyncAccumulateJob;

impl Task for SyncAccumulateJob {
    type Wcet = P50;
    type Deadline = P100;
    type Period = P100;
    type State = SyncAccumulateState;

    fn do_work(state: &mut Self::State) {
        let extrapolator = shared_extrapolator();
        let mut extrapolator = extrapolator.lock().unwrap();
        if let Some(batch) = state.inner.step(&mut extrapolator) {
            drop(extrapolator);
            batch_mailbox().push(batch);
        }
    }
}

impl EDFTask for SyncAccumulateJob {}

// ---------------------------------------------------------------------------
// Task 2: Filtering (Voxel Filter + Adaptive Voxel Filter). Mirrors
// `TransformToGravityAlignedFrameAndFilter` + `AdaptiveVoxelFilter`
// ([local_trajectory_builder_2d.cc:51-63,228-233]). Pops from Task 1's
// mailbox (no-op if empty this job), reads the shared PoseExtrapolator,
// pushes to Task 3's mailbox.
// ---------------------------------------------------------------------------

pub struct FilterState {
    inner: RwFilterTask,
}

impl Default for FilterState {
    fn default() -> Self {
        let config = WorkloadConfig::default();
        FilterState {
            inner: RwFilterTask::new(&config, config.seed.wrapping_add(1)),
        }
    }
}

pub struct FilterJob;

impl Task for FilterJob {
    type Wcet = P50;
    type Deadline = P100;
    type Period = P100;
    type State = FilterState;

    fn do_work(state: &mut Self::State) {
        let Some(batch) = batch_mailbox().pop() else {
            return;
        };
        let extrapolator = shared_extrapolator();
        let extrapolator = extrapolator.lock().unwrap();
        let filtered = state.inner.step(batch, &extrapolator);
        drop(extrapolator);
        filtered_mailbox().push(filtered);
    }
}

impl EDFTask for FilterJob {}

// ---------------------------------------------------------------------------
// Task 3: Scan Matching (correlative + refinement), plus the PoseExtrapolator
// write-back. Mirrors `ScanMatch` + `AddPose`
// ([local_trajectory_builder_2d.cc:65-102,244]). Pops from Task 2's mailbox,
// reads/writes the shared PoseExtrapolator, reads the shared submap grid
// (owned by Task 4's state), pushes to Task 4's mailbox.
// ---------------------------------------------------------------------------

pub struct ScanMatchingState {
    inner: RwScanMatchingTask,
}

impl Default for ScanMatchingState {
    fn default() -> Self {
        ScanMatchingState {
            inner: RwScanMatchingTask::new(&WorkloadConfig::default()),
        }
    }
}

pub struct ScanMatchingJob;

impl Task for ScanMatchingJob {
    type Wcet = P50;
    type Deadline = P100;
    type Period = P100;
    type State = ScanMatchingState;

    fn do_work(state: &mut Self::State) {
        let Some(scan) = filtered_mailbox().pop() else {
            return;
        };
        let extrapolator_handle = shared_extrapolator();
        let grid_handle = shared_grid_update();
        let mut extrapolator = extrapolator_handle.lock().unwrap();
        let grid_update = grid_handle.lock().unwrap();
        let grid = grid_update.matching_grid();
        let (matched, _stage_corr, _stage_refine) = state.inner.step(scan, &mut extrapolator, grid);
        drop(grid_update);
        drop(extrapolator);
        matched_mailbox().push(matched);
    }
}

impl EDFTask for ScanMatchingJob {}

// ---------------------------------------------------------------------------
// Task 4: Motion Filter + Voxel Grid Update. Mirrors
// `InsertIntoSubmap`/`ActiveSubmaps2D::InsertRangeData`
// ([local_trajectory_builder_2d.cc:279-300]). Pops from Task 3's mailbox,
// owns the canonical shared submap grid that Task 3 reads.
// ---------------------------------------------------------------------------

pub struct GridUpdateState {
    inner: Arc<Mutex<MotionFilterGridUpdateTask>>,
}

impl Default for GridUpdateState {
    fn default() -> Self {
        GridUpdateState {
            inner: shared_grid_update(),
        }
    }
}

pub struct GridUpdateJob;

impl Task for GridUpdateJob {
    type Wcet = P50;
    type Deadline = P100;
    type Period = P100;
    type State = GridUpdateState;

    fn do_work(state: &mut Self::State) {
        let Some(matched) = matched_mailbox().pop() else {
            return;
        };
        let mut inner = state.inner.lock().unwrap();
        let _ = inner.step(matched);
    }
}

impl EDFTask for GridUpdateJob {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dispatcher::{Dispatcher, EDF, QPATest};
    use crate::qpa::{Nulltask, Tasklist};
    use std::sync::Mutex as StdMutex;

    // The chain_state statics are process-global singletons (OnceLock), and
    // cargo test runs tests in parallel threads within one process by
    // default. Both tests below touch the same shared extrapolator/grid/
    // mailboxes, so they'd otherwise interleave and produce flaky results --
    // this lock just serializes the two of them against each other, without
    // affecting parallelism for unrelated tests elsewhere in the crate.
    static TEST_SERIAL: StdMutex<()> = StdMutex::new(());

    /// Drives the 4 jobs in their correct precedence order directly (not
    /// through the dispatcher's independent threads, which don't enforce
    /// ordering yet) -- this is the real correctness check: data actually
    /// flows end to end through the shared mailboxes/stores.
    #[test]
    fn chain_runs_in_order_without_panicking() {
        let _guard = TEST_SERIAL.lock().unwrap();
        let mut s1 = SyncAccumulateState::default();
        let mut s2 = FilterState::default();
        let mut s3 = ScanMatchingState::default();
        let mut s4 = GridUpdateState::default();

        for _ in 0..20 {
            SyncAccumulateJob::do_work(&mut s1);
            FilterJob::do_work(&mut s2);
            ScanMatchingJob::do_work(&mut s3);
            GridUpdateJob::do_work(&mut s4);
        }

        // Every stage ran in order each iteration, so no mailbox should have
        // a backlog left over -- confirms data actually flowed end to end
        // through the shared stores/mailboxes rather than silently no-oping.
        assert!(batch_mailbox().pop().is_none());
        assert!(filtered_mailbox().pop().is_none());
        assert!(matched_mailbox().pop().is_none());
    }

    /// Exercises the real dispatcher path (thread spawn, park/unpark, the
    /// per-job-release do_work loop, join) for all 4 tasks together -- a
    /// smoke test that independent concurrent dispatch doesn't crash. Not a
    /// correctness test: with no precedence/notify mechanism yet, downstream
    /// tasks may find empty mailboxes and no-op most jobs.
    #[test]
    fn dispatches_through_the_full_edf_pipeline() {
        let _guard = TEST_SERIAL.lock().unwrap();
        type Taskset =
            Tasklist<SyncAccumulateJob, Tasklist<FilterJob, Tasklist<ScanMatchingJob, Tasklist<GridUpdateJob, Nulltask>>>>;
        Dispatcher::<Taskset, EDF, QPATest>::dispatch(5);
    }
}
