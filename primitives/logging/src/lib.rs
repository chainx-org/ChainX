// Copyright 2019-2020 ChainX Project Authors. Licensed under GPL-3.0.

//! Some logging macros wrapper used in ChainX.

#![cfg_attr(not(feature = "std"), no_std)]

pub const RUNTIME_TARGET: &str = "runtime";

#[macro_export]
macro_rules! error {
    (target: $target:expr, $($arg:tt)+) => (
        frame_support::debug::error!(target: $target, $($arg)+);
    );
    ($($arg:tt)+) => (
        #[cfg(feature = "std")]
        frame_support::debug::error!(target: &format!("runtime::{}", module_path!()), $($arg)+);
        #[cfg(not(feature = "std"))]
        frame_support::debug::error!(target: "runtime", $($arg)+);
    )
}

#[macro_export]
macro_rules! warn {
     (target: $target:expr, $($arg:tt)+) => (
         frame_support::debug::warn!(target: $target, $($arg)+);
     );
     ($($arg:tt)+) => (
        #[cfg(feature = "std")]
        frame_support::debug::warn!(target: &format!("runtime::{}", module_path!()), $($arg)+);
        #[cfg(not(feature = "std"))]
        frame_support::debug::warn!(target: "runtime", $($arg)+);
     )
 }

#[macro_export]
macro_rules! info {
    (target: $target:expr, $($arg:tt)+) => (
        frame_support::debug::info!(target: $target, $($arg)+);
    );
    ($($arg:tt)+) => (
        #[cfg(feature = "std")]
        frame_support::debug::info!(target: &format!("runtime::{}", module_path!()), $($arg)+);
        #[cfg(not(feature = "std"))]
        frame_support::debug::info!(target: "runtime", $($arg)+);
    )
}

#[macro_export]
macro_rules! debug {
    (target: $target:expr, $($arg:tt)+) => (
        frame_support::debug::debug!(target: $target, $($arg)+);
    );
    ($($arg:tt)+) => (
        #[cfg(feature = "std")]
        frame_support::debug::debug!(target: &format!("runtime::{}", module_path!()), $($arg)+);
        #[cfg(not(feature = "std"))]
        frame_support::debug::debug!(target: "runtime", $($arg)+);
    )
}

#[macro_export]
macro_rules! trace {
    (target: $target:expr, $($arg:tt)+) => (
        frame_support::debug::trace!(target: $target, $($arg)+);
    );
    ($($arg:tt)+) => (
        #[cfg(feature = "std")]
        frame_support::debug::trace!(target: &format!("runtime::{}", module_path!()), $($arg)+);
        #[cfg(not(feature = "std"))]
        frame_support::debug::trace!(target: "runtime", $($arg)+);
    )
}
