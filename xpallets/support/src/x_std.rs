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
