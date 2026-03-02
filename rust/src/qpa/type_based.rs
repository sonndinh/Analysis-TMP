use core::ops::{Add, Sub, Div, Mul, BitAnd};
use std::marker::PhantomData;
use typenum::{Bit, False, Integer, N5, P1, P5, P7, P10, P12, P14, P15, P100, P200, P1000000000000000000, True, Z0};
use typenum::{Sum, Diff, Prod, Quot, Min, Max, Maximum, IsLess, IsEqual, IsLessOrEqual, IsGreater, And};

type PMax = P1000000000000000000;

trait Task
{
    type Wcet;
    type Deadline;
    type Period;
}

struct Nulltask;
impl Task for Nulltask
{
    type Wcet = Z0;
    type Deadline = Z0;
    type Period = Z0;
}

struct Tasklist<T, U>(PhantomData<T>, PhantomData<U>);

trait TotalWcet
{
    type Output;
}

impl TotalWcet for Nulltask
{
    type Output = Z0;
}

impl<T: Task, U: TotalWcet> TotalWcet for Tasklist<T, U>
where <T as Task>::Wcet: Add<<U as TotalWcet>::Output>
{
    type Output = Sum<<T as Task>::Wcet, <U as TotalWcet>::Output>;
}

trait Dmin
{
    type Output;
}

impl Dmin for Nulltask
{
    type Output = PMax;
}

impl<T: Task, U: Dmin> Dmin for Tasklist<T, U>
where <T as Task>::Deadline: Min<<U as Dmin>::Output>
{
    type Output = <T::Deadline as Min<<U as Dmin>::Output>>::Output;
}

trait Pdf<L>
{
    type MaxOperand;
    type MyValue;
    type Output;
}

impl<L> Pdf<L> for Nulltask
{
    type MaxOperand = Z0;
    type MyValue = Z0;
    type Output = Z0;
}

impl<T: Task, U: Pdf<L>, L> Pdf<L> for Tasklist<T, U>
where
    L: Sub<T::Deadline>,
    <L as Sub<T::Deadline>>::Output: Div<T::Period>,
    <<L as Sub<T::Deadline>>::Output as Div<T::Period>>::Output: Add<P1>,
    Sum<<<L as Sub<T::Deadline>>::Output as Div<T::Period>>::Output, P1>: Max<Z0>,
    <Sum<<<L as Sub<T::Deadline>>::Output as Div<T::Period>>::Output, P1> as Max<Z0>>::Output: Mul<T::Wcet>,
    <<Sum<<<L as Sub<T::Deadline>>::Output as Div<T::Period>>::Output, P1> as Max<Z0>>::Output as Mul<T::Wcet>>::Output: Add<<U as Pdf<L>>::Output>
{
    type MaxOperand = Sum<<<L as Sub<T::Deadline>>::Output as Div<T::Period>>::Output, P1>;
    type MyValue = <<Self::MaxOperand as Max<Z0>>::Output as Mul<T::Wcet>>::Output;
    type Output = <Self::MyValue as Add<<U as Pdf<L>>::Output>>::Output;
}

trait If<Then, Else>
{
    type Output;
}

impl<Then, Else> If<Then, Else> for True
{
    type Output = Then;
}

impl<Then, Else> If<Then, Else> for False
{
    type Output = Else;
}

trait Dmax<L>
{
    type Output;
}

impl<L> Dmax<L> for Nulltask
{
    type Output = Z0;
}

type DmaxHelper<L, T> = Sum<Prod<Quot<Diff<L, <T as Task>::Deadline>, <T as Task>::Period>, <T as Task>::Period>, <T as Task>::Deadline>;
type DmaxHelperAdjusted<L, T> = <<DmaxHelper<L, T> as IsEqual<L>>::Output as If<Diff<DmaxHelper<L, T>, <T as Task>::Period>, DmaxHelper<L, T>>>::Output;

impl<T: Task, U: Dmax<L>, L> Dmax<L> for Tasklist<T, U>
where
    L: Sub<T::Deadline>,
    Diff<L, T::Deadline>: Div<T::Period>,
    Quot<Diff<L, T::Deadline>, T::Period>: Mul<T::Period>,
    Prod<Quot<Diff<L, T::Deadline>, T::Period>, T::Period>: Add<T::Deadline>,
    DmaxHelper<L, T>: Max<<U as Dmax<L>>::Output>,
    DmaxHelper<L, T>: IsEqual<L>,
    DmaxHelper<L, T>: Sub<T::Period>,
    <DmaxHelper<L, T> as IsEqual<L>>::Output: If<Diff<DmaxHelper<L, T>, T::Period>, DmaxHelper<L, T>>,
    <<DmaxHelper<L, T> as IsEqual<L>>::Output as If<Diff<DmaxHelper<L, T>, T::Period>, DmaxHelper<L, T>>>::Output: Max<<U as Dmax<L>>::Output>,
    T::Deadline: IsLess<L>,
    DmaxHelperAdjusted<L, T>: Max<<U as Dmax<L>>::Output>,
    <T::Deadline as IsLess<L>>::Output: If<Maximum<DmaxHelperAdjusted<L, T>, <U as Dmax<L>>::Output>, Z0>
{
    type Output = <<T::Deadline as IsLess<L>>::Output as If<Maximum<DmaxHelperAdjusted<L, T>, <U as Dmax<L>>::Output>, Z0>>::Output;
}

type DminValue<T, U> = <Tasklist<T, U> as Dmin>::Output;
type PdfValue<T, U, L> = <Tasklist<T, U> as Pdf<L>>::Output;

trait QpaCondition<L>
{
    type Output;
}

type DminOutput<T, U> = <<T as Task>::Deadline as Min<<U as Dmin>::Output>>::Output;
type PdfMaxOperand<T, L> = Sum<<<L as Sub<<T as Task>::Deadline>>::Output as Div<<T as Task>::Period>>::Output, P1>;
type PdfMyValue<T, L> = <<PdfMaxOperand<T, L> as Max<Z0>>::Output as Mul<<T as Task>::Wcet>>::Output;
type PdfOutput<T, U, L> = <PdfMyValue<T, L> as Add<<U as Pdf<L>>::Output>>::Output;

impl<T: Task, U: Pdf<L> + Dmin, L> QpaCondition<L> for Tasklist<T, U>
where
    // Trait bounds needed for using PdfOutput type alias.
    // These are also trait bounds needed to impl Pdf<L> for Tasklist<T, U>.
    // Thus we can use PdfValue type alias in the Output associated type, i.e.,
    // can cast Tasklist<T, U> as Pdf<L>.
    L: Sub<T::Deadline>,
    <L as Sub<T::Deadline>>::Output: Div<T::Period>,
    <<L as Sub<T::Deadline>>::Output as Div<T::Period>>::Output: Add<P1>,
    Sum<<<L as Sub<T::Deadline>>::Output as Div<T::Period>>::Output, P1>: Max<Z0>,
    <Sum<<<L as Sub<T::Deadline>>::Output as Div<T::Period>>::Output, P1> as Max<Z0>>::Output: Mul<T::Wcet>,
    <<Sum<<<L as Sub<T::Deadline>>::Output as Div<T::Period>>::Output, P1> as Max<Z0>>::Output as Mul<T::Wcet>>::Output: Add<<U as Pdf<L>>::Output>,
    // Bounds needed for using DminOutput type alias
    T::Deadline: Min<<U as Dmin>::Output>,
    // Now we can use the type aliases.
    // PdfOutput is essentially the same type as the Output associated type from
    // impl Pdf<L> block for Tasklist<T, U>. But if we express the bound through
    // Tasklist, e.g, <Tasklist<T, U> as Pdf<L>>::Output: IsLessOrEqual<L>,
    // since Tasklist is a concrete type, Rust will do thorough bound proof.
    // Whereas, here expressing through PdfOutput with abstract types (T, U, L)
    // can defer some checks and accept the IsLessOrEqual bound, for example.
    PdfOutput<T, U, L>: IsLessOrEqual<L>,
    PdfOutput<T, U, L>: IsGreater<DminOutput<T, U>>,
    <PdfOutput<T, U, L> as IsLessOrEqual<L>>::Output: BitAnd<<PdfOutput<T, U, L> as IsGreater<DminOutput<T, U>>>::Output>
{
    type Output = And<<PdfValue<T, U, L> as IsLessOrEqual<L>>::Output, <PdfValue<T, U, L> as IsGreater<DminValue<T, U>>>::Output>;
}

trait QpaDispatch<T, U, L> {
    type Output;
}

// Base case.
// This happens when the QPA condition is false and the QPA loop stops.
// It can return schedulability result here.
impl<T: Task, U: Pdf<L>, L> QpaDispatch<T, U, L> for False
where
    Tasklist<T, U>: Dmin,
    // Bounds for PdfOutput type alias
    L: Sub<T::Deadline>,
    <L as Sub<T::Deadline>>::Output: Div<T::Period>,
    <<L as Sub<T::Deadline>>::Output as Div<T::Period>>::Output: Add<P1>,
    Sum<<<L as Sub<T::Deadline>>::Output as Div<T::Period>>::Output, P1>: Max<Z0>,
    <Sum<<<L as Sub<T::Deadline>>::Output as Div<T::Period>>::Output, P1> as Max<Z0>>::Output: Mul<T::Wcet>,
    <<Sum<<<L as Sub<T::Deadline>>::Output as Div<T::Period>>::Output, P1> as Max<Z0>>::Output as Mul<T::Wcet>>::Output: Add<<U as Pdf<L>>::Output>,
    // Other bounds for the Output associated type
    PdfOutput<T, U, L>: IsLessOrEqual<DminValue<T, U>>,
    <PdfOutput<T, U, L> as IsLessOrEqual<DminValue<T, U>>>::Output: If<True, False>
{
    type Output = <<PdfOutput<T, U, L> as IsLessOrEqual<DminValue<T, U>>>::Output as If<True, False>>::Output;
}

// Recursive case
impl<T: Task, U: Pdf<L> + Dmin, L> QpaDispatch<T, U, L> for True
where
    // Bounds for PdfOutput type alias
    L: Sub<T::Deadline>,
    <L as Sub<T::Deadline>>::Output: Div<T::Period>,
    <<L as Sub<T::Deadline>>::Output as Div<T::Period>>::Output: Add<P1>,
    Sum<<<L as Sub<T::Deadline>>::Output as Div<T::Period>>::Output, P1>: Max<Z0>,
    <Sum<<<L as Sub<T::Deadline>>::Output as Div<T::Period>>::Output, P1> as Max<Z0>>::Output: Mul<T::Wcet>,
    <<Sum<<<L as Sub<T::Deadline>>::Output as Div<T::Period>>::Output, P1> as Max<Z0>>::Output as Mul<T::Wcet>>::Output: Add<<U as Pdf<L>>::Output>,
    // Bounds for UpdatedL
    PdfOutput<T, U, L>: IsLess<L>,
    Tasklist<T, U>: Dmax<L>,
    <PdfOutput<T, U, L> as IsLess<L>>::Output: If<PdfOutput<T, U, L>, <Tasklist<T, U> as Dmax<L>>::Output>,
    // Recursive call
    (T, U, UpdatedL<T, U, L>): Qpa,
{
    type Output = <(T, U, UpdatedL<T, U, L>) as Qpa>::Output;
}

trait Qpa
{
    type Output;
}

// New L value computed within the iteration of the QPA's while loop
type UpdatedL<T, U, L> = <<PdfOutput<T, U, L> as IsLess<L>>::Output as If<PdfOutput<T, U, L>, <Tasklist<T, U> as Dmax<L>>::Output>>::Output;

impl<T, U, L> Qpa for (T, U, L)
where
    Tasklist<T, U>: QpaCondition<L>,
    <Tasklist<T, U> as QpaCondition<L>>::Output: QpaDispatch<T, U, L>,
{
    type Output = <<Tasklist<T, U> as QpaCondition<L>>::Output as QpaDispatch<T, U, L>>::Output;
}

#[test]
fn test() {
    type X = Sum<P10, N5>;
    let result = <X as Integer>::to_i32();
    assert_eq!(result, 5);
    println!("Result is {}", result);

    struct Task1;
    impl Task for Task1 {
        type Wcet = P5;
        type Deadline = P10;
        type Period = P15;
    }

    struct Task2;
    impl Task for Task2 {
        type Wcet = P7;
        type Deadline = P12;
        type Period = P14;
    }

    type Taskset = Tasklist<Task1, Tasklist<Task2, Nulltask>>;

    type Y = <<Task1 as Task>::Wcet as Div<<Task1 as Task>::Period>>::Output;
    let util = <Y as Integer>::I32;
    println!("Utilization of task1: {}", util);

    type Z = <Taskset as TotalWcet>::Output;
    let total_wcet = <Z as Integer>::to_i32();
    println!("Total wcet: {}", total_wcet);

    type MinDeadline = <Taskset as Dmin>::Output;
    println!("Dmin: {}", <MinDeadline as Integer>::to_i32());

    type PdfOf100 = <Taskset as Pdf<P100>>::Output;
    println!("h(100): {}", <PdfOf100 as Integer>::to_i32());

    type DmaxOf100 = <Taskset as Dmax<P100>>::Output;
    println!("Dmax(100): {}", <DmaxOf100 as Integer>::to_i32());

    type QpaConditionOf100 = <Taskset as QpaCondition<P100>>::Output;
    println!("QpaCondition(100): {}", <QpaConditionOf100 as Bit>::to_bool());

    type QpaConditionOf200 = <Taskset as QpaCondition<P200>>::Output;
    println!("QpaCondition(200): {}", <QpaConditionOf200 as Bit>::to_bool());

    type QpaOf100 = <(Task1, Tasklist<Task2, Nulltask>, P100) as Qpa>::Output;
    println!("Qpa(100): {}", <QpaOf100 as Bit>::to_bool());

    type QpaOf12 = <(Task1, Tasklist<Task2, Nulltask>, P12) as Qpa>::Output;
    println!("Qpa(12): {}", <QpaOf12 as Bit>::to_bool());
}
