//! Pragmatic process-global sharing for the decomposed Local-SLAM task chain.
//!
//! `Task::State` is constructed via `Default::default()` with no parameters,
//! and `Task::do_work` takes no input and returns nothing -- so there's no
//! built-in way for the dispatcher to hand one task's state a reference to
//! another task's state, or to pass one task's output to the next task's
//! input. These statics are the simplest mechanism that fits those
//! constraints: every task that needs a given piece of cross-task state or
//! data clones the same handle out of one of these lazily-initialized
//! globals.
//!
//! Two kinds of sharing:
//! - **Stores**: `PoseExtrapolator` and the submap grid (wrapped inside
//!   `MotionFilterGridUpdateTask`), each written by one stage and read by an
//!   earlier-looking stage on a later release -- the two real feedback edges
//!   identified in the earlier task-decomposition analysis.
//! - **Mailboxes**: a small bounded queue per handoff edge between
//!   consecutive stages (accumulated batch -> filtered scan -> matched
//!   pose), standing in for the data each stage's output hands to the next
//!   stage's input.
//!
//! What this does NOT do: enforce precedence between tasks. If a downstream
//! task's mailbox is empty when its `do_work` runs, that call is just a
//! no-op for this job -- there is no notify/wait mechanism yet forcing, say,
//! Scan Matching to run only after Filtering has produced this release's
//! data. That runtime-ordering piece is separate, still-open work, distinct
//! from the deadline/offset assignment the user is deferring.

use nalgebra::Isometry2;
use rt_slam_workload::config::WorkloadConfig;
use rt_slam_workload::extrapolator::PoseExtrapolator;
use rt_slam_workload::workload::{AccumulatedBatch, FilteredScan, MatchedPose, MotionFilterGridUpdateTask};
use std::collections::VecDeque;
use std::sync::{Arc, Mutex, OnceLock};

pub fn shared_extrapolator() -> Arc<Mutex<PoseExtrapolator>> {
    static STORE: OnceLock<Arc<Mutex<PoseExtrapolator>>> = OnceLock::new();
    STORE
        .get_or_init(|| {
            let config = WorkloadConfig::default();
            let mut extrapolator = PoseExtrapolator::new(config.pose_queue_duration);
            extrapolator.add_pose(0.0, Isometry2::identity());
            Arc::new(Mutex::new(extrapolator))
        })
        .clone()
}

pub fn shared_grid_update() -> Arc<Mutex<MotionFilterGridUpdateTask>> {
    static STORE: OnceLock<Arc<Mutex<MotionFilterGridUpdateTask>>> = OnceLock::new();
    STORE
        .get_or_init(|| Arc::new(Mutex::new(MotionFilterGridUpdateTask::new(&WorkloadConfig::default()))))
        .clone()
}

/// A small mailbox queue between two consecutive tasks. Defensively capped
/// so a stalled/missing consumer doesn't grow this without bound.
pub struct Mailbox<T> {
    queue: Mutex<VecDeque<T>>,
}

const MAILBOX_CAP: usize = 64;

impl<T> Mailbox<T> {
    fn new() -> Self {
        Mailbox {
            queue: Mutex::new(VecDeque::new()),
        }
    }

    pub fn push(&self, item: T) {
        let mut q = self.queue.lock().unwrap();
        q.push_back(item);
        while q.len() > MAILBOX_CAP {
            q.pop_front();
        }
    }

    pub fn pop(&self) -> Option<T> {
        self.queue.lock().unwrap().pop_front()
    }
}

pub fn batch_mailbox() -> Arc<Mailbox<AccumulatedBatch>> {
    static STORE: OnceLock<Arc<Mailbox<AccumulatedBatch>>> = OnceLock::new();
    STORE.get_or_init(|| Arc::new(Mailbox::new())).clone()
}

pub fn filtered_mailbox() -> Arc<Mailbox<FilteredScan>> {
    static STORE: OnceLock<Arc<Mailbox<FilteredScan>>> = OnceLock::new();
    STORE.get_or_init(|| Arc::new(Mailbox::new())).clone()
}

pub fn matched_mailbox() -> Arc<Mailbox<MatchedPose>> {
    static STORE: OnceLock<Arc<Mailbox<MatchedPose>>> = OnceLock::new();
    STORE.get_or_init(|| Arc::new(Mailbox::new())).clone()
}
