// Copyright 2019-2020 ChainX Project Authors. Licensed under GPL-3.0.

use std::{
    cell::RefCell,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use codec::{Decode, Encode};
use frame_support::{impl_outer_origin, parameter_types, traits::UnixTime};
use frame_system::EnsureSignedBy;
use sp_core::{sr25519::Signature, H256};
use sp_runtime::{
    testing::{Header, TestXt},
    traits::{BlakeTwo256, Extrinsic as ExtrinsicT, IdentifyAccount, IdentityLookup, Verify},
};

use chainx_primitives::AssetId;

use crate::{app::RelayAuthId, AuthorityId, Call, Module, Trait};

impl_outer_origin! {
    pub enum Origin for Test where system = frame_system {}
}

// For testing the module, we construct most of a mock runtime. This means
// first constructing a configuration type (`Test`) which `impl`s each of the
// configuration traits of modules we want to use.
#[derive(Clone, Eq, PartialEq, Encode, Decode)]
pub struct Test;
parameter_types! {
    pub const BlockHashCount: u64 = 250;
}
impl frame_system::Trait for Test {
    type BaseCallFilter = ();
    type Origin = Origin;
    type Call = ();
    type Index = u64;
    type BlockNumber = u64;
    type Hash = H256;
    type Hashing = BlakeTwo256;
    type AccountId = sp_core::sr25519::Public;
    type Lookup = IdentityLookup<Self::AccountId>;
    type Header = Header;
    type Event = ();
    type BlockHashCount = BlockHashCount;
    type MaximumBlockWeight = ();
    type DbWeight = ();
    type BlockExecutionWeight = ();
    type ExtrinsicBaseWeight = ();
    type MaximumExtrinsicWeight = ();
    type MaximumBlockLength = ();
    type AvailableBlockRatio = ();
    type Version = ();
    type PalletInfo = ();
    type AccountData = pallet_balances::AccountData<u128>;
    type OnNewAccount = ();
    type OnKilledAccount = ();
    type SystemWeightInfo = ();
}

type Extrinsic = TestXt<Call<Test>, ()>;
type AccountId = <<Signature as Verify>::Signer as IdentifyAccount>::AccountId;

impl frame_system::offchain::SigningTypes for Test {
    type Public = <Signature as Verify>::Signer;
    type Signature = Signature;
}

impl<LocalCall> frame_system::offchain::SendTransactionTypes<LocalCall> for Test
where
    Call<Test>: From<LocalCall>,
{
    type Extrinsic = Extrinsic;
    type OverarchingCall = Call<Test>;
}

impl<LocalCall> frame_system::offchain::CreateSignedTransaction<LocalCall> for Test
where
    Call<Test>: From<LocalCall>,
{
    fn create_transaction<C: frame_system::offchain::AppCrypto<Self::Public, Self::Signature>>(
        call: Call<Test>,
        _public: <Signature as Verify>::Signer,
        _account: AccountId,
        nonce: u64,
    ) -> Option<(Call<Test>, <Extrinsic as ExtrinsicT>::SignaturePayload)> {
        Some((call, (nonce, ())))
    }
}

parameter_types! {
    pub const ExistentialDeposit: u64 = 1;
}
impl pallet_balances::Trait for Test {
    type MaxLocks = ();
    type Balance = u128;
    type DustRemoval = ();
    type Event = ();
    type ExistentialDeposit = ExistentialDeposit;
    type AccountStore = System;
    type WeightInfo = ();
}

parameter_types! {
    pub const ChainXAssetId: AssetId = 0;
}
impl xpallet_assets_registrar::Trait for Test {
    type Event = ();
    type NativeAssetId = ChainXAssetId;
    type RegistrarHandler = ();
    type WeightInfo = ();
}

impl xpallet_assets::Trait for Test {
    type Event = ();
    type Currency = Balances;
    type Amount = i128;
    type TreasuryAccount = ();
    type OnCreatedAccount = ();
    type OnAssetChanged = ();
    type WeightInfo = ();
}

thread_local! {
    pub static NOW: RefCell<Option<Duration>> = RefCell::new(None);
}
pub struct Timestamp;
impl UnixTime for Timestamp {
    fn now() -> Duration {
        NOW.with(|m| {
            m.borrow().unwrap_or_else(|| {
                let start = SystemTime::now();
                let since_the_epoch = start
                    .duration_since(UNIX_EPOCH)
                    .expect("Time went backwards");
                since_the_epoch
            })
        })
    }
}

impl xpallet_gateway_records::Trait for Test {
    type Event = ();
    type WeightInfo = ();
}

impl xpallet_gateway_common::Trait for Test {
    type Event = ();
    type Validator = ();
    type DetermineMultisigAddress = ();
    type Bitcoin = XGatewayBitcoin;
    type BitcoinTrustee = XGatewayBitcoin;
    type WeightInfo = ();
}

impl xpallet_gateway_bitcoin::Trait for Test {
    type Event = ();
    type UnixTime = Timestamp;
    type AccountExtractor = ();
    type TrusteeSessionProvider =
        xpallet_gateway_common::trustees::bitcoin::BtcTrusteeSessionManager<Test>;
    type TrusteeOrigin = EnsureSignedBy<
        xpallet_gateway_common::trustees::bitcoin::BtcTrusteeMultisig<Test>,
        AccountId,
    >;
    type ReferralBinding = XGatewayCommon;
    type AddressBinding = XGatewayCommon;
    type WeightInfo = ();
}

impl Trait for Test {
    type Event = ();
    type Call = Call<Test>;
    type AuthorityId = AuthorityId;
    type WeightInfo = ();
    type UnsignedPriority = ();
}

type System = frame_system::Module<Test>;
type Balances = pallet_balances::Module<Test>;
type XGatewayCommon = xpallet_gateway_common::Module<Test>;
type XGatewayBitcoin = xpallet_gateway_bitcoin::Module<Test>;
pub type XGatewayBitcoinRelay = Module<Test>;
