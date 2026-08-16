use std::marker::PhantomData;
use crate::qpa::{QPA, Task, Tasklist, Nulltask, Lb, TotalWcet};

trait EDFTask
{
    fn setup_task()
    {
        // TODO: set up a thread for the task.
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
        // Set up the head task, and
        <T as EDFTask>::setup_task();

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
