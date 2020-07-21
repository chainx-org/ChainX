pub use log::*;

#[cfg(feature = "std")]
#[macro_export]
macro_rules! str {
    ( $x:expr ) => {{
        use $crate::x_std::u8array_to_string;
        $crate::x_std::Str(&u8array_to_string(&$x))
    }};
}

#[cfg(not(feature = "std"))]
#[macro_export]
macro_rules! str {
    ( $x:expr ) => {{
        &$x
    }};
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
        use $crate::x_std::u8array_to_addr;
        $crate::x_std::Str(&u8array_to_addr(&$x))
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
        use $crate::x_std::try_hex_or_str;
        $crate::x_std::Str(&try_hex_or_str(&$x))
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

mod log {
    pub const RUNTIME_TARGET: &'static str = "runtime";

    #[macro_export]
    macro_rules! error {
        (target: $target:expr, $($arg:tt)+) => (
            frame_support::debug::error!(target: $target, $($arg)+);
        );
        ($($arg:tt)+) => (
            frame_support::debug::error!(target: "runtime", $($arg)+);
        )
    }

    #[macro_export]
    macro_rules! warn {
        (target: $target:expr, $($arg:tt)+) => (
            frame_support::debug::warn!(target: $target, $($arg)+);
        );
        ($($arg:tt)+) => (
            frame_support::debug::warn!(target: "runtime", $($arg)+);
        )
    }

    #[macro_export]
    macro_rules! info {
        (target: $target:expr, $($arg:tt)+) => (
            frame_support::debug::info!(target: $target, $($arg)+);
        );
        ($($arg:tt)+) => (
            $crate::info!(target: "runtime", $($arg)+);
            // frame_support::debug::info!(target: "runtime", $($arg)+);
        )
    }

    #[macro_export]
    macro_rules! debug {
        (target: $target:expr, $($arg:tt)+) => (
            frame_support::debug::debug!(target: $target, $($arg)+);
        );
        ($($arg:tt)+) => (
            frame_support::debug::debug!(target: "runtime", $($arg)+);
        )
    }

    #[macro_export]
    macro_rules! trace {
        (target: $target:expr, $($arg:tt)+) => (
            frame_support::debug::trace!(target: $target, $($arg)+);
        );
        ($($arg:tt)+) => (
            frame_support::debug::trace!(target: "runtime", $($arg)+);
        )
    }
}
