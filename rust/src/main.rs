use std::marker::PhantomData;

// Task list
pub trait TaskTrait {
  const wcet: u32;
  const deadline: u32;
  const period: u32;
}

pub struct NullTask;

impl TaskTrait for NullTask {
  const wcet: u32 = 0;
  const deadline: u32 = 0;
  const period: u32 = 0;
}

// TODO: Rust doesn't have the flexibility of C++ templates and we need to
// specify the trait for each type since Rust won't allow using an arbitrary type for a type parameter.
// - need trait for Task
// - Change TypelistTrait to TasksetTrait and Head is bounded to the Task's trait
pub trait NonEmptyListTrait {}

pub trait TasklistTrait {
  type Head: TaskTrait;
  type Tail: TasklistTrait;
}

pub struct Tasklist<T: TaskTrait, U: TasklistTrait> {
  _head: PhantomData<T>,
  _tail: PhantomData<U>,
}

impl<T, U> TasklistTrait for Tasklist<T, U>
  where T: TaskTrait, U: TasklistTrait {
    type Head = T;
    type Tail = U;
}

impl<T, U> NonEmptyListTrait for Tasklist<T, U>
  where T: TaskTrait, U: TasklistTrait {}

pub struct NullTasklist;

impl TasklistTrait for NullTasklist {
  type Head = NullTask;
  type Tail = NullTasklist;
}

// Total WCET
trait TotalWcetTrait {
  const result: u32;
}

pub struct TotalWcet<TL: TasklistTrait> {
  _typelist: PhantomData<TL>
}

impl TotalWcetTrait for TotalWcet<NullTasklist> {
  const result: u32 = 0;
}

impl<TL: TasklistTrait + NonEmptyListTrait> TotalWcetTrait for TotalWcet<TL> {
  const result: u32 = TL::Head::wcet + TotalWcet::<TL::Tail>::result;
}

fn main() {
  struct Task1;

  #[allow(non_upper_case_globals)]
  #[allow(dead_code)]
  impl TaskTrait for Task1 {
    const wcet: u32 = 10;
    const deadline: u32 = 15;
    const period: u32 = 20;
  }

  struct Task2;

  #[allow(non_upper_case_globals)]
  #[allow(dead_code)]
  impl TaskTrait for Task2 {
    const wcet: u32 = 4;
    const deadline: u32 = 6;
    const period: u32 = 10;
  }

  #[allow(dead_code)]
  type Tasks = Tasklist<Task1, Tasklist<Task2, NullTasklist>>;

  println!("Total WCET = {}", TotalWcet::<Tasks>::result);
}
