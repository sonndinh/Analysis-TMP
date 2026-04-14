// Rational numbers using typenum integers
use std::ops::{Add, Sub, Mul, Div, Rem};
use std::marker::PhantomData;
use typenum::Z0;
use typenum::{False, True, IsGreaterOrEqual, IsNotEqual};

pub trait If<Then, Else>
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

trait GCD
{
    type A;
    type B;
    type Output;
}

trait GCDBody<A, B>
{
    // type NextA;
    // type NextB;
    type Output;
}

impl<A, B> GCDBody<A, B> for False
{
    // We don't care about next A and B since we are stopping the recursion
    // type NextA = Z0;
    // type NextB = Z0;
    type Output = A;
}

type NextA<B> = B;
type NextB<A, B> = <A as Rem<B>>::Output;

impl<A, B> GCDBody<A, B> for True
where
    A: Rem<B>,
    NextA<B>: IsGreaterOrEqual<NextB<A, B>>,
    <NextA<B> as IsGreaterOrEqual<NextB<A, B>>>::Output: If<NextA<B>, NextB<A, B>>,
    <NextA<B> as IsGreaterOrEqual<NextB<A, B>>>::Output: If<NextB<A, B>, NextA<B>>
    // B: IsGreaterOrEqual<<A as Rem<B>>::Output>,
    // <B as IsGreaterOrEqual<<A as Rem<B>>::Output>>::Output: If<B, <A as Rem<B>>::Output>
{
    // type NextA = B;
    // type NextB = <A as Rem<B>>::Output;
    // type Output = <(Self::NextA, Self::NextB) as GCD>::Output;
    type Output = <(NextA<B>, NextB<A, B>) as GCD>::Output;
}

// TODO: Compute the GCD of two integers A and B
impl<X, Y> GCD for (X, Y)
where
    X: IsGreaterOrEqual<Y>,
    <X as IsGreaterOrEqual<Y>>::Output: If<X, Y>,
    <X as IsGreaterOrEqual<Y>>::Output: If<Y, X>,
    <<X as IsGreaterOrEqual<Y>>::Output as If<Y, X>>::Output: IsNotEqual<Z0>,

{
    // Set A and B such that A >= B
    type A = <<X as IsGreaterOrEqual<Y>>::Output as If<X, Y>>::Output;
    type B = <<X as IsGreaterOrEqual<Y>>::Output as If<Y, X>>::Output;
    type Output = <<Self::B as IsNotEqual<Z0>>::Output as GCDBody<Self::A, Self::B>>::Output;
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

use typenum::{P1, P2, P4};
use typenum::{Integer};

#[test]
fn test_rational_add() {
    type R1 = Rational<P1, P2>;
    type R2 = Rational<P1, P4>;

    type SumResult = <R1 as Add<R2>>::Output;
    println!("Numerator: {}", <<SumResult as RationalNumber>::Numerator as Integer>::to_i32());
    println!("Denominator: {}", <<SumResult as RationalNumber>::Denominator as Integer>::to_i32());

    type SubResult = <R1 as Sub<R2>>::Output;
    println!("Numerator: {}", <<SubResult as RationalNumber>::Numerator as Integer>::to_i32());
    println!("Denominator: {}", <<SubResult as RationalNumber>::Denominator as Integer>::to_i32());

    type MulResult = <R1 as Mul<R2>>::Output;
    println!("Numerator: {}", <<MulResult as RationalNumber>::Numerator as Integer>::to_i32());
    println!("Denominator: {}", <<MulResult as RationalNumber>::Denominator as Integer>::to_i32());

    type DivResult = <R1 as Div<R2>>::Output;
    println!("Numerator: {}", <<DivResult as RationalNumber>::Numerator as Integer>::to_i32());
    println!("Denominator: {}", <<DivResult as RationalNumber>::Denominator as Integer>::to_i32());
}
