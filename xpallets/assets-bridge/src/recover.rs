// Copyright 2019-2023 ChainX Project Authors. Licensed under GPL-3.0.

use super::*;

/// Converts the given binary data into ASCII-encoded hex. It will be twice
/// the length.
pub fn to_ascii_hex(data: &[u8]) -> Vec<u8> {
    let mut r = Vec::with_capacity(data.len() * 2);
    let mut push_nibble = |n| r.push(if n < 10 { b'0' + n } else { b'a' - 10 + n });
    for &b in data.iter() {
        push_nibble(b / 16);
        push_nibble(b % 16);
    }
    r
}

/// Attempts to recover the Ethereum address from a message signature signed by
/// using the Ethereum RPC's `personal_sign` and `eth_sign`.
pub fn eth_recover(s: &EcdsaSignature, what: &[u8], extra: &[u8]) -> Option<H160> {
    let msg = keccak_256(&ethereum_signable_message(what, extra));
    let mut res = H160::default();
    res.0
        .copy_from_slice(&keccak_256(&secp256k1_ecdsa_recover(&s.0, &msg).ok()?[..])[12..]);
    Some(res)
}

/// Constructs the message that Ethereum RPC's `personal_sign` and `eth_sign`
/// would sign.
pub fn ethereum_signable_message(what: &[u8], extra: &[u8]) -> Vec<u8> {
    let prefix = b"evm:";
    let mut l = prefix.len() + what.len() + extra.len();
    let mut rev = Vec::new();
    while l > 0 {
        rev.push(b'0' + (l % 10) as u8);
        l /= 10;
    }
    let mut v = b"\x19Ethereum Signed Message:\n".to_vec();
    v.extend(rev.into_iter().rev());
    v.extend_from_slice(&prefix[..]);
    v.extend_from_slice(what);
    v.extend_from_slice(extra);
    v
}
