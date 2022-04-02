// Copyright 2019-2022 ChainX Project Authors. Licensed under GPL-3.0.

#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(not(feature = "std"))]
extern crate alloc;
#[cfg(not(feature = "std"))]
use alloc::{format, string::String};

pub mod traits;

/// Try to convert a slice of bytes to a string.
#[inline]
pub fn try_str<S: AsRef<[u8]>>(src: S) -> String {
    if src
        .as_ref()
        .iter()
        .try_for_each(|byte| {
            if byte.is_ascii_graphic() {
                Ok(())
            } else {
                Err(())
            }
        })
        .is_ok()
    {
        str(src.as_ref())
    } else {
        hex(src.as_ref())
    }
}

/// Try to convert a slice of bytes to a address string.
#[inline]
pub fn try_addr<S: AsRef<[u8]>>(src: S) -> String {
    if src
        .as_ref()
        .iter()
        .try_for_each(|byte| {
            if byte.is_ascii_alphanumeric() {
                Ok(())
            } else {
                Err(())
            }
        })
        .is_ok()
    {
        str(src.as_ref())
    } else {
        hex(src.as_ref())
    }
}

/// Converts a slice of bytes to a string.
#[inline]
fn str(s: &[u8]) -> String {
    String::from_utf8_lossy(s).into_owned()
}

/// Converts a slice of bytes to a hex value, and then converts to a string with 0x prefix added.
#[inline]
fn hex(s: &[u8]) -> String {
    format!("0x{}", hex::encode(s))
}
