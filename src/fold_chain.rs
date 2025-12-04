use core::panic;
use core::{iter::FusedIterator, marker::PhantomData, ptr::NonNull};

use crate::fold_settings::SettingsWithSize;
use crate::misc::{NoneFun, OptOpFun, SingleEndedRange, SomeFun, TupleFun};
use crate::{fold_list::{FoldList}, fold_settings::{FoldSettings, FoldSettingsStruct}, fold_simplification::{FoldSimplification, SimplificationWithShortcut, SimplificationWithoutShortcut}, misc::{bool_assert_into, bool_ifelse_clone, cswap, Bool, EmptyFn, False, Fun, True}};

//https://en.wikipedia.org/wiki/WAVL_tree
pub(crate) struct WAVLNode<T,D> {
    value: T,
    delta_whole: D,
    rank: u8,
    left: Option<Box<WAVLNode<T,D>>>,
    right: Option<Box<WAVLNode<T,D>>>,
    is_right_child: bool,
    parent_ptr: Option<NonNull<WAVLNode<T,D>>>,
    _pin: std::marker::PhantomPinned
}

impl<T,D: Clone> WAVLNode<T,D> {
    fn new_leaf_unboxed<P: FoldSettings<T,D>>(p: P, value: T) -> WAVLNode<T,D> {
        let d = p.delta_of(&value);
        Self { parent_ptr: None, value, rank: 0, delta_whole: d, left: None, right: None, is_right_child: false, _pin: std::marker::PhantomPinned }
    }

    fn new_leaf< P: FoldSettings<T,D>>(p: P, value: T) -> Option<Box<WAVLNode<T,D>>> {
        let ret = Box::new(Self::new_leaf_unboxed(p, value));
        Some(ret)
    }

    fn rank(this: &Option<Box<Self>>) -> isize {
        match this {
            Some(this) => this.rank as isize,
            None => -1,
        }
    }

    //like update, but we know no ranks have changed, and the children still know we're their parent
    fn recalc_delta<P: FoldSettings<T,D>>(&mut self,p: P) {
        let d0 = p.delta_of(&self.value);
        let d1 = if let Some(l) = &self.left {
            p.op(l.delta_whole.clone(),d0)
        } else {d0};
        let d2 = if let Some(r) = &self.right {
            p.op(d1,r.delta_whole.clone())
        } else {d1};
        self.delta_whole = d2;
    }
    
    fn inform_children_and_recalc<P: FoldSettings<T,D>>(&mut self,p: P) {
        let self_ptr: NonNull<WAVLNode<T,D>> = self.into();
        let d0 = p.delta_of(&self.value);
        let d1 = if let Some(left) = &mut self.left {
            left.is_right_child = false;
            left.parent_ptr = Some(self_ptr);

            p.op(left.delta_whole.clone(),d0)
        } else {d0};
        let d2 = if let Some(right) = &mut self.right {
            right.is_right_child = true;
            right.parent_ptr = Some(self_ptr);

            p.op(d1,right.delta_whole.clone())
        } else {d1};
        self.delta_whole = d2;
    }

    fn update_and_rebalance_continues(self_opt: &mut Option<Box<Self>>, settings: impl FoldSettings<T,D>) -> bool {
        let Some(this) = self_opt else {return false};
        let left_rank = Self::rank(&this.left);
        let right_rank = Self::rank(&this.right);
        let child_ranks_diff = left_rank - right_rank;
        let left_is_smaller = child_ranks_diff < 0;
        let child_ranks_diff_abs = if left_is_smaller {-child_ranks_diff} else {child_ranks_diff} as usize;
        match child_ranks_diff_abs {
            3.. => {
                let (left,right) = this.shed_children(settings);
                WAVLNode::append_tree_left_opt(self_opt, left, settings);
                WAVLNode::append_tree_right_opt(self_opt,right,settings);
                true
            },
            2 => {
                if left_is_smaller {
                    WAVLNode::rebalance_when_left_rank_is_greater_by_2_and_rank_changed_template::<True>(this, settings)
                } else {
                    WAVLNode::rebalance_when_left_rank_is_greater_by_2_and_rank_changed_template::<False>(this, settings)
                }
            },
            0..=1 => {
                this.inform_children_and_recalc_and_rank_update_was_necessary_assuming_balanced(
                    if left_is_smaller {left_rank as i8} else {right_rank as i8}, 
                    left_rank != right_rank, 
                    settings
                )
            },
        }
    }

    fn inform_children_and_recalc_and_rank_update_was_necessary_assuming_balanced(&mut self, smaller_child_rank: i8, children_have_different_ranks: bool, settings: impl FoldSettings<T,D>) -> bool {
        self.inform_children_and_recalc(settings);
        let smaller_plus_one = (smaller_child_rank + 1) as u8;
        let larger_plus_one = smaller_plus_one + children_have_different_ranks as u8;
        let new_rank = if children_have_different_ranks {
            larger_plus_one
        } else {
            //we could either be smaller+1 or larger+1
            //first, prefer to keep it the same
            if (smaller_plus_one..=smaller_plus_one+1).contains(&self.rank) && larger_plus_one != 0 {
                return false
            }
            //let's be the once that's closer to a multiple of 3, settling ties by the one closer to a multiple of 6
            let smaller_mod_6 = smaller_child_rank as u8 % 6;
            smaller_plus_one + ((1 << smaller_mod_6) & 0b110100 != 0) as u8
        };
        return core::mem::replace(&mut self.rank,new_rank) != new_rank
    }

    fn rebalance_when_left_rank_is_greater_by_2_and_rank_changed_template<Reversed: Bool>(this: &mut Box<Self>, settings: impl FoldSettings<T,D>) -> bool {
        let _this_former_rank = this.rank;
        let left_opt_mut = this.left_child_template_mut::<Reversed>();
        let Some(left_mut) = left_opt_mut else {unreachable!()};
        let n = left_mut.rank as isize;
        //     this_former_rank
        //         /   \
        //        n    n-2
        //       /  \
        //      ?    ?
        let lr_rank_diff_is_2 = n - Self::rank(left_mut.left_child_template::<Reversed::Not>()) > 1;
        if lr_rank_diff_is_2 {
            //     this_former_rank
            //         /   \
            //        n    n-2
            //       /  \
            //    n-1|2  n-2
            let ll_rank_diff_is_2 = n - Self::rank(left_mut.left_child_template::<Reversed>()) > 1;
            if ll_rank_diff_is_2 {
                //we can just demote x
                left_mut.rank = (n-1) as u8;
                return this.inform_children_and_recalc_and_rank_update_was_necessary_assuming_balanced((n-2) as i8, true, settings)
            }
            //     this_former_rank
            //         /   \
            //        n    n-2
            //       /  \
            //    n-1    n-2
            // see "Rotate" in [https://sidsen.azurewebsites.net/papers/rb-trees-talg.pdf#page=7]
            let (x,c) = this.shed_children_cswap::<Reversed>(settings);
            let mut x = x.unwrap();
            let (a,b) = x.shed_children_cswap::<Reversed>(settings);
            let mut z = core::mem::replace(this,x);
            let x = this;
            let None = core::mem::replace(z.left_child_template_mut::<Reversed::Not>(),c) else {unreachable!()};
            let None = core::mem::replace(z.left_child_template_mut::<Reversed>(),b) else {unreachable!()};
            z.rank = (n - 1) as u8;
            z.inform_children_and_recalc(settings);
            let None = core::mem::replace(x.left_child_template_mut::<Reversed::Not>(),Some(z)) else {unreachable!()};
            let None = core::mem::replace(x.left_child_template_mut::<Reversed>(),a) else {unreachable!()};
            x.rank = n as u8;
            x.parent_ptr = None;
            x.inform_children_and_recalc(settings);
            //           n
            //         /   \
            //    n-1|2    n-1
            //             /  \
            //           n-2    n-2
            return true
        } else {
            //     this_former_rank
            //         /   \
            //        n    n-2
            //       /  \
            //  n-1|2    n-1
            //         /    \
            //       n-2|3   n-2|3
            // see "Double Rotate" in [https://sidsen.azurewebsites.net/papers/rb-trees-talg.pdf#page=7]
            let x = left_mut;
            let mut y = core::mem::take(x.left_child_template_mut::<Reversed::Not>()).unwrap();
            let (b,c) = y.shed_children_cswap::<Reversed>(settings);
            let None = core::mem::replace(x.left_child_template_mut::<Reversed::Not>(),b) else {unreachable!()};
            x.rank = (n - 1) as u8;
            let mut x = core::mem::replace(this.left_child_template_mut::<Reversed>(),c);
            let maybe_x_rank_changed = WAVLNode::update_and_rebalance_continues(&mut x, settings);
            let x_is_xl = maybe_x_rank_changed && Self::rank(&mut x) != n-1;
            //handle the case where A only has a rank difference of 1 with x
            //and also inform children etc
            debug_assert!(!x_is_xl || x.as_ref().unwrap().rank == n as u8);
            let None = core::mem::replace(y.left_child_template_mut::<Reversed>(),x) else {unreachable!()};
            let mut z = core::mem::replace(this,y);
            z.rank = (n-1) as u8;
            z.inform_children_and_recalc(settings);
            let y = this;
            let None = core::mem::replace(y.left_child_template_mut::<Reversed::Not>(),Some(z)) else {unreachable!()};
            y.rank = (n+1) as u8;
            y.parent_ptr = None;
            //              n+1
            //            /     \
            //           /       \
            //         n-1|n       n-1
            //                   /     \
            //               n-2|3    n-2
            y.inform_children_and_recalc_and_rank_update_was_necessary_assuming_balanced((n-1) as i8, x_is_xl, settings);
            return true
        }
    }

    fn get_first_node_where_fold_left_is_template<Reversed: Bool, P: FoldSettings<T,D>, S: FoldSimplification<T,D>>(
        &self, 
        p: P, 
        s: S,
        predicate: impl Fn(&S::D2)->bool,
        pre_fold: S::D2) -> (S::D2,Option<&Self>) {
            if predicate(&pre_fold) {
                return (pre_fold,Some(self.all_the_way_left_template::<Reversed>()))
            }
            let entire_fold = s.op_cswap::<Reversed>(pre_fold.clone(), s.simplify(&self.delta_whole),p);
            if !predicate(&entire_fold) {
                return (entire_fold,None)
            }
            return inner::<Reversed,_,_,_,_>(self,p,s,predicate,pre_fold);
            fn inner<Reversed: Bool, P: FoldSettings<T,D>,S: FoldSimplification<T,D>,T,D: Clone>(
                this: &WAVLNode<T,D>, 
                p: P, 
                s: S,
                predicate: impl Fn(&S::D2)->bool,
                pre_fold: S::D2) -> (S::D2,Option<&WAVLNode<T,D>>) {
                    let (l,r) = cswap::<Reversed,_>(&this.left, &this.right);
                    let v = &this.value;
                    let fold_l = if let Some(l) = l {
                        let fold_l = s.op_cswap::<Reversed>(pre_fold.clone(),s.simplify(&l.delta_whole),p);
                        if predicate(&fold_l) {
                            return inner::<Reversed,_,_,_,_>(&l, p, s,predicate, pre_fold)
                        }
                        fold_l
                    } else {
                        pre_fold
                    };
                    let value_delta = s.delta_of(&v,p);
                    let fold_v = s.op_cswap::<Reversed>(fold_l.clone(),value_delta,p);
                    if predicate(&fold_v) {
                        return (fold_l,Some(this))
                    }
                    let Some(r) = r else {unreachable!()};
                    inner::<Reversed,_,_,_,_>(r, p, s,predicate, fold_v)
                }
    }

    fn left_child_template_mut<Reversed: Bool>(&mut self) -> &mut Option<Box<Self>> {
        if Reversed::b {
            &mut self.right
        } else {
            &mut self.left
        }
    }

    fn left_child_template<Reversed: Bool>(&self) -> &Option<Box<Self>> {
        if Reversed::b {
            &self.right
        } else {
            &self.left
        }
    }

    #[inline]
    fn is_right_child_template<Reversed: Bool>(&self) -> bool {
        if Reversed::b {
            !self.is_right_child
        } else {
            self.is_right_child
        }
    }

    fn next_parent_left_to_right_template<Reversed: Bool>(&self) -> Option<NonNull<Self>> {
        unsafe {
            let mut node_with_parent_next: &WAVLNode<T, D> = self;
            while node_with_parent_next.is_right_child_template::<Reversed>() {
                match node_with_parent_next.parent_ptr {
                    Some(parent_ptr) => 
                        node_with_parent_next = parent_ptr.as_ref(),
                    None => return None,
                }
            }
            let Some(ret) = node_with_parent_next.parent_ptr else {return None};
            Some(ret)
        }
    }

    fn next_single_left_to_right_template<Reversed: Bool>(&self) -> Option<NonNull<Self>> {
        match self.left_child_template::<Reversed::Not>() {
            Some(r) => {
                Some(r.all_the_way_left_template::<Reversed>().into())
            },
            None => {
                self.next_parent_left_to_right_template::<Reversed>()
            },
        }
    }

    fn all_the_way_left_template<Reversed: Bool>(&self) -> &Self {
        let mut ret = self;
        while let Some(l) = ret.left_child_template::<Reversed>() {
            ret = l.as_ref();
        }
        ret
    }

    fn debug_assert_a_has_parent_in_common_with_b_and_is_not_after(a: NonNull<Self>, b: NonNull<Self>) -> bool {
        unsafe {
            if a == b {
                debug_assert!(a.as_ref().debug_check_structural_integrity());
                return true
            }
            let a_rank = a.as_ref().rank;
            let b_rank = b.as_ref().rank;
            if a_rank < b_rank {
                let Some(a_right_parent) = a.as_ref().next_parent_left_to_right_template::<False>() else {return false};
                debug_assert!(Self::debug_assert_a_has_parent_in_common_with_b_and_is_not_after(a_right_parent, b));
            } else {
                let Some(b_left_parent) = b.as_ref().next_parent_left_to_right_template::<True>() else {return false};
                debug_assert!(Self::debug_assert_a_has_parent_in_common_with_b_and_is_not_after(a, b_left_parent));
            }
            true
        }
    }

    fn debug_check_structural_integrity(&self) -> bool {
        fn check_child<T,D: Clone>(this: &WAVLNode<T,D>,child: &Option<Box<WAVLNode<T,D>>>) {
            let crank = WAVLNode::<T,D>::rank(child);
            let cdiff: isize = this.rank as isize - crank;
            if !(1..=2).contains(&cdiff) {
                panic!("Expected child to have a rank difference of 1 or 2, actual: {}",cdiff);
            }
            if let Some(c) = child {
                if c.parent_ptr.unwrap().as_ptr() as *const _ != this as *const _ {
                    panic!("Mismatched parent pointer!!")
                }
                let actually_on_that_side = if c.is_right_child {&this.right} else {&this.left};
                let Some(actually_on_that_side) = actually_on_that_side else {
                    panic!("Expected to be a right child? {}. But parent actually has no child on that side!",c.is_right_child)
                };
                if c.as_ref() as *const _ != actually_on_that_side.as_ref() as *const _{
                    panic!("Child isn't actually on the side it claims to be on")
                }
                c.debug_check_structural_integrity();
            }
        }
        check_child(self, &self.left);
        check_child(self, &self.right);
        true
    }

    unsafe fn rebalance_child_of_template_and_continues<Reversed: Bool>(mut parent: NonNull<Self>, is_right_child: bool, settings: impl FoldSettings<T,D>) -> bool {
        unsafe {
            if Reversed::b {return Self::rebalance_child_of_template_and_continues::<Reversed::Not>(parent, !is_right_child, settings)}
            let node_box = if is_right_child {
                &mut parent.as_mut().right
            } else {
                &mut parent.as_mut().left
            };
            WAVLNode::update_and_rebalance_continues(node_box, settings)
        }
    }

    unsafe fn mutate_box_of_and_update_parents<R>(mut node: NonNull<Self>, mut root: NonNull<Option<Box<Self>>>, settings: impl FoldSettings<T,D>, f: impl FnOnce(&mut Option<Box<Self>>)->R) -> R {
        unsafe {
            match node.as_mut().parent_ptr {
                None => {
                    debug_assert!(node == root.as_ref().as_ref().unwrap().as_ref().into());
                    let ret = f(root.as_mut());
                    if let Some(root_mut) = root.as_mut().as_mut() {
                        root_mut.parent_ptr = None;
                    }
                    ret
                },
                Some(mut parent) => {
                    let node_box = if node.as_ref().is_right_child {
                        &mut parent.as_mut().right
                    } else {
                        &mut parent.as_mut().left
                    };
                    let ret = f(node_box);
                    bubble_up_rebalance_from_node(parent, root, settings);
                    ret
                },
            }
        }
    }

    fn pop_left_in_place_boxed_template_and_rebalance_continues<Reversed: Bool>(this_opt: &mut Option<Box<WAVLNode<T,D>>>, settings: impl FoldSettings<T,D>) -> Option<(Box<WAVLNode<T,D>>,bool)> {
        let this = this_opt.as_mut()?.as_mut();
        let left_child = this.left_child_template_mut::<Reversed>();
        let inner = WAVLNode::pop_left_in_place_boxed_template_and_rebalance_continues::<Reversed>(left_child, settings);
        if let Some((left_popped,rabalance_continues)) = inner {
            let cont2 = if !rabalance_continues {
                this.inform_children_and_recalc(settings);
                false
            } else {
                WAVLNode::update_and_rebalance_continues(this_opt, settings)
            };
            return Some((left_popped,cont2))
        }
        //we have no left child, so we're the leftmost
        let right_child = this.left_child_template_mut::<Reversed::Not>();
        let right_taken = core::mem::take(right_child);
        //so just replace ourself with the right child
        return core::mem::replace(this_opt,right_taken).map(|t| (t,true))
    }

    fn pop_top_in_place_boxed(this_opt: &mut Option<Box<WAVLNode<T,D>>>, settings: impl FoldSettings<T,D>) -> Option<Box<WAVLNode<T,D>>> {
        let this = this_opt.as_mut()?.as_mut();
        let mut our_replacement_opt = if let left @ Some(_) = WAVLNode::pop_left_in_place_boxed_template_and_rebalance_continues::<True>(&mut this.left, settings) {
            left
        } else {
            WAVLNode::pop_left_in_place_boxed_template_and_rebalance_continues::<False>(&mut this.right, settings)
        }.map(|(r,_)| r); //ignore rebalance info because we always rebalance
        if let Some(replacement) = &mut our_replacement_opt {
            let None = core::mem::replace(&mut replacement.left,core::mem::take(&mut this.left)) else {unreachable!()};
            let None = core::mem::replace(&mut replacement.right,core::mem::take(&mut this.right)) else {unreachable!()};
        }
        let ret = core::mem::replace(this_opt,our_replacement_opt);
        WAVLNode::update_and_rebalance_continues(this_opt, settings);
        ret
    }

    fn push_left_and_get_address_of_node_with_new_element_template<Reversed: Bool>(this_opt: &mut Option<Box<Self>>, settings: impl FoldSettings<T,D>, value: T) -> NonNull<WAVLNode<T,D>> {
        let Some(this) = this_opt else {
            let new = Self::new_leaf(settings, value);
            let ret = NonNull::from(new.as_ref().unwrap().as_ref());
            *this_opt = new;
            return ret
        };
        let ret = Self::push_left_and_get_address_of_node_with_new_element_template::<Reversed>(this.left_child_template_mut::<Reversed>(), settings, value);
        Self::update_and_rebalance_continues(this_opt, settings);
        return ret
    }

    //returns true if the parent needs to rebalance
    fn append_tree_right_opt(this_opt: &mut Option<Box<Self>>, mut other_opt: Option<Box<Self>>, settings: impl FoldSettings<T,D>) -> bool {
        let Some(this) = this_opt else {
            *this_opt = other_opt;
            return this_opt.is_some()
        };
        let Some(other) = &mut other_opt else {
            return false
        };
        let this_rank = this.rank as isize;
        let other_rank = other.rank as isize;
        let rank_diff = this_rank - other_rank;
        if rank_diff >= 2 {
            //this is too big. use its right child instead
            let need_rebalance = Self::append_tree_right_opt(&mut this.right, other_opt, settings);
            if need_rebalance {
                return WAVLNode::update_and_rebalance_continues(this_opt, settings)
            } else {
                this.inform_children_and_recalc(settings);
                return false
            }            
        } else if rank_diff > -2 {
            let smaller_rank;
            let larger_rank;
            let prospective_parent = if rank_diff > 0 {
                let Some((p, _)) = Self::pop_left_in_place_boxed_template_and_rebalance_continues::<True>(this_opt, settings) else {unreachable!()};
                larger_rank = Self::rank(&this_opt);
                smaller_rank = other_rank;
                p
            } else {
                let Some((p, _)) = Self::pop_left_in_place_boxed_template_and_rebalance_continues::<False>(&mut other_opt, settings) else {unreachable!()};
                let new_other_rank = Self::rank(&other_opt);
                if new_other_rank < this_rank {
                    larger_rank = this_rank;
                    smaller_rank = new_other_rank;
                } else {
                    larger_rank = new_other_rank;
                    smaller_rank = this_rank;
                }
                p
            };
            let this_taken = core::mem::replace(this_opt,Some(prospective_parent));
            let parent = this_opt.as_mut().unwrap();
            parent.left = this_taken;
            parent.right = other_opt;
            parent.parent_ptr = None;
            parent.inform_children_and_recalc_and_rank_update_was_necessary_assuming_balanced(
                smaller_rank as i8,
                smaller_rank != larger_rank,
                settings
            )
        } else {
            //other is too big
            //use its left child instead
            let to_append_left = core::mem::replace(this,other_opt.unwrap());
            Self::append_tree_left_opt(&mut this.left, Some(to_append_left), settings);
            return WAVLNode::update_and_rebalance_continues(this_opt, settings);
        }
    }


    fn append_tree_left_opt(this_opt: &mut Option<Box<Self>>, other_opt: Option<Box<Self>>, settings: impl FoldSettings<T,D>) -> bool {
        let this = core::mem::replace(this_opt,other_opt);
        Self::append_tree_right_opt(this_opt, this, settings);
        true
    }

    fn append_tree_right_opt_template<Reversed: Bool>(this_opt: &mut Option<Box<Self>>, other_opt: Option<Box<Self>>, settings: impl FoldSettings<T,D>) -> bool {
        if Reversed::b {
            return Self::append_tree_left_opt(this_opt, other_opt, settings);
        } else {
            return Self::append_tree_right_opt(this_opt, other_opt, settings);
        }
    }


    fn append_tree_right_of_top(this: &mut Option<Box<Self>>, other_opt: Option<Box<Self>>, settings: impl FoldSettings<T,D>) {
        Self::append_tree_left_opt(&mut this.as_mut().unwrap().right, other_opt, settings);
        Self::update_and_rebalance_continues(this, settings);
    }

    fn append_tree_left_of_top(this: &mut Option<Box<Self>>, other_opt: Option<Box<Self>>, settings: impl FoldSettings<T,D>) {
        Self::append_tree_right_opt(&mut this.as_mut().unwrap().left, other_opt, settings);
        Self::update_and_rebalance_continues(this, settings);
    }

    fn append_tree_right_of_top_template<Reversed: Bool>(this: &mut Option<Box<Self>>, other_opt: Option<Box<Self>>, settings: impl FoldSettings<T,D>) {
        if Reversed::b {
            Self::append_tree_left_of_top(this,other_opt,settings)
        } else {
            Self::append_tree_right_of_top(this, other_opt, settings);
        }
    }

    fn new_from_iterator_left_to_right_template<Reversed: Bool>(mut iter: impl Iterator<Item=T>, settings: impl FoldSettings<T,D>) -> Option<Box<Self>> {
        fn with_height_estimate<Reversed: Bool,T,D: Clone>(height_est: usize, iter: &mut impl Iterator<Item=T>, settings: impl FoldSettings<T,D>) -> Result<Box<WAVLNode<T,D>>,Option<Box<WAVLNode<T,D>>>> {
            if height_est == 0 {
                let Some(value) = iter.next() else {
                    return Err(None)
                };
                return Ok(WAVLNode::new_leaf(settings, value).unwrap())
            }
            let left = with_height_estimate::<Reversed,_,_>(height_est-1, iter, settings)?;
            let Some(parent) = iter.next() else {
                return Err(Some(left))
            };
            let right = match with_height_estimate::<Reversed,_,_>(height_est - 1, iter, settings) {
                Ok(right) => right,
                Err(right_opt) => {
                    let mut ret = Some(left);
                    WAVLNode::push_left_and_get_address_of_node_with_new_element_template::<Reversed::Not>(&mut ret, settings, parent);
                    WAVLNode::append_tree_right_opt_template::<Reversed>(&mut ret, right_opt, settings);
                    return Err(ret)
                },
            };
            let (mut left,mut right) = cswap::<Reversed,_>(left, right);
            let delta_whole = settings.op(
                left.delta_whole.clone(),
                settings.op(
                    settings.delta_of(&parent),
                    right.delta_whole.clone()
                )
            );
            let rank = ((height_est * 3) / 2) as u8;
            let mut ret = Box::new(WAVLNode {
                parent_ptr: None,
                delta_whole,
                is_right_child: false,
                rank,
                value: parent,
                left: None,
                right: None,
                _pin: std::marker::PhantomPinned,
            });
            right.is_right_child = true;
            left.parent_ptr = Some(ret.as_mut().into());
            right.parent_ptr = Some(ret.as_mut().into());
            ret.left = Some(left);
            ret.right = Some(right);
            Ok(ret)
        }
        fn inc_height<Reversed: Bool,T,D: Clone>(node: Box<WAVLNode<T,D>>, cur_height: usize, iter: &mut impl Iterator<Item=T>, settings: impl FoldSettings<T,D>) -> Result<Box<WAVLNode<T,D>>,Box<WAVLNode<T,D>>> {
            let Some(parent) = iter.next() else {return Err(node)};
            match with_height_estimate::<Reversed,_,_>(cur_height, iter, settings) {
                Ok(right) => {
                    let (mut left,mut right) = cswap::<Reversed,_>(node,right);
                    let delta_whole = settings.op(
                        left.delta_whole.clone(),
                        settings.op(
                            settings.delta_of(&parent),
                            right.delta_whole.clone()
                        )
                    );
                    let rank = (((cur_height + 1) * 3) / 2) as u8;
                    let mut ret = Box::new(WAVLNode {
                        parent_ptr: None,
                        delta_whole,
                        is_right_child: false,
                        rank,
                        value: parent,
                        left: None,
                        right: None,
                        _pin: std::marker::PhantomPinned,
                    });
                    right.is_right_child = true;
                    left.parent_ptr = Some(ret.as_mut().into());
                    right.parent_ptr = Some(ret.as_mut().into());
                    ret.left = Some(left);
                    ret.right = Some(right);
                    Ok(ret)
                },
                Err(right) => {
                    let mut node_opt = Some(node);
                    WAVLNode::push_left_and_get_address_of_node_with_new_element_template::<Reversed::Not>(&mut node_opt, settings, parent);
                    WAVLNode::append_tree_right_opt_template::<Reversed>(&mut node_opt, right, settings);
                    Err(node_opt.unwrap())
                }
            }
        }
        let min_size_hint = iter.size_hint().0;
        let mut cur_height = ((min_size_hint | 2) - 1).ilog2() as usize;
        let mut ret = match with_height_estimate::<Reversed,_,_>(cur_height, &mut iter, settings) {
            Ok(r) => r,
            Err(r) => return r,
        };
        loop {
            ret = match inc_height::<Reversed,_,_>(ret, cur_height, &mut iter, settings) {
                Ok(r) => {
                    cur_height += 1;
                    r
                },
                Err(r) => return Some(r),
            }
        }
    }

    fn shed_children(&mut self, settings: impl FoldSettings<T,D>) -> (Option<Box<Self>>,Option<Box<Self>>) {
        let left = core::mem::take(&mut self.left);
        let right = core::mem::take(&mut self.right);
        self.rank = 0;
        self.delta_whole = settings.delta_of(&self.value);
        (left,right)
    }

    fn shed_children_cswap<DoSwap: Bool>(&mut self, settings: impl FoldSettings<T,D>) -> (Option<Box<Self>>,Option<Box<Self>>) {
        let (l,r) = self.shed_children(settings);
        if DoSwap::b {
            return (r,l)
        } else {
            return (l,r)
        }
    }

    // fn size_estimate_bounds(&self) -> (usize,usize) {
    //     let rank = self.rank as u32;
    //     //lower(0) = 1
    //     //lower(1) = 2
    //     //lower(n) = 1 + 2*lower(n-2)
    //     //solved: 
    //     //https://www.wolframalpha.com/input?i=g%280%29%3D1%2C+g%281%29+%3D+2%2C+g%28n%2B2%29%3D1%2B2*g%28n%29
    //     //lower(n) = 2^((n - 3)/2)*((2 sqrt(2) - 3)*(-1)^n + 3 + 2 sqrt(2)) - 1
    //     //let s = sqrt(2)
    //     //let c = cos(n*π) = (-1)^n
    //     //lower(n) = 2*2^(n/2-3/2)*s*c + 2*2^(n/2-3/2)*s - 3*2^(n/2-3/2)*c + 3*2^(n/2-3/2) - 1
    //     //lower(n) = 2*s^(n-3)*s*c + 2*s^(n-3)*s - 3*s^(n-3)*c + 3*s^(n-3) - 1
    //     //lower(n) = s^n*c + s^n - 3*s^(n-3)*c + 3*s^(n-3) - 1
    //     //assume n is even:
    //     //      c = 1
    //     //      lower(n) = s^n + s^n - 3*s^(n-3) + 3*s^(n-3) - 1
    //     //      lower(n) = s^n + s^n - 1
    //     //      lower(n) = s^(n+2) - 1
    //     //      lower(n) = 2^(n/2+1) - 1
    //     //assume n is odd:
    //     //      c = -1
    //     //      lower(n) = -s^n + s^n + 3*s^(n-3) + 3*s^(n-3) - 1
    //     //      lower(n) = 6*s^(n-3) - 1
    //     //      lower(n) = 3*s^(n-1) - 1
    //     //      lower(n) = 3*2^(floor(n/2)) - 1
    //     let lower_bound = if rank & 1 == 0 {
    //         1_usize.wrapping_shl(rank.wrapping_shr(1) + 1) - 1
    //     } else {
    //         3_usize.wrapping_shl(rank.wrapping_shr(1)) - 1
    //     };
    //     //upper(0) = 1
    //     //upper(n) = 1 * 2*upper(n-1)
    //     //upper(n) = 2^(n+1)-1
    //     let upper_bound = 1_usize.wrapping_shl(rank + 1)-1;
    //     (lower_bound,upper_bound)
    // }

    // unsafe fn bounded_range_size_estimate_bounds(start_ptr: NonNull<Self>, end_ptr: NonNull<Self>) -> (usize,usize) {
    //     unsafe {
    //         if start_ptr == end_ptr {return (1,1)}
    //         let left_next_tree = if let Some(left_child) = start_ptr.as_ref().right {
    //             NonNull::from(left_child.as_ref())
    //         } else {
    //             start_ptr.as_ref().next_parent_left_to_right_template::<False>().expect("Invalid bounds")
    //         };
    //         let right_next_tree = if let Some(right_child) = start_ptr.as_ref().left {
    //             NonNull::from(right_child.as_ref())
    //         } else {
    //             end_ptr.as_ref().next_parent_left_to_right_template::<True>().expect("Invalid bounds")
    //         };

    //         unsafe fn inner_entires<T,D>(left_ptr_entire: NonNull<WAVLNode<T,D>>,right_ptr_entire: NonNull<WAVLNode<T,D>>, lo_acc: isize, hi_acc: isize) {
    
    //         }
    //     }
    //     unsafe {
    //         if start_ptr == end_ptr {
    //             let (hi,lo) = start_ptr.as_ref().size_estimate_bounds();

    //         }
    //     }
    // }
}

impl<T: Clone, D: Clone> WAVLNode<T,D> {
    fn clone_boxed(&self) -> Box<Self> {
        let mut ret = Box::new(Self {
            value: self.value.clone(),
            delta_whole: self.delta_whole.clone(),
            rank: self.rank,
            left: self.left.as_ref().map(|l| l.as_ref().clone_boxed()),
            right: self.right.as_ref().map(|l| l.as_ref().clone_boxed()),
            is_right_child: self.is_right_child,
            parent_ptr: None,
            _pin: std::marker::PhantomPinned,
        });
        let ret_ptr = NonNull::from(ret.as_ref());
        if let Some(l) = &mut ret.left {
            l.parent_ptr = Some(ret_ptr)
        }
        if let Some(r) = &mut ret.right {
            r.parent_ptr = Some(ret_ptr)
        }
        ret
    }
}

unsafe fn bubble_up_fold_from_node<T, D: Clone>(mut node: NonNull<WAVLNode<T,D>>, settings: impl FoldSettings<T,D>) {
    unsafe {
        let node_mut = node.as_mut();
        node_mut.recalc_delta(settings);
        if let Some(parent) = node_mut.parent_ptr {
            bubble_up_fold_from_node(parent, settings);
        }
    }
}

unsafe fn bubble_up_rebalance_from_node<T, D: Clone>(node: NonNull<WAVLNode<T,D>>, mut root: NonNull<Option<Box<WAVLNode<T,D>>>>, settings: impl FoldSettings<T,D>) {
    unsafe {
        let Some(mut parent) = node.as_ref().parent_ptr else {
            debug_assert!(node == root.as_ref().as_ref().unwrap().as_ref().into());
            let root_mut = root.as_mut();
            WAVLNode::update_and_rebalance_continues(root.as_mut(), settings);
            root_mut.as_mut().unwrap().parent_ptr = None;
            return
        };
        let node_box = if node.as_ref().is_right_child {
            &mut parent.as_mut().right
        } else {
            &mut parent.as_mut().left
        };
        if WAVLNode::update_and_rebalance_continues(node_box, settings) {
            bubble_up_rebalance_from_node(parent, root, settings);
        } else {
            bubble_up_fold_from_node(parent, settings);
        }
    }
}

unsafe fn traverse_left_to_right_template<Reversed: Bool, IsFlushLeft: Bool, IsFlushRight: Bool, T,D: Clone, Acc,R>(
    left_inc_single: IsFlushLeft::IfElse<(),NonNull<WAVLNode<T,D>>>, 
    right_inc_single: IsFlushRight::IfElse<(),NonNull<WAVLNode<T,D>>>,
    root_if_both_flush: <IsFlushLeft::And<IsFlushRight> as Bool>::IfElse<NonNull<WAVLNode<T,D>>,()>,
    mut acc: Acc,
    accept_is_single: impl Fn(bool,Acc,&mut WAVLNode<T,D>)->Result<Acc,R> + Copy
) -> Result<Acc,R> {
    unsafe {
        match (IsFlushLeft::b,IsFlushRight::b) {
            (true, true) => accept_is_single(false,acc,<IsFlushLeft::And<IsFlushRight> as Bool>::assert_unwrap(root_if_both_flush).as_mut()),
            (true, false) => {
                //go backwards
                let mut right_inc_single = IsFlushRight::assert_false_unwrap(right_inc_single);
                if let Some(left_parent) = right_inc_single.as_ref().next_parent_left_to_right_template::<Reversed::Not>() {
                    acc = traverse_left_to_right_template::<Reversed,IsFlushLeft,IsFlushRight,_,_,_,_>(left_inc_single, IsFlushRight::assert_false_init(left_parent), root_if_both_flush,acc, accept_is_single)?
                }
                if let Some(left_child) = right_inc_single.as_mut().left_child_template_mut::<Reversed>() {
                    acc = accept_is_single(false,acc,left_child.as_mut())?
                }
                accept_is_single(true,acc,right_inc_single.as_mut())
            },
            (false, true) => {
                //go forwards
                let mut left_inc_single = IsFlushLeft::assert_false_unwrap(left_inc_single);
                acc = accept_is_single(true,acc,left_inc_single.as_mut())?;
                if let Some(right_child) = left_inc_single.as_mut().left_child_template_mut::<Reversed::Not>() {
                    acc = accept_is_single(false,acc,right_child.as_mut())?
                }
                if let Some(right_parent) = left_inc_single.as_mut().next_parent_left_to_right_template::<Reversed>() {
                    traverse_left_to_right_template::<Reversed,IsFlushLeft,IsFlushRight,_,_,_,_>(IsFlushLeft::assert_false_init(right_parent), right_inc_single, root_if_both_flush, acc, accept_is_single)
                } else {
                    Ok(acc)
                }
            },
            (false, false) =>{
                //meet in the middle, alternating by smaller rank
                let mut left_inc_single = IsFlushLeft::assert_false_unwrap(left_inc_single);
                let mut right_inc_single = IsFlushRight::assert_false_unwrap(right_inc_single);
                if left_inc_single == right_inc_single {
                    return accept_is_single(true,acc,left_inc_single.as_mut());
                }
                let left_rank = left_inc_single.as_mut().rank;
                let right_rank = right_inc_single.as_mut().rank;
                if left_rank < right_rank {
                    acc = accept_is_single(true,acc,left_inc_single.as_mut())?;
                    if let Some(right_child) = left_inc_single.as_mut().left_child_template_mut::<Reversed::Not>() {
                        acc = accept_is_single(false,acc,right_child.as_mut())?;
                    }
                    let Some(new_left_single) = left_inc_single.as_mut().next_parent_left_to_right_template::<Reversed>() else {
                        panic!("start inclusive node doesn't have a next parent even though end inclusive should be after it, and start's rank is less")
                    };
                    return traverse_left_to_right_template::<Reversed,IsFlushLeft,IsFlushRight,_,_,_,_>(IsFlushLeft::assert_false_init(new_left_single), IsFlushRight::assert_false_init(right_inc_single), root_if_both_flush,acc, accept_is_single);
                }
                let Some(new_right_single) = right_inc_single.as_mut().next_parent_left_to_right_template::<Reversed::Not>() else {
                    panic!("end inclusive node doesn't have a prev parent even though start inclusive should be before it, and end's rank is less or equal")
                };
                acc = traverse_left_to_right_template::<Reversed,IsFlushLeft,IsFlushRight,_,_,_,_>(IsFlushLeft::assert_false_init(left_inc_single), IsFlushRight::assert_false_init(new_right_single), root_if_both_flush,acc, accept_is_single)?;
                if let Some(left_child) = right_inc_single.as_mut().left_child_template_mut::<Reversed>() {
                    acc = accept_is_single(false,acc,left_child)?;
                }
                accept_is_single(true,acc,right_inc_single.as_mut())
            },
        }
        
    } 
}

unsafe fn node_of_first_where_fold_left_is_template<Reversed: Bool, IsFlushLeft: Bool, IsFlushRight: Bool, P: FoldSettings<T,D>, S: FoldSimplification<T,D>,T,D: Clone>(
    left_inc_single: IsFlushLeft::IfElse<(),NonNull<WAVLNode<T,D>>>, 
    right_inc_single: IsFlushRight::IfElse<(),NonNull<WAVLNode<T,D>>>,
    root_if_both_flush: <IsFlushLeft::And<IsFlushRight> as Bool>::IfElse<NonNull<WAVLNode<T,D>>,()>,
    p: P,
    s: S,
    predicate: impl Fn(&S::D2)->bool,
    pre_fold: S::D2) -> (S::D2,Option<NonNull<WAVLNode<T,D>>>) {
        let res = unsafe {traverse_left_to_right_template::<Reversed,IsFlushLeft,IsFlushRight,_,_,_,_>(left_inc_single,right_inc_single,root_if_both_flush,pre_fold, |is_single, acc, node| {
            if is_single {
                let acc2 = s.op_cswap::<Reversed>(acc.clone(),s.delta_of(&node.value,p),p);
                if predicate(&acc2) {
                    Err((acc,NonNull::from(node)))
                } else {
                    Ok(acc2)
                }
            } else {
                let (acc2,n_opt) = node.get_first_node_where_fold_left_is_template::<Reversed,_,_>(p,s.clone(),&predicate, acc);
                match n_opt {
                    Some(n) => Err((acc2,NonNull::from(n))),
                    None => Ok(acc2),
                }
            }
        })};
        match res {
            Ok(d) => (d,None),
            Err((d,n)) => (d,Some(n)),
        }
}

/// The trait for views into a [`FoldChain`].
/// 
/// For views which are also mutable, see [`MutFoldChainSlice`].
pub trait FoldChainSlice<'a,T: 'a,D: Clone + 'a> : 'a + Sized {
    /// The `D` type of the base [`FoldChain`]. May differ from this `D` if a simplification has been applied.
    type OriginalD: Clone + 'a;

    /// A [type-level boolean](Bool) indicating whether this view has been [reversed](crate#reverse) an odd number of times
    type IsReversed: Bool;

    /// A [type-level boolean](Bool) indicating whether this view necessarily shares the base [`FoldChain`]'s left endpoint
    type IsFlushLeft: Bool;

    /// A [type-level boolean](Bool) indicating whether this view necessarily shares the base [`FoldChain`]'s right endpoint
    type IsFlushRight: Bool;

    /// The type of the current [simplification](crate#simplification)
    type Simplification: FoldSimplification<T,Self::OriginalD,D2 = D>;

    /// Get a copy of the current [simplification](crate#simplification). Will be `()` if no simplification has been applied.
    fn get_current_simplification(&self) -> Self::Simplification;

    /// The type of the base [`FoldChain`]'s [settings](crate#fold-settings).
    type Settings: FoldSettings<T,Self::OriginalD>;

    /// Get a copy of the base [`FoldChain`]'s [settings](crate#fold-settings).
    fn get_settings(&self) -> Self::Settings;

    /// Make this view immutable.
    fn as_imm(self)    ->      ImmFoldChainSliceStruct<'a,Self::IsReversed,Self::IsFlushLeft,Self::IsFlushRight,Self::Settings,Self::Simplification,T,Self::OriginalD>;
    /// Immutably borrow this view.
    fn borrow<'b>(&'b self) -> ImmFoldChainSliceStruct<'b,Self::IsReversed,Self::IsFlushLeft,Self::IsFlushRight,Self::Settings,Self::Simplification,T,Self::OriginalD>;

    /// Contract this view on the left while the to-be-discarded range's fold doesn't meet `predicate`.
    /// 
    /// The given `predicate` must be well-behaved; see [Slicing](crate#slicing) for examples.
    /// 
    /// This operation's mutable version is [`mut_view_drop_left_until`](MutFoldChainSlice::mut_view_drop_left_until) and 
    /// its mirror image is [`view_drop_right_until`](FoldChainSlice::view_drop_right_until).
    /// 
    /// This is equivalent to ```self.view_drop(..predicate)```.
    fn view_drop_left_until(self, predicate: impl Fn(&D)->bool) -> ImmFoldChainSliceStruct<'a,Self::IsReversed,
        <Self::IsFlushLeft as Bool>::And<Self::IsReversed>,
        <Self::IsFlushRight as Bool>::And<<Self::IsReversed as Bool>::Not>,
        Self::Settings,Self::Simplification,T,Self::OriginalD> {
            self.as_imm().view_drop_left_until(predicate)
    }

    /// Restrict this view to the longest range that starts on the right and whose fold doesn't meet `predicate`.
    /// 
    /// The given `predicate` must be well-behaved; see [Slicing](crate#slicing) for examples.
    /// 
    /// This operation's mutable version is [`mut_view_take_right_until`](MutFoldChainSlice::mut_view_take_right_until) and
    /// its mirror image is [`view_take_left_until`](FoldChainSlice::view_take_left_until).
    /// 
    /// This is equivalent to ```self.view_take(predicate..)```.
    fn view_take_right_until(self, predicate: impl Fn(&D)->bool) -> ImmFoldChainSliceStruct<'a,Self::IsReversed,
        <Self::IsFlushLeft as Bool>::And<Self::IsReversed>,
        <Self::IsFlushRight as Bool>::And<<Self::IsReversed as Bool>::Not>,
        Self::Settings,Self::Simplification,T,Self::OriginalD> {
            self.as_imm().view_take_right_until(predicate)
    }

    /// Contract this view on the right while the to-be-discarded range's fold doesn't meet `predicate`.
    /// 
    /// The given `predicate` must be well-behaved; see [Slicing](crate#slicing) for examples.
    /// 
    /// This operation's mutable version is [`mut_view_drop_right_until`](MutFoldChainSlice::mut_view_drop_right_until) and 
    /// its mirror image is [`view_drop_left_until`](FoldChainSlice::view_drop_left_until).
    /// 
    /// This is equivalent to ```self.view_drop(predicate..)```.
    fn view_drop_right_until(self, predicate: impl Fn(&D)->bool) -> ImmFoldChainSliceStruct<'a, 
        <<Self::IsReversed as Bool>::Not as Bool>::Not, 
        <Self::IsFlushLeft as Bool>::And<<Self::IsReversed as Bool>::Not>, 
        <Self::IsFlushRight as Bool>::And<<<Self::IsReversed as Bool>::Not as Bool>::Not>, 
        Self::Settings, Self::Simplification, T, Self::OriginalD> {
            self.as_imm().view_reversed().view_drop_left_until(predicate).view_reversed()
    }

    /// Restrict this view to the longest range that starts on the left and whose fold doesn't meet `predicate`.
    /// 
    /// The given `predicate` must be well-behaved; see [Slicing](crate#slicing) for examples.
    /// 
    /// This operation's mutable version is [`mut_view_take_left_until`](MutFoldChainSlice::mut_view_take_left_until) and
    /// its mirror image is [`view_take_right_until`](FoldChainSlice::view_take_right_until).
    /// 
    /// This is equivalent to ```self.view_take(..predicate)```.
    fn view_take_left_until(self, predicate: impl Fn(&D)->bool) -> ImmFoldChainSliceStruct<'a, 
        <<Self::IsReversed as Bool>::Not as Bool>::Not, 
        <Self::IsFlushLeft as Bool>::And<<Self::IsReversed as Bool>::Not>, 
        <Self::IsFlushRight as Bool>::And<<<Self::IsReversed as Bool>::Not as Bool>::Not>, 
        Self::Settings, Self::Simplification, T, Self::OriginalD>{
            self.as_imm().view_reversed().view_take_right_until(predicate).view_reversed()
    }


    /// If `range` is ```..predicate```, calls [`view_drop_left_until(predicate)`](FoldChainSlice::view_drop_left_until).
    /// 
    /// If `range` is ```predicate..```, calls [`view_drop_right_until(predicate)`](FoldChainSlice::view_drop_right_until) (You may need to parenthesize `predicate` in this case).
    /// 
    /// The given `predicate` must be well-behaved; see [Slicing](crate#slicing) for examples.
    /// 
    /// This operation's mutable version is [`mut_view_drop`](MutFoldChainSlice::mut_view_drop).
    fn view_drop<Predicate: Fn(&D)->bool, Range: SingleEndedRange<Predicate>>(self, range: Range) -> <<Range as SingleEndedRange<Predicate>>::EndIsLeft as Bool>::IfElse<ImmFoldChainSliceStruct<'a, <<<Self as FoldChainSlice<'a, T, D>>::IsReversed as Bool>::Not as Bool>::Not, <<Self as FoldChainSlice<'a, T, D>>::IsFlushLeft as Bool>::And<<<Self as FoldChainSlice<'a, T, D>>::IsReversed as Bool>::Not>, <<Self as FoldChainSlice<'a, T, D>>::IsFlushRight as Bool>::And<<<<Self as FoldChainSlice<'a, T, D>>::IsReversed as Bool>::Not as Bool>::Not>, <Self as FoldChainSlice<'a, T, D>>::Settings, <Self as FoldChainSlice<'a, T, D>>::Simplification, T, <Self as FoldChainSlice<'a, T, D>>::OriginalD>, ImmFoldChainSliceStruct<'a, <Self as FoldChainSlice<'a, T, D>>::IsReversed, <<Self as FoldChainSlice<'a, T, D>>::IsFlushLeft as Bool>::And<<Self as FoldChainSlice<'a, T, D>>::IsReversed>, <<Self as FoldChainSlice<'a, T, D>>::IsFlushRight as Bool>::And<<<Self as FoldChainSlice<'a, T, D>>::IsReversed as Bool>::Not>, <Self as FoldChainSlice<'a, T, D>>::Settings, <Self as FoldChainSlice<'a, T, D>>::Simplification, T, <Self as FoldChainSlice<'a, T, D>>::OriginalD>> {
        Range::EndIsLeft::init_if_else((self,range.end()), 
        |(this,end)| this.view_drop_right_until(end), 
        |(this,end)| this.view_drop_left_until(end), 
        )
    }

    /// If `range` is ```..predicate```, calls [`view_take_left_until(predicate)`](FoldChainSlice::view_take_left_until).
    /// 
    /// If `range` is ```predicate..```, calls [`view_take_right_until(predicate)`](FoldChainSlice::view_take_right_until) (You may need to parenthesize `predicate` in this case).
    /// 
    /// The given `predicate` must be well-behaved; see [Slicing](crate#slicing) for examples.
    /// 
    /// This operation's mutable version is [`mut_view_take`](MutFoldChainSlice::mut_view_take).
    fn view_take<Predicate: Fn(&D)->bool, Range: SingleEndedRange<Predicate>>(self, range: Range) -> <<Range as SingleEndedRange<Predicate>>::EndIsLeft as Bool>::IfElse<ImmFoldChainSliceStruct<'a, <Self as FoldChainSlice<'a, T, D>>::IsReversed, <<Self as FoldChainSlice<'a, T, D>>::IsFlushLeft as Bool>::And<<Self as FoldChainSlice<'a, T, D>>::IsReversed>, <<Self as FoldChainSlice<'a, T, D>>::IsFlushRight as Bool>::And<<<Self as FoldChainSlice<'a, T, D>>::IsReversed as Bool>::Not>, <Self as FoldChainSlice<'a, T, D>>::Settings, <Self as FoldChainSlice<'a, T, D>>::Simplification, T, <Self as FoldChainSlice<'a, T, D>>::OriginalD>, ImmFoldChainSliceStruct<'a, <<<Self as FoldChainSlice<'a, T, D>>::IsReversed as Bool>::Not as Bool>::Not, <<Self as FoldChainSlice<'a, T, D>>::IsFlushLeft as Bool>::And<<<Self as FoldChainSlice<'a, T, D>>::IsReversed as Bool>::Not>, <<Self as FoldChainSlice<'a, T, D>>::IsFlushRight as Bool>::And<<<<Self as FoldChainSlice<'a, T, D>>::IsReversed as Bool>::Not as Bool>::Not>, <Self as FoldChainSlice<'a, T, D>>::Settings, <Self as FoldChainSlice<'a, T, D>>::Simplification, T, <Self as FoldChainSlice<'a, T, D>>::OriginalD>> {
        Range::EndIsLeft::init_if_else((self,range.end()), 
        |(this,end)| this.view_take_right_until(end), 
        |(this,end)| this.view_take_left_until(end), 
        )
    }

    /// Get a reversed version of this view. See [Reverse](crate#reverse).
    /// 
    /// This operation's mutable version is [`mut_view_reversed`](MutFoldChainSlice::mut_view_reversed).
    fn view_reversed(self) -> ImmFoldChainSliceStruct<'a,<Self::IsReversed as Bool>::Not,Self::IsFlushLeft,Self::IsFlushRight,Self::Settings,Self::Simplification,T,Self::OriginalD> {
        self.as_imm().view_reversed()
    }

    /// Compose this view's current simplification with another one explicitly. See [Simplification](crate#simplification).
    /// 
    /// This operation's mutable version is [`mut_view_with_simplification`](MutFoldChainSlice::mut_view_with_simplification).
    fn view_with_simplification<NewSimplification: FoldSimplification<T,D>>(self, new_simplification: NewSimplification) -> ImmFoldChainSliceStruct<'a,Self::IsReversed,Self::IsFlushLeft,Self::IsFlushRight,Self::Settings,
        NewSimplification::ComposeAfterOther<Self::OriginalD,Self::Simplification>,
        T,Self::OriginalD> {
            self.as_imm().view_with_simplification(new_simplification)
    }

    /// Simplify this view. See [Simplification](crate#simplification).
    /// 
    /// This operation's mutable version is [`mut_view_simplify`](MutFoldChainSlice::mut_view_simplify).
    fn view_simplify<D2: Clone + 'a, 
        Simplifier: for<'x> Fun<&'x D,D2> + Copy + 'a,
        OP2: Fun<(D2,D2),D2> + Copy + 'a>(self,simplifier: Simplifier, simplified_op: OP2) 
        -> ImmFoldChainSliceStruct<'a,Self::IsReversed,Self::IsFlushLeft,Self::IsFlushRight,Self::Settings,
            <Self::Simplification as FoldSimplification<T,Self::OriginalD>>::Compose<D2,Simplifier,OP2>,
            T,Self::OriginalD> {
                self.view_with_simplification(SimplificationWithoutShortcut{ 
                    simplifier, 
                    op2: simplified_op, 
                    _m: PhantomData 
                })
    }

    /// Simplify this view in a possibly more efficient way. See [Simplification](crate#simplification).
    /// 
    /// This operation's mutable version is [`mut_view_simplify_with_shortcut`](MutFoldChainSlice::mut_view_simplify_with_shortcut).
    fn view_simplify_with_shortcut<D2: Clone + 'a, 
        Simplifier: for<'x> Fun<&'x D,D2> + Copy + 'a,
        OP2: Fun<(D2,D2),D2> + Copy + 'a,
        EmptyShortcut: Fun<(),D2> + Copy + 'a,
        DeltaShortcut: for<'x> Fun<&'x T, D2> + Copy + 'a>(self,simplifier: Simplifier, simplified_op: OP2, empty_shortcut: EmptyShortcut, delta_shortcut: DeltaShortcut)
        -> ImmFoldChainSliceStruct<'a,Self::IsReversed,Self::IsFlushLeft,Self::IsFlushRight,Self::Settings,
            <Self::Simplification as FoldSimplification<T,Self::OriginalD>>::ComposeWithShortcut<D2,Simplifier,OP2,EmptyShortcut,DeltaShortcut>,
            T,Self::OriginalD> {
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
    /// The [current simplification](FoldChainSlice::get_current_simplification) of the resulting view will be `()`.
    /// 
    /// This operation's mutable version is [`mut_view_unsimplify`](MutFoldChainSlice::mut_view_unsimplify).
    fn view_unsimplify(self) -> ImmFoldChainSliceStruct<'a,Self::IsReversed,Self::IsFlushLeft,Self::IsFlushRight,Self::Settings,(),T,Self::OriginalD> {
        self.as_imm().view_unsimplify()
    }

    /// Get this slice's fold.
    /// 
    /// Note that this is *O*(log(n)) every time.
    fn fold(&self) -> D {
        self.borrow().fold()
    }

    /// Returns true if this slice is empty (that is, when it contains 0 elements).
    /// 
    /// Note that it's sometimes possible for a non-empty slice's fold to be equal to the empty delta.
    fn is_empty(&self) -> bool {
        self.borrow().is_empty()
    }

    /// Get an immutable reference to this slice's leftmost element, if this slice is not empty.
    /// 
    /// Note that if you use this reference to change the element's delta, the backing data structure won't notice, and its folds won't update correctly; see [Mutation / Indexing](crate#mutation--indexing).
    /// 
    /// To mutate this element, see [`update_left`](MutFoldChainSlice::update_left) or [`set_left_or_err`](MutFoldChainSlice::set_left_or_err).
    fn left<'b>(&'b self) -> Option<&'b T> where 'a: 'b {
        self.borrow().left_consume()
    }

    /// Get an immutable reference to this slice's rightmost element, if this slice is not empty.
    /// 
    /// Note that if you use this reference to change the element's delta, the backing data structure won't notice, and its folds won't update correctly; see [Mutation / Indexing](crate#mutation--indexing).
    /// 
    /// To mutate this element, see [`update_right`](MutFoldChainSlice::update_right) or [`set_right_or_err`](MutFoldChainSlice::set_right_or_err).
    fn right<'b>(&'b self) -> Option<&'b T> where 'a: 'b {
        self.borrow().view_reversed().left_consume()
    }

    /// Run a closure for each of this slice's elements, from left to right.
    /// 
    /// See also [`iter`](FoldChainSlice::iter).
    /// 
    /// This operation's mutable version is [`foreach_mut`](MutFoldChainSlice::foreach_mut).
    fn foreach(&self, f: impl FnMut(&T)) {
        self.borrow().foreach(f);
    }

    /// Get an iterator over immutable references to this slice's elements, from left to right.
    /// 
    /// Note that if you use any such reference to change its element's delta, the backing data structure won't notice, and its folds won't update correctly; see [Mutation / Indexing](crate#mutation--indexing).
    /// 
    /// This operation has no mutable version for this same reason, and because in rust, you can't require that an [Iterator] outlive all that it emits (that would be a [streaming iterator](https://docs.rs/streaming-iterator/latest/streaming_iterator/), which is totally different).
    /// 
    /// The closest analogue to a mutable version is [`foreach_mut`](MutFoldChainSlice::foreach_mut).
    fn iter<'b>(&'b self) -> Iter<'b,Self::IsReversed,T,Self::OriginalD> where 'a: 'b {
        self.borrow().iter_consume()
    }

    /// [`debug_assert!`] that the backing data structure is in a valid state. You should never have to use this.
    fn debug_check_structural_integrity(&self) -> bool;
}


/// The trait for mutable views into a [`FoldChain`].
/// 
/// This is a subtrait of [`FoldChainSlice`].
pub trait MutFoldChainSlice<'a,T: 'a,D: Clone + 'a> : FoldChainSlice<'a,T,D> {
    /// Normalize this view's type. This is mostly useless.
    fn as_mut(self) -> MutFoldChainSliceStruct<'a,Self::IsReversed,Self::IsFlushLeft,Self::IsFlushRight,T,Self::OriginalD,Self::Settings,Self::Simplification>;

    /// Mutably borrow this view.
    fn borrow_mut<'b>(&'b mut self) -> MutFoldChainSliceStruct<'b,Self::IsReversed,Self::IsFlushLeft,Self::IsFlushRight,T,Self::OriginalD,Self::Settings,Self::Simplification>;

    /// Contract this view on the left while the to-be-discarded range's fold doesn't meet `predicate`.
    /// 
    /// The given `predicate` must be well-behaved; see [Slicing](crate#slicing) for examples.
    /// 
    /// This operation's immutable version is [`view_drop_left_until`](FoldChainSlice::view_drop_left_until) and 
    /// its mirror image is [`mut_view_drop_right_until`](MutFoldChainSlice::mut_view_drop_right_until).
    /// 
    /// This is equivalent to ```self.mut_view_drop(..predicate)```.
    fn mut_view_drop_left_until(self, predicate: impl Fn(&D)->bool) -> MutFoldChainSliceStruct<'a,Self::IsReversed,
        <Self::IsFlushLeft as Bool>::And<Self::IsReversed>,
        <Self::IsFlushRight as Bool>::And<<Self::IsReversed as Bool>::Not>,
        T,Self::OriginalD,Self::Settings,Self::Simplification> {
            self.as_mut().mut_view_drop_left_until(predicate)
    }

    /// Restrict this view to the longest range that starts on the right and whose fold doesn't meet `predicate`.
    /// 
    /// The given `predicate` must be well-behaved; see [Slicing](crate#slicing) for examples.
    /// 
    /// This operation's immutable version is [`view_take_right_until`](FoldChainSlice::view_take_right_until) and
    /// its mirror image is [`mut_view_take_left_until`](MutFoldChainSlice::mut_view_take_left_until).
    /// 
    /// This is equivalent to ```self.mut_view_take(predicate..)```.
    fn mut_view_take_right_until(self, predicate: impl Fn(&D)->bool) -> MutFoldChainSliceStruct<'a,Self::IsReversed,
        <Self::IsFlushLeft as Bool>::And<Self::IsReversed>,
        <Self::IsFlushRight as Bool>::And<<Self::IsReversed as Bool>::Not>,
        T,Self::OriginalD,Self::Settings,Self::Simplification> {
            self.as_mut().mut_view_take_right_until(predicate)
    }

    /// Contract this view on the right while the to-be-discarded range's fold doesn't meet `predicate`.
    /// 
    /// The given `predicate` must be well-behaved; see [Slicing](crate#slicing) for examples.
    /// 
    /// This operation's immutable version is [`view_drop_right_until`](FoldChainSlice::view_drop_right_until) and 
    /// its mirror image is [`mut_view_drop_left_until`](MutFoldChainSlice::mut_view_drop_left_until).
    /// 
    /// This is equivalent to ```self.mut_view_drop(predicate..)```.
    fn mut_view_drop_right_until(self, predicate: impl Fn(&D)->bool) -> MutFoldChainSliceStruct<'a, 
        <<Self::IsReversed as Bool>::Not as Bool>::Not, 
        <Self::IsFlushLeft as Bool>::And<<Self::IsReversed as Bool>::Not>, 
        <Self::IsFlushRight as Bool>::And<<<Self::IsReversed as Bool>::Not as Bool>::Not>, 
        T, Self::OriginalD, Self::Settings, Self::Simplification> {
            self.as_mut().mut_view_reversed().mut_view_drop_left_until(predicate).mut_view_reversed()
    }

    /// Restrict this view to the longest range that starts on the left and whose fold doesn't meet `predicate`.
    /// 
    /// The given `predicate` must be well-behaved; see [Slicing](crate#slicing) for examples.
    /// 
    /// This operation's immutable version is [`view_take_left_until`](FoldChainSlice::view_take_left_until) and
    /// its mirror image is [`mut_view_take_right_until`](MutFoldChainSlice::mut_view_take_right_until).
    /// 
    /// This is equivalent to ```self.mut_view_take(..predicate)```.
    fn mut_view_take_left_until(self, predicate: impl Fn(&D)->bool) -> MutFoldChainSliceStruct<'a, 
        <<Self::IsReversed as Bool>::Not as Bool>::Not, 
        <Self::IsFlushLeft as Bool>::And<<Self::IsReversed as Bool>::Not>, 
        <Self::IsFlushRight as Bool>::And<<<Self::IsReversed as Bool>::Not as Bool>::Not>, 
        T, Self::OriginalD, Self::Settings, Self::Simplification> {
            self.as_mut().mut_view_reversed().mut_view_take_right_until(predicate).mut_view_reversed()
    }

    /// If `range` is ```..predicate```, calls [`mut_view_drop_left_until(predicate)`](MutFoldChainSlice::mut_view_drop_left_until).
    /// 
    /// If `range` is ```predicate..```, calls [`mut_view_drop_right_until(predicate)`](MutFoldChainSlice::mut_view_drop_right_until) (You may need to parenthesize `predicate` in this case).
    /// 
    /// The given `predicate` must be well-behaved; see [Slicing](crate#slicing) for examples.
    /// 
    /// This operation's immutable version is [`view_drop`](FoldChainSlice::view_drop).
    fn mut_view_drop<Predicate: Fn(&D)->bool, Range: SingleEndedRange<Predicate>>(self, range: Range) -> <<Range as SingleEndedRange<Predicate>>::EndIsLeft as Bool>::IfElse<MutFoldChainSliceStruct<'a, <<<Self as FoldChainSlice<'a, T, D>>::IsReversed as Bool>::Not as Bool>::Not, <<Self as FoldChainSlice<'a, T, D>>::IsFlushLeft as Bool>::And<<<Self as FoldChainSlice<'a, T, D>>::IsReversed as Bool>::Not>, <<Self as FoldChainSlice<'a, T, D>>::IsFlushRight as Bool>::And<<<<Self as FoldChainSlice<'a, T, D>>::IsReversed as Bool>::Not as Bool>::Not>, T, <Self as FoldChainSlice<'a, T, D>>::OriginalD, <Self as FoldChainSlice<'a, T, D>>::Settings, <Self as FoldChainSlice<'a, T, D>>::Simplification>, MutFoldChainSliceStruct<'a, <Self as FoldChainSlice<'a, T, D>>::IsReversed, <<Self as FoldChainSlice<'a, T, D>>::IsFlushLeft as Bool>::And<<Self as FoldChainSlice<'a, T, D>>::IsReversed>, <<Self as FoldChainSlice<'a, T, D>>::IsFlushRight as Bool>::And<<<Self as FoldChainSlice<'a, T, D>>::IsReversed as Bool>::Not>, T, <Self as FoldChainSlice<'a, T, D>>::OriginalD, <Self as FoldChainSlice<'a, T, D>>::Settings, <Self as FoldChainSlice<'a, T, D>>::Simplification>> {
        Range::EndIsLeft::init_if_else((self,range.end()), 
        |(this,end)| this.mut_view_drop_right_until(end), 
        |(this,end)| this.mut_view_drop_left_until(end), 
        )
    }

    /// If `range` is ```..predicate```, calls [`mut_view_take_left_until(predicate)`](MutFoldChainSlice::mut_view_take_left_until).
    /// 
    /// If `range` is ```predicate..```, calls [`mut_view_take_right_until(predicate)`](MutFoldChainSlice::mut_view_take_right_until) (You may need to parenthesize `predicate` in this case).
    /// 
    /// The given `predicate` must be well-behaved; see [Slicing](crate#slicing) for examples.
    /// 
    /// This operation's immutable version is [`view_take`](FoldChainSlice::view_take).
    fn mut_view_take<Predicate: Fn(&D)->bool, Range: SingleEndedRange<Predicate>>(self, range: Range) -> <<Range as SingleEndedRange<Predicate>>::EndIsLeft as Bool>::IfElse<MutFoldChainSliceStruct<'a, <Self as FoldChainSlice<'a, T, D>>::IsReversed, <<Self as FoldChainSlice<'a, T, D>>::IsFlushLeft as Bool>::And<<Self as FoldChainSlice<'a, T, D>>::IsReversed>, <<Self as FoldChainSlice<'a, T, D>>::IsFlushRight as Bool>::And<<<Self as FoldChainSlice<'a, T, D>>::IsReversed as Bool>::Not>, T, <Self as FoldChainSlice<'a, T, D>>::OriginalD, <Self as FoldChainSlice<'a, T, D>>::Settings, <Self as FoldChainSlice<'a, T, D>>::Simplification>, MutFoldChainSliceStruct<'a, <<<Self as FoldChainSlice<'a, T, D>>::IsReversed as Bool>::Not as Bool>::Not, <<Self as FoldChainSlice<'a, T, D>>::IsFlushLeft as Bool>::And<<<Self as FoldChainSlice<'a, T, D>>::IsReversed as Bool>::Not>, <<Self as FoldChainSlice<'a, T, D>>::IsFlushRight as Bool>::And<<<<Self as FoldChainSlice<'a, T, D>>::IsReversed as Bool>::Not as Bool>::Not>, T, <Self as FoldChainSlice<'a, T, D>>::OriginalD, <Self as FoldChainSlice<'a, T, D>>::Settings, <Self as FoldChainSlice<'a, T, D>>::Simplification>> {
        Range::EndIsLeft::init_if_else((self,range.end()), 
        |(this,end)| this.mut_view_take_right_until(end), 
        |(this,end)| this.mut_view_take_left_until(end), 
        )
    }

    /// Get a reversed version of this view. See [Reverse](crate#reverse).
    /// 
    /// This operation's immutable version is [`view_reversed`](FoldChainSlice::view_reversed).
    fn mut_view_reversed(self) -> MutFoldChainSliceStruct<'a,<Self::IsReversed as Bool>::Not,Self::IsFlushLeft,Self::IsFlushRight,T,Self::OriginalD,Self::Settings,Self::Simplification> {
        self.as_mut().mut_view_reversed()
    }

    /// Compose this view's current simplification with another one explicitly. See [Simplification](crate#simplification).
    /// 
    /// This operation's immutable version is [`view_with_simplification`](FoldChainSlice::view_with_simplification).
    fn mut_view_with_simplification<NewSimplification: FoldSimplification<T,D>>(self, new_simplification: NewSimplification) -> MutFoldChainSliceStruct<'a,Self::IsReversed,Self::IsFlushLeft,Self::IsFlushRight,T,Self::OriginalD,Self::Settings,
        NewSimplification::ComposeAfterOther<Self::OriginalD,Self::Simplification>> {
            self.as_mut().mut_view_with_simplification(new_simplification)
        }

    /// Simplify this view. See [Simplification](crate#simplification).
    /// 
    /// This operation's immutable version is [`view_simplify`](FoldChainSlice::view_simplify).
    fn mut_view_simplify<D2: Clone + 'a, 
        Simplifier: for<'x> Fun<&'x D,D2> + Copy + 'a,
        OP2: Fun<(D2,D2),D2> + Copy + 'a>(
            self,simplifier: Simplifier, simplified_op: OP2) -> MutFoldChainSliceStruct<'a,
                Self::IsReversed,Self::IsFlushLeft,Self::IsFlushRight,T,Self::OriginalD,Self::Settings,
                <Self::Simplification as FoldSimplification<T,Self::OriginalD>>::Compose<D2,Simplifier,OP2>> {
                    self.mut_view_with_simplification(SimplificationWithoutShortcut {
                        simplifier,
                        op2: simplified_op,
                        _m: PhantomData,
                    })
                    
    }

    /// Simplify this view in a possibly more efficient way. See [Simplification](crate#simplification).
    /// 
    /// This operation's immutable version is [`view_simplify_with_shortcut`](FoldChainSlice::view_simplify_with_shortcut).
    fn mut_view_simplify_with_shortcut<D2: Clone + 'a, 
        Simplifier: for<'x> Fun<&'x D,D2> + Copy + 'a,
        OP2: Fun<(D2,D2),D2> + Copy + 'a,
        EmptyShortcut: Fun<(),D2> + Copy + 'a,
        DeltaShortcut: for<'x> Fun<&'x T, D2> + Copy + 'a>(
            self,simplifier: Simplifier, simplified_op: OP2, empty_shortcut: EmptyShortcut, delta_shortcut: DeltaShortcut)
                -> MutFoldChainSliceStruct<'a,Self::IsReversed,Self::IsFlushLeft,Self::IsFlushRight,T,Self::OriginalD,Self::Settings,
                    <Self::Simplification as FoldSimplification<T,Self::OriginalD>>::ComposeWithShortcut<D2,Simplifier,OP2,EmptyShortcut,DeltaShortcut>> {
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
    /// The [current simplification](FoldChainSlice::get_current_simplification) of the resulting view will be `()`.
    /// 
    /// This operation's immutable version is [`view_unsimplify`](FoldChainSlice::view_unsimplify).
    fn mut_view_unsimplify(self) -> MutFoldChainSliceStruct<'a,Self::IsReversed,Self::IsFlushLeft,Self::IsFlushRight,T,Self::OriginalD,Self::Settings,()> {
        self.as_mut().mut_view_unsimplify()
    }

    /// Remove the leftmost element from this slice.
    /// 
    /// Returns the removed element, or [`None`], if this slice was already empty.
    fn pop_left(&mut self) -> Option<T> {
        self.borrow_mut().pop_left()
    }

    /// Remove the rightmost element from this slice.
    /// 
    /// Returns the removed element, or [`None`], if this slice was already empty.
    fn pop_right(&mut self) -> Option<T> {
        self.borrow_mut().mut_view_reversed().pop_left()
    }

    /// Append `value` to the left of this slice.
    fn append_left(&mut self, value: T) {
        self.borrow_mut().append_left(value);
    }

    /// Append `value` to the right of this slice.
    fn append_right(&mut self, value: T) {
        self.borrow_mut().mut_view_reversed().append_left(value);
    }

    /// Replace the leftmost element of this slice with `value`.
    /// 
    /// If this slice is empty, this does nothing and returns `Err(value)`.
    /// 
    /// Otherwise, returns `Ok(prev)`, where `prev` is the previous leftmost element.
    /// 
    /// For the unwrapping version, see [`set_left`](MutFoldChainSlice::set_left).
    fn set_left_or_err(&mut self, value: T) -> Result<T,T> {
        self.borrow_mut().set_left_or_err(value)
    }

    /// Replace the rightmost element of this slice with `value`.
    /// 
    /// If this slice is empty, this does nothing and returns `Err(value)`.
    /// 
    /// Otherwise, returns `Ok(prev)`, where `prev` is the previous rightmost element.
    /// 
    /// For the unwrapping version, see [`set_right`](MutFoldChainSlice::set_right).
    fn set_right_or_err(&mut self, value: T) -> Result<T,T> {
        self.borrow_mut().mut_view_reversed().set_left_or_err(value)
    }

    /// Replace the leftmost element of this slice with `value` and return its previous value.
    /// 
    /// Panics if this slice is empty.
    /// 
    /// For the non-panicing version, see [`set_left_or_err`](MutFoldChainSlice::set_left_or_err).
    fn set_left(&mut self, value: T) -> T {
        self.set_left_or_err(value).unwrap_or_else(|_| panic!("The chain should not be empty"))
    }

    /// Replace the rightmost element of this slice with `value` and return its previous value.
    /// 
    /// Panics if this slice is empty.
    /// 
    /// For the non-panicing version, see [`set_right_or_err`](MutFoldChainSlice::set_right_or_err).
    fn set_right(&mut self, value: T) -> T {
        self.set_right_or_err(value).unwrap_or_else(|_| panic!("The chain should not be empty"))
    }

    /// Mutate the leftmost element of this slice via a closure, and return the result of the closure.
    /// 
    /// If this slice is empty, the input of the closure will be [`None`].
    fn update_left<R>(&mut self, f: impl FnOnce(Option<&mut T>) -> R) -> R {
        self.borrow_mut().update_left(f)
    }

    /// Mutate the rightmost element of this slice via a closure, and return the result of the closure.
    /// 
    /// If this slice is empty, the input of the closure will be [`None`].
    fn update_right<R>(&mut self, f: impl FnOnce(Option<&mut T>) -> R) -> R {
        self.borrow_mut().mut_view_reversed().update_left(f)
    }

    /// Run a closure on each of this slice's elements, possibly mutating them, from left to right.
    /// 
    /// This operation's immutable version is [`foreach`](FoldChainSlice::foreach).
    /// 
    /// Note that this operation cannot be safely done with an [`Iterator`], because that would allow direct access to mutable references to this data structure's elements,
    /// in a way that might outlive the iterator (so the iterator couldn't reliably do the proper cleanup/bookkeeping on [`Drop`]). 
    /// An "iterator" capable of this would need to be a [streaming iterator](https://docs.rs/streaming-iterator/latest/streaming_iterator/) instead of an [`Iterator`].
    /// 
    /// An iterator can iterate over the elements themselves (rather than references), though; see [`drain`](MutFoldChainSlice::drain) or [`FoldChain`]'s [IntoIterator implementation](FoldChain#impl-IntoIterator-for-FoldChain<T,+D,+Settings>).
    fn foreach_mut(&mut self, f: impl FnMut(&mut T)) {
        self.borrow_mut().foreach_mut(f);
    }

    /// Remove all elements from this slice, and return a new [`FoldChain`] containing them, in the same order as in this slice's base [`FoldChain`].
    /// 
    /// The returned `FoldChain` will have the same type as this slice's base.
    fn take_all(&mut self) -> FoldChain<T,Self::OriginalD,Self::Settings> {
        self.borrow_mut().take_all()
    }

    /// Convert this slice into an [`Iterator`] that removes elements from the left as it emits them (or from the right with [`DoubleEndedIterator::next_back`]).
    /// 
    /// Note that not all elements in this slice will be removed; only those that the iterator emits.
    fn drain(self) -> Drain<'a,T,D,Self> {
        Drain { slice: self, _m: PhantomData }
    }

    /// Append the entire contents of a [`FoldChain`] to the right of this slice.
    /// 
    /// From this slice's base's perspective, the elements' order will be the same as in `chain`.
    /// 
    /// Note that `chain`'s type must be the same as this slice's base.
    fn append_all_right(&mut self, chain: FoldChain<T,Self::OriginalD,Self::Settings>) {
        self.borrow_mut().append_all_right(chain);
    }

    /// Append the entire contents of a [`FoldChain`] to the left of this slice.
    /// 
    /// From this slice's base's perspective, the elements' order will be the same as in `chain`.
    /// 
    /// Note that `chain`'s type must be the same as this slice's base.
    fn append_all_left(&mut self, chain: FoldChain<T,Self::OriginalD,Self::Settings>) {
        self.borrow_mut().mut_view_reversed().append_all_right(chain);
    }

    /// Append every element from an iterator to the left of this slice.
    /// 
    /// This is faster than [`append_left`](MutFoldChainSlice::append_left)ing them one-by-one, but not asymptotically faster.
    fn append_left_from_iter(&mut self, iter: impl Iterator<Item=T>) {
        let settings = self.get_settings();
        let to_add = WAVLNode::new_from_iterator_left_to_right_template::<<Self::IsReversed as Bool>::Not>(iter, settings);
        self.append_all_left(FoldChain {
            leftmost_node_ptr: to_add.as_ref().map(|t| t.all_the_way_left_template::<False>().into()),
            rightmost_node_ptr: to_add.as_ref().map(|t| t.all_the_way_left_template::<True>().into()),
            root: to_add,
            settings: settings,
        });
    }

    /// Append every element from an iterator to the right of this slice.
    /// 
    /// This is faster than [`append_right`](MutFoldChainSlice::append_right)ing them one-by-one, but not asymptotically faster.
    fn append_right_from_iter(&mut self, iter: impl Iterator<Item=T>) {
        let settings = self.get_settings();
        let to_add = WAVLNode::new_from_iterator_left_to_right_template::<Self::IsReversed>(iter, settings);
        self.append_all_right(FoldChain {
            leftmost_node_ptr: to_add.as_ref().map(|t| t.all_the_way_left_template::<False>().into()),
            rightmost_node_ptr: to_add.as_ref().map(|t| t.all_the_way_left_template::<True>().into()),
            root: to_add,
            settings: settings,
        });
    }
}

/// A base `FoldChain`. See [FoldChain](crate#foldchain).
pub struct FoldChain<T,D: Clone, Settings: FoldSettings<T,D>> {
    pub(crate) root: Option<Box<WAVLNode<T,D>>>,
    pub(crate) leftmost_node_ptr: Option<NonNull<WAVLNode<T,D>>>,
    pub(crate) rightmost_node_ptr: Option<NonNull<WAVLNode<T,D>>>,
    //the above pointers should never be none unless root is none
    pub(crate) settings: Settings
}


impl<T, D: Clone, Settings: FoldSettings<T,D>> FoldChain<T, (usize,D), SettingsWithSize<Settings>> {
    /// Convert a  `FoldChain` that keeps track of size to a [`FoldList`].
    pub fn as_fold_list(self) -> FoldList<T, D, Settings> {
        FoldList{ underlying: self }
    }
}

impl<T,D: Clone, OP: Fn(D,D)->D + Copy, DeltaOf: Fn(&T)->D + Copy, Empty: Fn()->D + Copy> FoldChain<T,D,FoldSettingsStruct<T,D,TupleFun<OP>,DeltaOf,EmptyFn<Empty>>> {
    /// Create a new empty `FoldChain`, given the closures for [Settings](crate#fold-settings).
    pub fn new(op: OP, delta_of: DeltaOf, empty_delta_fn: Empty) -> Self {
        FoldChain::from_settings(FoldSettingsStruct {
            op_closure: TupleFun(op),
            t2d_closure: delta_of,
            empty_closure: EmptyFn(empty_delta_fn),
            _m: PhantomData,
        })
    }

    /// Create a new FoldChain, given the closures for [Settings](crate#fold-settings), and fill it using an iterator, from left to right.
    pub fn from_iter(op: OP, delta_of: DeltaOf, empty_delta_fn: Empty, iter: impl Iterator<Item=T>) -> Self {
        let mut ret = Self::new(op,delta_of,empty_delta_fn);
        ret.append_right_from_iter(iter);
        ret
    }
}

impl<T,D: Clone, OP: Fn(D,D)->D + Copy, DeltaOf: Fn(&T) -> D + Copy> FoldChain<T,Option<D>,FoldSettingsStruct<T,Option<D>,OptOpFun<TupleFun<OP>>,SomeFun<DeltaOf>,NoneFun>> {
    /// Create a new empty `FoldChain`, given the closures for [Settings](crate#fold-settings), except the one for the empty delta, which will always be [`None`].
    /// 
    /// The resulting delta type will be `Option<D>` instead of `D`.
    pub fn new_with_opt(op: OP, delta_of: DeltaOf) -> Self {
        FoldChain::from_settings(FoldSettingsStruct { 
            op_closure: OptOpFun(TupleFun(op)),
            t2d_closure: SomeFun(delta_of),
            empty_closure: NoneFun, 
            _m: PhantomData
        })
    }

    /// Create a new empty `FoldChain`, given the closures for [Settings](crate#fold-settings), except the one for the empty delta, which will always be [`None`], and fill it using an iterator, from left to right.
    /// 
    /// The resulting delta type will be `Option<D>` instead of `D`.
    pub fn new_with_opt_from_iter(op: OP, delta_of: DeltaOf, iter: impl Iterator<Item=T>) -> Self {
        let mut ret = Self::new_with_opt(op, delta_of);
        ret.append_right_from_iter(iter);
        ret
    }
}

impl<T, D: Clone, Settings: FoldSettings<T,D>> FoldChain<T, D, Settings> {
    /// Create a new empty `FoldChain`, with the specified [Settings](crate#fold-settings).
    pub fn from_settings(settings: Settings) -> Self {
        Self {
            root: None,
            leftmost_node_ptr: None,
            rightmost_node_ptr: None,
            settings,
        }
    }

    fn into_imm_template<'a>(&'a self) -> ImmFoldChainSliceStruct<'a,False,True,True,Settings,(),T,D> {
        ImmFoldChainSliceStruct{ 
            endpoints: self.root.as_ref().map(|r|
                ImmSliceEndpoints{
                    root: r.as_ref().into(),
                    left: self.leftmost_node_ptr.unwrap(), 
                    right: self.rightmost_node_ptr.unwrap()
                }
            ), 
            settings: self.settings, 
            simplification: (), 
            _m: PhantomData 
        }
    }

    fn leftmost_node_ptr_mut_template<Reversed: Bool>(&mut self) -> &mut Option<NonNull<WAVLNode<T,D>>> {
        if Reversed::b {
            &mut self.rightmost_node_ptr
        } else {
            &mut self.leftmost_node_ptr
        }
    }

    fn debug_check_structural_integrity_orig(&self) -> bool {
        if let Some(root) = &self.root {
            debug_assert!(root.debug_check_structural_integrity());
            debug_assert!(root.parent_ptr.is_none());
            let Some(left_ptr) = self.leftmost_node_ptr else {panic!()};
            let Some(right_ptr) = self.rightmost_node_ptr else {panic!()};
            let leftward_ptr = root.all_the_way_left_template::<False>().into();
            debug_assert_eq!(left_ptr,leftward_ptr);
            let rightward_ptr = root.all_the_way_left_template::<True>().into();
            debug_assert_eq!(right_ptr,rightward_ptr);
        } else {
            debug_assert_eq!(self.leftmost_node_ptr,None);
            debug_assert_eq!(self.rightmost_node_ptr,None);
        }
        true
    }

    //below are redefinitions of the functions for FoldChainSlice and MutFoldChainSlice, to enable the use of e.g. chain.f() instead of needing to write (&mut chain).f()

    /// An alias of [`get_current_simplification`](FoldChainSlice::get_current_simplification).
    pub fn get_current_simplification(&self) -> (){}
    /// An alias of [`get_settings`](FoldChainSlice::get_settings).
    pub fn get_settings(&self) -> Settings {self.settings}
    /// An alias of [`as_imm`](FoldChainSlice::as_imm).
    pub fn as_imm(&self) -> ImmFoldChainSliceStruct<'_, False,True,True,Settings,(),T,D> {
        FoldChainSlice::as_imm(self)
    }
    /// An alias of [`borrow`](FoldChainSlice::borrow).
    pub fn borrow(&self) -> ImmFoldChainSliceStruct<'_, False,True,True,Settings,(),T,D> {
        FoldChainSlice::as_imm(self)
    }
    /// An alias of [`view_drop_left_until`](FoldChainSlice::view_drop_left_until).
    pub fn view_drop_left_until(&self, predicate: impl Fn(&D)->bool) -> ImmFoldChainSliceStruct<'_, False, False, True, Settings, (), T, D> {
        FoldChainSlice::view_drop_left_until(self, predicate)
    }
    /// An alias of [`view_take_right_until`](FoldChainSlice::view_take_right_until).
    pub fn view_take_right_until(&self, predicate: impl Fn(&D)->bool) -> ImmFoldChainSliceStruct<'_, False, False, True, Settings, (), T, D> {
        FoldChainSlice::view_take_right_until(self, predicate)
    }
    /// An alias of [`view_drop_right_until`](FoldChainSlice::view_drop_right_until).
    pub fn view_drop_right_until(&self, predicate: impl Fn(&D)->bool) -> ImmFoldChainSliceStruct<'_, False, True, False, Settings, (), T, D> {
        FoldChainSlice::view_drop_right_until(self,predicate)
    }
    /// An alias of [`view_take_left_until`](FoldChainSlice::view_take_left_until).
    pub fn view_take_left_until(&self, predicate: impl Fn(&D)->bool) -> ImmFoldChainSliceStruct<'_, False, True, False, Settings, (), T, D> {
        FoldChainSlice::view_take_left_until(self,predicate)
    }
    /// An alias of [`view_drop`](FoldChainSlice::view_drop).
    pub fn view_drop<Predicate: Fn(&D)->bool, Range: SingleEndedRange<Predicate>>(&self, range: Range) -> <<Range as SingleEndedRange<Predicate>>::EndIsLeft as Bool>::IfElse<ImmFoldChainSliceStruct<'_, False, True, False, Settings, (), T, D>, ImmFoldChainSliceStruct<'_, False, False, True, Settings, (), T, D>> {
        FoldChainSlice::view_drop(self,range)
    }
    /// An alias of [`view_take`](FoldChainSlice::view_take).
    pub fn view_take<Predicate: Fn(&D)->bool, Range: SingleEndedRange<Predicate>>(&self, range: Range) -> <<Range as SingleEndedRange<Predicate>>::EndIsLeft as Bool>::IfElse<ImmFoldChainSliceStruct<'_, False, False, True, Settings, (), T, D>, ImmFoldChainSliceStruct<'_, False, True, False, Settings, (), T, D>> {
        FoldChainSlice::view_take(self,range)
    }
    /// An alias of [`view_reversed`](FoldChainSlice::view_reversed).
    pub fn view_reversed(&self) -> ImmFoldChainSliceStruct<'_, True, True, True, Settings, (), T, D> {
        FoldChainSlice::view_reversed(self)
    }
    /// An alias of [`view_with_simplification`](FoldChainSlice::view_with_simplification).
    pub fn view_with_simplification<NewSimplification: FoldSimplification<T,D>>(&self, new_simplification: NewSimplification) -> ImmFoldChainSliceStruct<'_, False, True, True, Settings, <NewSimplification as FoldSimplification<T, D>>::ComposeAfterOther<D, ()>, T, D> {
        FoldChainSlice::view_with_simplification(self,new_simplification)
    }
    /// An alias of [`view_simplify`](FoldChainSlice::view_simplify).
    pub fn view_simplify<'a, D2: Clone + 'a, Simplifier: for<'x> Fun<&'x D,D2> + Copy + 'a,OP2: Fun<(D2,D2),D2> + Copy + 'a>(&'a self,simplifier: Simplifier, simplified_op: OP2) -> ImmFoldChainSliceStruct<'a, False, True, True, Settings, SimplificationWithoutShortcut<T, D, D2, Simplifier, OP2>, T, D> {
        FoldChainSlice::view_simplify(self, simplifier, simplified_op)
    }
    /// An alias of [`view_simplify_with_shortcut`](FoldChainSlice::view_simplify_with_shortcut).
    pub fn view_simplify_with_shortcut<'a, D2: Clone + 'a, Simplifier: for<'x> Fun<&'x D,D2> + Copy + 'a, OP2: Fun<(D2,D2),D2> + Copy + 'a, EmptyShortcut: Fun<(),D2> + Copy + 'a, DeltaShortcut: for<'x> Fun<&'x T, D2> + Copy + 'a>(&'a self,simplifier: Simplifier, simplified_op: OP2, empty_shortcut: EmptyShortcut, delta_shortcut: DeltaShortcut) -> ImmFoldChainSliceStruct<'a, False, True, True, Settings, SimplificationWithShortcut<T, D, D2, Simplifier, OP2, EmptyShortcut, DeltaShortcut>, T, D> {
        FoldChainSlice::view_simplify_with_shortcut(self, simplifier, simplified_op, empty_shortcut, delta_shortcut)
    }
    /// An alias of [`view_unsimplify`](FoldChainSlice::view_unsimplify).
    pub fn view_unsimplify(&self) -> ImmFoldChainSliceStruct<'_, False, True, True, Settings, (), T, D> {
        FoldChainSlice::view_unsimplify(self)
    }
    /// An alias of [`fold`](FoldChainSlice::fold).
    pub fn fold(&self) -> D {
        FoldChainSlice::fold(&self)
    }
    /// An alias of [`is_empty`](FoldChainSlice::is_empty).
    pub fn is_empty(&self) -> bool {
        FoldChainSlice::is_empty(&self)
    }
    /// An alias of [`left`](FoldChainSlice::left).
    pub fn left<'b>(&'b self) -> Option<&'b T> {
        self.borrow().left_consume()
    }
    /// An alias of [`right`](FoldChainSlice::right).
    pub fn right<'b>(&'b self) -> Option<&'b T> {
        self.borrow().view_reversed().left_consume()
    }
    /// An alias of [`foreach`](FoldChainSlice::foreach).
    pub fn foreach(&self, f: impl FnMut(&T)) {
        self.borrow().foreach(f);
    }
    /// An alias of [`iter`](FoldChainSlice::iter).
    pub fn iter<'b>(&'b self) -> Iter<'b, False, T, D> {
        self.borrow().iter_consume()
    }

    /// An alias of [`as_mut`](MutFoldChainSlice::as_mut).
    pub fn as_mut(&mut self) -> MutFoldChainSliceStruct<'_, False, True, True, T, D, Settings, ()> {
        MutFoldChainSlice::as_mut(self)
    }
    /// An alias of [`borrow_mut`](MutFoldChainSlice::borrow_mut).
    pub fn borrow_mut(&mut self) -> MutFoldChainSliceStruct<'_, False, True, True, T, D, Settings, ()> {
        MutFoldChainSlice::as_mut(self)
    }
    /// An alias of [`mut_view_drop_left_until`](MutFoldChainSlice::mut_view_drop_left_until).
    pub fn mut_view_drop_left_until(&mut self, predicate: impl Fn(&D)->bool) -> MutFoldChainSliceStruct<'_, False, False, True, T, D, Settings, ()> {
        MutFoldChainSlice::mut_view_drop_left_until(self, predicate)
    }
    /// An alias of [`mut_view_take_right_until`](MutFoldChainSlice::mut_view_take_right_until).
    pub fn mut_view_take_right_until(&mut self, predicate: impl Fn(&D)->bool) -> MutFoldChainSliceStruct<'_, False, False, True, T, D, Settings, ()> {
        MutFoldChainSlice::mut_view_take_right_until(self, predicate)
    }
    /// An alias of [`mut_view_drop_right_until`](MutFoldChainSlice::mut_view_drop_right_until).
    pub fn mut_view_drop_right_until(&mut self, predicate: impl Fn(&D)->bool) -> MutFoldChainSliceStruct<'_, False, True, False, T, D, Settings, ()> {
        MutFoldChainSlice::mut_view_drop_right_until(self, predicate)
    }
    /// An alias of [`mut_view_take_left_until`](MutFoldChainSlice::mut_view_take_left_until).
    pub fn mut_view_take_left_until(&mut self, predicate: impl Fn(&D)->bool) -> MutFoldChainSliceStruct<'_, False, True, False, T, D, Settings, ()> {
        MutFoldChainSlice::mut_view_take_left_until(self, predicate)
    }
    /// An alias of [`mut_view_drop`](MutFoldChainSlice::mut_view_drop).
    pub fn mut_view_drop<Predicate: Fn(&D)->bool, Range: SingleEndedRange<Predicate>>(&mut self, range: Range) -> <<Range as SingleEndedRange<Predicate>>::EndIsLeft as Bool>::IfElse<MutFoldChainSliceStruct<'_, False, True, False, T, D, Settings, ()>, MutFoldChainSliceStruct<'_, False, False, True, T, D, Settings, ()>> {
        MutFoldChainSlice::mut_view_drop(self, range)
    }
    /// An alias of [`mut_view_take`](MutFoldChainSlice::mut_view_take).
    pub fn mut_view_take<Predicate: Fn(&D)->bool, Range: SingleEndedRange<Predicate>>(&mut self, range: Range) -> <<Range as SingleEndedRange<Predicate>>::EndIsLeft as Bool>::IfElse<MutFoldChainSliceStruct<'_, False, False, True, T, D, Settings, ()>, MutFoldChainSliceStruct<'_, False, True, False, T, D, Settings, ()>> {
        MutFoldChainSlice::mut_view_take(self, range)
    }
    /// An alias of [`mut_view_reversed`](MutFoldChainSlice::mut_view_reversed).
    pub fn mut_view_reversed(&mut self) -> MutFoldChainSliceStruct<'_, True, True, True, T, D, Settings, ()> {
        MutFoldChainSlice::mut_view_reversed(self)
    }
    /// An alias of [`mut_view_with_simplification`](MutFoldChainSlice::mut_view_with_simplification).
    pub fn mut_view_with_simplification<NewSimplification: FoldSimplification<T,D>>(&mut self, new_simplification: NewSimplification) -> MutFoldChainSliceStruct<'_, False, True, True, T, D, Settings, <NewSimplification as FoldSimplification<T, D>>::ComposeAfterOther<D, ()>> {
        MutFoldChainSlice::mut_view_with_simplification(self, new_simplification)
    }
    /// An alias of [`mut_view_simplify`](MutFoldChainSlice::mut_view_simplify).
    pub fn mut_view_simplify<'a, D2: Clone + 'a, Simplifier: for<'x> Fun<&'x D,D2> + Copy + 'a, OP2: Fun<(D2,D2),D2> + Copy + 'a>(&'a mut self,simplifier: Simplifier, simplified_op: OP2) -> MutFoldChainSliceStruct<'a, False, True, True, T, D, Settings, SimplificationWithoutShortcut<T, D, D2, Simplifier, OP2>> {
        MutFoldChainSlice::mut_view_simplify(self, simplifier, simplified_op)
    }
    /// An alias of [`mut_view_simplify_with_shortcut`](MutFoldChainSlice::mut_view_simplify_with_shortcut).
    pub fn mut_view_simplify_with_shortcut<'a, D2: Clone + 'a, Simplifier: for<'x> Fun<&'x D,D2> + Copy + 'a, OP2: Fun<(D2,D2),D2> + Copy + 'a, EmptyShortcut: Fun<(),D2> + Copy + 'a, DeltaShortcut: for<'x> Fun<&'x T, D2> + Copy + 'a>(&'a mut self,simplifier: Simplifier, simplified_op: OP2, empty_shortcut: EmptyShortcut, delta_shortcut: DeltaShortcut) -> MutFoldChainSliceStruct<'a, False, True, True, T, D, Settings, SimplificationWithShortcut<T, D, D2, Simplifier, OP2, EmptyShortcut, DeltaShortcut>>{
        MutFoldChainSlice::mut_view_simplify_with_shortcut(self, simplifier, simplified_op, empty_shortcut, delta_shortcut)
    }
    /// An alias of [`mut_view_unsimplify`](MutFoldChainSlice::mut_view_unsimplify).
    pub fn mut_view_unsimplify(&mut self) -> MutFoldChainSliceStruct<'_, False, True, True, T, D, Settings, ()> {
        MutFoldChainSlice::mut_view_unsimplify(self)
    }
    /// An alias of [`pop_left`](MutFoldChainSlice::pop_left).
    pub fn pop_left(&mut self) -> Option<T> {
        self.borrow_mut().pop_left()
    }
    /// An alias of [`pop_right`](MutFoldChainSlice::pop_right).
    pub fn pop_right(&mut self) -> Option<T> {
        self.borrow_mut().mut_view_reversed().pop_left()
    }
    /// An alias of [`append_left`](MutFoldChainSlice::append_left).
    pub fn append_left(&mut self, value: T) {
        self.borrow_mut().append_left(value);
    }
    /// An alias of [`append_right`](MutFoldChainSlice::append_right).
    pub fn append_right(&mut self, value: T) {
        self.borrow_mut().mut_view_reversed().append_left(value);
    }
    /// An alias of [`set_left_or_err`](MutFoldChainSlice::set_left_or_err).
    pub fn set_left_or_err(&mut self, value: T) -> Result<T,T> {
        self.borrow_mut().set_left_or_err(value)
    }
    /// An alias of [`set_right_or_err`](MutFoldChainSlice::set_right_or_err).
    pub fn set_right_or_err(&mut self, value: T) -> Result<T,T> {
        self.borrow_mut().mut_view_reversed().set_left_or_err(value)
    }
    /// An alias of [`set_left`](MutFoldChainSlice::set_left).
    pub fn set_left(&mut self, value: T) -> T {
        self.set_left_or_err(value).unwrap_or_else(|_| panic!("The chain should not be empty"))
    }
    /// An alias of [`set_right`](MutFoldChainSlice::set_right).
    pub fn set_right(&mut self, value: T) -> T {
        self.set_right_or_err(value).unwrap_or_else(|_| panic!("The chain should not be empty"))
    }
    /// An alias of [`update_left`](MutFoldChainSlice::update_left).
    pub fn update_left<R>(&mut self, f: impl FnOnce(Option<&mut T>) -> R) -> R {
        self.borrow_mut().update_left(f)
    }
    /// An alias of [`update_right`](MutFoldChainSlice::update_right).
    pub fn update_right<R>(&mut self, f: impl FnOnce(Option<&mut T>) -> R) -> R {
        self.borrow_mut().mut_view_reversed().update_left(f)
    }
    /// An alias of [`foreach_mut`](MutFoldChainSlice::foreach_mut).
    pub fn foreach_mut(&mut self, f: impl FnMut(&mut T)) {
        self.borrow_mut().foreach_mut(f);
    }
    /// An alias of [`take_all`](MutFoldChainSlice::take_all).
    pub fn take_all(&mut self) -> FoldChain<T, D, Settings> {
        self.borrow_mut().take_all()
    }
    /// An alias of [`append_all_right`](MutFoldChainSlice::append_all_right).
    pub fn append_all_right(&mut self, chain: FoldChain<T,D,Settings>) {
        self.borrow_mut().append_all_right(chain);
    }
    /// An alias of [`append_all_left`](MutFoldChainSlice::append_all_left).
    pub fn append_all_left(&mut self, chain: FoldChain<T,D,Settings>) {
        self.borrow_mut().mut_view_reversed().append_all_right(chain);
    }
    /// An alias of [`append_left_from_iter`](MutFoldChainSlice::append_left_from_iter).
    pub fn append_left_from_iter(&mut self, iter: impl Iterator<Item=T>) {
        MutFoldChainSlice::append_left_from_iter(&mut &mut *self, iter);
    }
    /// An alias of [`append_right_from_iter`](MutFoldChainSlice::append_right_from_iter).
    pub fn append_right_from_iter(&mut self, iter: impl Iterator<Item=T>) {
        MutFoldChainSlice::append_right_from_iter(&mut &mut *self, iter);
    }
}



impl<'a,T, D: Clone, Settings: FoldSettings<T,D>> FoldChainSlice<'a,T,D> for &'a FoldChain<T, D, Settings> {
    type OriginalD = D;
    type IsReversed = False;
    type IsFlushLeft = True;
    type IsFlushRight = True;
    type Simplification = ();
    type Settings = Settings;

    fn get_settings(&self) -> Self::Settings {
        self.settings
    }

    fn get_current_simplification(&self) -> Self::Simplification {}

    fn as_imm(self) -> ImmFoldChainSliceStruct<'a,Self::IsReversed,Self::IsFlushLeft,Self::IsFlushRight,Self::Settings,Self::Simplification,T,Self::OriginalD> {
        self.into_imm_template()
    }

    fn borrow<'b>(&'b self) -> ImmFoldChainSliceStruct<'b,Self::IsReversed,Self::IsFlushLeft,Self::IsFlushRight,Self::Settings,Self::Simplification,T,Self::OriginalD> {
        self.into_imm_template()
    }

    fn debug_check_structural_integrity(&self) -> bool {
        self.debug_check_structural_integrity_orig()
    }
}

impl<'a,T, D: Clone, Settings: FoldSettings<T,D>> FoldChainSlice<'a,T,D> for &'a mut FoldChain<T, D, Settings> {
    type OriginalD = D;
    type IsReversed = False;
    type IsFlushLeft = True;
    type IsFlushRight = True;
    type Simplification = ();
    type Settings = Settings;

    fn get_settings(&self) -> Self::Settings {
        self.settings
    }

    fn get_current_simplification(&self) -> Self::Simplification {}

    fn as_imm(self) -> ImmFoldChainSliceStruct<'a,Self::IsReversed,Self::IsFlushLeft,Self::IsFlushRight,Self::Settings,Self::Simplification,T,Self::OriginalD> {
        self.into_imm_template()
    }

    fn borrow<'b>(&'b self) -> ImmFoldChainSliceStruct<'b,Self::IsReversed,Self::IsFlushLeft,Self::IsFlushRight,Self::Settings,Self::Simplification,T,Self::OriginalD> {
        self.into_imm_template()
    }

    fn debug_check_structural_integrity(&self) -> bool {
        self.debug_check_structural_integrity_orig()
    }
}

impl<'a,T, D: Clone, Settings: FoldSettings<T,D>> MutFoldChainSlice<'a,T,D> for &'a mut FoldChain<T, D, Settings> {
    fn as_mut(self) -> MutFoldChainSliceStruct<'a,Self::IsReversed,Self::IsFlushLeft,Self::IsFlushRight,T,Self::OriginalD,Self::Settings,Self::Simplification> {
        MutFoldChainSliceStruct::new_from(self)
    }

    fn borrow_mut<'b>(&'b mut self) -> MutFoldChainSliceStruct<'b,Self::IsReversed,Self::IsFlushLeft,Self::IsFlushRight,T,Self::OriginalD,Self::Settings,Self::Simplification> {
        MutFoldChainSliceStruct::new_from(self)
    }
}

fn endpoints_get_fold<IsFlushLeft: Bool, IsFlushRight: Bool, Settings: FoldSettings<T,D>, Simp: FoldSimplification<T,D>, T, D: Clone>(
    left: IsFlushLeft::IfElse<(),NonNull<WAVLNode<T,D>>>, 
    right: IsFlushRight::IfElse<(),NonNull<WAVLNode<T,D>>>,
    root_if_both_flush: <IsFlushLeft::And<IsFlushRight> as Bool>::IfElse<NonNull<WAVLNode<T,D>>,()>,
    settings: Settings,
    simp: Simp
) -> Simp::D2 {
    unsafe {
        if IsFlushLeft::b && !IsFlushRight::b {
            //in a normal traversal this would require the stack and be slower, so let's fold in reverse order in this case
            traverse_left_to_right_template::<True,False,True,_,_,_,()>(IsFlushRight::assert_false_unwrap(right), (), (),simp.empty(settings), 
            |is_single: bool,acc: Simp::D2,node: &mut WAVLNode<T, D>| {
                if is_single {
                    Ok(simp.op(simp.delta_of(&node.value,settings),acc, settings))
                } else {
                    Ok(simp.op(simp.simplify(&node.delta_whole),acc,settings))
                }
            }).unwrap_unchecked()
        } else {
            traverse_left_to_right_template::<False,IsFlushLeft,IsFlushRight,_,_,_,()>(left, right, root_if_both_flush,simp.empty(settings), 
            |is_single: bool,acc: Simp::D2,node: &mut WAVLNode<T, D>| {
                if is_single {
                    Ok(simp.op(acc,simp.delta_of(&node.value,settings), settings))
                } else {
                    Ok(simp.op(acc,simp.simplify(&node.delta_whole),settings))
                }
            }).unwrap_unchecked()
        }

    }
}

// reminder: 
// if (left,right) is...
//      (None,None) => slice is empty because base is empty
//      (None,Some) => slice is empty to the right of right
//      (Some,None) => slice is empty to the left of left
//      (Some,Some) => slice is everything between left and right, inclusive for both
// assume at least one of left and right is Some
unsafe fn endpoints_push_left_template<Reversed: Bool, Settings: FoldSettings<T,D>, T, D: Clone>(
    left_opt_mut: &mut Option<NonNull<WAVLNode<T,D>>>,
    right_opt_mut: &mut Option<NonNull<WAVLNode<T,D>>>,
    root: NonNull<Option<Box<WAVLNode<T,D>>>>,
    settings: Settings,
    value: T
) {
    unsafe {
        if left_opt_mut.is_some() {
            let new = WAVLNode::new_leaf(settings,value);
            let new_ptr = NonNull::from(new.as_ref().unwrap().as_ref());
            let left_mut = left_opt_mut.as_mut().unwrap();
            WAVLNode::mutate_box_of_and_update_parents(*left_mut, root, settings, |left| {
                WAVLNode::append_tree_right_of_top_template::<Reversed::Not>(left, new, settings);
            });
            *left_opt_mut = Some(new_ptr);
            if right_opt_mut.is_none() {
                *right_opt_mut = Some(new_ptr);
            }
        } else {
            if right_opt_mut.is_some() {
                endpoints_push_left_template::<Reversed::Not,_,_,_>(right_opt_mut, left_opt_mut, root,settings, value)
            } else {
                unreachable!()
            }
        }
    }
}

unsafe fn endpoints_append_all_right_template<AppendLeft: Bool, Settings: FoldSettings<T,D>, T, D: Clone>(
    left_opt_mut: &mut Option<NonNull<WAVLNode<T,D>>>,
    right_opt_mut: &mut Option<NonNull<WAVLNode<T,D>>>,
    root: NonNull<Option<Box<WAVLNode<T,D>>>>,
    settings: Settings,
    other: FoldChain<T,D,Settings>
) {
    unsafe {
        let Some(other_root) = other.root else {return};
        let left = *left_opt_mut;
        let right = *right_opt_mut;
        match (left,right) {
            (None, None) => unreachable!(),
            (None, Some(r)) => {
                //slice is empty to the right of right
                *left_opt_mut = other.leftmost_node_ptr;
                *right_opt_mut = other.rightmost_node_ptr;
                WAVLNode::mutate_box_of_and_update_parents(r, root,settings,|r| {
                    WAVLNode::append_tree_right_of_top(r, Some(other_root), other.settings);
                });
            },
            (Some(l), None) => {
                *left_opt_mut = other.leftmost_node_ptr;
                *right_opt_mut = other.rightmost_node_ptr;
                WAVLNode::mutate_box_of_and_update_parents(l, root,settings,|l| {
                    WAVLNode::append_tree_left_of_top(l, Some(other_root), other.settings);
                });
            },
            (Some(l), Some(r)) => {
                if AppendLeft::b {
                    *left_opt_mut = other.leftmost_node_ptr;
                    WAVLNode::mutate_box_of_and_update_parents(l, root,settings,|l| {
                        WAVLNode::append_tree_left_of_top(l, Some(other_root), other.settings);
                    })
                } else {
                    *right_opt_mut = other.rightmost_node_ptr;
                    WAVLNode::mutate_box_of_and_update_parents(r, root,settings,|r| {
                        WAVLNode::append_tree_right_of_top(r, Some(other_root), other.settings);
                    })
                }
            },
        }
    }
}

unsafe fn endpoints_foreach_template<Reversed: Bool, T, D: Clone>(
    mut left: NonNull<WAVLNode<T,D>>,
    right: NonNull<WAVLNode<T,D>>,
    mut f: impl FnMut(&T)
){
    unsafe {
        loop {
            f(&left.as_ref().value);
            if left == right {return}
            if let Some(next_left) = WAVLNode::next_single_left_to_right_template::<Reversed>(left.as_ref()) {
                // endpoints_foreach_template::<Reversed,_,_>(next_left, right, f);
                left = next_left;
            } else {break}
        }
    }
}

unsafe fn endpoints_foreach_mut_template<Reversed: Bool, Settings: FoldSettings<T,D>, T, D: Clone>(
    mut left: NonNull<WAVLNode<T,D>>,
    right: NonNull<WAVLNode<T,D>>,
    settings: Settings,
    mut f: impl FnMut(&mut T)
) {
    unsafe {
        loop {
            f(&mut left.as_mut().value);
            if left == right {
                return bubble_up_fold_from_node(right, settings);
            }
            if let Some(right_child) = left.as_mut().left_child_template_mut::<Reversed::Not>() {
                // return endpoints_foreach_mut_template::<Reversed,_,_,_>(right_child.all_the_way_left_template::<Reversed>().into(), right, settings, f);
                left = right_child.all_the_way_left_template::<Reversed>().into();
                continue
            };
            let mut with_next_parent = left.as_mut();
            loop {
                with_next_parent.recalc_delta(settings);
                if with_next_parent.is_right_child_template::<Reversed>() {
                    let Some(mut left_parent) = with_next_parent.parent_ptr else {
                        panic!("Invalid endpoints!")
                    };
                    with_next_parent = left_parent.as_mut();
                } else {
                    break
                }
            }
            let Some(parent_on_the_right) = with_next_parent.parent_ptr else {
                return bubble_up_fold_from_node(left, settings);
            };
            // return endpoints_foreach_mut_template::<Reversed,_,_,_>(parent_on_the_right, right, settings, f);
            left = parent_on_the_right;
            continue;
        }
    }
}

#[derive(PartialEq,Eq,Clone,Copy)]
enum EndpointTakerState {
    ExclusiveUnbalanced, //the entire subtree is not included, and also is unbalanced
    ExclusiveUnUpdated, //like above, but only needs update, not rebalance
    InclusiveLopsided, //is inclusive but its child on the outer side has been taken. A subcase of InclusiveUnbalanced
    InclusiveUnbalanced, //is inclusive, and unbalanced
    InclusiveUnUpdated //is inclusive, and only needs update
}

struct LeftEndpointTaker<Reversed: Bool, T, D: Clone> {
    state: EndpointTakerState,
    node: NonNull<WAVLNode<T,D>>,
    acc_inc: Option<Box<WAVLNode<T,D>>>,
    acc_exc: Option<Box<WAVLNode<T,D>>>,
    _m: PhantomData<Reversed>
}

impl<Reversed: Bool, T, D: Clone> LeftEndpointTaker<Reversed, T, D> {
    fn raise(&mut self,settings: impl FoldSettings<T,D>) {
        use EndpointTakerState::*;
        unsafe {
            let is_right_child = self.node.as_ref().is_right_child_template::<Reversed>();
            let Some(mut node_parent) = self.node.as_ref().parent_ptr else {unreachable!()};
            let mut former_node = core::mem::replace(&mut self.node,node_parent);
            match self.state {
                ExclusiveUnbalanced => {
                    let rebalance_continues = WAVLNode::rebalance_child_of_template_and_continues::<Reversed>(node_parent,is_right_child,settings);
                    if is_right_child {
                        //just go up one
                        if rebalance_continues {
                            // self.state = ExclusiveUnbalanced;
                        } else {
                            self.state =  ExclusiveUnUpdated;
                        }
                    } else {
                        //take all of left and add it to the exclusive part
                        let left_taken = core::mem::take(node_parent.as_mut().left_child_template_mut::<Reversed>());
                        WAVLNode::append_tree_right_opt_template::<Reversed::Not>(&mut self.acc_exc, left_taken, settings);
                        self.state = InclusiveLopsided;
                    }
                },
                ExclusiveUnUpdated => {
                    former_node.as_mut().inform_children_and_recalc(settings);
                    if is_right_child {
                        //just go up one
                        //self.state =  ExclusiveUnUpdated;
                    } else {
                        //take all of left and add it to the exclusive part
                        let left_taken = core::mem::take(node_parent.as_mut().left_child_template_mut::<Reversed>());
                        WAVLNode::append_tree_right_opt_template::<Reversed::Not>(&mut self.acc_exc, left_taken, settings);
                        self.state = InclusiveLopsided;
                    }
                },
                InclusiveLopsided => {
                    if is_right_child {
                        //           parent
                        //          /      \
                        //         ?       NODE
                        //               /    \
                        //              None   node_left
                        //replace NODE with acc_exc
                        let acc_exc_taken = core::mem::take(&mut self.acc_exc);
                        let mut node_taken = core::mem::replace(node_parent.as_mut().left_child_template_mut::<Reversed::Not>(),acc_exc_taken).unwrap();
                        let (None,node_left) = node_taken.shed_children_cswap::<Reversed>(settings) else {unreachable!()};
                        WAVLNode::append_tree_right_opt_template::<Reversed>(&mut self.acc_inc, Some(node_taken), settings);
                        WAVLNode::append_tree_right_opt_template::<Reversed>(&mut self.acc_inc, node_left, settings);
                        self.state = ExclusiveUnbalanced;
                    } else {
                        if WAVLNode::rebalance_child_of_template_and_continues::<Reversed>(node_parent,is_right_child,settings) {
                            self.state = InclusiveUnbalanced;
                        } else {
                            self.state = InclusiveUnUpdated;
                        }
                    }
                },
                InclusiveUnbalanced => {
                    let rebalance_continues = WAVLNode::rebalance_child_of_template_and_continues::<Reversed>(node_parent,is_right_child,settings);
                    if is_right_child {
                        let left_taken = core::mem::take(node_parent.as_mut().left_child_template_mut::<Reversed::Not>());
                        WAVLNode::append_tree_right_opt_template::<Reversed>(&mut self.acc_inc, left_taken, settings);
                        self.state = ExclusiveUnbalanced;
                    } else {
                        //just go one up
                        if rebalance_continues {
                            // self.state = InclusiveUnbalanced;
                        } else {
                            self.state = InclusiveUnUpdated;
                        }
                    };
                },
                InclusiveUnUpdated => {
                    former_node.as_mut().inform_children_and_recalc(settings);
                    if is_right_child {
                        let left_taken = core::mem::take(node_parent.as_mut().left_child_template_mut::<Reversed::Not>());
                        WAVLNode::append_tree_right_opt_template::<Reversed>(&mut self.acc_inc, left_taken, settings);
                        self.state = ExclusiveUnbalanced;
                    } else {
                        //just go one up
                        // self.state = InclusiveUnUpdated;
                    };
                },
            }
        }
    }

    fn is_inclusive(&self) -> bool {
        use EndpointTakerState::*;
        match self.state {
            ExclusiveUnbalanced |
            ExclusiveUnUpdated => false,
            InclusiveLopsided |
            InclusiveUnbalanced |
            InclusiveUnUpdated => true,
        }
    }

    fn is_unbalanced(&self) -> bool {
        use EndpointTakerState::*;
        match self.state {
            ExclusiveUnbalanced |
            InclusiveLopsided |
            InclusiveUnbalanced => true,
            ExclusiveUnUpdated |
            InclusiveUnUpdated => false,
        }
    }

    fn new_from_left(mut left: NonNull<WAVLNode<T,D>>) -> Self {
        unsafe {
            let acc_exc = core::mem::take(left.as_mut().left_child_template_mut::<Reversed>());
            LeftEndpointTaker::<Reversed,_,_> { 
                state: EndpointTakerState::InclusiveLopsided, 
                node: left, 
                acc_inc: None, 
                acc_exc, 
                _m: PhantomData 
            }
        }
    }
}

unsafe fn endpoint_take_all_from_left_onwards_template<Reversed: Bool,Settings: FoldSettings<T,D>,T,D: Clone>(
    left: NonNull<WAVLNode<T,D>>,
    mut root: NonNull<Option<Box<WAVLNode<T,D>>>>,
    settings: Settings
) -> Box<WAVLNode<T,D>> {
    unsafe {
        let mut taker = LeftEndpointTaker::<Reversed,_,_>::new_from_left(left);
        while taker.node.as_ref().parent_ptr.is_some() {
            taker.raise(settings);
        }
        debug_assert_eq!(NonNull::from(root.as_ref().as_ref().unwrap().as_ref()), taker.node);
        let root_mut_opt = root.as_mut();
        if taker.is_unbalanced() {
            WAVLNode::update_and_rebalance_continues(root_mut_opt, settings);
        } else {
            root_mut_opt.as_mut().unwrap().inform_children_and_recalc(settings);
        }
        if taker.is_inclusive() {
            //replace root with exc
            let root_taken = core::mem::replace(root_mut_opt,taker.acc_exc);
            WAVLNode::append_tree_right_opt_template::<Reversed>(&mut taker.acc_inc, root_taken, settings);
        } else {
            //add exc to root
            WAVLNode::append_tree_right_opt_template::<Reversed>(root_mut_opt, taker.acc_exc, settings);
        }
        if let Some(root_mut) = root_mut_opt {
            root_mut.parent_ptr = None;
        }
        let mut ret = taker.acc_inc.unwrap();
        ret.parent_ptr = None;
        ret
    }
}

unsafe fn endpoints_take_all<Settings: FoldSettings<T,D>, T, D: Clone>(
    left: NonNull<WAVLNode<T,D>>,
    right: NonNull<WAVLNode<T,D>>,
    root: NonNull<Option<Box<WAVLNode<T,D>>>>,
    settings: Settings
) -> Box<WAVLNode<T,D>> {
    unsafe {
        let mut left_taker = LeftEndpointTaker::<False,_,_>::new_from_left(left);
        let mut right_taker = LeftEndpointTaker::<True,_,_>::new_from_left(right);
        while left_taker.node != right_taker.node {
            let left_rank = left_taker.node.as_ref().rank;
            let right_rank = right_taker.node.as_ref().rank;
            //raise the one with the smaller rank
            if left_rank <= right_rank {
                left_taker.raise(settings);
            } 
            if left_rank >= right_rank {
                right_taker.raise(settings);
            }
        }
        if !left_taker.is_inclusive() || !right_taker.is_inclusive() {
            //this case can only happen when left was after right
            panic!("invalid endpoints")
        }
        WAVLNode::mutate_box_of_and_update_parents(left_taker.node, root, settings, |box_mut| {
            if left_taker.is_unbalanced() || right_taker.is_unbalanced() {
                WAVLNode::update_and_rebalance_continues(box_mut, settings);
            } else {
                box_mut.as_mut().unwrap().inform_children_and_recalc(settings);
            }
            let middle_taken = core::mem::take(box_mut).unwrap();

            WAVLNode::append_tree_right_opt(&mut left_taker.acc_inc, Some(middle_taken), settings);
            WAVLNode::append_tree_right_opt(&mut left_taker.acc_inc, right_taker.acc_inc, settings);

            WAVLNode::append_tree_left_opt(box_mut, left_taker.acc_exc, settings);
            WAVLNode::append_tree_right_opt(box_mut, right_taker.acc_exc, settings);
    
            left_taker.acc_inc.unwrap()
        })
    }
}

pub(crate) struct ImmSliceEndpoints<T, D: Clone> {
    left: NonNull<WAVLNode<T,D>>,
    right: NonNull<WAVLNode<T,D>>,
    root: NonNull<WAVLNode<T,D>>,
}

impl<T, D: Clone> ImmSliceEndpoints<T, D> {
    fn drop_left_until_template<Reversed: Bool, IsFlushLeft: Bool, IsFlushRight: Bool, Settings: FoldSettings<T,D>, Simplification: FoldSimplification<T,D>>(
        self,
        settings: Settings,
        simp: Simplification,
        predicate: impl Fn(&Simplification::D2) -> bool
    ) -> Option<Self> {
        unsafe {
            let left_if = IsFlushLeft::init_if_else((), |()| (), |()| self.left.clone());
            let right_if = IsFlushRight::init_if_else((), |()| (), |()| self.right.clone());
            let pre_fold = simp.empty(settings);
            let (_d,first_kept_opt) = if Reversed::b {
                let root_if = <IsFlushRight::And<IsFlushLeft> as Bool>::init_if_else((), |()| self.root.clone(), |()| ());
                node_of_first_where_fold_left_is_template::<Reversed,IsFlushRight,IsFlushLeft,_,_,_,_>(right_if, left_if, root_if, settings, simp, predicate, pre_fold)
            } else {
                let root_if= <IsFlushLeft::And<IsFlushRight> as Bool>::init_if_else((), |()| self.root.clone(), |()| ());
                node_of_first_where_fold_left_is_template::<Reversed,IsFlushLeft,IsFlushRight,_,_,_,_>(left_if, right_if, root_if, settings, simp, predicate, pre_fold)
            };
            let Some(first_kept) = first_kept_opt else {
                //nothing kept
                return None
            };
            return Some(if Reversed::b {
                Self {
                    left: self.left,
                    right: first_kept,
                    root: self.root,
                }
            } else {
                Self {
                    left: first_kept,
                    right: self.right,
                    root: self.root,
                }
            })
        }
    }

    fn take_right_until_template<Reversed: Bool, IsFlushLeft: Bool, IsFlushRight: Bool, Settings: FoldSettings<T,D>, Simplification: FoldSimplification<T,D>>(
        self,
        settings: Settings,
        simp: Simplification,
        predicate: impl Fn(&Simplification::D2) -> bool
    ) -> Option<Self> {
        unsafe {
            let left_if = IsFlushLeft::init_if_else((), |()| (), |()| self.left.clone());
            let right_if = IsFlushRight::init_if_else((), |()| (), |()| self.right.clone());
            let pre_fold = simp.empty(settings);
            let (_d,last_dropped_opt) = if Reversed::b {
                let root_if= <IsFlushLeft::And<IsFlushRight> as Bool>::init_if_else((), |()| self.root.clone(), |()| ());
                node_of_first_where_fold_left_is_template::<Reversed::Not,IsFlushLeft,IsFlushRight,_,_,_,_>(left_if, right_if, root_if, settings, simp, predicate, pre_fold)
            } else {
                let root_if = <IsFlushRight::And<IsFlushLeft> as Bool>::init_if_else((), |()| self.root.clone(), |()| ());
                node_of_first_where_fold_left_is_template::<Reversed::Not,IsFlushRight,IsFlushLeft,_,_,_,_>(right_if, left_if, root_if, settings, simp, predicate, pre_fold)
            };
            let Some(last_dropped) = last_dropped_opt else {
                //nothing dropped
                return Some(self)
            };
            if last_dropped == if Reversed::b {self.left} else {self.right} {
                //everything dropped
                return None
            }
            let Some(first_kept) = last_dropped.as_ref().next_single_left_to_right_template::<Reversed>() else {
                //everything dropped, but we already checked for this case
                unreachable!()
            };
            return Some(if Reversed::b {
                Self {
                    left: self.left,
                    right: first_kept,
                    root: self.root,
                }
            } else {
                Self {
                    left: first_kept,
                    right: self.right,
                    root: self.root,
                }
            })
        }
    }

    fn debug_check_structural_integrity_orig(&self) -> bool {
        debug_assert!(WAVLNode::debug_assert_a_has_parent_in_common_with_b_and_is_not_after(self.left, self.right));
        true
    }
}

impl<T, D: Clone> Clone for ImmSliceEndpoints<T, D> {
    fn clone(&self) -> Self {
        Self { left: self.left.clone(), right: self.right.clone(), root: self.root.clone() }
    }
}

/// The struct responsible for most immutable views into a [`FoldChain`].
pub struct ImmFoldChainSliceStruct<'a,
    IsReversed: Bool,
    IsFlushLeft: Bool,
    IsFlushRight: Bool,
    Settings: FoldSettings<T,D> + 'a,
    Simplification: FoldSimplification<T,D> + 'a,T: 'a,D: Clone + 'a> {
        pub(crate) endpoints: Option<ImmSliceEndpoints<T,D>>,
        pub(crate) settings: Settings,
        pub(crate) simplification: Simplification,
        pub(crate) _m: PhantomData<(&'a FoldChain<T,D,Settings>,IsReversed,IsFlushLeft,IsFlushRight)>
}

impl<'a, IsReversed: Bool, IsFlushLeft: Bool, IsFlushRight: Bool, Settings: FoldSettings<T,D> + 'a, Simplification: FoldSimplification<T,D> + 'a, T: 'a, D: Clone + 'a> ImmFoldChainSliceStruct<'a, IsReversed, IsFlushLeft, IsFlushRight, Settings, Simplification, T, D> {
    pub(crate) fn left_consume(self) -> Option<&'a T> {
        if IsReversed::b {
            unsafe {Some(&self.endpoints?.right.as_ref().value)}
        } else {
            unsafe {Some(&self.endpoints?.left.as_ref().value)}
        }
    }

    pub(crate) fn iter_consume(self) -> Iter<'a, IsReversed, T, D> {
        match self.endpoints {
            Some(ImmSliceEndpoints { left, right, root: _ }) => 
                Iter { next_and_next_back: Some((left,right)), _m: PhantomData },
            None => Iter { next_and_next_back: None, _m: PhantomData }
        }
    }
}

impl<'a, IsReversed: Bool, IsFlushLeft: Bool,IsFlushRight: Bool, Settings: FoldSettings<T,D>, Simplification: FoldSimplification<T,D>, T, D: Clone> 
Clone for ImmFoldChainSliceStruct<'a, IsReversed, IsFlushLeft,IsFlushRight, Settings, Simplification, T, D> {
    fn clone(&self) -> Self {
        Self { endpoints: self.endpoints.clone(),settings: self.settings.clone(), simplification: self.simplification.clone(), _m: self._m.clone() }
    }
}

impl<'a, IsReversed: Bool,  IsFlushLeft: Bool,IsFlushRight: Bool, Settings: FoldSettings<T,D>, Simplification: FoldSimplification<T,D>, T, D: Clone> 
FoldChainSlice<'a,T,Simplification::D2> for ImmFoldChainSliceStruct<'a, IsReversed,IsFlushLeft,IsFlushRight,Settings, Simplification, T, D> {
    type OriginalD = D;

    type Settings = Settings;

    fn get_settings(&self) -> Self::Settings {
        self.settings
    }

    type IsReversed = IsReversed;
    
    type IsFlushLeft = IsFlushLeft;
    
    type IsFlushRight = IsFlushRight;
    
    type Simplification = Simplification;

    fn get_current_simplification(&self) -> Self::Simplification {
        self.simplification.clone()
    }

    fn as_imm(self) -> ImmFoldChainSliceStruct<'a,Self::IsReversed,Self::IsFlushLeft,Self::IsFlushRight,Self::Settings,Self::Simplification,T,Self::OriginalD> {
        self
    }
    
    fn borrow<'b>(&'b self) -> ImmFoldChainSliceStruct<'b,Self::IsReversed,Self::IsFlushLeft,Self::IsFlushRight,Self::Settings,Self::Simplification,T,Self::OriginalD> {
        self.clone()
    }

    fn view_drop_left_until(self, predicate: impl Fn(&Simplification::D2)->bool) -> ImmFoldChainSliceStruct<'a, IsReversed, <IsFlushLeft as Bool>::And<IsReversed>, <IsFlushRight as Bool>::And<<IsReversed as Bool>::Not>, Settings, Simplification, T, D> {
        let endpoints2 = match self.endpoints {
            Some(ep) => ep.drop_left_until_template::<IsReversed,IsFlushLeft,IsFlushRight,_,_>(self.settings, self.simplification.clone(), predicate),
            None => None,
        };
        ImmFoldChainSliceStruct {
            endpoints: endpoints2,
            settings: self.settings,
            simplification: self.simplification,
            _m: PhantomData,
        }
    }

    fn view_take_right_until(self, predicate: impl Fn(&Simplification::D2)->bool) -> ImmFoldChainSliceStruct<'a, IsReversed, <IsFlushLeft as Bool>::And<IsReversed>, <IsFlushRight as Bool>::And<<IsReversed as Bool>::Not>, Settings, Simplification, T, D> {
        let endpoints2 = match self.endpoints {
            Some(ep) => ep.take_right_until_template::<IsReversed,IsFlushLeft,IsFlushRight,_,_>(self.settings, self.simplification.clone(), predicate),
            None => None,
        };
        ImmFoldChainSliceStruct {
            endpoints: endpoints2,
            settings: self.settings,
            simplification: self.simplification,
            _m: PhantomData,
        }
    }


    fn view_reversed(self) -> ImmFoldChainSliceStruct<'a, <IsReversed as Bool>::Not, IsFlushLeft, IsFlushRight, Settings, Simplification, T, D> {
        ImmFoldChainSliceStruct {
            endpoints: self.endpoints,
            settings: self.settings,
            simplification: self.simplification,
            _m: PhantomData,
        }
    }

    fn view_with_simplification<NewSimplification: FoldSimplification<T,Simplification::D2>>(self, new_simplification: NewSimplification) -> ImmFoldChainSliceStruct<'a,Self::IsReversed,Self::IsFlushLeft,Self::IsFlushRight,Self::Settings,
            NewSimplification::ComposeAfterOther<Self::OriginalD,Self::Simplification>,
            T,Self::OriginalD> {
        ImmFoldChainSliceStruct { 
            endpoints: self.endpoints, 
            settings: self.settings, 
            simplification: new_simplification.compose_after_other(self.simplification), 
            _m: PhantomData
        }
        
    }

    fn view_unsimplify(self) -> ImmFoldChainSliceStruct<'a, IsReversed, IsFlushLeft, IsFlushRight, Settings, (), T, D> {
       ImmFoldChainSliceStruct {
            endpoints: self.endpoints,
            settings: self.settings,
            simplification: (),
            _m: PhantomData,
        }
    }

    fn fold(&self) -> Simplification::D2 {
        let Some(eps) = self.endpoints.clone() else {return self.simplification.empty(self.settings)};
        let left_if = IsFlushLeft::init_if_else(eps.left, |_| (), |l| l);
        let right_if = IsFlushRight::init_if_else(eps.right, |_| (), |r| r);
        let root_if = <IsFlushLeft::And<IsFlushRight> as Bool>::init_if_else(eps.root, |r| r, |_| ());
        endpoints_get_fold::<IsFlushLeft,IsFlushRight,_,_,_,_>(left_if, right_if,root_if,  self.settings, self.simplification.clone())
    }

    fn is_empty(&self) -> bool {
        self.endpoints.is_none()
    }

    fn left<'b>(&'b self) -> Option<&'b T> where 'a: 'b{
        unsafe {
            if IsReversed::b {
                Some(&self.endpoints.as_ref()?.right.as_ref().value)
            } else {
                Some(&self.endpoints.as_ref()?.left.as_ref().value)
            }
        }
    }

    fn right<'b>(&'b self) -> Option<&'b T> where 'a: 'b {
        unsafe {
            if IsReversed::b {
                Some(&self.endpoints.as_ref()?.left.as_ref().value)
            } else {
                Some(&self.endpoints.as_ref()?.right.as_ref().value)
            }
        }
    }
    
    fn foreach(&self, f: impl FnMut(&T)) {
        let Some(ImmSliceEndpoints { left, right, root: _ }) = self.endpoints else {return};
        let (l,r) = cswap::<IsReversed,_>(left,right);
        unsafe {endpoints_foreach_template::<IsReversed,_,_>(l, r, f);}
    }
    
    fn debug_check_structural_integrity(&self) -> bool {
        if let Some(eps) = &self.endpoints {
            debug_assert!(eps.debug_check_structural_integrity_orig());
        }
        true
    }
}

fn res_ref_clone<T: Clone>(resref: &Result<&mut T,T>) -> T {
    match resref {
        Ok(r) => (*r).clone(),
        Err(r) => r.clone(),
    }
}

fn res_ref_borrow_mut<'a,'b,T>(resref: &'b mut Result<&'a mut T,T>) -> &'b mut T {
    match resref {
        Ok(r) => &mut **r,
        Err(r) => r,
    }
}

enum MutSliceEndpointsWhenBothNotFlush<'a,T> {
    ShrankBoth(T,T),
    ShrankLeft(T, &'a mut T),
    ShrankRight(&'a mut T, T),
    ShrankLeftBorrowed(&'a mut T, &'a mut T),
    ShrankRightBorrowed(&'a mut T, &'a mut T)
    //the above two states are different because they tell the parent to shrink in different ways in process of this slice becoming empty
}

pub(crate) struct MutSliceEndpoints<'a,IsFlushLeft: Bool, IsFlushRight: Bool, Settings: FoldSettings<T,D>, T, D: Clone> {
    base: NonNull<FoldChain<T,D,Settings>>, //pointer beause left or right might have a mutable reference into it
    left_right: IsFlushLeft::IfElse<
        IsFlushRight::IfElse<
            (), //get both from base
            Result<
                &'a mut Option<NonNull<WAVLNode<T,D>>>,
                Option<NonNull<WAVLNode<T,D>>>
            > //possibly override right
        >,
        IsFlushRight::IfElse<
            Result<
                &'a mut Option<NonNull<WAVLNode<T,D>>>,
                Option<NonNull<WAVLNode<T,D>>>
            >, //possibly override left
            MutSliceEndpointsWhenBothNotFlush<'a,Option<NonNull<WAVLNode<T,D>>>> //possibly override both
        >
    >,
    _m: PhantomData<&'a mut FoldChain<T,D,Settings>>
    // if we're flush on a side, we inherit it from base
    // if (left,right) is...
    //      (None,None) => slice is empty *and* base is empty
    //      (None,Some) => slice is empty to the right of right
    //      (Some,None) => slice is empty to the left of left
    //      (Some,Some) => slice is everything between left and right, inclusive for both
}

impl<'a, IsFlushLeft: Bool, IsFlushRight: Bool, Settings: FoldSettings<T,D>, T, D: Clone> MutSliceEndpoints<'a, IsFlushLeft, IsFlushRight, Settings, T, D> {
    fn get_left(&self) -> Option<NonNull<WAVLNode<T,D>>> {
        unsafe {
            IsFlushLeft::close_if_else_ref(&self.left_right, 
                |_| self.base.as_ref().leftmost_node_ptr,
                |lr| IsFlushRight::close_if_else_ref(lr, 
                    |l| res_ref_clone(l), 
                    |lr| match lr {
                        MutSliceEndpointsWhenBothNotFlush::ShrankBoth(l, _) |
                        MutSliceEndpointsWhenBothNotFlush::ShrankLeft(l, _) => *l,
                        MutSliceEndpointsWhenBothNotFlush::ShrankRight(l, _) |
                        MutSliceEndpointsWhenBothNotFlush::ShrankLeftBorrowed(l, _) | 
                        MutSliceEndpointsWhenBothNotFlush::ShrankRightBorrowed(l, _) => **l,
                    }
                ), 
            )
        }
    }

    fn get_right(&self) -> Option<NonNull<WAVLNode<T,D>>> {
        unsafe {
            IsFlushLeft::close_if_else_ref(&self.left_right, 
                |lr| IsFlushRight::close_if_else_ref(lr, 
                    |()| self.base.as_ref().rightmost_node_ptr, 
                    |r| res_ref_clone(r)), 
                |lr| IsFlushRight::close_if_else_ref(lr, 
                    |_| self.base.as_ref().rightmost_node_ptr, 
                    |lr| match lr {
                        MutSliceEndpointsWhenBothNotFlush::ShrankBoth(_, r) |
                        MutSliceEndpointsWhenBothNotFlush::ShrankRight(_, r) => *r,
                        MutSliceEndpointsWhenBothNotFlush::ShrankLeft(_, r) |
                        MutSliceEndpointsWhenBothNotFlush::ShrankLeftBorrowed(_, r) |
                        MutSliceEndpointsWhenBothNotFlush::ShrankRightBorrowed(_, r) => **r,
                    }
                )
            )
        }
    }

    fn get_left_template<Reversed: Bool>(&self) -> Option<NonNull<WAVLNode<T,D>>> {
        if Reversed::b {
            self.get_right()
        } else {
            self.get_left()
        }
    }

    //ignores root. caller must take of that themself
    fn endpoints_become_empty(&mut self) {
        fn set_left_endpoint_to_none<ActuallyRight: Bool, T, D: Clone, Settings: FoldSettings<T,D>>(left: &mut Option<NonNull<WAVLNode<T,D>>>, mut base: NonNull<FoldChain<T,D,Settings>>) {
            unsafe {
                let Some(former_left) = core::mem::take(left) else {return};
                //if left was actually the first, then set the other base endpoint to None as well
                if NonNull::from(left) == NonNull::from(base.as_mut().leftmost_node_ptr_mut_template::<ActuallyRight>()) {
                    *base.as_mut().leftmost_node_ptr_mut_template::<ActuallyRight::Not>() = None;
                } else {
                    let left_of_left = former_left.as_ref().next_single_left_to_right_template::<ActuallyRight::Not>();
                    debug_assert!(left_of_left.is_some());
                    *base.as_mut().leftmost_node_ptr_mut_template::<ActuallyRight::Not>() = left_of_left;
                }
            }
        }
        unsafe {
            match (IsFlushLeft::b, IsFlushRight::b) {
                (true, true) => {
                    let base_mut = self.base.as_mut();
                    base_mut.leftmost_node_ptr = None;
                    base_mut.rightmost_node_ptr = None;
                },
                (true, false) => {
                    let right = IsFlushRight::assert_false_unwrap_mut(IsFlushLeft::assert_unwrap_mut(&mut self.left_right));
                    match right {
                        Ok(r) => set_left_endpoint_to_none::<True,_,_,_>(r, self.base),
                        Err(r) => set_left_endpoint_to_none::<True,_,_,_>(r, self.base),
                    }
                },
                (false, true) => {
                    let left = IsFlushRight::assert_unwrap_mut(IsFlushLeft::assert_false_unwrap_mut(&mut self.left_right));
                    match left {
                        Ok(l) => set_left_endpoint_to_none::<False,_,_,_>(l, self.base),
                        Err(l) => set_left_endpoint_to_none::<False,_,_,_>(l, self.base),
                    }
                },
                (false, false) => {
                    let left_right = IsFlushRight::assert_false_unwrap_mut(IsFlushLeft::assert_false_unwrap_mut(&mut self.left_right));
                    use MutSliceEndpointsWhenBothNotFlush::*;
                    match left_right {
                        ShrankBoth(None, _) |
                        ShrankBoth(_, None) |
                        ShrankLeft(None, _) |
                        ShrankLeft(_, None) |
                        ShrankRight(None, _) |
                        ShrankRight(_, None) |
                        ShrankLeftBorrowed(None, _) |
                        ShrankLeftBorrowed(_, None) |
                        ShrankRightBorrowed(None, _) | 
                        ShrankRightBorrowed(_, None) => {
                            //already empty. do nothing
                        }
                        ShrankBoth(Some(l),ropt @ Some(_)) => {
                            //arbitrarily choose right to be none
                            when_shrank_left::<True,_,_,_>(self.base, ropt, l);
                        }
                        ShrankLeft(lopt @ Some(_), Some(r)) => {
                            when_shrank_left::<False,_,_,_>(self.base, lopt, r);
                        },
                        ShrankLeftBorrowed(lopt @ Some(_), Some(r)) => {
                            when_shrank_left::<False,_,_,_>(self.base, lopt, r);
                        },
                        ShrankRight(Some(l), ropt @ Some(_)) => {
                            when_shrank_left::<True,_,_,_>(self.base, ropt, l);
                        },
                        ShrankRightBorrowed(Some(l), ropt @ Some(_)) => {
                            when_shrank_left::<True,_,_,_>(self.base, ropt, l);
                        },
                    }
                    fn when_shrank_left<ActuallyRight: Bool, T,D: Clone, Settings: FoldSettings<T,D>>(mut base: NonNull<FoldChain<T,D,Settings>>, lopt: &mut Option<NonNull<WAVLNode<T,D>>>, r: &mut NonNull<WAVLNode<T,D>>) {
                        unsafe {
                            let Some(left_of_left) = lopt.as_mut().unwrap().as_mut().next_single_left_to_right_template::<ActuallyRight::Not>() else {
                                //we're entire
                                base.as_mut().leftmost_node_ptr = None;
                                base.as_mut().rightmost_node_ptr = None;
                                return
                            };
                            *r = left_of_left;
                            *lopt = None;
                        }
                    }
                },
            }
        }
    }

    fn to_imm(&self) -> Option<ImmSliceEndpoints<T,D>> {
        unsafe {
            Some(ImmSliceEndpoints { 
                left: self.get_left()?, 
                right: self.get_right()?, 
                root: self.base.as_ref().root.as_ref().unwrap().as_ref().into(), //unwrap because left and right existing imply root exists
            })
        }
    }

    fn borrow_mut<'b>(&'b mut self) -> MutSliceEndpoints<'b,IsFlushLeft,IsFlushRight,Settings,T,D> {
        use MutSliceEndpointsWhenBothNotFlush::*;
        MutSliceEndpoints { 
            base: self.base, 
            left_right: IsFlushLeft::map_cases(IsFlushLeft::as_mut(&mut self.left_right), 
                |lr| IsFlushRight::map_cases(IsFlushRight::as_mut(lr), 
                    |()| (), 
                    |r| Ok(res_ref_borrow_mut(r))
                ), 
                |lr| IsFlushRight::map_cases(IsFlushRight::as_mut(lr),
                    |l| Ok(res_ref_borrow_mut(l)),
                    |lr| match lr {
                        ShrankBoth(l,r) => ShrankLeftBorrowed(l, r), //arbitrary
                        ShrankLeft(l, r) => ShrankLeftBorrowed(l, *r),
                        ShrankRight(l, r) => ShrankRightBorrowed(*l, r),
                        ShrankLeftBorrowed(l, r) => ShrankLeftBorrowed(*l, *r),
                        ShrankRightBorrowed(l, r) => ShrankRightBorrowed(*l, *r),
                    }
                )
            ),
            _m: PhantomData, 
        }
    }

    fn downgrade_left<Reversed: Bool>(mut self) -> MutSliceEndpoints<'a,IsFlushLeft::And<Reversed>,IsFlushRight::And<Reversed::Not>,Settings,T,D> {
        unsafe {
            MutSliceEndpoints {
                base: self.base,
                left_right: match (Reversed::b, IsFlushLeft::b, IsFlushRight::b) {
                    (false,false,_) |
                    (true,_,false) => {
                        //do nothing
                        bool_assert_into::<IsFlushLeft,IsFlushLeft::And<Reversed>,_,_>(IsFlushLeft::map_cases(self.left_right, 
                            |lr| bool_assert_into::<IsFlushRight,IsFlushRight::And<Reversed::Not>,_,_>(lr), 
                            |lr| bool_assert_into::<IsFlushRight,IsFlushRight::And<Reversed::Not>,_,_>(lr), 
                        ))
                    },
                    (true, true, true) => {
                        IsFlushLeft::And::<Reversed>::assert_init(
                            IsFlushRight::And::<Reversed::Not>::assert_false_init(
                                Ok(&mut self.base.as_mut().rightmost_node_ptr)))
                    },
                    (false, true, true) => {
                        IsFlushLeft::And::<Reversed>::assert_false_init(
                            IsFlushRight::And::<Reversed::Not>::assert_init(
                                Ok(&mut self.base.as_mut().leftmost_node_ptr)))
                    },
                    (true, false, true) => {
                        let left = IsFlushRight::assert_unwrap(IsFlushLeft::assert_false_unwrap(self.left_right));
                        IsFlushLeft::And::<Reversed>::assert_false_init(
                            IsFlushRight::And::<Reversed::Not>::assert_false_init(
                                match left {
                                    Ok(l) => 
                                        MutSliceEndpointsWhenBothNotFlush::ShrankLeftBorrowed(l, &mut self.base.as_mut().rightmost_node_ptr),
                                    Err(l) => 
                                        MutSliceEndpointsWhenBothNotFlush::ShrankLeft(l, &mut self.base.as_mut().rightmost_node_ptr),
                                }
                            )
                        )
                    },
                    (false, true, false) => {
                        let right = IsFlushRight::assert_false_unwrap(IsFlushLeft::assert_unwrap(self.left_right));
                        IsFlushLeft::And::<Reversed>::assert_false_init(
                            IsFlushRight::And::<Reversed::Not>::assert_false_init(
                                match right {
                                    Ok(r) => 
                                        MutSliceEndpointsWhenBothNotFlush::ShrankRightBorrowed(&mut self.base.as_mut().leftmost_node_ptr, r),
                                    Err(r) => 
                                        MutSliceEndpointsWhenBothNotFlush::ShrankRight(&mut self.base.as_mut().leftmost_node_ptr, r),
                                }
                            )
                        )
                    },
                },
                _m: PhantomData,
            }
        }
    }

    fn contract_left_to_template<Reversed: Bool>(self, new_left: Option<NonNull<WAVLNode<T,D>>>) -> MutSliceEndpoints<'a,IsFlushLeft::And<Reversed>,IsFlushRight::And<Reversed::Not>,Settings,T,D> {
        use MutSliceEndpointsWhenBothNotFlush::*;
        MutSliceEndpoints {
            base: self.base,
            left_right: IsFlushLeft::And::<Reversed>::init_if_else( self.left_right, 
                |_| IsFlushRight::And::<Reversed::Not>::assert_false_init(Err(new_left)), 
                |lr| IsFlushRight::And::<Reversed::Not>::init_if_else(lr, 
                    |_| Err(new_left), 
                    |lr| IsFlushLeft::close_if_else(lr, 
                        |lr| match IsFlushRight::assert_false_unwrap(lr) {
                            //must not be reversed (was flush left, now not)
                            Ok(r) => ShrankLeft(new_left, r),
                            Err(r) => ShrankBoth(new_left, r),
                        }, 
                        |lr| IsFlushRight::close_if_else(lr, 
                            |l| match l {
                                //must be reversed (was flush right, now not)
                                Ok(l) => ShrankRight(l, new_left),
                                Err(l) => ShrankBoth(l, new_left),
                            }, 
                            |lr| {
                                if Reversed::b {
                                    match lr {
                                        ShrankBoth(l, _) |
                                        ShrankLeft(l, _) => ShrankBoth(l, new_left),
                                        ShrankRight(l, _) |
                                        ShrankLeftBorrowed(l, _) |
                                        ShrankRightBorrowed(l, _) => ShrankRight(l, new_left),
                                    }
                                } else {
                                    match lr {
                                        ShrankBoth(_, r) |
                                        ShrankRight(_, r) => ShrankBoth(new_left, r),
                                        ShrankLeft(_, r) |
                                        ShrankLeftBorrowed(_, r) |
                                        ShrankRightBorrowed(_, r) => ShrankLeft(new_left, r),
                                    }
                                }
                            }
                        )
                    )
                )
            ),
            _m: PhantomData,
        }
    }

    #[inline]
    fn left_if_opt_and_right_if_opt(&self) -> (IsFlushLeft::IfElse<(),Option<NonNull<WAVLNode<T,D>>>>,IsFlushRight::IfElse<(),Option<NonNull<WAVLNode<T,D>>>>) {
        IsFlushLeft::close_if_else_ref(&self.left_right, 
            |lr| (IsFlushLeft::assert_init(()), IsFlushRight::close_if_else_ref(lr, 
                |()| IsFlushRight::assert_init(()), 
                |r| IsFlushRight::assert_false_init(res_ref_clone(r))
            )), 
            |lr| IsFlushRight::close_if_else_ref(lr, 
                |l| (IsFlushLeft::assert_false_init(res_ref_clone(l)),IsFlushRight::assert_init(())), 
                |lr| {
                    let (l,r) = match lr {
                        MutSliceEndpointsWhenBothNotFlush::ShrankBoth(l, r) => (*l,*r),
                        MutSliceEndpointsWhenBothNotFlush::ShrankLeft(l, r) => (*l,**r),
                        MutSliceEndpointsWhenBothNotFlush::ShrankRight(l, r) => (**l,*r),
                        MutSliceEndpointsWhenBothNotFlush::ShrankLeftBorrowed(l, r) |
                        MutSliceEndpointsWhenBothNotFlush::ShrankRightBorrowed(l, r) => (**l,**r),
                    };
                    (IsFlushLeft::assert_false_init(l),IsFlushRight::assert_false_init(r))
                }
            )
        )
    }

    fn drop_left_until_template<Reversed : Bool, Simp: FoldSimplification<T,D>>(
        self, 
        simp: Simp, 
        predicate: impl Fn(&Simp::D2)->bool
    ) -> MutSliceEndpoints<'a,IsFlushLeft::And<Reversed>,IsFlushRight::And<Reversed::Not>,Settings,T,D> {
        unsafe {
            let (left_if,right_if) = self.left_if_opt_and_right_if_opt();
            if !IsFlushLeft::b && IsFlushLeft::assert_false_unwrap(IsFlushLeft::as_ref(&left_if)).is_none() {
                //already empty
                return self.downgrade_left()
            }
            let left_if = IsFlushLeft::map_cases(left_if, |()| (), |l| l.unwrap());
            if !IsFlushRight::b && IsFlushRight::assert_false_unwrap(IsFlushRight::as_ref(&right_if)).is_none() {
                //already empty
                return self.downgrade_left()
            }
            let right_if = IsFlushRight::map_cases(right_if, |()| (), |r| r.unwrap());
            let settings = self.base.as_ref().settings;
            let pre_fold = simp.empty(settings);
            let Some(root_ref) = self.base.as_ref().root.as_ref() else {
                //also already empty
                return self.downgrade_left()
            };
            let (_delta,first_kept_opt) = if Reversed::b {
                let root_if_both_flush = <IsFlushRight::And<IsFlushLeft> as Bool>::init_if_else((), |()| NonNull::from(root_ref.as_ref()), |()| ());
                node_of_first_where_fold_left_is_template::<Reversed,IsFlushRight,IsFlushLeft,_,_,_,_>(right_if, left_if, root_if_both_flush, settings, simp, predicate, pre_fold)
            } else {
                let root_if_both_flush = <IsFlushLeft::And<IsFlushRight> as Bool>::init_if_else((), |()| NonNull::from(root_ref.as_ref()), |()| ());
                node_of_first_where_fold_left_is_template::<Reversed,IsFlushLeft,IsFlushRight,_,_,_,_>(left_if, right_if, root_if_both_flush,settings, simp, predicate, pre_fold)
            };
            if first_kept_opt == self.get_left_template::<Reversed>() {
                //first kept is first, do nothing
                return self.downgrade_left()
            }
            self.contract_left_to_template(first_kept_opt)
        }
    }

    fn take_right_until_template<Reversed : Bool, Simp: FoldSimplification<T,D>>(
        self, 
        simp: Simp, 
        predicate: impl Fn(&Simp::D2)->bool
    ) -> MutSliceEndpoints<'a,IsFlushLeft::And<Reversed>,IsFlushRight::And<Reversed::Not>,Settings,T,D> {
        unsafe {
            let (left_if,right_if) = self.left_if_opt_and_right_if_opt();
            if !IsFlushLeft::b && IsFlushLeft::assert_false_unwrap(IsFlushLeft::as_ref(&left_if)).is_none() {
                //already empty
                return self.downgrade_left()
            }
            let left_if = IsFlushLeft::map_cases(left_if, |()| (), |l| l.unwrap());
            if !IsFlushRight::b && IsFlushRight::assert_false_unwrap(IsFlushRight::as_ref(&right_if)).is_none() {
                //already empty
                return self.downgrade_left()
            }
            let right_if = IsFlushRight::map_cases(right_if, |()| (), |r| r.unwrap());
            let settings = self.base.as_ref().settings;
            let pre_fold = simp.empty(settings);
            let left_if_clone = bool_ifelse_clone::<IsFlushLeft,_,_>(&left_if);
            let right_if_clone = bool_ifelse_clone::<IsFlushRight,_,_>(&right_if);
            let Some(root) = self.base.as_ref().root.as_ref() else {
                if IsFlushLeft::b && IsFlushRight::b {
                    return self.downgrade_left()
                } else {
                    //case should have been caught already
                    unreachable!()
                }
            };
            let (_delta, first_dropped_opt) = if Reversed::b {
                let root_if_both_flush = <IsFlushLeft::And<IsFlushRight> as Bool>::init_if_else((), |()| NonNull::from(root.as_ref()), |()| ());
                node_of_first_where_fold_left_is_template::<Reversed::Not,IsFlushLeft,IsFlushRight,_,_,_,_>(left_if, right_if, root_if_both_flush,settings, simp, predicate, pre_fold)
            } else {
                let root_if_both_flush = <IsFlushRight::And<IsFlushLeft> as Bool>::init_if_else((), |()| NonNull::from(root.as_ref()), |()| ());
                node_of_first_where_fold_left_is_template::<Reversed::Not,IsFlushRight,IsFlushLeft,_,_,_,_>(right_if, left_if, root_if_both_flush, settings, simp, predicate, pre_fold)
            };
            let Some(last_dropped) = first_dropped_opt else {
                //nothing was dropped
                return self.downgrade_left()
            };
            let start_side_is_flush = cswap::<Reversed,_>(IsFlushRight::b,IsFlushLeft::b).0;
            if !start_side_is_flush {
                if !Reversed::b {
                    if IsFlushRight::assert_false_unwrap(right_if_clone) == last_dropped {
                        //everything was dropped
                        return self.contract_left_to_template(None)
                    }
                } else {
                    if IsFlushLeft::assert_false_unwrap(left_if_clone) == last_dropped {
                        //everything was dropped
                        return self.contract_left_to_template(None)
                    }
                }
            }
            let first_kept = last_dropped.as_ref().next_single_left_to_right_template::<Reversed>();
            if first_kept == self.get_left_template::<Reversed>() {
                //first kept is first, do nothing
                return self.downgrade_left()
            }
            self.contract_left_to_template(first_kept)
        } 
    }

    #[inline]
    fn left_right_opt_mut(&mut self) -> (&mut Option<NonNull<WAVLNode<T,D>>>,&mut Option<NonNull<WAVLNode<T,D>>>) {
        unsafe {
            IsFlushLeft::close_if_else_mut(&mut self.left_right, 
                |lr| IsFlushRight::close_if_else_mut(lr, 
                    |()| (&mut self.base.clone().as_mut().leftmost_node_ptr,&mut self.base.clone().as_mut().rightmost_node_ptr), 
                    |r| (&mut self.base.clone().as_mut().leftmost_node_ptr, res_ref_borrow_mut(r))
                ), 
                |lr| IsFlushRight::close_if_else_mut(lr, 
                    |l| (res_ref_borrow_mut(l),&mut self.base.clone().as_mut().rightmost_node_ptr), 
                    |lr| match lr {
                        MutSliceEndpointsWhenBothNotFlush::ShrankBoth(l, r) => (l,r),
                        MutSliceEndpointsWhenBothNotFlush::ShrankLeft(l, r) => (l,r),
                        MutSliceEndpointsWhenBothNotFlush::ShrankRight(l, r) => (l,r),
                        MutSliceEndpointsWhenBothNotFlush::ShrankLeftBorrowed(l, r) |
                        MutSliceEndpointsWhenBothNotFlush::ShrankRightBorrowed(l, r) => (l,r),
                    }
                )
            )
        }
    }

    fn left_right_opt_mut_opt(&mut self) -> Option<(&mut Option<NonNull<WAVLNode<T,D>>>,&mut Option<NonNull<WAVLNode<T,D>>>)> {
        let (left,right) = self.left_right_opt_mut();
        if left.is_none() || right.is_none() {return None}
        Some((left,right))
    }

    fn pop_left_template<Reversed: Bool>(&mut self) -> Option<T> {
        unsafe {
            let root = &self.base.as_ref().root;
            if root.is_none() {return None}
            let settings = self.base.as_ref().settings;
            let root_ptr = NonNull::from(root);
            let (left_mut,right_mut) = self.left_right_opt_mut_opt()?;
            let (l,r) = cswap::<Reversed,_>(left_mut, right_mut);
            let to_pop = if *l == *r {
                let l_copy = (*l).unwrap();
                self.endpoints_become_empty();
                l_copy
            } else {
                let Some(after_left) = l.unwrap().as_ref().next_single_left_to_right_template::<Reversed>() else {unreachable!()};
                let Some(former_left) = core::mem::replace(l,Some(after_left)) else {unreachable!()};
                former_left
            };
            return WAVLNode::mutate_box_of_and_update_parents(to_pop, root_ptr, settings, |b| {
                Some(WAVLNode::pop_top_in_place_boxed(b, settings).unwrap().value)
            })
        }
    }

    fn push_left_template<Reversed: Bool>(&mut self, value: T) {
        unsafe {
            let settings = self.base.as_ref().settings;
            let root = NonNull::from(&self.base.as_ref().root);
            let (left_mut,right_mut) = self.left_right_opt_mut();
            if left_mut.is_none() && right_mut.is_none() {
                debug_assert!(self.base.as_ref().root.is_none());
                let base = self.base.as_mut();
                let leaf = WAVLNode::new_leaf(settings, value);
                let leaf_addr = leaf.as_ref().map(|n| NonNull::from(n.as_ref()));
                let None = core::mem::replace(&mut base.root,leaf) else {unreachable!()};
                base.leftmost_node_ptr = leaf_addr;
                base.rightmost_node_ptr = leaf_addr;
                self.left_right = IsFlushLeft::init_if_else((), 
                    |()| IsFlushRight::init_if_else((), 
                        |()| (), 
                        |()| Ok(&mut self.base.clone().as_mut().rightmost_node_ptr)
                    ), 
                    |()| IsFlushRight::init_if_else((), 
                        |()| Ok(&mut self.base.clone().as_mut().leftmost_node_ptr), 
                        |()| MutSliceEndpointsWhenBothNotFlush::ShrankLeftBorrowed(&mut self.base.clone().as_mut().leftmost_node_ptr, &mut self.base.clone().as_mut().rightmost_node_ptr)
                    )
                );
                return
            }
            let (l,r) = cswap::<Reversed,_>(left_mut, right_mut);
            endpoints_push_left_template::<Reversed,_,_,_>(l, r, root, settings, value);
        }
    }

    fn set_left_or_err_template<Reversed: Bool>(&mut self, value: T) -> Result<T,T> {
        unsafe {
            let Some((left_mut,right_mut)) = self.left_right_opt_mut_opt() else {return Err(value)};
            let left = if Reversed::b {right_mut} else {left_mut}
                .as_mut().unwrap().as_mut();
            let ret = core::mem::replace(&mut left.value,value);
            bubble_up_fold_from_node(left.into(), self.base.as_ref().settings);
            Ok(ret)
        }
    }

    fn update_left_template<Reversed: Bool,R>(&mut self, f: impl FnOnce(Option<&mut T>) -> R) -> R {
        unsafe {
            let Some((left_mut,right_mut)) = self.left_right_opt_mut_opt() else {return f(None)};
            let left = if Reversed::b {right_mut} else {left_mut}
                .as_mut().unwrap().as_mut();
            let ret = f(Some(&mut left.value));
            bubble_up_fold_from_node(left.into(), self.base.as_ref().settings);
            ret
        }
    }

    fn take_all_template(&mut self) -> FoldChain<T,D,Settings> {
        unsafe {
            let settings = self.base.as_ref().settings;
            if IsFlushLeft::b && IsFlushRight::b {
                return core::mem::replace(&mut self.base.as_mut(),FoldChain::from_settings(settings))
            }
            let Some(ImmSliceEndpoints { left, right, root: _ }) = self.to_imm() else {
                return FoldChain::from_settings(settings);
            };
            self.endpoints_become_empty();
            let root_ptr = NonNull::from(&mut self.base.as_mut().root);
            let mut taken =  match (IsFlushLeft::b,IsFlushRight::b) {
                (true, true) => unreachable!(),
                (true, false) =>{
                    endpoint_take_all_from_left_onwards_template::<True,_,_,_>(right, root_ptr, settings)
                },
                (false, true) => {
                    endpoint_take_all_from_left_onwards_template::<False,_,_,_>(left, root_ptr, settings)
                },
                (false, false) => {
                    endpoints_take_all(left, right, root_ptr, settings)
                },
            };
            taken.parent_ptr = None;
            FoldChain {
                leftmost_node_ptr: Some(taken.all_the_way_left_template::<False>().into()),
                rightmost_node_ptr: Some(taken.all_the_way_left_template::<True>().into()),
                root: Some(taken),
                settings,
            }
        }
    }

    fn append_all_right_template<Reversed: Bool>(&mut self, other: FoldChain<T,D,Settings>) {
        unsafe {
            let root = NonNull::from(&self.base.as_ref().root);
            let settings = self.base.as_ref().settings;
            let (left_mut,right_mut) = self.left_right_opt_mut();
            if left_mut.is_none() && right_mut.is_none() {
                let base_mut = self.base.as_mut();
                debug_assert!(base_mut.root.is_none());
                *base_mut = other;
                self.left_right = IsFlushLeft::init_if_else((), 
                    |()| IsFlushRight::init_if_else((), 
                        |()| (), 
                        |()| Ok(&mut self.base.clone().as_mut().rightmost_node_ptr)
                    ), 
                    |()| IsFlushRight::init_if_else((), 
                        |()| Ok(&mut self.base.clone().as_mut().leftmost_node_ptr), 
                        |()| MutSliceEndpointsWhenBothNotFlush::ShrankLeftBorrowed(&mut self.base.clone().as_mut().leftmost_node_ptr, &mut self.base.clone().as_mut().rightmost_node_ptr)
                    )
                );
                return
            }
            endpoints_append_all_right_template::<Reversed,_,_,_>(left_mut, right_mut,root,settings,other);
        }
    }

    fn foreach_mut_template<Reversed: Bool>(&mut self, f: impl FnMut(&mut T)) {
        unsafe {
            let Some(ImmSliceEndpoints { left, right, root: _ }) = self.to_imm() else {return};
            let (l2,r2) = cswap::<Reversed,_>(left,right);
            endpoints_foreach_mut_template::<Reversed,_,_,_>(l2, r2, self.base.as_ref().settings, f);
        }
    }

    fn debug_check_structural_integrity(&self) -> bool {
        unsafe {
            debug_assert!(self.base.as_ref().debug_check_structural_integrity());
            if !IsFlushLeft::b && !IsFlushRight::b {
                use MutSliceEndpointsWhenBothNotFlush::*;
                let lr = IsFlushRight::assert_false_unwrap_ref(IsFlushLeft::assert_false_unwrap_ref(&self.left_right));
                if let ShrankBoth(Some(l),_) | ShrankLeft(Some(l), _) = lr {
                    debug_assert!(l.as_ref().next_single_left_to_right_template::<True>().is_some())
                }
                if let ShrankBoth(_,Some(r)) | ShrankRight(_,Some(r)) = lr {
                    debug_assert!(r.as_ref().next_single_left_to_right_template::<False>().is_some())
                }
            }
            debug_assert!(self.to_imm().map_or(true, |i| i.debug_check_structural_integrity_orig()));
            true
        }
    }
}

/// The struct responsible for most mutable views into a [`FoldChain`].
pub struct MutFoldChainSliceStruct<'a,
    IsReversed: Bool,
    IsFlushLeft: Bool,
    IsFlushRight: Bool,
    T,
    D: Clone, 
    Settings: FoldSettings<T,D> + 'a,
    Simplification: FoldSimplification<T,D> + 'a> {
        pub(crate) endpoints: MutSliceEndpoints<'a,IsFlushLeft,IsFlushRight,Settings,T,D>,
        pub(crate) simplification: Simplification,
        pub(crate) _m: PhantomData<IsReversed>
}

impl<'a, T, D: Clone, Settings: FoldSettings<T,D> + 'a> MutFoldChainSliceStruct<'a, False, True, True, T, D, Settings,()> {
    fn new_from(chain: &'a mut FoldChain<T,D,Settings>) -> Self {
        Self {
            endpoints: MutSliceEndpoints { 
                left_right: (),
                base: NonNull::from(chain),
                _m: PhantomData, 
            },
            simplification: (),
            _m: PhantomData,
        }
    }
}

impl<'a, IsReversed: Bool, IsFlushLeft: Bool, IsFlushRight: Bool, T, D: Clone, Settings: FoldSettings<T,D> + 'a, Simplification: FoldSimplification<T,D> + 'a> 
FoldChainSlice<'a,T,Simplification::D2> for MutFoldChainSliceStruct<'a, IsReversed, IsFlushLeft, IsFlushRight, T, D, Settings, Simplification> {
    type OriginalD = D;
    type IsReversed = IsReversed;
    type IsFlushLeft = IsFlushLeft;
    type IsFlushRight = IsFlushRight;
    type Simplification = Simplification;
    type Settings = Settings;

    fn get_settings(&self) -> Self::Settings {
        unsafe { self.endpoints.base.as_ref().settings }
    }

    fn get_current_simplification(&self) -> Self::Simplification {
        self.simplification.clone()
    }

    fn as_imm(self) -> ImmFoldChainSliceStruct<'a,Self::IsReversed,Self::IsFlushLeft,Self::IsFlushRight,Self::Settings,Self::Simplification,T,Self::OriginalD> {
        ImmFoldChainSliceStruct { 
            settings: self.get_settings(), 
            endpoints: self.endpoints.to_imm(), 
            simplification: self.simplification,
            _m: PhantomData 
        }
    }

    fn borrow<'b>(&'b self) -> ImmFoldChainSliceStruct<'b,Self::IsReversed,Self::IsFlushLeft,Self::IsFlushRight,Self::Settings,Self::Simplification,T,Self::OriginalD> {
        unsafe {
            ImmFoldChainSliceStruct { 
                settings: self.endpoints.base.as_ref().settings, 
                endpoints: self.endpoints.to_imm(), 
                simplification: self.simplification.clone(),
                _m: PhantomData 
            }
        }
    }

    fn debug_check_structural_integrity(&self) -> bool {
        debug_assert!(self.endpoints.debug_check_structural_integrity());
        true
    }
}

impl<'a, IsReversed: Bool, IsFlushLeft: Bool, IsFlushRight: Bool, T, D: Clone, Settings: FoldSettings<T,D> + 'a, Simplification: FoldSimplification<T,D> + 'a> 
MutFoldChainSlice<'a,T,Simplification::D2> for MutFoldChainSliceStruct<'a, IsReversed, IsFlushLeft, IsFlushRight, T, D, Settings, Simplification> {
    fn as_mut(self) -> MutFoldChainSliceStruct<'a,Self::IsReversed,Self::IsFlushLeft,Self::IsFlushRight,T,Self::OriginalD,Self::Settings,Self::Simplification> {
        self
    }
    
    fn borrow_mut<'b>(&'b mut self) -> MutFoldChainSliceStruct<'b, IsReversed, IsFlushLeft, IsFlushRight, T, D, Settings, Simplification> {
        MutFoldChainSliceStruct {
            endpoints: self.endpoints.borrow_mut(),
            simplification: self.simplification.clone(),
            _m: PhantomData,
        }
    }

    fn mut_view_drop_left_until(self, predicate: impl Fn(&Simplification::D2)->bool) -> MutFoldChainSliceStruct<'a, IsReversed, <IsFlushLeft as Bool>::And<IsReversed>, <IsFlushRight as Bool>::And<<IsReversed as Bool>::Not>, T, D, Settings, Simplification> {
        MutFoldChainSliceStruct {
            endpoints: self.endpoints.drop_left_until_template(self.simplification.clone(), predicate),
            simplification: self.simplification,
            _m: PhantomData,
        }
    }

    fn mut_view_take_right_until(self, predicate: impl Fn(&Simplification::D2)->bool) -> MutFoldChainSliceStruct<'a, IsReversed, <IsFlushLeft as Bool>::And<IsReversed>, <IsFlushRight as Bool>::And<<IsReversed as Bool>::Not>, T, D, Settings, Simplification> {
        MutFoldChainSliceStruct {
            endpoints: self.endpoints.take_right_until_template(self.simplification.clone(), predicate),
            simplification: self.simplification,
            _m: PhantomData,
        }
    }

    fn mut_view_reversed(self) -> MutFoldChainSliceStruct<'a, <IsReversed as Bool>::Not, IsFlushLeft, IsFlushRight, T, D, Settings, Simplification> {
        MutFoldChainSliceStruct {
            endpoints: self.endpoints,
            simplification: self.simplification,
            _m: PhantomData,
        }
    }

    fn mut_view_with_simplification<NewSimplification: FoldSimplification<T,Simplification::D2>>(self, new_simplification: NewSimplification) -> MutFoldChainSliceStruct<'a,Self::IsReversed,Self::IsFlushLeft,Self::IsFlushRight,T,Self::OriginalD,Self::Settings,
            NewSimplification::ComposeAfterOther<Self::OriginalD,Self::Simplification>> {
        MutFoldChainSliceStruct { 
            endpoints: self.endpoints, 
            simplification: new_simplification.compose_after_other(self.simplification), 
            _m: PhantomData
        }
    }

    fn mut_view_unsimplify(self) -> MutFoldChainSliceStruct<'a, IsReversed, IsFlushLeft, IsFlushRight, T, D, Settings, ()> {
        MutFoldChainSliceStruct {
            endpoints: self.endpoints,
            simplification: (),
            _m: PhantomData,
        }
    }

    fn pop_left(&mut self) -> Option<T> {
        self.endpoints.pop_left_template::<IsReversed>()
    }

    fn append_left(&mut self, value: T) {
        self.endpoints.push_left_template::<IsReversed>(value);
    }

    fn set_left_or_err(&mut self, value: T) -> Result<T,T> {
        self.endpoints.set_left_or_err_template::<IsReversed>(value)
    }

    fn update_left<R>(&mut self, f: impl FnOnce(Option<&mut T>) -> R) -> R {
        self.endpoints.update_left_template::<IsReversed,_>(f)
    }

    fn take_all(&mut self) -> FoldChain<T,Self::OriginalD,Self::Settings> {
        self.endpoints.take_all_template()
    }

    fn append_all_right(&mut self, chain: FoldChain<T,Self::OriginalD,Self::Settings>) {
        self.endpoints.append_all_right_template::<IsReversed>(chain);
    }
    
    fn foreach_mut(&mut self, f: impl FnMut(&mut T)) {
        self.endpoints.foreach_mut_template::<IsReversed>(f);
    }
}

impl<T: Clone, D: Clone, Settings: FoldSettings<T,D>> Clone for FoldChain<T, D, Settings> {
    fn clone(&self) -> Self {
        let root2 = self.root.as_ref().map(|r| r.clone_boxed());
        Self { 
            leftmost_node_ptr: root2.as_ref().map(|r| r.all_the_way_left_template::<False>().into()), 
            rightmost_node_ptr: root2.as_ref().map(|r| r.all_the_way_left_template::<True>().into()), 
            root: root2, 
            settings: self.settings.clone() 
        }
    }
}

/// An iterator that emits immutable references to the elements in a [FoldChainSlice].
pub struct Iter<'a,REVERSED: Bool, T: 'a, D: Clone + 'a> {
    next_and_next_back: Option<(NonNull<WAVLNode<T,D>>,NonNull<WAVLNode<T,D>>)>,
    _m: PhantomData<(REVERSED,&'a T)>
}

impl<'a,REVERSED: Bool,T, D: Clone> Iter<'a,REVERSED, T, D> {
    fn next_template<ReversedAgain: Bool>(&mut self) -> Option<&'a T> {
        unsafe {
            let (next,next_back) = self.next_and_next_back.as_mut()?;
            let (n,nb) = cswap::<ReversedAgain,_>(next, next_back);
            if n == nb {
                return Some(&core::mem::take(&mut self.next_and_next_back).unwrap().0.as_ref().value)
            }
            let Some(after_n) = n.as_ref().next_single_left_to_right_template::<ReversedAgain>() else {unreachable!()};
            Some(&core::mem::replace(n,after_n).as_ref().value)
        }
    }
}

impl<'a,REVERSED: Bool, T, D: Clone> Iterator for Iter<'a,REVERSED, T, D> {
    type Item = &'a T;
    fn next(&mut self) -> Option<Self::Item> {
        self.next_template::<REVERSED>()
    }
}

impl<'a, REVERSED: Bool, T: 'a, D: Clone + 'a> FusedIterator for Iter<'a, REVERSED, T, D> {}
impl<'a, REVERSED: Bool, T: 'a, D: Clone + 'a> DoubleEndedIterator for Iter<'a, REVERSED, T, D> {
    fn next_back(&mut self) -> Option<Self::Item> {
        self.next_template::<REVERSED::Not>()
    }
}

/// An iterator which removes elements from the left of a [`MutFoldChainSlice`] as it emits them.
/// 
/// Can also remove from the right with [`DoubleEndedIterator::next_back`].
pub struct Drain<'a,T: 'a,D: Clone + 'a, Slice: MutFoldChainSlice<'a,T,D>>{
    slice: Slice,
    _m: PhantomData<&'a mut (T,D)>
}
impl<'a, T: 'a, D: Clone + 'a, Slice: MutFoldChainSlice<'a,T,D>> Iterator for Drain<'a, T, D, Slice> {
    type Item = T;
    fn next(&mut self) -> Option<Self::Item> {
        self.slice.pop_left()
    }
}
impl<'a, T: 'a, D: Clone + 'a, Slice: MutFoldChainSlice<'a,T,D>> DoubleEndedIterator for Drain<'a, T, D, Slice> {
    fn next_back(&mut self) -> Option<Self::Item> {
        self.slice.pop_right()
    }
}
impl<'a, T: 'a, D: Clone + 'a, Slice: MutFoldChainSlice<'a,T,D>> FusedIterator for Drain<'a, T, D, Slice> {}

/// An wrapper around a [`FoldChain`] which makes it act as an iterator.
/// 
/// [`next`](Iterator::next) calls [`MutFoldChainSlice::pop_left`] and [`next_back`](DoubleEndedIterator::next_back) calls [`MutFoldChainSlice::pop_right`].
pub struct DrainOwned<T,D: Clone,Settings: FoldSettings<T,D>>(pub FoldChain<T,D,Settings>);
impl<T, D: Clone, Settings: FoldSettings<T,D>> Iterator for DrainOwned<T, D, Settings> {
    type Item = T;
    fn next(&mut self) -> Option<Self::Item> {
        self.0.pop_left()
    }
}
impl<T, D: Clone, Settings: FoldSettings<T,D>> DoubleEndedIterator for DrainOwned<T, D, Settings> {
    fn next_back(&mut self) -> Option<Self::Item> {
        self.0.pop_right()
    }
}
impl<T, D: Clone, Settings: FoldSettings<T,D>> FusedIterator for DrainOwned<T, D, Settings> {}

impl<T, D: Clone, Settings: FoldSettings<T,D>> IntoIterator for FoldChain<T, D, Settings> {
    type Item = T;
    type IntoIter = DrainOwned<T,D,Settings>;
    fn into_iter(self) -> Self::IntoIter {
        DrainOwned(self)
    }
}

impl<'a,T, D: Clone, Settings: FoldSettings<T,D>> IntoIterator for &'a FoldChain<T, D, Settings> {
    type Item = &'a T;
    type IntoIter = Iter<'a,False,T,D>;
    fn into_iter(self) -> Self::IntoIter {
        let Some(l) = self.leftmost_node_ptr else {
            return Iter { next_and_next_back: None, _m: PhantomData }
        };
        let Some(r) = self.rightmost_node_ptr else {unreachable!()};
        Iter {
            next_and_next_back: Some((l,r)),
            _m: PhantomData,
        }
    }
}

impl<'a, IsReversed: Bool, IsFlushLeft: Bool, IsFlushRight: Bool, Settings: FoldSettings<T,D> + 'a, Simplification: FoldSimplification<T,D> + 'a, T: 'a, D: Clone + 'a> 
IntoIterator for ImmFoldChainSliceStruct<'a, IsReversed, IsFlushLeft, IsFlushRight, Settings, Simplification, T, D> {
    type Item = &'a T;
    type IntoIter = Iter<'a,IsReversed,T,D>;
    fn into_iter(self) -> Self::IntoIter {
        Iter {
            next_and_next_back: self.endpoints.map(|ImmSliceEndpoints { left, right, root: _ }| (left,right)),
            _m: PhantomData,
        }
    }
}

impl<'a, 'b, IsReversed: Bool, IsFlushLeft: Bool, IsFlushRight: Bool, Settings: FoldSettings<T,D> + 'a, Simplification: FoldSimplification<T,D> + 'a, T: 'a, D: Clone + 'a> 
IntoIterator for &'b ImmFoldChainSliceStruct<'a, IsReversed, IsFlushLeft, IsFlushRight, Settings, Simplification, T, D> {
    type Item = &'b T;
    type IntoIter = Iter<'b,IsReversed,T,D>;
    fn into_iter(self) -> Self::IntoIter {
        self.borrow().into_iter()
    }
}

impl<'a, 'b, IsReversed: Bool, IsFlushLeft: Bool, IsFlushRight: Bool, T, D: Clone, Settings: FoldSettings<T,D> + 'a, Simplification: FoldSimplification<T,D> + 'a> 
IntoIterator for &'b MutFoldChainSliceStruct<'a, IsReversed, IsFlushLeft, IsFlushRight, T, D, Settings, Simplification> {
    type Item = &'b T;
    type IntoIter = Iter<'b,IsReversed,T,D>;
    fn into_iter(self) -> Self::IntoIter {
        self.borrow().into_iter()
    }
}

impl<T: core::fmt::Debug, D: Clone, Settings: FoldSettings<T,D>> 
core::fmt::Debug for FoldChain<T, D, Settings> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_list().entries(self.iter()).finish()
    }
}

impl<'a, IsReversed: Bool, IsFlushLeft: Bool, IsFlushRight: Bool, Settings: FoldSettings<T,D> + 'a, Simplification: FoldSimplification<T,D> + 'a, T: core::fmt::Debug + 'a, D: Clone + 'a> 
core::fmt::Debug for ImmFoldChainSliceStruct<'a, IsReversed, IsFlushLeft, IsFlushRight, Settings, Simplification, T, D> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_list().entries(self.iter()).finish()
    }
}

impl<'a, IsReversed: Bool, IsFlushLeft: Bool, IsFlushRight: Bool, T: core::fmt::Debug, D: Clone, Settings: FoldSettings<T,D> + 'a, Simplification: FoldSimplification<T,D> + 'a> 
core::fmt::Debug for MutFoldChainSliceStruct<'a, IsReversed, IsFlushLeft, IsFlushRight, T, D, Settings, Simplification> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_list().entries(self.iter()).finish()    
    }
}