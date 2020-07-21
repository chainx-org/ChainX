use rustc_hex::ToHex;
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
    let to_string = s.iter().try_for_each(|i| {
        if (b'0' <= *i && *i <= b'9') || (b'A' <= *i && *i <= b'z') {
            Ok(())
        } else {
            // 0x30 = '0' 0x7A = 'z'
            Err(())
        }
    });

    if to_string.is_ok() {
        u8array_to_string(s)
    } else {
        u8array_to_hex(s)
    }
}

#[inline]
pub fn u8array_to_hex(s: &[u8]) -> String {
    format!("0x{}", s.to_hex::<String>())
}

#[inline]
pub fn try_hex_or_str(src: &[u8]) -> String {
    let to_string = src.iter().try_for_each(|c| {
        if b'!' <= *c && *c <= b'~' {
            Ok(())
        } else {
            Err(())
        }
    });
    if to_string.is_ok() {
        u8array_to_string(src)
    } else {
        u8array_to_hex(src)
    }
}
