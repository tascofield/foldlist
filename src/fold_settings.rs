use core::marker::PhantomData;

use crate::misc::Fun;

/// The trait for types which tell [`FoldChain`](crate::fold_chain::FoldChain)s and [`FoldList`](crate::fold_list::FoldList)s how to perform their folds. See [Fold Settings](crate#fold-settings)
pub trait FoldSettings<T,D> : Copy {
    /// Performs the fold operation
    fn op(&self, a: D, b: D) -> D;

    /// Gets the delta of an element
    fn delta_of(&self, t: &T) -> D;

    /// Creates a new empty delta
    fn empty(&self) -> D;
}

/// A struct which implements [`FoldSettings`] via three closures, which can be [named](crate#nameable-type).
pub struct FoldSettingsStruct<T,D,OP : Fun<(D,D),D> + Copy, T2D: for<'a> Fun<&'a T,D> + Copy,EMPTY: Fun<(),D> + Copy> {
    /// The closure for [`FoldSettings::op`]
    pub op_closure: OP,

    /// The closure for [`FoldSettings::delta_of`]
    pub t2d_closure: T2D,

    /// The closure for [`FoldSettings::delta_of`]
    pub empty_closure: EMPTY,

    #[allow(missing_docs)]
    pub _m: PhantomData<fn(T,D)->D>
}

impl<T, D, OP: Fun<(D,D),D> + Copy, T2D: for<'a> Fun<&'a T,D> + Copy, EMPTY: Fun<(),D> + Copy> Clone for FoldSettingsStruct<T, D, OP, T2D, EMPTY> {
    fn clone(&self) -> Self {
        Self { op_closure: self.op_closure.clone(), t2d_closure: self.t2d_closure.clone(), empty_closure: self.empty_closure.clone(), _m: self._m.clone() }
    }
}
impl<T, D, OP: Fun<(D,D),D> + Copy, T2D: for<'a> Fun<&'a T,D> + Copy, EMPTY: Fun<(),D> + Copy> Copy for FoldSettingsStruct<T, D, OP, T2D, EMPTY> {}
impl<T, D, OP: Fun<(D,D),D> + Copy, T2D: for<'a> Fun<&'a T,D> + Copy, EMPTY: Fun<(),D> + Copy> FoldSettings<T,D> for FoldSettingsStruct<T, D, OP, T2D, EMPTY> {
    fn op(&self, a: D, b: D) -> D {
        self.op_closure.apply((a,b))
    }
    fn delta_of(&self, t: &T) -> D {
        self.t2d_closure.apply(t)
    }
    fn empty(&self) -> D {
        self.empty_closure.apply(())
    }
}

/// A wrapper around a [`FoldSettings`] value which causes it to also keep track of size, by replacing its delta type, `D`, with `(usize,D)`.
/// See [FoldChain](crate#foldchain).
/// 
/// The `usize` part of the delta behaves as follows:
///   * `op` adds sizes
///   * `delta_of` is always `1`, since each element is one element
///   * `empty` is always `0`, the additive identity
#[derive(Clone,Copy)]
pub struct SettingsWithSize<S>(pub S);
impl<T,D: Clone, S: FoldSettings<T,D>> FoldSettings<T,(usize,D)> for SettingsWithSize<S> {
    fn op(&self, (n,a): (usize,D), (m,b): (usize,D)) -> (usize,D) {
        (n + m, self.0.op(a,b))
    }
    fn delta_of(&self, t: &T) -> (usize,D) {
        (1,self.0.delta_of(t))
    }
    fn empty(&self) -> (usize,D) {
        (0,self.0.empty())
    }
}