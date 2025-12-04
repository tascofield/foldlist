use std::marker::PhantomData;

use crate::{fold_settings::FoldSettings, misc::{Bool, ComposeRefFn, Fun}};

/// The trait for [simplifications](crate#simplification)
pub trait FoldSimplification<T,D: Clone> : Clone  + Copy {
    /// The new, simplified version of `D`
    type D2: Clone;

    /// Apply the original operation, under this simplification
    fn op(&self, a: Self::D2, b: Self::D2, settings: impl FoldSettings<T,D>) -> Self::D2;

    /// Get the simplified version of a `D`
    fn simplify(&self, delta: &D) -> Self::D2;

    /// Get the simplified version of the empty `D`
    fn empty(&self, settings: impl FoldSettings<T,D>) -> Self::D2;

    /// Get the simplified version of the `D` of a `T`
    fn delta_of(&self, value: &T, settings: impl FoldSettings<T,D>) -> Self::D2;

    /// The type of the simplification that results from applying another simplification after this one
    type Compose<D3: Clone, 
        Simplifier: for<'x> Fun<&'x Self::D2,D3> + Copy,
        OP2: Fun<(D3,D3),D3> + Copy> 
            : FoldSimplification<T,D,D2 = D3>;
    /// Apply another simplification after this one
    fn compose<D3: Clone, 
        Simplifier: for<'x> Fun<&'x Self::D2,D3> + Copy,
        OP2: Fun<(D3,D3),D3> + Copy>(self, simplifier: Simplifier, op2: OP2) -> Self::Compose<D3,Simplifier,OP2>;

    /// The type of the simplification that results from applying another simplification after this one, with a shortcut
    type ComposeWithShortcut<D3: Clone, 
        Simplifier: for<'x> Fun<&'x Self::D2,D3> + Copy,
        OP2: Fun<(D3,D3),D3> + Copy,
        EmptyShortcut: Fun<(),D3> + Copy,
        DeltaShortcut: for<'x> Fun<&'x T, D3> + Copy>
            : FoldSimplification<T,D,D2 = D3>;
    /// Apply another simplification after this one, with a shortcut
    fn compose_with_shortcut<D3: Clone, 
        Simplifier: for<'x> Fun<&'x Self::D2,D3> + Copy,
        OP2: Fun<(D3,D3),D3> + Copy,
        EmptyShortcut: Fun<(),D3> + Copy,
        DeltaShortcut: for<'x> Fun<&'x T, D3> + Copy>(self, simplifier: Simplifier, op2: OP2,empty_shortcut: EmptyShortcut, delta_shortcut: DeltaShortcut) -> Self::ComposeWithShortcut<D3,Simplifier,OP2,EmptyShortcut,DeltaShortcut>;

    /// The type of the simplification that results from applying this simplification after another (explicit) one
    type ComposeAfterOther<D0: Clone,Other: FoldSimplification<T,D0,D2=D>> : FoldSimplification<T,D0,D2=Self::D2>;
    /// Apply this simplification after another (explicit) one
    fn compose_after_other<D0: Clone,Other: FoldSimplification<T,D0,D2=D>>(self,other: Other) -> Self::ComposeAfterOther<D0,Other>;

    /// The type of the version of this simplification that also keeps track of size
    type WithSize : FoldSimplification<T,(usize,D),D2=(usize,Self::D2)>;
    /// Get the version of this simplification that also keeps track of size
    fn with_size(self) -> Self::WithSize;

    /// Call `op`, but reverse the order of its inputs if `Reversed` is [`True`](crate::misc::True)
    fn op_cswap<Reversed: Bool>(&self, a: Self::D2, b: Self::D2,settings: impl FoldSettings<T,D>) -> Self::D2 {
        if Reversed::b {
            self.op(b,a,settings)
        } else {
            self.op(a,b,settings)
        }
    }
}

impl<T,D: Clone> FoldSimplification<T, D> for () {
    type D2 = D;
    fn op(&self, a: Self::D2, b: Self::D2, settings: impl FoldSettings<T,D>) -> Self::D2 {
        settings.op(a,b)
    }
    fn simplify(&self, delta: &D) -> Self::D2 {
        delta.clone()
    }
    fn empty(&self, settings: impl FoldSettings<T,D>) -> Self::D2 {
        settings.empty()
    }
    fn delta_of(&self, value: &T, settings: impl FoldSettings<T,D>) -> Self::D2 {
        settings.delta_of(value)
    }

    type Compose<D3: Clone, 
        Simplifier: for<'x> Fun<&'x Self::D2,D3> + Copy,
        OP2: Fun<(D3,D3),D3> + Copy> 
            = SimplificationWithoutShortcut<T,D,D3,Simplifier,OP2>;
    
    fn compose<D3: Clone, 
        Simplifier: for<'x> Fun<&'x Self::D2,D3> + Copy,
        OP2: Fun<(D3,D3),D3> + Copy>(self, simplifier: Simplifier, op2: OP2) -> Self::Compose<D3,Simplifier,OP2> {
        SimplificationWithoutShortcut { simplifier, op2, _m: PhantomData }
    }

    type ComposeWithShortcut<D3: Clone, 
        Simplifier: for<'x> Fun<&'x Self::D2,D3> + Copy,
        OP2: Fun<(D3,D3),D3> + Copy,
        EmptyShortcut: Fun<(),D3> + Copy,
        DeltaShortcut: for<'x> Fun<&'x T, D3> + Copy>
            = SimplificationWithShortcut<T,D,D3,Simplifier,OP2,EmptyShortcut,DeltaShortcut>;
            

    fn compose_with_shortcut<D3: Clone, 
        Simplifier: for<'x> Fun<&'x Self::D2,D3> + Copy,
        OP2: Fun<(D3,D3),D3> + Copy,
        EmptyShortcut: Fun<(),D3> + Copy,
        DeltaShortcut: for<'x> Fun<&'x T, D3> + Copy>(self, simplifier: Simplifier, op2: OP2,empty_shortcut: EmptyShortcut, delta_shortcut: DeltaShortcut) -> Self::ComposeWithShortcut<D3,Simplifier,OP2,EmptyShortcut,DeltaShortcut> {
        SimplificationWithShortcut{ simplifier, op2, empty_shortcut, delta_shortcut, _m: PhantomData }
    }
    
    type ComposeAfterOther<D0: Clone,Other: FoldSimplification<T,D0,D2=D>> = Other;
    fn compose_after_other<D0: Clone,Other: FoldSimplification<T,D0,D2=D>>(self,other: Other) -> Self::ComposeAfterOther<D0,Other> {
        other
    }
    
    type WithSize = ();
    fn with_size(self) -> Self::WithSize {}
}

/// A struct which implements [`FoldSimplification`] via two closures, which can be [named](crate#nameable-type).
pub struct SimplificationWithoutShortcut<T,D: Clone, D2: Clone, 
    Simplifier: for<'x> Fun<&'x D,D2> + Copy, 
    OP2: Fun<(D2,D2),D2> + Copy> {
    /// The closure for [`FoldSimplification::simplify`]
    pub simplifier: Simplifier,
    /// The closure for [`FoldSimplification::op`]
    pub op2: OP2,
    #[allow(missing_docs)]
    pub _m: PhantomData<(fn(&D)->D2,fn(&T)->D2)>
}

impl<T, D: Clone, D2: Clone, Simplifier: for<'x> Fun<&'x D,D2> + Copy, OP2: Fun<(D2,D2),D2> + Copy> Copy for SimplificationWithoutShortcut<T, D, D2, Simplifier, OP2> {}
impl<T, D: Clone, D2: Clone, Simplifier: for<'x> Fun<&'x D,D2> + Copy, OP2: Fun<(D2,D2),D2> + Copy> 
Clone for SimplificationWithoutShortcut<T, D, D2, Simplifier, OP2> {
    fn clone(&self) -> Self {
        Self { simplifier: self.simplifier.clone(), op2: self.op2.clone(), _m: self._m.clone() }
    }
}

impl<T, D: Clone, D2: Clone, Simplifier: for<'x> Fun<&'x D,D2> + Copy, OP2: Fun<(D2,D2),D2> + Copy> 
FoldSimplification<T,D> for SimplificationWithoutShortcut<T, D, D2, Simplifier, OP2> {
    type D2 = D2;
    fn op(&self, a: Self::D2, b: Self::D2, _: impl FoldSettings<T,D>) -> Self::D2 {
        self.op2.apply((a,b))
    }
    fn simplify(&self, delta: &D) -> Self::D2 {
        self.simplifier.apply(delta)
    }
    fn empty(&self, settings: impl FoldSettings<T,D>) -> Self::D2 {
        self.simplifier.apply(&settings.empty())
    }
    fn delta_of(&self, value: &T, settings: impl FoldSettings<T,D>) -> Self::D2 {
        self.simplifier.apply(&settings.delta_of(value))
    }
    type Compose<D3: Clone, 
        Simplifier2: for<'x> Fun<&'x Self::D2,D3> + Copy,
        OP3: Fun<(D3,D3),D3> + Copy> 
            = SimplificationWithoutShortcut<T,D,D3,ComposeRefFn<Simplifier2,Simplifier,D2>,OP3>;

    fn compose<D3: Clone, 
        Simplifier2: for<'x> Fun<&'x Self::D2,D3> + Copy,
        OP3: Fun<(D3,D3),D3> + Copy>(self, simplifier2: Simplifier2, op3: OP3) -> Self::Compose<D3,Simplifier2,OP3> {
        SimplificationWithoutShortcut {
            simplifier: ComposeRefFn(simplifier2,self.simplifier,PhantomData),
            op2: op3,
            _m: PhantomData,
        }
    }

    type ComposeWithShortcut<D3: Clone, 
        Simplifier2: for<'x> Fun<&'x Self::D2,D3> + Copy,
        OP3: Fun<(D3,D3),D3> + Copy,
        EmptyShortcut: Fun<(),D3> + Copy,
        DeltaShortcut: for<'x> Fun<&'x T, D3> + Copy>
            = SimplificationWithShortcut<T,D,D3,
                ComposeRefFn<Simplifier2,Simplifier,D2>,
                OP3,
                EmptyShortcut,
                DeltaShortcut>;
            

    fn compose_with_shortcut<D3: Clone, 
        Simplifier2: for<'x> Fun<&'x Self::D2,D3> + Copy,
        OP3: Fun<(D3,D3),D3> + Copy,
        EmptyShortcut: Fun<(),D3> + Copy,
        DeltaShortcut: for<'x> Fun<&'x T, D3> + Copy>(self, simplifier: Simplifier2, op2: OP3, empty_shortcut: EmptyShortcut, delta_shortcut: DeltaShortcut) -> Self::ComposeWithShortcut<D3,Simplifier2,OP3,EmptyShortcut,DeltaShortcut> {
        SimplificationWithShortcut { 
            simplifier: ComposeRefFn(simplifier, self.simplifier, PhantomData), 
            op2, 
            empty_shortcut, 
            delta_shortcut, 
            _m: PhantomData 
        }
    }
    
    type ComposeAfterOther<D0: Clone,Other: FoldSimplification<T,D0,D2=D>> = Other::Compose<D2,Simplifier,OP2>;
    fn compose_after_other<D0: Clone,Other: FoldSimplification<T,D0,D2=D>>(self,other: Other) -> Self::ComposeAfterOther<D0,Other> {
        other.compose(self.simplifier,self.op2)
    }
    
    type WithSize = SimplificationWithoutShortcut<T,(usize,D),(usize,D2),KeepSizeAnd<Simplifier>,AddSizesAnd<OP2>>;
    fn with_size(self) -> Self::WithSize {
        SimplificationWithoutShortcut {
            simplifier: KeepSizeAnd(self.simplifier),
            op2: AddSizesAnd(self.op2),
            _m: PhantomData,
        }
    }
}

/// A struct which implements [`FoldSimplification`] via four closures, which can be [named](crate#nameable-type).
pub struct SimplificationWithShortcut<T,D: Clone, D2: Clone, 
    Simplifier: for<'x> Fun<&'x D,D2> + Copy, 
    OP2: Fun<(D2,D2),D2> + Copy,
    EmptyShortcut: Fun<(),D2> + Copy,
    DeltaShortcut: for<'x> Fun<&'x T, D2> + Copy> {
        /// The closure for [`FoldSimplification::simplify`]
        pub simplifier: Simplifier,
        /// The closure for [`FoldSimplification::op`]
        pub op2: OP2,
        /// The closure for [`FoldSimplification::empty`]
        pub empty_shortcut: EmptyShortcut,
        /// The closure for [`FoldSimplification::delta_of`]
        pub delta_shortcut: DeltaShortcut,
        #[allow(missing_docs)]
        pub _m: PhantomData<(fn(&D)->D2,fn(&T)->D2)>
}

impl<T, D: Clone, D2: Clone, Simplifier: for<'x> Fun<&'x D,D2> + Copy, OP2: Fun<(D2,D2),D2> + Copy, EmptyShortcut: Fun<(),D2> + Copy, DeltaShortcut: for<'x> Fun<&'x T, D2> + Copy> 
Clone for SimplificationWithShortcut<T, D, D2, Simplifier, OP2, EmptyShortcut, DeltaShortcut> {
    fn clone(&self) -> Self {
        Self { simplifier: self.simplifier.clone(), op2: self.op2.clone(), empty_shortcut: self.empty_shortcut.clone(), delta_shortcut: self.delta_shortcut.clone(), _m: self._m.clone() }
    }
}

impl<T, D: Clone, D2: Clone, Simplifier: for<'x> Fun<&'x D,D2> + Copy, OP2: Fun<(D2,D2),D2> + Copy, EmptyShortcut: Fun<(),D2> + Copy, DeltaShortcut: for<'x> Fun<&'x T, D2> + Copy> Copy for SimplificationWithShortcut<T, D, D2, Simplifier, OP2, EmptyShortcut, DeltaShortcut> {}


impl<T, D: Clone, D2: Clone, Simplifier: for<'x> Fun<&'x D,D2> + Copy, OP2: Fun<(D2,D2),D2> + Copy, EmptyShortcut: Fun<(),D2> + Copy, DeltaShortcut: for<'x> Fun<&'x T, D2> + Copy> 
FoldSimplification<T,D> for SimplificationWithShortcut<T, D, D2, Simplifier, OP2, EmptyShortcut, DeltaShortcut> {
    type D2 = D2;
    fn op(&self, a: Self::D2, b: Self::D2, _: impl FoldSettings<T,D>) -> Self::D2 {
        self.op2.apply((a,b))
    }
    fn simplify(&self, delta: &D) -> Self::D2 {
        self.simplifier.apply(delta)
    }
    fn empty(&self, _: impl FoldSettings<T,D>) -> Self::D2 {
        self.empty_shortcut.apply(())
    }
    fn delta_of(&self, value: &T, _: impl FoldSettings<T,D>) -> Self::D2 {
        self.delta_shortcut.apply(value)
    }

    type Compose<D3: Clone, 
        Simplifier2: for<'x> Fun<&'x Self::D2,D3> + Copy,
        OP3: Fun<(D3,D3),D3> + Copy> 
            = SimplificationWithShortcut<T,D,D3,
                ComposeRefFn<Simplifier2,Simplifier,D2>,
                OP3,
                ComposeRefFn<Simplifier2,EmptyShortcut,D2>,
                ComposeRefFn<Simplifier2,DeltaShortcut,D2>>;

    fn compose<D3: Clone, 
        Simplifier2: for<'x> Fun<&'x Self::D2,D3> + Copy,
        OP3: Fun<(D3,D3),D3> + Copy>(self, simplifier: Simplifier2, op2: OP3) -> Self::Compose<D3,Simplifier2,OP3> {
        SimplificationWithShortcut { 
            simplifier: ComposeRefFn(simplifier, self.simplifier, PhantomData), 
            op2: op2, 
            empty_shortcut: ComposeRefFn(simplifier, self.empty_shortcut, PhantomData),
            delta_shortcut: ComposeRefFn(simplifier, self.delta_shortcut, PhantomData), 
            _m: PhantomData 
        }
    }

    type ComposeWithShortcut<D3: Clone, 
        Simplifier2: for<'x> Fun<&'x Self::D2,D3> + Copy,
        OP3: Fun<(D3,D3),D3> + Copy,
        EmptyShortcut2: Fun<(),D3> + Copy,
        DeltaShortcut2: for<'x> Fun<&'x T, D3> + Copy>
            = SimplificationWithShortcut<T,D,D3,
                ComposeRefFn<Simplifier2,Simplifier,D2>,
                OP3,
                EmptyShortcut2,
                DeltaShortcut2>;

    fn compose_with_shortcut<D3: Clone, 
        Simplifier2: for<'x> Fun<&'x Self::D2,D3> + Copy,
        OP3: Fun<(D3,D3),D3> + Copy,
        EmptyShortcut2: Fun<(),D3> + Copy,
        DeltaShortcut2: for<'x> Fun<&'x T, D3> + Copy>(self, simplifier: Simplifier2, op2: OP3,empty_shortcut: EmptyShortcut2, delta_shortcut: DeltaShortcut2) -> Self::ComposeWithShortcut<D3,Simplifier2,OP3,EmptyShortcut2,DeltaShortcut2> {
        SimplificationWithShortcut{ 
            simplifier: ComposeRefFn(simplifier, self.simplifier, PhantomData), 
            op2, 
            empty_shortcut,
            delta_shortcut, 
            _m: PhantomData 
        }
    }
    
    type ComposeAfterOther<D0: Clone,Other: FoldSimplification<T,D0,D2=D>> = Other::ComposeWithShortcut<D2,Simplifier,OP2,EmptyShortcut,DeltaShortcut>;
    fn compose_after_other<D0: Clone,Other: FoldSimplification<T,D0,D2=D>>(self,other: Other) -> Self::ComposeAfterOther<D0,Other> {
        other.compose_with_shortcut(self.simplifier, self.op2, self.empty_shortcut, self.delta_shortcut)
    }
    
    type WithSize = SimplificationWithShortcut<T,(usize,D),(usize,D2),
        KeepSizeAnd<Simplifier>,
        AddSizesAnd<OP2>,
        AlwaysAnd<0,EmptyShortcut>,
        AlwaysAnd<1,DeltaShortcut>>;
    
    fn with_size(self) -> Self::WithSize {
        SimplificationWithShortcut {
            simplifier: KeepSizeAnd(self.simplifier),
            op2: AddSizesAnd(self.op2),
            empty_shortcut: AlwaysAnd(self.empty_shortcut),
            delta_shortcut: AlwaysAnd(self.delta_shortcut),
            _m: PhantomData,
        }
    }
}


/// A named closure which ignores the first element of a 2-tuple, and clones the other
#[derive(Clone,Copy)]
pub struct SizeIgnoreFn;
impl<'a,U,D: Clone> Fun<&'a (U,D),D> for SizeIgnoreFn {
    fn apply(&self,a: &'a (U,D)) -> D {
        a.1.clone()
    }
}

/// A wrapper around a [`FoldSettings`] which extracts its `op` behavior into a closure
pub struct OpFromSettings<T,D: Clone,S: FoldSettings<T,D>> {
    /// The settings value
    pub settings: S,
    #[allow(missing_docs)]
    pub _m: PhantomData<(T,fn(D,D)->D)>
}

impl<T, D: Clone, S: FoldSettings<T,D>> Copy for OpFromSettings<T, D, S> {}

impl<T, D: Clone, S: FoldSettings<T,D>> Fun<(D,D),D> for OpFromSettings<T, D, S> {
    fn apply(&self,(a,b): (D,D)) -> D {
        self.settings.op(a,b)
    }
}
impl<T, D: Clone, S: FoldSettings<T,D>> Clone for OpFromSettings<T, D, S> {
    fn clone(&self) -> Self {
        Self { settings: self.settings.clone(), _m: self._m.clone() }
    }
}

/// An alias for a simplification that ignores the `usize` part of `(usize,D)`
pub type IgnoringSize<T,D,Settings> = SimplificationWithoutShortcut<T,(usize,D),D,SizeIgnoreFn,OpFromSettings<T,D,Settings>>;

/// A wrapper around a closure which applies it to just the second element of a 2-tuple
#[derive(Clone,Copy)]
pub struct KeepSizeAnd<F>(pub F);
impl<'x,U: Copy,D: Clone, D2: Clone, F: for<'a> Fun<&'a D,D2>> Fun<&'x (U,D),(U,D2)> for KeepSizeAnd<F> {
    fn apply(&self,a: &'x (U,D)) -> (U,D2) {
        (a.0,self.0.apply(&a.1))
    }
}

/// A wrapper around an operator closure which transforms it in the same way [`SettingsWithSize`](crate::fold_settings::SettingsWithSize) does
#[derive(Clone,Copy)]
pub struct AddSizesAnd<OP>(pub OP);
impl<U: core::ops::Add<Output = U> + Copy,D, OP: Fun<(D,D),D>> Fun<((U,D),(U,D)),(U,D)> for AddSizesAnd<OP> {
    fn apply(&self,((n1,d1),(n2,d2)): ((U,D),(U,D))) -> (U,D) {
        (n1 + n2, self.0.apply((d1,d2)))
    }
}

/// A wrapper around a closure which pairs a constant `usize` with its output
#[derive(Clone,Copy)]
pub struct AlwaysAnd<const N: usize, F>(pub F);
impl<const N: usize, D, I, F: Fun<I,D>> Fun<I,(usize,D)> for AlwaysAnd<N,F> {
    fn apply(&self,a: I) -> (usize,D) {
        (N,self.0.apply(a))
    }
}
