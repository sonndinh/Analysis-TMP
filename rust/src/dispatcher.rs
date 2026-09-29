use crate::qpa::{QPA, Task, Tasklist, Nulltask};
use typenum::Integer;
use std::{marker::PhantomData, thread::JoinHandle};

struct TaskParams
{
    wcet: u32,
    deadline: u32,
    period: u32,
    // SCHED_DEADLINE configures a runtime/deadline/period budget but does not
    // release jobs on its own -- the thread must explicitly signal "done with
    // this job" (via sched_yield) to get parked until the next period. So the
    // dispatcher needs to know how many job releases to run before the thread
    // should exit and be joined.
    num_jobs: u32,
}

pub trait EDFTask: Task
{
    fn setup_task(params: TaskParams) -> JoinHandle<()>
    {
        #[cfg(target_os = "linux")]
        {
            return std::thread::spawn(move || {
                println!("Task thread started with params: wcet={}, deadline={}, period={}, num_jobs={}", params.wcet, params.deadline, params.period, params.num_jobs);
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

                // Wait here until the dispatcher unparks this thread to start the task.
                std::thread::park();

                // Persistent per-task state, constructed once and reused across
                // every job release (see `Task::State`).
                let mut state = <Self as Task>::State::default();
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
            // No SCHED_DEADLINE off Linux, so this doesn't enforce real-time
            // budgets -- but it still runs the same do_work-per-job-release
            // loop, so the task's actual work is testable cross-platform.
            return std::thread::spawn(move || {
                println!(
                    "Task thread started (non-Linux fallback, no SCHED_DEADLINE) with params: wcet={}, deadline={}, period={}, num_jobs={}",
                    params.wcet, params.deadline, params.period, params.num_jobs
                );

                std::thread::park();

                let mut state = <Self as Task>::State::default();
                for _ in 0..params.num_jobs {
                    Self::do_work(&mut state);
                }
            });
        }
    }
}

trait EDFTasklist
{
    fn setup(num_jobs: u32) -> Vec<JoinHandle<()>>;
}

impl EDFTasklist for Nulltask
{
    fn setup(_num_jobs: u32) -> Vec<JoinHandle<()>> {
        vec![]
    }
}


impl<T: Task + EDFTask, U: EDFTasklist> EDFTasklist for Tasklist<T, U>
{
    fn setup(num_jobs: u32) -> Vec<JoinHandle<()>>
    {
        let params = TaskParams {
            wcet: <<T as Task>::Wcet as Integer>::to_i32() as u32,
            deadline: <T::Deadline as Integer>::to_i32() as u32,
            period: <T::Period as Integer>::to_i32() as u32,
            num_jobs,
        };

        // Set up the head task, and
        let handle =<T as EDFTask>::setup_task(params);

        // Recursively launch the rest
        let mut remaining_handles = U::setup(num_jobs);
        remaining_handles.push(handle);
        remaining_handles
    }
}

// Each scheduling policy, such as RM or EDF, implements this trait to generate
// a dispatcher for the given task set under the given scheduling policy.
trait DispatcherGenerator<Policy>
{
    fn generate_dispatcher(num_jobs: u32);
}

// Tags for different scheduling policies, so a dispatcher can be generated for a given task set
// under each scheduling policy.
pub struct EDF;

impl<T: Task + EDFTask, U: EDFTasklist> DispatcherGenerator<EDF> for Tasklist<T, U>
{
    fn generate_dispatcher(num_jobs: u32)
    {
        // Create a thread for each task and register them with the OS scheduler.
        let handles = <Tasklist<T, U> as EDFTasklist>::setup(num_jobs);

        // TODO: Start the tasks
        // schedule()

        for handle in &handles {
            handle.thread().unpark();
        }

        for handle in handles {
            handle.join().unwrap();
        }
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

// Generic on the task set, the scheduling policy, and the schedulability analysis for
// the specified task set under the specified scheduling policy.
pub struct Dispatcher<Taskset, Policy, Analysis>(PhantomData<Taskset>, PhantomData<Policy>, PhantomData<Analysis>);

// Then, generate the dispatcher for the given task set under the given scheduling policy.
// Example with EDF policy, QPA analysis, ExampleTaskset:
// Dispatcher::<ExampleTaskset, EDF, QPA>::Result has the feasibility result of the task set.
// Dispatcher::<ExampleTaskset, EDF, QPA>::dispatch() generates the dispatcher and dispatches the tasks.
impl<Taskset: DispatcherGenerator<Policy>, Policy, Analysis> Dispatcher<Taskset, Policy, Analysis>
{
    pub fn dispatch(num_jobs: u32)
    {
        println!("Dispatcher::dispatch()...");
        <Taskset as DispatcherGenerator<Policy>>::generate_dispatcher(num_jobs);
    }
}
