use rustc_hex::ToHex;
pub fn try_hex_or_str(src: &[u8]) -> String {
    let check_is_str = |src: &[u8]| -> bool {
        for c in src {
            if (b'0' <= *c && *c <= b'9')
                || (b'a' <= *c && *c <= b'z')
                || (b'A' <= *c && *c <= b'Z')
            {
                continue;
            } else {
                return false;
            }
        }
        return true;
    };
    if check_is_str(src) {
        String::from_utf8_lossy(src).into_owned()
    } else {
        format!("0x{:}", src.to_hex::<String>())
    }
}
