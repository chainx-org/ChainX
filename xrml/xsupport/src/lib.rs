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

/// print trustee accountid and his name, DANGER, this macro only use when you ensure he is a trustee
#[cfg(feature = "std")]
#[macro_export]
macro_rules! trustees {
    ( $( $x:expr )+ ) => {
        $($x)+.iter()
            .map(|v|
                 <xaccounts::IntentionNameOf<T>>::get(v)
                 .map(|name| format!("{:?}({:})", v, u8array_to_string(&name)))
                 .unwrap()
            )
            .collect::<Vec<_>>()
    };
}

// Util for displaying validator's name instead of AccountId.
/// print validator accountid and his name, DANGER, this macro only use when you ensure he is an intention
#[cfg(feature = "std")]
#[macro_export]
macro_rules! validators {
    ( $( $x:expr )+ ) => {
        $($x)+.iter()
            .map(|(v, w)| {
                use xsupport::u8array_to_string;
                (
                    <xaccounts::IntentionNameOf<T>>::get(v)
                        .map(|name| format!("{:?}({:})", v, u8array_to_string(&name)))
                        .unwrap(),
                    w,
                )
            })
            .collect::<Vec<_>>()
    };
}

/// print validator accountid and his name, DANGER, this macro only use when you ensure he is an intention
#[cfg(feature = "std")]
#[macro_export]
macro_rules! who {
    ( $( $x:ident )+ ) => {

        <xaccounts::IntentionNameOf<T>>::get($($x)+)
            .map(|name| {
                use xsupport::u8array_to_string;
                format!("{:?}({:})", $($x)+, u8array_to_string(&name))
            })
            .unwrap()
    };
}
