use core::ops::{Add, Sub, Div, Mul, BitAnd, Rem};
use std::marker::PhantomData;
use typenum::{Bit, False, Integer, P1, P2, P3, P4, P5, P6, P7, P8, P9, P10, P12, P14, P15, P16, P17, P18, P19, P26, P31, P90, P96, P100, P160, P200, P280, P404, P660, P800, P1000, P1000000000000000000, True, Z0};
use typenum::{Sum, Diff, Prod, Quot, Mod, Min, Max, Maximum, IsLess, IsEqual, IsLessOrEqual, IsGreater, IsGreaterOrEqual, And};

type P2000 = typenum::op!(P1000 * P2);
type P3000 = typenum::op!(P1000 * P3);
type P4200 = typenum::op!(P1000 * P4 + P200);
type P6000 = typenum::op!(P1000 * P6);
type P9000 = typenum::op!(P1000 * P9);
type P9800 = typenum::op!(P1000 * P9 + P800);
type P12000 = typenum::op!(P1000 * P12);
type P15404 = typenum::op!(P1000 * P15 + P404);
type P17000 = typenum::op!(P1000 * P17);
type P18000 = typenum::op!(P1000 * P18);
type P31000 = typenum::op!(P1000 * P31);

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
where
    <T as Task>::Wcet: Add<<U as TotalWcet>::Output>
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
where
    <T as Task>::Deadline: Min<<U as Dmin>::Output>
{
    type Output = <T::Deadline as Min<<U as Dmin>::Output>>::Output;
}

trait Pdf<L>
{
    type Output;
}

impl<L> Pdf<L> for Nulltask
{
    type Output = Z0;
}

type PdfCommonFloorTerm<T, L> = Quot<<L as Sub<<T as Task>::Deadline>>::Output, <T as Task>::Period>;
type PdfNegativeFloorTerm<T, L> = <<<<L as Sub<<T as Task>::Deadline>>::Output as Rem<<T as Task>::Period>>::Output as IsEqual<Z0>>::Output as If<PdfCommonFloorTerm<T, L>, <PdfCommonFloorTerm<T, L> as Sub<P1>>::Output>>::Output;
type PdfFloorTerm<T, L> = <<L as IsGreaterOrEqual<<T as Task>::Deadline>>::Output as If<PdfCommonFloorTerm<T, L>, PdfNegativeFloorTerm<T, L>>>::Output;
type PdfMyValue<T, L> = <<Sum<PdfFloorTerm<T, L>, P1> as Max<Z0>>::Output as Mul<<T as Task>::Wcet>>::Output;
type PdfOutput<T, U, L> = <PdfMyValue<T, L> as Add<<U as Pdf<L>>::Output>>::Output;

impl<T: Task, U: Pdf<L>, L> Pdf<L> for Tasklist<T, U>
where
    // Bounds for PdfCommonFloorTerm
    L: Sub<T::Deadline>,
    <L as Sub<T::Deadline>>::Output: Div<T::Period>,
    // Bounds for PdfNegativeFloorTerm
    <L as Sub<T::Deadline>>::Output: Rem<T::Period>,
    <<L as Sub<T::Deadline>>::Output as Rem<T::Period>>::Output: IsEqual<Z0>,
    PdfCommonFloorTerm<T, L>: Sub<P1>,
    <<<L as Sub<T::Deadline>>::Output as Rem<T::Period>>::Output as IsEqual<Z0>>::Output: If<PdfCommonFloorTerm<T, L>, <PdfCommonFloorTerm<T, L> as Sub<P1>>::Output>,
    // Bounds for PdfFloorTerm
    L: IsGreaterOrEqual<T::Deadline>,
    <L as IsGreaterOrEqual<T::Deadline>>::Output: If<PdfCommonFloorTerm<T, L>, PdfNegativeFloorTerm<T, L>>,
    // Bounds for the associated types
    PdfFloorTerm<T, L>: Add<P1>,
    Sum<PdfFloorTerm<T, L>, P1>: Max<Z0>,
    <Sum<PdfFloorTerm<T, L>, P1> as Max<Z0>>::Output: Mul<T::Wcet>,
    <<Sum<PdfFloorTerm<T, L>, P1> as Max<Z0>>::Output as Mul<T::Wcet>>::Output: Add<<U as Pdf<L>>::Output>
{
    type Output = PdfOutput<T, U, L>;
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
    <T::Deadline as IsLess<L>>::Output: If<DmaxHelperAdjusted<L, T>, Z0>,
    <<T::Deadline as IsLess<L>>::Output as If<DmaxHelperAdjusted<L, T>, Z0>>::Output: Max<<U as Dmax<L>>::Output>
{
    type Output = Maximum<<<T::Deadline as IsLess<L>>::Output as If<DmaxHelperAdjusted<L, T>, Z0>>::Output, <U as Dmax<L>>::Output>;
}

// This only compute the max{(D1 - T1), ..., (Dn - Tn)} part of LaStar.
// TODO: The remaining operand requires floating point values for utilization.
trait LaStar
{
    type Output;
}

impl<T: Task, U: LaStar> LaStar for Tasklist<T, U>
where
    T::Deadline: Sub<T::Period>,
    Diff<T::Deadline, T::Period>: Max<<U as LaStar>::Output>
{
    type Output = <Diff<T::Deadline, T::Period> as Max<<U as LaStar>::Output>>::Output;
}

impl LaStar for Nulltask
{
    type Output = Z0;
}

// Stop when Lb value converges
trait LbStopCondition<PrevL, L>
{
    type Output;
}

impl<PrevL, L> LbStopCondition<PrevL, L> for ()
where
    PrevL: IsEqual<L>
{
    type Output = <PrevL as IsEqual<L>>::Output;
}

trait LbHelper
{
    type Output;
}

type LbCeilTerm<T, L> = <<Mod<L, <T as Task>::Period> as IsGreater<Z0>>::Output as If<Sum<Quot<L, <T as Task>::Period>, P1>, Quot<L, <T as Task>::Period>>>::Output;

// Return the next Lb value in Output given the current Lb in L
impl<T: Task, U, L> LbHelper for (Tasklist<T, U>, L)
where
    (U, L): LbHelper,
    // Bounds for LbCeilTerm type alias
    L: Rem<T::Period>,
    Mod<L, T::Period>: IsGreater<Z0>,
    L: Div<T::Period>,
    Quot<L, T::Period>: Add<P1>,
    <Mod<L, T::Period> as IsGreater<Z0>>::Output: If<Sum<Quot<L, T::Period>, P1>, Quot<L, T::Period>>,
    // Bounds for the Output associated type
    LbCeilTerm<T, L>: Mul<T::Wcet>,
    Prod<LbCeilTerm<T, L>, T::Wcet>: Add<<(U, L) as LbHelper>::Output>,
{
    type Output = Sum<Prod<LbCeilTerm<T, L>, T::Wcet>, <(U, L) as LbHelper>::Output>;
}

impl<L> LbHelper for (Nulltask, L)
{
    type Output = Z0;
}

trait LbDispatch<T, U, L>
{
    type Output;
}

impl<T, U, L> LbDispatch<T, U, L> for True
{
    type Output = L;
}

impl<T, U, L> LbDispatch<T, U, L> for False
where
    (Tasklist<T, U>, L): LbHelper,
    (T, U, L, <(Tasklist<T, U>, L) as LbHelper>::Output): Lb
{
    type Output = <(T, U, L, <(Tasklist<T, U>, L) as LbHelper>::Output) as Lb>::Output;
}

trait Lb
{
    type Output;
}

impl<T, U, PrevL, L> Lb for (T, U, PrevL, L)
where
    (): LbStopCondition<PrevL, L>,
    <() as LbStopCondition<PrevL, L>>::Output: LbDispatch<T, U, L>
{
    type Output = <<() as LbStopCondition<PrevL, L>>::Output as LbDispatch<T, U, L>>::Output;
}

type DminValue<T, U> = <Tasklist<T, U> as Dmin>::Output;
type PdfValue<T, U, L> = <Tasklist<T, U> as Pdf<L>>::Output;

trait QpaCondition<L>
{
    type Output;
}

type DminOutput<T, U> = <<T as Task>::Deadline as Min<<U as Dmin>::Output>>::Output;

impl<T: Task, U: Pdf<L> + Dmin, L> QpaCondition<L> for Tasklist<T, U>
where
    // Trait bounds needed for using PdfOutput type alias.
    // These are also trait bounds needed to impl Pdf<L> for Tasklist<T, U>.
    // Thus we can use PdfValue type alias in the Output associated type, i.e.,
    // can cast Tasklist<T, U> as Pdf<L>.
    // TODO: Factor out these bounds for PdfOutput so we don't have to repeat them here or other places where it's used.
    // Bounds for PdfCommonFloorTerm
    L: Sub<T::Deadline>,
    <L as Sub<T::Deadline>>::Output: Div<T::Period>,
    // Bounds for PdfNegativeFloorTerm
    <L as Sub<T::Deadline>>::Output: Rem<T::Period>,
    <<L as Sub<T::Deadline>>::Output as Rem<T::Period>>::Output: IsEqual<Z0>,
    PdfCommonFloorTerm<T, L>: Sub<P1>,
    <<<L as Sub<T::Deadline>>::Output as Rem<T::Period>>::Output as IsEqual<Z0>>::Output: If<PdfCommonFloorTerm<T, L>, <PdfCommonFloorTerm<T, L> as Sub<P1>>::Output>,
    // Bounds for PdfFloorTerm
    L: IsGreaterOrEqual<T::Deadline>,
    <L as IsGreaterOrEqual<T::Deadline>>::Output: If<PdfCommonFloorTerm<T, L>, PdfNegativeFloorTerm<T, L>>,
    // Bounds for the associated types
    PdfFloorTerm<T, L>: Add<P1>,
    Sum<PdfFloorTerm<T, L>, P1>: Max<Z0>,
    <Sum<PdfFloorTerm<T, L>, P1> as Max<Z0>>::Output: Mul<T::Wcet>,
    <<Sum<PdfFloorTerm<T, L>, P1> as Max<Z0>>::Output as Mul<T::Wcet>>::Output: Add<<U as Pdf<L>>::Output>,

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

trait QpaDispatch<T, U, L>
{
    type Output;
}

// Base case.
// This happens when the QPA condition is false and the QPA loop stops.
// It can return schedulability result here.
impl<T: Task, U: Pdf<L>, L> QpaDispatch<T, U, L> for False
where
    Tasklist<T, U>: Dmin,
    // Bounds for PdfOutput type alias (copied from Pdf impl block for Tasklist)
    // Bounds for PdfCommonFloorTerm
    L: Sub<T::Deadline>,
    <L as Sub<T::Deadline>>::Output: Div<T::Period>,
    // Bounds for PdfNegativeFloorTerm
    <L as Sub<T::Deadline>>::Output: Rem<T::Period>,
    <<L as Sub<T::Deadline>>::Output as Rem<T::Period>>::Output: IsEqual<Z0>,
    PdfCommonFloorTerm<T, L>: Sub<P1>,
    <<<L as Sub<T::Deadline>>::Output as Rem<T::Period>>::Output as IsEqual<Z0>>::Output: If<PdfCommonFloorTerm<T, L>, <PdfCommonFloorTerm<T, L> as Sub<P1>>::Output>,
    // Bounds for PdfFloorTerm
    L: IsGreaterOrEqual<T::Deadline>,
    <L as IsGreaterOrEqual<T::Deadline>>::Output: If<PdfCommonFloorTerm<T, L>, PdfNegativeFloorTerm<T, L>>,
    // Bounds for the associated types
    PdfFloorTerm<T, L>: Add<P1>,
    Sum<PdfFloorTerm<T, L>, P1>: Max<Z0>,
    <Sum<PdfFloorTerm<T, L>, P1> as Max<Z0>>::Output: Mul<T::Wcet>,
    <<Sum<PdfFloorTerm<T, L>, P1> as Max<Z0>>::Output as Mul<T::Wcet>>::Output: Add<<U as Pdf<L>>::Output>,

    // Other bounds for the Output associated type
    PdfOutput<T, U, L>: IsLessOrEqual<DminValue<T, U>>,
    <PdfOutput<T, U, L> as IsLessOrEqual<DminValue<T, U>>>::Output: If<True, False>
{
    type Output = <<PdfOutput<T, U, L> as IsLessOrEqual<DminValue<T, U>>>::Output as If<True, False>>::Output;
}

// Recursive case
impl<T: Task, U: Pdf<L> + Dmin, L> QpaDispatch<T, U, L> for True
where
    // Bounds for PdfOutput type alias (copied from Pdf impl block for Tasklist)
    // Bounds for PdfCommonFloorTerm
    L: Sub<T::Deadline>,
    <L as Sub<T::Deadline>>::Output: Div<T::Period>,
    // Bounds for PdfNegativeFloorTerm
    <L as Sub<T::Deadline>>::Output: Rem<T::Period>,
    <<L as Sub<T::Deadline>>::Output as Rem<T::Period>>::Output: IsEqual<Z0>,
    PdfCommonFloorTerm<T, L>: Sub<P1>,
    <<<L as Sub<T::Deadline>>::Output as Rem<T::Period>>::Output as IsEqual<Z0>>::Output: If<PdfCommonFloorTerm<T, L>, <PdfCommonFloorTerm<T, L> as Sub<P1>>::Output>,
    // Bounds for PdfFloorTerm
    L: IsGreaterOrEqual<T::Deadline>,
    <L as IsGreaterOrEqual<T::Deadline>>::Output: If<PdfCommonFloorTerm<T, L>, PdfNegativeFloorTerm<T, L>>,
    // Bounds for the associated types
    PdfFloorTerm<T, L>: Add<P1>,
    Sum<PdfFloorTerm<T, L>, P1>: Max<Z0>,
    <Sum<PdfFloorTerm<T, L>, P1> as Max<Z0>>::Output: Mul<T::Wcet>,
    <<Sum<PdfFloorTerm<T, L>, P1> as Max<Z0>>::Output as Mul<T::Wcet>>::Output: Add<<U as Pdf<L>>::Output>,

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
fn test1() {
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

    type SumWcet = <Taskset as TotalWcet>::Output;
    let total_wcet = <SumWcet as Integer>::to_i32();
    println!("Total wcet: {}", total_wcet);

    type MinDeadline = <Taskset as Dmin>::Output;
    println!("Dmin: {}", <MinDeadline as Integer>::to_i32());

    type MyLb = <(Task1, Tasklist<Task2, Nulltask>, Z0, SumWcet) as Lb>::Output;
    assert_eq!(<MyLb as Integer>::to_i32(), 12);
    println!("Lb: {}", <MyLb as Integer>::to_i32());

    type DmaxOf12 = <Taskset as Dmax<P12>>::Output;
    assert_eq!(<DmaxOf12 as Integer>::to_i32(), 10);
    println!("Dmax(Lb=12): {}", <DmaxOf12 as Integer>::to_i32());

    type PdfOf10 = <Taskset as Pdf<P10>>::Output;
    assert_eq!(<PdfOf10 as Integer>::to_i32(), 5);
    println!("h(t=10): {}", <PdfOf10 as Integer>::to_i32());

    type MyQpa = <(Task1, Tasklist<Task2, Nulltask>, MyLb) as Qpa>::Output;
    assert!(<MyQpa as Bit>::to_bool());
    println!("Qpa: {}", <MyQpa as Bit>::to_bool());
}

#[test]
fn test2() {
    struct Task1;
    impl Task for Task1 {
        type Wcet = P6000;
        type Deadline = P18000;
        type Period = P31000;
    }

    struct Task2;
    impl Task for Task2 {
        type Wcet = P2000;
        type Deadline = P9000;
        type Period = P9800;
    }

    struct Task3;
    impl Task for Task3 {
        type Wcet = P1000;
        type Deadline = P12000;
        type Period = P17000;
    }

    struct Task4;
    impl Task for Task4 {
        type Wcet = P90;
        type Deadline = P3000;
        type Period = P4200;
    }

    struct Task5;
    impl Task for Task5 {
        type Wcet = P8;
        type Deadline = P10;
        type Period = P96;
    }

    struct Task6;
    impl Task for Task6 {
        type Wcet = P2;
        type Deadline = P16;
        type Period = P12;
    }

    struct Task7;
    impl Task for Task7 {
        type Wcet = P10;
        type Deadline = P19;
        type Period = P280;
    }

    struct Task8;
    impl Task for Task8 {
        type Wcet = P26;
        type Deadline = P160;
        type Period = P660;
    }

    type RemainingTasks = Tasklist<Task2, Tasklist<Task3, Tasklist<Task4, Tasklist<Task5, Tasklist<Task6, Tasklist<Task7, Tasklist<Task8, Nulltask>>>>>>>;
    type Taskset = Tasklist<Task1, RemainingTasks>;

    type SumWcet = <Taskset as TotalWcet>::Output;
    let total_wcet = <SumWcet as Integer>::to_i32();
    println!("Total wcet: {}", total_wcet);

    type MyLb = <(Task1, RemainingTasks, Z0, SumWcet) as Lb>::Output;
    assert_eq!(<MyLb as Integer>::to_i32(), 16984);
    println!("Lb: {}", <MyLb as Integer>::to_i32());

    type Dmax16984 = <Tasklist<Task1, RemainingTasks> as Dmax<MyLb>>::Output;
    println!("Dmax(16984): {}", <Dmax16984 as Integer>::to_i32());

    // La* is 15404
    type Dmax15404 = <Tasklist<Task1, RemainingTasks> as Dmax<P15404>>::Output;
    assert_eq!(<Dmax15404 as Integer>::to_i32(), 15400);
    println!("Dmax(15404): {}", <Dmax15404 as Integer>::to_i32());

    // Test QPA using La*
    type QpaUsingLaStar = <(Task1, RemainingTasks, Dmax15404) as Qpa>::Output;
    println!("Qpa(La*): {}", <QpaUsingLaStar as Bit>::to_bool());

    // Test QPA using Lb -- should get the same result as when using La*
    type QpaUsingLb = <(Task1, RemainingTasks, Dmax16984) as Qpa>::Output;
    println!("Qpa(Lb): {}", <QpaUsingLb as Bit>::to_bool());

    // Tracing the QPA iterations
    // t = Dmax15404 = 15400
    // type QpaCondition15400 = <Tasklist<Task1, RemainingTasks> as QpaCondition<Dmax15404>>::Output;
    // println!("QpaCondition<15400>: {}", <QpaCondition15400 as Bit>::to_bool());

    type Pdf15400 = PdfValue<Task1, RemainingTasks, Dmax15404>;
    println!("Pdf<15400>: {}", <Pdf15400 as Integer>::to_i32());
}
