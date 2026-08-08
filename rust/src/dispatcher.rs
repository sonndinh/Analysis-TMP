use std::marker::PhantomData;

use crate::qpa::Tasklist;

// Each scheduling policy, such as RM or EDF, implements this trait to generate
// a dispatcher for the given task set under the given scheduling policy.
trait DispatcherGenerator
{
    fn generate_dispatcher<Tasklist>();
}

struct EDF;

impl EDF
{
    fn create_edf_task<T>()
    {
        // TODO: set up task T
    }
}

impl DispatcherGenerator for EDF
{
    fn generate_dispatcher<Tasklist>()
    {
        // TODO: generate a thread for each task and register them with the OS scheduler.
        EDF::create_edf_task::<Tasklist::T>();

        // Start the tasks
        // schedule()
    }
}

trait Feasibility
{
    type Result;
}

// Template on the task set, the scheduling policy, and the schedulability analysis for
// the specified task set under the specified scheduling policy.
struct Dispatcher<Tasklist, Policy, Analysis>(PhantomData<Tasklist>, PhantomData<Policy>, PhantomData<Analysis>);

// Caller is expected to, first, check the schedulability result using the specified analysis algorithm.
impl<Tasklist, Policy, Analysis> Feasibility for Dispatcher<Tasklist, Policy, Analysis>
{
    // TODO: Delegate to the given feasibility test
    type Result = bool;
}

// Then, generate the dispatcher for the given task set under the given scheduling policy.
// Example with EDF policy, QPA analysis, ExampleTaskset:
// Dispatcher::<ExampleTaskset, EDF, QPA>::Result has the feasibility result of the task set.
// Dispatcher::<ExampleTaskset, EDF, QPA>::dispatch() generates the dispatcher and dispatches the tasks.
impl<Tasklist, Policy, Analysis> Dispatcher<Tasklist, Policy, Analysis>
where
    Policy: DispatcherGenerator
{
    fn dispatch()
    {
        // TODO: generate dispatcher for the given task set under the given scheduling policy.
        <Policy as DispatcherGenerator>::generate_dispatcher::<Tasklist>();
    }
}
