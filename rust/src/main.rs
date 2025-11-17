use std::marker::PhantomData;

// Task list
#[allow(non_upper_case_globals)]
#[allow(dead_code)] // deadline and period are unused for now
trait TaskTrait {
    const wcet: u32;
    const deadline: u32;
    const period: u32;
}

struct NullTask;

struct Tasklist<T, U>(PhantomData<T>, PhantomData<U>);

// Total WCET
#[allow(non_upper_case_globals)]
trait TotalWcet {
    const result: u32;
}

impl<T: TaskTrait, U: TotalWcet> TotalWcet for Tasklist<T, U> {
    const result: u32 = T::wcet + U::result;
}

impl TotalWcet for NullTask {
    const result: u32 = 0;
}

// Total utilization
#[allow(non_upper_case_globals)]
trait TotalUtil {
    const result: f64;
}

impl<T: TaskTrait, U: TotalUtil> TotalUtil for Tasklist<T, U> {
    const result: f64 = (T::wcet as f64) / (T::period as f64) + U::result;
}

impl TotalUtil for NullTask {
    const result: f64 = 0.0;
}

// Lastar
#[allow(non_upper_case_globals)]
trait MaxDelta {
    const result: i32;
}

trait Equality {}

// #[derive(Debug)]
struct EQ;

impl EQ {
    const fn to_string() -> &'static str {
        "Equal"
    }
}

// #[derive(Debug)]
struct LT;

impl LT {
    const fn to_string() ->&'static str {
        "Less Than"
    }
}

// #[derive(Debug)]
struct GT;

impl GT {
    const fn to_string() -> &'static str {
        "Greater Than"
    }
}

impl Equality for EQ {}
impl Equality for LT {}
impl Equality for GT {}

// Representations:
// Zero is 0.
// Succ<Zero> is 1, Succ<Succ<Zero>> is 2, and so on.
// Pred<Zero> is -1, Pred<Pred<Zero>>  is -2, and so on.
// For simplicity, each nunber has a unique type representation.
// For example, 1 is represented by Succ<Zero> and not Pred<Succ<Succ<Zero>>>.

// Relationships:
// Zero < Succ<P>, where P is a type representing 0 or a positive number.
// Succ<A> <= Succ<B> if A <= B.
// Pred<N> < Zero, where N is a type representing 0 or a negative number.
// Pred<A> <= Pred<B> if A <= B.

struct Zero;
struct Succ<P>(P);
struct Pred<N>(N);

trait Cmp<A> {
    type Output: Equality;
}

impl Cmp<Zero> for Zero {
    type Output = EQ;
}

impl<P> Cmp<Succ<P>> for Zero {
    type Output = LT;
}

impl<P> Cmp<Zero> for Succ<P> {
    type Output = GT;
}

impl<A: Cmp<B>, B> Cmp<Succ<B>> for Succ<A> {
    type Output = <A as Cmp<B>>::Output;
}

impl<N> Cmp<Pred<N>> for Zero {
    type Output = GT;
}

// impl<T: TaskTrait, U: MaxDelta> MaxDelta for Tasklist<T, U> {
//     // TODO: compare these 2 operands
//     const result: i32 = (T::deadline as i32 - T::period as i32), U::result;
// }

fn main() {
    struct Task1;

    impl TaskTrait for Task1 {
        const wcet: u32 = 10;
        const deadline: u32 = 15;
        const period: u32 = 20;
    }

    struct Task2;

    impl TaskTrait for Task2 {
        const wcet: u32 = 4;
        const deadline: u32 = 6;
        const period: u32 = 10;
    }

    type Taskset = Tasklist<Task1, Tasklist<Task2, NullTask>>;

    println!("Total WCET = {}", <Taskset as TotalWcet>::result);
    println!("Total Utilization = {}", <Taskset as TotalUtil>::result);

    // Test comparing numbers
    println!("Succ<Zero> vs Zero: {:?}", <Succ<Zero> as Cmp<Zero>>::Output::to_string());
}
