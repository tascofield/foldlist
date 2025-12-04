use std::marker::PhantomData;

/// A custom version of `Fn(A)->B`. Every `Fn(A)->B` is also a `Fun<A,B>`.
pub trait Fun<A,B> {
    /// Apply this Fun to `a`
    fn apply(&self,a: A) -> B;
}

impl<A,B,F: Fn(A)->B> Fun<A,B> for F {
    fn apply(&self,a: A) -> B {
        self(a)
    }
}

/// A wrapper around a [`Fun`] that wraps its output in a [`Some`].
#[derive(Clone,Copy)]
pub struct SomeFun<F>(pub F);
impl<A,B,F: Fun<A,B>> Fun<A,Option<B>> for SomeFun<F> {
    fn apply(&self,a: A) -> Option<B> {
        Some(self.0.apply(a))
    }
}

/// A wrapper around a `Fn()->T` that converts it to a [`Fun<(),T>`]
#[derive(Clone,Copy)]
pub struct EmptyFn<F>(pub F);
impl<T,F: Fn()->T> Fun<(),T> for EmptyFn<F> {
    fn apply(&self,_a: ()) -> T {
        self.0()
    }
}

/// A wrapper around a `Fn(A,B)->C` that converts it to [`Fun<(A,B),C>`]
#[derive(Clone,Copy)]
pub struct TupleFun<F>(pub F);
impl<A,B,C,F: Fn(A,B)->C> Fun<(A,B),C> for TupleFun<F> {
    fn apply(&self,(a,b): (A,B)) -> C {
        self.0(a,b)
    }
}

/// A struct that composes a [`Fun<A,B>`] with a [`Fun<&B,C>`] into a [`Fun<A,C>`]
pub struct ComposeRefFn<F,G,B>(pub F,pub G,pub PhantomData<B>);

impl<A,B,C,G: Fun<A,B>, F: for<'b> Fun<&'b B,C>> Fun<A,C> for ComposeRefFn<F,G,B> {
    fn apply(&self,a: A) -> C {
        self.0.apply(&self.1.apply(a))
    }
}
impl<F: Clone, G: Clone, B> Clone for ComposeRefFn<F, G, B> {
    fn clone(&self) -> Self {
        Self(self.0.clone(), self.1.clone(), self.2.clone())
    }
}
impl<F: Copy, G: Copy, B> Copy for ComposeRefFn<F, G, B> {}

/// A wrapper around a [`Fun<(A,A),A>`] that converts it to [`Fun<(Option<A>,Option<A>),Option<A>>`]
#[derive(Clone,Copy)]
pub struct OptOpFun<F>(pub F);
impl<A,F: Fun<(A,A),A>> Fun<(Option<A>,Option<A>),Option<A>> for OptOpFun<F> {
    fn apply(&self,a: (Option<A>,Option<A>)) -> Option<A> {
        match a {
            (None, r) |
            (r, None) => r,
            (Some(a), Some(b)) => Some(self.0.apply((a,b))),
        }
    }
}

/// A [Fun] which always returns [None]
#[derive(Clone,Copy)]
pub struct NoneFun;
impl<T> Fun<(),Option<T>> for NoneFun {
    fn apply(&self,_: ()) -> Option<T> {
        None
    }
}

/// A trait for range literals of the form `..x` or `x..`
pub trait SingleEndedRange<T> {
    /// A [type-level boolean](Bool) which is [True] for `x..` and [False] for `..x`
    type EndIsLeft: Bool;
    /// The endpoint in question.
    fn end(self) -> T;
}

impl<T> SingleEndedRange<T> for core::ops::RangeFrom<T> {
    type EndIsLeft = True;
    fn end(self) -> T {
        self.start
    }
}

impl<T> SingleEndedRange<T> for core::ops::RangeTo<T> {
    type EndIsLeft = False;
    fn end(self) -> T {
        self.end
    }
}

/// A trait for type-level booleans, used internally to implement slice types more efficiently.
pub trait Bool: 'static + Sized {
    #![allow(missing_docs)]
    type IfElse<A,B>;
    type And<B: Bool> : Bool;
    type Not : Bool;
    
    #[allow(non_upper_case_globals)]
    const b: bool;

    fn init_if_else<A,B,X,IF: FnOnce(X)->A, ELSE: FnOnce(X)->B>(x: X, true_case: IF, false_case: ELSE) -> Self::IfElse<A,B>;
    fn map_cases<A,B,A2,B2, IF: FnOnce(A)->A2, ELSE: FnOnce(B)->B2>(ab: Self::IfElse<A,B>, true_case: IF, false_case: ELSE) -> Self::IfElse<A2,B2>;
    fn close_if_else<A,B,IF: FnOnce(A) -> R, ELSE: FnOnce(B)->R,R>(ab: Self::IfElse<A,B>,true_case: IF, false_case: ELSE) -> R;
    fn close_if_else_ref<A,B,IF: FnOnce(&A) -> R, ELSE: FnOnce(&B)->R,R>(ab: &Self::IfElse<A,B>,true_case: IF, false_case: ELSE) -> R;
    fn close_if_else_mut<'a,A: 'a,B: 'a,IF: FnOnce(&'a mut A) -> R, ELSE: FnOnce(&'a mut B)->R,R>(ab: &'a mut Self::IfElse<A,B>,true_case: IF, false_case: ELSE) -> R;
    
    fn assert_unwrap<A,B>(x: Self::IfElse<A,B>) -> A;
    fn assert_false_unwrap<A,B>(x: Self::IfElse<A,B>) -> B;
    fn assert_unwrap_ref<A,B>(x: &Self::IfElse<A,B>) -> &A;
    fn assert_false_unwrap_ref<A,B>(x: &Self::IfElse<A,B>) -> &B;
    fn assert_unwrap_mut<A,B>(x: &mut Self::IfElse<A,B>) -> &mut A;
    fn assert_false_unwrap_mut<A,B>(x: &mut Self::IfElse<A,B>) -> &mut B;
    
    fn assert_init<A,B>(a: A) -> Self::IfElse<A,B>;
    fn assert_false_init<A,B>(b: B) -> Self::IfElse<A,B>;
    
    fn as_ref<'x,A,B>(x: &'x Self::IfElse<A,B>) -> Self::IfElse<&'x A,&'x B>;
    fn as_mut<'x,A,B>(x: &'x mut Self::IfElse<A,B>) -> Self::IfElse<&'x mut A,&'x mut B>;
    
    fn and_true<A,B>(x: Self::IfElse<A,B>) -> <Self::And<True>as Bool>::IfElse<A,B>;
    fn and_false<A,B>(b: B) -> <Self::And<False> as Bool>::IfElse<A,B>;
    fn inc_not<A,B>(x: Self::IfElse<A,B>) -> <Self::Not as Bool>::IfElse<B,A>;
    fn dec_not<A,B>(x: <Self::Not as Bool>::IfElse<A,B>) -> Self::IfElse<B,A>;
    
    fn commute_and<B: Bool,C,D>(and: <Self::And<B> as Bool>::IfElse<C,D>) -> <B::And<Self> as Bool>::IfElse<C,D>;
    
    fn decomp_and<B: Bool, C,D>(and: <Self::And<B> as Bool>::IfElse<C,D>) -> Self::IfElse<B::IfElse<C,D>,D>;
    fn recomp_and<B: Bool, C,D>(and: Self::IfElse<B::IfElse<C,D>,D>) -> <Self::And<B> as Bool>::IfElse<C,D>;
} 

/// The [type-level boolean](Bool) value of true
pub struct True;
impl Bool for True {
    type IfElse<A,B> = A;
    type And<B: Bool> = B;
    type Not = False;
    const b: bool = true;
    fn close_if_else<A,B,IF: FnOnce(A) -> R, ELSE: FnOnce(B)->R,R>(ab: Self::IfElse<A,B>, true_case: IF, _false_case: ELSE) -> R {
        true_case(ab)
    }
    fn map_cases<A,B,A2,B2, IF: FnOnce(A)->A2, ELSE: FnOnce(B)->B2>(ab: Self::IfElse<A,B>, true_case: IF, _false_case: ELSE) -> Self::IfElse<A2,B2> {
        true_case(ab)
    }
    fn init_if_else<A,B,X,IF: FnOnce(X)->A, ELSE: FnOnce(X)->B>(x: X, true_case: IF, _false_case: ELSE) -> Self::IfElse<A,B> {
        true_case(x)
    }
    fn assert_unwrap<A,B>(x: Self::IfElse<A,B>) -> A {
        x
    }
    fn assert_false_unwrap<A,B>(_x: Self::IfElse<A,B>) -> B {
        panic!()
    }
    fn close_if_else_ref<A,B,IF: FnOnce(&A) -> R, ELSE: FnOnce(&B)->R,R>(ab: &Self::IfElse<A,B>,true_case: IF, _false_case: ELSE) -> R {
        true_case(ab)
    }
    fn close_if_else_mut<'a,A: 'a,B: 'a,IF: FnOnce(&'a mut A) -> R, ELSE: FnOnce(&'a mut B)->R,R>(ab: &'a mut Self::IfElse<A,B>,true_case: IF, _false_case: ELSE) -> R {
        true_case(ab)
    }
    fn assert_unwrap_ref<A,B>(x: &Self::IfElse<A,B>) -> &A {
        x
    }
    fn assert_false_unwrap_ref<A,B>(_x: &Self::IfElse<A,B>) -> &B {
        panic!()
    }
    fn assert_unwrap_mut<A,B>(x: &mut Self::IfElse<A,B>) -> &mut A {
        x
    }
    fn assert_false_unwrap_mut<A,B>(_x: &mut Self::IfElse<A,B>) -> &mut B {
       panic!()
    }
    fn assert_init<A,B>(a: A) -> Self::IfElse<A,B> {
        a
    }
    fn assert_false_init<A,B>(_b: B) -> Self::IfElse<A,B> {
        panic!()
    }
    fn as_ref<A,B>(x: &Self::IfElse<A,B>) -> Self::IfElse<&A,&B> {
        x
    }
    fn as_mut<A,B>(x: &mut Self::IfElse<A,B>) -> Self::IfElse<&mut A,&mut B> {
        x
    }
    fn and_true<A,B>(x: Self::IfElse<A,B>) -> <Self::And<True>as Bool>::IfElse<A,B> {
        x
    }
    fn and_false<A,B>(b: B) -> <Self::And<False> as Bool>::IfElse<A,B> {
        b
    }
    fn inc_not<A,B>(x: Self::IfElse<A,B>) -> <Self::Not as Bool>::IfElse<B,A> {
        x
    }
    fn dec_not<A,B>(x: <Self::Not as Bool>::IfElse<A,B>) -> Self::IfElse<B,A> {
        x
    }
    fn commute_and<B: Bool,C,D>(and: <Self::And<B> as Bool>::IfElse<C,D>) -> <B::And<Self> as Bool>::IfElse<C,D> {
        B::and_true(and)
    }
    fn decomp_and<B: Bool, C,D>(and: <Self::And<B> as Bool>::IfElse<C,D>) -> Self::IfElse<B::IfElse<C,D>,D> {
        and
    }
    fn recomp_and<B: Bool, C,D>(and: Self::IfElse<B::IfElse<C,D>,D>) -> <Self::And<B> as Bool>::IfElse<C,D> {
        and
    }
}

/// The [type-level boolean](Bool) value of false
pub struct False;
impl Bool for False {
    type IfElse<A,B> = B;
    type And<B: Bool> = False;
    type Not = True;
    const b: bool = false;
    fn close_if_else<A,B,IF: FnOnce(A) -> R, ELSE: FnOnce(B)->R,R>(ab: Self::IfElse<A,B>,_true_case: IF, false_case: ELSE) -> R {
        false_case(ab)
    }
    fn map_cases<A,B,A2,B2, IF: FnOnce(A)->A2, ELSE: FnOnce(B)->B2>(ab: Self::IfElse<A,B>, _true_case: IF, false_case: ELSE) -> Self::IfElse<A2,B2> {
        false_case(ab)
    }
    fn init_if_else<A,B,X,IF: FnOnce(X)->A, ELSE: FnOnce(X)->B>(x: X, _true_case: IF, false_case: ELSE) -> Self::IfElse<A,B> {
        false_case(x)
    }
    fn assert_unwrap<A,B>(_x: Self::IfElse<A,B>) -> A {
        panic!()
    }
    fn assert_false_unwrap<A,B>(x: Self::IfElse<A,B>) -> B {
        x
    }
    fn close_if_else_ref<A,B,IF: FnOnce(&A) -> R, ELSE: FnOnce(&B)->R,R>(ab: &Self::IfElse<A,B>,_true_case: IF, false_case: ELSE) -> R {
        false_case(ab)
    }
    fn close_if_else_mut<'a,A: 'a,B: 'a,IF: FnOnce(&'a mut A) -> R, ELSE: FnOnce(&'a mut B)->R,R>(ab: &'a mut Self::IfElse<A,B>,_true_case: IF, false_case: ELSE) -> R {
        false_case(ab)
    }
    fn assert_unwrap_ref<A,B>(_x: &Self::IfElse<A,B>) -> &A {
        panic!()
    }
    fn assert_false_unwrap_ref<A,B>(x: &Self::IfElse<A,B>) -> &B {
        x
    }
    fn assert_unwrap_mut<A,B>(_x: &mut Self::IfElse<A,B>) -> &mut A {
        panic!()
    }
    fn assert_false_unwrap_mut<A,B>(x: &mut Self::IfElse<A,B>) -> &mut B {
        x
    }
    fn assert_init<A,B>(_a: A) -> Self::IfElse<A,B> {
        panic!()
    }
    fn assert_false_init<A,B>(b: B) -> Self::IfElse<A,B> {
        b
    }
    fn as_ref<A,B>(x: &Self::IfElse<A,B>) -> Self::IfElse<&A,&B> {
        x
    }
    fn as_mut<A,B>(x: &mut Self::IfElse<A,B>) -> Self::IfElse<&mut A,&mut B> {
        x
    }
    fn and_true<A,B>(x: Self::IfElse<A,B>) -> <Self::And<True>as Bool>::IfElse<A,B> {
        x
    }
    fn and_false<A,B>(b: B) -> <Self::And<False> as Bool>::IfElse<A,B> {
        b
    }
    fn inc_not<A,B>(x: Self::IfElse<A,B>) -> <Self::Not as Bool>::IfElse<B,A> {
        x
    }
    fn dec_not<A,B>(x: <Self::Not as Bool>::IfElse<A,B>) -> Self::IfElse<B,A> {
        x
    }
    fn commute_and<B: Bool,C,D>(and: D) -> <B::And<False> as Bool>::IfElse<C,D> {
        B::and_false(and)
    }
    fn decomp_and<B: Bool, C,D>(and: D) -> D {
        and
    }
    fn recomp_and<B: Bool, C,D>(and: D) -> D {
        and
    }
}

pub(crate) fn bool_ifelse_clone<B: Bool, X: Clone, Y: Clone>(xy: &B::IfElse<X,Y>) -> B::IfElse<X,Y> {
    let xy_ref = B::as_ref(xy);
    let xy_cloned = B::map_cases(xy_ref, |x| x.clone(), |y| y.clone());
    xy_cloned
}

pub(crate) fn bool_assert_into<A: Bool, B: Bool, X,Y>(xy: A::IfElse<X,Y>) -> B::IfElse<X,Y> {
    return B::init_if_else(xy, 
        |xy| A::assert_unwrap(xy), 
        |xy| A::assert_false_unwrap(xy)
    );
}

#[inline(always)]
pub(crate) fn cswap<B: Bool,T>(a: T, b: T) -> (T,T) {
    if B::b {
        (b,a)
    } else {
        (a,b)
    }
}