use core::ops::{Add, Sub, Div, Mul, BitAnd, Rem};
use std::marker::PhantomData;
use typenum::{False, True};
use typenum::{Sum, Diff, Prod, Quot, Mod, Min, Max, Maximum, IsEqual, IsLess, IsLessOrEqual, IsGreater, IsGreaterOrEqual, And};
use typenum::{P1, P1000000000000000000, Z0};
use crate::common::If;

type PMax = P1000000000000000000;

pub trait Task
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

pub struct Tasklist<T, U>(PhantomData<T>, PhantomData<U>);

pub trait TotalWcet
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

pub trait Dmin
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

pub trait Pdf<L>
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

pub trait Dmax<L>
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
pub trait LbStopCondition<PrevL, L>
{
    type Output;
}

impl<PrevL, L> LbStopCondition<PrevL, L> for ()
where
    PrevL: IsEqual<L>
{
    type Output = <PrevL as IsEqual<L>>::Output;
}

pub trait LbHelper
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

pub trait LbDispatch<T, U, L>
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

pub trait Lb
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

pub trait QpaCondition<L>
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

pub trait QpaDispatch<T, U, L>
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

pub trait Qpa
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

#[cfg(test)]
mod tests;
