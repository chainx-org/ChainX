// Copyright 2019-2020 ChainX Project Authors. Licensed under GPL-3.0.

use std::{cell::RefCell, convert::TryFrom, time::Duration};

use codec::{Decode, Encode};
use frame_support::{
    parameter_types, sp_io,
    traits::{GenesisBuild, UnixTime},
};
use frame_system::EnsureSignedBy;
use sp_core::{crypto::UncheckedInto, H256};
use sp_io::hashing::blake2_256;
use sp_runtime::{
    testing::Header,
    traits::{BlakeTwo256, IdentityLookup},
    AccountId32, DispatchError, DispatchResult,
};

use chainx_primitives::AssetId;
pub use xp_protocol::{X_BTC, X_ETH};
use xpallet_assets::{AssetRestrictions, BalanceOf, ChainT, WithdrawalLimit};
use xpallet_assets_registrar::{AssetInfo, Chain};
use xpallet_support::traits::{MultisigAddressFor, Validator};

use crate::{
    self as xpallet_gateway_common,
    traits::TrusteeForChain,
    trustees::bitcoin::{BtcTrusteeAddrInfo, BtcTrusteeMultisig, BtcTrusteeType},
    types::*,
};

pub(crate) type AccountId = AccountId32;
pub(crate) type BlockNumber = u64;
pub(crate) type Balance = u128;
pub(crate) type Amount = i128;

type UncheckedExtrinsic = frame_system::mocking::MockUncheckedExtrinsic<Test>;
type Block = frame_system::mocking::MockBlock<Test>;

frame_support::construct_runtime!(
    pub enum Test where
        Block = Block,
        NodeBlock = Block,
        UncheckedExtrinsic = UncheckedExtrinsic,
    {
        System: frame_system::{Pallet, Call, Config, Storage, Event<T>},
        Balances: pallet_balances::{Pallet, Call, Storage, Config<T>, Event<T>},
        XAssetsRegistrar: xpallet_assets_registrar::{Pallet, Call, Storage, Event<T>, Config},
        XAssets: xpallet_assets::{Pallet, Call, Storage, Event<T>, Config<T>},
        XGatewayRecords: xpallet_gateway_records::{Pallet, Call, Storage, Event<T>},
        XGatewayCommon: xpallet_gateway_common::{Pallet, Call, Storage, Event<T>, Config<T>},
        XGatewayBitcoin: xpallet_gateway_bitcoin::{Pallet, Call, Storage, Event<T>, Config<T>},
    }
);

parameter_types! {
    pub const BlockHashCount: u64 = 250;
    pub const SS58Prefix: u8 = 42;
}
impl frame_system::Config for Test {
    type BaseCallFilter = ();
    type BlockWeights = ();
    type BlockLength = ();
    type Origin = Origin;
    type Call = Call;
    type Index = u64;
    type BlockNumber = BlockNumber;
    type Hash = H256;
    type Hashing = BlakeTwo256;
    type AccountId = AccountId;
    type Lookup = IdentityLookup<Self::AccountId>;
    type Header = Header;
    type Event = ();
    type BlockHashCount = BlockHashCount;
    type DbWeight = ();
    type Version = ();
    type PalletInfo = PalletInfo;
    type AccountData = pallet_balances::AccountData<Balance>;
    type OnNewAccount = ();
    type OnKilledAccount = ();
    type SystemWeightInfo = ();
    type SS58Prefix = SS58Prefix;
    type OnSetCode = ();
}

parameter_types! {
    pub const ExistentialDeposit: u64 = 0;
}
impl pallet_balances::Config for Test {
    type MaxLocks = ();
    type Balance = Balance;
    type DustRemoval = ();
    type Event = ();
    type ExistentialDeposit = ExistentialDeposit;
    type AccountStore = System;
    type WeightInfo = ();
}

parameter_types! {
    pub const ChainXAssetId: AssetId = 0;
}
impl xpallet_assets_registrar::Config for Test {
    type Event = ();
    type NativeAssetId = ChainXAssetId;
    type RegistrarHandler = ();
    type WeightInfo = ();
}
impl xpallet_assets::Config for Test {
    type Event = ();
    type Currency = Balances;
    type Amount = Amount;
    type TreasuryAccount = ();
    type OnCreatedAccount = frame_system::Provider<Test>;
    type OnAssetChanged = ();
    type WeightInfo = ();
}

impl xpallet_gateway_records::Config for Test {
    type Event = ();
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
                use std::time::{SystemTime, UNIX_EPOCH};
                let start = SystemTime::now();
                let since_the_epoch = start
                    .duration_since(UNIX_EPOCH)
                    .expect("Time went backwards");
                since_the_epoch
            })
        })
    }
}
impl xpallet_gateway_bitcoin::Config for Test {
    type Event = ();
    type UnixTime = Timestamp;
    type AccountExtractor = ();
    type TrusteeSessionProvider = ();
    type TrusteeOrigin = EnsureSignedBy<BtcTrusteeMultisig<Test>, AccountId>;
    type ReferralBinding = ();
    type AddressBinding = ();
    type WeightInfo = ();
}

pub struct MultisigAddr;
impl MultisigAddressFor<AccountId> for MultisigAddr {
    fn calc_multisig(who: &[AccountId], threshold: u16) -> AccountId {
        let entropy = (b"modlpy/utilisuba", who, threshold).using_encoded(blake2_256);
        AccountId::decode(&mut &entropy[..]).unwrap_or_default()
    }
}
pub struct AlwaysValidator;
impl Validator<AccountId> for AlwaysValidator {
    fn is_validator(_who: &AccountId) -> bool {
        true
    }

    fn validator_for(_: &[u8]) -> Option<AccountId> {
        None
    }
}
pub struct MockBitcoin<T: xpallet_gateway_bitcoin::Config>(sp_std::marker::PhantomData<T>);
impl<T: xpallet_gateway_bitcoin::Config> ChainT<BalanceOf<T>> for MockBitcoin<T> {
    const ASSET_ID: u32 = X_BTC;

    fn chain() -> Chain {
        Chain::Bitcoin
    }

    fn check_addr(_: &[u8], _: &[u8]) -> DispatchResult {
        Ok(())
    }

    fn withdrawal_limit(asset_id: &u32) -> Result<WithdrawalLimit<BalanceOf<T>>, DispatchError> {
        xpallet_gateway_bitcoin::Module::<T>::withdrawal_limit(asset_id)
    }
}
impl<T: xpallet_gateway_bitcoin::Config>
    TrusteeForChain<T::AccountId, BtcTrusteeType, BtcTrusteeAddrInfo> for MockBitcoin<T>
{
    fn check_trustee_entity(raw_addr: &[u8]) -> Result<BtcTrusteeType, DispatchError> {
        let trustee_type =
            BtcTrusteeType::try_from(raw_addr.to_vec()).map_err(|_| "InvalidPublicKey")?;
        Ok(trustee_type)
    }

    fn generate_trustee_session_info(
        props: Vec<(T::AccountId, TrusteeIntentionProps<BtcTrusteeType>)>,
        _: TrusteeInfoConfig,
    ) -> Result<TrusteeSessionInfo<T::AccountId, BtcTrusteeAddrInfo>, DispatchError> {
        let len = props.len();
        Ok(TrusteeSessionInfo {
            trustee_list: props.into_iter().map(|(a, _)| a).collect::<_>(),
            threshold: len as u16,
            hot_address: BtcTrusteeAddrInfo {
                addr: vec![],
                redeem_script: vec![],
            },
            cold_address: BtcTrusteeAddrInfo {
                addr: vec![],
                redeem_script: vec![],
            },
        })
    }
}
impl crate::Config for Test {
    type Event = ();
    type Validator = AlwaysValidator;
    type DetermineMultisigAddress = MultisigAddr;
    type Bitcoin = MockBitcoin<Test>;
    type BitcoinTrustee = MockBitcoin<Test>;
    type WeightInfo = ();
}

pub struct ExtBuilder;
impl Default for ExtBuilder {
    fn default() -> Self {
        Self
    }
}
impl ExtBuilder {
    pub fn build(self) -> sp_io::TestExternalities {
        let mut storage = frame_system::GenesisConfig::default()
            .build_storage::<Test>()
            .unwrap();

        let btc_assets = btc();
        let assets = vec![(btc_assets.0, btc_assets.1, btc_assets.2, true, true)];

        let mut init_assets = vec![];
        let mut assets_restrictions = vec![];
        for (a, b, c, d, e) in assets {
            init_assets.push((a, b, d, e));
            assets_restrictions.push((a, c))
        }

        GenesisBuild::<Test>::assimilate_storage(
            &xpallet_assets_registrar::GenesisConfig {
                assets: init_assets,
            },
            &mut storage,
        )
        .unwrap();

        let _ = xpallet_assets::GenesisConfig::<Test> {
            assets_restrictions,
            endowed: Default::default(),
        }
        .assimilate_storage(&mut storage);

        let _ = crate::GenesisConfig::<Test> {
            trustees: trustees(),
        }
        .assimilate_storage(&mut storage);

        let ext = sp_io::TestExternalities::new(storage);
        ext
    }
}

fn btc() -> (AssetId, AssetInfo, AssetRestrictions) {
    (
        X_BTC,
        AssetInfo::new::<Test>(
            b"X-BTC".to_vec(),
            b"X-BTC".to_vec(),
            Chain::Bitcoin,
            8,
            b"ChainX's cross-chain Bitcoin".to_vec(),
        )
        .unwrap(),
        AssetRestrictions::DESTROY_USABLE,
    )
}

fn trustees() -> Vec<(
    Chain,
    TrusteeInfoConfig,
    Vec<(AccountId, Vec<u8>, Vec<u8>, Vec<u8>)>,
)> {
    let btc_trustees = vec![
        (
            H256::repeat_byte(1).unchecked_into(),
            b"".to_vec(),
            hex::decode("02df92e88c4380778c9c48268460a124a8f4e7da883f80477deaa644ced486efc6")
                .expect("hex decode failed")
                .into(),
            hex::decode("0386b58f51da9b37e59c40262153173bdb59d7e4e45b73994b99eec4d964ee7e88")
                .expect("hex decode failed")
                .into(),
        ),
        (
            H256::repeat_byte(2).unchecked_into(),
            b"".to_vec(),
            hex::decode("0244d81efeb4171b1a8a433b87dd202117f94e44c909c49e42e77b69b5a6ce7d0d")
                .expect("hex decode failed")
                .into(),
            hex::decode("02e4631e46255571122d6e11cda75d5d601d5eb2585e65e4e87fe9f68c7838a278")
                .expect("hex decode failed")
                .into(),
        ),
        (
            H256::repeat_byte(3).unchecked_into(),
            b"".to_vec(),
            hex::decode("03a36339f413da869df12b1ab0def91749413a0dee87f0bfa85ba7196e6cdad102")
                .expect("hex decode failed")
                .into(),
            hex::decode("0263d46c760d3e04883d4b433c9ce2bc32130acd9faad0192a2b375dbba9f865c3")
                .expect("hex decode failed")
                .into(),
        ),
    ];
    let btc_config = TrusteeInfoConfig {
        min_trustee_count: 3,
        max_trustee_count: 15,
    };
    vec![(Chain::Bitcoin, btc_config, btc_trustees)]
}
