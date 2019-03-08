// Copyright 2018 Chainpool.

#![cfg_attr(not(feature = "std"), no_std)]

pub mod logger;
pub mod storage;

//use sr_io as runtime_io;
use sr_primitives as runtime_primitives;
use sr_std as rstd;
use srml_support as support;

#[cfg(feature = "std")]
pub use self::logger::{u8array_to_hex, u8array_to_string};
pub use support::fail;

#[macro_export]
macro_rules! ensure_with_errorlog {
	( $x:expr, $y:expr, $($arg:tt)*) => {{
		if !$x {
		    #[cfg(feature = "std")]
		    $crate::error!("{}|{}", $y, format!($($arg)*));
			$crate::fail!($y);
		}
	}}
}
