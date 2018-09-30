// help function
#[allow(dead_code)]
pub fn slice_to_u8_8(s: &[u8]) -> [u8; 8] {
    let len = if s.len() < 8 { s.len() } else { 8 };
    let mut arr: [u8; 8] = Default::default();
    arr[..len].clone_from_slice(&s[..len]);
    arr
}

#[allow(dead_code)]
pub fn slice_to_u8_32(s: &[u8]) -> [u8; 32] {
    let len = if s.len() < 32 { s.len() } else { 32 };
    let mut arr: [u8; 32] = Default::default();
    arr[..len].clone_from_slice(&s[..len]);
    arr
}