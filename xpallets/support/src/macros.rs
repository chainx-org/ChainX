// Copyright 2019-2020 ChainX Project Authors. Licensed under GPL-3.0.

pub use self::log::*;

#[cfg(feature = "std")]
#[macro_export]
macro_rules! str {
    ( $x:expr ) => {
        $crate::x_std::Str(&$crate::x_std::as_string(&$x))
    };
}

#[cfg(not(feature = "std"))]
#[macro_export]
macro_rules! str {
    ( $x:expr ) => {
        &$x
    };
}

#[macro_export]
macro_rules! token {
    ( $x:expr ) => {
        $crate::str!($x)
    };
}

#[cfg(feature = "std")]
#[macro_export]
macro_rules! try_addr {
    ( $x:expr ) => {{
        $crate::x_std::Str(&$crate::x_std::as_addr(&$x))
    }};
}

#[cfg(not(feature = "std"))]
#[macro_export]
macro_rules! try_addr {
    ( $x:expr ) => {{
        &$x
    }};
}

#[cfg(feature = "std")]
#[macro_export]
macro_rules! try_hex {
    ( $x:expr ) => {{
        $crate::x_std::Str(&$crate::x_std::try_hex_or_str(&$x))
    }};
}

#[cfg(not(feature = "std"))]
#[macro_export]
macro_rules! try_hex {
    ( $x:expr ) => {{
        &$x
    }};
}

#[cfg(feature = "std")]
#[macro_export]
macro_rules! ensure_with_errorlog {
    ( $x:expr, $y:expr, $($arg:tt)*) => {{
        if !$x {
            $crate::error!("{:?}|{}", $y, format!($($arg)*));
            $crate::fail!($y);
        }
    }}
    }

#[cfg(not(feature = "std"))]
#[macro_export]
macro_rules! ensure_with_errorlog {
    ( $x:expr, $y:expr, $($arg:tt)*) => {{
        if !$x {
            $crate::fail!($y);
        }
    }};
}

pub mod log {
    pub const RUNTIME_TARGET: &str = "runtime";

    #[macro_export]
    macro_rules! error {
        (target: $target:expr, $($arg:tt)+) => (
            frame_support::debug::error!(target: $target, $($arg)+);
        );
        ($($arg:tt)+) => (
            #[cfg(feature = "std")]
            $crate::error!(target: &format!("runtime::{}:{}", module_path!(), line!()), $($arg)+);
            #[cfg(not(feature = "std"))]
            $crate::error!(target: "runtime", $($arg)+);
        )
    }

    #[macro_export]
    macro_rules! warn {
        (target: $target:expr, $($arg:tt)+) => (
            frame_support::debug::warn!(target: $target, $($arg)+);
        );
        ($($arg:tt)+) => (
            #[cfg(feature = "std")]
            $crate::warn!(target: &format!("runtime::{}:{}", module_path!(), line!()), $($arg)+);
            #[cfg(not(feature = "std"))]
            $crate::warn!(target: "runtime", $($arg)+);
        )
    }

    #[macro_export]
    macro_rules! info {
        (target: $target:expr, $($arg:tt)+) => (
            frame_support::debug::info!(target: $target, $($arg)+);
        );
        ($($arg:tt)+) => (
            #[cfg(feature = "std")]
            $crate::info!(target: &format!("runtime::{}", module_path!()), $($arg)+);
            #[cfg(not(feature = "std"))]
            $crate::info!(target: "runtime", $($arg)+);
        )
    }

    #[macro_export]
    macro_rules! debug {
        (target: $target:expr, $($arg:tt)+) => (
            frame_support::debug::debug!(target: $target, $($arg)+);
        );
        ($($arg:tt)+) => (
            #[cfg(feature = "std")]
            $crate::debug!(target: &format!("runtime::{}:{}", module_path!(), line!()), $($arg)+);
            #[cfg(not(feature = "std"))]
            $crate::debug!(target: "runtime", $($arg)+);
        )
    }

    #[macro_export]
    macro_rules! trace {
        (target: $target:expr, $($arg:tt)+) => (
            frame_support::debug::trace!(target: $target, $($arg)+);
        );
        ($($arg:tt)+) => (
            #[cfg(feature = "std")]
            $crate::trace!(target: &format!("runtime::{}:{}", module_path!(), line!()), $($arg)+);
            #[cfg(not(feature = "std"))]
            $crate::trace!(target: "runtime", $($arg)+);
        )
    }
}
