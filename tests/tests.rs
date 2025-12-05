#[cfg(test)]
mod vec_based_fold_chain_slice;
use foldlist::{fold_chain::{FoldChain, FoldChainSlice, ImmFoldChainSliceStruct, MutFoldChainSlice, MutFoldChainSliceStruct}, fold_list::{FoldList, FoldListSlice, FoldListSliceFrom, MutFoldListSlice}, fold_settings::{FoldSettings, FoldSettingsStruct, SettingsWithSize}, fold_simplification::FoldSimplification, misc::{Bool,TupleFun}};
use rand::Rng;
use std::{cell::RefCell, fmt::Debug, io::Write, marker::PhantomData, rc::Rc};
use rand::{SeedableRng, rngs::StdRng};

use crate::vec_based_fold_chain_slice::VecBasedFoldChainSlice;


#[allow(dead_code)]
#[test]
fn test() {
    //just the syntax
    fn assert_is_send_and_sync<T: Send + Sync>(){}
    fn for_types<'a,T: 'a + Send + Sync,D: Clone + 'a + Send + Sync,Settings: FoldSettings<T,D> + 'a + Send + Sync, Simplification: FoldSimplification<T,D> + 'a + Send + Sync,
        IsReversed: Bool, IsFlushLeft: Bool, IsFlushRight: Bool>() {
        assert_is_send_and_sync::<FoldChain<T,D,Settings>>();

        assert_is_send_and_sync::<FoldList<T,D,Settings>>();

        assert_is_send_and_sync::<ImmFoldChainSliceStruct<'a,IsReversed,IsFlushLeft,IsFlushRight,Settings,Simplification,T,D>>();

        assert_is_send_and_sync::<MutFoldChainSliceStruct<'a,IsReversed,IsFlushLeft,IsFlushRight,T,D,Settings,Simplification>>();

        assert_is_send_and_sync::<FoldListSliceFrom<'a,T,D,Settings,Simplification,&'a FoldChain<T,(usize,D),SettingsWithSize<Settings>>>>();

        assert_is_send_and_sync::<FoldListSliceFrom<'a,T,D,Settings,Simplification,&'a mut FoldChain<T,(usize,D),SettingsWithSize<Settings>>>>();

        assert_is_send_and_sync::<FoldListSliceFrom<'a,T,D,Settings,Simplification,ImmFoldChainSliceStruct<'a,IsReversed,IsFlushLeft,IsFlushRight,SettingsWithSize<Settings>,(),T,(usize,D)>>>();

        assert_is_send_and_sync::<FoldListSliceFrom<'a,T,D,Settings,Simplification,MutFoldChainSliceStruct<'a,IsReversed,IsFlushLeft,IsFlushRight,T,(usize,D),SettingsWithSize<Settings>,()>>>();
    }
    assert!(true);
}

#[test]
fn test_main_example1() {
    let fold_list = FoldList::new(|a,b| a+b, |str: &String| str.len(),|| 0);
    assert_eq!(fold_list.len(),0);
    assert_eq!(fold_list.fold(),0);
}

#[test]
fn test_main_example2() {
    let mut fold_list = FoldList::from_iter(
        |a,b| a+b,
        |str| str.len(),
        || 0,
        ["a","hi","wxyz","l","syzygy"].into_iter(),
    );

    assert_eq!(fold_list.len(),5);
    assert_eq!(fold_list.fold(),14);

    let list_view = &mut fold_list;
    assert_eq!(list_view.len(),5);
    assert_eq!(list_view.fold(),14);

    assert_eq!(list_view.mut_view_take_left_until(|length: &usize| *length > 5).iter().collect::<Vec<_>>(),vec![&"a",&"hi"]);
    assert_eq!(list_view    .view_take_left_until(|length: &usize| *length > 5).iter().collect::<Vec<_>>(),vec![&"a",&"hi"]);

    assert_eq!(list_view.mut_view_take_left_until( |length: &usize| *length > 6).iter().collect::<Vec<_>>(),vec![&"a",&"hi"]);
    assert_eq!(list_view.mut_view_take_right_until(|length: &usize| *length > 6).iter().collect::<Vec<_>>(),vec![&"syzygy"]);
    assert_eq!(list_view.mut_view_drop_left_until( |length: &usize| *length > 6).iter().collect::<Vec<_>>(),vec![&"wxyz",&"l",&"syzygy"]);
    assert_eq!(list_view.mut_view_drop_right_until(|length: &usize| *length > 6).iter().collect::<Vec<_>>(),vec![&"a",&"hi",&"wxyz",&"l"]);

    assert_eq!(list_view.view_take_left_until( |length: &usize| *length > 6).iter().collect::<Vec<_>>(),vec![&"a",&"hi"]);
    assert_eq!(list_view.view_take_right_until(|length: &usize| *length > 6).iter().collect::<Vec<_>>(),vec![&"syzygy"]);
    assert_eq!(list_view.view_drop_left_until( |length: &usize| *length > 6).iter().collect::<Vec<_>>(),vec![&"wxyz",&"l",&"syzygy"]);
    assert_eq!(list_view.view_drop_right_until(|length: &usize| *length > 6).iter().collect::<Vec<_>>(),vec![&"a",&"hi",&"wxyz",&"l"]);

    assert_eq!(list_view.mut_view_take( ..|length: &usize| *length > 6) .iter().collect::<Vec<_>>(),vec![&"a",&"hi"]);
    assert_eq!(list_view.mut_view_take((|length: &usize| *length > 6)..).iter().collect::<Vec<_>>(),vec![&"syzygy"]);
    assert_eq!(list_view.mut_view_drop(..|length: &usize| *length > 6)  .iter().collect::<Vec<_>>(),vec![&"wxyz",&"l",&"syzygy"]);
    assert_eq!(list_view.mut_view_drop((|length: &usize| *length > 6)..).iter().collect::<Vec<_>>(),vec![&"a",&"hi",&"wxyz",&"l"]);

    assert_eq!(list_view.view_take( ..|length: &usize| *length > 6) .iter().collect::<Vec<_>>(),vec![&"a",&"hi"]);
    assert_eq!(list_view.view_take((|length: &usize| *length > 6)..).iter().collect::<Vec<_>>(),vec![&"syzygy"]);
    assert_eq!(list_view.view_drop(..|length: &usize| *length > 6)  .iter().collect::<Vec<_>>(),vec![&"wxyz",&"l",&"syzygy"]);
    assert_eq!(list_view.view_drop((|length: &usize| *length > 6)..).iter().collect::<Vec<_>>(),vec![&"a",&"hi",&"wxyz",&"l"]);
}

#[test]
fn test_main_example3() {
    //this time op is string concatentation
    let mut fold_list = FoldList::from_iter(
        |a: String,b: String | a + &b,
        |str| str.clone(),
        || String::new(),
        ["a","hi","wxyz","l","syzygy"].into_iter().map(|str|String::from(str)),
    );

    assert_eq!(fold_list.fold(),String::from("ahiwxyzlsyzygy"));

    let mut rev_view = fold_list.mut_view_reversed();

    assert_eq!(rev_view.fold(),String::from("ahiwxyzlsyzygy"));

    assert_eq!(rev_view.iter().collect::<Vec<_>>(),vec!["syzygy","l","wxyz","hi","a"]);

    let mut list2 = FoldList::from_settings(rev_view.get_settings());
    list2.append_right_from_iter(["one","two","three"].into_iter().map(|str|String::from(str)));

    rev_view.append_all_left(list2);

    assert_eq!(rev_view.iter().collect::<Vec<_>>(),vec!["three", "two", "one", "syzygy","l","wxyz","hi","a"]);
}

#[allow(dead_code)]
#[test]
fn test_main_example4() {
    //just the syntax
    fn f64_of_element<T>(_t: &T) -> f64 {todo!()}

    fn my_f64_sum<'a,T: 'a>(view: impl FoldListSlice<'a,T,(String,f64)>) -> f64 {
        //Assume that the view's operation adds the f64s (This function's type doesn't require this. To do so would overcomplicate this example).
        let view_with_simplification = view.view_simplify_with_shortcut(
            |(_string, float): &(String,f64)| *float, 
            |(a,b): (f64,f64)| a+b,
            |()| 0.0, 
            |elem: &T| f64_of_element(elem)
        );
        view_with_simplification.fold()
    }
    assert!(true)
}

#[allow(dead_code)]
#[test]
fn test_main_example5() {
    //just the syntax

    #[derive(Clone,Copy)]
    struct MyAddingClosure;
    impl foldlist::misc::Fun<(usize,usize),usize> for MyAddingClosure {
        fn apply(&self,a: (usize,usize)) -> usize {
            a.0 + a.1
        }
    }

    #[derive(Clone,Copy)]
    struct MyStringLengthClosure;
    impl foldlist::misc::Fun<&String,usize> for MyStringLengthClosure {
        fn apply(&self,a: &String) -> usize {
            a.len()
        }
    }

    #[derive(Clone,Copy)]
    struct MyZeroClosure;
    impl foldlist::misc::Fun<(),usize> for MyZeroClosure {
        fn apply(&self,_a: ()) -> usize {
            0
        }
    }

    pub struct MyStructThatUsesFoldList {
        foldlist: FoldList<String,usize,FoldSettingsStruct<String,usize,MyAddingClosure,MyStringLengthClosure,MyZeroClosure>>,
        is_cool: bool
    }

    impl MyStructThatUsesFoldList {
        fn new() -> Self {
            let fold_list_settings = FoldSettingsStruct {
                op_closure: MyAddingClosure,
                t2d_closure: MyStringLengthClosure,
                empty_closure: MyZeroClosure,
                _m: std::marker::PhantomData,
            };
            Self {
                foldlist: FoldList::from_settings(fold_list_settings),
                is_cool: true,
            }
        }
    }
}


#[test]
fn test_fl_mut_with_strings() {
    let mut rng = StdRng::seed_from_u64(435);
    let settings: FoldSettingsStruct<char,String,_,_,_> = FoldSettingsStruct {
        op_closure: TupleFun(|a: String, b: String| a + &b),
        t2d_closure: |c: &char| c.to_string(),
        empty_closure: |()| String::new(),
        _m: PhantomData,
    };
    let mut a = VecBasedFoldChainSlice { 
        vec: Rc::new(RefCell::new(Vec::new())), 
        start_inc: 0, 
        end_exc: 0, 
        parent_end_exc_ptrs: Vec::new(), 
        is_reversed: false, 
        settings: settings, 
        simplification: (), 
        _m: PhantomData
    };
    let mut b = FoldList::from_settings(settings);
    fl_mut_do_tests(&mut rng, &mut 0, &mut 20000, a.borrow_mut(), &mut b, |a: &String, b: &String| a.len() as isize - b.len() as isize, |rng| rand_char(rng));

    let a_simp = a.borrow_mut().mut_view_simplify(|s: &String| s.len(), |(a,b)| a + b);
    let b_simp = b.mut_view_with_simplification(a_simp.simplification);
    fl_mut_do_tests(&mut rng, &mut 20000, &mut 20000, a_simp, b_simp, |a,b| *a as isize - *b as isize,|rng| rand_char(rng));
}

#[test]
fn test_fc_mut_with_strings() {
    let mut rng = StdRng::seed_from_u64(768);
    let settings: FoldSettingsStruct<char,String,_,_,_> = FoldSettingsStruct {
        op_closure: TupleFun(|a: String, b: String| a + &b),
        t2d_closure: |c: &char| c.to_string(),
        empty_closure: |()| String::new(),
        _m: PhantomData,
    };
    let mut a = VecBasedFoldChainSlice { 
        vec: Rc::new(RefCell::new(Vec::new())), 
        start_inc: 0, 
        end_exc: 0, 
        parent_end_exc_ptrs: Vec::new(), 
        is_reversed: false, 
        settings: settings, 
        simplification: (), 
        _m: PhantomData
    };
    let mut b = FoldChain::from_settings(settings);
    fc_mut_do_tests(&mut rng, &mut 0, &mut 10000, a.borrow_mut(), &mut b, |a: &String, b: &String| a.len() as isize - b.len() as isize, |rng| rand_char(rng));

    let a_simp = a.borrow_mut().mut_view_simplify(|s: &String| s.len(), |(a,b)| a + b);
    let b_simp = b.mut_view_simplify(|s: &String| s.len(), |(a,b)| a + b);
    fc_mut_do_tests(&mut rng, &mut 10000, &mut 10000, a_simp, b_simp, |a,b| *a as isize - *b as isize,|rng| rand_char(rng));
}

#[test]
fn test_fl_imm_with_strings() {
    let mut rng = StdRng::seed_from_u64(124);
    for len in 1..100 {
        println!("\nlen={}",len);
        if len == 8 {
            let _ = 2+2;
        }
        let str = String::from_iter(std::iter::from_fn(|| Some(rand_char(&mut rng))).take(len));
        let settings: FoldSettingsStruct<char,String,_,_,_> = FoldSettingsStruct {
            op_closure: TupleFun(|a: String, b: String| a + &b),
            t2d_closure: |c: &char| c.to_string(),
            empty_closure: |()| String::new(),
            _m: PhantomData,
        };
        let a: VecBasedFoldChainSlice<char,String,_,()> = VecBasedFoldChainSlice {
            vec: Rc::new(RefCell::new(str.chars().into_iter().collect())),
            start_inc: 0,
            end_exc: str.len(),
            parent_end_exc_ptrs: Vec::new(),
            is_reversed: false,
            settings,
            simplification: (),
            _m: PhantomData,
        };
        let mut b = FoldList::from_settings(settings);
        if rng.random_bool(0.5) {
            b.append_right_from_iter(str.chars().into_iter());
        } else {
            b.append_left_from_iter(str.chars().into_iter().rev());
        }
        fl_imm_do_tests(&mut rng, &mut 0, &mut 1000, a.borrow(), b.borrow(), |a: &String, b: &String| a.len() as isize - b.len() as isize);

        let a_simp = a.borrow().view_simplify(|s: &String| s.len(), |(a,b)| a + b);
        let b_simp = b.view_with_simplification(a_simp.simplification);
        fl_imm_do_tests(&mut rng, &mut 1000, &mut 1000, a_simp, b_simp, |a,b| *a as isize - *b as isize);

        let a_simp_sc = a.view_simplify_with_shortcut(|s: &String| s.len(), |(a,b)| a+b, |()| 0, |_ : &char| 1);
        let b_simp_sc = b.view_unsimplify().view_with_simplification(a_simp_sc.simplification);
        fl_imm_do_tests(&mut rng, &mut 2000, &mut 1000, a_simp_sc, b_simp_sc, |a,b| *a as isize - *b as isize);
    }
}

#[test]
fn test_fc_imm_with_strings() {
    let mut rng = StdRng::seed_from_u64(45867);
    for len in 1..100 {
        println!("\nlen={}",len);
        if len == 8 {
            let _ = 2+2;
        }
        let str = String::from_iter(std::iter::from_fn(|| Some(rand_char(&mut rng))).take(len));
        let settings: FoldSettingsStruct<char,String,_,_,_> = FoldSettingsStruct {
            op_closure: TupleFun(|a: String, b: String| a + &b),
            t2d_closure: |c: &char| c.to_string(),
            empty_closure: |()| String::new(),
            _m: PhantomData,
        };
        let a: VecBasedFoldChainSlice<char,String,_,()> = VecBasedFoldChainSlice {
            vec: Rc::new(RefCell::new(str.chars().into_iter().collect())),
            start_inc: 0,
            end_exc: str.len(),
            parent_end_exc_ptrs: Vec::new(),
            is_reversed: false,
            settings,
            simplification: (),
            _m: PhantomData,
        };
        let mut b = FoldChain::from_settings(settings);
        if rng.random_bool(0.5) {
            b.append_right_from_iter(str.chars().into_iter());
        } else {
            b.append_left_from_iter(str.chars().into_iter().rev());
        }
        fc_imm_do_tests(&mut rng, &mut 0, &mut 1000, a.borrow(), b.borrow(), |a: &String, b: &String| a.len() as isize - b.len() as isize);

        let a_simp = a.borrow().view_simplify(|s: &String| s.len(), |(a,b)| a + b);
        let b_simp = b.view_simplify(|s: &String| s.len(), |(a,b)| a + b);
        fc_imm_do_tests(&mut rng, &mut 1000, &mut 1000, a_simp, b_simp, |a,b| *a as isize - *b as isize);

        let a_simp_sc = a.view_simplify_with_shortcut(|s: &String| s.len(), |(a,b)| a+b, |()| 0, |_ : &char| 1);
        let b_simp_sc = b.view_simplify_with_shortcut(|s: &String| s.len(), |(a,b)| a+b, |()| 0, |_ : &char| 1);
        fc_imm_do_tests(&mut rng, &mut 2000, &mut 1000, a_simp_sc, b_simp_sc, |a,b| *a as isize - *b as isize);
    }
}

fn fl_mut_do_tests<'a, T: PartialEq + Eq + Debug + Clone + 'a, D: Clone + 'a, D2: Clone + PartialEq + Eq + Debug + 'a, Settings: FoldSettings<T,D>, Simplification: FoldSimplification<T,D,D2=D2>,Rand: Rng + Clone, B: FoldListSlice<'a,T,D2,OriginalD = D, Simplification = Simplification> + MutFoldListSlice<'a,T,D2>> (
    rng: &mut Rand, 
    counter: &mut isize,
    n: &mut isize, 

    mut a: VecBasedFoldChainSlice<T,D,Settings,Simplification>, 
    mut b: B,
    comparer: impl Fn(&Simplification::D2,&Simplification::D2) -> isize + Copy,
    rand_maker: impl Fn(&mut Rand) -> T + Copy
) where <B as FoldListSlice<'a, T, D2>>::UnderlyingChain: MutFoldChainSlice<'a, T, (usize, D)>{
    while *n > 0 {
        fl_imm_do_tests(rng, &mut 0, &mut -1, a.borrow_mut(), b.borrow_mut(), comparer);
        *counter+=1;
        print!("\r{}, total size={}, slice size = {}",*counter,a.vec.borrow().len(),a.end_exc - a.start_inc);
        std::io::stdout().flush().unwrap();
        if *counter % 100 == 0 {println!()}
        if *counter == 1879 {
            let _ = 2+2;
        }
        let case = rng.random_range(0_u32..=26);
        match case {
            0 => {
                //push left
                let to_add = rand_maker(rng);
                a.append_left(to_add.clone());
                b.append_left(to_add);
            }
            1 => {
                //push right
                let to_add = rand_maker(rng);
                a.append_right(to_add.clone());
                b.append_right(to_add);
            }
            2 => {
                //pop left
                let a_pop = a.pop_left();
                let b_pop = b.pop_left();
                assert_eq!(a_pop,b_pop)
            }
            3 => {
                //pop right
                let a_pop = a.pop_right();
                let b_pop = b.pop_right();
                assert_eq!(a_pop,b_pop)
            }
            4 => {
                //set left
                let to_set = rand_maker(rng);
                let a_set = a.set_left_or_err(to_set.clone());
                let b_set = b.set_left_or_err(to_set);
                assert_eq!(a_set,b_set)
            }
            5 => {
                //set right
                let to_set = rand_maker(rng);
                let a_set = a.set_right_or_err(to_set.clone());
                let b_set = b.set_right_or_err(to_set);
                assert_eq!(a_set,b_set)
            }
            6 => {
                //update left
                let to_become = rand_maker(rng);
                let a_update = a.update_left(|t| t.map(|t| *t = to_become.clone()));
                let b_update = b.update_left(|t| t.map(|t| *t = to_become));
                assert_eq!(a_update,b_update)
            }
            7 => {
                //update right
                let to_become = rand_maker(rng);
                let a_update = a.update_right(|t| t.map(|t| *t = to_become.clone()));
                let b_update = b.update_right(|t| t.map(|t| *t = to_become));
                assert_eq!(a_update,b_update)
            }
            8 => {
                //reverse
                let mut to_take = *n / 10;
                *n -= to_take;
                fl_mut_do_tests(rng, counter, &mut to_take, 
                    a.borrow_mut().mut_view_reversed(),
                    b.borrow_mut().mut_view_reversed(), 
                    comparer, rand_maker
                );
                *n += to_take
            }
            9 => {
                //drop left
                let mut to_take = *n / 10;
                *n -= to_take;
                let fold_to_find = a.rand_fold_left(rng);
                let a_dropped: _ = a.borrow_mut().mut_view_drop_left_until(|fold| comparer(&fold,&fold_to_find) > 0);
                let b_dropped: _ = b.borrow_mut().mut_view_drop_left_until(|fold| comparer(&fold,&fold_to_find) > 0);
                fl_mut_do_tests(rng, counter, &mut to_take, a_dropped, b_dropped, comparer, rand_maker);
                *n += to_take
            }
            10 => {
                //take left
                let mut to_take = *n / 10;
                *n -= to_take;
                let fold_to_find = a.rand_fold_left(rng);
                let a_dropped: _ = a.borrow_mut().mut_view_take_left_until(|fold| comparer(&fold,&fold_to_find) > 0);
                let b_dropped: _ = b.borrow_mut().mut_view_take_left_until(|fold| comparer(&fold,&fold_to_find) > 0);
                fl_mut_do_tests(rng, counter, &mut to_take, a_dropped, b_dropped, comparer, rand_maker);
                *n += to_take
            }
            11 => {
                //drop right
                let mut to_take = *n / 10;
                *n -= to_take;
                let fold_to_find = a.rand_fold_left(rng);
                let a_dropped: _ = a.borrow_mut().mut_view_drop_right_until(|fold| comparer(&fold,&fold_to_find) > 0);
                let b_dropped: _ = b.borrow_mut().mut_view_drop_right_until(|fold| comparer(&fold,&fold_to_find) > 0);
                fl_mut_do_tests(rng, counter, &mut to_take, a_dropped, b_dropped, comparer, rand_maker);
                *n += to_take
            }
            12 => {
                //take right
                let mut to_take = *n / 10;
                *n -= to_take;
                let fold_to_find = a.rand_fold_left(rng);
                let a_dropped: _ = a.borrow_mut().mut_view_take_right_until(|fold| comparer(&fold,&fold_to_find) > 0);
                let b_dropped: _ = b.borrow_mut().mut_view_take_right_until(|fold| comparer(&fold,&fold_to_find) > 0);
                fl_mut_do_tests(rng, counter, &mut to_take, a_dropped, b_dropped, comparer, rand_maker);
                *n += to_take
            }
            13 => {
                //foreach mut
                let slice_len = a.end_exc - a.start_inc;
                let to_replace_with : Vec<_> = std::iter::from_fn(|| Some(rand_maker(rng))).take(slice_len).collect();
                let mut idx = 0;
                a.foreach_mut(|t| {
                    *t = to_replace_with[idx].clone();
                    idx += 1;
                });
                idx = 0;
                b.foreach_mut(|t| {
                    // print!("\rgot to {}!",idx);
                    // if idx == 6836 {
                    //     let _ = 2 + 2;
                    // }
                    *t = to_replace_with[idx].clone();
                    idx += 1;
                });
            }
            14 => {
                //test imm with self as imm
                let mut to_take = 5;
                *n -= to_take;
                fl_imm_do_tests(rng, counter, &mut to_take, a.borrow(), b.borrow(), comparer);
                *n += to_take;
            }
            15 => {
                //test imm with self
                let mut to_take = 5;
                *n -= to_take;
                fl_imm_do_tests(rng, counter, &mut to_take, a.borrow_mut(), b.borrow_mut(), comparer);
                *n += to_take;
            },
            16 => {
                //split off and do sub-tests, then put back
                let mut to_take = *n / 10;
                *n -= to_take;
                let fold_to_find = a.rand_fold_left(rng);
                let way_to_take = rng.random_range(0..4);
                match way_to_take {
                    0 => {
                        let mut a_taken = a.borrow_mut().mut_view_take_left_until(|fold| comparer(&fold,&fold_to_find) > 0)
                            .take_all();
                        if rng.random_bool(0.1) {
                            a_taken = a_taken.clone()
                        }
                        let b_taken = b.borrow_mut().mut_view_take_left_until(|fold| comparer(&fold,&fold_to_find) > 0)
                            .take_all();
                        fl_mut_do_tests(rng, counter, &mut to_take, a.borrow_mut(), b.borrow_mut(), comparer, rand_maker);
                        fl_imm_do_tests(rng, &mut 0, &mut -1, a.borrow_mut(), b.borrow_mut(), comparer);
                        a.append_all_left(a_taken);
                        b.append_all_left(b_taken);
                    }
                    1 => {
                        let mut a_taken = a.borrow_mut().mut_view_drop_left_until(|fold| comparer(&fold,&fold_to_find) > 0)
                            .take_all();
                        if rng.random_bool(0.1) {
                            a_taken = a_taken.clone()
                        }
                        let b_taken = b.borrow_mut().mut_view_drop_left_until(|fold| comparer(&fold,&fold_to_find) > 0)
                            .take_all();
                        fl_mut_do_tests(rng, counter, &mut to_take, a.borrow_mut(), b.borrow_mut(), comparer, rand_maker);
                        fl_imm_do_tests(rng, &mut 0, &mut -1, a.borrow_mut(), b.borrow_mut(), comparer);
                        a.append_all_right(a_taken);
                        b.append_all_right(b_taken);
                    }
                    2 => {
                        let mut a_taken = a.borrow_mut().mut_view_take_right_until(|fold| comparer(&fold,&fold_to_find) > 0)
                            .take_all();
                        let b_taken = b.borrow_mut().mut_view_take_right_until(|fold| comparer(&fold,&fold_to_find) > 0)
                            .take_all();
                        if rng.random_bool(0.1) {
                            a_taken = a_taken.clone()
                        }
                        fl_mut_do_tests(rng, counter, &mut to_take, a.borrow_mut(), b.borrow_mut(), comparer, rand_maker);
                        fl_imm_do_tests(rng, &mut 0, &mut -1, a.borrow_mut(), b.borrow_mut(), comparer);
                        a.append_all_right(a_taken);
                        b.append_all_right(b_taken);
                    }
                    3 => {
                        let mut a_taken = a.borrow_mut().mut_view_drop_right_until(|fold| comparer(&fold,&fold_to_find) > 0)
                            .take_all();
                        let b_taken = b.borrow_mut().mut_view_drop_right_until(|fold| comparer(&fold,&fold_to_find) > 0)
                            .take_all();
                        if rng.random_bool(0.1) {
                            a_taken = a_taken.clone()
                        }
                        fl_mut_do_tests(rng, counter, &mut to_take, a.borrow_mut(), b.borrow_mut(), comparer, rand_maker);
                        fl_imm_do_tests(rng, &mut 0, &mut -1, a.borrow_mut(), b.borrow_mut(), comparer);
                        a.append_all_left(a_taken);
                        b.append_all_left(b_taken);
                    }
                    _ => panic!()
                };
                *n += to_take;
            }
            17 => {
                // append left or right from iter
                let amt_to_add = rng.random_range(1..20);
                let mut elems = Vec::with_capacity(amt_to_add);
                for _ in 0..amt_to_add {
                    elems.push(rand_maker(rng))
                }
                if rng.random_bool(0.5) {
                    for e in &elems {
                        a.append_left(e.clone())
                    }
                    b.append_left_from_iter(elems.into_iter());
                } else {
                    for e in &elems {
                        a.append_right(e.clone())
                    }
                    b.append_right_from_iter(elems.into_iter());
                }
            }
            18 => {
                //take n left
                let mut to_take = *n / 10;
                *n -= to_take;
                let amt_to_take = rng.random_range(0..=b.len()+10);
                let a_taked = a.borrow().view_take_left(amt_to_take);
                let b_taked = b.borrow_mut().mut_view_take_left(amt_to_take);
                fl_mut_do_tests(rng, counter, &mut to_take, a_taked, b_taked, comparer, rand_maker);
            }
            19 => {
                //take n right
                let mut to_take = *n / 10;
                *n -= to_take;
                let amt_to_take = rng.random_range(0..=b.len()+10);
                let a_taked = a.borrow().view_take_right(amt_to_take);
                let b_taked = b.borrow_mut().mut_view_take_right(amt_to_take);
                fl_mut_do_tests(rng, counter, &mut to_take, a_taked, b_taked, comparer, rand_maker);
            }
            20 => {
                //take n left
                let mut to_take = *n / 10;
                *n -= to_take;
                let amt_to_drop = rng.random_range(0..=b.len()+10);
                let a_taked = a.borrow().view_drop_left(amt_to_drop);
                let b_taked = b.borrow_mut().mut_view_drop_left(amt_to_drop);
                fl_mut_do_tests(rng, counter, &mut to_take, a_taked, b_taked, comparer, rand_maker);
            }
            21 => {
                //take n right
                let mut to_take = *n / 10;
                *n -= to_take;
                let amt_to_drop = rng.random_range(0..=b.len()+10);
                let a_taked = a.borrow().view_drop_right(amt_to_drop);
                let b_taked = b.borrow_mut().mut_view_drop_right(amt_to_drop);
                fl_mut_do_tests(rng, counter, &mut to_take, a_taked, b_taked, comparer, rand_maker);
            }
            22 => {
                //replace at index
                if b.is_empty() {continue;}
                let idx = rng.random_range(0..b.len());
                let new_val = rand_maker(rng);
                a.borrow().view_drop_left(idx).set_left_or_err(new_val.clone()).unwrap();
                b.set_at(idx, new_val);
            }
            23 => {
                //remove at index
                if b.is_empty() {continue;}
                let idx = rng.random_range(0..b.len());
                assert_eq!(
                    a.borrow_mut().view_drop_left(idx).pop_left().unwrap(),
                    b.remove_at(idx)
                )
            }
            24 => {
                //insert at index
                let idx = rng.random_range(0..=b.len());
                let new_val = rand_maker(rng);
                a.borrow().view_drop_left(idx).append_left(new_val.clone());
                b.insert_at(idx, new_val);
            }
            25 => {
                //test underlying chain (ignoring size)
                let mut to_take = *n / 30;
                *n -= to_take;
                let b_chained = b.borrow_mut().mut_as_unsized_chain_keeping_simplification();
                fc_mut_do_tests(rng, counter, &mut to_take, a.borrow(), b_chained, comparer, rand_maker);
            }
            26 => {
                //take a subrange and move it all to the left or right
                let mut range_start = rng.random_range(0..=b.len());
                let mut range_end = rng.random_range(0..=b.len());

                let end_drop = b.len() - range_end;
                if range_start > range_end {core::mem::swap(&mut range_start, &mut range_end)}
                
                let a_taken = a.borrow()
                    .view_drop_left(range_start)
                    .view_drop_right(end_drop)
                    .take_all();

                let b_taken = b.borrow_mut()
                    .mut_view_drop_left(range_start)
                    .mut_view_drop_right(end_drop)
                    .take_all();

                if rng.next_u32() & 1 == 0 {
                    a.append_all_left(a_taken);
                    b.append_all_left(b_taken);
                } else {
                    a.append_all_right(a_taken);
                    b.append_all_right(b_taken);
                }
            }
            _ => panic!()
        }
        *n-=1;
    }
}

fn fc_mut_do_tests<'a, T: PartialEq + Eq + Debug + Clone + 'a, D: Clone, D2: Clone + PartialEq + Eq + Debug + 'a, Settings: FoldSettings<T,D>, Simplification: FoldSimplification<T,D,D2=D2>,Rand: Rng + Clone, B: MutFoldChainSlice<'a,T,D2>>(
    rng: &mut Rand, 
    counter: &mut isize,
    n: &mut isize, 

    mut a: VecBasedFoldChainSlice<T,D,Settings,Simplification>, 
    mut b: B,
    comparer: impl Fn(&Simplification::D2,&Simplification::D2) -> isize + Copy,
    rand_maker: impl Fn(&mut Rand) -> T + Copy
) {
    while *n > 0 {
        fc_imm_do_tests::<_,_,_,_,_,MutFoldChainSliceStruct<_,_,_,_,_,_,_>>(rng, &mut 0, &mut -1, a.borrow_mut(), b.borrow_mut(), comparer);
        *counter+=1;
        print!("\r{}, total size={}, slice size = {}",*counter,a.vec.borrow().len(),a.end_exc - a.start_inc);
        std::io::stdout().flush().unwrap();
        if *counter % 100 == 0 {println!()}
        if *counter == 28893 {
            let _ = 2+2;
        }
        let case = rng.random_range(0_u32..=17);
        match case {
            0 => {
                //push left
                let to_add = rand_maker(rng);
                a.append_left(to_add.clone());
                b.append_left(to_add);
            }
            1 => {
                //push right
                let to_add = rand_maker(rng);
                a.append_right(to_add.clone());
                b.append_right(to_add);
            }
            2 => {
                //pop left
                let a_pop = a.pop_left();
                let b_pop = b.pop_left();
                assert_eq!(a_pop,b_pop)
            }
            3 => {
                //pop right
                let a_pop = a.pop_right();
                let b_pop = b.pop_right();
                assert_eq!(a_pop,b_pop)
            }
            4 => {
                //set left
                let to_set = rand_maker(rng);
                let a_set = a.set_left_or_err(to_set.clone());
                let b_set = b.set_left_or_err(to_set);
                assert_eq!(a_set,b_set)
            }
            5 => {
                //set right
                let to_set = rand_maker(rng);
                let a_set = a.set_right_or_err(to_set.clone());
                let b_set = b.set_right_or_err(to_set);
                assert_eq!(a_set,b_set)
            }
            6 => {
                //update left
                let to_become = rand_maker(rng);
                let a_update = a.update_left(|t| t.map(|t| *t = to_become.clone()));
                let b_update = b.update_left(|t| t.map(|t| *t = to_become));
                assert_eq!(a_update,b_update)
            }
            7 => {
                //update right
                let to_become = rand_maker(rng);
                let a_update = a.update_right(|t| t.map(|t| *t = to_become.clone()));
                let b_update = b.update_right(|t| t.map(|t| *t = to_become));
                assert_eq!(a_update,b_update)
            }
            8 => {
                //reverse
                let mut to_take = *n / 10;
                *n -= to_take;
                fc_mut_do_tests(rng, counter, &mut to_take, 
                    a.borrow_mut().mut_view_reversed(),
                    b.borrow_mut().mut_view_reversed(), 
                    comparer, rand_maker
                );
                *n += to_take
            }
            9 => {
                //drop left
                let mut to_take = *n / 10;
                *n -= to_take;
                let fold_to_find = a.rand_fold_left(rng);
                let a_dropped: _ = a.borrow_mut().mut_view_drop_left_until(|fold| comparer(&fold,&fold_to_find) > 0);
                let b_dropped: _ = b.borrow_mut().mut_view_drop_left_until(|fold| comparer(&fold,&fold_to_find) > 0);
                fc_mut_do_tests(rng, counter, &mut to_take, a_dropped, b_dropped, comparer, rand_maker);
                *n += to_take
            }
            10 => {
                //take left
                let mut to_take = *n / 10;
                *n -= to_take;
                let fold_to_find = a.rand_fold_left(rng);
                let a_dropped: _ = a.borrow_mut().mut_view_take_left_until(|fold| comparer(&fold,&fold_to_find) > 0);
                let b_dropped: _ = b.borrow_mut().mut_view_take_left_until(|fold| comparer(&fold,&fold_to_find) > 0);
                fc_mut_do_tests(rng, counter, &mut to_take, a_dropped, b_dropped, comparer, rand_maker);
                *n += to_take
            }
            11 => {
                //drop right
                let mut to_take = *n / 10;
                *n -= to_take;
                let fold_to_find = a.rand_fold_left(rng);
                let a_dropped: _ = a.borrow_mut().mut_view_drop_right_until(|fold| comparer(&fold,&fold_to_find) > 0);
                let b_dropped: _ = b.borrow_mut().mut_view_drop_right_until(|fold| comparer(&fold,&fold_to_find) > 0);
                fc_mut_do_tests(rng, counter, &mut to_take, a_dropped, b_dropped, comparer, rand_maker);
                *n += to_take
            }
            12 => {
                //take right
                let mut to_take = *n / 10;
                *n -= to_take;
                let fold_to_find = a.rand_fold_left(rng);
                let a_dropped: _ = a.borrow_mut().mut_view_take_right_until(|fold| comparer(&fold,&fold_to_find) > 0);
                let b_dropped: _ = b.borrow_mut().mut_view_take_right_until(|fold| comparer(&fold,&fold_to_find) > 0);
                fc_mut_do_tests(rng, counter, &mut to_take, a_dropped, b_dropped, comparer, rand_maker);
                *n += to_take
            }
            13 => {
                //foreach mut
                let slice_len = a.end_exc - a.start_inc;
                let to_replace_with : Vec<_> = std::iter::from_fn(|| Some(rand_maker(rng))).take(slice_len).collect();
                let mut idx = 0;
                a.foreach_mut(|t| {
                    *t = to_replace_with[idx].clone();
                    idx += 1;
                });
                idx = 0;
                b.foreach_mut(|t| {
                    // print!("\rgot to {}!",idx);
                    // if idx == 6836 {
                    //     let _ = 2 + 2;
                    // }
                    *t = to_replace_with[idx].clone();
                    idx += 1;
                });
            }
            14 => {
                //test imm with self as imm
                let mut to_take = 5;
                *n -= to_take;
                fc_imm_do_tests(rng, counter, &mut to_take, a.borrow(), b.borrow(), comparer);
                *n += to_take;
            }
            15 => {
                //test imm with self
                let mut to_take = 5;
                *n -= to_take;
                fc_imm_do_tests(rng, counter, &mut to_take, a.borrow_mut(), b.borrow_mut(), comparer);
                *n += to_take;
            },
            16 => {
                //split off and do sub-tests, then put back
                let mut to_take = *n / 10;
                *n -= to_take;
                let fold_to_find = a.rand_fold_left(rng);
                let way_to_take = rng.random_range(0..4);
                match way_to_take {
                    0 => {
                        let mut a_taken = a.borrow_mut().mut_view_take_left_until(|fold| comparer(&fold,&fold_to_find) > 0)
                            .take_all();
                        if rng.random_bool(0.1) {
                            a_taken = a_taken.clone()
                        }
                        let b_taken = b.borrow_mut().mut_view_take_left_until(|fold| comparer(&fold,&fold_to_find) > 0)
                            .take_all();
                        fc_mut_do_tests(rng, counter, &mut to_take, a.borrow_mut(), b.borrow_mut(), comparer, rand_maker);
                        fc_imm_do_tests(rng, &mut 0, &mut -1, a.borrow_mut(), b.borrow_mut(), comparer);
                        a.append_all_left(a_taken);
                        b.append_all_left(b_taken);
                    }
                    1 => {
                        let mut a_taken = a.borrow_mut().mut_view_drop_left_until(|fold| comparer(&fold,&fold_to_find) > 0)
                            .take_all();
                        if rng.random_bool(0.1) {
                            a_taken = a_taken.clone()
                        }
                        let b_taken = b.borrow_mut().mut_view_drop_left_until(|fold| comparer(&fold,&fold_to_find) > 0)
                            .take_all();
                        fc_mut_do_tests(rng, counter, &mut to_take, a.borrow_mut(), b.borrow_mut(), comparer, rand_maker);
                        fc_imm_do_tests(rng, &mut 0, &mut -1, a.borrow_mut(), b.borrow_mut(), comparer);
                        a.append_all_right(a_taken);
                        b.append_all_right(b_taken);
                    }
                    2 => {
                        let mut a_taken = a.borrow_mut().mut_view_take_right_until(|fold| comparer(&fold,&fold_to_find) > 0)
                            .take_all();
                        let b_taken = b.borrow_mut().mut_view_take_right_until(|fold| comparer(&fold,&fold_to_find) > 0)
                            .take_all();
                        if rng.random_bool(0.1) {
                            a_taken = a_taken.clone()
                        }
                        fc_mut_do_tests(rng, counter, &mut to_take, a.borrow_mut(), b.borrow_mut(), comparer, rand_maker);
                        fc_imm_do_tests(rng, &mut 0, &mut -1, a.borrow_mut(), b.borrow_mut(), comparer);
                        a.append_all_right(a_taken);
                        b.append_all_right(b_taken);
                    }
                    3 => {
                        let mut a_taken = a.borrow_mut().mut_view_drop_right_until(|fold| comparer(&fold,&fold_to_find) > 0)
                            .take_all();
                        let b_taken = b.borrow_mut().mut_view_drop_right_until(|fold| comparer(&fold,&fold_to_find) > 0)
                            .take_all();
                        if rng.random_bool(0.1) {
                            a_taken = a_taken.clone()
                        }
                        fc_mut_do_tests(rng, counter, &mut to_take, a.borrow_mut(), b.borrow_mut(), comparer, rand_maker);
                        fc_imm_do_tests(rng, &mut 0, &mut -1, a.borrow_mut(), b.borrow_mut(), comparer);
                        a.append_all_left(a_taken);
                        b.append_all_left(b_taken);
                    }
                    _ => panic!()
                };
                *n += to_take;
            }
            17 => {
                // append left or right from iter
                let amt_to_add = rng.random_range(1..20);
                let mut elems = Vec::with_capacity(amt_to_add);
                for _ in 0..amt_to_add {
                    elems.push(rand_maker(rng))
                }
                if rng.random_bool(0.5) {
                    for e in &elems {
                        a.append_left(e.clone())
                    }
                    b.append_left_from_iter(elems.into_iter());
                } else {
                    for e in &elems {
                        a.append_right(e.clone())
                    }
                    b.append_right_from_iter(elems.into_iter());
                }
            }
            _ => panic!()
        }
        *n-=1;
    }
}

fn fl_imm_do_tests<'a,T: PartialEq + Eq + Debug + Clone + 'a, D: Clone + 'a, D2: Clone + PartialEq + Eq + Debug + 'a, Settings: FoldSettings<T,D>, Simplification: FoldSimplification<T,D,D2=D2> , B: FoldListSlice<'a,T,D2,Simplification = Simplification,OriginalD = D>>(
    rng: &mut impl Rng, 
    counter: &mut isize,
    n: &mut isize, 

    a: VecBasedFoldChainSlice<T,D,Settings,Simplification>, 
    b: B, 
    comparer: impl Fn(&Simplification::D2,&Simplification::D2) -> isize + Copy
) {
    a.debug_check_structural_integrity();
    b.debug_check_structural_integrity();
    //len
    assert_eq!(a.end_exc - a.start_inc,b.len());
    //fold
    let afold = a.fold();
    let bfold = b.fold();
    assert_eq!(afold,bfold);
    //left
    let aleft = a.left();
    let bleft = b.left();
    assert_eq!(aleft,bleft);
    //right
    let aright = a.right();
    let bright = b.right();
    assert_eq!(aright,bright);

    //foreach
    let a_vec = a.vec.borrow();
    let mut a_iter = (a_vec[a.start_inc..a.end_exc]).into_iter();
    b.foreach(|t| {
        let a_nxt = if a.is_reversed {a_iter.next_back()} else {a_iter.next()};
        let Some(a_nxt) = a_nxt else {panic!()};
        assert_eq!(t,a_nxt);
    });
    assert_eq!(a_iter.next(), None);
    //drop all left
    assert!(b.borrow().view_drop_left_until(|_| false).is_empty());
    //drop all right
    assert!(b.borrow().view_drop_right_until(|_| false).is_empty());
    //keep none left
    assert!(b.borrow().view_take_left_until(|_| true).is_empty());
    //keep none right
    assert!(b.borrow().view_take_right_until(|_| true).is_empty());
    //drop none left


    assert_eq!(b.borrow().view_drop_left_until(|_| true).fold(),bfold);
    //drop none right
    assert_eq!(b.borrow().view_drop_right_until(|_| true).fold(),bfold);
    //keep all left
    assert_eq!(b.borrow().view_take_left_until(|_| false).fold(),bfold);
    //keep all right
    assert_eq!(b.borrow().view_take_right_until(|_| false).fold(),bfold);
    //is_empty
    assert_eq!(a.is_empty(),b.is_empty());
    if a.is_empty() {
        assert_eq!(b.len(),0);
        return
    }
    while *n > 0 {
        *counter+=1;
        print!("\r{}",*counter);
        std::io::stdout().flush().unwrap();
        // if *counter % 100 == 0 {println!()}
        if *counter == 113 {
            let _ = 2+2;
        }
        let case = rng.random_range(0_u32..=11);
        match case {
            0 => {
                //drop left
                let mut to_take = *n / 10;
                *n -= to_take;
                let fold_to_find = a.rand_fold_left(rng);
                let a_dropped = a.borrow().view_drop_left_until(|fold| comparer(&fold,&fold_to_find) > 0);
                let b_dropped = b.borrow().view_drop_left_until(|fold| comparer(&fold,&fold_to_find) > 0);
                fl_imm_do_tests::<_,_,_,_,_,FoldListSliceFrom<'_, T, _, _, _, _>>(rng, counter, &mut to_take, a_dropped, b_dropped,comparer);
                *n += to_take;
            } 
            1 => {
                //take left
                let mut to_take = *n / 10;
                *n -= to_take;
                let fold_to_find = a.rand_fold_left(rng);
                let a_taked = a.borrow().view_take_left_until(|fold| comparer(&fold,&fold_to_find) > 0);
                let b_taked = b.borrow().view_take_left_until(|fold| comparer(&fold,&fold_to_find) > 0);
                fl_imm_do_tests::<_,_,_,_,_,FoldListSliceFrom<'_, T, _, _, _, _>>(rng, counter, &mut to_take, a_taked, b_taked,comparer);
                *n += to_take;
            }
            2 => {
                //drop right
                let mut to_take = *n / 10;
                *n -= to_take;
                let fold_to_find = a.rand_fold_left(rng);
                let a_taked = a.borrow().view_drop_right_until(|fold| comparer(&fold,&fold_to_find) > 0);
                let b_taked = b.borrow().view_drop_right_until(|fold| comparer(&fold,&fold_to_find) > 0);
                fl_imm_do_tests::<_,_,_,_,_,FoldListSliceFrom<'_, T, _, _, _, _>>(rng, counter, &mut to_take, a_taked, b_taked,comparer);
                *n += to_take;
            }
            3 => {
                //take right
                let mut to_take = *n / 10;
                *n -= to_take;
                let fold_to_find = a.rand_fold_left(rng);
                let a_taked = a.borrow().view_take_right_until(|fold| comparer(&fold,&fold_to_find) > 0);
                let b_taked = b.borrow().view_take_right_until(|fold| comparer(&fold,&fold_to_find) > 0);
                fl_imm_do_tests::<_,_,_,_,_,FoldListSliceFrom<'_, T, _, _, _, _>>(rng, counter, &mut to_take, a_taked, b_taked,comparer);
                *n += to_take;
            }
            4 => {
                //reverse
                let mut to_take = *n / 30;
                *n -= to_take;
                let a_rev = a.borrow().view_reversed();
                let b_rev = b.borrow().view_reversed();
                fl_imm_do_tests::<_,_,_,_,_,FoldListSliceFrom<'_, T, _, _, _, _>>(rng, counter, &mut to_take, a_rev, b_rev,comparer);
                *n += to_take;
            }
            5 => {
                //test iterator
                let p_of_next_back = rng.next_u64();
                let iterations = rng.random_range(0..((a.end_exc - a.start_inc) + 5));
                let mut a_iter = (&a).into_iter();
                let mut b_iter = b.iter();
                for _ in 0..iterations {
                    let do_next_back = rng.next_u64() < p_of_next_back;
                    let (a_next,b_next) = if do_next_back {
                        (a_iter.next_back(),b_iter.next_back())
                    } else {
                        (a_iter.next(),b_iter.next())
                    };
                    let a_next = a_next.map(|an| &*an);
                    assert_eq!(a_next,b_next);
                }
            }
            6 => {
                // take n left
                let mut to_take = *n / 30;
                let amt_to_take = rng.random_range(0..=b.len()+10);
                let a_taked = a.borrow().view_take_left(amt_to_take);
                let b_taked = b.borrow().view_take_left(amt_to_take);
                fl_imm_do_tests::<_,_,_,_,_,FoldListSliceFrom<'_, T, _, _, _, _>>(rng, counter, &mut to_take, a_taked, b_taked, comparer);
            }
            7 => {
                // take n right
                let mut to_take = *n / 30;
                *n -= to_take;
                let amt_to_take = rng.random_range(0..=b.len()+10);
                let a_taked = a.borrow().view_take_right(amt_to_take);
                let b_taked = b.borrow().view_take_right(amt_to_take);
                fl_imm_do_tests::<_,_,_,_,_,FoldListSliceFrom<'_, T, _, _, _, _>>(rng, counter, &mut to_take, a_taked, b_taked, comparer);
            }
            8 => {
                // drop n left
                let mut to_take = *n / 30;
                *n -= to_take;
                let amt_to_drop = rng.random_range(0..=b.len()+10);
                let a_taked = a.borrow().view_drop_left(amt_to_drop);
                let b_taked = b.borrow().view_drop_left(amt_to_drop);
                fl_imm_do_tests::<_,_,_,_,_,FoldListSliceFrom<'_, T, _, _, _, _>>(rng, counter, &mut to_take, a_taked, b_taked, comparer);
            }
            9 => {
                // drop n right
                let mut to_take = *n / 30;
                *n -= to_take;
                let amt_to_drop = rng.random_range(0..=b.len()+10);
                let a_taked = a.borrow().view_drop_right(amt_to_drop);
                let b_taked = b.borrow().view_drop_right(amt_to_drop);
                fl_imm_do_tests::<_,_,_,_,_,FoldListSliceFrom<'_, T, _, _, _, _>>(rng, counter, &mut to_take, a_taked, b_taked, comparer);
            }
            10 => {
                // test underlying chain (ignoring size)
                let mut to_take = *n / 30;
                *n -= to_take;
                let b_chained = b.borrow().as_unsized_chain_keeping_simplification();
                fc_imm_do_tests::<_,_,_,_,_,ImmFoldChainSliceStruct<'_, _, _, _, _, _, _, _>>(rng, counter, &mut to_take, a.borrow(), b_chained, comparer);
            }
            11 => {
                // get at index
                let idx = rng.random_range(0..b.len());
                assert_eq!(a.borrow().view_drop_left(idx).left().unwrap(),&b[idx]);
            }
            _ => panic!(),
        }
        *n-=1;
    }
}

fn fc_imm_do_tests<'a,T: PartialEq + Eq + Debug + Clone + 'a, D: Clone, D2: Clone + PartialEq + Eq + Debug + 'a, Settings: FoldSettings<T,D>, Simplification: FoldSimplification<T,D,D2=D2>, B: FoldChainSlice<'a,T,D2>>(
    rng: &mut impl Rng, 
    counter: &mut isize,
    n: &mut isize, 

    a: VecBasedFoldChainSlice<T,D,Settings,Simplification>, 
    b: B, 
    comparer: impl Fn(&Simplification::D2,&Simplification::D2) -> isize + Copy
) {
    a.debug_check_structural_integrity();
    b.debug_check_structural_integrity();
    //fold
    let afold = a.fold();
    let bfold = b.fold();
    assert_eq!(afold,bfold);
    //left
    let aleft = a.left();
    let bleft = b.left();
    assert_eq!(aleft,bleft);
    //right
    let aright = a.right();
    let bright = b.right();
    assert_eq!(aright,bright);

    //foreach
    let a_vec = a.vec.borrow();
    let mut a_iter = (a_vec[a.start_inc..a.end_exc]).into_iter();
    b.foreach(|t| {
        let a_nxt = if a.is_reversed {a_iter.next_back()} else {a_iter.next()};
        let Some(a_nxt) = a_nxt else {panic!()};
        assert_eq!(t,a_nxt);
    });
    assert_eq!(a_iter.next(), None);
    //drop all left
    assert!(b.borrow().view_drop_left_until(|_| false).is_empty());
    //drop all right
    assert!(b.borrow().view_drop_right_until(|_| false).is_empty());
    //keep none left
    assert!(b.borrow().view_take_left_until(|_| true).is_empty());
    //keep none right
    assert!(b.borrow().view_take_right_until(|_| true).is_empty());
    //drop none left


    assert_eq!(b.borrow().view_drop_left_until(|_| true).fold(),bfold);
    //drop none right
    assert_eq!(b.borrow().view_drop_right_until(|_| true).fold(),bfold);
    //keep all left
    assert_eq!(b.borrow().view_take_left_until(|_| false).fold(),bfold);
    //keep all right
    assert_eq!(b.borrow().view_take_right_until(|_| false).fold(),bfold);
    //is_empty
    assert_eq!(a.is_empty(),b.is_empty());
    if a.is_empty() {return}
    while *n > 0 {
        *counter+=1;
        print!("\r{}",*counter);
        std::io::stdout().flush().unwrap();
        // if *counter % 100 == 0 {println!()}
        if *counter == 113 {
            let _ = 2+2;
        }
        let case = rng.random_range(0_u32..6);
        match case {
            0 => {
                //drop left
                let mut to_take = *n / 10;
                *n -= to_take;
                let fold_to_find = a.rand_fold_left(rng);
                let a_dropped = a.borrow().view_drop_left_until(|fold| comparer(&fold,&fold_to_find) > 0);
                let b_dropped = b.borrow().view_drop_left_until(|fold| comparer(&fold,&fold_to_find) > 0);
                fc_imm_do_tests::<_,_,_,_,_,ImmFoldChainSliceStruct<_,_,_,_,_,_,_>>(rng, counter, &mut to_take, a_dropped, b_dropped,comparer);
                *n += to_take;
            } 
            1 => {
                //take left
                let mut to_take = *n / 10;
                *n -= to_take;
                let fold_to_find = a.rand_fold_left(rng);
                let a_taked = a.borrow().view_take_left_until(|fold| comparer(&fold,&fold_to_find) > 0);
                let b_taked = b.borrow().view_take_left_until(|fold| comparer(&fold,&fold_to_find) > 0);
                fc_imm_do_tests::<_,_,_,_,_,ImmFoldChainSliceStruct<_,_,_,_,_,_,_>>(rng, counter, &mut to_take, a_taked, b_taked,comparer);
                *n += to_take;
            }
            2 => {
                //drop right
                let mut to_take = *n / 10;
                *n -= to_take;
                let fold_to_find = a.rand_fold_left(rng);
                let a_taked = a.borrow().view_drop_right_until(|fold| comparer(&fold,&fold_to_find) > 0);
                let b_taked = b.borrow().view_drop_right_until(|fold| comparer(&fold,&fold_to_find) > 0);
                fc_imm_do_tests::<_,_,_,_,_,ImmFoldChainSliceStruct<_,_,_,_,_,_,_>>(rng, counter, &mut to_take, a_taked, b_taked,comparer);
                *n += to_take;
            }
            3 => {
                //take right
                let mut to_take = *n / 10;
                *n -= to_take;
                let fold_to_find = a.rand_fold_left(rng);
                let a_taked = a.borrow().view_take_right_until(|fold| comparer(&fold,&fold_to_find) > 0);
                let b_taked = b.borrow().view_take_right_until(|fold| comparer(&fold,&fold_to_find) > 0);
                fc_imm_do_tests::<_,_,_,_,_,ImmFoldChainSliceStruct<_,_,_,_,_,_,_>>(rng, counter, &mut to_take, a_taked, b_taked,comparer);
                *n += to_take;
            }
            4 => {
                //reverse
                let mut to_take = *n / 30;
                *n -= to_take;
                let a_rev = a.borrow().view_reversed();
                let b_rev = b.borrow().view_reversed();
                fc_imm_do_tests::<_,_,_,_,_,ImmFoldChainSliceStruct<_,_,_,_,_,_,_>>(rng, counter, &mut to_take, a_rev, b_rev,comparer);
                *n += to_take;
            }
            5 => {
                //test iterator
                let p_of_next_back = rng.next_u64();
                let iterations = rng.random_range(0..((a.end_exc - a.start_inc) + 5));
                let mut a_iter = (&a).into_iter();
                let mut b_iter = b.iter();
                for _ in 0..iterations {
                    let do_next_back = rng.next_u64() < p_of_next_back;
                    let (a_next,b_next) = if do_next_back {
                        (a_iter.next_back(),b_iter.next_back())
                    } else {
                        (a_iter.next(),b_iter.next())
                    };
                    let a_next = a_next.map(|an| &*an);
                    assert_eq!(a_next,b_next);
                }
            }
            _ => panic!(),
        }
        *n-=1;
    }
}

fn rand_char(rng: &mut impl Rng) -> char {
    let i = rng.random_range(0..64);
    let ret = match i {
        0..26 => char::from_u32(('a' as u32) + i).unwrap(),
        26..52 => char::from_u32(('A' as u32) + i - 26).unwrap(),
        52..62 => char::from_u32(('0' as u32) + i- 52).unwrap(),
        62 => '-',
        63 => '_',
        _ => panic!("i={}\n",i)
    };
    ret
}