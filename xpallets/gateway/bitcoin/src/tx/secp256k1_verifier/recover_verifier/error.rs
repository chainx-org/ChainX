// Copyright 2019-2020 ChainX Project Authors. Licensed under GPL-3.0.

#[derive(Clone, Copy, Eq, PartialEq)]
pub enum Secp256k1Error {
    InvalidSignature,
    // InvalidPublicKey,
    // InvalidSecretKey,
    // InvalidRecoveryId,
    // InvalidMessage,
    // InvalidInputLength,
    // TweakOutOfRange,
}
