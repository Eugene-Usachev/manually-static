use std::ops::Deref;
#[cfg(debug_assertions)]
use std::sync::atomic::AtomicUsize;
#[cfg(debug_assertions)]
use std::sync::atomic::{AtomicBool, Ordering};
#[cfg(debug_assertions)]
use std::sync::Arc;

/// `ManuallyStaticPtr<T>` allocates a value `T` on the heap and provides
/// a raw pointer to it. It requires manual deallocation
/// via the [`free`](ManuallyStaticPtr::free) method.
///
/// In debug builds, it tracks if the pointer has already been freed,
/// causing a panic if [`free`](ManuallyStaticPtr::free) is called multiple times or if the pointer
/// is dereferenced after being freed.
///
/// # Example
///
/// ```rust
/// use manually_static::ManuallyStaticPtr;
/// use std::sync::Mutex;
/// use std::array;
///
/// const N: usize = 10280;
/// const PAR: usize = 16;
///
/// #[allow(dead_code, reason = "It is an example.")]
/// struct Pool(Mutex<([Vec<u8>; N], usize)>);
///
/// fn main() {
///     let pool = ManuallyStaticPtr::new(Pool(Mutex::new((array::from_fn(|_| Vec::new()), 0))));
///     let mut joins = Vec::with_capacity(PAR);
///
///     for _ in 0..PAR {
///         #[allow(unused_variables, reason = "It is an example.")]
///         let pool = pool.clone();
///
///         joins.push(std::thread::spawn(move || {
///             /* ... do some work ... */
///         }));
///     }
///
///     for join in joins {
///         join.join().unwrap();
///     }
///
///     unsafe { pool.free(); }
/// }
/// ```
pub struct ManuallyStaticPtr<T> {
    ptr: *mut T,
    /// This flag is only present in debug builds (`cfg(debug_assertions)`).
    /// It is set to `true` when the `ManuallyStaticPtr` instance is freed.
    #[cfg(debug_assertions)]
    is_freed: Arc<AtomicBool>,
    #[cfg(debug_assertions)]
    ref_count: Arc<AtomicUsize>,
}

impl<T> ManuallyStaticPtr<T> {
    /// Allocates a new `ManuallyStaticPtr` instance by moving `value` to the heap.
    ///
    /// # Examples
    ///
    /// ```
    /// use manually_static::ManuallyStaticPtr;
    ///
    /// let my_ptr = ManuallyStaticPtr::new(42);
    ///
    /// assert_eq!(*my_ptr, 42);
    ///
    /// // Don't forget to call `free` when done!
    /// unsafe { my_ptr.free(); }
    /// ```
    pub fn new(value: T) -> Self {
        Self {
            ptr: Box::into_raw(Box::new(value)),
            #[cfg(debug_assertions)]
            is_freed: Arc::new(AtomicBool::new(false)),
            #[cfg(debug_assertions)]
            ref_count: Arc::new(AtomicUsize::new(1)),
        }
    }

    /// Deallocates the memory associated with this `ManuallyStaticPtr`.
    ///
    /// # Safety
    ///
    /// This function is `unsafe` because:
    /// - It must be called exactly once for each `ManuallyStaticPtr` instance.
    ///   Calling it more than once will result in a double-free, leading to
    ///   undefined behavior. In debug builds, this will panic.
    /// - Not calling `free` will result in a memory leak.
    /// - The raw pointer must not be aliased or used after `free` is called.
    ///
    /// # Panics
    ///
    /// In debug builds, this function will panic if the pointer
    /// has already been freed.
    ///
    /// # Examples
    ///
    /// ```
    /// use manually_static::ManuallyStaticPtr;
    ///
    /// let my_ptr = ManuallyStaticPtr::new(vec![1, 2, 3]);
    ///
    /// // ... use my_ptr ...
    ///
    /// unsafe { my_ptr.free(); } // Explicitly free the memory
    ///
    /// // my_ptr is now consumed and cannot be used
    /// ```
    pub unsafe fn free(self) {
        #[cfg(debug_assertions)]
        {
            assert!(
                !self.is_freed.swap(true, Ordering::AcqRel),
                "Attempted to double free ManuallyStaticPtr!"
            );
        }

        drop(Box::from_raw(self.ptr));
    }
}

impl<T> Deref for ManuallyStaticPtr<T> {
    type Target = T;

    fn deref(&self) -> &T {
        #[cfg(debug_assertions)]
        {
            assert!(
                !self.is_freed.load(Ordering::Acquire),
                "ManuallyStaticPtr: Attempted to dereference a freed pointer!"
            );
        }

        unsafe { &*self.ptr }
    }
}

impl<T> Clone for ManuallyStaticPtr<T> {
    fn clone(&self) -> Self {
        #[cfg(debug_assertions)]
        {
            self.ref_count.fetch_add(1, Ordering::AcqRel);
        }

        Self {
            ptr: self.ptr,
            #[cfg(debug_assertions)]
            is_freed: self.is_freed.clone(),
            #[cfg(debug_assertions)]
            ref_count: self.ref_count.clone(),
        }
    }
}

unsafe impl<T: Send> Send for ManuallyStaticPtr<T> {}
unsafe impl<T: Sync> Sync for ManuallyStaticPtr<T> {}

#[cfg(debug_assertions)]
impl<T> Drop for ManuallyStaticPtr<T> {
    fn drop(&mut self) {
        let prev = self.ref_count.fetch_sub(1, Ordering::AcqRel);

        if prev == 1 {
            assert!(
                self.is_freed.load(Ordering::Acquire),
                "Attempted to drop the last ManuallyStaticPtr instance before it was freed!"
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_manually_static_ptr_creation_and_deref() {
        let ptr = ManuallyStaticPtr::new(42);

        assert_eq!(*ptr, 42);

        unsafe {
            ptr.free();
        }
    }

    #[test]
    #[cfg(debug_assertions)]
    #[should_panic(expected = "Attempted to double free ManuallyStaticPtr!")]
    fn test_manually_static_ptr_double_free_panics() {
        let ptr = ManuallyStaticPtr::new(1);
        let ptr2 = ptr.clone();

        unsafe {
            ptr.free();
        }

        // This second call should panic in debug mode
        unsafe {
            ptr2.free();
        }
    }

    #[test]
    #[cfg(debug_assertions)]
    #[should_panic(expected = "ManuallyStaticPtr: Attempted to dereference a freed pointer!")]
    fn test_manually_static_ptr_deref_after_free_panics() {
        let ptr = ManuallyStaticPtr::new(2);
        let ptr2 = ptr.clone();

        unsafe {
            ptr.free();
        }

        let _ = *ptr2;
    }

    #[test]
    #[cfg(debug_assertions)]
    #[should_panic(
        expected = "Attempted to drop the last ManuallyStaticPtr instance before it was freed!"
    )]
    fn test_manually_static_ptr_drop_without_free_panics() {
        // Create a ManuallyStaticPtr, but don't call `free()`
        let _ptr = ManuallyStaticPtr::new(3);
    }
}
