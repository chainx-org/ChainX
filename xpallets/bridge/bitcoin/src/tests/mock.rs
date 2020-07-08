// Copyright 2018-2019 Chainpool.

#![cfg(test)]

use crate::*;

// Substrate
use primitives::testing::{Digest, DigestItem, Header, UintAuthorityId};
use primitives::traits::{BlakeTwo256, IdentityLookup};
use primitives::BuildStorage;
use substrate_primitives::ed25519::Public;
use substrate_primitives::{Blake2Hasher, H256 as S_H256};
use support::impl_outer_origin;

// light-bitcoin
use btc_primitives::{h256_from_rev_str, Compact};
use xbridge_common::traits::IntoVecu8;

impl_outer_origin! {
    pub enum Origin for Test {}
}

type AccountId = Public;

#[derive(Clone, Eq, PartialEq)]
pub struct Test;

impl system::Trait for Test {
    type Origin = Origin;
    type Index = u64;
    type BlockNumber = u64;
    type Hash = S_H256;
    type Hashing = BlakeTwo256;
    type Digest = Digest;
    type AccountId = AccountId;
    type Lookup = IdentityLookup<AccountId>;
    type Header = Header;
    type Event = ();
    type Log = DigestItem;
}

impl consensus::Trait for Test {
    type Log = DigestItem;
    type SessionKey = UintAuthorityId;
    type InherentOfflineReport = ();
}

impl timestamp::Trait for Test {
    type Moment = u64;
    type OnTimestampSet = ();
}

impl xsystem::Trait for Test {
    type ValidatorList = MockValidatorList;
    type Validator = MockValidator;
}

pub struct MockValidatorList;
impl xsystem::ValidatorList<AccountId> for MockValidatorList {
    fn validator_list() -> Vec<AccountId> {
        vec![]
    }
}

pub struct MockValidator;
impl xsystem::Validator<AccountId> for MockValidator {
    fn get_validator_by_name(_name: &[u8]) -> Option<AccountId> {
        Some(AccountId::default())
    }
    fn get_validator_name(_: &AccountId) -> Option<Vec<u8>> {
        None
    }
}

impl xaccounts::Trait for Test {
    type DetermineIntentionJackpotAccountId = MockDeterminator;
}

pub struct MockDeterminator;
impl xaccounts::IntentionJackpotAccountIdFor<AccountId> for MockDeterminator {
    fn accountid_for_unsafe(_: &AccountId) -> AccountId {
        AccountId::default()
    }
    fn accountid_for_safe(_: &AccountId) -> Option<AccountId> {
        Some(AccountId::default())
    }
}

impl xrecords::Trait for Test {
    type Event = ();
}

impl xpallet_assets::Trait for Test {
    type Balance = u64;
    type OnNewAccount = ();
    type OnAssetChanged = ();
    type OnAssetRegisterOrRevoke = ();
    type Event = ();
}

impl xfee_manager::Trait for Test {
    type Event = ();
}

impl xbridge_common::Trait for Test {
    type Event = ();
}

impl lockup::Trait for Test {
    type Event = ();
}

impl Trait for Test {
    type XBitcoinLockup = Self;

    type AccountExtractor = xbridge_common::extractor::Extractor<AccountId>;
    type TrusteeSessionProvider = DummyTrusteeSession;
    type TrusteeMultiSigProvider = DummyBitcoinTrusteeMultiSig;
    type CrossChainProvider = DummyCrossChain;
    type Event = ();
}

pub struct DummyTrusteeSession;
impl xbridge_common::traits::TrusteeSession<AccountId, TrusteeAddrInfo> for DummyTrusteeSession {
    fn trustee_session(
        _: u32,
    ) -> result::Result<TrusteeSessionInfo<AccountId, TrusteeAddrInfo>, &'static str> {
        Ok(TrusteeSessionInfo {
            trustee_list: [
                AccountId::from_slice(&[0]),
                AccountId::from_slice(&[1]),
                AccountId::from_slice(&[2]),
            ]
            .to_vec(),
            hot_address: TrusteeAddrInfo::from_vecu8(&[0]).unwrap(),
            cold_address: TrusteeAddrInfo::from_vecu8(&[1]).unwrap(),
        })
    }

    fn current_trustee_session(
    ) -> std::result::Result<TrusteeSessionInfo<AccountId, TrusteeAddrInfo>, &'static str> {
        Ok(TrusteeSessionInfo {
            trustee_list: [
                AccountId::from_slice(&[0]),
                AccountId::from_slice(&[1]),
                AccountId::from_slice(&[2]),
            ]
            .to_vec(),
            hot_address: TrusteeAddrInfo::from_vecu8(&[0]).unwrap(),
            cold_address: TrusteeAddrInfo::from_vecu8(&[1]).unwrap(),
        })
    }

    fn last_trustee_session(
    ) -> std::result::Result<TrusteeSessionInfo<AccountId, TrusteeAddrInfo>, &'static str> {
        Ok(TrusteeSessionInfo {
            trustee_list: [
                AccountId::from_slice(&[0]),
                AccountId::from_slice(&[1]),
                AccountId::from_slice(&[2]),
            ]
            .to_vec(),
            hot_address: TrusteeAddrInfo::from_vecu8(&[0]).unwrap(),
            cold_address: TrusteeAddrInfo::from_vecu8(&[1]).unwrap(),
        })
    }
}

pub struct DummyCrossChain;
impl xbridge_common::traits::CrossChainBinding<AccountId, BTCAddress> for DummyCrossChain {
    fn update_binding(_: &AccountId, _: btc_keys::Address, _: Option<Vec<u8>>) {}

    fn get_binding_info(_: &btc_keys::Address) -> Option<(AccountId, Option<AccountId>)> {
        Some((
            AccountId::from_slice(&[0]),
            Some(AccountId::from_slice(&[1])),
        ))
    }
}

pub struct DummyBitcoinTrusteeMultiSig;
impl xbridge_common::traits::TrusteeMultiSig<AccountId> for DummyBitcoinTrusteeMultiSig {
    fn multisig_for_trustees() -> AccountId {
        AccountId::from_slice(&[9])
    }
}

pub type XAssets = xpallet_assets::Module<Test>;
pub type XBridgeOfBTC = Module<Test>;
pub type XBridgeOfBTCLockup = lockup::Module<Test>;

pub fn new_test_ext() -> runtime_io::TestExternalities<Blake2Hasher> {
    let mut r = system::GenesisConfig::<Test>::default()
        .build_storage()
        .unwrap()
        .0;

    // bridge btc
    r.extend(
        GenesisConfig::<Test> {
            // start genesis block: (genesis, blocknumber)
            genesis: (
                BTCHeader {
                    version: 980090880,
                    previous_header_hash: h256_from_rev_str(
                        "00000000000000ab706b663326210d03780fea6ecfe0cc59c78f0c7dddba9cc2",
                    ),
                    merkle_root_hash: h256_from_rev_str(
                        "91ee572484dabc6edf5a8da44a4fb55b5040facf66624b2a37c4f633070c60c8",
                    ),
                    time: 1550454022,
                    bits: Compact::new(436283074),
                    nonce: 47463732,
                },
                1457525,
            ),
            genesis_hash: h256_from_rev_str(
                "0000000000000059227e29b86313c99ac908a1d71db97632b402f13a569b4709",
            ),
            params_info: BTCParams::new(
                520159231,            // max_bits
                2 * 60 * 60,          // block_max_future
                2 * 7 * 24 * 60 * 60, // target_timespan_seconds
                10 * 60,              // target_spacing_seconds
                4,
            ), // retargeting_factor
            network_id: 0,
            confirmation_number: 3,
            reserved_block: 2100,
            btc_withdrawal_fee: 1000,
            max_withdrawal_count: 100,
            _genesis_phantom_data: Default::default(),
        }
        .build_storage()
        .unwrap()
        .0,
    );
    r.into()
}

pub fn new_test_mainnet() -> runtime_io::TestExternalities<Blake2Hasher> {
    let mut r = system::GenesisConfig::<Test>::default()
        .build_storage()
        .unwrap()
        .0;

    r.extend(
        xsystem::GenesisConfig::<Test> {
            network_props: (xsystem::NetworkType::Mainnet, 44),
            _genesis_phantom_data: Default::default(),
        }
        .build_storage()
        .unwrap()
        .0,
    );

    // bridge btc
    r.extend(
        GenesisConfig::<Test> {
            // start genesis block: (genesis, blocknumber)
            genesis: (
                BTCHeader {
                    version: 545259520,
                    previous_header_hash: h256_from_rev_str(
                        "00000000000000000001b2505c11119fcf29be733ec379f686518bf1090a522a",
                    ),
                    merkle_root_hash: h256_from_rev_str(
                        "cc09d95fd8ccc985826b9eb46bf73f8449116f18535423129f0574500985cf90",
                    ),
                    time: 1556958733,
                    bits: Compact::new(388628280),
                    nonce: 2897942742,
                },
                574560,
            ),
            genesis_hash: h256_from_rev_str(
                "00000000000000000008c8427670a65dec4360e88bf6c8381541ef26b30bd8fc",
            ),
            params_info: BTCParams::new(
                486604799,            // max_bits
                2 * 60 * 60,          // block_max_future
                2 * 7 * 24 * 60 * 60, // target_timespan_seconds
                10 * 60,              // target_spacing_seconds
                4,                    // retargeting_factor
            ), // retargeting_factor
            network_id: 0,
            confirmation_number: 4,
            reserved_block: 2100,
            btc_withdrawal_fee: 40000,
            max_withdrawal_count: 100,
            _genesis_phantom_data: Default::default(),
        }
        .build_storage()
        .unwrap()
        .0,
    );
    r.into()
}

pub fn generate_blocks() -> (Vec<BTCHeader>, Vec<BTCHeader>) {
    let b0: BTCHeader = BTCHeader {
        version: 536870912,
        previous_header_hash: Default::default(),
        merkle_root_hash: h256_from_rev_str(
            "815ca8bbed88af8afaa6c4995acba6e6e7453e705e0bc7039472aa3b6191a707",
        ),
        time: 1546999089,
        bits: Compact::new(436290411),
        nonce: 562223693,
    }; //1451572

    let b1: BTCHeader = BTCHeader {
        version: 536928256,
        previous_header_hash: h256_from_rev_str(
            "00000000000000fd9cea8b846895f507c63b005d20ac56e87d1cdf80effd5c0a",
        ),
        merkle_root_hash: h256_from_rev_str(
            "c16a4a6a6cc43c67770cbec9dd0cc4bf7e956d6b4c9e7c15ff1a2dc8ef3afc63",
        ),
        time: 1547000297,
        bits: Compact::new(486604799),
        nonce: 2982943095,
    };

    let b2: BTCHeader = BTCHeader {
        version: 536870912,
        previous_header_hash: h256_from_rev_str(
            "0000000000008bc1a5a3ee37368eeeb958f61464a1a5d18ed22e1430965ab3dd",
        ),
        merkle_root_hash: h256_from_rev_str(
            "14f332ae3422cfa8726f5e5fcf2d309b54ce005f3581f1f20f252772717044b5",
        ),
        time: 1547000572,
        bits: Compact::new(436290411),
        nonce: 744509129,
    };

    let b3: BTCHeader = BTCHeader {
        version: 536870912,
        previous_header_hash: h256_from_rev_str(
            "00000000000000a6350fbd74c4f75decdc9e49ed3c89a53d5122bc699730c6fe",
        ),
        merkle_root_hash: h256_from_rev_str(
            "048e1e4749826e877bed94c811f282c93bcab78d024cd01e0e5c3b2e86a7c0eb",
        ),
        time: 1547001773,
        bits: Compact::new(486604799),
        nonce: 2225829261,
    };

    let b4: BTCHeader = BTCHeader {
        version: 536870912,
        previous_header_hash: h256_from_rev_str(
            "000000005239e07019651d0cd871d2f4d663c827202442aff61fbc8b01c4afe8",
        ),
        merkle_root_hash: h256_from_rev_str(
            "64cc2d51b45420c4965c24ee3b0a63827291e400cad4ccc9f956db9f653e60f4",
        ),
        time: 1547001916,
        bits: Compact::new(436290411),
        nonce: 4075542957,
    };

    let b1_fork: BTCHeader = BTCHeader {
        version: 536870912,
        previous_header_hash: h256_from_rev_str(
            "00000000000000e83086b78ebc3da4af6d892963fa3fd5e1648c693de623d1b7",
        ),
        merkle_root_hash: h256_from_rev_str(
            "20c8b156c122a28d63f0344bdb38cc402b80a078eacec3de08150032c524536c",
        ),
        time: 1547002101,
        bits: Compact::new(520159231),
        nonce: 1425818149,
    };

    (vec![b0.clone(), b1, b2, b3, b4], vec![b0, b1_fork])
}

pub fn generate_mock_blocks() -> (Vec<BTCHeader>, Vec<BTCHeader>) {
    let b0: BTCHeader = BTCHeader {
        version: 536870912,
        previous_header_hash: Default::default(),
        merkle_root_hash: h256_from_rev_str(
            "815ca8bbed88af8afaa6c4995acba6e6e7453e705e0bc7039472aa3b6191a707",
        ),
        time: 1546999089,
        bits: Compact::new(436290411),
        nonce: 562223693,
    }; //1451572

    let b1: BTCHeader = BTCHeader {
        version: 536928256,
        previous_header_hash: h256_from_rev_str(
            "00000000000000fd9cea8b846895f507c63b005d20ac56e87d1cdf80effd5c0a",
        ),
        merkle_root_hash: h256_from_rev_str(
            "c16a4a6a6cc43c67770cbec9dd0cc4bf7e956d6b4c9e7c15ff1a2dc8ef3afc63",
        ),
        time: 1547000297,
        bits: Compact::new(486604799),
        nonce: 2982943095,
    };

    let b2: BTCHeader = BTCHeader {
        version: 536870912,
        previous_header_hash: h256_from_rev_str(
            "0000000000008bc1a5a3ee37368eeeb958f61464a1a5d18ed22e1430965ab3dd",
        ),
        merkle_root_hash: h256_from_rev_str(
            "14f332ae3422cfa8726f5e5fcf2d309b54ce005f3581f1f20f252772717044b5",
        ),
        time: 1547000572,
        bits: Compact::new(436290411),
        nonce: 744509129,
    };

    let b3: BTCHeader = BTCHeader {
        version: 536870912,
        previous_header_hash: h256_from_rev_str(
            "00000000000000a6350fbd74c4f75decdc9e49ed3c89a53d5122bc699730c6fe",
        ),
        merkle_root_hash: h256_from_rev_str(
            "048e1e4749826e877bed94c811f282c93bcab78d024cd01e0e5c3b2e86a7c0eb",
        ),
        time: 1547001773,
        bits: Compact::new(486604799),
        nonce: 2225829261,
    };

    let b4: BTCHeader = BTCHeader {
        version: 536870912,
        previous_header_hash: h256_from_rev_str(
            "000000005239e07019651d0cd871d2f4d663c827202442aff61fbc8b01c4afe8",
        ),
        merkle_root_hash: h256_from_rev_str(
            "64cc2d51b45420c4965c24ee3b0a63827291e400cad4ccc9f956db9f653e60f4",
        ),
        time: 1547001916,
        bits: Compact::new(436290411),
        nonce: 4075542957,
    };

    let b1_fork: BTCHeader = BTCHeader {
        version: 1,
        previous_header_hash: h256_from_rev_str(
            "0305b6acb0feee5bd7f5f74606190c35877299b881691db2e56a53452e3929f9",
        ),
        merkle_root_hash: h256_from_rev_str(
            "a93cb284a0b0cdf28a1d764ec442a59b1b77284db1fcf34d7a951710e292e400",
        ),
        time: 1540290070,
        bits: Compact::new(520159231),
        nonce: 26781,
    };

    let b2_fork: BTCHeader = BTCHeader {
        version: 1,
        previous_header_hash: h256_from_rev_str(
            "0000b7b52e51d3b424d349e9b277e35c69c5ac46856e60a6abe65c052238d429",
        ),
        merkle_root_hash: h256_from_rev_str(
            "2353cdfe80ee98f1def0d0db73c4a70049fb633cf331bdbf717ea15dfa523c86",
        ),
        time: 1540291070,
        bits: Compact::new(520159231),
        nonce: 55581,
    };
    (vec![b0.clone(), b1, b2, b3, b4], vec![b0, b1_fork, b2_fork])
}
