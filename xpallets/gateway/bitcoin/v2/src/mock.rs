use sp_core::H256;
use sp_keyring::sr25519;
use sp_runtime::{
    testing::Header,
    traits::{BlakeTwo256, IdentityLookup},
    AccountId32,
};
use xp_assets_registrar::Chain;
use xpallet_assets_registrar::AssetInfo;

use frame_support::{construct_runtime, parameter_types, sp_io, traits::GenesisBuild};

use crate::pallet;

/// The AccountId alias in this test module.
pub(crate) type AccountId = AccountId32;
pub(crate) type BlockNumber = u64;
pub(crate) type Amount = i128;
pub(crate) type Balance = u128;

pub(crate) fn alice() -> AccountId32 {
    sr25519::Keyring::Alice.to_account_id()
}
pub(crate) fn bob() -> AccountId32 {
    sr25519::Keyring::Bob.to_account_id()
}
pub(crate) fn charlie() -> AccountId32 {
    sr25519::Keyring::Charlie.to_account_id()
}

pub(crate) fn dave() -> AccountId32 {
    sr25519::Keyring::Dave.to_account_id()
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
    type BlockHashCount = ();
    type DbWeight = ();
    type Version = ();
    type PalletInfo = PalletInfo;
    type AccountData = pallet_balances::AccountData<Balance>;
    type OnNewAccount = ();
    type OnKilledAccount = ();
    type SystemWeightInfo = ();
    type SS58Prefix = ();
    type OnSetCode = ();
}
parameter_types! {
    pub const ExistentialDeposit: u64 = 0;
    pub const ChainXAssetId: u32 = 0;
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
    type OnCreatedAccount = ();
    type OnAssetChanged = ();
    type WeightInfo = ();
}

parameter_types! {
    pub const BridgeTargetAssetId: u32 = xp_protocol::X_BTC;
    pub const DustCollateral: Balance = 1000;
    pub const SecureThreshold: u16 = 300;
    pub const PremiumThreshold: u16 = 250;
    pub const LiquidationThreshold: u16 = 180;
    pub const IssueRequestExpiredPeriod: BlockNumber = 10000;
    pub const RedeemRequestExpiredPeriod: BlockNumber = 10000;
    pub const ExchangeRateExpiredPeriod: BlockNumber = 10;
    pub const MinimumRedeemValue: Balance = 1;
}

impl pallet::Config for Test {
    type Event = ();
    type TargetAssetId = BridgeTargetAssetId;
    type DustCollateral = DustCollateral;
    type SecureThreshold = SecureThreshold;
    type PremiumThreshold = PremiumThreshold;
    type LiquidationThreshold = LiquidationThreshold;
    type IssueRequestExpiredPeriod = IssueRequestExpiredPeriod;
    type RedeemRequestExpiredPeriod = RedeemRequestExpiredPeriod;
    type MinimumRedeemValue = MinimumRedeemValue;
    type ExchangeRateExpiredPeriod = ExchangeRateExpiredPeriod;
}

type Block = frame_system::mocking::MockBlock<Test>;
type UncheckedExtrinsic = frame_system::mocking::MockUncheckedExtrinsic<Test>;

construct_runtime! {
    pub enum Test where
        Block = Block,
        NodeBlock = Block,
        UncheckedExtrinsic = UncheckedExtrinsic,
        {
            System: frame_system::{Pallet, Call, Event<T>},
            Balances: pallet_balances::{Pallet, Call, Event<T>},
            XAssets: xpallet_assets::{Pallet,Call, Event<T>, Config<T>},
            XGatewayBitcoin: pallet::{Pallet, Call, Event<T>, Config<T>},
        }
}

pub struct BuildConfig {
    pub(crate) exchange_price: u128,
    pub(crate) exchange_decimal: u8,
}

impl Default for BuildConfig {
    fn default() -> Self {
        Self {
            exchange_price: 1,
            exchange_decimal: 3,
        }
    }
}

pub struct ExtBuilder;
impl ExtBuilder {
    pub fn build(
        BuildConfig {
            exchange_price,
            exchange_decimal,
        }: BuildConfig,
    ) -> sp_io::TestExternalities {
        use super::types::TradingPrice;

        let mut storage = frame_system::GenesisConfig::default()
            .build_storage::<Test>()
            .unwrap();

        let _ = GenesisBuild::<Test>::assimilate_storage(
            &pallet::GenesisConfig {
                exchange_rate: TradingPrice {
                    price: exchange_price,
                    decimal: exchange_decimal,
                },
                oracle_accounts: vec![alice()],
                liquidator_id: alice(),
                issue_griefing_fee: 10,
                ..Default::default()
            },
            &mut storage,
        );

        let _ = xpallet_assets_registrar::GenesisConfig {
            assets: vec![(
                xp_protocol::X_BTC,
                AssetInfo::new::<Test>(
                    b"X-BTC".to_vec(),
                    b"X-BTC".to_vec(),
                    Chain::Bitcoin,
                    8,
                    b"ChainX's cross-chain Bitcoin".to_vec(),
                )
                .unwrap(),
                true,
                true,
            )],
        }
        .assimilate_storage::<Test>(&mut storage);

        let _ = pallet_balances::GenesisConfig::<Test> {
            balances: vec![
                (alice(), 100_000),
                (bob(), 10000),
                (charlie(), 20000),
                (dave(), 30000),
            ],
        }
        .assimilate_storage(&mut storage);

        sp_io::TestExternalities::from(storage)
    }
}
