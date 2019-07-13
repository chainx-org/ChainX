use rustc_hex::ToHex;
pub fn try_hex_or_str(src: &[u8]) -> String {
    let check_is_str = |src: &[u8]| -> bool {
        for c in src {
            if 0x21 <= *c && *c <= 0x7E {
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
