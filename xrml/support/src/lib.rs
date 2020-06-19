#![cfg_attr(not(feature = "std"), no_std)]

pub mod base58;
pub mod rlog;
pub use rlog::RUNTIME_TARGET;

use frame_support::dispatch::{DispatchError, DispatchResult};
pub use frame_support::fail;

/// Although xss is imperceptible on-chain, we merely want to make it look safer off-chain.
#[inline]
pub fn xss_check(input: &[u8]) -> DispatchResult {
    if input.contains(&b'<') || input.contains(&b'>') {
        Err(DispatchError::Other(
            "'<' and '>' are not allowed, which could be abused off-chain.",
        ))?;
    }
    Ok(())
}

#[cfg(feature = "std")]
pub mod _std {
    use std::fmt;

    pub struct Str<'a>(pub &'a String);
    impl<'a> fmt::Debug for Str<'a> {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            write!(f, "{}", self.0)
        }
    }
    #[inline]
    pub fn u8array_to_string(s: &[u8]) -> String {
        String::from_utf8_lossy(s).into_owned()
    }

    #[inline]
    pub fn u8array_to_addr(s: &[u8]) -> String {
        for i in s {
            // 0x30 = '0' 0x39 = '9'; 0x41 = 'A' 0x7A = 'z'
            if (0x30 <= *i && *i <= 0x39) || (0x41 <= *i && *i <= 0x7A) {
                continue;
            } else {
                // 0x30 = '0' 0x7A = 'z'
                return u8array_to_hex(s); // when any item is not a char, use hex to decode it
            }
        }
        return u8array_to_string(s);
    }
    #[inline]
    pub fn u8array_to_hex(s: &[u8]) -> String {
        use rustc_hex::ToHex;
        let s: String = s.to_hex();
        "0x".to_string() + &s
    }
    #[inline]
    pub fn try_hex_or_str(src: &[u8]) -> String {
        let check_is_str = |src: &[u8]| -> bool {
            for c in src {
                // 0x21 = !, 0x71 = ~
                if 0x21 <= *c && *c <= 0x7E {
                    continue;
                } else {
                    return false;
                }
            }
            return true;
        };
        if check_is_str(src) {
            u8array_to_string(src)
        } else {
            u8array_to_hex(src)
        }
    }
}

#[cfg(feature = "std")]
#[macro_export]
macro_rules! str {
    ( $x:expr ) => {{
        use $crate::_std::u8array_to_string;
        $crate::_std::Str(&u8array_to_string(&$x))
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
        use $crate::_std::u8array_to_addr;
        $crate::_std::Str(&u8array_to_addr(&$x))
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
        use $crate::_std::try_hex_or_str;
        $crate::_std::Str(&try_hex_or_str(&$x))
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
