use core::marker::PhantomData;

use crate::{fold_chain::{self, Drain, FoldChain, FoldChainSlice, ImmFoldChainSliceStruct, Iter, MutFoldChainSlice, MutFoldChainSliceStruct}, fold_settings::{FoldSettings, FoldSettingsStruct, SettingsWithSize}, fold_simplification::{FoldSimplification, IgnoringSize, OpFromSettings, SimplificationWithShortcut, SimplificationWithoutShortcut, SizeIgnoreFn}, misc::{Bool, EmptyFn, False, Fun, NoneFun, OptOpFun, SingleEndedRange, SomeFun, True, TupleFun, private::Sealed}};

/// A base [FoldList](crate).
#[derive(Clone)]
pub struct FoldList<T,D: Clone, Settings: FoldSettings<T,D>> {
    /// The `FoldChain` which underlies this `FoldList`. See [FoldChain](crate#foldchain).
    pub underlying: FoldChain<T, (usize,D), SettingsWithSize<Settings>>
}

impl<T, D: Clone, Settings: FoldSettings<T,D>> Sealed for &FoldList<T, D, Settings> {}
impl<T, D: Clone, Settings: FoldSettings<T,D>> Sealed for &mut FoldList<T, D, Settings> {}

impl<T,D: Clone, OP: Fn(D,D)->D + Copy, DeltaOf: Fn(&T)->D + Copy, Empty: Fn()->D + Copy> FoldList<T,D,FoldSettingsStruct<T,D,TupleFun<OP>,DeltaOf,EmptyFn<Empty>>> {
    /// Create a new empty `FoldList`, given the closures for [Settings](crate#fold-settings).
    pub fn new(op: OP, delta_of: DeltaOf, empty_delta_fn: Empty) -> Self {
        FoldList::from_settings(FoldSettingsStruct {
            op_closure: TupleFun(op),
            t2d_closure: delta_of,
            empty_closure: EmptyFn(empty_delta_fn),
            _m: PhantomData,
        })
    }

    /// Create a new FoldList, given the closures for [Settings](crate#fold-settings), and fill it using an iterator, from left to right.
    pub fn from_iter(op: OP, delta_of: DeltaOf, empty_delta_fn: Empty, iter: impl Iterator<Item=T>) -> Self {
        let mut ret = Self::new(op,delta_of,empty_delta_fn);
        (&mut ret).append_right_from_iter(iter);
        ret
    }
}

impl<T,D: Clone, OP: Fn(D,D)->D + Copy, DeltaOf: Fn(&T) -> D + Copy> FoldList<T,Option<D>,FoldSettingsStruct<T,Option<D>,OptOpFun<TupleFun<OP>>,SomeFun<DeltaOf>,NoneFun>> {
    /// Create a new empty `FoldList`, given the closures for [Settings](crate#fold-settings), except the one for the empty delta, which will always be [`None`].
    /// 
    /// The resulting delta type will be `Option<D>` instead of `D`.
    pub fn new_with_opt(op: OP, delta_of: DeltaOf) -> Self {
        FoldList::from_settings(FoldSettingsStruct { 
            op_closure: OptOpFun(TupleFun(op)),
            t2d_closure: SomeFun(delta_of),
            empty_closure: NoneFun, 
            _m: PhantomData
        })
    }

    /// Create a new empty `FoldList`, given the closures for [Settings](crate#fold-settings), except the one for the empty delta, which will always be [`None`], and fill it using an iterator, from left to right.
    /// 
    /// The resulting delta type will be `Option<D>` instead of `D`.
    pub fn new_with_opt_from_iter(op: OP, delta_of: DeltaOf, iter: impl Iterator<Item=T>) -> Self {
        let mut ret = Self::new_with_opt(op, delta_of);
        ret.append_right_from_iter(iter);
        ret
    }
}

impl<T, D: Clone, Settings: FoldSettings<T,D>> FoldList<T, D, Settings> {
    /// Create a new empty `FoldList`, with the specified [Settings](crate#fold-settings).
    pub fn from_settings(settings: Settings) -> Self {
        Self {
            underlying: FoldChain::from_settings(SettingsWithSize(settings)),
        }
    }

    //below are redefinitions of the functions for FoldListSlice and MutFoldListSlice, to enable the use of e.g. list.f() instead of needing to write (&mut list).f()
    
    /// An alias of [`get_current_simplification`](FoldListSlice::get_current_simplification).
    pub fn get_current_simplification(&self) -> (){}
    /// An alias of [`get_settings`](FoldListSlice::get_settings).
    pub fn get_settings(&self) -> Settings {self.underlying.get_settings().0}
    /// An alias of [`as_sized_chain`](FoldListSlice::as_sized_chain).
    pub fn as_sized_chain(&self) -> &FoldChain<T, (usize, D), SettingsWithSize<Settings>> {
        FoldListSlice::as_sized_chain(self)
    }
    /// An alias of [`borrow`](FoldListSlice::borrow).
    pub fn borrow(&self) -> FoldListSliceFrom<'_, T, D, Settings, (), ImmFoldChainSliceStruct<'_, False, True, True, SettingsWithSize<Settings>, (), T, (usize, D)>> {
        self.as_imm()
    }
    /// An alias of [`as_unsized_chain`](FoldListSlice::as_unsized_chain).
    pub fn as_unsized_chain(&self) -> ImmFoldChainSliceStruct<'_, False, True, True, SettingsWithSize<Settings>, SimplificationWithoutShortcut<T, (usize, D), D, SizeIgnoreFn, OpFromSettings<T, D, Settings>>, T, (usize, D)> {
        FoldListSlice::as_unsized_chain(self)
    }
    /// An alias of [`as_sized_chain_keeping_simplification`](FoldListSlice::as_sized_chain_keeping_simplification).
    pub fn as_sized_chain_keeping_simplification(&self) -> ImmFoldChainSliceStruct<'_, False, True, True, SettingsWithSize<Settings>, (), T, (usize, D)> {
        FoldListSlice::as_sized_chain_keeping_simplification(self)
    }
    /// An alias of [`as_unsized_chain_keeping_simplification`](FoldListSlice::as_unsized_chain_keeping_simplification).
    pub fn as_unsized_chain_keeping_simplification(&self) -> ImmFoldChainSliceStruct<'_, False, True, True, SettingsWithSize<Settings>, SimplificationWithoutShortcut<T, (usize, D), D, SizeIgnoreFn, OpFromSettings<T, D, Settings>>, T, (usize, D)> {
        FoldListSlice::as_unsized_chain_keeping_simplification(self)
    }
    /// An alias of [`as_imm`](FoldListSlice::as_imm).
    pub fn as_imm(&self) -> FoldListSliceFrom<'_, T, D, Settings, (), ImmFoldChainSliceStruct<'_, False, True, True, SettingsWithSize<Settings>, (), T, (usize, D)>> {
        FoldListSlice::as_imm(self)
    }
    /// An alias of [`view_drop_left_until`](FoldListSlice::view_drop_left_until).
    pub fn view_drop_left_until(&self, predicate: impl Fn(&D)->bool) -> FoldListSliceFrom<'_, T, D, Settings, (), ImmFoldChainSliceStruct<'_, False, False, True, SettingsWithSize<Settings>, (), T, (usize, D)>> {
        FoldListSlice::view_drop_left_until(self, predicate)
    }
    /// An alias of [`view_take_right_until`](FoldListSlice::view_take_right_until).
    pub fn view_take_right_until(&self, predicate: impl Fn(&D)->bool) -> FoldListSliceFrom<'_, T, D, Settings, (), ImmFoldChainSliceStruct<'_, False, False, True, SettingsWithSize<Settings>, (), T, (usize, D)>> {
        FoldListSlice::view_take_right_until(self, predicate)
    }
    /// An alias of [`view_drop_right_until`](FoldListSlice::view_drop_right_until).
    pub fn view_drop_right_until(&self, predicate: impl Fn(&D)->bool) -> FoldListSliceFrom<'_, T, D, Settings, (), ImmFoldChainSliceStruct<'_, False, True, False, SettingsWithSize<Settings>, (), T, (usize, D)>> {
        FoldListSlice::view_drop_right_until(self, predicate)
    }
    /// An alias of [`view_take_left_until`](FoldListSlice::view_take_left_until).
    pub fn view_take_left_until(&self, predicate: impl Fn(&D)->bool) -> FoldListSliceFrom<'_, T, D, Settings, (), ImmFoldChainSliceStruct<'_, False, True, False, SettingsWithSize<Settings>, (), T, (usize, D)>> {
        FoldListSlice::view_take_left_until(self, predicate)
    }
    /// An alias of [`view_drop`](FoldListSlice::view_drop).
    pub fn view_drop<Predicate: Fn(&D)->bool, Range: SingleEndedRange<Predicate>>(&self, range: Range) -> <<Range as SingleEndedRange<Predicate>>::EndIsLeft as Bool>::IfElse<FoldListSliceFrom<'_, T, D, Settings, (), ImmFoldChainSliceStruct<'_, False, True, False, SettingsWithSize<Settings>, (), T, (usize, D)>>, FoldListSliceFrom<'_, T, D, Settings, (), ImmFoldChainSliceStruct<'_, False, False, True, SettingsWithSize<Settings>, (), T, (usize, D)>>> {
        FoldListSlice::view_drop(self, range)
    }
    /// An alias of [`view_take`](FoldListSlice::view_take).
    pub fn view_take<Predicate: Fn(&D)->bool, Range: SingleEndedRange<Predicate>>(&self, range: Range) -> <<Range as SingleEndedRange<Predicate>>::EndIsLeft as Bool>::IfElse<FoldListSliceFrom<'_, T, D, Settings, (), ImmFoldChainSliceStruct<'_, False, False, True, SettingsWithSize<Settings>, (), T, (usize, D)>>, FoldListSliceFrom<'_, T, D, Settings, (), ImmFoldChainSliceStruct<'_, False, True, False, SettingsWithSize<Settings>, (), T, (usize, D)>>> {
        FoldListSlice::view_take(self, range)
    }
    /// An alias of [`view_take_left_until_with_size`](FoldListSlice::view_take_left_until_with_size).
    pub fn view_take_left_until_with_size(&self, predicate: impl Fn(usize,&D)->bool) -> FoldListSliceFrom<'_, T, D, Settings, (), ImmFoldChainSliceStruct<'_, False, True, False, SettingsWithSize<Settings>, (), T, (usize, D)>> {
        FoldListSlice::view_take_left_until_with_size(self, predicate)
    }
    /// An alias of [`view_drop_right_until_with_size`](FoldListSlice::view_drop_right_until_with_size).
    pub fn view_drop_right_until_with_size(&self, predicate: impl Fn(usize,&D)->bool) -> FoldListSliceFrom<'_, T, D, Settings, (), ImmFoldChainSliceStruct<'_, False, True, False, SettingsWithSize<Settings>, (), T, (usize, D)>> {
        FoldListSlice::view_drop_right_until_with_size(self, predicate)
    }
    /// An alias of [`view_take_right_until_with_size`](FoldListSlice::view_take_right_until_with_size).
    pub fn view_take_right_until_with_size(&self, predicate: impl Fn(usize,&D)->bool) -> FoldListSliceFrom<'_, T, D, Settings, (), ImmFoldChainSliceStruct<'_, False, False, True, SettingsWithSize<Settings>, (), T, (usize, D)>> {
        FoldListSlice::view_take_right_until_with_size(self, predicate)
    }
    /// An alias of [`view_drop_left_until_with_size`](FoldListSlice::view_drop_left_until_with_size).
    pub fn view_drop_left_until_with_size(&self, predicate: impl Fn(usize,&D)->bool) -> FoldListSliceFrom<'_, T, D, Settings, (), ImmFoldChainSliceStruct<'_, False, False, True, SettingsWithSize<Settings>, (), T, (usize, D)>> {
        FoldListSlice::view_drop_left_until_with_size(self, predicate)
    }
    /// An alias of [`view_drop_with_size`](FoldListSlice::view_drop_with_size).
    pub fn view_drop_with_size<Predicate: Fn(usize,&D)->bool, Range: SingleEndedRange<Predicate>>(&self, range: Range) -> <<Range as SingleEndedRange<Predicate>>::EndIsLeft as Bool>::IfElse<FoldListSliceFrom<'_, T, D, Settings, (), ImmFoldChainSliceStruct<'_, False, True, False, SettingsWithSize<Settings>, (), T, (usize, D)>>, FoldListSliceFrom<'_, T, D, Settings, (), ImmFoldChainSliceStruct<'_, False, False, True, SettingsWithSize<Settings>, (), T, (usize, D)>>> {
        FoldListSlice::view_drop_with_size(self, range)
    }
    /// An alias of [`view_take_with_size`](FoldListSlice::view_take_with_size).
    pub fn view_take_with_size<Predicate: Fn(usize,&D)->bool, Range: SingleEndedRange<Predicate>>(&self, range: Range) -> <<Range as SingleEndedRange<Predicate>>::EndIsLeft as Bool>::IfElse<FoldListSliceFrom<'_, T, D, Settings, (), ImmFoldChainSliceStruct<'_, False, False, True, SettingsWithSize<Settings>, (), T, (usize, D)>>, FoldListSliceFrom<'_, T, D, Settings, (), ImmFoldChainSliceStruct<'_, False, True, False, SettingsWithSize<Settings>, (), T, (usize, D)>>> {
        FoldListSlice::view_take_with_size(self, range)
    }
    /// An alias of [`view_take_left`](FoldListSlice::view_take_left).
    pub fn view_take_left(&self, n: usize) -> FoldListSliceFrom<'_, T, D, Settings, (), ImmFoldChainSliceStruct<'_, False, True, False, SettingsWithSize<Settings>, (), T, (usize, D)>> {
        FoldListSlice::view_take_left(self, n)
    }
    /// An alias of [`view_drop_right`](FoldListSlice::view_drop_right).
    pub fn view_drop_right(&self, n: usize) -> FoldListSliceFrom<'_, T, D, Settings, (), ImmFoldChainSliceStruct<'_, False, True, False, SettingsWithSize<Settings>, (), T, (usize, D)>> {
        FoldListSlice::view_drop_right(self, n)
    }
    /// An alias of [`view_take_right`](FoldListSlice::view_take_right).
    pub fn view_take_right(&self, n: usize) -> FoldListSliceFrom<'_, T, D, Settings, (), ImmFoldChainSliceStruct<'_, False, False, True, SettingsWithSize<Settings>, (), T, (usize, D)>> {
        FoldListSlice::view_take_right(self, n)
    }
    /// An alias of [`view_drop_left`](FoldListSlice::view_drop_left).
    pub fn view_drop_left(&self, n: usize) -> FoldListSliceFrom<'_, T, D, Settings, (), ImmFoldChainSliceStruct<'_, False, False, True, SettingsWithSize<Settings>, (), T, (usize, D)>> {
        FoldListSlice::view_drop_left(self, n)
    }
    /// An alias of [`view_reversed`](FoldListSlice::view_reversed).
    pub fn view_reversed(&self) -> FoldListSliceFrom<'_, T, D, Settings, (), ImmFoldChainSliceStruct<'_, True, True, True, SettingsWithSize<Settings>, (), T, (usize, D)>> {
        FoldListSlice::view_reversed(self)
    }
    /// An alias of [`view_with_simplification`](FoldListSlice::view_with_simplification).
    pub fn view_with_simplification<NewSimplification: FoldSimplification<T,D>>(&self, new_simplification: NewSimplification) -> FoldListSliceFrom<'_, T, D, Settings, <NewSimplification as FoldSimplification<T, D>>::ComposeAfterOther<D, ()>, ImmFoldChainSliceStruct<'_, False, True, True, SettingsWithSize<Settings>, (), T, (usize, D)>> {
        FoldListSlice::view_with_simplification(self, new_simplification)
    }
    /// An alias of [`view_simplify`](FoldListSlice::view_simplify).
    pub fn view_simplify<'a, D2: Clone + 'a, Simplifier: for<'x> Fun<&'x D,D2> + Copy + 'a, OP2: Fun<(D2,D2),D2> + Copy + 'a>(&'a self,simplifier: Simplifier, simplified_op: OP2) -> FoldListSliceFrom<'a, T, D, Settings, SimplificationWithoutShortcut<T, D, D2, Simplifier, OP2>, ImmFoldChainSliceStruct<'a, False, True, True, SettingsWithSize<Settings>, (), T, (usize, D)>> {
        FoldListSlice::view_simplify(self, simplifier, simplified_op)
    }
    /// An alias of [`view_simplify_with_shortcut`](FoldListSlice::view_simplify_with_shortcut).
    pub fn view_simplify_with_shortcut<'a, D2: Clone + 'a, Simplifier: for<'x> Fun<&'x D,D2> + Copy + 'a, OP2: Fun<(D2,D2),D2> + Copy + 'a, EmptyShortcut: Fun<(),D2> + Copy + 'a, DeltaShortcut: for<'x> Fun<&'x T, D2> + Copy + 'a>(&'a self,simplifier: Simplifier, simplified_op: OP2, empty_shortcut: EmptyShortcut, delta_shortcut: DeltaShortcut) -> FoldListSliceFrom<'a, T, D, Settings, SimplificationWithShortcut<T, D, D2, Simplifier, OP2, EmptyShortcut, DeltaShortcut>, ImmFoldChainSliceStruct<'a, False, True, True, SettingsWithSize<Settings>, (), T, (usize, D)>> {
        FoldListSlice::view_simplify_with_shortcut(self, simplifier, simplified_op, empty_shortcut, delta_shortcut)
    }
    /// An alias of [`view_unsimplify`](FoldListSlice::view_unsimplify).
    pub fn view_unsimplify(&self) -> FoldListSliceFrom<'_, T, D, Settings, (), ImmFoldChainSliceStruct<'_, False, True, True, SettingsWithSize<Settings>, (), T, (usize, D)>> {
        FoldListSlice::view_unsimplify(self)
    }
    /// An alias of [`fold`](FoldListSlice::fold).
    pub fn fold(&self) -> D {
        self.as_unsized_chain().fold()
    }
    /// An alias of [`len`](FoldListSlice::len).
    pub fn len(&self) -> usize {
        self.as_imm().len()
    }
    /// An alias of [`is_empty`](FoldListSlice::is_empty).
    pub fn is_empty(&self) -> bool {
        self.underlying.is_empty()
    }
    /// An alias of [`left`](FoldListSlice::left).
    pub fn left<'b>(&'b self) -> Option<&'b T> {
        self.underlying.left()
    }
    /// An alias of [`right`](FoldListSlice::right).
    pub fn right<'b>(&'b self) -> Option<&'b T> {
        self.underlying.right()
    }
    /// An alias of [`get`](FoldListSlice::get).
    pub fn get<'b>(&'b self,index: usize) -> &'b T {
        foldlist_index_impl(self.borrow(), index)
    }
    /// An alias of [`foreach`](FoldListSlice::foreach).
    pub fn foreach(&self, f: impl FnMut(&T)) {
        self.underlying.foreach(f);
    }
    /// An alias of [`iter`](FoldListSlice::iter).
    pub fn iter<'b>(&self) -> Iter<'_, False, T, (usize, D)> {
        self.underlying.iter()
    }

    /// An alias of [`as_mut`](MutFoldListSlice::as_mut).
    pub fn as_mut(&mut self) -> FoldListSliceFrom<'_, T, D, Settings, (), MutFoldChainSliceStruct<'_, False, True, True, T, (usize, D), SettingsWithSize<Settings>, ()>> {
        MutFoldListSlice::as_mut(self)
    }
    /// An alias of [`borrow_mut`](MutFoldListSlice::borrow_mut).
    pub fn borrow_mut(&mut self) -> FoldListSliceFrom<'_, T, D, Settings, (), MutFoldChainSliceStruct<'_, False, True, True, T, (usize, D), SettingsWithSize<Settings>, ()>> {
        self.as_mut()
    }
    /// An alias of [`mut_as_unsized_chain`](MutFoldListSlice::mut_as_unsized_chain).
    pub fn mut_as_unsized_chain(&mut self) -> MutFoldChainSliceStruct<'_, False, True, True, T, (usize, D), SettingsWithSize<Settings>, SimplificationWithoutShortcut<T, (usize, D), D, SizeIgnoreFn, OpFromSettings<T, D, Settings>>> {
        MutFoldListSlice::mut_as_unsized_chain(self)
    }
    /// An alias of [`mut_as_sized_chain_keeping_simplification`](MutFoldListSlice::mut_as_sized_chain_keeping_simplification).
    pub fn mut_as_sized_chain_keeping_simplification(&mut self) -> MutFoldChainSliceStruct<'_, False, True, True, T, (usize, D), SettingsWithSize<Settings>, ()> {
        MutFoldListSlice::mut_as_sized_chain_keeping_simplification(self)
    }
    /// An alias of [`mut_as_unsized_chain_keeping_simplification`](MutFoldListSlice::mut_as_unsized_chain_keeping_simplification).
    pub fn mut_as_unsized_chain_keeping_simplification(&mut self) -> MutFoldChainSliceStruct<'_, False, True, True, T, (usize, D), SettingsWithSize<Settings>, SimplificationWithoutShortcut<T, (usize, D), D, SizeIgnoreFn, OpFromSettings<T, D, Settings>>> {
        MutFoldListSlice::mut_as_unsized_chain_keeping_simplification(self)
    }
    /// An alias of [`mut_view_drop_left_until`](MutFoldListSlice::mut_view_drop_left_until).
    pub fn mut_view_drop_left_until(&mut self, predicate: impl Fn(&D)->bool) -> FoldListSliceFrom<'_, T, D, Settings, (), MutFoldChainSliceStruct<'_, False, False, True, T, (usize, D), SettingsWithSize<Settings>, ()>> {
        MutFoldListSlice::mut_view_drop_left_until(self, predicate)
    }
    /// An alias of [`mut_view_take_right_until`](MutFoldListSlice::mut_view_take_right_until).
    pub fn mut_view_take_right_until(&mut self, predicate: impl Fn(&D)->bool) -> FoldListSliceFrom<'_, T, D, Settings, (), MutFoldChainSliceStruct<'_, False, False, True, T, (usize, D), SettingsWithSize<Settings>, ()>> {
        MutFoldListSlice::mut_view_take_right_until(self, predicate)
    }
    /// An alias of [`mut_view_drop_right_until`](MutFoldListSlice::mut_view_drop_right_until).
    pub fn mut_view_drop_right_until(&mut self, predicate: impl Fn(&D)->bool) -> FoldListSliceFrom<'_, T, D, Settings, (), MutFoldChainSliceStruct<'_, False, True, False, T, (usize, D), SettingsWithSize<Settings>, ()>> {
        MutFoldListSlice::mut_view_drop_right_until(self, predicate)
    }
    /// An alias of [`mut_view_take_left_until`](MutFoldListSlice::mut_view_take_left_until).
    pub fn mut_view_take_left_until(&mut self, predicate: impl Fn(&D)->bool) -> FoldListSliceFrom<'_, T, D, Settings, (), MutFoldChainSliceStruct<'_, False, True, False, T, (usize, D), SettingsWithSize<Settings>, ()>> {
        MutFoldListSlice::mut_view_take_left_until(self, predicate)
    }
    /// An alias of [`mut_view_drop`](MutFoldListSlice::mut_view_drop).
    pub fn mut_view_drop<Predicate: Fn(&D)->bool, Range: SingleEndedRange<Predicate>>(&mut self, range: Range) -> <<Range as SingleEndedRange<Predicate>>::EndIsLeft as Bool>::IfElse<FoldListSliceFrom<'_, T, D, Settings, (), MutFoldChainSliceStruct<'_, False, True, False, T, (usize, D), SettingsWithSize<Settings>, ()>>, FoldListSliceFrom<'_, T, D, Settings, (), MutFoldChainSliceStruct<'_, False, False, True, T, (usize, D), SettingsWithSize<Settings>, ()>>> {
        MutFoldListSlice::mut_view_drop(self, range)
    }
    /// An alias of [`mut_view_take`](MutFoldListSlice::mut_view_take).
    pub fn mut_view_take<Predicate: Fn(&D)->bool, Range: SingleEndedRange<Predicate>>(&mut self, range: Range) -> <<Range as SingleEndedRange<Predicate>>::EndIsLeft as Bool>::IfElse<FoldListSliceFrom<'_, T, D, Settings, (), MutFoldChainSliceStruct<'_, False, False, True, T, (usize, D), SettingsWithSize<Settings>, ()>>, FoldListSliceFrom<'_, T, D, Settings, (), MutFoldChainSliceStruct<'_, False, True, False, T, (usize, D), SettingsWithSize<Settings>, ()>>> {
        MutFoldListSlice::mut_view_take(self, range)
    }
    /// An alias of [`mut_view_drop_left_until_with_size`](MutFoldListSlice::mut_view_drop_left_until_with_size).
    pub fn mut_view_drop_left_until_with_size(&mut self, predicate: impl Fn(usize,&D)->bool) -> FoldListSliceFrom<'_, T, D, Settings, (), MutFoldChainSliceStruct<'_, False, False, True, T, (usize, D), SettingsWithSize<Settings>, ()>> {
        MutFoldListSlice::mut_view_drop_left_until_with_size(self, predicate)
    }
    /// An alias of [`mut_view_take_right_until_with_size`](MutFoldListSlice::mut_view_take_right_until_with_size).
    pub fn mut_view_take_right_until_with_size(&mut self, predicate: impl Fn(usize,&D)->bool) -> FoldListSliceFrom<'_, T, D, Settings, (), MutFoldChainSliceStruct<'_, False, False, True, T, (usize, D), SettingsWithSize<Settings>, ()>> {
        MutFoldListSlice::mut_view_take_right_until_with_size(self, predicate)
    }
    /// An alias of [`mut_view_drop_right_until_with_size`](MutFoldListSlice::mut_view_drop_right_until_with_size).
    pub fn mut_view_drop_right_until_with_size(&mut self, predicate: impl Fn(usize,&D)->bool) -> FoldListSliceFrom<'_, T, D, Settings, (), MutFoldChainSliceStruct<'_, False, True, False, T, (usize, D), SettingsWithSize<Settings>, ()>> {
        MutFoldListSlice::mut_view_drop_right_until_with_size(self, predicate)
    }
    /// An alias of [`mut_view_take_left_until_with_size`](MutFoldListSlice::mut_view_take_left_until_with_size).
    pub fn mut_view_take_left_until_with_size(&mut self, predicate: impl Fn(usize,&D)->bool) -> FoldListSliceFrom<'_, T, D, Settings, (), MutFoldChainSliceStruct<'_, False, True, False, T, (usize, D), SettingsWithSize<Settings>, ()>> {
        MutFoldListSlice::mut_view_take_left_until_with_size(self, predicate)
    }
    /// An alias of [`mut_view_drop_with_size`](MutFoldListSlice::mut_view_drop_with_size).
    pub fn mut_view_drop_with_size<Predicate: Fn(usize,&D)->bool, Range: SingleEndedRange<Predicate>>(&mut self, range: Range) -> <<Range as SingleEndedRange<Predicate>>::EndIsLeft as Bool>::IfElse<FoldListSliceFrom<'_, T, D, Settings, (), MutFoldChainSliceStruct<'_, False, True, False, T, (usize, D), SettingsWithSize<Settings>, ()>>, FoldListSliceFrom<'_, T, D, Settings, (), MutFoldChainSliceStruct<'_, False, False, True, T, (usize, D), SettingsWithSize<Settings>, ()>>> {
        MutFoldListSlice::mut_view_drop_with_size(self, range)
    }
    /// An alias of [`mut_view_take_with_size`](MutFoldListSlice::mut_view_take_with_size).
    pub fn mut_view_take_with_size<Predicate: Fn(usize,&D)->bool, Range: SingleEndedRange<Predicate>>(&mut self, range: Range) -> <<Range as SingleEndedRange<Predicate>>::EndIsLeft as Bool>::IfElse<FoldListSliceFrom<'_, T, D, Settings, (), MutFoldChainSliceStruct<'_, False, False, True, T, (usize, D), SettingsWithSize<Settings>, ()>>, FoldListSliceFrom<'_, T, D, Settings, (), MutFoldChainSliceStruct<'_, False, True, False, T, (usize, D), SettingsWithSize<Settings>, ()>>> {
        MutFoldListSlice::mut_view_take_with_size(self, range)
    }
    /// An alias of [`mut_view_drop_left`](MutFoldListSlice::mut_view_drop_left).
    pub fn mut_view_drop_left(&mut self, n: usize) -> FoldListSliceFrom<'_, T, D, Settings, (), MutFoldChainSliceStruct<'_, False, False, True, T, (usize, D), SettingsWithSize<Settings>, ()>> {
        MutFoldListSlice::mut_view_drop_left(self, n)
    }
    /// An alias of [`mut_view_take_right`](MutFoldListSlice::mut_view_take_right).
    pub fn mut_view_take_right(&mut self, n: usize) -> FoldListSliceFrom<'_, T, D, Settings, (), MutFoldChainSliceStruct<'_, False, False, True, T, (usize, D), SettingsWithSize<Settings>, ()>> {
        MutFoldListSlice::mut_view_take_right(self, n)
    }
    /// An alias of [`mut_view_drop_right`](MutFoldListSlice::mut_view_drop_right).
    pub fn mut_view_drop_right(&mut self, n: usize) -> FoldListSliceFrom<'_, T, D, Settings, (), MutFoldChainSliceStruct<'_, False, True, False, T, (usize, D), SettingsWithSize<Settings>, ()>> {
        MutFoldListSlice::mut_view_drop_right(self, n)
    }
    /// An alias of [`mut_view_take_left`](MutFoldListSlice::mut_view_take_left).
    pub fn mut_view_take_left(&mut self, n: usize) -> FoldListSliceFrom<'_, T, D, Settings, (), MutFoldChainSliceStruct<'_, False, True, False, T, (usize, D), SettingsWithSize<Settings>, ()>> {
        MutFoldListSlice::mut_view_take_left(self, n)
    }
    /// An alias of [`mut_view_reversed`](MutFoldListSlice::mut_view_reversed).
    pub fn mut_view_reversed(&mut self) -> FoldListSliceFrom<'_, T, D, Settings, (), MutFoldChainSliceStruct<'_, True, True, True, T, (usize, D), SettingsWithSize<Settings>, ()>> {
        MutFoldListSlice::mut_view_reversed(self)
    }
    /// An alias of [`mut_view_with_simplification`](MutFoldListSlice::mut_view_with_simplification).
    pub fn mut_view_with_simplification<NewSimplification: FoldSimplification<T,D>>(&mut self, new_simplification: NewSimplification) -> FoldListSliceFrom<'_, T, D, Settings, <NewSimplification as FoldSimplification<T, D>>::ComposeAfterOther<D, ()>, MutFoldChainSliceStruct<'_, False, True, True, T, (usize, D), SettingsWithSize<Settings>, ()>> {
        MutFoldListSlice::mut_view_with_simplification(self, new_simplification)
    }
    /// An alias of [`mut_view_simplify`](MutFoldListSlice::mut_view_simplify).
    pub fn mut_view_simplify<'a,D2: Clone + 'a, Simplifier: for<'x> Fun<&'x D,D2> + Copy + 'a, OP2: Fun<(D2,D2),D2> + Copy + 'a>(&'a mut self,simplifier: Simplifier, simplified_op: OP2) -> FoldListSliceFrom<'a, T, D, Settings, SimplificationWithoutShortcut<T, D, D2, Simplifier, OP2>, MutFoldChainSliceStruct<'a, False, True, True, T, (usize, D), SettingsWithSize<Settings>, ()>> {
        MutFoldListSlice::mut_view_simplify(self, simplifier, simplified_op)
    }
    /// An alias of [`mut_view_simplify_with_shortcut`](MutFoldListSlice::mut_view_simplify_with_shortcut).
    pub fn mut_view_simplify_with_shortcut<'a,D2: Clone + 'a, Simplifier: for<'x> Fun<&'x D,D2> + Copy + 'a, OP2: Fun<(D2,D2),D2> + Copy + 'a, EmptyShortcut: Fun<(),D2> + Copy + 'a, DeltaShortcut: for<'x> Fun<&'x T, D2> + Copy + 'a>(&'a mut self,simplifier: Simplifier, simplified_op: OP2, empty_shortcut: EmptyShortcut, delta_shortcut: DeltaShortcut) -> FoldListSliceFrom<'a, T, D, Settings, SimplificationWithShortcut<T, D, D2, Simplifier, OP2, EmptyShortcut, DeltaShortcut>, MutFoldChainSliceStruct<'a, False, True, True, T, (usize, D), SettingsWithSize<Settings>, ()>> {
        MutFoldListSlice::mut_view_simplify_with_shortcut(self, simplifier, simplified_op, empty_shortcut, delta_shortcut)
    }
    /// An alias of [`mut_view_unsimplify`](MutFoldListSlice::mut_view_unsimplify).
    pub fn mut_view_unsimplify(&mut self) -> FoldListSliceFrom<'_, T, D, Settings, (), MutFoldChainSliceStruct<'_, False, True, True, T, (usize, D), SettingsWithSize<Settings>, ()>> {
        MutFoldListSlice::mut_view_unsimplify(self)
    }
    /// An alias of [`pop_left`](MutFoldListSlice::pop_left).
    pub fn pop_left(&mut self) -> Option<T> {
        self.underlying.pop_left()
    }
    /// An alias of [`pop_right`](MutFoldListSlice::pop_right).
    pub fn pop_right(&mut self) -> Option<T> {
        self.underlying.pop_right()
    }
    /// An alias of [`append_left`](MutFoldListSlice::append_left).
    pub fn append_left(&mut self, value: T) {
        self.underlying.append_left(value);
    }
    /// An alias of [`append_right`](MutFoldListSlice::append_right).
    pub fn append_right(&mut self, value: T) {
        self.underlying.append_right(value);
    }
    /// An alias of [`set_left_or_err`](MutFoldListSlice::set_left_or_err).
    pub fn set_left_or_err(&mut self, value: T) -> Result<T,T> {
        self.underlying.set_left_or_err(value)
    }
    /// An alias of [`set_right_or_err`](MutFoldListSlice::set_right_or_err).
    pub fn set_right_or_err(&mut self, value: T) -> Result<T,T> {
        self.underlying.set_right_or_err(value)
    }
    /// An alias of [`set_left`](MutFoldListSlice::set_left).
    pub fn set_left(&mut self, value: T) -> T {
        self.underlying.set_left(value)
    }
    /// An alias of [`set_right`](MutFoldListSlice::set_right).
    pub fn set_right(&mut self, value: T) -> T {
        self.underlying.set_right(value)
    }
    /// An alias of [`update_left`](MutFoldListSlice::update_left).
    pub fn update_left<R>(&mut self, f: impl FnOnce(Option<&mut T>) -> R) -> R {
        self.underlying.update_left(f)
    }
    /// An alias of [`update_right`](MutFoldListSlice::update_right).
    pub fn update_right<R>(&mut self, f: impl FnOnce(Option<&mut T>) -> R) -> R {
        self.underlying.update_right(f)
    }
    /// An alias of [`update_at`](MutFoldListSlice::update_at).
    pub fn update_at<R>(&mut self, index: usize, f: impl FnOnce(&mut T)->R)->R {
        MutFoldListSlice::update_at(&mut &mut *self, index, f)
    }
    /// An alias of [`set_at`](MutFoldListSlice::set_at).
    pub fn set_at(&mut self, index: usize, value: T) -> T {
        MutFoldListSlice::set_at(&mut &mut *self,index,value)
    }
    /// An alias of [`insert_at`](MutFoldListSlice::insert_at).
    pub fn insert_at(&mut self, index: usize, value: T) {
        MutFoldListSlice::insert_at(&mut &mut *self, index, value);
    }
    /// An alias of [`remove_at`](MutFoldListSlice::remove_at).
    pub fn remove_at(&mut self, index: usize) -> T {
        MutFoldListSlice::remove_at(&mut &mut *self, index)
    }
    /// An alias of [`foreach_mut`](MutFoldListSlice::foreach_mut).
    pub fn foreach_mut(&mut self, f: impl FnMut(&mut T)) {
        self.underlying.foreach_mut(f);
    }
    /// An alias of [`take_all`](MutFoldListSlice::take_all).
    pub fn take_all(&mut self) -> FoldList<T, D, Settings> {
        MutFoldListSlice::take_all(&mut &mut *self)
    }
    /// An alias of [`append_all_right`](MutFoldListSlice::append_all_right).
    pub fn append_all_right(&mut self, list: FoldList<T,D,Settings>) {
        self.underlying.append_all_right(list.underlying);
    }
    /// An alias of [`append_all_left`](MutFoldListSlice::append_all_left).
    pub fn append_all_left(&mut self, list: FoldList<T,D,Settings>) {
        self.underlying.append_all_left(list.underlying);
    }
    /// An alias of [`append_left_from_iter`](MutFoldListSlice::append_left_from_iter).
    pub fn append_left_from_iter(&mut self, iter: impl Iterator<Item=T>) {
        self.underlying.append_left_from_iter(iter);
    }
    /// An alias of [`append_right_from_iter`](MutFoldListSlice::append_right_from_iter).
    pub fn append_right_from_iter(&mut self, iter: impl Iterator<Item=T>) {
        self.underlying.append_right_from_iter(iter);
    }
}

/// The trait for views into a [`FoldList`].
/// 
/// For views which are also mutable, see [`MutFoldListSlice`].
/// 
/// This trait is sealed and cannot be implemented for types outside this crate.
pub trait FoldListSlice<'a,T: 'a, D: Clone + 'a> : 'a + Sized + core::ops::Index<usize,Output=T> + Sealed {
    /// The `D` type of the base [`FoldList`]. May differ from this `D` if a simplification has been applied.
    type OriginalD: Clone + 'a;
    /// The type of the current [simplification](crate#simplification)
    type Simplification: FoldSimplification<T,Self::OriginalD,D2 = D>;
    /// Get a copy of the current [simplification](crate#simplification). Will be `()` if no simplification has been applied.
    fn get_current_simplification(&self) -> Self::Simplification;

    /// The type of the base [`FoldList`]'s [settings](crate#fold-settings).
    type Settings: FoldSettings<T,Self::OriginalD>;
    /// Get a copy of the base [`FoldList`]'s [settings](crate#fold-settings).
    fn get_settings(&self) -> Self::Settings;

    /// The type of the [`FoldChainSlice`] of the [`FoldChain`] that the base [`FoldList`] is build upon, that this slice is built upon. See [FoldChain](crate#foldchain).
    type UnderlyingChain: FoldChainSlice<'a,T,(usize,Self::OriginalD),
        Settings = SettingsWithSize<Self::Settings>,
        Simplification = (), OriginalD = (usize,Self::OriginalD)>;

    /// Get the underlying [`FoldChainSlice`] of this slice. See [FoldChain](crate#foldchain).
    /// 
    /// Note that this will discard the current simplification; to keep it, see [`as_sized_chain_keeping_simplification`](FoldListSlice::as_sized_chain_keeping_simplification).
    fn as_sized_chain(self) -> Self::UnderlyingChain;

    /// Immutably borrow this view.
    fn borrow<'b>(&'b self) -> FoldListSliceFrom<'b,T,Self::OriginalD,
        Self::Settings,
        Self::Simplification,
        ImmFoldChainSliceStruct<'b,
            <Self::UnderlyingChain as FoldChainSlice<'a,T,(usize,Self::OriginalD)>>::IsReversed,
            <Self::UnderlyingChain as FoldChainSlice<'a,T,(usize,Self::OriginalD)>>::IsFlushLeft,
            <Self::UnderlyingChain as FoldChainSlice<'a,T,(usize,Self::OriginalD)>>::IsFlushRight,
            SettingsWithSize<Self::Settings>,
            (),
            T,
            (usize,Self::OriginalD)>>;

    /// Get the underlying [`FoldChainSlice`] of this slice, and then apply a simplification to it that makes it ignore its size information. See [FoldChain](crate#foldchain).
    /// 
    /// This operation's mutable version is [`mut_as_unsized_chain`](MutFoldListSlice::mut_as_unsized_chain).
    fn as_unsized_chain(self) -> ImmFoldChainSliceStruct<'a,
        <Self::UnderlyingChain as FoldChainSlice<'a,T,(usize,Self::OriginalD)>>::IsReversed,
        <Self::UnderlyingChain as FoldChainSlice<'a,T,(usize,Self::OriginalD)>>::IsFlushLeft,
        <Self::UnderlyingChain as FoldChainSlice<'a,T,(usize,Self::OriginalD)>>::IsFlushRight,
        SettingsWithSize<Self::Settings>,
        IgnoringSize<T,Self::OriginalD,Self::Settings>,
        T,
        (usize,Self::OriginalD)> {
            let underlying = self.as_sized_chain().as_imm();
            let SettingsWithSize(settings) = underlying.get_settings();
            underlying.view_simplify(SizeIgnoreFn,
                OpFromSettings{ 
                    settings, 
                    _m: PhantomData 
                }
            )
    }

    /// Get the underlying [`FoldChainSlice`] of this slice, keeping the current simplification. See [FoldChain](crate#foldchain).
    /// 
    /// This operation's mutable version is [`mut_as_sized_chain_keeping_simplification`](MutFoldListSlice::mut_as_sized_chain_keeping_simplification).
    fn as_sized_chain_keeping_simplification(self) -> ImmFoldChainSliceStruct<'a, 
        <Self::UnderlyingChain as FoldChainSlice<'a, T, (usize, Self::OriginalD)>>::IsReversed, 
        <Self::UnderlyingChain as FoldChainSlice<'a, T, (usize, Self::OriginalD)>>::IsFlushLeft, 
        <Self::UnderlyingChain as FoldChainSlice<'a, T, (usize, Self::OriginalD)>>::IsFlushRight, 
        SettingsWithSize<Self::Settings>, 
        <Self::Simplification as FoldSimplification<T, Self::OriginalD>>::WithSize, 
        T, (usize, Self::OriginalD)> {
            let simplification: Self::Simplification = self.get_current_simplification();
            let underlying: ImmFoldChainSliceStruct<'a, <Self::UnderlyingChain as FoldChainSlice<'a, T, (usize, Self::OriginalD)>>::IsReversed, <Self::UnderlyingChain as FoldChainSlice<'a, T, (usize, Self::OriginalD)>>::IsFlushLeft, <Self::UnderlyingChain as FoldChainSlice<'a, T, (usize, Self::OriginalD)>>::IsFlushRight, SettingsWithSize<Self::Settings>, (), T, (usize, Self::OriginalD)> = self.as_sized_chain().as_imm();
            ImmFoldChainSliceStruct {
                endpoints: underlying.endpoints,
                settings: underlying.settings,
                simplification: simplification.with_size(),
                _m: PhantomData,
            }
    }

    /// Get the underlying [`FoldChainSlice`] of this slice, and then apply a simplification to it that makes it ignore its size information, and then also apply the current simplification to the result. See [FoldChain](crate#foldchain).
    /// 
    /// This operation's mutable version is [`mut_as_unsized_chain_keeping_simplification`](MutFoldListSlice::mut_as_unsized_chain_keeping_simplification).
    fn as_unsized_chain_keeping_simplification(self) -> ImmFoldChainSliceStruct<'a, 
        <Self::UnderlyingChain as FoldChainSlice<'a, T, (usize, Self::OriginalD)>>::IsReversed, 
        <Self::UnderlyingChain as FoldChainSlice<'a, T, (usize, Self::OriginalD)>>::IsFlushLeft, 
        <Self::UnderlyingChain as FoldChainSlice<'a, T, (usize, Self::OriginalD)>>::IsFlushRight, 
        SettingsWithSize<Self::Settings>,
        <Self::Simplification as FoldSimplification<T, Self::OriginalD>>::ComposeAfterOther<(usize, Self::OriginalD), 
            IgnoringSize<T,Self::OriginalD,Self::Settings>>, 
        T, (usize, Self::OriginalD)> {
            let simplification: Self::Simplification = self.get_current_simplification();
            let underlying_without_simplification = self.as_unsized_chain();
            underlying_without_simplification.view_with_simplification(simplification)
    }

    /// Make this view immutable.
    fn as_imm(self) -> FoldListSliceFrom<'a,T,Self::OriginalD,
        Self::Settings,
        Self::Simplification,
        ImmFoldChainSliceStruct<'a,
            <Self::UnderlyingChain as FoldChainSlice<'a,T,(usize,Self::OriginalD)>>::IsReversed,
            <Self::UnderlyingChain as FoldChainSlice<'a,T,(usize,Self::OriginalD)>>::IsFlushLeft,
            <Self::UnderlyingChain as FoldChainSlice<'a,T,(usize,Self::OriginalD)>>::IsFlushRight,
            SettingsWithSize<Self::Settings>,
            (),
            T,
            (usize,Self::OriginalD)>> {
                FoldListSliceFrom { 
                    simplification: self.get_current_simplification(), 
                    underlying: self.as_sized_chain().as_imm(), 
                    _m: PhantomData
                }
    }

    /// Contract this view on the left while the to-be-discarded range's fold doesn't meet `predicate`.
    /// 
    /// The given `predicate` must be well-behaved; see [Slicing](crate#slicing) for examples.
    /// 
    /// This operation's mutable version is [`mut_view_drop_left_until`](MutFoldListSlice::mut_view_drop_left_until) and 
    /// its mirror image is [`view_drop_right_until`](FoldListSlice::view_drop_right_until).
    /// 
    /// To also take into account the length of the range, see [`view_drop_left_until_with_size`](FoldListSlice::view_drop_left_until_with_size).
    /// 
    /// This is equivalent to ```self.view_drop(..predicate)```.
    fn view_drop_left_until(self, predicate: impl Fn(&D)->bool) -> FoldListSliceFrom<'a, T, Self::OriginalD, Self::Settings, Self::Simplification, ImmFoldChainSliceStruct<'a, <Self::UnderlyingChain as FoldChainSlice<'a, T, (usize, Self::OriginalD)>>::IsReversed, <<Self::UnderlyingChain as FoldChainSlice<'a, T, (usize, Self::OriginalD)>>::IsFlushLeft as Bool>::And<<Self::UnderlyingChain as FoldChainSlice<'a, T, (usize, Self::OriginalD)>>::IsReversed>, <<Self::UnderlyingChain as FoldChainSlice<'a, T, (usize, Self::OriginalD)>>::IsFlushRight as Bool>::And<<<Self::UnderlyingChain as FoldChainSlice<'a, T, (usize, Self::OriginalD)>>::IsReversed as Bool>::Not>, SettingsWithSize<Self::Settings>, (), T, (usize, Self::OriginalD)>> {
        FoldListSliceFrom {
            simplification: self.get_current_simplification(),
            underlying: self.as_unsized_chain_keeping_simplification()
                .view_drop_left_until(predicate)
                .view_unsimplify(),
            _m: PhantomData,
        }
    }

    /// Restrict this view to the longest range that starts on the right and whose fold doesn't meet `predicate`.
    /// 
    /// The given `predicate` must be well-behaved; see [Slicing](crate#slicing) for examples.
    /// 
    /// This operation's mutable version is [`mut_view_take_right_until`](MutFoldListSlice::mut_view_take_right_until) and
    /// its mirror image is [`view_take_left_until`](FoldListSlice::view_take_left_until).
    /// 
    /// To also take into account the length of the range, see [`view_take_right_until_with_size`](FoldListSlice::view_take_right_until_with_size).
    /// 
    /// This is equivalent to ```self.view_take(predicate..)```.
    fn view_take_right_until(self, predicate: impl Fn(&D)->bool) -> FoldListSliceFrom<'a, T, Self::OriginalD, Self::Settings, Self::Simplification, ImmFoldChainSliceStruct<'a, <Self::UnderlyingChain as FoldChainSlice<'a, T, (usize, Self::OriginalD)>>::IsReversed, <<Self::UnderlyingChain as FoldChainSlice<'a, T, (usize, Self::OriginalD)>>::IsFlushLeft as Bool>::And<<Self::UnderlyingChain as FoldChainSlice<'a, T, (usize, Self::OriginalD)>>::IsReversed>, <<Self::UnderlyingChain as FoldChainSlice<'a, T, (usize, Self::OriginalD)>>::IsFlushRight as Bool>::And<<<Self::UnderlyingChain as FoldChainSlice<'a, T, (usize, Self::OriginalD)>>::IsReversed as Bool>::Not>, SettingsWithSize<Self::Settings>, (), T, (usize, Self::OriginalD)>> {
        FoldListSliceFrom {
            simplification: self.get_current_simplification(),
            underlying: self.as_unsized_chain_keeping_simplification()
                .view_take_right_until(predicate)
                .view_unsimplify(),
            _m: PhantomData,
        }
    }

    /// Contract this view on the right while the to-be-discarded range's fold doesn't meet `predicate`.
    /// 
    /// The given `predicate` must be well-behaved; see [Slicing](crate#slicing) for examples.
    /// 
    /// This operation's mutable version is [`mut_view_drop_right_until`](MutFoldListSlice::mut_view_drop_right_until) and 
    /// its mirror image is [`view_drop_left_until`](FoldListSlice::view_drop_left_until).
    /// 
    /// To also take into account the length of the range, see [`view_drop_right_until_with_size`](FoldListSlice::view_drop_right_until_with_size).
    /// 
    /// This is equivalent to ```self.view_drop(predicate..)```.
    fn view_drop_right_until(self, predicate: impl Fn(&D)->bool) -> FoldListSliceFrom<'a, T, Self::OriginalD, Self::Settings, Self::Simplification, ImmFoldChainSliceStruct<'a, <<<Self::UnderlyingChain as FoldChainSlice<'a, T, (usize, Self::OriginalD)>>::IsReversed as Bool>::Not as Bool>::Not, <<Self::UnderlyingChain as FoldChainSlice<'a, T, (usize, Self::OriginalD)>>::IsFlushLeft as Bool>::And<<<Self::UnderlyingChain as FoldChainSlice<'a, T, (usize, Self::OriginalD)>>::IsReversed as Bool>::Not>, <<Self::UnderlyingChain as FoldChainSlice<'a, T, (usize, Self::OriginalD)>>::IsFlushRight as Bool>::And<<<<Self::UnderlyingChain as FoldChainSlice<'a, T, (usize, Self::OriginalD)>>::IsReversed as Bool>::Not as Bool>::Not>, SettingsWithSize<Self::Settings>, (), T, (usize, Self::OriginalD)>> {
        FoldListSliceFrom {
            simplification: self.get_current_simplification(),
            underlying: self.as_unsized_chain_keeping_simplification()
                .view_drop_right_until(predicate)
                .view_unsimplify(),
            _m: PhantomData,
        }
    }

    /// Restrict this view to the longest range that starts on the left and whose fold doesn't meet `predicate`.
    /// 
    /// The given `predicate` must be well-behaved; see [Slicing](crate#slicing) for examples.
    /// 
    /// This operation's mutable version is [`mut_view_take_left_until`](MutFoldListSlice::mut_view_take_left_until) and
    /// its mirror image is [`view_take_right_until`](FoldListSlice::view_take_right_until).
    /// 
    /// To also take into account the length of the range, see [`view_take_left_until_with_size`](FoldListSlice::view_take_left_until_with_size).
    /// 
    /// This is equivalent to ```self.view_take(..predicate)```.
    fn view_take_left_until(self, predicate: impl Fn(&D)->bool) -> FoldListSliceFrom<'a, T, Self::OriginalD, Self::Settings, Self::Simplification, ImmFoldChainSliceStruct<'a, <<<Self::UnderlyingChain as FoldChainSlice<'a, T, (usize, Self::OriginalD)>>::IsReversed as Bool>::Not as Bool>::Not, <<Self::UnderlyingChain as FoldChainSlice<'a, T, (usize, Self::OriginalD)>>::IsFlushLeft as Bool>::And<<<Self::UnderlyingChain as FoldChainSlice<'a, T, (usize, Self::OriginalD)>>::IsReversed as Bool>::Not>, <<Self::UnderlyingChain as FoldChainSlice<'a, T, (usize, Self::OriginalD)>>::IsFlushRight as Bool>::And<<<<Self::UnderlyingChain as FoldChainSlice<'a, T, (usize, Self::OriginalD)>>::IsReversed as Bool>::Not as Bool>::Not>, SettingsWithSize<Self::Settings>, (), T, (usize, Self::OriginalD)>> {
        FoldListSliceFrom {
            simplification: self.get_current_simplification(),
            underlying: self.as_unsized_chain_keeping_simplification()
                .view_take_left_until(predicate)
                .view_unsimplify(),
            _m: PhantomData,
        }
    }

    /// If `range` is ```..predicate```, calls [`view_drop_left_until(predicate)`](FoldListSlice::view_drop_left_until).
    /// 
    /// If `range` is ```predicate..```, calls [`view_drop_right_until(predicate)`](FoldListSlice::view_drop_right_until) (You may need to parenthesize `predicate` in this case).
    /// 
    /// The given `predicate` must be well-behaved; see [Slicing](crate#slicing) for examples.
    /// 
    /// This operation's mutable version is [`mut_view_drop`](MutFoldListSlice::mut_view_drop).
    fn view_drop<Predicate: Fn(&D)->bool, Range: SingleEndedRange<Predicate>>(self, range: Range) -> <<Range as SingleEndedRange<Predicate>>::EndIsLeft as Bool>::IfElse<FoldListSliceFrom<'a, T, <Self as FoldListSlice<'a, T, D>>::OriginalD, <Self as FoldListSlice<'a, T, D>>::Settings, <Self as FoldListSlice<'a, T, D>>::Simplification, ImmFoldChainSliceStruct<'a, <<<<Self as FoldListSlice<'a, T, D>>::UnderlyingChain as FoldChainSlice<'a, T, (usize, <Self as FoldListSlice<'a, T, D>>::OriginalD)>>::IsReversed as Bool>::Not as Bool>::Not, <<<Self as FoldListSlice<'a, T, D>>::UnderlyingChain as FoldChainSlice<'a, T, (usize, <Self as FoldListSlice<'a, T, D>>::OriginalD)>>::IsFlushLeft as Bool>::And<<<<Self as FoldListSlice<'a, T, D>>::UnderlyingChain as FoldChainSlice<'a, T, (usize, <Self as FoldListSlice<'a, T, D>>::OriginalD)>>::IsReversed as Bool>::Not>, <<<Self as FoldListSlice<'a, T, D>>::UnderlyingChain as FoldChainSlice<'a, T, (usize, <Self as FoldListSlice<'a, T, D>>::OriginalD)>>::IsFlushRight as Bool>::And<<<<<Self as FoldListSlice<'a, T, D>>::UnderlyingChain as FoldChainSlice<'a, T, (usize, <Self as FoldListSlice<'a, T, D>>::OriginalD)>>::IsReversed as Bool>::Not as Bool>::Not>, SettingsWithSize<<Self as FoldListSlice<'a, T, D>>::Settings>, (), T, (usize, <Self as FoldListSlice<'a, T, D>>::OriginalD)>>, FoldListSliceFrom<'a, T, <Self as FoldListSlice<'a, T, D>>::OriginalD, <Self as FoldListSlice<'a, T, D>>::Settings, <Self as FoldListSlice<'a, T, D>>::Simplification, ImmFoldChainSliceStruct<'a, <<Self as FoldListSlice<'a, T, D>>::UnderlyingChain as FoldChainSlice<'a, T, (usize, <Self as FoldListSlice<'a, T, D>>::OriginalD)>>::IsReversed, <<<Self as FoldListSlice<'a, T, D>>::UnderlyingChain as FoldChainSlice<'a, T, (usize, <Self as FoldListSlice<'a, T, D>>::OriginalD)>>::IsFlushLeft as Bool>::And<<<Self as FoldListSlice<'a, T, D>>::UnderlyingChain as FoldChainSlice<'a, T, (usize, <Self as FoldListSlice<'a, T, D>>::OriginalD)>>::IsReversed>, <<<Self as FoldListSlice<'a, T, D>>::UnderlyingChain as FoldChainSlice<'a, T, (usize, <Self as FoldListSlice<'a, T, D>>::OriginalD)>>::IsFlushRight as Bool>::And<<<<Self as FoldListSlice<'a, T, D>>::UnderlyingChain as FoldChainSlice<'a, T, (usize, <Self as FoldListSlice<'a, T, D>>::OriginalD)>>::IsReversed as Bool>::Not>, SettingsWithSize<<Self as FoldListSlice<'a, T, D>>::Settings>, (), T, (usize, <Self as FoldListSlice<'a, T, D>>::OriginalD)>>> {
        Range::EndIsLeft::init_if_else((self,SingleEndedRange::end(range)), 
            |(this,end)| this.view_drop_right_until(end), 
            |(this,end)| this.view_drop_left_until(end), 
        )
    }

    /// If `range` is ```..predicate```, calls [`view_take_left_until(predicate)`](FoldListSlice::view_take_left_until).
    /// 
    /// If `range` is ```predicate..```, calls [`view_take_right_until(predicate)`](FoldListSlice::view_take_right_until) (You may need to parenthesize `predicate` in this case).
    /// 
    /// The given `predicate` must be well-behaved; see [Slicing](crate#slicing) for examples.
    /// 
    /// This operation's mutable version is [`mut_view_take`](MutFoldListSlice::mut_view_take).
    fn view_take<Predicate: Fn(&D)->bool, Range: SingleEndedRange<Predicate>>(self, range: Range) -> <<Range as SingleEndedRange<Predicate>>::EndIsLeft as Bool>::IfElse<FoldListSliceFrom<'a, T, <Self as FoldListSlice<'a, T, D>>::OriginalD, <Self as FoldListSlice<'a, T, D>>::Settings, <Self as FoldListSlice<'a, T, D>>::Simplification, ImmFoldChainSliceStruct<'a, <<Self as FoldListSlice<'a, T, D>>::UnderlyingChain as FoldChainSlice<'a, T, (usize, <Self as FoldListSlice<'a, T, D>>::OriginalD)>>::IsReversed, <<<Self as FoldListSlice<'a, T, D>>::UnderlyingChain as FoldChainSlice<'a, T, (usize, <Self as FoldListSlice<'a, T, D>>::OriginalD)>>::IsFlushLeft as Bool>::And<<<Self as FoldListSlice<'a, T, D>>::UnderlyingChain as FoldChainSlice<'a, T, (usize, <Self as FoldListSlice<'a, T, D>>::OriginalD)>>::IsReversed>, <<<Self as FoldListSlice<'a, T, D>>::UnderlyingChain as FoldChainSlice<'a, T, (usize, <Self as FoldListSlice<'a, T, D>>::OriginalD)>>::IsFlushRight as Bool>::And<<<<Self as FoldListSlice<'a, T, D>>::UnderlyingChain as FoldChainSlice<'a, T, (usize, <Self as FoldListSlice<'a, T, D>>::OriginalD)>>::IsReversed as Bool>::Not>, SettingsWithSize<<Self as FoldListSlice<'a, T, D>>::Settings>, (), T, (usize, <Self as FoldListSlice<'a, T, D>>::OriginalD)>>, FoldListSliceFrom<'a, T, <Self as FoldListSlice<'a, T, D>>::OriginalD, <Self as FoldListSlice<'a, T, D>>::Settings, <Self as FoldListSlice<'a, T, D>>::Simplification, ImmFoldChainSliceStruct<'a, <<<<Self as FoldListSlice<'a, T, D>>::UnderlyingChain as FoldChainSlice<'a, T, (usize, <Self as FoldListSlice<'a, T, D>>::OriginalD)>>::IsReversed as Bool>::Not as Bool>::Not, <<<Self as FoldListSlice<'a, T, D>>::UnderlyingChain as FoldChainSlice<'a, T, (usize, <Self as FoldListSlice<'a, T, D>>::OriginalD)>>::IsFlushLeft as Bool>::And<<<<Self as FoldListSlice<'a, T, D>>::UnderlyingChain as FoldChainSlice<'a, T, (usize, <Self as FoldListSlice<'a, T, D>>::OriginalD)>>::IsReversed as Bool>::Not>, <<<Self as FoldListSlice<'a, T, D>>::UnderlyingChain as FoldChainSlice<'a, T, (usize, <Self as FoldListSlice<'a, T, D>>::OriginalD)>>::IsFlushRight as Bool>::And<<<<<Self as FoldListSlice<'a, T, D>>::UnderlyingChain as FoldChainSlice<'a, T, (usize, <Self as FoldListSlice<'a, T, D>>::OriginalD)>>::IsReversed as Bool>::Not as Bool>::Not>, SettingsWithSize<<Self as FoldListSlice<'a, T, D>>::Settings>, (), T, (usize, <Self as FoldListSlice<'a, T, D>>::OriginalD)>>> {
        Range::EndIsLeft::init_if_else((self,SingleEndedRange::end(range)), 
            |(this,end)| this.view_take_right_until(end), 
            |(this,end)| this.view_take_left_until(end), 
        )
    }

    /// Restrict this view to the longest range that starts on the left and whose length and fold don't meet `predicate`.
    /// 
    /// The given `predicate` must be well-behaved; see [Slicing](crate#slicing) for examples.
    /// 
    /// This operation's mutable version is [`mut_view_take_left_until_with_size`](MutFoldListSlice::mut_view_take_left_until_with_size) and
    /// its mirror image is [`view_take_right_until_with_size`](FoldListSlice::view_take_right_until_with_size).
    /// Its unsized version is [`view_take_left_until`](FoldListSlice::view_take_left_until).
    /// 
    /// This is equivalent to ```self.view_take_with_size(..predicate)```.
    fn view_take_left_until_with_size(self, predicate: impl Fn(usize,&D)->bool) -> FoldListSliceFrom<'a, T, Self::OriginalD, Self::Settings, Self::Simplification, ImmFoldChainSliceStruct<'a, <<<Self::UnderlyingChain as FoldChainSlice<'a, T, (usize, Self::OriginalD)>>::IsReversed as Bool>::Not as Bool>::Not, <<Self::UnderlyingChain as FoldChainSlice<'a, T, (usize, Self::OriginalD)>>::IsFlushLeft as Bool>::And<<<Self::UnderlyingChain as FoldChainSlice<'a, T, (usize, Self::OriginalD)>>::IsReversed as Bool>::Not>, <<Self::UnderlyingChain as FoldChainSlice<'a, T, (usize, Self::OriginalD)>>::IsFlushRight as Bool>::And<<<<Self::UnderlyingChain as FoldChainSlice<'a, T, (usize, Self::OriginalD)>>::IsReversed as Bool>::Not as Bool>::Not>, SettingsWithSize<Self::Settings>, (), T, (usize, Self::OriginalD)>> {
        FoldListSliceFrom {
            simplification: self.get_current_simplification(),
            underlying: self.as_sized_chain_keeping_simplification()
                .view_take_left_until(|(n,d)| predicate(*n,d))
                .view_unsimplify(),
            _m: PhantomData
        }
    }

    /// Contract this view on the right while the to-be-discarded range's length and fold don't meet `predicate`.
    /// 
    /// The given `predicate` must be well-behaved; see [Slicing](crate#slicing) for examples.
    /// 
    /// This operation's mutable version is [`mut_view_drop_right_until_with_size`](MutFoldListSlice::mut_view_drop_right_until_with_size) and 
    /// its mirror image is [`view_drop_left_until_with_size`](FoldListSlice::view_drop_left_until_with_size).
    /// Its unsized version is [`view_drop_right_until`](FoldListSlice::view_drop_right_until).
    /// 
    /// This is equivalent to ```self.view_drop_with_size(predicate..)```.
    fn view_drop_right_until_with_size(self, predicate: impl Fn(usize,&D)->bool) -> FoldListSliceFrom<'a, T, Self::OriginalD, Self::Settings, Self::Simplification, ImmFoldChainSliceStruct<'a, <<<Self::UnderlyingChain as FoldChainSlice<'a, T, (usize, Self::OriginalD)>>::IsReversed as Bool>::Not as Bool>::Not, <<Self::UnderlyingChain as FoldChainSlice<'a, T, (usize, Self::OriginalD)>>::IsFlushLeft as Bool>::And<<<Self::UnderlyingChain as FoldChainSlice<'a, T, (usize, Self::OriginalD)>>::IsReversed as Bool>::Not>, <<Self::UnderlyingChain as FoldChainSlice<'a, T, (usize, Self::OriginalD)>>::IsFlushRight as Bool>::And<<<<Self::UnderlyingChain as FoldChainSlice<'a, T, (usize, Self::OriginalD)>>::IsReversed as Bool>::Not as Bool>::Not>, SettingsWithSize<Self::Settings>, (), T, (usize, Self::OriginalD)>> {
        FoldListSliceFrom {
            simplification: self.get_current_simplification(),
            underlying: self.as_sized_chain_keeping_simplification()
                .view_drop_right_until(|(n,d)| predicate(*n,d))
                .view_unsimplify(),
            _m: PhantomData
        }
    }

    /// Restrict this view to the longest range that starts on the right and whose length and fold don't meet `predicate`.
    /// 
    /// The given `predicate` must be well-behaved; see [Slicing](crate#slicing) for examples.
    /// 
    /// This operation's mutable version is [`mut_view_take_right_until_with_size`](MutFoldListSlice::mut_view_take_right_until_with_size) and
    /// its mirror image is [`view_take_left_until_with_size`](FoldListSlice::view_take_left_until_with_size).
    /// Its unsized version is [`view_take_right_until`](FoldListSlice::view_take_right_until).
    /// 
    /// This is equivalent to ```self.view_take_with_size(predicate..)```.
    fn view_take_right_until_with_size(self, predicate: impl Fn(usize,&D)->bool) -> FoldListSliceFrom<'a, T, Self::OriginalD, Self::Settings, Self::Simplification, ImmFoldChainSliceStruct<'a, <Self::UnderlyingChain as FoldChainSlice<'a, T, (usize, Self::OriginalD)>>::IsReversed, <<Self::UnderlyingChain as FoldChainSlice<'a, T, (usize, Self::OriginalD)>>::IsFlushLeft as Bool>::And<<Self::UnderlyingChain as FoldChainSlice<'a, T, (usize, Self::OriginalD)>>::IsReversed>, <<Self::UnderlyingChain as FoldChainSlice<'a, T, (usize, Self::OriginalD)>>::IsFlushRight as Bool>::And<<<Self::UnderlyingChain as FoldChainSlice<'a, T, (usize, Self::OriginalD)>>::IsReversed as Bool>::Not>, SettingsWithSize<Self::Settings>, (), T, (usize, Self::OriginalD)>> {
        FoldListSliceFrom {
            simplification: self.get_current_simplification(),
            underlying: self.as_sized_chain_keeping_simplification()
                .view_take_right_until(|(n,d)| predicate(*n,d))
                .view_unsimplify(),
            _m: PhantomData
        }
    }

    /// Contract this view on the left while the to-be-discarded range's length and fold don't meet `predicate`.
    /// 
    /// The given `predicate` must be well-behaved; see [Slicing](crate#slicing) for examples.
    /// 
    /// This operation's mutable version is [`mut_view_drop_left_until_with_size`](MutFoldListSlice::mut_view_drop_left_until_with_size) and 
    /// its mirror image is [`view_drop_right_until_with_size`](FoldListSlice::view_drop_right_until_with_size).
    /// Its unsized version is [`view_drop_left_until`](FoldListSlice::view_drop_left_until).
    /// 
    /// This is equivalent to ```self.view_drop_with_size(..predicate)```.
    fn view_drop_left_until_with_size(self, predicate: impl Fn(usize,&D)->bool) -> FoldListSliceFrom<'a, T, Self::OriginalD, Self::Settings, Self::Simplification, ImmFoldChainSliceStruct<'a, <Self::UnderlyingChain as FoldChainSlice<'a, T, (usize, Self::OriginalD)>>::IsReversed, <<Self::UnderlyingChain as FoldChainSlice<'a, T, (usize, Self::OriginalD)>>::IsFlushLeft as Bool>::And<<Self::UnderlyingChain as FoldChainSlice<'a, T, (usize, Self::OriginalD)>>::IsReversed>, <<Self::UnderlyingChain as FoldChainSlice<'a, T, (usize, Self::OriginalD)>>::IsFlushRight as Bool>::And<<<Self::UnderlyingChain as FoldChainSlice<'a, T, (usize, Self::OriginalD)>>::IsReversed as Bool>::Not>, SettingsWithSize<Self::Settings>, (), T, (usize, Self::OriginalD)>> {
        FoldListSliceFrom {
            simplification: self.get_current_simplification(),
            underlying: self.as_sized_chain_keeping_simplification()
                .view_drop_left_until(|(n,d)| predicate(*n,d))
                .view_unsimplify(),
            _m: PhantomData
        }
    }

    /// If `range` is ```..predicate```, calls [`view_drop_left_until_with_size(predicate)`](FoldListSlice::view_drop_left_until_with_size).
    /// 
    /// If `range` is ```predicate..```, calls [`view_drop_right_until_with_size(predicate)`](FoldListSlice::view_drop_right_until_with_size) (You may need to parenthesize `predicate` in this case).
    /// 
    /// The given `predicate` must be well-behaved; see [Slicing](crate#slicing) for examples.
    /// 
    /// This operation's mutable version is [`mut_view_drop_with_size`](MutFoldListSlice::mut_view_drop_with_size) and 
    /// its unsized version is [`view_drop`](FoldListSlice::view_drop).
    fn view_drop_with_size<Predicate: Fn(usize,&D)->bool, Range: SingleEndedRange<Predicate>>(self, range: Range) -> <<Range as SingleEndedRange<Predicate>>::EndIsLeft as Bool>::IfElse<FoldListSliceFrom<'a, T, <Self as FoldListSlice<'a, T, D>>::OriginalD, <Self as FoldListSlice<'a, T, D>>::Settings, <Self as FoldListSlice<'a, T, D>>::Simplification, ImmFoldChainSliceStruct<'a, <<<<Self as FoldListSlice<'a, T, D>>::UnderlyingChain as FoldChainSlice<'a, T, (usize, <Self as FoldListSlice<'a, T, D>>::OriginalD)>>::IsReversed as Bool>::Not as Bool>::Not, <<<Self as FoldListSlice<'a, T, D>>::UnderlyingChain as FoldChainSlice<'a, T, (usize, <Self as FoldListSlice<'a, T, D>>::OriginalD)>>::IsFlushLeft as Bool>::And<<<<Self as FoldListSlice<'a, T, D>>::UnderlyingChain as FoldChainSlice<'a, T, (usize, <Self as FoldListSlice<'a, T, D>>::OriginalD)>>::IsReversed as Bool>::Not>, <<<Self as FoldListSlice<'a, T, D>>::UnderlyingChain as FoldChainSlice<'a, T, (usize, <Self as FoldListSlice<'a, T, D>>::OriginalD)>>::IsFlushRight as Bool>::And<<<<<Self as FoldListSlice<'a, T, D>>::UnderlyingChain as FoldChainSlice<'a, T, (usize, <Self as FoldListSlice<'a, T, D>>::OriginalD)>>::IsReversed as Bool>::Not as Bool>::Not>, SettingsWithSize<<Self as FoldListSlice<'a, T, D>>::Settings>, (), T, (usize, <Self as FoldListSlice<'a, T, D>>::OriginalD)>>, FoldListSliceFrom<'a, T, <Self as FoldListSlice<'a, T, D>>::OriginalD, <Self as FoldListSlice<'a, T, D>>::Settings, <Self as FoldListSlice<'a, T, D>>::Simplification, ImmFoldChainSliceStruct<'a, <<Self as FoldListSlice<'a, T, D>>::UnderlyingChain as FoldChainSlice<'a, T, (usize, <Self as FoldListSlice<'a, T, D>>::OriginalD)>>::IsReversed, <<<Self as FoldListSlice<'a, T, D>>::UnderlyingChain as FoldChainSlice<'a, T, (usize, <Self as FoldListSlice<'a, T, D>>::OriginalD)>>::IsFlushLeft as Bool>::And<<<Self as FoldListSlice<'a, T, D>>::UnderlyingChain as FoldChainSlice<'a, T, (usize, <Self as FoldListSlice<'a, T, D>>::OriginalD)>>::IsReversed>, <<<Self as FoldListSlice<'a, T, D>>::UnderlyingChain as FoldChainSlice<'a, T, (usize, <Self as FoldListSlice<'a, T, D>>::OriginalD)>>::IsFlushRight as Bool>::And<<<<Self as FoldListSlice<'a, T, D>>::UnderlyingChain as FoldChainSlice<'a, T, (usize, <Self as FoldListSlice<'a, T, D>>::OriginalD)>>::IsReversed as Bool>::Not>, SettingsWithSize<<Self as FoldListSlice<'a, T, D>>::Settings>, (), T, (usize, <Self as FoldListSlice<'a, T, D>>::OriginalD)>>> {
        Range::EndIsLeft::init_if_else((self,SingleEndedRange::end(range)), 
            |(this,end)| this.view_drop_right_until_with_size(end), 
            |(this,end)| this.view_drop_left_until_with_size(end), 
        )
    }

    /// If `range` is ```..predicate```, calls [`view_take_left_until_with_size(predicate)`](FoldListSlice::view_take_left_until_with_size).
    /// 
    /// If `range` is ```predicate..```, calls [`view_take_right_until_with_size(predicate)`](FoldListSlice::view_take_right_until_with_size) (You may need to parenthesize `predicate` in this case).
    /// 
    /// The given `predicate` must be well-behaved; see [Slicing](crate#slicing) for examples.
    /// 
    /// This operation's mutable version is [`mut_view_take_with_size`](MutFoldListSlice::mut_view_take_with_size) and
    /// its unsized version is [`view_take`](FoldListSlice::view_take).
    fn view_take_with_size<Predicate: Fn(usize,&D)->bool, Range: SingleEndedRange<Predicate>>(self, range: Range) -> <<Range as SingleEndedRange<Predicate>>::EndIsLeft as Bool>::IfElse<FoldListSliceFrom<'a, T, <Self as FoldListSlice<'a, T, D>>::OriginalD, <Self as FoldListSlice<'a, T, D>>::Settings, <Self as FoldListSlice<'a, T, D>>::Simplification, ImmFoldChainSliceStruct<'a, <<Self as FoldListSlice<'a, T, D>>::UnderlyingChain as FoldChainSlice<'a, T, (usize, <Self as FoldListSlice<'a, T, D>>::OriginalD)>>::IsReversed, <<<Self as FoldListSlice<'a, T, D>>::UnderlyingChain as FoldChainSlice<'a, T, (usize, <Self as FoldListSlice<'a, T, D>>::OriginalD)>>::IsFlushLeft as Bool>::And<<<Self as FoldListSlice<'a, T, D>>::UnderlyingChain as FoldChainSlice<'a, T, (usize, <Self as FoldListSlice<'a, T, D>>::OriginalD)>>::IsReversed>, <<<Self as FoldListSlice<'a, T, D>>::UnderlyingChain as FoldChainSlice<'a, T, (usize, <Self as FoldListSlice<'a, T, D>>::OriginalD)>>::IsFlushRight as Bool>::And<<<<Self as FoldListSlice<'a, T, D>>::UnderlyingChain as FoldChainSlice<'a, T, (usize, <Self as FoldListSlice<'a, T, D>>::OriginalD)>>::IsReversed as Bool>::Not>, SettingsWithSize<<Self as FoldListSlice<'a, T, D>>::Settings>, (), T, (usize, <Self as FoldListSlice<'a, T, D>>::OriginalD)>>, FoldListSliceFrom<'a, T, <Self as FoldListSlice<'a, T, D>>::OriginalD, <Self as FoldListSlice<'a, T, D>>::Settings, <Self as FoldListSlice<'a, T, D>>::Simplification, ImmFoldChainSliceStruct<'a, <<<<Self as FoldListSlice<'a, T, D>>::UnderlyingChain as FoldChainSlice<'a, T, (usize, <Self as FoldListSlice<'a, T, D>>::OriginalD)>>::IsReversed as Bool>::Not as Bool>::Not, <<<Self as FoldListSlice<'a, T, D>>::UnderlyingChain as FoldChainSlice<'a, T, (usize, <Self as FoldListSlice<'a, T, D>>::OriginalD)>>::IsFlushLeft as Bool>::And<<<<Self as FoldListSlice<'a, T, D>>::UnderlyingChain as FoldChainSlice<'a, T, (usize, <Self as FoldListSlice<'a, T, D>>::OriginalD)>>::IsReversed as Bool>::Not>, <<<Self as FoldListSlice<'a, T, D>>::UnderlyingChain as FoldChainSlice<'a, T, (usize, <Self as FoldListSlice<'a, T, D>>::OriginalD)>>::IsFlushRight as Bool>::And<<<<<Self as FoldListSlice<'a, T, D>>::UnderlyingChain as FoldChainSlice<'a, T, (usize, <Self as FoldListSlice<'a, T, D>>::OriginalD)>>::IsReversed as Bool>::Not as Bool>::Not>, SettingsWithSize<<Self as FoldListSlice<'a, T, D>>::Settings>, (), T, (usize, <Self as FoldListSlice<'a, T, D>>::OriginalD)>>> {
        Range::EndIsLeft::init_if_else((self,SingleEndedRange::end(range)), 
            |(this,end)| this.view_take_right_until_with_size(end), 
            |(this,end)| this.view_take_left_until_with_size(end), 
        )
    }

    /// Restrict this view to the `n` leftmost elements.
    /// 
    /// If `n` is greater than or equal to the length of this slice, this will do nothing, except possibly change this slice's type.
    /// 
    /// This operation's mutable version is [`mut_view_take_left`](MutFoldListSlice::mut_view_take_left).
    fn view_take_left(self, n: usize) -> FoldListSliceFrom<'a, T, Self::OriginalD, Self::Settings, Self::Simplification, ImmFoldChainSliceStruct<'a, <<<Self::UnderlyingChain as FoldChainSlice<'a, T, (usize, Self::OriginalD)>>::IsReversed as Bool>::Not as Bool>::Not, <<Self::UnderlyingChain as FoldChainSlice<'a, T, (usize, Self::OriginalD)>>::IsFlushLeft as Bool>::And<<<Self::UnderlyingChain as FoldChainSlice<'a, T, (usize, Self::OriginalD)>>::IsReversed as Bool>::Not>, <<Self::UnderlyingChain as FoldChainSlice<'a, T, (usize, Self::OriginalD)>>::IsFlushRight as Bool>::And<<<<Self::UnderlyingChain as FoldChainSlice<'a, T, (usize, Self::OriginalD)>>::IsReversed as Bool>::Not as Bool>::Not>, SettingsWithSize<Self::Settings>, (), T, (usize, Self::OriginalD)>> {
        FoldListSliceFrom {
            simplification: self.get_current_simplification(),
            underlying: self.as_sized_chain().view_simplify_with_shortcut(
                    |(n,_): &(_,_)| *n, 
                    |(a,b)|a+b, 
                    |()| 0, 
                    |_ : &_| 1
                ).view_take_left_until(|size: &usize| *size > n)
                .view_unsimplify(),
            _m: PhantomData
        }
    }

    /// Contract this view by `n` elements, on the right.
    /// 
    /// If `n` is `0`, this will do nothing, except possibly change this slice's type.
    /// 
    /// This operation's mutable version is [`mut_view_drop_right`](MutFoldListSlice::mut_view_drop_right).
    fn view_drop_right(self, n: usize) -> FoldListSliceFrom<'a, T, Self::OriginalD, Self::Settings, Self::Simplification, ImmFoldChainSliceStruct<'a, <<<Self::UnderlyingChain as FoldChainSlice<'a, T, (usize, Self::OriginalD)>>::IsReversed as Bool>::Not as Bool>::Not, <<Self::UnderlyingChain as FoldChainSlice<'a, T, (usize, Self::OriginalD)>>::IsFlushLeft as Bool>::And<<<Self::UnderlyingChain as FoldChainSlice<'a, T, (usize, Self::OriginalD)>>::IsReversed as Bool>::Not>, <<Self::UnderlyingChain as FoldChainSlice<'a, T, (usize, Self::OriginalD)>>::IsFlushRight as Bool>::And<<<<Self::UnderlyingChain as FoldChainSlice<'a, T, (usize, Self::OriginalD)>>::IsReversed as Bool>::Not as Bool>::Not>, SettingsWithSize<Self::Settings>, (), T, (usize, Self::OriginalD)>> {
        FoldListSliceFrom {
            simplification: self.get_current_simplification(),
            underlying: self.as_sized_chain().view_simplify_with_shortcut(
                    |(n,_): &(_,_)| *n,
                    |(a,b)|a+b, 
                    |()| 0, 
                    |_ : &_| 1
                ).view_drop_right_until(|size: &usize| *size > n)
                .view_unsimplify(),
            _m: PhantomData
        }
    }

    /// Restrict this view to the `n` rightmost elements.
    /// 
    /// If `n` is greater than or equal to the length of this slice, this will do nothing, except possibly change this slice's type.
    /// 
    /// This operation's mutable version is [`mut_view_take_right`](MutFoldListSlice::mut_view_take_right).
    fn view_take_right(self, n: usize) -> FoldListSliceFrom<'a, T, Self::OriginalD, Self::Settings, Self::Simplification, ImmFoldChainSliceStruct<'a, <Self::UnderlyingChain as FoldChainSlice<'a, T, (usize, Self::OriginalD)>>::IsReversed, <<Self::UnderlyingChain as FoldChainSlice<'a, T, (usize, Self::OriginalD)>>::IsFlushLeft as Bool>::And<<Self::UnderlyingChain as FoldChainSlice<'a, T, (usize, Self::OriginalD)>>::IsReversed>, <<Self::UnderlyingChain as FoldChainSlice<'a, T, (usize, Self::OriginalD)>>::IsFlushRight as Bool>::And<<<Self::UnderlyingChain as FoldChainSlice<'a, T, (usize, Self::OriginalD)>>::IsReversed as Bool>::Not>, SettingsWithSize<Self::Settings>, (), T, (usize, Self::OriginalD)>> {
        FoldListSliceFrom {
            simplification: self.get_current_simplification(),
            underlying: self.as_sized_chain().view_simplify_with_shortcut(
                    |(n,_): &(_,_)| *n, 
                    |(a,b)|a+b, 
                    |()| 0, 
                    |_ : &_| 1
                ).view_take_right_until(|size: &usize| *size > n)
                .view_unsimplify(),
            _m: PhantomData
        }
    }

    /// Contract this view by `n` elements, on the left.
    /// 
    /// If `n` is `0`, this will do nothing, except possibly change this slice's type.
    /// 
    /// This operation's mutable version is [`mut_view_drop_left`](MutFoldListSlice::mut_view_drop_left).
    fn view_drop_left(self, n: usize) -> FoldListSliceFrom<'a, T, Self::OriginalD, Self::Settings, Self::Simplification, ImmFoldChainSliceStruct<'a, <Self::UnderlyingChain as FoldChainSlice<'a, T, (usize, Self::OriginalD)>>::IsReversed, <<Self::UnderlyingChain as FoldChainSlice<'a, T, (usize, Self::OriginalD)>>::IsFlushLeft as Bool>::And<<Self::UnderlyingChain as FoldChainSlice<'a, T, (usize, Self::OriginalD)>>::IsReversed>, <<Self::UnderlyingChain as FoldChainSlice<'a, T, (usize, Self::OriginalD)>>::IsFlushRight as Bool>::And<<<Self::UnderlyingChain as FoldChainSlice<'a, T, (usize, Self::OriginalD)>>::IsReversed as Bool>::Not>, SettingsWithSize<Self::Settings>, (), T, (usize, Self::OriginalD)>> {
        FoldListSliceFrom {
            simplification: self.get_current_simplification(),
            underlying: self.as_sized_chain().view_simplify_with_shortcut(
                    |(n,_): &(_,_)| *n, 
                    |(a,b)|a+b, 
                    |()| 0, 
                    |_ : &_| 1
                ).view_drop_left_until(|size: &usize| *size > n)
                .view_unsimplify(),
            _m: PhantomData
        }
    }

    /// Get a reversed version of this view. See [Reverse](crate#reverse).
    /// 
    /// This operation's mutable version is [`mut_view_reversed`](MutFoldListSlice::mut_view_reversed).
    fn view_reversed(self) -> FoldListSliceFrom<'a, T, Self::OriginalD, Self::Settings, Self::Simplification, ImmFoldChainSliceStruct<'a, <<Self::UnderlyingChain as FoldChainSlice<'a, T, (usize, Self::OriginalD)>>::IsReversed as Bool>::Not, <Self::UnderlyingChain as FoldChainSlice<'a, T, (usize, Self::OriginalD)>>::IsFlushLeft, <Self::UnderlyingChain as FoldChainSlice<'a, T, (usize, Self::OriginalD)>>::IsFlushRight, SettingsWithSize<Self::Settings>, (), T, (usize, Self::OriginalD)>> {
        FoldListSliceFrom {
            simplification: self.get_current_simplification(),
            underlying: self.as_sized_chain().view_reversed(),
            _m: PhantomData
        }
    }

    /// Compose this view's current simplification with another one explicitly. See [Simplification](crate#simplification).
    /// 
    /// This operation's mutable version is [`mut_view_with_simplification`](MutFoldListSlice::mut_view_with_simplification).
    fn view_with_simplification<NewSimplification: FoldSimplification<T,D>>(self, new_simplification: NewSimplification) -> FoldListSliceFrom<'a, T, Self::OriginalD, Self::Settings, <NewSimplification as FoldSimplification<T, D>>::ComposeAfterOther<Self::OriginalD, Self::Simplification>, ImmFoldChainSliceStruct<'a, <Self::UnderlyingChain as FoldChainSlice<'a, T, (usize, Self::OriginalD)>>::IsReversed, <Self::UnderlyingChain as FoldChainSlice<'a, T, (usize, Self::OriginalD)>>::IsFlushLeft, <Self::UnderlyingChain as FoldChainSlice<'a, T, (usize, Self::OriginalD)>>::IsFlushRight, SettingsWithSize<Self::Settings>, (), T, (usize, Self::OriginalD)>>  {
        FoldListSliceFrom {
            simplification: new_simplification.compose_after_other(self.get_current_simplification()),
            underlying: self.as_sized_chain().as_imm(),
            _m: PhantomData
        }
    }

    /// Simplify this view. See [Simplification](crate#simplification).
    /// 
    /// This operation's mutable version is [`mut_view_simplify`](MutFoldListSlice::mut_view_simplify).
    fn view_simplify<D2: Clone + 'a, 
        Simplifier: for<'x> Fun<&'x D,D2> + Copy + 'a,
        OP2: Fun<(D2,D2),D2> + Copy + 'a>(self,simplifier: Simplifier, simplified_op: OP2) -> FoldListSliceFrom<'a, T, Self::OriginalD, Self::Settings, 
            <SimplificationWithoutShortcut<T,D,D2,Simplifier,OP2> as FoldSimplification<T,D>>::ComposeAfterOther<Self::OriginalD,Self::Simplification>, 
            ImmFoldChainSliceStruct<'a, <Self::UnderlyingChain as FoldChainSlice<'a, T, (usize, Self::OriginalD)>>::IsReversed, <Self::UnderlyingChain as FoldChainSlice<'a, T, (usize, Self::OriginalD)>>::IsFlushLeft, <Self::UnderlyingChain as FoldChainSlice<'a, T, (usize, Self::OriginalD)>>::IsFlushRight, SettingsWithSize<Self::Settings>, (), T, (usize, Self::OriginalD)>> {
                self.view_with_simplification(SimplificationWithoutShortcut {
                    simplifier,
                    op2: simplified_op,
                    _m: PhantomData,
                })
    }

    /// Simplify this view in a possibly more efficient way. See [Simplification](crate#simplification).
    /// 
    /// This operation's mutable version is [`mut_view_simplify_with_shortcut`](MutFoldListSlice::mut_view_simplify_with_shortcut).
    fn view_simplify_with_shortcut<D2: Clone + 'a, 
        Simplifier: for<'x> Fun<&'x D,D2> + Copy + 'a,
        OP2: Fun<(D2,D2),D2> + Copy + 'a,
        EmptyShortcut: Fun<(),D2> + Copy + 'a,
        DeltaShortcut: for<'x> Fun<&'x T, D2> + Copy + 'a>(self,simplifier: Simplifier, simplified_op: OP2, empty_shortcut: EmptyShortcut, delta_shortcut: DeltaShortcut) -> FoldListSliceFrom<'a, T, Self::OriginalD, Self::Settings,
            <SimplificationWithShortcut<T,D,D2,Simplifier,OP2,EmptyShortcut,DeltaShortcut> as FoldSimplification<T,D>>::ComposeAfterOther<Self::OriginalD,Self::Simplification>, 
            ImmFoldChainSliceStruct<'a, <Self::UnderlyingChain as FoldChainSlice<'a, T, (usize, Self::OriginalD)>>::IsReversed, <Self::UnderlyingChain as FoldChainSlice<'a, T, (usize, Self::OriginalD)>>::IsFlushLeft, <Self::UnderlyingChain as FoldChainSlice<'a, T, (usize, Self::OriginalD)>>::IsFlushRight, SettingsWithSize<Self::Settings>, (), T, (usize, Self::OriginalD)>> {
            self.view_with_simplification(SimplificationWithShortcut {
                simplifier,
                op2: simplified_op,
                empty_shortcut,
                delta_shortcut,
                _m: PhantomData,
            })
    }

    /// Remove all simplifications that were applied to this view. See [Simplification](crate#simplification).
    /// 
    /// The [current simplification](FoldListSlice::get_current_simplification) of the resulting view will be `()`.
    /// 
    /// This operation's mutable version is [`mut_view_unsimplify`](MutFoldListSlice::mut_view_unsimplify).
    fn view_unsimplify(self) -> FoldListSliceFrom<'a, T, Self::OriginalD, Self::Settings, (), ImmFoldChainSliceStruct<'a, <Self::UnderlyingChain as FoldChainSlice<'a, T, (usize, Self::OriginalD)>>::IsReversed, <Self::UnderlyingChain as FoldChainSlice<'a, T, (usize, Self::OriginalD)>>::IsFlushLeft, <Self::UnderlyingChain as FoldChainSlice<'a, T, (usize, Self::OriginalD)>>::IsFlushRight, SettingsWithSize<Self::Settings>, (), T, (usize, Self::OriginalD)>>  {
        FoldListSliceFrom {
            simplification: (),
            underlying: self.as_sized_chain().as_imm(),
            _m: PhantomData
        }
    }

    /// Get this slice's fold.
    /// 
    /// Note that this is *O*(log(n)) every time.
    fn fold(&self) -> D {
        self.borrow().as_unsized_chain_keeping_simplification().fold()
    }

    /// Get this slice's current number of elements.
    /// 
    /// Note that this is *O*(log(n)), because it's just calling [`fold`](FoldListSlice::fold) under the hood (after a simplification).
    /// 
    /// (Actually, if the underlying [`FoldChainSlice`] is flush on [both](FoldChainSlice::IsFlushLeft) [sides](FoldChainSlice::IsFlushRight), 
    /// it will be *O*(1). But it will be *O*(log(n)) in all other cases. This is also true of [`fold`](FoldListSlice::fold).)
    fn len(&self) -> usize {
        self.borrow()
            .as_sized_chain()
            .view_simplify_with_shortcut(
                |(n,_): &(usize,_)| *n, 
                |(a,b)|a+b, 
                |()| 0, 
                |_ : &_| 1
            ).fold()
    }

    /// Returns true if this slice is empty (that is, when it contains 0 elements).
    /// 
    /// Note that it's sometimes possible for a non-empty slice's fold to be equal to the empty delta.
    fn is_empty(&self) -> bool {
        self.borrow().as_sized_chain().is_empty()
    }

    /// Get an immutable reference to this slice's leftmost element, if this slice is not empty.
    /// 
    /// Note that if you use this reference to change the element's delta, the backing data structure won't notice, and its folds won't update correctly; see [Mutation / Indexing](crate#mutation--indexing).
    /// 
    /// To mutate this element, see [`update_left`](MutFoldListSlice::update_left) or [`set_left_or_err`](MutFoldListSlice::set_left_or_err).
    fn left<'b>(&'b self) -> Option<&'b T> where 'a: 'b {
        self.borrow().underlying.left_consume()
    }
    
    /// Get an immutable reference to this slice's rightmost element, if this slice is not empty.
    /// 
    /// Note that if you use this reference to change the element's delta, the backing data structure won't notice, and its folds won't update correctly; see [Mutation / Indexing](crate#mutation--indexing).
    /// 
    /// To mutate this element, see [`update_right`](MutFoldListSlice::update_right) or [`set_right_or_err`](MutFoldListSlice::set_right_or_err).
    fn right<'b>(&'b self) -> Option<&'b T> where 'a: 'b {
        self.borrow().underlying.view_reversed().left_consume()
    }

    /// Get a reference to the element at index `index`. 
    /// 
    /// This can also be accomplished via ```&self[index]```.
    fn get<'b>(&'b self, index: usize) -> &'b T where 'a:'b{
        self.borrow().view_drop_left(index).underlying.left_consume()
            .unwrap_or_else(|| panic!("Index out of bounds: the index is {} but the length is {}",index,self.len()))
    }

    /// Run a closure for each of this slice's elements, from left to right.
    /// 
    /// See also [`iter`](FoldListSlice::iter).
    /// 
    /// This operation's mutable version is [`foreach_mut`](MutFoldListSlice::foreach_mut).
    fn foreach(&self, f: impl FnMut(&T)) {
        self.borrow().as_sized_chain().foreach(f);
    }

    /// Get an iterator over immutable references to this slice's elements, from left to right.
    /// 
    /// Note that if you use any such reference to change its element's delta, the backing data structure won't notice, and its folds won't update correctly; see [Mutation / Indexing](crate#mutation--indexing).
    /// 
    /// This operation has no mutable version for this same reason, and because in rust, you can't require that an [Iterator] outlive all that it emits (that would be a [streaming iterator](https://docs.rs/streaming-iterator/latest/streaming_iterator/), which is totally different).
    /// 
    /// The closest analogue to a mutable version is [`foreach_mut`](MutFoldListSlice::foreach_mut).
    fn iter<'b>(&'b self) -> Iter<'b, <Self::UnderlyingChain as FoldChainSlice<'a, T, (usize, Self::OriginalD)>>::IsReversed, T, (usize, Self::OriginalD)> where 'a: 'b {
        self.borrow().underlying.iter_consume()
    }
    
    /// [`debug_assert!`] that the backing data structure is in a valid state. You should never have to use this.
    fn debug_check_structural_integrity(&self) -> bool {
        debug_assert!(self.borrow().as_sized_chain().debug_check_structural_integrity());
        true
    }
}

/// The trait for mutable views into a [`FoldList`].
/// 
/// This is a subtrait of [`FoldListSlice`].
/// 
/// This trait is sealed and cannot be implemented for types outside this crate.
pub trait MutFoldListSlice<'a,T: 'a,D: Clone + 'a>: FoldListSlice<'a,T,D> where Self::UnderlyingChain : MutFoldChainSlice<'a,T,(usize,Self::OriginalD)> + Sealed {
    /// Normalize this view's type. This is mostly useless.
    fn as_mut(self) -> FoldListSliceFrom<'a,T,Self::OriginalD,
        Self::Settings,
        Self::Simplification,
        MutFoldChainSliceStruct<'a,
            <Self::UnderlyingChain as FoldChainSlice<'a,T,(usize,Self::OriginalD)>>::IsReversed,
            <Self::UnderlyingChain as FoldChainSlice<'a,T,(usize,Self::OriginalD)>>::IsFlushLeft,
            <Self::UnderlyingChain as FoldChainSlice<'a,T,(usize,Self::OriginalD)>>::IsFlushRight,
            T,
            (usize,Self::OriginalD),
            SettingsWithSize<Self::Settings>,
            ()>>;
    
    /// Mutably borrow this view.
    fn borrow_mut<'b>(&'b mut self) -> FoldListSliceFrom<'b,T,Self::OriginalD,
        Self::Settings,
        Self::Simplification,
        MutFoldChainSliceStruct<'b,
            <Self::UnderlyingChain as FoldChainSlice<'a,T,(usize,Self::OriginalD)>>::IsReversed,
            <Self::UnderlyingChain as FoldChainSlice<'a,T,(usize,Self::OriginalD)>>::IsFlushLeft,
            <Self::UnderlyingChain as FoldChainSlice<'a,T,(usize,Self::OriginalD)>>::IsFlushRight,
            T,
            (usize,Self::OriginalD),
            SettingsWithSize<Self::Settings>,
            ()>>;

    /// Get the underlying [`MutFoldChainSlice`] of this slice, and then apply a simplification to it that makes it ignore its size information. See [FoldChain](crate#foldchain).
    /// 
    /// This operation's immutable version is [`as_unsized_chain`](FoldListSlice::as_unsized_chain).
    fn mut_as_unsized_chain(self) -> MutFoldChainSliceStruct<'a, <Self::UnderlyingChain as FoldChainSlice<'a, T, (usize, Self::OriginalD)>>::IsReversed, <Self::UnderlyingChain as FoldChainSlice<'a, T, (usize, Self::OriginalD)>>::IsFlushLeft, <Self::UnderlyingChain as FoldChainSlice<'a, T, (usize, Self::OriginalD)>>::IsFlushRight, T, (usize, Self::OriginalD), SettingsWithSize<Self::Settings>, SimplificationWithoutShortcut<T, (usize, Self::OriginalD), Self::OriginalD, SizeIgnoreFn, OpFromSettings<T, Self::OriginalD, Self::Settings>>> {
        let underlying = self.as_sized_chain().as_mut();
        let SettingsWithSize(settings) = underlying.get_settings();
        underlying.mut_view_simplify(SizeIgnoreFn,
            OpFromSettings{ 
                settings, 
                _m: PhantomData 
            }
        )
    }

    /// Get the underlying [`MutFoldChainSlice`] of this slice, keeping the current simplification. See [FoldChain](crate#foldchain).
    /// 
    /// This operation's immutable version is [`as_sized_chain_keeping_simplification`](FoldListSlice::as_sized_chain_keeping_simplification).
    fn mut_as_sized_chain_keeping_simplification(self) -> MutFoldChainSliceStruct<'a,
        <Self::UnderlyingChain as FoldChainSlice<'a, T, (usize, Self::OriginalD)>>::IsReversed, 
        <Self::UnderlyingChain as FoldChainSlice<'a, T, (usize, Self::OriginalD)>>::IsFlushLeft, 
        <Self::UnderlyingChain as FoldChainSlice<'a, T, (usize, Self::OriginalD)>>::IsFlushRight, 
        T, (usize, Self::OriginalD), 
        SettingsWithSize<Self::Settings>, 
        <Self::Simplification as FoldSimplification<T,Self::OriginalD>>::WithSize> {
            let simplification: Self::Simplification = self.get_current_simplification();
            let underlying = self.as_sized_chain().as_mut();
            MutFoldChainSliceStruct {
                endpoints: underlying.endpoints,
                simplification: simplification.with_size(),
                _m: PhantomData,
            }
    }

    /// Get the underlying [`MutFoldChainSlice`] of this slice, and then apply a simplification to it that makes it ignore its size information, and then also apply the current simplification to the result. See [FoldChain](crate#foldchain).
    /// 
    /// This operation's immutable version is [`as_unsized_chain_keeping_simplification`](FoldListSlice::as_unsized_chain_keeping_simplification).
    fn mut_as_unsized_chain_keeping_simplification(self) -> MutFoldChainSliceStruct<'a, 
        <Self::UnderlyingChain as FoldChainSlice<'a, T, (usize, Self::OriginalD)>>::IsReversed, 
        <Self::UnderlyingChain as FoldChainSlice<'a, T, (usize, Self::OriginalD)>>::IsFlushLeft, 
        <Self::UnderlyingChain as FoldChainSlice<'a, T, (usize, Self::OriginalD)>>::IsFlushRight, 
        T, (usize, Self::OriginalD), 
        SettingsWithSize<Self::Settings>, 
        <Self::Simplification as FoldSimplification<T, Self::OriginalD>>::ComposeAfterOther<(usize, Self::OriginalD), 
        SimplificationWithoutShortcut<T, (usize, Self::OriginalD), Self::OriginalD, SizeIgnoreFn, OpFromSettings<T, Self::OriginalD, Self::Settings>>>> {
            let simplification: Self::Simplification = self.get_current_simplification();
            let underlying_without_simplification = self.mut_as_unsized_chain();
            underlying_without_simplification.mut_view_with_simplification(simplification)
    }

    /// Contract this view on the left while the to-be-discarded range's fold doesn't meet `predicate`.
    /// 
    /// The given `predicate` must be well-behaved; see [Slicing](crate#slicing) for examples.
    /// 
    /// This operation's immutable version is [`view_drop_left_until`](FoldListSlice::view_drop_left_until) and 
    /// its mirror image is [`mut_view_drop_right_until`](MutFoldListSlice::mut_view_drop_right_until).
    /// 
    /// To also take into account the length of the range, see [`mut_view_drop_left_until_with_size`](MutFoldListSlice::mut_view_drop_left_until_with_size).
    /// 
    /// This is equivalent to ```self.mut_view_drop(..predicate)```.
    fn mut_view_drop_left_until(self, predicate: impl Fn(&D)->bool) -> FoldListSliceFrom<'a, T, Self::OriginalD, Self::Settings, Self::Simplification, MutFoldChainSliceStruct<'a, <Self::UnderlyingChain as FoldChainSlice<'a, T, (usize, Self::OriginalD)>>::IsReversed, <<Self::UnderlyingChain as FoldChainSlice<'a, T, (usize, Self::OriginalD)>>::IsFlushLeft as Bool>::And<<Self::UnderlyingChain as FoldChainSlice<'a, T, (usize, Self::OriginalD)>>::IsReversed>, <<Self::UnderlyingChain as FoldChainSlice<'a, T, (usize, Self::OriginalD)>>::IsFlushRight as Bool>::And<<<Self::UnderlyingChain as FoldChainSlice<'a, T, (usize, Self::OriginalD)>>::IsReversed as Bool>::Not>, T, (usize, Self::OriginalD), SettingsWithSize<Self::Settings>, ()>> {
        FoldListSliceFrom {
            simplification: self.get_current_simplification(),
            underlying: self.mut_as_unsized_chain_keeping_simplification()
                .mut_view_drop_left_until(predicate)
                .mut_view_unsimplify(),
            _m: PhantomData,
        }
    }

    /// Restrict this view to the longest range that starts on the right and whose fold doesn't meet `predicate`.
    /// 
    /// The given `predicate` must be well-behaved; see [Slicing](crate#slicing) for examples.
    /// 
    /// This operation's immutable version is [`view_take_right_until`](FoldListSlice::view_take_right_until) and
    /// its mirror image is [`mut_view_take_left_until`](MutFoldListSlice::mut_view_take_left_until).
    /// 
    /// To also take into account the length of the range, see [`mut_view_take_right_until_with_size`](MutFoldListSlice::mut_view_take_right_until_with_size).
    /// 
    /// This is equivalent to ```self.mut_view_take(predicate..)```.
    fn mut_view_take_right_until(self, predicate: impl Fn(&D)->bool) -> FoldListSliceFrom<'a, T, Self::OriginalD, Self::Settings, Self::Simplification, MutFoldChainSliceStruct<'a, <Self::UnderlyingChain as FoldChainSlice<'a, T, (usize, Self::OriginalD)>>::IsReversed, <<Self::UnderlyingChain as FoldChainSlice<'a, T, (usize, Self::OriginalD)>>::IsFlushLeft as Bool>::And<<Self::UnderlyingChain as FoldChainSlice<'a, T, (usize, Self::OriginalD)>>::IsReversed>, <<Self::UnderlyingChain as FoldChainSlice<'a, T, (usize, Self::OriginalD)>>::IsFlushRight as Bool>::And<<<Self::UnderlyingChain as FoldChainSlice<'a, T, (usize, Self::OriginalD)>>::IsReversed as Bool>::Not>, T, (usize, Self::OriginalD), SettingsWithSize<Self::Settings>, ()>> {
        FoldListSliceFrom {
            simplification: self.get_current_simplification(),
            underlying: self.mut_as_unsized_chain_keeping_simplification()
                .mut_view_take_right_until(predicate)
                .mut_view_unsimplify(),
            _m: PhantomData,
        }
    }

    /// Contract this view on the right while the to-be-discarded range's fold doesn't meet `predicate`.
    /// 
    /// The given `predicate` must be well-behaved; see [Slicing](crate#slicing) for examples.
    /// 
    /// This operation's immutable version is [`view_drop_right_until`](FoldListSlice::view_drop_right_until) and 
    /// its mirror image is [`mut_view_drop_left_until`](MutFoldListSlice::mut_view_drop_left_until).
    /// 
    /// To also take into account the length of the range, see [`mut_view_drop_right_until_with_size`](MutFoldListSlice::mut_view_drop_right_until_with_size).
    /// 
    /// This is equivalent to ```self.mut_view_drop(predicate..)```.
    fn mut_view_drop_right_until(self, predicate: impl Fn(&D)->bool) -> FoldListSliceFrom<'a, T, Self::OriginalD, Self::Settings, Self::Simplification, MutFoldChainSliceStruct<'a, <<<Self::UnderlyingChain as FoldChainSlice<'a, T, (usize, Self::OriginalD)>>::IsReversed as Bool>::Not as Bool>::Not, <<Self::UnderlyingChain as FoldChainSlice<'a, T, (usize, Self::OriginalD)>>::IsFlushLeft as Bool>::And<<<Self::UnderlyingChain as FoldChainSlice<'a, T, (usize, Self::OriginalD)>>::IsReversed as Bool>::Not>, <<Self::UnderlyingChain as FoldChainSlice<'a, T, (usize, Self::OriginalD)>>::IsFlushRight as Bool>::And<<<<Self::UnderlyingChain as FoldChainSlice<'a, T, (usize, Self::OriginalD)>>::IsReversed as Bool>::Not as Bool>::Not>, T, (usize, Self::OriginalD), SettingsWithSize<Self::Settings>, ()>> {
        FoldListSliceFrom {
            simplification: self.get_current_simplification(),
            underlying: self.mut_as_unsized_chain_keeping_simplification()
                .mut_view_drop_right_until(predicate)
                .mut_view_unsimplify(),
            _m: PhantomData,
        }
    }

    /// Restrict this view to the longest range that starts on the left and whose fold doesn't meet `predicate`.
    /// 
    /// The given `predicate` must be well-behaved; see [Slicing](crate#slicing) for examples.
    /// 
    /// This operation's immutable version is [`view_take_left_until`](FoldListSlice::view_take_left_until) and
    /// its mirror image is [`mut_view_take_right_until`](MutFoldListSlice::mut_view_take_right_until).
    /// 
    /// To also take into account the length of the range, see [`mut_view_take_left_until_with_size`](MutFoldListSlice::mut_view_take_left_until_with_size).
    /// 
    /// This is equivalent to ```self.mut_view_take(..predicate)```.
    fn mut_view_take_left_until(self, predicate: impl Fn(&D)->bool) -> FoldListSliceFrom<'a, T, Self::OriginalD, Self::Settings, Self::Simplification, MutFoldChainSliceStruct<'a, <<<Self::UnderlyingChain as FoldChainSlice<'a, T, (usize, Self::OriginalD)>>::IsReversed as Bool>::Not as Bool>::Not, <<Self::UnderlyingChain as FoldChainSlice<'a, T, (usize, Self::OriginalD)>>::IsFlushLeft as Bool>::And<<<Self::UnderlyingChain as FoldChainSlice<'a, T, (usize, Self::OriginalD)>>::IsReversed as Bool>::Not>, <<Self::UnderlyingChain as FoldChainSlice<'a, T, (usize, Self::OriginalD)>>::IsFlushRight as Bool>::And<<<<Self::UnderlyingChain as FoldChainSlice<'a, T, (usize, Self::OriginalD)>>::IsReversed as Bool>::Not as Bool>::Not>, T, (usize, Self::OriginalD), SettingsWithSize<Self::Settings>, ()>> {
        FoldListSliceFrom {
            simplification: self.get_current_simplification(),
            underlying: self.mut_as_unsized_chain_keeping_simplification()
                .mut_view_take_left_until(predicate)
                .mut_view_unsimplify(),
            _m: PhantomData,
        }
    }

    /// If `range` is ```..predicate```, calls [`mut_view_drop_left_until(predicate)`](MutFoldListSlice::mut_view_drop_left_until).
    /// 
    /// If `range` is ```predicate..```, calls [`mut_view_drop_right_until(predicate)`](MutFoldListSlice::mut_view_drop_right_until) (You may need to parenthesize `predicate` in this case).
    /// 
    /// The given `predicate` must be well-behaved; see [Slicing](crate#slicing) for examples.
    /// 
    /// This operation's immutable version is [`view_drop`](FoldListSlice::view_drop).
    fn mut_view_drop<Predicate: Fn(&D)->bool, Range: SingleEndedRange<Predicate>>(self, range: Range) -> <<Range as SingleEndedRange<Predicate>>::EndIsLeft as Bool>::IfElse<FoldListSliceFrom<'a, T, <Self as FoldListSlice<'a, T, D>>::OriginalD, <Self as FoldListSlice<'a, T, D>>::Settings, <Self as FoldListSlice<'a, T, D>>::Simplification, MutFoldChainSliceStruct<'a, <<<<Self as FoldListSlice<'a, T, D>>::UnderlyingChain as FoldChainSlice<'a, T, (usize, <Self as FoldListSlice<'a, T, D>>::OriginalD)>>::IsReversed as Bool>::Not as Bool>::Not, <<<Self as FoldListSlice<'a, T, D>>::UnderlyingChain as FoldChainSlice<'a, T, (usize, <Self as FoldListSlice<'a, T, D>>::OriginalD)>>::IsFlushLeft as Bool>::And<<<<Self as FoldListSlice<'a, T, D>>::UnderlyingChain as FoldChainSlice<'a, T, (usize, <Self as FoldListSlice<'a, T, D>>::OriginalD)>>::IsReversed as Bool>::Not>, <<<Self as FoldListSlice<'a, T, D>>::UnderlyingChain as FoldChainSlice<'a, T, (usize, <Self as FoldListSlice<'a, T, D>>::OriginalD)>>::IsFlushRight as Bool>::And<<<<<Self as FoldListSlice<'a, T, D>>::UnderlyingChain as FoldChainSlice<'a, T, (usize, <Self as FoldListSlice<'a, T, D>>::OriginalD)>>::IsReversed as Bool>::Not as Bool>::Not>, T, (usize, <Self as FoldListSlice<'a, T, D>>::OriginalD), SettingsWithSize<<Self as FoldListSlice<'a, T, D>>::Settings>, ()>>, FoldListSliceFrom<'a, T, <Self as FoldListSlice<'a, T, D>>::OriginalD, <Self as FoldListSlice<'a, T, D>>::Settings, <Self as FoldListSlice<'a, T, D>>::Simplification, MutFoldChainSliceStruct<'a, <<Self as FoldListSlice<'a, T, D>>::UnderlyingChain as FoldChainSlice<'a, T, (usize, <Self as FoldListSlice<'a, T, D>>::OriginalD)>>::IsReversed, <<<Self as FoldListSlice<'a, T, D>>::UnderlyingChain as FoldChainSlice<'a, T, (usize, <Self as FoldListSlice<'a, T, D>>::OriginalD)>>::IsFlushLeft as Bool>::And<<<Self as FoldListSlice<'a, T, D>>::UnderlyingChain as FoldChainSlice<'a, T, (usize, <Self as FoldListSlice<'a, T, D>>::OriginalD)>>::IsReversed>, <<<Self as FoldListSlice<'a, T, D>>::UnderlyingChain as FoldChainSlice<'a, T, (usize, <Self as FoldListSlice<'a, T, D>>::OriginalD)>>::IsFlushRight as Bool>::And<<<<Self as FoldListSlice<'a, T, D>>::UnderlyingChain as FoldChainSlice<'a, T, (usize, <Self as FoldListSlice<'a, T, D>>::OriginalD)>>::IsReversed as Bool>::Not>, T, (usize, <Self as FoldListSlice<'a, T, D>>::OriginalD), SettingsWithSize<<Self as FoldListSlice<'a, T, D>>::Settings>, ()>>> {
        Range::EndIsLeft::init_if_else((self,SingleEndedRange::end(range)), 
            |(this,end)| this.mut_view_drop_right_until(end), 
            |(this,end)| this.mut_view_drop_left_until(end), 
        )
    }

    /// If `range` is ```..predicate```, calls [`mut_view_take_left_until(predicate)`](MutFoldListSlice::mut_view_take_left_until).
    /// 
    /// If `range` is ```predicate..```, calls [`mut_view_take_right_until(predicate)`](MutFoldListSlice::mut_view_take_right_until) (You may need to parenthesize `predicate` in this case).
    /// 
    /// The given `predicate` must be well-behaved; see [Slicing](crate#slicing) for examples.
    /// 
    /// This operation's immutable version is [`view_take`](FoldListSlice::view_take).
    fn mut_view_take<Predicate: Fn(&D)->bool, Range: SingleEndedRange<Predicate>>(self, range: Range) -> <<Range as SingleEndedRange<Predicate>>::EndIsLeft as Bool>::IfElse<FoldListSliceFrom<'a, T, <Self as FoldListSlice<'a, T, D>>::OriginalD, <Self as FoldListSlice<'a, T, D>>::Settings, <Self as FoldListSlice<'a, T, D>>::Simplification, MutFoldChainSliceStruct<'a, <<Self as FoldListSlice<'a, T, D>>::UnderlyingChain as FoldChainSlice<'a, T, (usize, <Self as FoldListSlice<'a, T, D>>::OriginalD)>>::IsReversed, <<<Self as FoldListSlice<'a, T, D>>::UnderlyingChain as FoldChainSlice<'a, T, (usize, <Self as FoldListSlice<'a, T, D>>::OriginalD)>>::IsFlushLeft as Bool>::And<<<Self as FoldListSlice<'a, T, D>>::UnderlyingChain as FoldChainSlice<'a, T, (usize, <Self as FoldListSlice<'a, T, D>>::OriginalD)>>::IsReversed>, <<<Self as FoldListSlice<'a, T, D>>::UnderlyingChain as FoldChainSlice<'a, T, (usize, <Self as FoldListSlice<'a, T, D>>::OriginalD)>>::IsFlushRight as Bool>::And<<<<Self as FoldListSlice<'a, T, D>>::UnderlyingChain as FoldChainSlice<'a, T, (usize, <Self as FoldListSlice<'a, T, D>>::OriginalD)>>::IsReversed as Bool>::Not>, T, (usize, <Self as FoldListSlice<'a, T, D>>::OriginalD), SettingsWithSize<<Self as FoldListSlice<'a, T, D>>::Settings>, ()>>, FoldListSliceFrom<'a, T, <Self as FoldListSlice<'a, T, D>>::OriginalD, <Self as FoldListSlice<'a, T, D>>::Settings, <Self as FoldListSlice<'a, T, D>>::Simplification, MutFoldChainSliceStruct<'a, <<<<Self as FoldListSlice<'a, T, D>>::UnderlyingChain as FoldChainSlice<'a, T, (usize, <Self as FoldListSlice<'a, T, D>>::OriginalD)>>::IsReversed as Bool>::Not as Bool>::Not, <<<Self as FoldListSlice<'a, T, D>>::UnderlyingChain as FoldChainSlice<'a, T, (usize, <Self as FoldListSlice<'a, T, D>>::OriginalD)>>::IsFlushLeft as Bool>::And<<<<Self as FoldListSlice<'a, T, D>>::UnderlyingChain as FoldChainSlice<'a, T, (usize, <Self as FoldListSlice<'a, T, D>>::OriginalD)>>::IsReversed as Bool>::Not>, <<<Self as FoldListSlice<'a, T, D>>::UnderlyingChain as FoldChainSlice<'a, T, (usize, <Self as FoldListSlice<'a, T, D>>::OriginalD)>>::IsFlushRight as Bool>::And<<<<<Self as FoldListSlice<'a, T, D>>::UnderlyingChain as FoldChainSlice<'a, T, (usize, <Self as FoldListSlice<'a, T, D>>::OriginalD)>>::IsReversed as Bool>::Not as Bool>::Not>, T, (usize, <Self as FoldListSlice<'a, T, D>>::OriginalD), SettingsWithSize<<Self as FoldListSlice<'a, T, D>>::Settings>, ()>>> {
        Range::EndIsLeft::init_if_else((self,SingleEndedRange::end(range)), 
            |(this,end)| this.mut_view_take_right_until(end), 
            |(this,end)| this.mut_view_take_left_until(end), 
        )
    }

    /// Contract this view on the left while the to-be-discarded range's length and fold don't meet `predicate`.
    /// 
    /// The given `predicate` must be well-behaved; see [Slicing](crate#slicing) for examples.
    /// 
    /// This operation's immutable version is [`view_drop_left_until_with_size`](FoldListSlice::view_drop_left_until_with_size) and 
    /// its mirror image is [`mut_view_drop_right_until_with_size`](MutFoldListSlice::mut_view_drop_right_until_with_size).
    /// Its unsized version is [`mut_view_drop_left_until`](MutFoldListSlice::mut_view_drop_left_until).
    /// 
    /// This is equivalent to ```self.mut_view_drop_with_size(..predicate)```.
    fn mut_view_drop_left_until_with_size(self, predicate: impl Fn(usize,&D)->bool) -> FoldListSliceFrom<'a, T, Self::OriginalD, Self::Settings, Self::Simplification, MutFoldChainSliceStruct<'a, <Self::UnderlyingChain as FoldChainSlice<'a, T, (usize, Self::OriginalD)>>::IsReversed, <<Self::UnderlyingChain as FoldChainSlice<'a, T, (usize, Self::OriginalD)>>::IsFlushLeft as Bool>::And<<Self::UnderlyingChain as FoldChainSlice<'a, T, (usize, Self::OriginalD)>>::IsReversed>, <<Self::UnderlyingChain as FoldChainSlice<'a, T, (usize, Self::OriginalD)>>::IsFlushRight as Bool>::And<<<Self::UnderlyingChain as FoldChainSlice<'a, T, (usize, Self::OriginalD)>>::IsReversed as Bool>::Not>, T, (usize, Self::OriginalD), SettingsWithSize<Self::Settings>, ()>> {
        FoldListSliceFrom {
            simplification: self.get_current_simplification(),
            underlying: self.mut_as_sized_chain_keeping_simplification()
                .mut_view_drop_left_until(|(n,d)| predicate(*n,d))
                .mut_view_unsimplify(),
            _m: PhantomData
        }
    }

    /// Restrict this view to the longest range that starts on the right and whose length and fold don't meet `predicate`.
    /// 
    /// The given `predicate` must be well-behaved; see [Slicing](crate#slicing) for examples.
    /// 
    /// This operation's immutable version is [`view_take_right_until_with_size`](FoldListSlice::view_take_right_until_with_size) and
    /// its mirror image is [`mut_view_take_left_until_with_size`](MutFoldListSlice::mut_view_take_left_until_with_size).
    /// Its unsized version is [`mut_view_take_right_until`](MutFoldListSlice::mut_view_take_right_until).
    /// 
    /// This is equivalent to ```self.mut_view_take_with_size(predicate..)```.
    fn mut_view_take_right_until_with_size(self, predicate: impl Fn(usize,&D)->bool) -> FoldListSliceFrom<'a, T, Self::OriginalD, Self::Settings, Self::Simplification, MutFoldChainSliceStruct<'a, <Self::UnderlyingChain as FoldChainSlice<'a, T, (usize, Self::OriginalD)>>::IsReversed, <<Self::UnderlyingChain as FoldChainSlice<'a, T, (usize, Self::OriginalD)>>::IsFlushLeft as Bool>::And<<Self::UnderlyingChain as FoldChainSlice<'a, T, (usize, Self::OriginalD)>>::IsReversed>, <<Self::UnderlyingChain as FoldChainSlice<'a, T, (usize, Self::OriginalD)>>::IsFlushRight as Bool>::And<<<Self::UnderlyingChain as FoldChainSlice<'a, T, (usize, Self::OriginalD)>>::IsReversed as Bool>::Not>, T, (usize, Self::OriginalD), SettingsWithSize<Self::Settings>, ()>> {
        FoldListSliceFrom {
            simplification: self.get_current_simplification(),
            underlying: self.mut_as_sized_chain_keeping_simplification()
                .mut_view_take_right_until(|(n,d)| predicate(*n,d))
                .mut_view_unsimplify(),
            _m: PhantomData
        }
    }

    /// Contract this view on the right while the to-be-discarded range's length and fold don't meet `predicate`.
    /// 
    /// The given `predicate` must be well-behaved; see [Slicing](crate#slicing) for examples.
    /// 
    /// This operation's immutable version is [`view_drop_right_until_with_size`](FoldListSlice::view_drop_right_until_with_size) and 
    /// its mirror image is [`mut_view_drop_left_until_with_size`](MutFoldListSlice::mut_view_drop_left_until_with_size).
    /// Its unsized version is [`mut_view_drop_right_until`](MutFoldListSlice::mut_view_drop_right_until).
    /// 
    /// This is equivalent to ```self.mut_view_drop_with_size(predicate..)```.
    fn mut_view_drop_right_until_with_size(self, predicate: impl Fn(usize,&D)->bool) -> FoldListSliceFrom<'a, T, Self::OriginalD, Self::Settings, Self::Simplification, MutFoldChainSliceStruct<'a, <<<Self::UnderlyingChain as FoldChainSlice<'a, T, (usize, Self::OriginalD)>>::IsReversed as Bool>::Not as Bool>::Not, <<Self::UnderlyingChain as FoldChainSlice<'a, T, (usize, Self::OriginalD)>>::IsFlushLeft as Bool>::And<<<Self::UnderlyingChain as FoldChainSlice<'a, T, (usize, Self::OriginalD)>>::IsReversed as Bool>::Not>, <<Self::UnderlyingChain as FoldChainSlice<'a, T, (usize, Self::OriginalD)>>::IsFlushRight as Bool>::And<<<<Self::UnderlyingChain as FoldChainSlice<'a, T, (usize, Self::OriginalD)>>::IsReversed as Bool>::Not as Bool>::Not>, T, (usize, Self::OriginalD), SettingsWithSize<Self::Settings>, ()>> {
        FoldListSliceFrom {
            simplification: self.get_current_simplification(),
            underlying: self.mut_as_sized_chain_keeping_simplification()
                .mut_view_drop_right_until(|(n,d)| predicate(*n,d))
                .mut_view_unsimplify(),
            _m: PhantomData
        }
    }

    /// Restrict this view to the longest range that starts on the left and whose length and fold don't meet `predicate`.
    /// 
    /// The given `predicate` must be well-behaved; see [Slicing](crate#slicing) for examples.
    /// 
    /// This operation's immutable version is [`view_take_left_until_with_size`](FoldListSlice::view_take_left_until_with_size) and
    /// its mirror image is [`mut_view_take_right_until_with_size`](MutFoldListSlice::mut_view_take_right_until_with_size).
    /// Its unsized version is [`mut_view_take_left_until`](MutFoldListSlice::mut_view_take_left_until).
    /// 
    /// This is equivalent to ```self.mut_view_take_with_size(..predicate)```.
    fn mut_view_take_left_until_with_size(self, predicate: impl Fn(usize,&D)->bool) -> FoldListSliceFrom<'a, T, Self::OriginalD, Self::Settings, Self::Simplification, MutFoldChainSliceStruct<'a, <<<Self::UnderlyingChain as FoldChainSlice<'a, T, (usize, Self::OriginalD)>>::IsReversed as Bool>::Not as Bool>::Not, <<Self::UnderlyingChain as FoldChainSlice<'a, T, (usize, Self::OriginalD)>>::IsFlushLeft as Bool>::And<<<Self::UnderlyingChain as FoldChainSlice<'a, T, (usize, Self::OriginalD)>>::IsReversed as Bool>::Not>, <<Self::UnderlyingChain as FoldChainSlice<'a, T, (usize, Self::OriginalD)>>::IsFlushRight as Bool>::And<<<<Self::UnderlyingChain as FoldChainSlice<'a, T, (usize, Self::OriginalD)>>::IsReversed as Bool>::Not as Bool>::Not>, T, (usize, Self::OriginalD), SettingsWithSize<Self::Settings>, ()>> {
        FoldListSliceFrom {
            simplification: self.get_current_simplification(),
            underlying: self.mut_as_sized_chain_keeping_simplification()
                .mut_view_take_left_until(|(n,d)| predicate(*n,d))
                .mut_view_unsimplify(),
            _m: PhantomData
        }
    }

    /// If `range` is ```..predicate```, calls [`mut_view_drop_left_until_with_size(predicate)`](MutFoldListSlice::mut_view_drop_left_until_with_size).
    /// 
    /// If `range` is ```predicate..```, calls [`mut_view_drop_right_until_with_size(predicate)`](MutFoldListSlice::mut_view_drop_right_until_with_size) (You may need to parenthesize `predicate` in this case).
    /// 
    /// The given `predicate` must be well-behaved; see [Slicing](crate#slicing) for examples.
    /// 
    /// This operation's immutable version is [`view_drop_with_size`](FoldListSlice::view_drop_with_size) and 
    /// its unsized version is [`mut_view_drop`](MutFoldListSlice::mut_view_drop).
    fn mut_view_drop_with_size<Predicate: Fn(usize,&D)->bool, Range: SingleEndedRange<Predicate>>(self, range: Range) -> <<Range as SingleEndedRange<Predicate>>::EndIsLeft as Bool>::IfElse<FoldListSliceFrom<'a, T, <Self as FoldListSlice<'a, T, D>>::OriginalD, <Self as FoldListSlice<'a, T, D>>::Settings, <Self as FoldListSlice<'a, T, D>>::Simplification, MutFoldChainSliceStruct<'a, <<<<Self as FoldListSlice<'a, T, D>>::UnderlyingChain as FoldChainSlice<'a, T, (usize, <Self as FoldListSlice<'a, T, D>>::OriginalD)>>::IsReversed as Bool>::Not as Bool>::Not, <<<Self as FoldListSlice<'a, T, D>>::UnderlyingChain as FoldChainSlice<'a, T, (usize, <Self as FoldListSlice<'a, T, D>>::OriginalD)>>::IsFlushLeft as Bool>::And<<<<Self as FoldListSlice<'a, T, D>>::UnderlyingChain as FoldChainSlice<'a, T, (usize, <Self as FoldListSlice<'a, T, D>>::OriginalD)>>::IsReversed as Bool>::Not>, <<<Self as FoldListSlice<'a, T, D>>::UnderlyingChain as FoldChainSlice<'a, T, (usize, <Self as FoldListSlice<'a, T, D>>::OriginalD)>>::IsFlushRight as Bool>::And<<<<<Self as FoldListSlice<'a, T, D>>::UnderlyingChain as FoldChainSlice<'a, T, (usize, <Self as FoldListSlice<'a, T, D>>::OriginalD)>>::IsReversed as Bool>::Not as Bool>::Not>, T, (usize, <Self as FoldListSlice<'a, T, D>>::OriginalD), SettingsWithSize<<Self as FoldListSlice<'a, T, D>>::Settings>, ()>>, FoldListSliceFrom<'a, T, <Self as FoldListSlice<'a, T, D>>::OriginalD, <Self as FoldListSlice<'a, T, D>>::Settings, <Self as FoldListSlice<'a, T, D>>::Simplification, MutFoldChainSliceStruct<'a, <<Self as FoldListSlice<'a, T, D>>::UnderlyingChain as FoldChainSlice<'a, T, (usize, <Self as FoldListSlice<'a, T, D>>::OriginalD)>>::IsReversed, <<<Self as FoldListSlice<'a, T, D>>::UnderlyingChain as FoldChainSlice<'a, T, (usize, <Self as FoldListSlice<'a, T, D>>::OriginalD)>>::IsFlushLeft as Bool>::And<<<Self as FoldListSlice<'a, T, D>>::UnderlyingChain as FoldChainSlice<'a, T, (usize, <Self as FoldListSlice<'a, T, D>>::OriginalD)>>::IsReversed>, <<<Self as FoldListSlice<'a, T, D>>::UnderlyingChain as FoldChainSlice<'a, T, (usize, <Self as FoldListSlice<'a, T, D>>::OriginalD)>>::IsFlushRight as Bool>::And<<<<Self as FoldListSlice<'a, T, D>>::UnderlyingChain as FoldChainSlice<'a, T, (usize, <Self as FoldListSlice<'a, T, D>>::OriginalD)>>::IsReversed as Bool>::Not>, T, (usize, <Self as FoldListSlice<'a, T, D>>::OriginalD), SettingsWithSize<<Self as FoldListSlice<'a, T, D>>::Settings>, ()>>> {
        Range::EndIsLeft::init_if_else((self,SingleEndedRange::end(range)), 
            |(this,end)| this.mut_view_drop_right_until_with_size(end), 
            |(this,end)| this.mut_view_drop_left_until_with_size(end), 
        )
    }

    /// If `range` is ```..predicate```, calls [`mut_view_take_left_until_with_size(predicate)`](MutFoldListSlice::mut_view_take_left_until_with_size).
    /// 
    /// If `range` is ```predicate..```, calls [`mut_view_take_right_until_with_size(predicate)`](MutFoldListSlice::mut_view_take_right_until_with_size) (You may need to parenthesize `predicate` in this case).
    /// 
    /// The given `predicate` must be well-behaved; see [Slicing](crate#slicing) for examples.
    /// 
    /// This operation's immutable version is [`view_take_with_size`](FoldListSlice::view_take_with_size) and
    /// its unsized version is [`mut_view_take`](MutFoldListSlice::mut_view_take).
    fn mut_view_take_with_size<Predicate: Fn(usize,&D)->bool, Range: SingleEndedRange<Predicate>>(self, range: Range) -> <<Range as SingleEndedRange<Predicate>>::EndIsLeft as Bool>::IfElse<FoldListSliceFrom<'a, T, <Self as FoldListSlice<'a, T, D>>::OriginalD, <Self as FoldListSlice<'a, T, D>>::Settings, <Self as FoldListSlice<'a, T, D>>::Simplification, MutFoldChainSliceStruct<'a, <<Self as FoldListSlice<'a, T, D>>::UnderlyingChain as FoldChainSlice<'a, T, (usize, <Self as FoldListSlice<'a, T, D>>::OriginalD)>>::IsReversed, <<<Self as FoldListSlice<'a, T, D>>::UnderlyingChain as FoldChainSlice<'a, T, (usize, <Self as FoldListSlice<'a, T, D>>::OriginalD)>>::IsFlushLeft as Bool>::And<<<Self as FoldListSlice<'a, T, D>>::UnderlyingChain as FoldChainSlice<'a, T, (usize, <Self as FoldListSlice<'a, T, D>>::OriginalD)>>::IsReversed>, <<<Self as FoldListSlice<'a, T, D>>::UnderlyingChain as FoldChainSlice<'a, T, (usize, <Self as FoldListSlice<'a, T, D>>::OriginalD)>>::IsFlushRight as Bool>::And<<<<Self as FoldListSlice<'a, T, D>>::UnderlyingChain as FoldChainSlice<'a, T, (usize, <Self as FoldListSlice<'a, T, D>>::OriginalD)>>::IsReversed as Bool>::Not>, T, (usize, <Self as FoldListSlice<'a, T, D>>::OriginalD), SettingsWithSize<<Self as FoldListSlice<'a, T, D>>::Settings>, ()>>, FoldListSliceFrom<'a, T, <Self as FoldListSlice<'a, T, D>>::OriginalD, <Self as FoldListSlice<'a, T, D>>::Settings, <Self as FoldListSlice<'a, T, D>>::Simplification, MutFoldChainSliceStruct<'a, <<<<Self as FoldListSlice<'a, T, D>>::UnderlyingChain as FoldChainSlice<'a, T, (usize, <Self as FoldListSlice<'a, T, D>>::OriginalD)>>::IsReversed as Bool>::Not as Bool>::Not, <<<Self as FoldListSlice<'a, T, D>>::UnderlyingChain as FoldChainSlice<'a, T, (usize, <Self as FoldListSlice<'a, T, D>>::OriginalD)>>::IsFlushLeft as Bool>::And<<<<Self as FoldListSlice<'a, T, D>>::UnderlyingChain as FoldChainSlice<'a, T, (usize, <Self as FoldListSlice<'a, T, D>>::OriginalD)>>::IsReversed as Bool>::Not>, <<<Self as FoldListSlice<'a, T, D>>::UnderlyingChain as FoldChainSlice<'a, T, (usize, <Self as FoldListSlice<'a, T, D>>::OriginalD)>>::IsFlushRight as Bool>::And<<<<<Self as FoldListSlice<'a, T, D>>::UnderlyingChain as FoldChainSlice<'a, T, (usize, <Self as FoldListSlice<'a, T, D>>::OriginalD)>>::IsReversed as Bool>::Not as Bool>::Not>, T, (usize, <Self as FoldListSlice<'a, T, D>>::OriginalD), SettingsWithSize<<Self as FoldListSlice<'a, T, D>>::Settings>, ()>>> {
        Range::EndIsLeft::init_if_else((self,SingleEndedRange::end(range)), 
            |(this,end)| this.mut_view_take_right_until_with_size(end), 
            |(this,end)| this.mut_view_take_left_until_with_size(end), 
        )
    }

    /// Contract this view by `n` elements, on the left.
    /// 
    /// If `n` is `0`, this will do nothing, except possibly change this slice's type.
    /// 
    /// This operation's immutable version is [`view_drop_left`](FoldListSlice::view_drop_left).
    fn mut_view_drop_left(self, n: usize) -> FoldListSliceFrom<'a, T, Self::OriginalD, Self::Settings, Self::Simplification, MutFoldChainSliceStruct<'a, <Self::UnderlyingChain as FoldChainSlice<'a, T, (usize, Self::OriginalD)>>::IsReversed, <<Self::UnderlyingChain as FoldChainSlice<'a, T, (usize, Self::OriginalD)>>::IsFlushLeft as Bool>::And<<Self::UnderlyingChain as FoldChainSlice<'a, T, (usize, Self::OriginalD)>>::IsReversed>, <<Self::UnderlyingChain as FoldChainSlice<'a, T, (usize, Self::OriginalD)>>::IsFlushRight as Bool>::And<<<Self::UnderlyingChain as FoldChainSlice<'a, T, (usize, Self::OriginalD)>>::IsReversed as Bool>::Not>, T,(usize, Self::OriginalD), SettingsWithSize<Self::Settings>, ()>> {
        FoldListSliceFrom {
            simplification: self.get_current_simplification(),
            underlying: self.as_sized_chain().mut_view_simplify_with_shortcut(
                    |(n,_): &(_,_)| *n, 
                    |(a,b)|a+b, 
                    |()| 0, 
                    |_ : &_| 1
                ).mut_view_drop_left_until(|size: &usize| *size > n)
                .mut_view_unsimplify(),
            _m: PhantomData
        }
    }

    /// Restrict this view to the `n` rightmost elements.
    /// 
    /// If `n` is greater than or equal to the length of this slice, this will do nothing, except possibly change this slice's type.
    /// 
    /// This operation's immutable version is [`view_take_right`](FoldListSlice::view_take_right).
    fn mut_view_take_right(self, n: usize) -> FoldListSliceFrom<'a, T, Self::OriginalD, Self::Settings, Self::Simplification, MutFoldChainSliceStruct<'a, <Self::UnderlyingChain as FoldChainSlice<'a, T, (usize, Self::OriginalD)>>::IsReversed, <<Self::UnderlyingChain as FoldChainSlice<'a, T, (usize, Self::OriginalD)>>::IsFlushLeft as Bool>::And<<Self::UnderlyingChain as FoldChainSlice<'a, T, (usize, Self::OriginalD)>>::IsReversed>, <<Self::UnderlyingChain as FoldChainSlice<'a, T, (usize, Self::OriginalD)>>::IsFlushRight as Bool>::And<<<Self::UnderlyingChain as FoldChainSlice<'a, T, (usize, Self::OriginalD)>>::IsReversed as Bool>::Not>, T,(usize, Self::OriginalD), SettingsWithSize<Self::Settings>, ()>> {
        FoldListSliceFrom {
            simplification: self.get_current_simplification(),
            underlying: self.as_sized_chain().mut_view_simplify_with_shortcut(
                    |(n,_): &(_,_)| *n, 
                    |(a,b)|a+b, 
                    |()| 0, 
                    |_ : &_| 1
                ).mut_view_take_right_until(|size: &usize| *size > n)
                .mut_view_unsimplify(),
            _m: PhantomData
        }
    }

    /// Contract this view by `n` elements, on the right.
    /// 
    /// If `n` is `0`, this will do nothing, except possibly change this slice's type.
    /// 
    /// This operation's immutable version is [`view_drop_right`](FoldListSlice::view_drop_right).
    fn mut_view_drop_right(self, n: usize) -> FoldListSliceFrom<'a, T, Self::OriginalD, Self::Settings, Self::Simplification, MutFoldChainSliceStruct<'a, <<<Self::UnderlyingChain as FoldChainSlice<'a, T, (usize, Self::OriginalD)>>::IsReversed as Bool>::Not as Bool>::Not, <<Self::UnderlyingChain as FoldChainSlice<'a, T, (usize, Self::OriginalD)>>::IsFlushLeft as Bool>::And<<<Self::UnderlyingChain as FoldChainSlice<'a, T, (usize, Self::OriginalD)>>::IsReversed as Bool>::Not>, <<Self::UnderlyingChain as FoldChainSlice<'a, T, (usize, Self::OriginalD)>>::IsFlushRight as Bool>::And<<<<Self::UnderlyingChain as FoldChainSlice<'a, T, (usize, Self::OriginalD)>>::IsReversed as Bool>::Not as Bool>::Not>, T, (usize, Self::OriginalD), SettingsWithSize<Self::Settings>, ()>> {
        FoldListSliceFrom {
            simplification: self.get_current_simplification(),
            underlying: self.as_sized_chain().mut_view_simplify_with_shortcut(
                    |(n,_): &(_,_)| *n, 
                    |(a,b)|a+b, 
                    |()| 0, 
                    |_ : &_| 1
                ).mut_view_drop_right_until(|size: &usize| *size > n)
                .mut_view_unsimplify(),
            _m: PhantomData
        }
    }

    /// Restrict this view to the `n` leftmost elements.
    /// 
    /// If `n` is greater than or equal to the length of this slice, this will do nothing, except possibly change this slice's type.
    /// 
    /// This operation's immutable version is [`view_take_left`](FoldListSlice::view_take_left).
    fn mut_view_take_left(self, n: usize) -> FoldListSliceFrom<'a, T, Self::OriginalD, Self::Settings, Self::Simplification, MutFoldChainSliceStruct<'a, <<<Self::UnderlyingChain as FoldChainSlice<'a, T, (usize, Self::OriginalD)>>::IsReversed as Bool>::Not as Bool>::Not, <<Self::UnderlyingChain as FoldChainSlice<'a, T, (usize, Self::OriginalD)>>::IsFlushLeft as Bool>::And<<<Self::UnderlyingChain as FoldChainSlice<'a, T, (usize, Self::OriginalD)>>::IsReversed as Bool>::Not>, <<Self::UnderlyingChain as FoldChainSlice<'a, T, (usize, Self::OriginalD)>>::IsFlushRight as Bool>::And<<<<Self::UnderlyingChain as FoldChainSlice<'a, T, (usize, Self::OriginalD)>>::IsReversed as Bool>::Not as Bool>::Not>, T, (usize, Self::OriginalD), SettingsWithSize<Self::Settings>, ()>> {
        FoldListSliceFrom {
            simplification: self.get_current_simplification(),
            underlying: self.as_sized_chain().mut_view_simplify_with_shortcut(
                    |(n,_): &(_,_)| *n, 
                    |(a,b)|a+b, 
                    |()| 0, 
                    |_ : &_| 1
                ).mut_view_take_left_until(|size: &usize| *size > n)
                .mut_view_unsimplify(),
            _m: PhantomData
        }
    }

    /// Get a reversed version of this view. See [Reverse](crate#reverse).
    /// 
    /// This operation's immutable version is [`view_reversed`](FoldListSlice::view_reversed).
    fn mut_view_reversed(self) -> FoldListSliceFrom<'a, T, Self::OriginalD, Self::Settings, Self::Simplification, MutFoldChainSliceStruct<'a, <<Self::UnderlyingChain as FoldChainSlice<'a, T, (usize, Self::OriginalD)>>::IsReversed as Bool>::Not, <Self::UnderlyingChain as FoldChainSlice<'a, T, (usize, Self::OriginalD)>>::IsFlushLeft, <Self::UnderlyingChain as FoldChainSlice<'a, T, (usize, Self::OriginalD)>>::IsFlushRight, T, (usize, Self::OriginalD), SettingsWithSize<Self::Settings>, ()>> {
        FoldListSliceFrom {
            simplification: self.get_current_simplification(),
            underlying: self.as_sized_chain().mut_view_reversed(),
            _m: PhantomData,
        }
    }

    /// Compose this view's current simplification with another one explicitly. See [Simplification](crate#simplification).
    /// 
    /// This operation's immutable version is [`view_with_simplification`](FoldListSlice::view_with_simplification).
    fn mut_view_with_simplification<NewSimplification: FoldSimplification<T,D>>(self, new_simplification: NewSimplification) -> FoldListSliceFrom<'a, T, Self::OriginalD, Self::Settings, <NewSimplification as FoldSimplification<T, D>>::ComposeAfterOther<Self::OriginalD, Self::Simplification>, MutFoldChainSliceStruct<'a, <Self::UnderlyingChain as FoldChainSlice<'a, T, (usize, Self::OriginalD)>>::IsReversed, <Self::UnderlyingChain as FoldChainSlice<'a, T, (usize, Self::OriginalD)>>::IsFlushLeft, <Self::UnderlyingChain as FoldChainSlice<'a, T, (usize, Self::OriginalD)>>::IsFlushRight, T, (usize, Self::OriginalD), SettingsWithSize<Self::Settings>, ()>> {
        FoldListSliceFrom {
            simplification: new_simplification.compose_after_other(self.get_current_simplification()),
            underlying: self.as_sized_chain().as_mut(),
            _m: PhantomData
        }
    }

    /// Simplify this view. See [Simplification](crate#simplification).
    /// 
    /// This operation's immutable version is [`view_simplify`](FoldListSlice::view_simplify).
    fn mut_view_simplify<D2: Clone + 'a, 
        Simplifier: for<'x> Fun<&'x D,D2> + Copy + 'a,
        OP2: Fun<(D2,D2),D2> + Copy + 'a>(self,simplifier: Simplifier, simplified_op: OP2) -> FoldListSliceFrom<'a, T, Self::OriginalD, Self::Settings,
            <SimplificationWithoutShortcut<T,D,D2,Simplifier,OP2> as FoldSimplification<T,D>>::ComposeAfterOther<Self::OriginalD,Self::Simplification>, 
            MutFoldChainSliceStruct<'a, <Self::UnderlyingChain as FoldChainSlice<'a, T, (usize, Self::OriginalD)>>::IsReversed, <Self::UnderlyingChain as FoldChainSlice<'a, T, (usize, Self::OriginalD)>>::IsFlushLeft, <Self::UnderlyingChain as FoldChainSlice<'a, T, (usize, Self::OriginalD)>>::IsFlushRight, T, (usize, Self::OriginalD), SettingsWithSize<Self::Settings>, ()>> {
                self.mut_view_with_simplification(SimplificationWithoutShortcut {
                    simplifier,
                    op2: simplified_op,
                    _m: PhantomData,
                })
    }

    /// Simplify this view in a possibly more efficient way. See [Simplification](crate#simplification).
    /// 
    /// This operation's immutable version is [`view_simplify_with_shortcut`](FoldListSlice::view_simplify_with_shortcut).
    fn mut_view_simplify_with_shortcut<D2: Clone + 'a, 
        Simplifier: for<'x> Fun<&'x D,D2> + Copy + 'a,
        OP2: Fun<(D2,D2),D2> + Copy + 'a,
        EmptyShortcut: Fun<(),D2> + Copy + 'a,
        DeltaShortcut: for<'x> Fun<&'x T, D2> + Copy + 'a>(self,simplifier: Simplifier, simplified_op: OP2, empty_shortcut: EmptyShortcut, delta_shortcut: DeltaShortcut) -> FoldListSliceFrom<'a, T, Self::OriginalD, Self::Settings, 
            <SimplificationWithShortcut<T,D,D2,Simplifier,OP2,EmptyShortcut,DeltaShortcut> as FoldSimplification<T,D>>::ComposeAfterOther<Self::OriginalD,Self::Simplification>, 
            MutFoldChainSliceStruct<'a, <Self::UnderlyingChain as FoldChainSlice<'a, T, (usize, Self::OriginalD)>>::IsReversed, <Self::UnderlyingChain as FoldChainSlice<'a, T, (usize, Self::OriginalD)>>::IsFlushLeft, <Self::UnderlyingChain as FoldChainSlice<'a, T, (usize, Self::OriginalD)>>::IsFlushRight, T, (usize, Self::OriginalD), SettingsWithSize<Self::Settings>, ()>> {
                self.mut_view_with_simplification(SimplificationWithShortcut {
                    simplifier,
                    op2: simplified_op,
                    empty_shortcut,
                    delta_shortcut,
                    _m: PhantomData,
                })
    }

    /// Remove all simplifications that were applied to this view. See [Simplification](crate#simplification).
    /// 
    /// The [current simplification](FoldListSlice::get_current_simplification) of the resulting view will be `()`.
    /// 
    /// This operation's immutable version is [`view_unsimplify`](FoldListSlice::view_unsimplify).
    fn mut_view_unsimplify(self) -> FoldListSliceFrom<'a, T, Self::OriginalD, Self::Settings, (), MutFoldChainSliceStruct<'a, <Self::UnderlyingChain as FoldChainSlice<'a, T, (usize, Self::OriginalD)>>::IsReversed, <Self::UnderlyingChain as FoldChainSlice<'a, T, (usize, Self::OriginalD)>>::IsFlushLeft, <Self::UnderlyingChain as FoldChainSlice<'a, T, (usize, Self::OriginalD)>>::IsFlushRight, T, (usize, Self::OriginalD), SettingsWithSize<Self::Settings>, ()>> {
        FoldListSliceFrom {
            simplification: (),
            underlying: self.as_sized_chain().as_mut(),
            _m: PhantomData
        }
    }

    /// Remove the leftmost element from this slice.
    /// 
    /// Returns the removed element, or [`None`], if this slice was already empty.
    fn pop_left(&mut self) -> Option<T> {
        self.borrow_mut().as_sized_chain().pop_left()
    }

    /// Remove the rightmost element from this slice.
    /// 
    /// Returns the removed element, or [`None`], if this slice was already empty.
    fn pop_right(&mut self) -> Option<T> {
        self.borrow_mut().as_sized_chain().pop_right()
    }

    /// Append `value` to the left of this slice.
    fn append_left(&mut self, value: T) {
        self.borrow_mut().as_sized_chain().append_left(value);
    }

    /// Append `value` to the right of this slice.
    fn append_right(&mut self, value: T) {
        self.borrow_mut().as_sized_chain().append_right(value);
    }

    /// Replace the leftmost element of this slice with `value`.
    /// 
    /// If this slice is empty, this does nothing and returns `Err(value)`.
    /// 
    /// Otherwise, returns `Ok(prev)`, where `prev` is the previous leftmost element.
    /// 
    /// For the unwrapping version, see [`set_left`](MutFoldListSlice::set_left).
    fn set_left_or_err(&mut self, value: T) -> Result<T,T> {
        self.borrow_mut().as_sized_chain().set_left_or_err(value)
    }

    /// Replace the rightmost element of this slice with `value`.
    /// 
    /// If this slice is empty, this does nothing and returns `Err(value)`.
    /// 
    /// Otherwise, returns `Ok(prev)`, where `prev` is the previous rightmost element.
    /// 
    /// For the unwrapping version, see [`set_right`](MutFoldListSlice::set_right).
    fn set_right_or_err(&mut self, value: T) -> Result<T,T> {
        self.borrow_mut().as_sized_chain().set_right_or_err(value)
    }
    
    /// Replace the leftmost element of this slice with `value` and return its previous value.
    /// 
    /// Panics if this slice is empty.
    /// 
    /// For the non-panicing version, see [`set_left_or_err`](MutFoldListSlice::set_left_or_err).
    fn set_left(&mut self, value: T) -> T {
        self.set_left_or_err(value).unwrap_or_else(|_| panic!("The chain should not be empty"))
    }
    
    /// Replace the rightmost element of this slice with `value` and return its previous value.
    /// 
    /// Panics if this slice is empty.
    /// 
    /// For the non-panicing version, see [`set_right_or_err`](MutFoldListSlice::set_right_or_err).
    fn set_right(&mut self, value: T) -> T {
        self.set_right_or_err(value).unwrap_or_else(|_| panic!("The chain should not be empty"))
    }

    /// Mutate the leftmost element of this slice via a closure, and return the result of the closure.
    /// 
    /// If this slice is empty, the input of the closure will be [`None`].
    fn update_left<R>(&mut self, f: impl FnOnce(Option<&mut T>) -> R) -> R {
        self.borrow_mut().as_sized_chain().update_left(f)
    }

    /// Mutate the rightmost element of this slice via a closure, and return the result of the closure.
    /// 
    /// If this slice is empty, the input of the closure will be [`None`].
    fn update_right<R>(&mut self, f: impl FnOnce(Option<&mut T>) -> R) -> R {
        self.borrow_mut().as_sized_chain().update_right(f)
    }

    /// Mutate the element at index `index` via a closure, and return the result of the closure.
    /// 
    /// Panics if `index` is out of bounds.
    fn update_at<R>(&mut self, index: usize, f: impl FnOnce(&mut T)->R)->R {
        let ret = self.borrow_mut().mut_view_drop_left(index).update_left(|t| t.map(f));
        ret.unwrap_or_else(|| panic!("Index out of bounds: the index is {} but the length is {}",index,self.len()))
    }

    /// Replace the element at index `index` with `value` and return the previous value.
    /// 
    /// Panics if `index` is out of bounds.
    fn set_at(&mut self, index: usize, value: T) -> T {
        self.update_at(index, |t| core::mem::replace(t,value))
    }

    /// Insert `value` at index `index`.
    /// 
    /// The elements whose indices were `index` and above will have their indices shifted up by one.
    /// 
    /// Panics if `index` is outside ```0..=self.len()```.
    fn insert_at(&mut self, index: usize, value: T) {
        let mut slice = self.borrow_mut().mut_view_take_left(index);
        if slice.len() != index {
            drop(slice);
            panic!("Index out of bounds: the index is {} but the length is {}",index,self.len());
        }
        slice.append_right(value);
    }

    /// Remove and return the element at index `index`.
    /// 
    /// The elements that were to the right of it will have their indices shifted down by one.
    /// 
    /// Panics if `index` is out of bounds.
    fn remove_at(&mut self, index: usize) -> T {
        let mut slice = self.borrow_mut().mut_view_take_left(index + 1);
        if slice.len() != index + 1 {
            drop(slice);
            panic!("Index out of bounds: the index is {} but the length is {}",index,self.len())
        }
        let Some(ret) = slice.pop_right() else {unreachable!()};
        ret
    }

    /// Run a closure on each of this slice's elements, possibly mutating them, from left to right.
    /// 
    /// This operation's immutable version is [`foreach`](FoldListSlice::foreach).
    /// 
    /// Note that this operation cannot be safely done with an [`Iterator`], because that would allow direct access to mutable references to this data structure's elements,
    /// in a way that might outlive the iterator (so the iterator couldn't reliably do the proper cleanup/bookkeeping on [`Drop`]). 
    /// An "iterator" capable of this would need to be a [streaming iterator](https://docs.rs/streaming-iterator/latest/streaming_iterator/) instead of an [`Iterator`].
    /// 
    /// An iterator can iterate over the elements themselves (rather than references), though; see [`drain`](MutFoldListSlice::drain) or [`FoldList`]'s [IntoIterator implementation](FoldList#impl-IntoIterator-for-FoldList<T,+D,+Settings>).
    fn foreach_mut(&mut self, f: impl FnMut(&mut T)) {
        self.borrow_mut().as_sized_chain().foreach_mut(f);
    }

    /// Remove all elements from this slice, and return a new [`FoldList`] containing them, in the same order as in this slice's base [`FoldList`].
    /// 
    /// The returned `FoldChain` will have the same type as this slice's base.
    fn take_all(&mut self) -> FoldList<T,Self::OriginalD,Self::Settings> {
        FoldList {
            underlying: self.borrow_mut().as_sized_chain().take_all(),
        }
    }

    /// Convert this slice into an [`Iterator`] that removes elements from the left as it emits them (or from the right with [`DoubleEndedIterator::next_back`]).
    /// 
    /// Note that not all elements in this slice will be removed; only those that the iterator emits.
    fn drain(self) -> Drain<'a, T, (usize, <Self as FoldListSlice<'a, T, D>>::OriginalD), <Self as FoldListSlice<'a, T, D>>::UnderlyingChain>  {
        self.as_sized_chain().drain()
    }

    /// Append the entire contents of a [`FoldList`] to the right of this slice.
    /// 
    /// From this slice's base's perspective, the elements' order will be the same as in `list`.
    /// 
    /// Note that `list`'s type must be the same as this slice's base.
    fn append_all_right(&mut self, list: FoldList<T,Self::OriginalD,Self::Settings>) {
        self.borrow_mut().as_sized_chain().append_all_right(list.underlying);
    }

    /// Append the entire contents of a [`FoldList`] to the left of this slice.
    /// 
    /// From this slice's base's perspective, the elements' order will be the same as in `list`.
    /// 
    /// Note that `list`'s type must be the same as this slice's base.
    fn append_all_left(&mut self, list: FoldList<T,Self::OriginalD,Self::Settings>) {
        self.borrow_mut().as_sized_chain().append_all_left(list.underlying);
    }

    /// Append every element from an iterator to the left of this slice.
    /// 
    /// This is faster than [`append_left`](MutFoldListSlice::append_left)ing them one-by-one, but not asymptotically faster.
    fn append_left_from_iter(&mut self, iter: impl Iterator<Item=T>) {
        self.borrow_mut().as_sized_chain().append_left_from_iter(iter);
    }

    /// Append every element from an iterator to the right of this slice.
    /// 
    /// This is faster than [`append_right`](MutFoldListSlice::append_right)ing them one-by-one, but not asymptotically faster.
    fn append_right_from_iter(&mut self, iter: impl Iterator<Item=T>) {
        self.borrow_mut().as_sized_chain().append_right_from_iter(iter);
    }
}

fn foldlist_index_impl<'a,T: 'a,D: Clone + 'a>(foldlist: impl FoldListSlice<'a,T,D>, index: usize) -> &'a T {
    foldlist.view_drop_left(index).underlying.left_consume().unwrap_or_else(|| panic!("Index out of bounds: index is {}",index))
}

impl<'a, T: 'a, D: Clone + 'a, Settings: FoldSettings<T,D> + 'a> FoldListSlice<'a,T,D> for &'a FoldList<T, D, Settings> {
    type OriginalD = D;
    type Simplification = ();
    type Settings = Settings;
    fn get_settings(&self) -> Self::Settings {
        self.underlying.get_settings().0
    }
    type UnderlyingChain = &'a FoldChain<T,(usize,D),SettingsWithSize<Settings>>;
    fn get_current_simplification(&self) -> Self::Simplification {}
    fn as_sized_chain(self) -> Self::UnderlyingChain {
        let FoldList { underlying } = self;
        underlying
    }
    fn borrow<'b>(&'b self) -> FoldListSliceFrom<'b,T,D,
        Self::Settings,
        Self::Simplification,
        ImmFoldChainSliceStruct<'b,
            <Self::UnderlyingChain as FoldChainSlice<'a,T,(usize,D)>>::IsReversed,
            <Self::UnderlyingChain as FoldChainSlice<'a,T,(usize,D)>>::IsFlushLeft,
            <Self::UnderlyingChain as FoldChainSlice<'a,T,(usize,D)>>::IsFlushRight,
            SettingsWithSize<Self::Settings>,
            (),
            T,
            (usize,D)>> {
        (*self).as_imm()
    }
}





impl<'a, T: 'a, D: Clone + 'a, Settings: FoldSettings<T,D> + 'a> FoldListSlice<'a,T,D> for &'a mut FoldList<T, D, Settings> {
    type OriginalD = D;
    type Simplification = ();
    type Settings = Settings;
    fn get_settings(&self) -> Self::Settings {
        self.underlying.get_settings().0
    }
    type UnderlyingChain = &'a mut FoldChain<T,(usize,D),SettingsWithSize<Settings>>;
    fn get_current_simplification(&self) -> Self::Simplification {}
    fn as_sized_chain(self) -> Self::UnderlyingChain {
        let FoldList { underlying } = self;
        underlying
    }
    fn borrow<'b>(&'b self) -> FoldListSliceFrom<'b,T,D,
        Self::Settings,
        Self::Simplification,
        ImmFoldChainSliceStruct<'b,
            <Self::UnderlyingChain as FoldChainSlice<'a,T,(usize,D)>>::IsReversed,
            <Self::UnderlyingChain as FoldChainSlice<'a,T,(usize,D)>>::IsFlushLeft,
            <Self::UnderlyingChain as FoldChainSlice<'a,T,(usize,D)>>::IsFlushRight,
            SettingsWithSize<Self::Settings>,
            (),
            T,
            (usize,D)>> {
        (&**self).as_imm()
    }
}

impl<'a, T: 'a, D: Clone + 'a, Settings: FoldSettings<T,D> + 'a> MutFoldListSlice<'a,T,D> for &'a mut FoldList<T, D, Settings> {
    fn as_mut(self) -> FoldListSliceFrom<'a,T,Self::OriginalD,
        Self::Settings,
        Self::Simplification,
        MutFoldChainSliceStruct<'a,
            <Self::UnderlyingChain as FoldChainSlice<'a,T,(usize,Self::OriginalD)>>::IsReversed,
            <Self::UnderlyingChain as FoldChainSlice<'a,T,(usize,Self::OriginalD)>>::IsFlushLeft,
            <Self::UnderlyingChain as FoldChainSlice<'a,T,(usize,Self::OriginalD)>>::IsFlushRight,
            T,
            (usize,Self::OriginalD),
            SettingsWithSize<Self::Settings>,
            ()>> {
                FoldListSliceFrom {
                    underlying: self.as_sized_chain().as_mut(),
                    simplification: (),
                    _m: PhantomData,
                }
    }

    fn borrow_mut<'b>(&'b mut self) -> FoldListSliceFrom<'b,T,Self::OriginalD,
        Self::Settings,
        Self::Simplification,
        MutFoldChainSliceStruct<'b,
            <Self::UnderlyingChain as FoldChainSlice<'a,T,(usize,Self::OriginalD)>>::IsReversed,
            <Self::UnderlyingChain as FoldChainSlice<'a,T,(usize,Self::OriginalD)>>::IsFlushLeft,
            <Self::UnderlyingChain as FoldChainSlice<'a,T,(usize,Self::OriginalD)>>::IsFlushRight,
            T,
            (usize,Self::OriginalD),
            SettingsWithSize<Self::Settings>,
            ()>> {
                (*self).as_mut()
    }
}

/// The struct responsible for almost all views into a [`FoldList`].
pub struct FoldListSliceFrom<'a,T: 'a,D: Clone + 'a,
    Settings: FoldSettings<T,D> + 'a,
    Simplification: FoldSimplification<T,D> + 'a,
    Slice: FoldChainSlice<'a,T,(usize,D),
        Simplification = (), OriginalD = (usize,D),
        Settings=SettingsWithSize<Settings>>> {
    underlying: Slice,
    simplification: Simplification,
    _m: PhantomData<(T,D, &'a T)>
}

impl<'a, T: 'a, D: Clone + 'a, Settings: FoldSettings<T,D> + 'a, Simplification: FoldSimplification<T,D> + 'a, Slice: FoldChainSlice<'a,T,(usize,D), Simplification = (), OriginalD = (usize,D),Settings=SettingsWithSize<Settings>>> 
Sealed for FoldListSliceFrom<'a, T, D, Settings, Simplification, Slice> {}

//is clone if slice is clone
impl<'a, T: 'a, D: Clone + 'a, Settings: FoldSettings<T,D>, Simplification: FoldSimplification<T,D>, Slice: Clone + FoldChainSlice<'a,T,(usize,D),Simplification = (), OriginalD = (usize,D),Settings=SettingsWithSize<Settings>>> 
Clone for FoldListSliceFrom<'a, T, D, Settings, Simplification, Slice> {
    fn clone(&self) -> Self {
        Self { underlying: self.underlying.clone(), simplification: self.simplification.clone(), _m: self._m.clone() }
    }
}
//is copy if slice is copy
impl<'a, T: 'a, D: Clone + 'a, Settings: FoldSettings<T,D> + 'a, Simplification: FoldSimplification<T,D> + 'a, Slice: Copy + FoldChainSlice<'a,T,(usize,D), Simplification = (), OriginalD = (usize,D), Settings=SettingsWithSize<Settings>>> 
Copy for FoldListSliceFrom<'a, T, D, Settings, Simplification, Slice> {}


impl<'a, T: 'a, D: Clone + 'a, Settings: FoldSettings<T,D>, Simplification: FoldSimplification<T,D>, Slice: FoldChainSlice<'a,T,(usize,D),Simplification = (), OriginalD = (usize,D),Settings=SettingsWithSize<Settings>>> 
FoldListSlice<'a,T,Simplification::D2> for FoldListSliceFrom<'a, T, D, Settings, Simplification, Slice> {
    type OriginalD = D;
    type Simplification = Simplification;
    type Settings = Settings;

    fn get_settings(&self) -> Self::Settings {
        self.underlying.get_settings().0
    }

    type UnderlyingChain = Slice;

    fn get_current_simplification(&self) -> Self::Simplification {
        self.simplification.clone()
    }

    fn as_sized_chain(self) -> Self::UnderlyingChain {
        self.underlying
    }

    fn borrow<'b>(&'b self) -> FoldListSliceFrom<'b,T,Self::OriginalD,
        Self::Settings,
        Self::Simplification,
        ImmFoldChainSliceStruct<'b,
            <Self::UnderlyingChain as FoldChainSlice<'a,T,(usize,Self::OriginalD)>>::IsReversed,
            <Self::UnderlyingChain as FoldChainSlice<'a,T,(usize,Self::OriginalD)>>::IsFlushLeft,
            <Self::UnderlyingChain as FoldChainSlice<'a,T,(usize,Self::OriginalD)>>::IsFlushRight,
            SettingsWithSize<Self::Settings>,
            (),
            T,
            (usize,Self::OriginalD)>> {
        FoldListSliceFrom { 
            simplification: self.get_current_simplification(),
            underlying: self.underlying.borrow(), 
            _m: PhantomData
        }
    }
}

impl<'a, T: 'a, D: Clone + 'a, Settings: FoldSettings<T,D> + 'a, Simplification: FoldSimplification<T,D> + 'a, Slice: MutFoldChainSlice<'a,T,(usize,D),Simplification = (), OriginalD = (usize,D),Settings=SettingsWithSize<Settings>>> 
MutFoldListSlice<'a,T,Simplification::D2> for FoldListSliceFrom<'a, T, D, Settings, Simplification, Slice> {
    fn as_mut(self) -> FoldListSliceFrom<'a,T,Self::OriginalD,
        Self::Settings,
        Self::Simplification,
        MutFoldChainSliceStruct<'a,
            <Self::UnderlyingChain as FoldChainSlice<'a,T,(usize,Self::OriginalD)>>::IsReversed,
            <Self::UnderlyingChain as FoldChainSlice<'a,T,(usize,Self::OriginalD)>>::IsFlushLeft,
            <Self::UnderlyingChain as FoldChainSlice<'a,T,(usize,Self::OriginalD)>>::IsFlushRight,
            T,
            (usize,Self::OriginalD),
            SettingsWithSize<Self::Settings>,
            ()>> {
                FoldListSliceFrom { 
                    simplification: self.simplification, 
                    underlying: self.underlying.as_mut(), 
                    _m: PhantomData
                }
    }

    fn borrow_mut<'b>(&'b mut self) -> FoldListSliceFrom<'b,T,Self::OriginalD,
        Self::Settings,
        Self::Simplification,
        MutFoldChainSliceStruct<'b,
            <Self::UnderlyingChain as FoldChainSlice<'a,T,(usize,Self::OriginalD)>>::IsReversed,
            <Self::UnderlyingChain as FoldChainSlice<'a,T,(usize,Self::OriginalD)>>::IsFlushLeft,
            <Self::UnderlyingChain as FoldChainSlice<'a,T,(usize,Self::OriginalD)>>::IsFlushRight,
            T,
            (usize,Self::OriginalD),
            SettingsWithSize<Self::Settings>,
            ()>> {
                let FoldListSliceFrom { underlying, simplification, _m } = self;
                FoldListSliceFrom {
                    simplification: *simplification,
                    underlying: underlying.borrow_mut(),
                    _m: PhantomData,
                }
    }
}

impl<T: core::fmt::Debug, D: Clone, Settings: FoldSettings<T,D>> core::fmt::Debug for FoldList<T, D, Settings> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_list().entries(self.iter()).finish()
    }
}

impl<'a, T: 'a, D: Clone + 'a, Settings: FoldSettings<T,D> + 'a, Simplification: FoldSimplification<T,D> + 'a, Slice: FoldChainSlice<'a,T,(usize,D),Simplification = (), OriginalD = (usize,D),Settings=SettingsWithSize<Settings>> + core::fmt::Debug> 
core::fmt::Debug for FoldListSliceFrom<'a, T, D, Settings, Simplification, Slice> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.underlying.fmt(f)
    }
}

impl<T, D: Clone, Settings: FoldSettings<T,D>> IntoIterator for FoldList<T, D, Settings> {
    type Item = T;
    type IntoIter = fold_chain::DrainOwned<T,(usize,D),SettingsWithSize<Settings>>;
    fn into_iter(self) -> Self::IntoIter {
        self.underlying.into_iter()
    }
}

impl<'a,T, D: Clone, Settings: FoldSettings<T,D>> IntoIterator for &'a FoldList<T, D, Settings> {
    type Item = &'a T;
    type IntoIter = fold_chain::Iter<'a,False,T,(usize,D)>;
    fn into_iter(self) -> Self::IntoIter {
        self.underlying.iter()
    }
}

impl<'a, T: 'a, D: Clone + 'a, Settings: FoldSettings<T,D> + 'a, Simplification: FoldSimplification<T,D> + 'a, Slice: FoldChainSlice<'a,T,(usize,D),Simplification = (), OriginalD = (usize,D),Settings=SettingsWithSize<Settings>> + IntoIterator> 
IntoIterator for FoldListSliceFrom<'a, T, D, Settings, Simplification, Slice> {
    type Item = Slice::Item;
    type IntoIter = Slice::IntoIter;
    fn into_iter(self) -> Self::IntoIter {
        self.underlying.into_iter()
    }
}

impl<'a, T: 'a, D: Clone + 'a, Settings: FoldSettings<T,D> + 'a, Simplification: FoldSimplification<T,D> + 'a, Slice: FoldChainSlice<'a,T,(usize,D),Simplification = (), OriginalD = (usize,D),Settings=SettingsWithSize<Settings>> + IntoIterator> 
IntoIterator for &'a FoldListSliceFrom<'a, T, D, Settings, Simplification, Slice> {
    type Item = &'a T;
    type IntoIter = fold_chain::Iter<'a,Slice::IsReversed,T,(usize,D)>;
    fn into_iter(self) -> Self::IntoIter {
        self.underlying.iter()
    }
}

impl<T, D: Clone, Settings: FoldSettings<T,D>> core::ops::Index<usize> for FoldList<T, D, Settings> {
    type Output = T;
    fn index(&self, index: usize) -> &Self::Output {
        foldlist_index_impl(self.borrow(), index)
    }
}

impl<'a, T: 'a, D: Clone + 'a, Settings: FoldSettings<T,D> + 'a> core::ops::Index<usize> for &'a FoldList<T, D, Settings> {
    type Output = T;
    fn index(&self, index: usize) -> &T {
        foldlist_index_impl(self.borrow(), index)
    }
}

impl<'a, T: 'a, D: Clone + 'a, Settings: FoldSettings<T,D> + 'a> core::ops::Index<usize> for &'a mut FoldList<T, D, Settings> {
    type Output = T;
    fn index(&self, index: usize) -> &T {
        foldlist_index_impl(self.borrow(), index)
    }
}

impl<'a, T: 'a, D: Clone + 'a, Settings: FoldSettings<T,D> + 'a, Simplification: FoldSimplification<T,D> + 'a, Slice: FoldChainSlice<'a,T,(usize,D),Simplification = (), OriginalD = (usize,D),Settings=SettingsWithSize<Settings>>> 
core::ops::Index<usize> for FoldListSliceFrom<'a, T, D, Settings, Simplification, Slice> {
    type Output = T;
    fn index(&self, index: usize) -> &T {
        foldlist_index_impl(self.borrow(), index)
    }
}




