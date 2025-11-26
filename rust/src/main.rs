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

impl<T: TaskTrait, U: MaxDelta> Tasklist<T, U> {
    const fn result() -> i32 {
        let first = T::deadline as i32 - T::period as i32;
        let second = U::result;
        if first > second {
            first
        } else {
            second
        }
    }
}

impl<T: TaskTrait, U: MaxDelta> MaxDelta for Tasklist<T, U> {
    const result: i32 = Tasklist::<T, U>::result();
}

impl MaxDelta for NullTask {
    const result: i32 = i32::MIN;
}

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
    println!("Max delta = {}", <Taskset as MaxDelta>::result);
}
