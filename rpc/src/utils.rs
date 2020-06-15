/// Convert &[u8] to String
macro_rules! to_string {
    ($str:expr) => {
        String::from_utf8_lossy($str).into_owned()
    };
}
