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
}
