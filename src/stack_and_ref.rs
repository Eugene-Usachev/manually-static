use std::ops::Deref;
#[cfg(debug_assertions)]
use std::sync::atomic::{AtomicBool, Ordering};
#[cfg(debug_assertions)]
use std::sync::Arc;

/// `ManuallyStatic<T>` holds a value `T` and provides references to it
/// with checking with `debug_assertions`.
///
/// In debug builds, it tracks if the original `ManuallyStatic` instance
/// has been dropped, causing any derived `ManuallyStaticRef`
/// to panic upon dereference if the original
/// owner is no longer alive.
///
/// This is useful for simulating a 'static lifetime where you want
/// runtime checks for use-after-free in debug environments.
///
/// # Example
///
/// ```rust
/// use manually_static::ManuallyStatic;
/// use std::thread;
/// use std::time::Duration;
///
/// struct AppConfig {
///     version: String,
/// }
///
/// let config = ManuallyStatic::new(AppConfig {
///     version: String::from("1.0.0"),
/// });
///
/// // Get a 'static reference to the config.
/// // This is where ManuallyStatic shines, allowing us to pass
/// // a reference that the compiler would normally complain about
/// // without complex ownership transfers or Arc for simple reads.
/// let config_ref = config.get_ref();
///
/// let handle = thread::spawn(move || {
///     // In this thread, we can safely access the config via the 'static reference.
///     // In debug builds, if `config` (the original ManuallyStatic) was dropped
///     // before this thread accessed it, it would panic.
///
///     thread::sleep(Duration::from_millis(100)); // Simulate some work
///
///     println!("Thread: App Version: {}", config_ref.version);
/// });
///
/// handle.join().unwrap();
///
/// // config is dropped here after the thread has finished
/// ```
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
                "Attempted to dereference ManuallyStaticRef after ManuallyStatic was dropped!"
            );
        }

        unsafe { &*self.value_ref }
    }
}

unsafe impl<T: Send> Send for ManuallyStaticRef<T> {}
unsafe impl<T: Sync> Sync for ManuallyStaticRef<T> {}

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
    #[cfg(debug_assertions)] // This test only runs in debug mode
    #[should_panic(
        expected = "Attempted to dereference ManuallyStaticRef after ManuallyStatic was dropped!"
    )]
    fn test_manually_static_ref_panics_on_drop() {
        let ms_ref;

        {
            let ms = ManuallyStatic::new(42);

            ms_ref = ms.get_ref();
        }

        let _ = *ms_ref;
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
