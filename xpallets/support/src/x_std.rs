use rustc_hex::ToHex;

/// Converts a slice of bytes to a string.
#[inline]
pub fn as_string(s: &[u8]) -> String {
    String::from_utf8_lossy(s).into_owned()
}

/// Converts a slice of bytes to a hex value, and then converts to a string with 0x prefix added.
#[inline]
pub fn as_string_hex(s: &[u8]) -> String {
    format!("0x{}", s.to_hex::<String>())
}

#[inline]
pub fn as_addr(s: &[u8]) -> String {
    let should_as_string = s.iter().try_for_each(|i| {
        if (b'0' <= *i && *i <= b'9') || (b'A' <= *i && *i <= b'z') {
            Ok(())
        } else {
            // 0x30 = '0' 0x7A = 'z'
            Err(())
        }
    });

    if should_as_string.is_ok() {
        as_string(s)
    } else {
        as_string_hex(s)
    }
}

#[inline]
pub fn try_hex_or_str(src: &[u8]) -> String {
    let should_as_string = src.iter().try_for_each(|c| {
        if b'!' <= *c && *c <= b'~' {
            Ok(())
        } else {
            Err(())
        }
    });
    if should_as_string.is_ok() {
        as_string(src)
    } else {
        as_string_hex(src)
    }
}
