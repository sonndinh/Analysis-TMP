// Rational numbers using typenum integers
use std::ops::{Add, Sub, Mul, Div, Rem};
use std::marker::PhantomData;
use typenum::Z0;
use typenum::{True, False, IsNotEqual, Maximum, Minimum, Max, Min, Abs, AbsVal};

trait GCD
{
    type Output;
}

// Input A and B must be non-negative and such that A >= B
trait GCDBody<A, B>
{
    type Output;
}

impl<A, B> GCDBody<A, B> for False
{
    type Output = A;
}

// Compute GCD using the Euclidean algorithm.
type NextGcdA<B> = B;
type NextGcdB<A, B> = <A as Rem<B>>::Output;

impl<A, B> GCDBody<A, B> for True
where
    A: Rem<B>,
    (NextGcdA<B>, NextGcdB<A, B>): GCD
{
    type Output = <(NextGcdA<B>, NextGcdB<A, B>) as GCD>::Output;
}

type GcdA<X, Y> = Maximum<AbsVal<X>, AbsVal<Y>>;
type GcdB<X, Y> = Minimum<AbsVal<X>, AbsVal<Y>>;

// No constraint on X and Y -- X, Y can be negative and in arbitrary order.
// First, take absolute value of X and Y.
// Then pass to GCDBody such that the larger number is first.
impl<X, Y> GCD for (X, Y)
where
    X: Abs,
    Y: Abs,
    AbsVal<X>: Max<AbsVal<Y>>,
    AbsVal<X>: Min<AbsVal<Y>>,
    GcdB<X, Y>: IsNotEqual<Z0>,
    <GcdB<X, Y> as IsNotEqual<Z0>>::Output: GCDBody<GcdA<X, Y>, GcdB<X, Y>>
{
    type Output = <<GcdB<X, Y> as IsNotEqual<Z0>>::Output as GCDBody<GcdA<X, Y>, GcdB<X, Y>>>::Output;
}

trait RationalNumber
{
    type Numerator;
    type Denominator;
}

struct Rational<N, D>(PhantomData<N>, PhantomData<D>);

trait Reduce<F>
{
    type Output;
}

// Simplify the numerator and denominator by a factor F
impl<N, D, F> Reduce<F> for Rational<N, D>
where
    N: Div<F>,
    D: Div<F>
{
    type Output = Rational<<N as Div<F>>::Output, <D as Div<F>>::Output>;
}

impl<N, D> RationalNumber for Rational<N, D>
{
    type Numerator = N;
    type Denominator = D;
}

type CommonDenominator<D1, D2> = <D1 as Mul<D2>>::Output;
type AddNumerator<N1, D1, N2, D2> = <<N1 as Mul<D2>>::Output as Add<<N2 as Mul<D1>>::Output>>::Output;
type AddDenominator<D1, D2> = CommonDenominator<D1, D2>;

// TODO: Simplify the input N1, D1, N2, D2 before computing the sum, sub, mul, div
impl<N1, D1, N2, D2> Add<Rational<N2, D2>> for Rational<N1, D1>
where
    N1: Mul<D2>,
    N2: Mul<D1>,
    <N1 as Mul<D2>>::Output: Add<<N2 as Mul<D1>>::Output>,
    D1: Mul<D2>,
    (AddNumerator<N1, D1, N2, D2>, AddDenominator<D1, D2>): GCD,
    Rational<AddNumerator<N1, D1, N2, D2>, AddDenominator<D1, D2>>: Reduce<<(AddNumerator<N1, D1, N2, D2>, AddDenominator<D1, D2>) as GCD>::Output>
{
    type Output = <Rational<AddNumerator<N1, D1, N2, D2>, AddDenominator<D1, D2>> as Reduce<<(AddNumerator<N1, D1, N2, D2>, AddDenominator<D1, D2>) as GCD>::Output>>::Output;

    fn add(self, _other: Rational<N2, D2>) -> Self::Output {
        unimplemented!()
    }
}

type SubNumerator<N1, D1, N2, D2> = <<N1 as Mul<D2>>::Output as Sub<<N2 as Mul<D1>>::Output>>::Output;
type SubDenominator<D1, D2> = CommonDenominator<D1, D2>;

impl<N1, D1, N2, D2> Sub<Rational<N2, D2>> for Rational<N1, D1>
where
    N1: Mul<D2>,
    N2: Mul<D1>,
    <N1 as Mul<D2>>::Output: Sub<<N2 as Mul<D1>>::Output>,
    D1: Mul<D2>,
    (SubNumerator<N1, D1, N2, D2>, SubDenominator<D1, D2>): GCD,
    Rational<SubNumerator<N1, D1, N2, D2>, SubDenominator<D1, D2>>: Reduce<<(SubNumerator<N1, D1, N2, D2>, SubDenominator<D1, D2>) as GCD>::Output>
{
    type Output = <Rational<SubNumerator<N1, D1, N2, D2>, SubDenominator<D1, D2>> as Reduce<<(SubNumerator<N1, D1, N2, D2>, SubDenominator<D1, D2>) as GCD>::Output>>::Output;

    fn sub(self, _other: Rational<N2, D2>) -> Self::Output {
        unimplemented!()
    }
}

type MulNumerator<N1, N2> = <N1 as Mul<N2>>::Output;
type MulDenominator<D1, D2> = CommonDenominator<D1, D2>;

impl<N1, D1, N2, D2> Mul<Rational<N2, D2>> for Rational<N1, D1>
where
    N1: Mul<N2>,
    D1: Mul<D2>,
    (MulNumerator<N1, N2>, MulDenominator<D1, D2>): GCD,
    Rational<MulNumerator<N1, N2>, MulDenominator<D1, D2>>: Reduce<<(MulNumerator<N1, N2>, MulDenominator<D1, D2>) as GCD>::Output>
{
    type Output = <Rational<MulNumerator<N1, N2>, MulDenominator<D1, D2>> as Reduce<<(MulNumerator<N1, N2>, MulDenominator<D1, D2>) as GCD>::Output>>::Output;

    fn mul(self, _other: Rational<N2, D2>) -> Self::Output {
        unimplemented!()
    }
}

type DivNumerator<N1, D2> = <N1 as Mul<D2>>::Output;
type DivDenominator<D1, N2> = <D1 as Mul<N2>>::Output;

impl<N1, D1, N2, D2> Div<Rational<N2, D2>> for Rational<N1, D1>
where
    N1: Mul<D2>,
    D1: Mul<N2>,
    (DivNumerator<N1, D2>, DivDenominator<D1, N2>): GCD,
    Rational<DivNumerator<N1, D2>, DivDenominator<D1, N2>>: Reduce<<(DivNumerator<N1, D2>, DivDenominator<D1, N2>) as GCD>::Output>
{
    type Output = <Rational<DivNumerator<N1, D2>, DivDenominator<D1, N2>> as Reduce<<(DivNumerator<N1, D2>, DivDenominator<D1, N2>) as GCD>::Output>>::Output;

    fn div(self, _other: Rational<N2, D2>) -> Self::Output {
        unimplemented!()
    }
}

use typenum::{P1, P2, P3, P4, P6, P8};
use typenum::{N1, N3, N4};
use typenum::{Integer};

#[test]
fn test_rational_add() {
    type R1 = Rational<P1, P2>; // 1/2
    type R2 = Rational<P1, P4>; // 1/4
    type SumResult = <R1 as Add<R2>>::Output; // 3/4
    assert_eq!(<<SumResult as RationalNumber>::Numerator as Integer>::to_i32(), 3);
    assert_eq!(<<SumResult as RationalNumber>::Denominator as Integer>::to_i32(), 4);

    type R3 = Rational<P3, P8>; // 3/8
    type R4 = Rational<P1, P3>; // 1/3
    type SumResult2 = <R3 as Add<R4>>::Output; // 17/24
    assert_eq!(<<SumResult2 as RationalNumber>::Numerator as Integer>::to_i32(), 17);
    assert_eq!(<<SumResult2 as RationalNumber>::Denominator as Integer>::to_i32(), 24);

    type R5 = Rational<N1, P3>; // -1/3
    type R6 = Rational<N4, P6>; // -4/6
    type SumResult3 = <R5 as Add<R6>>::Output; // -1/1
    assert_eq!(<<SumResult3 as RationalNumber>::Numerator as Integer>::to_i32(), -1);
    assert_eq!(<<SumResult3 as RationalNumber>::Denominator as Integer>::to_i32(), 1);
}

#[test]
fn test_rational_sub() {
    type R1 = Rational<P1, P2>; // 1/2
    type R2 = Rational<P1, P4>; // 1/4
    type SubResult = <R1 as Sub<R2>>::Output; // 1/4
    assert_eq!(<<SubResult as RationalNumber>::Numerator as Integer>::to_i32(), 1);
    assert_eq!(<<SubResult as RationalNumber>::Denominator as Integer>::to_i32(), 4);

    type R3 = Rational<P3, P8>; // 3/8
    type R4 = Rational<P1, P3>; // 1/3
    type SubResult2 = <R3 as Sub<R4>>::Output; // 1/24
    assert_eq!(<<SubResult2 as RationalNumber>::Numerator as Integer>::to_i32(), 1);
    assert_eq!(<<SubResult2 as RationalNumber>::Denominator as Integer>::to_i32(), 24);

    type SubResult3 = <R2 as Sub<R1>>::Output; // -1/4
    assert_eq!(<<SubResult3 as RationalNumber>::Numerator as Integer>::to_i32(), -1);
    assert_eq!(<<SubResult3 as RationalNumber>::Denominator as Integer>::to_i32(), 4);

    type R5 = Rational<N1, P3>; // -1/3
    type R6 = Rational<N4, P6>; // -4/6
    type SubResult4 = <R5 as Sub<R6>>::Output; // 1/3
    assert_eq!(<<SubResult4 as RationalNumber>::Numerator as Integer>::to_i32(), 1);
    assert_eq!(<<SubResult4 as RationalNumber>::Denominator as Integer>::to_i32(), 3);
}

#[test]
fn test_rational_mul() {
    type R1 = Rational<P1, P2>; // 1/2
    type R2 = Rational<P1, P4>; // 1/4
    type MulResult = <R1 as Mul<R2>>::Output; // 1/8
    assert_eq!(<<MulResult as RationalNumber>::Numerator as Integer>::to_i32(), 1);
    assert_eq!(<<MulResult as RationalNumber>::Denominator as Integer>::to_i32(), 8);

    type R3 = Rational<P3, P8>; // 3/8
    type R4 = Rational<P1, P3>; // 1/3
    type MulResult2 = <R3 as Mul<R4>>::Output; // 1/8
    assert_eq!(<<MulResult2 as RationalNumber>::Numerator as Integer>::to_i32(), 1);
    assert_eq!(<<MulResult2 as RationalNumber>::Denominator as Integer>::to_i32(), 8);

    type R5 = Rational<N3, P8>; // -3/8
    type R6 = Rational<P1, P3>; // 1/3
    type MulResult3 = <R5 as Mul<R6>>::Output; // -1/8
    assert_eq!(<<MulResult3 as RationalNumber>::Numerator as Integer>::to_i32(), -1);
    assert_eq!(<<MulResult3 as RationalNumber>::Denominator as Integer>::to_i32(), 8);
}

#[test]
fn test_rational_div() {
    type R1 = Rational<P1, P2>; // 1/2
    type R2 = Rational<P1, P4>; // 1/4
    type DivResult = <R1 as Div<R2>>::Output; // 2/1
    assert_eq!(<<DivResult as RationalNumber>::Numerator as Integer>::to_i32(), 2);
    assert_eq!(<<DivResult as RationalNumber>::Denominator as Integer>::to_i32(), 1);

    type R3 = Rational<P3, P8>; // 3/8
    type R4 = Rational<P1, P3>; // 1/3
    type DivResult2 = <R3 as Div<R4>>::Output; // 9/8
    assert_eq!(<<DivResult2 as RationalNumber>::Numerator as Integer>::to_i32(), 9);
    assert_eq!(<<DivResult2 as RationalNumber>::Denominator as Integer>::to_i32(), 8);

    type R5 = Rational<N3, P8>; // -3/8
    type R6 = Rational<P1, P3>; // 1/3
    type DivResult3 = <R5 as Div<R6>>::Output; // -9/8
    assert_eq!(<<DivResult3 as RationalNumber>::Numerator as Integer>::to_i32(), -9);
    assert_eq!(<<DivResult3 as RationalNumber>::Denominator as Integer>::to_i32(), 8);
}
