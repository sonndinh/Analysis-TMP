use std::marker::PhantomData;

use crate::qpa::{Task, Tasklist, Nulltask};

// Each scheduling policy, such as RM or EDF, implements this trait to generate
// a dispatcher for the given task set under the given scheduling policy.
trait DispatcherGenerator<Policy>
{
    fn generate_dispatcher();
}

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

// Tags for different scheduling policies, so a dispatcher can be generated for a given task set
// under each scheduling policy.
struct EDF;

impl<T: Task + EDFTask, U: EDFTasklist> DispatcherGenerator<EDF> for Tasklist<T, U>
{
    fn generate_dispatcher()
    {
        // Create a thread for each task and register them with the OS scheduler.
        <Tasklist<T, U> as EDFTasklist>::setup();

        // Start the tasks
        // schedule()
    }
}

trait Feasibility
{
    type Result;
}

// Generic on the task set, the scheduling policy, and the schedulability analysis for
// the specified task set under the specified scheduling policy.
struct Dispatcher<Taskset, Policy, Analysis>(PhantomData<Taskset>, PhantomData<Policy>, PhantomData<Analysis>);

// Caller is expected to, first, check the schedulability result using the specified analysis algorithm.
impl<Taskset, Policy, Analysis> Feasibility for Dispatcher<Taskset, Policy, Analysis>
{
    // TODO: Delegate to the given feasibility test
    type Result = bool;
}

// Then, generate the dispatcher for the given task set under the given scheduling policy.
// Example with EDF policy, QPA analysis, ExampleTaskset:
// Dispatcher::<ExampleTaskset, EDF, QPA>::Result has the feasibility result of the task set.
// Dispatcher::<ExampleTaskset, EDF, QPA>::dispatch() generates the dispatcher and dispatches the tasks.
impl<Taskset: DispatcherGenerator<Policy>, Policy, Analysis> Dispatcher<Taskset, Policy, Analysis>
// where
    // Policy: DispatcherGenerator
{
    fn dispatch()
    {
        // TODO: generate dispatcher for the given task set under the given scheduling policy.
        // <Policy as DispatcherGenerator>::generate_dispatcher::<Tasklist>();
        <Taskset as DispatcherGenerator<Policy>>::generate_dispatcher();
    }
}
