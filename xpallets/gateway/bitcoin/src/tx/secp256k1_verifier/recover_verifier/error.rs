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
