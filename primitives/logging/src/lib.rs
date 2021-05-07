// Copyright 2019-2020 ChainX Project Authors. Licensed under GPL-3.0.

//! Some logging macros wrapper used in ChainX.

#![cfg_attr(not(feature = "std"), no_std)]

pub const RUNTIME_TARGET: &str = "runtime";

#[macro_export]
macro_rules! error {
    (target: $target:expr, $($arg:tt)+) => (
        log::error!(target: $target, $($arg)+);
    );
    ($($arg:tt)+) => (
        #[cfg(feature = "std")]
        log::error!(target: &format!("runtime::{}", module_path!()), $($arg)+);
        #[cfg(not(feature = "std"))]
        log::error!(target: "runtime", $($arg)+);
    )
}

#[macro_export]
macro_rules! warn {
     (target: $target:expr, $($arg:tt)+) => (
         log::warn!(target: $target, $($arg)+);
     );
     ($($arg:tt)+) => (
        #[cfg(feature = "std")]
        log::warn!(target: &format!("runtime::{}", module_path!()), $($arg)+);
        #[cfg(not(feature = "std"))]
        log::warn!(target: "runtime", $($arg)+);
     )
 }

#[macro_export]
macro_rules! info {
    (target: $target:expr, $($arg:tt)+) => (
        log::info!(target: $target, $($arg)+);
    );
    ($($arg:tt)+) => (
        #[cfg(feature = "std")]
        log::info!(target: &format!("runtime::{}", module_path!()), $($arg)+);
        #[cfg(not(feature = "std"))]
        log::info!(target: "runtime", $($arg)+);
    )
}

#[macro_export]
macro_rules! debug {
    (target: $target:expr, $($arg:tt)+) => (
        log::debug!(target: $target, $($arg)+);
    );
    ($($arg:tt)+) => (
        #[cfg(feature = "std")]
        log::debug!(target: &format!("runtime::{}", module_path!()), $($arg)+);
        #[cfg(not(feature = "std"))]
        log::debug!(target: "runtime", $($arg)+);
    )
}

#[macro_export]
macro_rules! trace {
    (target: $target:expr, $($arg:tt)+) => (
        log::trace!(target: $target, $($arg)+);
    );
    ($($arg:tt)+) => (
        #[cfg(feature = "std")]
        log::trace!(target: &format!("runtime::{}", module_path!()), $($arg)+);
        #[cfg(not(feature = "std"))]
        log::trace!(target: "runtime", $($arg)+);
    )
}
