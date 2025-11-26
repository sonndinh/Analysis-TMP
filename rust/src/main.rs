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
    const fn lastar() ->i64 {
        let max_delta = <Self as MaxDelta>::RESULT as i64;
        // TODO: handling case when total utilization >= 1.0
        let lastar_helper = (<Self as LastarHelper>::RESULT / (1.0 - <Self as TotalUtil>::RESULT)) as i64;
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
}
