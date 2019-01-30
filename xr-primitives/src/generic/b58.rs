// Copyright 2018 Chainpool

use sr_std::prelude::Vec;

static BASE58_CHARS: &'static [u8] = b"123456789ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz";

#[rustfmt::skip]
static BASE58_DIGITS: [Option<u8>; 128] = [
    None,     None,     None,     None,     None,     None,     None,     None,     // 0-7
    None,     None,     None,     None,     None,     None,     None,     None,     // 8-15
    None,     None,     None,     None,     None,     None,     None,     None,     // 16-23
    None,     None,     None,     None,     None,     None,     None,     None,     // 24-31
    None,     None,     None,     None,     None,     None,     None,     None,     // 32-39
    None,     None,     None,     None,     None,     None,     None,     None,     // 40-47
    None,     Some(0),  Some(1),  Some(2),  Some(3),  Some(4),  Some(5),  Some(6),  // 48-55
    Some(7),  Some(8),  None,     None,     None,     None,     None,     None,     // 56-63
    None,     Some(9),  Some(10), Some(11), Some(12), Some(13), Some(14), Some(15), // 64-71
    Some(16), None,     Some(17), Some(18), Some(19), Some(20), Some(21), None,     // 72-79
    Some(22), Some(23), Some(24), Some(25), Some(26), Some(27), Some(28), Some(29), // 80-87
    Some(30), Some(31), Some(32), None,     None,     None,     None,     None,     // 88-95
    None,     Some(33), Some(34), Some(35), Some(36), Some(37), Some(38), Some(39), // 96-103
    Some(40), Some(41), Some(42), Some(43), None,     Some(44), Some(45), Some(46), // 104-111
    Some(47), Some(48), Some(49), Some(50), Some(51), Some(52), Some(53), Some(54), // 112-119
    Some(55), Some(56), Some(57), None,     None,     None,     None,     None,     // 120-127
];

pub fn from(data: Vec<u8>) -> Result<Vec<u8>, &'static str> {
    // 11/15 is just over log_256(58)
    let mut scratch = Vec::new();
    for _i in 0..1 + data.len() * 11 / 15 {
        scratch.push(0);
    }
    // Build in base 256
    for d58 in data.clone() {
        // Compute "X = X * 58 + next_digit" in base 256
        if d58 as usize > BASE58_DIGITS.len() {
            return Err("BadByte");
        }
        let mut carry = match BASE58_DIGITS[d58 as usize] {
            Some(d58) => d58 as u32,
            None => {
                return Err("BadByte");
            }
        };
        for d256 in scratch.iter_mut().rev() {
            carry += *d256 as u32 * 58;
            *d256 = carry as u8;
            carry /= 256;
        }
        assert_eq!(carry, 0);
    }

    // Copy leading zeroes directly
    let mut ret: Vec<u8> = data
        .iter()
        .take_while(|&x| *x == BASE58_CHARS[0])
        .map(|_| 0)
        .collect();
    // Copy rest of string
    ret.extend(scratch.into_iter().skip_while(|&x| x == 0));
    Ok(ret)
}

pub fn to_base58(data: Vec<u8>) -> Vec<u8> {
    let zcount = data.iter().take_while(|x| **x == 0).count();
    let size: usize = (data.len() - zcount) * 138 / 100 + 1;

    let mut buffer: Vec<u8> = Vec::new();
    for _i in 0..size {
        buffer.push(0);
    }
    let mut i = zcount;
    let mut high = size - 1;
    while i < data.len() {
        let mut carry = data[i] as u32;
        let mut j = size - 1;

        while j > high || carry != 0 {
            carry += 256 * buffer[j] as u32;
            buffer[j] = (carry % 58) as u8;
            carry /= 58;

            if j > 0 {
                j -= 1;
            }
        }

        i += 1;
        high = j;
    }
    let mut j = buffer.iter().take_while(|x| **x == 0).count();
    let mut result = Vec::new();
    for _ in 0..zcount {
        result.push('1' as u8);
    }
    while j < size {
        result.push(BASE58_CHARS[buffer[j] as usize] as u8);
        j += 1;
    }
    result
}

#[cfg(test)]
mod tests {
    use super::{from, to_base58};
    #[test]
    fn test_from() {
        let s = String::from("mjKE11gjVN4JaC9U8qL6ZB5vuEBgmwik7b");
        let v = &[
            111, 41, 168, 159, 89, 51, 97, 179, 153, 104, 9, 74, 184, 193, 251, 6, 131, 166, 121,
            3, 1, 241, 112, 101, 146,
        ];
        assert_eq!(from(s.as_bytes().to_vec()).unwrap(), v);
    }
    #[test]
    fn test_to_base58() {
        let s = String::from("mjKE11gjVN4JaC9U8qL6ZB5vuEBgmwik7b");
        let v = &[
            111, 41, 168, 159, 89, 51, 97, 179, 153, 104, 9, 74, 184, 193, 251, 6, 131, 166, 121,
            3, 1, 241, 112, 101, 146,
        ];
        assert_eq!(to_base58(v.to_vec()), s.as_bytes());
    }
}
