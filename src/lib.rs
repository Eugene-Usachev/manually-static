#![deny(clippy::all)]
#![deny(clippy::assertions_on_result_states)]
#![deny(clippy::match_wild_err_arm)]
#![deny(clippy::allow_attributes_without_reason)]
#![warn(clippy::pedantic)]
#![warn(clippy::nursery)]
#![warn(clippy::cargo)]
#![allow(
    clippy::missing_const_for_fn,
    reason = "Since we cannot make a constant function non-constant after its release,
    we need to look for a reason to make it constant, and not vice versa."
)]
#![allow(
    clippy::must_use_candidate,
    reason = "It is better to developer think about it."
)]
#![allow(
    clippy::missing_errors_doc,
    reason = "Unless the error is something special,
    the developer should document it."
)]

use std::ops::{Deref, DerefMut};
#[cfg(debug_assertions)]
use std::sync::atomic::{AtomicBool, Ordering};
#[cfg(debug_assertions)]
use std::sync::Arc;

/// `ManuallyStatic<T>` holds a value `T` and provides references to it
/// with checking with `debug_assertions`.
///
/// In debug builds, it tracks if the original `ManuallyStatic` instance
/// has been dropped, causing any derived `ManuallyStaticRef` or
/// `ManuallyStaticRefMut` to panic upon dereference if the original
/// owner is no longer alive.
///
/// This is useful for simulating a 'static lifetime where you want
/// runtime checks for use-after-free in debug environments.
pub struct ManuallyStatic<T> {
    value: T,
    /// This flag is only present in debug builds (`cfg(debug_assertions)`).
    /// It is set to `true` when the `ManuallyStatic` instance is dropped.
    #[cfg(debug_assertions)]
    was_dropped: Arc<AtomicBool>,
}

impl<T> ManuallyStatic<T> {
    /// Creates a new `ManuallyStatic` instance holding the given value.
    pub fn new(value: T) -> Self {
        Self {
            value,
            #[cfg(debug_assertions)]
            was_dropped: Arc::new(AtomicBool::new(false)),
        }
    }

    /// Returns a [`ManuallyStaticRef`] that provides immutable access to the
    /// contained value. In debug builds, dereferencing this will panic
    /// if the original `ManuallyStatic` instance has been dropped.
    pub fn get_ref(&self) -> ManuallyStaticRef<T> {
        ManuallyStaticRef {
            value_ref: &self.value,
            #[cfg(debug_assertions)]
            was_dropped: self.was_dropped.clone(),
        }
    }

    /// Returns a [`ManuallyStaticRefMut`] that provides mutable access to the
    /// contained value. In debug builds, dereferencing this will panic
    /// if the original `ManuallyStatic` instance has been dropped.
    pub fn get_mut(&mut self) -> ManuallyStaticRefMut<T> {
        ManuallyStaticRefMut {
            value_ref_mut: &mut self.value,
            #[cfg(debug_assertions)]
            was_dropped: self.was_dropped.clone(),
        }
    }
}

/// Implements the `Drop` trait for [`ManuallyStatic<T>`] only in debug builds.
/// When [`ManuallyStatic`] is dropped, it sets the `was_dropped` flag to `true`.
#[cfg(debug_assertions)]
impl<T> Drop for ManuallyStatic<T> {
    fn drop(&mut self) {
        self.was_dropped.store(true, Ordering::Release);
    }
}

/// A reference to the value held by [`ManuallyStatic<T>`].
/// In debug builds, it will panic if dereferenced after the
/// original [`ManuallyStatic`] has been dropped.
pub struct ManuallyStaticRef<T> {
    value_ref: *const T,
    #[cfg(debug_assertions)]
    was_dropped: Arc<AtomicBool>,
}

impl<T> Deref for ManuallyStaticRef<T> {
    type Target = T;

    fn deref(&self) -> &T {
        #[cfg(debug_assertions)]
        {
            assert!(
                !self.was_dropped.load(Ordering::Acquire),
                "ManuallyStaticRef: Attempted to dereference value after ManuallyStatic was dropped!"
            );
        }

        unsafe { &*self.value_ref }
    }
}

/// A mutable reference to the value held by `ManuallyStatic<T>`.
/// In debug builds, it will panic if dereferenced after the
/// original `ManuallyStatic` has been dropped.
pub struct ManuallyStaticRefMut<T> {
    value_ref_mut: *mut T,
    #[cfg(debug_assertions)]
    was_dropped: Arc<AtomicBool>,
}

impl<T> Deref for ManuallyStaticRefMut<T> {
    type Target = T;

    fn deref(&self) -> &T {
        #[cfg(debug_assertions)]
        {
            assert!(
                !self.was_dropped.load(Ordering::Acquire),
                "ManuallyStaticRefMut: Attempted to dereference value after ManuallyStatic was dropped!"
            );
        }

        unsafe { &*self.value_ref_mut }
    }
}

impl<T> DerefMut for ManuallyStaticRefMut<T> {
    fn deref_mut(&mut self) -> &mut T {
        #[cfg(debug_assertions)]
        {
            assert!(
                !self.was_dropped.load(Ordering::Acquire),
                "ManuallyStaticRefMut: Attempted to dereference value after ManuallyStatic was dropped!"
            );
        }

        unsafe { &mut *self.value_ref_mut }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_manually_static_ref_access() {
        let ms = ManuallyStatic::new(42);
        let ms_ref = ms.get_ref();

        assert_eq!(*ms_ref, 42);
    }

    #[test]
    fn test_manually_static_ref_mut_access() {
        let mut ms = ManuallyStatic::new(42);
        let mut ms_mut = ms.get_mut();

        *ms_mut = 100;

        assert_eq!(*ms_mut, 100);
        assert_eq!(*ms.get_ref(), 100);
    }

    #[test]
    #[cfg(debug_assertions)] // This test only runs in debug mode
    #[should_panic(expected = "Attempted to dereference value after ManuallyStatic was dropped!")]
    fn test_manually_static_ref_panics_on_drop() {
        let ms_ref;

        {
            let ms = ManuallyStatic::new(42);

            ms_ref = ms.get_ref();
        }

        let _ = *ms_ref;
    }

    #[test]
    #[cfg(debug_assertions)] // This test only runs in debug mode
    #[should_panic(expected = "Attempted to dereference value after ManuallyStatic was dropped!")]
    fn test_manually_static_ref_mut_panics_on_drop() {
        let ms_mut;

        {
            let mut ms = ManuallyStatic::new(42);

            ms_mut = ms.get_mut();
        }

        let _ = *ms_mut;
    }

    #[test]
    #[cfg(not(debug_assertions))] // This test only runs in release mode
    fn test_manually_static_ref_no_panic_on_drop_release_mode() {
        let ms_ptr;

        {
            let ms = ManuallyStatic::new(42);

            ms_ptr = ms.get_ref();
        }

        let _ = *ms_ptr;
    }
}
