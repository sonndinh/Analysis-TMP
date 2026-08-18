use crate::qpa::{QPA, Task, Tasklist, Nulltask};
use typenum::Integer;
use std::marker::PhantomData;


struct TaskParams
{
    wcet: u32,
    deadline: u32,
    period: u32,
}

trait EDFTask: Task
{
    fn setup_task(params: TaskParams)
    {
        #[cfg(target_os = "linux")]
        {
            std::thread::spawn(move || {
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

                // Linux doesn't wrap sched_setattr(2) in libc, so issue it directly.
                // A pid of 0 targets the calling thread, i.e. this one.
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
    fn setup();
}

impl EDFTasklist for Nulltask
{
    fn setup() {}
}


impl<T: Task + EDFTask, U: EDFTasklist> EDFTasklist for Tasklist<T, U>
{
    fn setup()
    {
        let params = TaskParams {
            wcet: <<T as Task>::Wcet as Integer>::to_i32() as u32,
            deadline: <T::Deadline as Integer>::to_i32() as u32,
            period: <T::Period as Integer>::to_i32() as u32,
        };

        // Set up the head task, and
        <T as EDFTask>::setup_task(params);

        // Recursively launch the rest
        U::setup();
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
struct EDF;

impl<T: Task + EDFTask, U: EDFTasklist> DispatcherGenerator<EDF> for Tasklist<T, U>
{
    fn generate_dispatcher()
    {
        // Create a thread for each task and register them with the OS scheduler.
        <Tasklist<T, U> as EDFTasklist>::setup();

        // TODO: Start the tasks
        // schedule()
    }
}

trait Feasibility<Analysis>
{
    type Result;
}

// Tags to differentiate different schedulability analysis algorithms.
// Based on the tag, an implementation of the specifed algorithm is used.
struct QPATest;

// Caller is expected to, first, check the schedulability result using the specified analysis algorithm.
impl<T, U> Feasibility<QPATest> for Tasklist<T, U>
where
    (T, U): QPA
{
    type Result = <(T, U) as QPA>::Output;
}

// Generic on the task set, the scheduling policy, and the schedulability analysis for
// the specified task set under the specified scheduling policy.
struct Dispatcher<Taskset, Policy, Analysis>(PhantomData<Taskset>, PhantomData<Policy>, PhantomData<Analysis>);

// Then, generate the dispatcher for the given task set under the given scheduling policy.
// Example with EDF policy, QPA analysis, ExampleTaskset:
// Dispatcher::<ExampleTaskset, EDF, QPA>::Result has the feasibility result of the task set.
// Dispatcher::<ExampleTaskset, EDF, QPA>::dispatch() generates the dispatcher and dispatches the tasks.
impl<Taskset: DispatcherGenerator<Policy>, Policy, Analysis> Dispatcher<Taskset, Policy, Analysis>
{
    fn dispatch()
    {
        <Taskset as DispatcherGenerator<Policy>>::generate_dispatcher();
    }
}
