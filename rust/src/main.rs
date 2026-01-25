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
// Compute the next value for Lb given the value from previous iteration.
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

// // LbIterHelper - manages iteration with both prev_value and iteration count as const parameters
// // This mirrors the C++ template structure: LbHelper<OrigList, prev_value, iter>
// struct LbIterHelper<T, const PREV_VALUE: u64, const ITER: u32>(PhantomData<T>);

// trait LbIterValue<const PREV_VALUE: u64, const ITER: u32> {
//     const RESULT: u64;
// }

// // Base case: iter == 0, return TotalWcet as initial value
// impl<T: TaskTrait, U: TotalWcet, const PREV_VALUE: u64> LbIterValue<PREV_VALUE, 0>
//     for LbIterHelper<Tasklist<T, U>, PREV_VALUE, 0>
// {
//     const RESULT: u64 = <Tasklist<T, U> as TotalWcet>::RESULT as u64;
// }

// impl<T: TaskTrait, U, const PREV_VALUE: u64, const ITER: u32> LbIterValue<PREV_VALUE, ITER> for LbIterHelper<Tasklist<T, U>, PREV_VALUE, ITER>
// {

// }

struct TasklistWrapper<T, U, const PREV_VALUE: u64>(PhantomData<T>, PhantomData<U>);

impl<T: TaskTrait, U, const PREV_VALUE: u64> TasklistWrapper<T, U, PREV_VALUE> where LbSumHelper<U, PREV_VALUE>: LbSumValue<PREV_VALUE> {
    const fn next_value() -> u64 {
        <LbSumHelper<Tasklist<T, U>, PREV_VALUE> as LbSumValue<PREV_VALUE>>::RESULT
    }
}

// impl<T: TaskTrait, U> Tasklist<T, U> {
//     const fn next_value<const PREV_VALUE: u64>() -> u64 {
//         // TODO: To make this compile, U must be bounded such that LbSumHelper<U, PREV_VALUE>: LbSumValue<PREV_VALUE>
//         <LbSumHelper<Tasklist<T, U>, PREV_VALUE> as LbSumValue<PREV_VALUE>>::RESULT
//     }
// }

// Trait for computing Lb
trait LbHelper<const PREV_VALUE: u64> {
    const RESULT: u64;
}

impl<T: TaskTrait, U, const PREV_VALUE: u64> LbHelper<PREV_VALUE> for TasklistWrapper<T, U, PREV_VALUE> where LbSumHelper<U, PREV_VALUE>: LbSumValue<PREV_VALUE> {
    const RESULT: u64 = {
        if Self::next_value() == PREV_VALUE {
            PREV_VALUE
        } else {
            // TODO: We use a placeholder value here but need to figure out how to recursively call LbHelper with the new value.
            0
        }
    };
}

// impl<T: TaskTrait, U: TotalWcet, const PREV_VALUE: u64> Lb<PREV_VALUE, 0> for Tasklist<T, U> {
//     const RESULT: u64 = <Tasklist<T, U> as TotalWcet>::RESULT as u64;
// }

// ITER starts from 1
// impl<T: TaskTrait, U, const PREV_VALUE: u64> LbHelper<PREV_VALUE> for Tasklist<T, U> {
//     // TODO: Try moving this const RESULT to struct Tasklist so it becomes associated constant.
//     // See if it can reference the generic parameters after that.
//     const RESULT: u64 = {
//         // const MY_VALUE: u64 = <LbSumHelper<Tasklist<T, U>, PREV_VALUE> as LbSumValue<PREV_VALUE>>::RESULT;
//         // let MY_VALUE: u64 = <LbSumHelper<Tasklist<T, U>, PREV_VALUE> as LbSumValue<PREV_VALUE>>::RESULT;
//         // if MY_VALUE == PREV_VALUE {
//         //     MY_VALUE
//         // } else {
//         //     // const NEXT_ITER: u32 = ITER + 1;
//         //     // <Tasklist<T, U> as LbHelper<MY_VALUE, NEXT_ITER>>::RESULT
//         //     <Tasklist<T, U> as LbHelper<MY_VALUE, const {ITER + 1}>>::RESULT
//         // }


//         // if <LbSumHelper<Tasklist<T, U>, PREV_VALUE> as LbSumValue<PREV_VALUE>>::RESULT == PREV_VALUE {
//         //     PREV_VALUE
//         // } else {
//         //     <Tasklist<T, U> as LbHelper<<LbSumHelper<Tasklist<T, U>, PREV_VALUE> as LbSumValue<PREV_VALUE>>::RESULT/*, const {ITER + 1}*/>>::RESULT
//         // }

//         if Self::next_value::<PREV_VALUE>() == PREV_VALUE {
//             PREV_VALUE
//         } else {
//             // <Tasklist<T, U> as LbHelper<{ Self::next_value::<PREV_VALUE>() }>>::RESULT
//             0
//         }
//     };
// }

trait Lb {
    const RESULT: u64;
}

// Compute processor demand function h(t)
trait Pdf<const L: u64> {
    const RESULT: u64;
}

impl<T: TaskTrait, U> Tasklist<T, U> {
    // Helper to compute the pdf value associated with task T
    const fn pdf_value<const L: u64>() -> u64 {
        let sub: i64 = L as i64 - T::DEADLINE as i64;
        let floor_value: i64 = {
            if sub >= 0 {
                sub / T::PERIOD as i64
            } else {
                if sub % T::PERIOD as i64 == 0 {
                    sub / T::PERIOD as i64
                } else {
                    (sub / T::PERIOD as i64) - 1
                }
            }
        };
        if (1 + floor_value) < 0 {
            0
        } else {
            (1 + floor_value) as u64 * T::WCET as u64
        }
    }
}

impl<T: TaskTrait, U, const L: u64> Pdf<L> for Tasklist<T, U> where U: Pdf<L>
{
    const RESULT: u64 = {
        Self::pdf_value::<L>() + <U as Pdf<L>>::RESULT
    };
}

impl<const L: u64> Pdf<L> for NullTask {
    const RESULT: u64 = 0;
}

// Max absolute deadline that is less than or equal to t

// Main schedulability test


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
