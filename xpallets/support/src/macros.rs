// Copyright 2019-2020 ChainX Project Authors. Licensed under GPL-3.0.

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
