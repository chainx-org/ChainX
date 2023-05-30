// Copyright 2019-2023 ChainX Project Authors. Licensed under GPL-3.0.

use super::*;

pub fn mint_into_encode(account: H160, amount: u128) -> Vec<u8> {
    // signature ++ account ++ amount
    let length = 16 + 20 + 32;
    let mut v = Vec::with_capacity(length);

    // bytes4(keccak256(bytes("mint_into(address,uint256)"))
    // 0xefe51695
    let sig_mint = [239u8, 229, 22, 149];

    // first 16-bytes
    v.extend_from_slice(&sig_mint[..]);
    v.extend_from_slice(&[0u8; 12][..]);

    // second 20-bytes
    v.extend_from_slice(&account[..]);

    // third 32-bytes
    v.extend_from_slice(&[0u8; 16][..]);
    v.extend_from_slice(&amount.to_be_bytes()[..]);

    v
}

pub fn burn_from_encode(account: H160, amount: u128) -> Vec<u8> {
    // signature ++ account ++ amount
    let length = 16 + 20 + 32;
    let mut v = Vec::with_capacity(length);

    // bytes4(keccak256(bytes("burn_from(address,uint256)"))
    // 0x0f536f84
    let sig_burn = [15u8, 83, 111, 132];

    // first 16-bytes
    v.extend_from_slice(&sig_burn[..]);
    v.extend_from_slice(&[0u8; 12][..]);

    // second 20-bytes
    v.extend_from_slice(&account[..]);

    // third 32-bytes
    v.extend_from_slice(&[0u8; 16][..]);
    v.extend_from_slice(&amount.to_be_bytes()[..]);

    v
}
