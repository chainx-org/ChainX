// Copyright 2018-2019 Chainpool.

//! Shareable ChainX types.
#![cfg_attr(not(feature = "std"), no_std)]

use runtime_primitives::{
    generic,
    traits::{BlakeTwo256, Verify},
    OpaqueExtrinsic,
};

/// Alias to 512-bit hash when used in the context of a session signature on the chain.
pub type AuthoritySignature = primitives::ed25519::Signature;
/// The Ed25519 pub key of an session that belongs to an authority of the chain. This is
/// exactly equivalent to what the substrate calls an "authority".
pub type AuthorityId = <AuthoritySignature as Verify>::Signer;

pub type Signature = primitives::ed25519::Signature;
/// Alias to Ed25519 pub key that identifies an account on the relay chain.
pub type AccountId = <Signature as Verify>::Signer;

/// The account id impl for runtime api, is same as AccountId.
pub type AccountIdForApi = primitives::ed25519::Public;
/// The account id impl for rpc.
pub type AccountIdForRpc = primitives::H256;

/// A hash of some data used by the relay chain.
pub type Hash = primitives::H256;

/// Header type.
pub type Header = generic::Header<
    BlockNumber,
    BlakeTwo256,
    generic::DigestItem<Hash, AuthorityId, AuthoritySignature>,
>;

/// A "future-proof" block type for Polkadot. This will be resilient to upgrades in transaction
/// format, because it doesn't attempt to decode extrinsics.
///
/// Specialized code needs to link to (at least one version of) the runtime directly
/// in order to handle the extrinsics within.
pub type Block = generic::Block<Header, UncheckedExtrinsic>;

/// An index to a block.
/// 32-bits will allow for 136 years of blocks assuming 1 block per second.
/// TODO: switch to u32
pub type BlockNumber = u64;

/// The type for looking up accounts. We don't expect more than 4 billion of them, but you
/// never know...
pub type AccountIndex = u32;

/// Indentifier for a chain. 32-bit should be plenty.
pub type ChainId = u32;

/// Index of a transaction in the relay chain. 32-bit should be plenty.
pub type Index = u64;

/// Bigger Acceleration means more chances be to included in a block for a transaction.
pub type Acceleration = u32;

/// A timestamp: seconds since the unix epoch.
pub type Timestamp = u64;

/// The balance of an account.
/// u64 for chainx token and all assets type, if the asset is not suit for u64, choose a suitable precision
pub type Balance = u64;

/// "generic" block ID for the future-proof block type.
pub type BlockId = generic::BlockId<Block>;

/// Opaque, encoded, unchecked extrinsic.
pub type UncheckedExtrinsic = OpaqueExtrinsic;
