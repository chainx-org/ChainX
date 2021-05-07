// Copyright 2019-2020 ChainX Project Authors. Licensed under GPL-3.0.

//! Some logging macros wrapper used in ChainX.

#![cfg_attr(not(feature = "std"), no_std)]

pub const RUNTIME_TARGET: &str = "runtime";

#[macro_export]
macro_rules! error {
    (target: $target:expr, $($arg:tt)+) => (
        frame_support::log::error!(target: $target, $($arg)+);
    );
    ($($arg:tt)+) => (
        #[cfg(feature = "std")]
        frame_support::log::error!(target: &format!("runtime::{}", module_path!()), $($arg)+);
        #[cfg(not(feature = "std"))]
        frame_support::log::error!(target: "runtime", $($arg)+);
    )
}

#[macro_export]
macro_rules! warn {
     (target: $target:expr, $($arg:tt)+) => (
         frame_support::log::warn!(target: $target, $($arg)+);
     );
     ($($arg:tt)+) => (
        #[cfg(feature = "std")]
        frame_support::log::warn!(target: &format!("runtime::{}", module_path!()), $($arg)+);
        #[cfg(not(feature = "std"))]
        frame_support::log::warn!(target: "runtime", $($arg)+);
     )
 }

#[macro_export]
macro_rules! info {
    (target: $target:expr, $($arg:tt)+) => (
        frame_support::log::info!(target: $target, $($arg)+);
    );
    ($($arg:tt)+) => (
        #[cfg(feature = "std")]
        frame_support::log::info!(target: &format!("runtime::{}", module_path!()), $($arg)+);
        #[cfg(not(feature = "std"))]
        frame_support::log::info!(target: "runtime", $($arg)+);
    )
}

#[macro_export]
macro_rules! debug {
    (target: $target:expr, $($arg:tt)+) => (
        frame_support::log::debug!(target: $target, $($arg)+);
    );
    ($($arg:tt)+) => (
        #[cfg(feature = "std")]
        frame_support::log::debug!(target: &format!("runtime::{}", module_path!()), $($arg)+);
        #[cfg(not(feature = "std"))]
        frame_support::log::debug!(target: "runtime", $($arg)+);
    )
}

#[macro_export]
macro_rules! trace {
    (target: $target:expr, $($arg:tt)+) => (
        frame_support::log::trace!(target: $target, $($arg)+);
    );
    ($($arg:tt)+) => (
        #[cfg(feature = "std")]
        frame_support::log::trace!(target: &format!("runtime::{}", module_path!()), $($arg)+);
        #[cfg(not(feature = "std"))]
        frame_support::log::trace!(target: "runtime", $($arg)+);
    )
}
