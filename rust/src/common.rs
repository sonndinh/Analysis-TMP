// Integer representation:
// Zero for 0.
// Succ<Zero> for 1, Succ<Succ<Zero>> for 2, and so on.
// Pred<Zero> for -1, Pred<Pred<Zero>>  for -2, and so on.
// For simplicity, each nunber has a unique type representation.
// For example, 1 is represented by Succ<Zero> and not Pred<Succ<Succ<Zero>>>.

// Comparison relationships:
// Zero < Succ<P>, where P is a type representing 0 or a positive number.
// Succ<A> <= Succ<B> if A <= B.
// Pred<N> < Zero, where N is a type representing 0 or a negative number.
// Pred<A> <= Pred<B> if A <= B.
struct Zero;
struct Succ<P>(P);
struct Pred<N>(N);

// Get value
trait ToValue {
    const VALUE: i32;
}

impl ToValue for Zero {
    const VALUE: i32 = 0;
}

impl<A: ToValue> ToValue for Succ<A> {
    const VALUE: i32 = A::VALUE + 1;
}

impl<A: ToValue> ToValue for Pred<A> {
    const VALUE: i32 = A::VALUE - 1;
}

trait Equality {}

struct EQ;
#[allow(dead_code)]
impl EQ {
    const fn to_string() -> &'static str {
        "=="
    }
}

struct LT;
#[allow(dead_code)]
impl LT {
    const fn to_string() ->&'static str {
        "<"
    }
}

struct GT;
#[allow(dead_code)]
impl GT {
    const fn to_string() -> &'static str {
        ">"
    }
}

impl Equality for EQ {}
impl Equality for LT {}
impl Equality for GT {}

// Compare operator
trait Cmp<A> {
    type Output: Equality;
}

impl Cmp<Zero> for Zero {
    type Output = EQ;
}

impl<P> Cmp<Succ<P>> for Zero {
    type Output = LT;
}

impl<P> Cmp<Zero> for Succ<P> {
    type Output = GT;
}

impl<N> Cmp<Pred<N>> for Zero {
    type Output = GT;
}

impl<N> Cmp<Zero> for Pred<N> {
    type Output = LT;
}

// Compare positive numbers
impl<A: Cmp<B>, B> Cmp<Succ<B>> for Succ<A> {
    type Output = <A as Cmp<B>>::Output;
}

// Compare negative numbers
impl<A: Cmp<B>, B> Cmp<Pred<B>> for Pred<A> {
    type Output = <A as Cmp<B>>::Output;
}

// Compare positive and negative numbers
impl<P, N> Cmp<Pred<N>> for Succ<P> {
    type Output = GT;
}

impl<N, P> Cmp<Succ<P>> for Pred<N> {
    type Output = LT;
}

// Maximum operator
trait Max<A> {
    type Output;
}

impl Max<Zero> for Zero {
    type Output = Zero;
}

impl<P> Max<Succ<P>> for Zero {
    type Output = Succ<P>;
}

impl<P> Max<Zero> for Succ<P> {
    type Output = Succ<P>;
}

impl<N> Max<Pred<N>> for Zero {
    type Output = Zero;
}

impl<N> Max<Zero> for Pred<N> {
    type Output = Zero;
}

impl<A: Max<B>, B> Max<Succ<B>> for Succ<A> {
    type Output = Succ<<A as Max<B>>::Output>;
}

impl<A: Max<B>, B> Max<Pred<B>> for Pred<A> {
    type Output = Pred<<A as Max<B>>::Output>;
}

impl<N, P> Max<Succ<P>> for Pred<N> {
    type Output = Succ<P>;
}

impl<P, N> Max<Pred<N>> for Succ<P> {
    type Output = Succ<P>;
}

#[allow(dead_code)]
type TypeMax<A, B> = <A as Max<B>>::Output;

#[test]
fn test_cmp() {
    assert!(Succ::<Zero>::VALUE > Zero::VALUE);
    assert!(Succ::<Succ<Zero>>::VALUE > Succ::<Zero>::VALUE);
    assert!(Succ::<Zero>::VALUE == Succ::<Zero>::VALUE);
    assert!(Succ::<Zero>::VALUE < Succ::<Succ<Zero>>::VALUE);
    assert!(Pred::<Zero>::VALUE < Zero::VALUE);
    assert!(Zero::VALUE > Pred::<Zero>::VALUE);
    assert!(Pred::<Pred<Zero>>::VALUE < Pred::<Zero>::VALUE);
    assert!(Pred::<Zero>::VALUE > Pred::<Pred<Zero>>::VALUE);
    assert!(Succ::<Zero>::VALUE > Pred::<Zero>::VALUE);
    assert!(Pred::<Zero>::VALUE < Succ::<Zero>::VALUE);
}

#[test]
fn test_max() {
    assert_eq!(<TypeMax<Zero, Succ<Zero>> as ToValue>::VALUE, 1);
    assert_eq!(<TypeMax<Succ<Zero>, Zero> as ToValue>::VALUE, 1);
    assert_eq!(<TypeMax<Succ<Zero>, Succ<Succ<Zero>>> as ToValue>::VALUE, 2);
    assert_eq!(<TypeMax<Succ<Succ<Succ<Zero>>>, Zero> as ToValue>::VALUE, 3);

    assert_eq!(<TypeMax<Zero, Pred<Zero>> as ToValue>::VALUE, 0);
    assert_eq!(<TypeMax<Pred<Zero>, Pred<Pred<Zero>>> as ToValue>::VALUE, -1);
    assert_eq!(<TypeMax<Pred<Pred<Pred<Zero>>>, Pred<Pred<Zero>>> as ToValue>::VALUE, -2);

    assert_eq!(<TypeMax<Succ<Zero>, Pred<Zero>> as ToValue>::VALUE, 1);
    assert_eq!(<TypeMax<Pred<Pred<Zero>>, Succ<Succ<Zero>>> as ToValue>::VALUE, 2);
}

// Rational numbers using typenum integers
use std::ops::{Add, Sub, Mul, Div};
use std::marker::PhantomData;

trait GCD
{
    type Output;
}

// TODO: Compute the GCD of two integers A and B
impl<A, B> GCD for (A, B)
{
    type Output = P1;
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
