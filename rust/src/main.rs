use std::marker::PhantomData;

// Task list
trait TaskTrait {
    const WCET: u32;
    const DEADLINE: u32;
    const PERIOD: u32;
}

struct NullTask;

struct Tasklist<T, U>(PhantomData<T>, PhantomData<U>);

// Total WCET
trait TotalWcet {
    const RESULT: u32;
}

impl<T: TaskTrait, U: TotalWcet> TotalWcet for Tasklist<T, U> {
    const RESULT: u32 = T::WCET + U::RESULT;
}

impl TotalWcet for NullTask {
    const RESULT: u32 = 0;
}

// Total utilization
trait TotalUtil {
    const RESULT: f64;
}

impl<T: TaskTrait, U: TotalUtil> TotalUtil for Tasklist<T, U> {
    const RESULT: f64 = (T::WCET as f64) / (T::PERIOD as f64) + U::RESULT;
}

impl TotalUtil for NullTask {
    const RESULT: f64 = 0.0;
}

// Lastar
trait MaxDelta {
    const RESULT: i32;
}

impl<T: TaskTrait, U: MaxDelta> Tasklist<T, U> {
    const fn max_delta() -> i32 {
        let first = T::DEADLINE as i32 - T::PERIOD as i32;
        let second = U::RESULT;
        if first > second {
            first
        } else {
            second
        }
    }
}

impl<T: TaskTrait, U: MaxDelta> MaxDelta for Tasklist<T, U> {
    const RESULT: i32 = Self::max_delta();
}

impl MaxDelta for NullTask {
    const RESULT: i32 = i32::MIN;
}

trait LastarHelper {
    const RESULT: f64;
}

impl<T: TaskTrait, U: LastarHelper> Tasklist<T, U> {
    const fn lastar_helper() -> f64 {
        let my_util = (T::WCET as f64) / (T::PERIOD as f64);
        let my_value = (T::PERIOD as f64 - T::DEADLINE as f64) * my_util;
        my_value + U::RESULT
    }
}

impl<T: TaskTrait, U: LastarHelper> LastarHelper for Tasklist<T, U> {
    const RESULT: f64 = Self::lastar_helper();
}

impl LastarHelper for NullTask {
    const RESULT: f64 = 0.0;
}

trait Lastar {
    const RESULT: i64;
}

impl<T: TaskTrait, U: MaxDelta + LastarHelper + TotalUtil> Tasklist<T, U> {
    const fn lastar() -> i64 {
        let total_util = <Self as TotalUtil>::RESULT;
        if total_util >= 1.0 {
            return i64::MAX
        }

        let max_delta = <Self as MaxDelta>::RESULT as i64;
        let lastar_helper = (<Self as LastarHelper>::RESULT / (1.0 - total_util)) as i64;
        if max_delta > lastar_helper {
            max_delta
        } else {
            lastar_helper
        }
    }
}

impl<T: TaskTrait, U: MaxDelta + LastarHelper + TotalUtil> Lastar for Tasklist<T, U> {
    const RESULT: i64 = Self::lastar();
}

// Lb - Compute busy period lower bound iteratively
// LbSumHelper computes the sum: sum over all tasks of {ceiling(prev_value / period) * wcet}
// Uses const generics to encode prev_value as a type parameter
struct LbSumHelper<T, const PREV_VALUE: u64>(PhantomData<T>);

trait LbSumValue<const PREV_VALUE: u64> {
    const RESULT: u64;
}

impl<T: TaskTrait, U, const PREV_VALUE: u64> LbSumValue<PREV_VALUE>
    for LbSumHelper<Tasklist<T, U>, PREV_VALUE>
where
    LbSumHelper<U, PREV_VALUE>: LbSumValue<PREV_VALUE>
{
    const RESULT: u64 = {
        // Compute ceiling(PREV_VALUE / T::PERIOD) * T::WCET
        let ceiling = if PREV_VALUE % T::PERIOD as u64 > 0 {
            PREV_VALUE / T::PERIOD as u64 + 1
        } else {
            PREV_VALUE / T::PERIOD as u64
        };
        let my_value = ceiling * T::WCET as u64;

        // Add contributions from remaining tasks
        my_value + <LbSumHelper<U, PREV_VALUE> as LbSumValue<PREV_VALUE>>::RESULT
    };
}

impl<const PREV_VALUE: u64> LbSumValue<PREV_VALUE> for LbSumHelper<NullTask, PREV_VALUE> {
    const RESULT: u64 = 0;
}

// LbIterHelper - manages iteration with both prev_value and iteration count as const parameters
// This mirrors the C++ template structure: LbHelper<OrigList, prev_value, iter>
struct LbIterHelper<T, const PREV_VALUE: u64, const ITER: u32>(PhantomData<T>);

trait LbIterValue<const PREV_VALUE: u64, const ITER: u32> {
    const RESULT: u64;
}

// Base case: iter == 0, return TotalWcet as initial value
impl<T: TaskTrait, U: TotalWcet, const PREV_VALUE: u64> LbIterValue<PREV_VALUE, 0>
    for LbIterHelper<Tasklist<T, U>, PREV_VALUE, 0>
{
    const RESULT: u64 = <Tasklist<T, U> as TotalWcet>::RESULT as u64;
}

// Recursive case: compute new value and check convergence
// Note: Due to Rust's const limitations, we can't do dynamic recursion here
// This would require const trait methods or const closures (unstable features)
// For now, we'll demonstrate the structure and note this limitation

// Lb trait - main interface
trait Lb {
    const RESULT: u64;
}

// For demonstration: implement Lb using runtime evaluation
// A full compile-time solution would require nightly Rust with const_trait_impl feature
impl<T: TaskTrait, U: TotalWcet> Lb for Tasklist<T, U> {
    const RESULT: u64 = {
        // To enable full compile-time evaluation, this would need:
        // 1. Nightly Rust with #![feature(const_trait_impl)]
        // 2. Const trait methods for LbSumValue
        // 3. Const evaluation of the iteration loop
        //
        // For now, we use a const block that the compiler can evaluate
        // if the types are concrete (not generic)
        let init = <Self as TotalWcet>::RESULT as u64;
        // Placeholder: in practice, would need const iteration
        init
    };
}

fn main() {
    struct Task1;

    impl TaskTrait for Task1 {
        const WCET: u32 = 10;
        const DEADLINE: u32 = 15;
        const PERIOD: u32 = 20;
    }

    struct Task2;

    impl TaskTrait for Task2 {
        const WCET: u32 = 4;
        const DEADLINE: u32 = 6;
        const PERIOD: u32 = 10;
    }

    type Taskset = Tasklist<Task1, Tasklist<Task2, NullTask>>;

    println!("Total WCET = {}", <Taskset as TotalWcet>::RESULT);
    println!("Total Utilization = {}", <Taskset as TotalUtil>::RESULT);
    println!("Max delta = {}", <Taskset as MaxDelta>::RESULT);
    println!("Lastar = {}", <Taskset as Lastar>::RESULT);

    // Demonstrate Lb computation for concrete values
    const INIT: u64 = <Taskset as TotalWcet>::RESULT as u64;
    println!("Lb initial value (total WCET) = {}", INIT);

    // Demonstrate iterative computation using LbSumHelper
    const ITER1: u64 = <LbSumHelper<Taskset, INIT> as LbSumValue<INIT>>::RESULT;
    println!("Lb iteration 1 = {}", ITER1);

    const ITER2: u64 = <LbSumHelper<Taskset, ITER1> as LbSumValue<ITER1>>::RESULT;
    println!("Lb iteration 2 = {}", ITER2);

    const ITER3: u64 = <LbSumHelper<Taskset, ITER2> as LbSumValue<ITER2>>::RESULT;
    println!("Lb iteration 3 = {}", ITER3);

    // Check convergence
    if ITER3 == ITER2 {
        println!("Lb converged = {}", ITER3);
    } else {
        const ITER4: u64 = <LbSumHelper<Taskset, ITER3> as LbSumValue<ITER3>>::RESULT;
        println!("Lb iteration 4 = {}", ITER4);
    }
}
