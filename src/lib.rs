#![deny(missing_docs)]
#![deny(clippy::undocumented_unsafe_blocks)]
#![deny(unsafe_op_in_unsafe_fn)]
#![doc = include_str!("../README.md")]
pub extern crate anymap;

#[doc(hidden)]
pub mod static_anymap;
pub extern crate lazy_static;
#[doc(hidden)]
pub mod thread_local_static_anymap;

/// Get a static reference to a generic singleton or initialize it if it doesn't exist.
///
/// ### Example
/// ```rust
/// use std::cell::RefCell;
/// use std::collections::HashMap;
/// use std::ops::{Deref, DerefMut, Mul};
///
/// // The expensive function we're trying to cache using a singleton map, however,
/// // we want the user of the function to determine the type of the elements being
/// // multiplied.
/// fn multiply<T: Mul<Output = T>>(a: T, b: T) -> T {
///     a * b
/// }
///
/// fn multiply_with_cache<T: Mul<Output = T> + Send>(a: T, b: T) -> T
/// where
///     T: std::cmp::Eq,
///     T: Copy,
///     T: 'static,
///     (T, T): std::hash::Hash,
/// {
///     // This is a generic singleton map!!!
///     let map = generic_singleton::get_or_init!(|| RefCell::new(HashMap::new()));
///     let key = (a, b);
///     if map.borrow().contains_key(&key) {
///         *map.borrow().get(&key).unwrap()
///     } else {
///         let result = multiply(a, b);
///         map.borrow_mut().insert(key, result);
///         result
///     }
/// }
///
/// fn main() {
///     // This call will create the AnyMap and the HashMap<i32> and do the multiplication
///     multiply_with_cache::<u32>(10, 10);
///     // This call will only retrieve the value of the multiplication from HashMap<i32>
///     multiply_with_cache::<u32>(10, 10);
///
///     // This call will create a second HashMap< and do the multiplication
///     multiply_with_cache::<i32>(-1, -10);
///     // This call will only retrieve the value of the multiplication from HashMap
///     multiply_with_cache::<i32>(-1, -10);
/// }
/// ```
#[macro_export]
macro_rules! get_or_init {
    ($init:expr) => {{
        use $crate::lazy_static::lazy_static;
        use $crate::static_anymap::StaticAnyMap;

        lazy_static! {
            static ref STATIC_ANY_MAP: StaticAnyMap = StaticAnyMap::default();
        }
        STATIC_ANY_MAP.get_or_init($init)
    }};
}

/// Same as the [get_or_init!] macro but using thread local storage. Similar to the [thread_local!]
/// macro API, we use a closure that yields a mutable reference to your struct. The closure ensures
/// the reference cannot escape to a different thread.
///
/// ### Example
/// ```rust
/// use num_traits::{One, Zero};
/// use std::ops::AddAssign;
///
/// fn generic_call_counter<T: Zero + One + Copy + AddAssign + Send + 'static>() -> T {
///     let mut output = T::zero();
///     generic_singleton::get_or_init_thread_local!(|| T::zero(), |count| {
///         *count += T::one();
///         output = *count;
///     });
///     output
/// }
///
/// fn main() {
///     // Works with usize
///     assert_eq!(generic_call_counter::<usize>(), 1);
///     assert_eq!(generic_call_counter::<usize>(), 2);
///     assert_eq!(generic_call_counter::<usize>(), 3);
///
///     // Works with i32
///     assert_eq!(generic_call_counter::<i32>(), 1);
///     assert_eq!(generic_call_counter::<i32>(), 2);
///     assert_eq!(generic_call_counter::<i32>(), 3);
///
///     // Works with f32
///     assert_eq!(generic_call_counter::<f32>(), 1.0);
///     assert_eq!(generic_call_counter::<f32>(), 2.0);
///     assert_eq!(generic_call_counter::<f32>(), 3.0);
/// }
/// ```
#[macro_export]
macro_rules! get_or_init_thread_local {
     ($init:expr, $with:expr) => {{
         use $crate::thread_local_static_anymap::ThreadLocalStaticAnymap;
         thread_local!(static STATIC_ANY_MAP: ThreadLocalStaticAnymap = ThreadLocalStaticAnymap::default());
         // SAFETY:
         // The reference to the STATIC_ANY_MAP is contained within this macro soo the $init
         // expression cannot possibly reference it.
         unsafe {STATIC_ANY_MAP.with(|map| {
             map.get_or_init_with($init, $with)
         })}
    }};
}

#[cfg(test)]
mod tests {
    use super::*;

    fn testing_function() -> &'static i32 {
        get_or_init!(|| 0)
    }

    fn local_testing_function() -> i32 {
        let mut r = 0;
        get_or_init_thread_local!(|| 0, |a| {
            *a += 1;
            r = *a;
        });
        r
    }

    #[test]
    fn thread_local_works() {
        assert_eq!(local_testing_function(), 1);
        assert_eq!(local_testing_function(), 2);
        assert_eq!(local_testing_function(), 3);
    }

    #[test]
    fn works() {
        let a = testing_function();
        let b = testing_function();
        assert!(std::ptr::eq(a, b));
    }

    #[test]
    fn recursive_call_to_get_or_init_does_not_panic() {
        get_or_init!(|| get_or_init!(|| 0));
    }

    #[test]
    fn initializing_in_different_thread_works() {
        let v = std::thread::spawn(|| get_or_init!(|| String::from("foo")))
            .join()
            .unwrap();

        println!("{v}"); // Check for use-after-free, to be sure bugs are caught, run with miri
    }
}
