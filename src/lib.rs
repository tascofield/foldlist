#![warn(missing_docs)]


//! [`FoldList`](fold_list::FoldList) is a list-like data structure, stored as a tree, where the following operations are all *O*(log(n)) in the worst case:
//! - Folding (in a specific pre-defined way)
//! - View Slicing
//! - Insertion
//! - Deletion
//! - Remove all
//! - Insert all
//! 
//! A `FoldList<T,D,Settings>` is parameterized by three types:
//! - `T`, the type of the elements of the list.
//! - `D`, the type of folds of ranges of the elements. Can be called the "delta" type because it represents how much an element, or range of elements, changes a fold.
//! - `Settings`, a struct of closures which tells the `FoldList` how to perform the folds. Will ideally be a [ZST](https://doc.rust-lang.org/nomicon/exotic-sizes.html#zero-sized-types-zsts).
//! 
//! # Example
//! 
//! Suppose you have a [`Vec`] of [`String`]s and you want to find the sum of their lengths. Normally you would write something like this:
//! 
//! ```
//! strings_vec.iter().fold(0, |sum,str| sum + str.len())
//! ```
//! To make a new FoldList that does this same fold, you would write:
//! 
//! ```
//! let fold_list = FoldList::new(|a,b| a+b, |str: &String| str.len(),|| 0);
//! ```
//! Thereafter, ```fold_list.fold()``` will return the fold of the contents of this list, including subsequent modifications. It will currently return 0, because the list is empty.
//! 
//! The type of `fold_list` is [`FoldList<String,usize,_>`](fold_list::FoldList), where `_` is some [`FoldSettings`](fold_settings::FoldSettings) type that uses the three closures' types.
//! 
//! ## Fold Settings
//! 
//! The three closures that make up the `Settings` type for this list are used as follows:
//! - `OP`, of type ```impl Fn(D,D)->D```, tells the list how to combine two deltas. The "combining `OP`eration"
//!   *  In this example, the total length of two adjacent ranges of strings is just the sum of the two ranges' total lengths, hence ```|a,b| a+b```
//!   *  It is required that this operation be [associative](https://en.wikipedia.org/wiki/Associative_property), i.e. that, for any `a`,`b`,`c`: `op(a,op(b,c)) = op(op(a,b),c)`. If this isn't the case, then folds may be inconsistent.
//!   *  The operation does **_NOT_** need to be [commutative](https://en.wikipedia.org/wiki/Commutative_property) (though it can be, as in the example). The order of elements will be correctly kept track of in all folds.
//! - `DeltaOf`, of type ```impl Fn(&T)->D```, tells the list how to get the delta of an element
//!   *  In this example, the total length of a single string is just its length, hence ```|str: &String| str.len()```
//! - `Empty`, of type ```impl Fn()->D```, tells the list what the delta of an empty list should be
//!   *  In this example, the total length of a list of zero strings is 0, hence ```|| 0```
//!   *  It's a closure instead of just a value so that Settings can be a ZST more often.
//!   *  It is required that the empty delta be both a [left-identity and a right-identity](https://en.wikipedia.org/wiki/Identity_element) with respect to `OP`: Let's call its output `0`. Then, for any `a`, it must be the case that `op(0,a) = op(a,0) = a`.
//!       - If this isn't the case, then folds may be inconsistent.
//!       - If you can't find an element with this property for type `D`, consider using [`Option<D>`] instead of `D`. Then your empty element can be [`None`]. You can rework `OP` for this yourself, or use [`new_with_opt`](fold_list::FoldList::new_with_opt).
//! 
//! 
//! 
//! The three closures should behave consistently, and must all be [`Copy`].  
//! It's possible for the closures to have nameable types; see [Nameable Type](crate#nameable-type).  
//! 
//! The claim that all the operations on this data structure are *O*(log(n)) relies on the closures all being *O*(1). 
//! If they're instead some *O*(M) then the operations will be *O*(M*log(n)). 
//! For example, if `D` is [`im::HashSet<T>`](https://docs.rs/im/latest/im/struct.HashSet.html) and `OP` is set union, which is *O*(log(n)), then operations which depend on folds (or update them) will be *O*(log(n)<sup>2</sup>). 
//! The real guarantee is that each closure will only be called *O*(log(n))-many times.
//! 
//! # Views
//! 
//! [`FoldList`](fold_list::FoldList)s are interacted with using views. A view into a FoldList, also called a slice, is anything that implements [`FoldListSlice`](fold_list::FoldListSlice). 
//! A mutable slice is one that also implements [`MutFoldListSlice`](fold_list::MutFoldListSlice). Immutable and mutable views work similarly to rust's ```&[T]``` and ```&mut [T]``` types.
//! Every ```&FoldList``` is an immutable slice, and every ```&mut FoldList``` is a mutable one, though views obtained via view operations will have more complicated types.
//! 
//! Functions which start with `view_` or `mut_view_` will always only affect your view into the list; never the actual underlying data. 
//! For example, [`mut_view_drop_left`](fold_list::MutFoldListSlice::mut_view_drop_left) won't actually remove any elements from the list. It will only consume the current view and return a new one that includes fewer elements. 
//! To actually remove a range of elements from the list, you would instead use [`take_all`](fold_list::MutFoldListSlice::take_all), Or for just one element, [`pop_left`](fold_list::MutFoldListSlice::pop_left).
//! 
//! All functions that return new views will consume the previous ones. 
//! If you want to use the previous view after being done with the new one, consider borrowing it first, using [`borrow`](fold_list::FoldListSlice::borrow) or [`borrow_mut`](fold_list::MutFoldListSlice::borrow_mut)[^a]. These are the only exceptions to that rule.
//! 
//! [^a]: This can't be done automatically with [`core::ops::Deref`], or with [`core::borrow::Borrow`], since both of them require the result to be a reference, whereas not all `FoldListSlice`s are treated by rust as references (though they should be, conceptually).
//! 
//! Changes to borrowed views will always be reflected in their originals, including insertions and deletions, even if the original or the borrowed view was empty. 
//! 
//! If an element of the list is not included in a view, no combination of operations can make it included again. So a borrowed view can never outgrow its original. It will always be the same size, or smaller.
//! 
//! Only one mutable slice can be used at once per underlying list; `FoldList`s don't have an analogue of [`slice::split_at_mut`](https://doc.rust-lang.org/std/primitive.slice.html#method.split_at_mut). 
//! If you want to modify two different parts of a `FoldList` at once, you should instead [`take_all`](fold_list::MutFoldListSlice::take_all) of one of them into a different `FoldList`, and then [`append_all_left`](fold_list::MutFoldListSlice::append_all_left) (or [`_right`](fold_list::MutFoldListSlice::append_all_right)) it back later.
//! These two operations are both *O*(log(n)).
//! 
//! You can convert a mutable slice into an immutable one with [as_imm](fold_list::FoldListSlice::as_imm), or immutably borrow from one with [borrow](fold_list::FoldListSlice::borrow), but you can never get a mutable slice from an immutable one.
//! 
//! Immutable slices are [`Clone`].
//! 
//! # Slicing
//! 
//! Slicing can be done based on amounts or on folds. 
//! 
//! For amounts, it works like one would expect: [`view.mut_view_drop_left(3)`](fold_list::MutFoldListSlice::mut_view_drop_left) will return a new view that includes all but the 3 leftmost elements of the previous one. 
//! 
//! Slicing can also be done by performing a binary search on successive folds. For example:
//! 
//! ```
//! let mut fold_list = FoldList::from_iter(
//!    |a,b| a+b,
//!    |str| str.len(),
//!    || 0,
//!    ["a","hi","wxyz","bye","syzygy"].into_iter(),
//! );
//! 
//! let list_view = &mut fold_list;
//! println!("{:?}",list_view); // ["a","hi","xyz","l","syzygy"]
//! 
//! let list_slice = list_view.mut_view_take_left_until(|length: &usize| *length > 5);
//! println!("{:?}",list_slice); // ["a","hi"]
//! ```
//! 
//! This looks at the folds that start on the left, and finds the longest one whose contents' total length doesn't exceed 5. This results in `["a","hi"]` because its total length is 3, but `["a","hi","wxyz"]`'s total length is 7. Then it _takes_ this range, that is, restricts the view to be only this range (as opposed to dropping it, which would restrict the view to everything _but_ the range).
//! 
//! Notice how the search criteria was a caller-defined closure. The caller can use any predicate they want, as long as it behaves well under binary search.
//! 
//! The following properties of a predicate ```p(d: &D) -> bool``` are all equivalent to it being well-behaved:
//!   * Once a `D` satisfies `p`, there is no other `D` that you can combine it with using `op` that will make it go back to not satisfying `p`.
//!   * If you mapped `p` over the successive folds, the result would look something like ```[false,false,true,true,true]```. That is, there is some way you can cut the list in half such that the first half is all false and the other half is all true.
//!   * There is some quantity that `D`'s have that can never be decreased through `op`, and `p` is checking whether that quantity meets a certain threshold.
//! 
//! If the predicate isn't well behaved, then the result will be a cut at an arbitrary one of the many points at which the fold goes from ```false``` to ```true``` (if there aren't multiple such points, then the predicate _is_ well-behaved, if not in general then at least in this instance[^b]). 
//! [^b]: With the expection of patterns like ```[true,true,false,true,true]```, where there is only one such point, but the predicate is still not well-behaved. You can think of the empty fold as never satisfying any predicate, so there's always secretly a false at the beginning, i.e. ```[false,true,true,false,true,true]``` (It is not actually required that your predicate return `false` for the empty fold, though. This is true even if it doesn't). So in this case the returned slice point might also be at the beginning.
//! 
//! If the predicate always returns false, the range in question will be the entire list. If it always returns true, the range will be the empty range on the side that the folds started from.
//! 
//! A slicing performed this way can either start from the left, or from the right, and it can either take all the elements in question, or it can drop them, which altogether is 4 different possibilities.
//! Here's a table of how they all behave for the predicate ```|length: &usize| *length > 6``` on the earlier example of ```["a","hi","wxyz","l","syzygy"]```:
//! |Name                                                                                 |Take/Drop|Start|Result                     |
//! |-------------------------------------------------------------------------------------|---------|-----|---------------------------|
//! |[`mut_view_take_left_until`](fold_list::MutFoldListSlice::mut_view_take_left_until)  |Take     |Left |```["a","hi"]```           |
//! |[`mut_view_take_right_until`](fold_list::MutFoldListSlice::mut_view_take_right_until)|Take     |Right|```["syzygy"]```           |
//! |[`mut_view_drop_left_until`](fold_list::MutFoldListSlice::mut_view_drop_left_until)  |Drop     |Left |```["wxyz","l","syzygy"]```|
//! |[`mut_view_drop_right_until`](fold_list::MutFoldListSlice::mut_view_drop_right_until)|Drop     |Right|```["a","hi","wxyz","l"]```|
//! 
//! You can also use range syntax to more ergonomically specify where to start from, with [`mut_view_take`](fold_list::MutFoldListSlice::mut_view_take) and [`mut_view_drop`](fold_list::MutFoldListSlice::mut_view_drop):
//! |Expression                                               |Result                     |
//! |---------------------------------------------------------|---------------------------|
//! |```fold_list.mut_view_take(..\|l: &usize\| *l > 6))   ```|```["a","hi"]```           |
//! |```fold_list.mut_view_take( (\|l: &usize\| *l > 6).. )```|```["syzygy"]```           |
//! |```fold_list.mut_view_drop(..\|l: &usize\| *l > 6))   ```|```["wxyz","l","syzygy"]```|
//! |```fold_list.mut_view_drop( (\|l: &usize\| *l > 6).. )```|```["a","hi","wxyz","l"]```|
//! 
//! This works with either [`RangeFrom<P>`](core::ops::RangeFrom) or [`RangeTo<P>`](core::ops::RangeTo), where `P` is your predicate.
//! 
//! Each pair of corresponding Left and Right slicing functions are exact mirror images of each other. That is, running one is equivalent to [reversing](fold_list::MutFoldListSlice::mut_view_reversed), running the other, then reversing the result. 
//! 
//! For example, for any predicate `p`, ```list.mut_view_drop_right_until(p)``` will always be the same as ```list.mut_view_reversed().mut_view_drop_left_until(p).mut_view_reversed()```.
//! 
//! # Insertion
//! 
//! Elements can be [inserted at](fold_list::MutFoldListSlice::insert_at) specific indices: 
//! 
//! ```
//! list.insert_at(3,"hello");
//! //inserts "hello" between the elements formerly at indicies 2 and 3, so that its new index is 3
//! //and all elements after it have their indices increased by 1
//! ```
//! 
//! Or at [either](fold_list::MutFoldListSlice::append_left) [end](fold_list::MutFoldListSlice::append_right):
//! 
//! ```
//! list.append_left("left");
//! list.append_right("right");
//! ```
//! 
//! Inserting at the ends is somewhat faster, because under the hood, inserting in the middle is just taking a slice, and then inserting at the end of the slice.
//! 
//! # Deletion
//! 
//! Similarly, elements can be [removed at](fold_list::MutFoldListSlice::remove_at) specific indices: 
//! 
//! ```
//! let removed : T = list.remove_at(3); 
//! //panics if the index was out of bounds
//! ```
//! 
//! Or at [either](fold_list::MutFoldListSlice::pop_left) [end](fold_list::MutFoldListSlice::pop_right):
//! 
//! ```
//! let left_unless_list_was_empty : Option<T> = list.pop_left();
//! ```
//! ```
//! let right_unless_list_was_empty : Option<T> = list.pop_right(); 
//! ```
//! 
//! Deleting at the ends is also faster, for the same reason.
//! 
//! # Mutation / Indexing
//! 
//! Unlike [`Vec`], [`FoldList`](fold_list::FoldList) doesn't allow you to borrow direct mutable references to its elements, because if you could, then you could change its folds without it noticing[^c].
//! So instead of being able to do this:
//! 
//! [^c]: You are still technically able to do this, by using a type that lets you mutate it via an immutable reference, such as [`RefCell`](std::cell::RefCell) or an atomic type. But you shouldn't, or else folds might be inaccurate.
//! 
//! ```
//! let x = my_vec[10].fn_that_mutates(); 
//! //secretly makes a mutable reference
//! //you can't do this with a foldlist
//! ```
//! 
//! You have to do [this](fold_list::MutFoldListSlice::update_at):
//! 
//! ```
//! let x = my_foldlist.update_at(10, |t: &mut T| t.fn_that_mutates());
//! ```
//! 
//! But if you don't need a mutable reference to the element, you can still do this:
//! 
//! ```
//! let x = my_foldlist[10].fn_that_does_not_mutate();
//! ```
//! 
//! You can also set/replace elements:
//! 
//! ```
//! let former_value = my_foldlist.set_at(3, "yeah"); //works like mem::replace
//! ```
//! 
//! Versions of all of these exist for mutating just the leftmost and rightmost elements as well.
//! 
//! # Reverse
//! 
//! Views can be reversed, which works like you would expect:
//! 
//! ```
//! let mut fold_list = FoldList::from_iter(
//!     |a: String,b: String | a + &b,
//!     |str| str.clone(),
//!     || String::new(),
//!     ["a","hi","wxyz","l","syzygy"].into_iter().map(|str|String::from(str)),
//! );
//! let list_view = &mut fold_list;
//! println!("{:?}",list_view); //["a", "hi", "wxyz", "l", "syzygy"]
//! let rev_view = list_view.mut_view_reversed();
//! println!("{:?}",rev_view); //["syzygy", "l", "wxyz", "hi", "a"]
//! ```
//! 
//! The resulting reversed view works in all the expected ways, with the exception of:
//!  * Folding: reversing a view doesn't affect its folds
//!      - For example, the above view's fold (via string concatenation, which isn't commutative) is still ```"ahiwxyzlsyzygy"```, and not ```"syzygylwxyzhia"```, as one might expect.
//!      - Another way of looking at it is that reversing a view also reverses its operation, so `op_reversed(a,b) = op(b,a)`.
//!  * [`take_all`](fold_list::MutFoldListSlice::take_all): the resulting list will contain the elements in their original order, not the reversed order.
//!      - For example, in the above case, ```rev_view.mut_view_take_left(3).take_all()``` will be ```["wxyz", "l", "syzygy"]```, and not ```["syzygy", "l", "wxyz"]```.
//!  * `append_all_*`: the list is appended in its original order
//!      - For example, if `list2` is `["one","two","three"]`, then after calling ```rev_view.append_all_left(list2)```, `rev_view` will be ```["three", "two", "one", "syzygy", "l", "wxyz", "hi", "a"]```.
//!      - From the base `FoldList`'s point of view, other lists are never appended in a backwards order, but from a reversed view's perspective, they always are.
//!      - Note that the other list is still appended on the correct side, from the reversed view's perspective.
//! 
//! Those last two sadly mean that `FoldList`s can't be used to efficiently implement [pancake sort](https://en.wikipedia.org/wiki/Pancake_sorting).
//! 
//! # Simplification
//! 
//! A view can have a "simplification" applied to it, which changes the fold that the view performs, to a "simpler" version of the same fold.
//! 
//! For example, suppose that `D`, the type of folds, is ```(String,f64)```, where `OP` independently concatenates the `String`s, and adds the `f64`s.
//! Suppose that you want to know just the sum of the `f64`s in a view:
//! 
//! ```
//! fn my_f64_sum<'a,T: 'a>(view: impl FoldListSlice<'a,T,(String,f64)>) -> f64 {
//!     todo!()
//! }
//! ```
//! 
//! One possible solution would be this:
//! 
//! ```
//! fn my_f64_sum<'a,T: 'a>(view: impl FoldListSlice<'a,T,(String,f64)>) -> f64 {
//!     let (_string_ignored, sum) = view.fold();
//!     sum
//! }
//! ```
//! 
//! And this does give the correct answer, but it's way slower than it needs to be, since it involves performing a bunch of string concatenations and possibly making new ```String```s for each one, and we just throw the result of that away at the end.
//! We want to only perform the part of the fold involving `f64`s, and not the part involving `String`s.
//! 
//! The solution is to apply a [simplification](fold_list::FoldListSlice::view_simplify):
//! 
//! ```
//! fn my_f64_sum<'a,T: 'a>(view: impl FoldListSlice<'a,T,(String,f64)>) -> f64 {
//!     //Assume that the view's operation adds the f64s (This function's type doesn't require this. To do so would overcomplicate this example).
//!     let view_with_simplification = view.view_simplify(
//!         |(_string, float): &(String,f64)| *float, 
//!         |(a,b): (f64,f64)| a+b
//!     );
//!     //simplify the view to just the f64s
//!     view_with_simplification.fold()
//! }
//! ```
//! 
//! This works, and is already much faster. But there's still a way to make it slightly faster.
//! 
//! Currently, when the simplified view wants to know what the identity element is, it makes the unsimplified version of it, then simplifies that.
//! So it creates ```(String::new(),0.0 as f64)```, and then immediately throws the `String` part away. 
//! A similar thing happens when it wants to know what the delta of some `T` is.
//! To avoid this, we need to provide a faster way to do these two things; a ["shortcut"](fold_list::FoldListSlice::view_simplify_with_shortcut):
//! 
//! ```
//! fn my_f64_sum<'a,T: 'a>(view: impl FoldListSlice<'a,T,(String,f64)>) -> f64 {
//!     //Assume same as before.
//!     let view_with_simplification = view.view_simplify_with_shortcut(
//!         |(_string, float): &(String,f64)| *float, 
//!         |(a,b): (f64,f64)| a+b,
//!         //shortcuts below
//!         |()| 0.0,                       //fast identity element
//!         |elem: &T| f64_of_element(elem) //fast delta of an element. In a real-world example this function would know what type T is.
//!     );
//!     view_with_simplification.fold()
//! }
//! ```
//! 
//! In general, the necessary closures for a simplification are as follows:
//!  * `simplifier`, of type ```impl Fn(&D)->D2```. Converts a `D` to its simplified form.
//!    * In the above example, we take just the `f64`, hence ```|(_string, float): &(String,f64)| *float```
//!  * `simplified_op`, of type ```impl Fn((D2,D2)) -> D2```. Describes how the original view's operation behaves under the simplifier.
//!    * In the above example, the original view added the `f64`s, hence ```|(a,b): (f64,f64)| a+b```.
//!    * Needs to act the same way as the original `op`, i.e. it must be the case that, for all `a`, `b`:  
//!      `simplifier(&op(a,b)) = simplified_op(simplifier(&a),simplifier(&b))`. Otherwise, folds may be inconsistent.
//!    * Note that the input is a tuple instead of multiple arguments. This is because its type is actually [`impl Fun<(D2,D2),D2>`](misc::Fun). All of these closures are, in order to allow a simplified slice to have a [nameable type](crate#nameable-type).
//! 
//! And if the simplification includes shortcuts:
//!  * `empty_shortcut`, of type ```impl Fn(())->D2```. A faster way to make a new empty delta.
//!    * In the above example, the `f64` part of the empty delta was `0.0`, hence ```|()| 0.0```.
//!    * It must always have the same result as simplifying the previous empty delta. Otherwise, folds may be inconsistent.
//!    * It has a single input of ```()```, for the same reason as before.
//!  * `delta_shortcut`, of type ```impl Fn(&T)->D2```. A faster way to get the delta of an element.
//!    * In the above example, this would do whatever the previous version did, but without making a new string (the example doesn't specify).
//!    * It must always have the same result as simplifying the previous delta of the element. Otherwise, folds may be inconsistent.
//! 
//! To keep the simplified view mutable, you can use the mutable versions of the two functions; [mut_view_simplify](fold_list::MutFoldListSlice::mut_view_simplify) and [mut_view_simplify_with_shortcut](fold_list::MutFoldListSlice::mut_view_simplify_with_shortcut).
//! 
//! A simplification is actually its own type, similarly to `Settings`, which implements [`FoldSimplification`](fold_simplification::FoldSimplification). 
//! A view's currently-applied simplification can be queried with [`get_current_simplification`](fold_list::FoldListSlice::get_current_simplification), 
//! removed/cleared with [`view_unsimplify`](fold_list::FoldListSlice::view_unsimplify),
//! or explicitly composed/added with [`view_with_simplification`](fold_list::FoldListSlice::view_with_simplification). Mutable versions of all of these also exist.
//! 
//! If a view hasn't been simplified, its current simplification will be ```()```, which is a [`FoldSimplification`](fold_simplification::FoldSimplification) that does nothing.
//! 
//! After a simplification is applied to a view, it will implement ```FoldListSlice<T,D2>```, where `D2` is the new delta type.
//! 
//! # Nameable type
//! 
//! Suppose you want to use a [`FoldList`](fold_list::FoldList) as part of a struct, or return it from a function. What should its type be?
//! 
//! The type of a [`FoldList`](fold_list::FoldList) or [`FoldListSlice`](fold_list::FoldListSlice) will always involve its [`FoldSettings`](fold_settings::FoldSettings), which will involve its closures, 
//! and in rust, each closure has its own type that cannot be named.
//! One solution might be to define your own types for each of the closures, and then implement [`Fn`] for each of them, but this is [currently unstable](https://github.com/rust-lang/rust/issues/29625).
//! 
//! This crate has a workaround: [`Fun`](misc::Fun), a trait which works very similarly to [`Fn`], with notable differences:
//!   * The closure must have only one input (but you can use a tuple instead).
//!   * The output type is part of the trait's type parameters, which means that the same type can implement it it multiple ways, if each way has different output/input types (Whereas `Fn` can theoretically only be implemented once per input type).
//! 
//! All of the closure types inside [`FoldSettingsStruct`](fold_settings::FoldSettingsStruct) can also be [`Fun`](misc::Fun) types[^d].
//! [^d]: This is also true of [`SimplificationWithoutShortcut`](fold_simplification::SimplificationWithoutShortcut) and [`SimplificationWithShortcut`](fold_simplification::SimplificationWithShortcut), so simplified slices can also have named types, though they will be much more complicated.
//! 
//! Here's an example of how you would use [`Fun`](misc::Fun) to make a nameable `FoldList` type, which can then be used in a struct:
//! 
//! ```rust
//! #[derive(Clone,Copy)]
//! struct MyAddingClosure;
//! impl foldlist::misc::Fun<(usize,usize),usize> for MyAddingClosure {
//!     fn apply(&self,a: (usize,usize)) -> usize {
//!         a.0 + a.1
//!     }
//! }
//! 
//! #[derive(Clone,Copy)]
//! struct MyStringLengthClosure;
//! impl foldlist::misc::Fun<&String,usize> for MyStringLengthClosure {
//!     fn apply(&self,a: &String) -> usize {
//!         a.len()
//!     }
//! }
//! 
//! #[derive(Clone,Copy)]
//! struct MyZeroClosure;
//! impl foldlist::misc::Fun<(),usize> for MyZeroClosure {
//!     fn apply(&self,_a: ()) -> usize {
//!         0
//!     }
//! }
//! 
//! pub struct MyStructThatUsesFoldList {
//!     foldlist: FoldList<String,usize,FoldSettingsStruct<String,usize,MyAddingClosure,MyStringLengthClosure,MyZeroClosure>>,
//!     is_cool: bool
//! }
//! 
//! impl MyStructThatUsesFoldList {
//!     fn new() -> Self {
//!         let fold_list_settings = FoldSettingsStruct {
//!             op_closure: MyAddingClosure,
//!             t2d_closure: MyStringLengthClosure,
//!             empty_closure: MyZeroClosure,
//!             _m: std::marker::PhantomData,
//!         };
//!         Self {
//!             foldlist: FoldList::from_settings(fold_list_settings),
//!             is_cool: true,
//!         }
//!     }
//! }
//! ```
//! 
//! # FoldChain
//! 
//! A [`FoldChain`](fold_chain::FoldChain) is like a `FoldList`, except it doesn't keep track of sizes or indices. 
//! A `FoldChain` can do anything that a `FoldList` can do, as long as it doesn't need to know the size (element count) of any folds.
//! 
//! In fact, every `FoldList` is secretly just a `FoldChain` that happens to also keep track of size. 
//! Every `FoldList` with delta type `D` is built on top of a `FoldChain` with delta type `(usize,D)`, where the usize in a fold is the number of elements.
//! This underlying `FoldChain` can be accessed via [`as_sized_chain`](fold_list::FoldListSlice::as_sized_chain), for a slice, or the public [`underlying`](fold_list::FoldList::underlying) field of `FoldList`, for the base list.
//! Conversely, a `FoldChain` of the proper form can be converted back to a `FoldList` via [`as_fold_list`](fold_chain::FoldChain::as_fold_list).
//! 
//! If you never use a `FoldList`'s size information, consider using a `FoldChain` instead.

/// Miscellaneous things used by `FoldChain` and `FoldList`
pub mod misc;

/// The type which tells a `FoldChain` or `FoldList` how to do its fold; see [Fold Settings](crate#fold-settings).
pub mod fold_settings;

/// The type which tells a `FoldChain` or `FoldList` how to simplify its fold; see [Simplification](crate#simplification).
pub mod fold_simplification;

///Like [`fold_list`], but without size information; see [FoldChain](crate#foldchain)
pub mod fold_chain;

///`FoldList`s and slices thereof; See [FoldList](crate).
pub mod fold_list;