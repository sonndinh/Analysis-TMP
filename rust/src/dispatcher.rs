use crate::qpa::{QPA, Task, Tasklist, Nulltask};
use typenum::Integer;
use std::marker::PhantomData;
use std::sync::{Arc, OnceLock};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

// Deadline decrement (ms) per position from the end of a `Chain`.
const STAGGER_STEP_MS: u32 = 1;

// Gap between fixing the common start instant and the earliest release, so
// every task thread has reached its start gate before anything is released.
const START_LEAD: Duration = Duration::from_millis(10);

#[derive(Debug, PartialEq, Eq)]
struct TaskParams
{
    wcet: u32,
    deadline: u32,
    period: u32,
    offset: u32,
    // SCHED_DEADLINE configures a runtime/deadline/period budget but does not
    // release jobs on its own -- the thread must explicitly signal "done with
    // this job" (via sched_yield) to get parked until the next period. So the
    // dispatcher needs to know how many job releases to run before the thread
    // should exit and be joined.
    num_jobs: u32,
}

/// Builds a task's runtime parameters. `steps_from_end` is the number of tasks
/// after this one in its `Chain` (0 when not chained): its deadline is
/// tightened by that many `STAGGER_STEP_MS` so EDF picks chain predecessors
/// first among jobs released together. `num_jobs` counts the releases
/// `offset + k * period` that fall before `duration_ms`.
fn derive_params(wcet: u32, deadline: u32, period: u32, offset: u32, steps_from_end: u32, duration_ms: u32) -> TaskParams
{
    let decrement = steps_from_end * STAGGER_STEP_MS;
    assert!(
        deadline >= wcet + decrement,
        "staggering deadline {deadline} ms by {decrement} ms for chain precedence drops it below WCET {wcet} ms"
    );
    let num_jobs = if duration_ms > offset { (duration_ms - offset).div_ceil(period) } else { 0 };
    TaskParams { wcet, deadline: deadline - decrement, period, offset, num_jobs }
}

fn wait_for_start(start: &OnceLock<Instant>) -> Instant
{
    // Loop because park() may wake spuriously.
    loop {
        if let Some(t) = start.get() {
            return *t;
        }
        std::thread::park();
    }
}

fn sleep_until(target: Instant)
{
    let now = Instant::now();
    if target > now {
        std::thread::sleep(target - now);
    }
}

pub trait EDFTask: Task
{
    fn setup_task(params: TaskParams, start: Arc<OnceLock<Instant>>) -> JoinHandle<()>
    {
        #[cfg(target_os = "linux")]
        {
            return std::thread::spawn(move || {
                println!(
                    "Task thread started with params: wcet={}, deadline={}, period={}, offset={}, num_jobs={}",
                    params.wcet, params.deadline, params.period, params.offset, params.num_jobs
                );
                // sched_attr's runtime/deadline/period are in nanoseconds; TaskParams is in milliseconds.
                let attr = libc::sched_attr {
                    size: std::mem::size_of::<libc::sched_attr>() as u32,
                    sched_policy: libc::SCHED_DEADLINE as u32,
                    sched_flags: 0,
                    sched_nice: 0,
                    sched_priority: 0,
                    sched_runtime: params.wcet as u64 * 1_000_000,
                    sched_deadline: params.deadline as u64 * 1_000_000,
                    sched_period: params.period as u64 * 1_000_000,
                };

                let ret = unsafe {
                    libc::syscall(libc::SYS_sched_setattr, 0, &attr as *const libc::sched_attr, 0u32)
                };
                if ret != 0 {
                    panic!("sched_setattr failed: {}", std::io::Error::last_os_error());
                }

                // Persistent per-task state, constructed once (before the
                // first release, so its cost isn't charged to job 0) and
                // reused across every job release (see `Task::State`).
                let mut state = <Self as Task>::State::default();

                // Waking from this sleep starts a fresh SCHED_DEADLINE period,
                // anchoring this task's period grid at start + offset;
                // sched_yield keeps subsequent releases on that grid.
                let start = wait_for_start(&start);
                sleep_until(start + Duration::from_millis(params.offset as u64));

                for _ in 0..params.num_jobs {
                    Self::do_work(&mut state);
                    // Signal completion of this job to SCHED_DEADLINE; the
                    // kernel blocks this thread until the start of the next
                    // period.
                    let yield_ret = unsafe { libc::sched_yield() };
                    if yield_ret != 0 {
                        panic!("sched_yield failed: {}", std::io::Error::last_os_error());
                    }
                }
            });
        }

        #[cfg(not(target_os = "linux"))]
        {
            // No SCHED_DEADLINE off Linux: releases are emulated by sleeping
            // until each release instant, but there is no EDF/budget
            // enforcement -- so chain order among jobs released together is
            // not guaranteed here.
            return std::thread::spawn(move || {
                println!(
                    "Task thread started (non-Linux fallback, no SCHED_DEADLINE) with params: wcet={}, deadline={}, period={}, offset={}, num_jobs={}",
                    params.wcet, params.deadline, params.period, params.offset, params.num_jobs
                );

                let mut state = <Self as Task>::State::default();

                let start = wait_for_start(&start);
                let first_release = start + Duration::from_millis(params.offset as u64);
                let period = Duration::from_millis(params.period as u64);
                for k in 0..params.num_jobs {
                    sleep_until(first_release + period * k);
                    Self::do_work(&mut state);
                }
            });
        }
    }
}

pub trait ListLen
{
    const LEN: u32;
}

impl ListLen for Nulltask
{
    const LEN: u32 = 0;
}

impl<T, U: ListLen> ListLen for Tasklist<T, U>
{
    const LEN: u32 = 1 + U::LEN;
}

trait EDFTasklist
{
    fn setup(duration_ms: u32, chained: bool, start: &Arc<OnceLock<Instant>>) -> Vec<JoinHandle<()>>;
}

impl EDFTasklist for Nulltask
{
    fn setup(_duration_ms: u32, _chained: bool, _start: &Arc<OnceLock<Instant>>) -> Vec<JoinHandle<()>> {
        vec![]
    }
}

impl<T: Task + EDFTask, U: EDFTasklist + ListLen> EDFTasklist for Tasklist<T, U>
{
    fn setup(duration_ms: u32, chained: bool, start: &Arc<OnceLock<Instant>>) -> Vec<JoinHandle<()>>
    {
        let params = derive_params(
            <<T as Task>::Wcet as Integer>::to_i32() as u32,
            <T::Deadline as Integer>::to_i32() as u32,
            <T::Period as Integer>::to_i32() as u32,
            T::OFFSET,
            if chained { U::LEN } else { 0 },
            duration_ms,
        );

        // Set up the head task, and
        let handle = <T as EDFTask>::setup_task(params, Arc::clone(start));

        // Recursively launch the rest
        let mut remaining_handles = U::setup(duration_ms, chained, start);
        remaining_handles.push(handle);
        remaining_handles
    }
}

/// Declares that the tasks of a `Tasklist` form a precedence chain, in list
/// order (head first). The dispatcher enforces the order through EDF alone by
/// staggering deadlines: the last task keeps its declared deadline and each
/// predecessor gets `STAGGER_STEP_MS` less per position. The QPA analysis
/// still sees the declared (unstaggered) deadlines.
pub struct Chain<L>(PhantomData<L>);

fn launch<L: EDFTasklist>(duration_ms: u32, chained: bool)
{
    let start = Arc::new(OnceLock::new());

    // Create a thread for each task and register them with the OS scheduler.
    let handles = L::setup(duration_ms, chained, &start);

    start
        .set(Instant::now() + START_LEAD)
        .expect("dispatch start instant is set exactly once");
    for handle in &handles {
        handle.thread().unpark();
    }

    for handle in handles {
        handle.join().unwrap();
    }
}

// Each scheduling policy, such as RM or EDF, implements this trait to generate
// a dispatcher for the given task set under the given scheduling policy.
trait DispatcherGenerator<Policy>
{
    fn generate_dispatcher(duration_ms: u32);
}

// Tags for different scheduling policies, so a dispatcher can be generated for a given task set
// under each scheduling policy.
pub struct EDF;

impl<T: Task + EDFTask, U: EDFTasklist + ListLen> DispatcherGenerator<EDF> for Tasklist<T, U>
{
    fn generate_dispatcher(duration_ms: u32)
    {
        launch::<Tasklist<T, U>>(duration_ms, false);
    }
}

impl<L: EDFTasklist> DispatcherGenerator<EDF> for Chain<L>
{
    fn generate_dispatcher(duration_ms: u32)
    {
        launch::<L>(duration_ms, true);
    }
}

trait Feasibility<Analysis>
{
    type Result;
}

// Tags to differentiate different schedulability analysis algorithms.
// Based on the tag, an implementation of the specifed algorithm is used.
pub struct QPATest;

// Caller is expected to, first, check the schedulability result using the specified analysis algorithm.
impl<T, U> Feasibility<QPATest> for Tasklist<T, U>
where
    (T, U): QPA
{
    type Result = <(T, U) as QPA>::Output;
}

impl<L: Feasibility<QPATest>> Feasibility<QPATest> for Chain<L>
{
    type Result = <L as Feasibility<QPATest>>::Result;
}

// Generic on the task set, the scheduling policy, and the schedulability analysis for
// the specified task set under the specified scheduling policy.
pub struct Dispatcher<Taskset, Policy, Analysis>(PhantomData<Taskset>, PhantomData<Policy>, PhantomData<Analysis>);

// Then, generate the dispatcher for the given task set under the given scheduling policy.
// Example with EDF policy, QPA analysis, ExampleTaskset:
// Dispatcher::<ExampleTaskset, EDF, QPA>::Result has the feasibility result of the task set.
// Dispatcher::<ExampleTaskset, EDF, QPA>::dispatch(duration_ms) generates the dispatcher and
// runs every task's releases that fall within `duration_ms` of the dispatch start.
impl<Taskset: DispatcherGenerator<Policy>, Policy, Analysis> Dispatcher<Taskset, Policy, Analysis>
{
    pub fn dispatch(duration_ms: u32)
    {
        println!("Dispatcher::dispatch()...");
        <Taskset as DispatcherGenerator<Policy>>::generate_dispatcher(duration_ms);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn job_count_covers_releases_before_duration() {
        assert_eq!(derive_params(1, 10, 10, 0, 0, 100).num_jobs, 10);
        assert_eq!(derive_params(1, 10, 10, 0, 0, 101).num_jobs, 11);
        // N=2, T1=10: Tasks 2-4 have period 20, offset 10 -> releases 10, 30, 50, 70, 90.
        assert_eq!(derive_params(1, 20, 20, 10, 0, 100).num_jobs, 5);
        assert_eq!(derive_params(1, 20, 20, 100, 0, 100).num_jobs, 0);
        assert_eq!(derive_params(1, 20, 20, 150, 0, 100).num_jobs, 0);
    }

    #[test]
    fn unchained_deadline_is_unchanged() {
        let p = derive_params(5, 100, 100, 0, 0, 1000);
        assert_eq!(p, TaskParams { wcet: 5, deadline: 100, period: 100, offset: 0, num_jobs: 10 });
    }

    #[test]
    fn four_task_chain_with_equal_periods_staggers_every_task() {
        // N=1: all four tasks share period/deadline 100.
        let deadlines: Vec<u32> = (0..4).rev().map(|steps| derive_params(5, 100, 100, 0, steps, 1000).deadline).collect();
        assert_eq!(deadlines, vec![97, 98, 99, 100]);
    }

    #[test]
    fn chain_with_shorter_head_period_keeps_head_first() {
        // N=5, T1=20: task 1 (steps=3) vs tasks 2-4 (period 100, offset 80).
        let head = derive_params(5, 20, 20, 0, 3, 1000);
        let tail: Vec<u32> = (0..3).rev().map(|steps| derive_params(5, 100, 100, 80, steps, 1000).deadline).collect();
        assert_eq!(head.deadline, 17);
        assert_eq!(tail, vec![98, 99, 100]);
        assert_eq!(head.num_jobs, 50);
        assert_eq!(derive_params(5, 100, 100, 80, 0, 1000).num_jobs, 10);
    }

    #[test]
    #[should_panic(expected = "drops it below WCET")]
    fn stagger_below_wcet_panics() {
        derive_params(99, 100, 100, 0, 2, 1000);
    }

    #[test]
    fn list_len_counts_tasks() {
        use crate::qpa::Nulltask;
        assert_eq!(<Nulltask as ListLen>::LEN, 0);
        assert_eq!(<Tasklist<Nulltask, Tasklist<Nulltask, Nulltask>> as ListLen>::LEN, 2);
    }

    // Exercises the emulated periodic releases of the non-Linux fallback.
    #[cfg(not(target_os = "linux"))]
    #[test]
    fn fallback_honors_offsets_and_per_task_periods() {
        use std::sync::Mutex;
        use typenum::{P1, P20, P40};

        static RELEASES: Mutex<Vec<(char, Instant)>> = Mutex::new(Vec::new());

        struct A;
        impl Task for A {
            type Wcet = P1;
            type Deadline = P20;
            type Period = P20;
            type State = ();
            fn do_work(_state: &mut ()) {
                RELEASES.lock().unwrap().push(('A', Instant::now()));
            }
        }
        impl EDFTask for A {}

        struct B;
        impl Task for B {
            type Wcet = P1;
            type Deadline = P40;
            type Period = P40;
            const OFFSET: u32 = 20;
            type State = ();
            fn do_work(_state: &mut ()) {
                RELEASES.lock().unwrap().push(('B', Instant::now()));
            }
        }
        impl EDFTask for B {}

        Dispatcher::<Tasklist<A, Tasklist<B, Nulltask>>, EDF, QPATest>::dispatch(100);

        let releases = RELEASES.lock().unwrap();
        let a: Vec<Instant> = releases.iter().filter(|r| r.0 == 'A').map(|r| r.1).collect();
        let b: Vec<Instant> = releases.iter().filter(|r| r.0 == 'B').map(|r| r.1).collect();
        assert_eq!(a.len(), 5);
        assert_eq!(b.len(), 2);
        // Sleeps only overshoot, so lower bounds are robust; allow 2 ms slack
        // for A's first release itself being late.
        let slack = Duration::from_millis(2);
        assert!(b[0] + slack >= a[0] + Duration::from_millis(20));
        assert!(b[1] + slack >= a[0] + Duration::from_millis(60));
        assert!(a[4] + slack >= a[0] + Duration::from_millis(80));
    }
}
