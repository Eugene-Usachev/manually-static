//! # `ManuallyStatic`
//!
//! `ManuallyStatic` is a crate that provides a way to simulate a 'static lifetime
//! where you want runtime checks for use-after-free in debug environments.
//!
//! Read [`ManuallyStatic`'s documentation](ManuallyStatic)
//! and [`ManuallyStaticPtr`'s documentation](ManuallyStaticPtr) for more information.
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

mod ptr;
mod stack_and_ref;

pub use ptr::ManuallyStaticPtr;
pub use stack_and_ref::{ManuallyStatic, ManuallyStaticRef};
