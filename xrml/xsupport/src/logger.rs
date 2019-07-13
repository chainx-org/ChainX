#[cfg(feature = "std")]
pub use log::{
    debug as debug_m, error as error_m, info as info_m, trace as trace_m, warn as warn_m,
};

#[macro_export]
macro_rules! error {
    (target: $target:expr, $($arg:tt)+) => (
        #[cfg(feature = "std")]
        $crate::logger::error_m!(target: $target, "[runtime|{}] {}", module_path!(), format!($($arg)*));
    );
    ($($arg:tt)*) => (
        #[cfg(feature = "std")]
        $crate::logger::error_m!(target: "runtime", "[runtime|{}|{}L] {}", module_path!(), line!(), format!($($arg)*));
    )
}

#[macro_export]
macro_rules! warn {
    (target: $target:expr, $($arg:tt)+) => (
        #[cfg(feature = "std")]
        $crate::logger::warn_m!(target: $target, "[runtime|{}] {}", module_path!(), format!($($arg)*));
    );
    ($($arg:tt)*) => (
        #[cfg(feature = "std")]
        $crate::logger::warn_m!(target: "runtime", "[runtime|{}] {}", module_path!(), format!($($arg)*));
    )
}
#[macro_export]
macro_rules! info {
    (target: $target:expr, $($arg:tt)+) => (
        #[cfg(feature = "std")]
        $crate::logger::info_m!(target: $target, "[runtime|{}] {}", module_path!(), format!($($arg)*));
    );
    ($($arg:tt)*) => (
        #[cfg(feature = "std")]
        $crate::logger::info_m!(target: "runtime", "[runtime|{}] {}", module_path!(), format!($($arg)*));
    )
}
#[macro_export]
macro_rules! debug {
    (target: $target:expr, $($arg:tt)+) => (
        #[cfg(feature = "std")]
        $crate::logger::debug_m!(target: $target, "[runtime|{}] {}", module_path!(), format!($($arg)*));
    );
    ($($arg:tt)*) => (
        #[cfg(feature = "std")]
        $crate::logger::debug_m!(target: "runtime", "[runtime|{}] {}", module_path!(), format!($($arg)*));
    )
}
#[macro_export]
macro_rules! trace {
    (target: $target:expr, $($arg:tt)+) => (
        #[cfg(feature = "std")]
        $crate::logger::trace_m!(target: $target, "[runtime|{}] {}", module_path!(), format!($($arg)*));
    );
    ($($arg:tt)*) => (
        #[cfg(feature = "std")]
        $crate::logger::trace_m!(target: "runtime", "[runtime|{}] {}", module_path!(), format!($($arg)*));
    )
}

#[cfg(feature = "std")]
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

#[cfg(feature = "std")]
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

#[cfg(feature = "std")]
#[inline]
pub fn u8array_to_string(s: &[u8]) -> String {
    String::from_utf8_lossy(s).into_owned()
}

#[cfg(feature = "std")]
#[inline]
pub fn u8array_to_hex(s: &[u8]) -> String {
    use rustc_hex::ToHex;
    let s: String = s.to_hex();
    "0x".to_string() + &s
}
