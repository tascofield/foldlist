use std::{cell::RefCell, cmp::{max, min}, iter::FusedIterator, marker::PhantomData, ops::Range, rc::Rc};

use foldlist::{fold_chain::FoldChain, fold_settings::FoldSettings, fold_simplification::FoldSimplification, misc::Fun};
use rand::Rng;

pub struct VecBasedFoldChainSlice<T,D: Clone, Settings: FoldSettings<T,D>, Simplification: FoldSimplification<T,D>> {
    pub vec: Rc<RefCell<Vec<T>>>,
    pub start_inc: usize,
    pub end_exc: usize,
    pub parent_end_exc_ptrs: Vec<*mut usize>,
    pub is_reversed: bool,
    pub settings: Settings,
    pub simplification: Simplification,
    pub _m: PhantomData<D>,
}

impl<'a,T, D: Clone, Settings: FoldSettings<T,D>, Simplification: FoldSimplification<T,D>> IntoIterator for &'a mut VecBasedFoldChainSlice<T, D, Settings, Simplification> {
    type Item = &'a mut T;
    type IntoIter = ResultIter<
        &'a mut T,
        core::slice::IterMut<'a,T>,
        core::iter::Rev<core::slice::IterMut<'a,T>>
    >;

    fn into_iter(self) -> Self::IntoIter {
        let nonrev : core::slice::IterMut<'a,T> = unsafe {core::mem::transmute((&mut self.vec.borrow_mut()[self.start_inc..self.end_exc]).into_iter())};
        if self.is_reversed {
            ResultIter::Err(nonrev.rev())
        } else {
            ResultIter::Ok(nonrev)
        }
    }
}

impl<'a,T, D: Clone, Settings: FoldSettings<T,D>, Simplification: FoldSimplification<T,D>> IntoIterator for &'a VecBasedFoldChainSlice<T, D, Settings, Simplification> {
    type Item = &'a T;
    type IntoIter = ResultIter<
        &'a T,
        core::slice::Iter<'a,T>,
        core::iter::Rev<core::slice::Iter<'a,T>>
    >;

    fn into_iter(self) -> Self::IntoIter {
        let nonrev : core::slice::Iter<'a,T> = unsafe {core::mem::transmute((&self.vec.borrow()[self.start_inc..self.end_exc]).into_iter())};
        if self.is_reversed {
            ResultIter::Err(nonrev.rev())
        } else {
            ResultIter::Ok(nonrev)
        }
    }
}

impl<T, D: Clone, Settings: FoldSettings<T,D>, Simplification: FoldSimplification<T,D>> VecBasedFoldChainSlice<T, D, Settings, Simplification> {
    pub fn rand_fold_left(&self, rng: &mut impl Rng) -> Simplification::D2 {
        let len = self.end_exc - self.start_inc;
        let cut = rng.random_range(0..=len);
        let mut fold = self.simplification.empty(self.settings);
        for i in 0..cut {
            fold = self.simplification.op(fold,self.simplification.delta_of(&self.vec.borrow()[self.start_inc + i], self.settings),self.settings);
        }
        fold
    }

    pub fn range_split_by_fold_left(&self,predicate: impl Fn(&Simplification::D2)->bool) -> (Range<usize>,Range<usize>){
        let mut idx = self.start_inc;
        let mut fold_until_idx = self.simplification.empty(self.settings);
        if !predicate(&fold_until_idx) {
            loop {
                if idx >= self.end_exc {break}
                fold_until_idx = self.simplification.op(fold_until_idx,self.simplification.delta_of(&self.vec.borrow()[idx],self.settings),self.settings);
                if predicate(&fold_until_idx) {break} 
                idx += 1;
            }
        }
        (self.start_inc..idx,idx..self.end_exc)
    }

    pub fn range_split_by_fold_right(&self,predicate: impl Fn(&Simplification::D2)->bool) -> (Range<usize>,Range<usize>) {
        let mut idx = self.end_exc;
        let mut fold_from_idx = self.simplification.empty(self.settings);
        if !predicate(&fold_from_idx) {
            loop {
                if idx <= self.start_inc {break}
                fold_from_idx = self.simplification.op(self.simplification.delta_of(&self.vec.borrow()[idx-1], self.settings),fold_from_idx,self.settings);
                if predicate(&fold_from_idx) {break}
                idx -= 1;
            }
        }
        (self.start_inc..idx,idx..self.end_exc)
    }

    pub fn set_range(&mut self, range: Range<usize>) {
        self.start_inc = range.start;
        self.end_exc = range.end;
    }

}

impl<T, D: Clone, Settings: FoldSettings<T,D>, Simplification: FoldSimplification<T,D>> Clone for VecBasedFoldChainSlice<T, D, Settings, Simplification> {
    fn clone(&self) -> Self {
        Self { vec: self.vec.clone(), start_inc: self.start_inc.clone(), end_exc: self.end_exc.clone(), is_reversed: self.is_reversed.clone(), settings: self.settings.clone(), simplification: self.simplification.clone(), _m: self._m.clone(),
            parent_end_exc_ptrs: {
                let mut v = self.parent_end_exc_ptrs.clone();
                v.push(&self.end_exc as *const _ as *mut _);
                v
            },
        }
    }
}

impl<'a, T: 'a, D: Clone + 'a, Settings: FoldSettings<T,D> + 'a, Simplification: FoldSimplification<T,D> + 'a>  VecBasedFoldChainSlice<T, D, Settings, Simplification> {

    pub fn borrow<'b>(&'b self) -> VecBasedFoldChainSlice<T, D, Settings, Simplification> {
        self.clone()
    }

    // fn as_imm(self) -> VecBasedFoldChainSlice<T, D, Settings, Simplification> {
    //     self
    // }

    pub fn view_drop_left_until(mut self, predicate: impl Fn(&Simplification::D2)->bool) -> VecBasedFoldChainSlice<T, D, Settings, Simplification>  {
        if self.is_reversed {
            return self.view_reversed().view_drop_right_until(predicate).view_reversed()
        }
        let range = self.range_split_by_fold_left(predicate).1;
        self.set_range(range);
        self
    }

    pub fn view_take_right_until(mut self, predicate: impl Fn(&Simplification::D2)->bool) -> VecBasedFoldChainSlice<T, D, Settings, Simplification>  {
        if self.is_reversed {
            return self.view_reversed().view_take_left_until(predicate).view_reversed()
        }
        let range = self.range_split_by_fold_right(predicate).1;
        self.set_range(range);
        self
    }


    pub fn view_drop_right_until(mut self, predicate: impl Fn(&Simplification::D2)->bool) -> VecBasedFoldChainSlice<T, D, Settings, Simplification>  {
        if self.is_reversed {
            return self.clone().view_reversed().view_drop_left_until(predicate).view_reversed()
        }
        let range = self.range_split_by_fold_right(predicate).0;
        self.set_range(range);
        self
    }

    pub fn view_take_left_until(mut self, predicate: impl Fn(&Simplification::D2)->bool) -> VecBasedFoldChainSlice<T, D, Settings, Simplification>  {
        if self.is_reversed {
            return self.view_reversed().view_take_right_until(predicate).view_reversed()
        }
        let range = self.range_split_by_fold_left(predicate).0;
        self.set_range(range);
        self
    }

    pub fn view_take_left(mut self, n: usize) -> VecBasedFoldChainSlice<T, D, Settings, Simplification> {
        if self.is_reversed {
            return self.view_reversed().view_take_right(n).view_reversed()
        }
        self.end_exc = min(self.end_exc,self.start_inc + n);
        self
    }

    pub fn view_take_right(mut self, n: usize) -> VecBasedFoldChainSlice<T, D, Settings, Simplification> {
        if self.is_reversed {
            return self.view_reversed().view_take_left(n).view_reversed()
        }
        self.start_inc = max(self.start_inc as isize, self.end_exc as isize - n as isize) as usize;
        self
    }

    pub fn view_drop_left(mut self, n: usize) -> VecBasedFoldChainSlice<T, D, Settings, Simplification> {
        if self.is_reversed {
            return self.view_reversed().view_drop_right(n).view_reversed()
        }
        self.start_inc = min(self.end_exc, self.start_inc + n);
        self
    }

    pub fn view_drop_right(mut self, n: usize) -> VecBasedFoldChainSlice<T, D, Settings, Simplification> {
        if self.is_reversed {
            return self.view_reversed().view_drop_left(n).view_reversed()
        }
        self.end_exc = max(self.start_inc as isize, self.end_exc as isize - n as isize) as usize;
        self
    }

    pub fn view_reversed(mut self) -> VecBasedFoldChainSlice<T, D, Settings, Simplification>  {
        self.is_reversed = !self.is_reversed;
        self
    }

    pub fn view_simplify<D2: Clone + 'a, 
        Simplifier: for<'x> Fun<&'x Simplification::D2,D2> + Copy + 'a,
        OP2: Fun<(D2,D2),D2> + Copy + 'a>(self,simplifier: Simplifier, simplified_op: OP2) 
        -> VecBasedFoldChainSlice<T, D, Settings, <Simplification as FoldSimplification<T, D>>::Compose<D2, Simplifier, OP2>> {
        VecBasedFoldChainSlice {
            vec: self.vec,
            start_inc: self.start_inc,
            end_exc: self.end_exc,
            parent_end_exc_ptrs: self.parent_end_exc_ptrs,
            is_reversed: self.is_reversed,
            settings: self.settings,
            simplification: self.simplification.compose(simplifier, simplified_op),
            _m: PhantomData,
        }
    }

    pub fn view_simplify_with_shortcut<D2: Clone + 'a, 
        Simplifier: for<'x> Fun<&'x Simplification::D2,D2> + Copy + 'a,
        OP2: Fun<(D2,D2),D2> + Copy + 'a,
        EmptyShortcut: Fun<(),D2> + Copy + 'a,
        DeltaShortcut: for<'x> Fun<&'x T, D2> + Copy + 'a>(self,simplifier: Simplifier, simplified_op: OP2, empty_shortcut: EmptyShortcut, delta_shortcut: DeltaShortcut)
        -> VecBasedFoldChainSlice<T, D, Settings, <Simplification as FoldSimplification<T, D>>::ComposeWithShortcut<D2, Simplifier, OP2, EmptyShortcut, DeltaShortcut>>         {
        VecBasedFoldChainSlice {
            vec: self.vec,
            start_inc: self.start_inc,
            end_exc: self.end_exc,
            parent_end_exc_ptrs: self.parent_end_exc_ptrs,
            is_reversed: self.is_reversed,
            settings: self.settings,
            simplification: self.simplification.compose_with_shortcut(simplifier, simplified_op,empty_shortcut,delta_shortcut),
            _m: PhantomData,
        }
    }

    // fn view_unsimplify(self) -> VecBasedFoldChainSlice<T, D, Settings, ()>  {
    //     VecBasedFoldChainSlice {
    //         vec: self.vec,
    //         start_inc: self.start_inc,
    //         end_exc: self.end_exc,
    //         parent_end_exc_ptrs: self.parent_end_exc_ptrs,
    //         is_reversed: self.is_reversed,
    //         settings: self.settings,
    //         simplification: (),
    //         _m: PhantomData,
    //     }
    // }

    pub fn fold(&self) -> Simplification::D2 {
        self.vec.borrow()[self.start_inc..self.end_exc].iter().fold(
            self.simplification.empty(self.settings), 
            |a,b| self.simplification.op(a,self.simplification.delta_of(b, self.settings),self.settings)
        )
    }

    pub fn is_empty(&self) -> bool {
        self.start_inc == self.end_exc
    }

    pub fn left(&self) -> Option<&T> {
        if self.is_empty() {return None}
        if self.is_reversed {
            Some(unsafe {core::mem::transmute(&self.vec.borrow()[self.end_exc - 1])})
        } else {
            Some(unsafe {core::mem::transmute(&self.vec.borrow()[self.start_inc])})
        }
    }

    pub fn right(&self) -> Option<&T> {
        if self.is_empty() {return None}
        if self.is_reversed {
            Some(unsafe {core::mem::transmute(&self.vec.borrow()[self.start_inc])})
        } else {
            Some(unsafe {core::mem::transmute(&self.vec.borrow()[self.end_exc - 1])})
        }
    }

    // fn foreach(&self, mut f: impl FnMut(&T)) {
    //     if self.is_reversed {
    //         for t in (&self.vec.borrow()[self.start_inc..self.end_exc]).iter().rev() {
    //             f(t)
    //         }
    //     } else {
    //         for t in &self.vec.borrow()[self.start_inc..self.end_exc] {
    //             f(t)
    //         }
    //     }
    // }
    
    pub fn debug_check_structural_integrity(&self) -> bool {
        true
    }

    pub fn borrow_mut<'b>(&'b mut self) -> VecBasedFoldChainSlice<T, D, Settings, Simplification>  {
        self.clone()
    }
    
    pub fn mut_view_drop_left_until(self, predicate: impl Fn(&Simplification::D2)->bool) -> VecBasedFoldChainSlice<T, D, Settings, Simplification> {
        self.view_drop_left_until(predicate)
    }

    pub fn mut_view_take_right_until(self, predicate: impl Fn(&Simplification::D2)->bool) -> VecBasedFoldChainSlice<T, D, Settings, Simplification>  {
        self.view_take_right_until(predicate)
    }

    pub fn mut_view_drop_right_until(self, predicate: impl Fn(&Simplification::D2)->bool) -> VecBasedFoldChainSlice<T, D, Settings, Simplification>  {
        self.view_drop_right_until(predicate)
    }

    pub fn mut_view_take_left_until(self, predicate: impl Fn(&Simplification::D2)->bool) -> VecBasedFoldChainSlice<T, D, Settings, Simplification>  {
        self.view_take_left_until(predicate)
    }

    pub fn mut_view_reversed(self) -> VecBasedFoldChainSlice<T, D, Settings, Simplification>  {
        self.view_reversed()
    }

    pub fn mut_view_simplify<D2: Clone + 'a, 
        Simplifier: for<'x> Fun<&'x Simplification::D2,D2> + Copy + 'a,
        OP2: Fun<(D2,D2),D2> + Copy + 'a>(self,simplifier: Simplifier, simplified_op: OP2) 
        -> VecBasedFoldChainSlice<T, D, Settings, <Simplification as FoldSimplification<T, D>>::Compose<D2, Simplifier, OP2>> {
        self.view_simplify(simplifier, simplified_op)
    }

    // fn mut_view_simplify_with_shortcut<D2: Clone + 'a, 
    //     Simplifier: for<'x> Fun<&'x Simplification::D2,D2> + Copy + 'a,
    //     OP2: Fun<(D2,D2),D2> + Copy + 'a,
    //     EmptyShortcut: Fun<(),D2> + Copy + 'a,
    //     DeltaShortcut: for<'x> Fun<&'x T, D2> + Copy + 'a>(self,simplifier: Simplifier, simplified_op: OP2, empty_shortcut: EmptyShortcut, delta_shortcut: DeltaShortcut)
    //     -> VecBasedFoldChainSlice<T, D, Settings, <Simplification as FoldSimplification<T, D>>::ComposeWithShortcut<D2, Simplifier, OP2, EmptyShortcut, DeltaShortcut>> {
    //     self.view_simplify_with_shortcut(simplifier, simplified_op, empty_shortcut, delta_shortcut)
    // }

    // fn mut_view_unsimplify(self) -> VecBasedFoldChainSlice<T, D, Settings, ()> {
    //     self.view_unsimplify()
    // }

    pub fn pop_left(&mut self) -> Option<T> {
        if self.is_empty() {return None}
        if self.is_reversed {
            return self.clone().view_reversed().pop_right()
        }
        let ret = Some(self.vec.borrow_mut().remove(self.start_inc));
        self.end_exc -= 1;
        for ptr in &self.parent_end_exc_ptrs {
            unsafe {
                **ptr -= 1;
            }
        }
        ret
    }

    pub fn pop_right(&mut self) -> Option<T> {
        if self.is_empty() {return None}
        if self.is_reversed {
            return self.clone().view_reversed().pop_left()
        }
        let ret = Some(self.vec.borrow_mut().remove(self.end_exc-1));
        self.end_exc -= 1;
        for ptr in &self.parent_end_exc_ptrs {
            unsafe {
                **ptr -= 1;
            }
        }
        ret
    }

    pub fn append_left(&mut self, value: T) {
        if self.is_reversed {
            return self.clone().view_reversed().append_right(value);
        }
        self.vec.borrow_mut().insert(self.start_inc,value);
        self.end_exc+=1;
        for ptr in &self.parent_end_exc_ptrs {
            unsafe {
                **ptr += 1;
            }
        }
    }

    pub fn append_right(&mut self, value: T) {
        if self.is_reversed {
            return self.clone().view_reversed().append_left(value);
        }
        self.vec.borrow_mut().insert(self.end_exc,value);
        self.end_exc+=1;
        for ptr in &self.parent_end_exc_ptrs {
            unsafe {
                **ptr += 1;
            }
        }
    }

    pub fn set_left_or_err(&mut self, value: T) -> Result<T,T> {
        if self.is_empty() {return Err(value)}
        if self.is_reversed {
            Ok(core::mem::replace(&mut self.vec.borrow_mut()[self.end_exc-1],value))
        } else {
            Ok(core::mem::replace(&mut self.vec.borrow_mut()[self.start_inc],value))
        }
    }

    pub fn set_right_or_err(&mut self, value: T) -> Result<T,T> {
        if self.is_empty() {return Err(value)}
        if self.is_reversed {
            Ok(core::mem::replace(&mut self.vec.borrow_mut()[self.start_inc],value))
        } else {
            Ok(core::mem::replace(&mut self.vec.borrow_mut()[self.end_exc-1],value))
        }
    }

    pub fn update_left<R>(&mut self, f: impl FnOnce(Option<&mut T>) -> R) -> R {
        if self.is_empty() {return f(None)}
        if self.is_reversed {
            f(Some(&mut self.vec.borrow_mut()[self.end_exc - 1]))
        } else {
            f(Some(&mut self.vec.borrow_mut()[self.start_inc]))
        }
    }

    pub fn update_right<R>(&mut self, f: impl FnOnce(Option<&mut T>) -> R) -> R {
        if self.is_empty() {return f(None)}
        if self.is_reversed {
            f(Some(&mut self.vec.borrow_mut()[self.start_inc]))
        } else {
            f(Some(&mut self.vec.borrow_mut()[self.end_exc - 1]))
        }
    }

    pub fn foreach_mut(&mut self, mut f: impl FnMut(&mut T)) {
        if self.is_reversed {
            for t in (&mut self.vec.borrow_mut()[self.start_inc..self.end_exc]).iter_mut().rev() {
                f(t)
            }
        } else {
            for t in &mut self.vec.borrow_mut()[self.start_inc..self.end_exc] {
                f(t)
            }
        }
    }

    pub fn take_all(&mut self) -> FoldChain<T, D, Settings>  {
        let mut ret = FoldChain::from_settings(self.settings);
        while !self.is_empty() {
            (&mut ret).append_left(self.pop_right().unwrap())
        }
        ret
    }

    pub fn append_all_right(&mut self, mut chain: FoldChain<T,D,Settings>) {
        while !(&mut chain).is_empty() {
            self.append_right((&mut chain).pop_left().unwrap());
        }
    }

    pub fn append_all_left(&mut self, mut chain: FoldChain<T,D,Settings>) {
        while !(&mut chain).is_empty() {
            self.append_left((&mut chain).pop_right().unwrap());
        }
    }
}



pub enum ResultIter<T,I: Iterator<Item=T>, J: Iterator<Item=T>> {
    Ok(I),
    Err(J)
}

impl<T, I: Iterator<Item=T>, J: Iterator<Item=T>> Iterator for ResultIter<T, I, J> {
    type Item = T;
    fn next(&mut self) -> Option<Self::Item> {
        match self {
            ResultIter::Ok(i) => i.next(),
            ResultIter::Err(j) => j.next(),
        }
    }
}

impl<T, I: Iterator<Item=T> + DoubleEndedIterator, J: Iterator<Item=T> + DoubleEndedIterator> DoubleEndedIterator for ResultIter<T, I, J> {
    fn next_back(&mut self) -> Option<Self::Item> {
        match self {
            ResultIter::Ok(i) => i.next_back(),
            ResultIter::Err(j) => j.next_back(),
        }
    }
}

impl<T, I: Iterator<Item=T> + FusedIterator, J: Iterator<Item=T> + FusedIterator> FusedIterator for ResultIter<T, I, J> {}
