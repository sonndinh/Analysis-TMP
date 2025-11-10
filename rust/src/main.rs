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

struct Tasklist<T, U> (PhantomData<T>, PhantomData<U>);

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

  println!("Total WCET = {}", <Tasklist<Task1, Tasklist<Task2, NullTask>> as TotalWcet>::result);
}
