use crate::qpa::{QPA, Task, Tasklist, Nulltask};
use typenum::Integer;
use std::{marker::PhantomData, thread::JoinHandle};

struct TaskParams
{
    wcet: u32,
    deadline: u32,
    period: u32,
}

pub trait EDFTask: Task
{
    fn setup_task(params: TaskParams) -> JoinHandle<()>
    {
        #[cfg(target_os = "linux")]
        {
            return std::thread::spawn(move || {
                println!("Task thread started with params: wcet={}, deadline={}, period={}", params.wcet, params.deadline, params.period);
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

                Self::do_work();
            });
        }

        #[cfg(not(target_os = "linux"))]
        {
            let _ = params;
            unimplemented!("SCHED_DEADLINE setup is only supported on Linux");
        }
    }
}

trait EDFTasklist
{
    fn setup() -> Vec<JoinHandle<()>>;
}

impl EDFTasklist for Nulltask
{
    fn setup() -> Vec<JoinHandle<()>> {
        vec![]
    }
}


impl<T: Task + EDFTask, U: EDFTasklist> EDFTasklist for Tasklist<T, U>
{
    fn setup() -> Vec<JoinHandle<()>>
    {
        let params = TaskParams {
            wcet: <<T as Task>::Wcet as Integer>::to_i32() as u32,
            deadline: <T::Deadline as Integer>::to_i32() as u32,
            period: <T::Period as Integer>::to_i32() as u32,
        };

        // Set up the head task, and
        let handle =<T as EDFTask>::setup_task(params);

        // Recursively launch the rest
        let mut remaining_handles = U::setup();
        remaining_handles.push(handle);
        remaining_handles
    }
}

// Each scheduling policy, such as RM or EDF, implements this trait to generate
// a dispatcher for the given task set under the given scheduling policy.
trait DispatcherGenerator<Policy>
{
    fn generate_dispatcher();
}

// Tags for different scheduling policies, so a dispatcher can be generated for a given task set
// under each scheduling policy.
pub struct EDF;

impl<T: Task + EDFTask, U: EDFTasklist> DispatcherGenerator<EDF> for Tasklist<T, U>
{
    fn generate_dispatcher()
    {
        // Create a thread for each task and register them with the OS scheduler.
        let handles = <Tasklist<T, U> as EDFTasklist>::setup();

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
    pub fn dispatch()
    {
        println!("Dispatcher::dispatch()...");
        <Taskset as DispatcherGenerator<Policy>>::generate_dispatcher();
    }
}
