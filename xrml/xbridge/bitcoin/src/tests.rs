// Copyright 2019 Chainpool.

extern crate base58;
extern crate hex;
extern crate rustc_hex;
extern crate sr_primitives;
extern crate srml_consensus as consensus;
use self::base58::FromBase58;
use self::hex::FromHex;
//use self::rustc_hex::FromHex;
use self::keys::DisplayLayout;
use self::rustc_hex::ToHex;
use super::*;
use crypto::dhash160;
use runtime_io;
use runtime_io::with_externalities;
use runtime_primitives::testing::{Digest, DigestItem, Header, UintAuthorityId};
use runtime_primitives::traits::{BlakeTwo256, IdentityLookup};
use runtime_primitives::BuildStorage;
use runtime_support::StorageValue;
use script::{builder::Builder, script::Script, Opcode};
use substrate_primitives::{Blake2Hasher, H256 as S_H256};

impl_outer_origin! {
    pub enum Origin for Test {}
}

type AccountId = u64;

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
    type Header = Header;
    type Event = ();
    type Log = DigestItem;
    type Lookup = IdentityLookup<u64>;
}

impl balances::Trait for Test {
    type Balance = u64;
    type OnFreeBalanceZero = ();
    type EnsureAccountLiquid = ();
    type OnNewAccount = ();
    type Event = ();
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

impl xsystem::Trait for Test {}

pub struct MockDeterminator;

impl xaccounts::IntentionJackpotAccountIdFor<u64> for MockDeterminator {
    fn accountid_for(_: &u64) -> u64 {
        1000
    }
}

impl xaccounts::Trait for Test {
    type Event = ();
    type DetermineIntentionJackpotAccountId = MockDeterminator;
}

impl xrecords::Trait for Test {
    type Event = ();
}

impl xassets::Trait for Test {
    type Event = ();
    type OnAssetChanged = ();
    type OnAssetRegisterOrRevoke = ();
}

impl Trait for Test {
    type Event = ();
}

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
                BlockHeader {
                    version: 536870912,
                    previous_header_hash: H256::from_reversed_str(
                        "00000000f1c80c38f9bd6ebf9ca796d92122e5b2a1539ac06e09252a1a7e3d01",
                    ),
                    merkle_root_hash: H256::from_reversed_str(
                        "815ca8bbed88af8afaa6c4995acba6e6e7453e705e0bc7039472aa3b6191a707",
                    ),
                    time: 1546999089,
                    bits: Compact::new(436290411),
                    nonce: 562223693,
                },
                1451572,
            ),
            params_info: Params::new(
                520159231,            // max_bits
                2 * 60 * 60,          // block_max_future
                3,                    // max_fork_route_preset
                2 * 7 * 24 * 60 * 60, // target_timespan_seconds
                10 * 60,              // target_spacing_seconds
                4,
            ), // retargeting_factor
            network_id: 1,
            irr_block: 3,
            reserved: 2100,
            btc_fee: 1000,
            max_withdraw_amount: 100,
            _genesis_phantom_data: Default::default(),
        }
        .build_storage()
        .unwrap()
        .0,
    );
    r.into()
}

pub fn new_test_ext_err_genesisblock() -> runtime_io::TestExternalities<Blake2Hasher> {
    let mut r = system::GenesisConfig::<Test>::default()
        .build_storage()
        .unwrap()
        .0;

    // bridge btc
    r.extend(
        GenesisConfig::<Test> {
            // start genesis block: (genesis, blocknumber)
            genesis: (
                BlockHeader {
                    version: 536870912,
                    previous_header_hash: H256::from_reversed_str(
                        "00000000f1c80c38f9bd6ebf9ca796d92122e5b2a1539ac06e09252a1a7e3d01",
                    ),
                    merkle_root_hash: H256::from_reversed_str(
                        "815ca8bbed88af8afaa6c4995acba6e6e7453e705e0bc7039472aa3b6191a707",
                    ),
                    time: 1546999089,
                    bits: Compact::new(436290411),
                    nonce: 562223693,
                },
                1451572,
            ),
            params_info: Params::new(
                520159231,            // max_bits
                2 * 60 * 60,          // block_max_future
                3,                    // max_fork_route_preset
                2 * 7 * 24 * 60 * 60, // target_timespan_seconds
                10 * 60,              // target_spacing_seconds
                4,
            ), // retargeting_factor
            network_id: 1,
            irr_block: 3,
            reserved: 2100,
            btc_fee: 1000,
            max_withdraw_amount: 100,
            _genesis_phantom_data: Default::default(),
        }
        .build_storage()
        .unwrap()
        .0,
    );
    r.into()
}

type XBridgeOfBTC = Module<Test>;
type Timestamp = timestamp::Module<Test>;

fn generate_blocks() -> (Vec<BlockHeader>, Vec<BlockHeader>) {
    let b0: BlockHeader = BlockHeader {
        version: 536870912,
        previous_header_hash: Default::default(),
        merkle_root_hash: H256::from_reversed_str(
            "815ca8bbed88af8afaa6c4995acba6e6e7453e705e0bc7039472aa3b6191a707",
        ),
        time: 1546999089,
        bits: Compact::new(436290411),
        nonce: 562223693,
    }; //1451572

    let b1: BlockHeader = BlockHeader {
        version: 536928256,
        previous_header_hash: H256::from_reversed_str(
            "00000000000000fd9cea8b846895f507c63b005d20ac56e87d1cdf80effd5c0a",
        ),
        merkle_root_hash: H256::from_reversed_str(
            "c16a4a6a6cc43c67770cbec9dd0cc4bf7e956d6b4c9e7c15ff1a2dc8ef3afc63",
        ),
        time: 1547000297,
        bits: Compact::new(486604799),
        nonce: 2982943095,
    };

    let b2: BlockHeader = BlockHeader {
        version: 536870912,
        previous_header_hash: H256::from_reversed_str(
            "0000000000008bc1a5a3ee37368eeeb958f61464a1a5d18ed22e1430965ab3dd",
        ),
        merkle_root_hash: H256::from_reversed_str(
            "14f332ae3422cfa8726f5e5fcf2d309b54ce005f3581f1f20f252772717044b5",
        ),
        time: 1547000572,
        bits: Compact::new(436290411),
        nonce: 744509129,
    };

    let b3: BlockHeader = BlockHeader {
        version: 536870912,
        previous_header_hash: H256::from_reversed_str(
            "00000000000000a6350fbd74c4f75decdc9e49ed3c89a53d5122bc699730c6fe",
        ),
        merkle_root_hash: H256::from_reversed_str(
            "048e1e4749826e877bed94c811f282c93bcab78d024cd01e0e5c3b2e86a7c0eb",
        ),
        time: 1547001773,
        bits: Compact::new(486604799),
        nonce: 2225829261,
    };

    let b4: BlockHeader = BlockHeader {
        version: 536870912,
        previous_header_hash: H256::from_reversed_str(
            "000000005239e07019651d0cd871d2f4d663c827202442aff61fbc8b01c4afe8",
        ),
        merkle_root_hash: H256::from_reversed_str(
            "64cc2d51b45420c4965c24ee3b0a63827291e400cad4ccc9f956db9f653e60f4",
        ),
        time: 1547001916,
        bits: Compact::new(436290411),
        nonce: 4075542957,
    };

    let b1_fork: BlockHeader = BlockHeader {
        version: 536870912,
        previous_header_hash: H256::from_reversed_str(
            "00000000000000e83086b78ebc3da4af6d892963fa3fd5e1648c693de623d1b7",
        ),
        merkle_root_hash: H256::from_reversed_str(
            "20c8b156c122a28d63f0344bdb38cc402b80a078eacec3de08150032c524536c",
        ),
        time: 1547002101,
        bits: Compact::new(520159231),
        nonce: 1425818149,
    };

    (vec![b0.clone(), b1, b2, b3, b4], vec![b0, b1_fork])
}

fn generate_mock_blocks() -> (Vec<BlockHeader>, Vec<BlockHeader>) {
    let b0: BlockHeader = BlockHeader {
        version: 536870912,
        previous_header_hash: Default::default(),
        merkle_root_hash: H256::from_reversed_str(
            "815ca8bbed88af8afaa6c4995acba6e6e7453e705e0bc7039472aa3b6191a707",
        ),
        time: 1546999089,
        bits: Compact::new(436290411),
        nonce: 562223693,
    }; //1451572

    let b1: BlockHeader = BlockHeader {
        version: 536928256,
        previous_header_hash: H256::from_reversed_str(
            "00000000000000fd9cea8b846895f507c63b005d20ac56e87d1cdf80effd5c0a",
        ),
        merkle_root_hash: H256::from_reversed_str(
            "c16a4a6a6cc43c67770cbec9dd0cc4bf7e956d6b4c9e7c15ff1a2dc8ef3afc63",
        ),
        time: 1547000297,
        bits: Compact::new(486604799),
        nonce: 2982943095,
    };

    let b2: BlockHeader = BlockHeader {
        version: 536870912,
        previous_header_hash: H256::from_reversed_str(
            "0000000000008bc1a5a3ee37368eeeb958f61464a1a5d18ed22e1430965ab3dd",
        ),
        merkle_root_hash: H256::from_reversed_str(
            "14f332ae3422cfa8726f5e5fcf2d309b54ce005f3581f1f20f252772717044b5",
        ),
        time: 1547000572,
        bits: Compact::new(436290411),
        nonce: 744509129,
    };

    let b3: BlockHeader = BlockHeader {
        version: 536870912,
        previous_header_hash: H256::from_reversed_str(
            "00000000000000a6350fbd74c4f75decdc9e49ed3c89a53d5122bc699730c6fe",
        ),
        merkle_root_hash: H256::from_reversed_str(
            "048e1e4749826e877bed94c811f282c93bcab78d024cd01e0e5c3b2e86a7c0eb",
        ),
        time: 1547001773,
        bits: Compact::new(486604799),
        nonce: 2225829261,
    };

    let b4: BlockHeader = BlockHeader {
        version: 536870912,
        previous_header_hash: H256::from_reversed_str(
            "000000005239e07019651d0cd871d2f4d663c827202442aff61fbc8b01c4afe8",
        ),
        merkle_root_hash: H256::from_reversed_str(
            "64cc2d51b45420c4965c24ee3b0a63827291e400cad4ccc9f956db9f653e60f4",
        ),
        time: 1547001916,
        bits: Compact::new(436290411),
        nonce: 4075542957,
    };

    let b1_fork: BlockHeader = BlockHeader {
        version: 1,
        previous_header_hash: H256::from_reversed_str(
            "0305b6acb0feee5bd7f5f74606190c35877299b881691db2e56a53452e3929f9",
        ),
        merkle_root_hash: H256::from_reversed_str(
            "a93cb284a0b0cdf28a1d764ec442a59b1b77284db1fcf34d7a951710e292e400",
        ),
        time: 1540290070,
        bits: Compact::new(520159231),
        nonce: 26781,
    };

    let b2_fork: BlockHeader = BlockHeader {
        version: 1,
        previous_header_hash: H256::from_reversed_str(
            "0000b7b52e51d3b424d349e9b277e35c69c5ac46856e60a6abe65c052238d429",
        ),
        merkle_root_hash: H256::from_reversed_str(
            "2353cdfe80ee98f1def0d0db73c4a70049fb633cf331bdbf717ea15dfa523c86",
        ),
        time: 1540291070,
        bits: Compact::new(520159231),
        nonce: 55581,
    };
    (vec![b0.clone(), b1, b2, b3, b4], vec![b0, b1_fork, b2_fork])
}

fn current_time() -> u64 {
    use std::time;
    time::SystemTime::now()
        .duration_since(time::UNIX_EPOCH)
        .expect("now always later than unix epoch; qed")
        .as_secs()
}

#[test]
fn test() {
    with_externalities(&mut new_test_ext(), || {
        use substrate_primitives::hexdisplay::HexDisplay;
        let r = <BlockHeaderFor<Test>>::key_for(&H256::from_reversed_str(
            "00000000000025c23a19cc91ad8d3e33c2630ce1df594e1ae0bf0eabe30a9176",
        ));
        let a = substrate_primitives::twox_128(&r);
        println!("0x{:}", HexDisplay::from(&a));
    })
}

#[test]
fn test_init_blocks() {
    let (c1, _) = generate_blocks();

    assert_eq!(
        format!("{:?}", c1.get(0).unwrap().hash().reversed()).to_string(),
        "2c22ca732c7b99c43057df342f903ffc8a7e132e09563edb122b1f573458ac5b"
    );
    assert_eq!(
        format!("{:?}", c1.get(1).unwrap().hash().reversed()).to_string(),
        "0000000000008bc1a5a3ee37368eeeb958f61464a1a5d18ed22e1430965ab3dd"
    );
    assert_eq!(
        format!("{:?}", c1.get(2).unwrap().hash().reversed()).to_string(),
        "00000000000000a6350fbd74c4f75decdc9e49ed3c89a53d5122bc699730c6fe"
    );
    assert_eq!(
        format!("{:?}", c1.get(3).unwrap().hash().reversed()).to_string(),
        "000000005239e07019651d0cd871d2f4d663c827202442aff61fbc8b01c4afe8"
    );
    assert_eq!(
        format!("{:?}", c1.get(4).unwrap().hash().reversed()).to_string(),
        "00000000000000e83086b78ebc3da4af6d892963fa3fd5e1648c693de623d1b7"
    );
}

#[test]
fn test_init_mock_blocks() {
    let (c1, _) = generate_mock_blocks();
    assert_eq!(
        format!("{:?}", c1.get(0).unwrap().hash().reversed()).to_string(),
        "2c22ca732c7b99c43057df342f903ffc8a7e132e09563edb122b1f573458ac5b"
    );
    println!("{:?}", ser::serialize(c1.get(1).unwrap()));
    assert_eq!(
        format!("{:?}", c1.get(1).unwrap().hash().reversed()).to_string(),
        "0000000000008bc1a5a3ee37368eeeb958f61464a1a5d18ed22e1430965ab3dd"
    );
    assert_eq!(
        format!("{:?}", c1.get(2).unwrap().hash().reversed()).to_string(),
        "00000000000000a6350fbd74c4f75decdc9e49ed3c89a53d5122bc699730c6fe"
    );
    assert_eq!(
        format!("{:?}", c1.get(3).unwrap().hash().reversed()).to_string(),
        "000000005239e07019651d0cd871d2f4d663c827202442aff61fbc8b01c4afe8"
    );
    assert_eq!(
        format!("{:?}", c1.get(4).unwrap().hash().reversed()).to_string(),
        "00000000000000e83086b78ebc3da4af6d892963fa3fd5e1648c693de623d1b7"
    );
}

#[test]
fn test_genesis() {
    with_externalities(&mut new_test_ext(), || {
        let (header, num) = XBridgeOfBTC::genesis_info();
        let _r = <GenesisInfo<Test>>::get();
        let h = header.hash().reversed();
        assert_eq!(
            format!("{:?}", h).to_string(),
            "00000000000000fd9cea8b846895f507c63b005d20ac56e87d1cdf80effd5c0a"
        );
        assert_eq!(num, 1451572);

        let best_hash = XBridgeOfBTC::best_index();
        assert_eq!(best_hash, header.hash());
    })
}

#[test]
fn test_err_genesis_startnumber() {
    with_externalities(&mut new_test_ext_err_genesisblock(), || {})
}

#[test]
fn test_normal() {
    with_externalities(&mut new_test_ext(), || {
        Timestamp::set_timestamp(current_time());
        let (c1, _) = generate_blocks();
        assert_err!(
            XBridgeOfBTC::apply_push_header(c1.get(0).unwrap().clone(), &1),
            "Block parent is unknown"
        );
        assert_ok!(XBridgeOfBTC::apply_push_header(
            c1.get(1).unwrap().clone(),
            &2
        ));
        assert_ok!(XBridgeOfBTC::apply_push_header(
            c1.get(2).unwrap().clone(),
            &3
        ));

        let best_hash = XBridgeOfBTC::best_index();
        assert_eq!(best_hash, c1.get(2).unwrap().hash());
    })
}

#[test]
fn test_call() {
    with_externalities(&mut new_test_ext(), || {
        Timestamp::set_timestamp(current_time());
        let (c1, _) = generate_blocks();
        let origin = system::RawOrigin::Signed(99).into();
        let v = ser::serialize(c1.get(1).unwrap());
        let v = v.take();
        assert_ok!(XBridgeOfBTC::push_header(origin, v));
    })
}

pub fn new_test_ext2() -> runtime_io::TestExternalities<Blake2Hasher> {
    let mut r = system::GenesisConfig::<Test>::default()
        .build_storage()
        .unwrap()
        .0;

    // bridge btc
    r.extend(
        GenesisConfig::<Test> {
            // start genesis block: (genesis, blocknumber)
            genesis: (
                BlockHeader {
                    version: 536870912,
                    previous_header_hash: H256::from_reversed_str(
                        "00000000f1c80c38f9bd6ebf9ca796d92122e5b2a1539ac06e09252a1a7e3d01",
                    ),
                    merkle_root_hash: H256::from_reversed_str(
                        "815ca8bbed88af8afaa6c4995acba6e6e7453e705e0bc7039472aa3b6191a707",
                    ),
                    time: 1546999089,
                    bits: Compact::new(436290411),
                    nonce: 562223693,
                },
                1451572,
            ),
            params_info: Params::new(
                520159231,            // max_bits
                2 * 60 * 60,          // block_max_future
                3,                    // max_fork_route_preset
                2 * 7 * 24 * 60 * 60, // target_timespan_seconds
                10 * 60,              // target_spacing_seconds
                4,
            ), // retargeting_factor
            network_id: 1,
            irr_block: 3,
            reserved: 2100,
            btc_fee: 1000,
            max_withdraw_amount: 100,
            _genesis_phantom_data: Default::default(),
        }
        .build_storage()
        .unwrap()
        .0,
    );
    r.into()
}

#[test]
fn test_genesis2() {
    with_externalities(&mut new_test_ext2(), || {
        Timestamp::set_timestamp(current_time());
        let (c1, _) = generate_blocks();
        assert_err!(
            XBridgeOfBTC::apply_push_header(c1.get(0).unwrap().clone(), &1),
            "Block parent is unknown"
        );
        assert_ok!(XBridgeOfBTC::apply_push_header(
            c1.get(1).unwrap().clone(),
            &1
        ));
        assert_ok!(XBridgeOfBTC::apply_push_header(
            c1.get(2).unwrap().clone(),
            &1
        ));
        assert_ok!(XBridgeOfBTC::apply_push_header(
            c1.get(3).unwrap().clone(),
            &1
        ));
    })
}

#[allow(unused)]
pub fn new_test_ext3() -> runtime_io::TestExternalities<Blake2Hasher> {
    let mut r = system::GenesisConfig::<Test>::default()
        .build_storage()
        .unwrap()
        .0;

    // bridge btc
    r.extend(
        GenesisConfig::<Test> {
            // start genesis block: (genesis, blocknumber)
            genesis: (
                BlockHeader {
                    version: 536870912,
                    previous_header_hash: H256::from_reversed_str(
                        "00000000f1c80c38f9bd6ebf9ca796d92122e5b2a1539ac06e09252a1a7e3d01",
                    ),
                    merkle_root_hash: H256::from_reversed_str(
                        "815ca8bbed88af8afaa6c4995acba6e6e7453e705e0bc7039472aa3b6191a707",
                    ),
                    time: 1546999089,
                    bits: Compact::new(436290411),
                    nonce: 562223693,
                },
                1451572,
            ),
            params_info: Params::new(
                520159231,            // max_bits
                2 * 60 * 60,          // block_max_future
                3,                    // max_fork_route_preset
                2 * 7 * 24 * 60 * 60, // target_timespan_seconds
                10 * 60,              // target_spacing_seconds
                4,
            ), // retargeting_factor
            network_id: 1,
            irr_block: 3,
            reserved: 2100,
            btc_fee: 1000,
            max_withdraw_amount: 100,
            _genesis_phantom_data: Default::default(),
        }
        .build_storage()
        .unwrap()
        .0,
    );
    r.into()
}

#[test]
fn test_changebit() {
    with_externalities(&mut new_test_ext2(), || {
        let b1 = BlockHeader {
            version: 1,
            previous_header_hash: H256::from_reversed_str(
                "00000000864b744c5025331036aa4a16e9ed1cbb362908c625272150fa059b29",
            ),
            merkle_root_hash: H256::from_reversed_str(
                "70d6379650ac87eaa4ac1de27c21217b81a034a53abf156c422a538150bd80f4",
            ),
            time: 1337966314,
            bits: Compact::new(486604799),
            nonce: 2391008772,
        };
        // 2016
        assert_eq!(
            format!("{:?}", b1.hash().reversed()).to_string(),
            "0000000089d757fd95d79f7fcc2bc25ca7fc16492dca9aa610730ea05d9d3de9"
        );

        let _b2 = BlockHeader {
            version: 1,
            previous_header_hash: H256::from_reversed_str(
                "00000000864b744c5025331036aa4a16e9ed1cbb362908c625272150fa059b29",
            ),
            merkle_root_hash: H256::from_reversed_str(
                "70d6379650ac87eaa4ac1de27c21217b81a034a53abf156c422a538150bd80f4",
            ),
            time: 1337966314,
            bits: Compact::new(486604799),
            nonce: 2391008772,
        };
        // 2017
        assert_eq!(
            format!("{:?}", b1.hash().reversed()).to_string(),
            "0000000089d757fd95d79f7fcc2bc25ca7fc16492dca9aa610730ea05d9d3de9"
        );
    })
}

#[test]
pub fn test_address() {
    XBridgeOfBTC::verify_btc_address(&b"mqVznxoxdeSNYgDCg6ZVE5pc6476BY6zHK".to_vec()).unwrap();
}

#[test]
pub fn test_multi_address() {
    let pub1 = String::from("0311252930af8ba766b9c7a6580d8dc4bbf9b0befd17a8ef7fabac275bba77ae40");
    let pub2 = String::from("02e34d10113f2dd162e8d8614a4afbb8e2eb14eddf4036042b35d12cf5529056a2");
    let pub3 = String::from("023e505c48a955e759ce61145dc4a9a7447425290b8483f4e36f05169e7967c86d");

    let pubkey1_bytes = hex::decode(pub1).unwrap();
    let pubkey2_bytes = hex::decode(pub2).unwrap();
    let pubkey3_bytes = hex::decode(pub3).unwrap();

    let script = Builder::default()
        .push_opcode(Opcode::OP_2)
        .push_bytes(&pubkey1_bytes)
        .push_bytes(&pubkey2_bytes)
        .push_bytes(&pubkey3_bytes)
        .push_opcode(Opcode::OP_3)
        .push_opcode(Opcode::OP_CHECKMULTISIG)
        .into_script();
    //let test = hex_script!("52210311252930af8ba766b9c7a6580d8dc4bbf9b0befd17a8ef7fabac275bba77ae402102e34d10113f2dd162e8d8614a4afbb8e2eb14eddf4036042b35d12cf5529056a221023e505c48a955e759ce61145dc4a9a7447425290b8483f4e36f05169e7967c86d53ae");
    let multisig_address = Address {
        kind: keys::Type::P2SH,
        network: keys::Network::Testnet,
        hash: dhash160(&script),
    };
    assert_eq!(
        "2MtAUgQmdobnz2mu8zRXGSTwUv9csWcNwLU",
        multisig_address.to_string()
    );
}

fn create_multi_address(pubkeys: Vec<Vec<u8>>) -> Address {
    let mut build = Builder::default().push_opcode(Opcode::OP_2);
    for (i, pubkey) in pubkeys.iter().enumerate() {
        build = build.push_bytes(pubkey);
    }
    let script = build
        .push_opcode(Opcode::OP_3)
        .push_opcode(Opcode::OP_CHECKMULTISIG)
        .into_script();

    let multisig_address = Address {
        kind: keys::Type::P2SH,
        network: keys::Network::Testnet,
        hash: dhash160(&script),
    };
    multisig_address
}

#[test]
fn test_create_multi_address() {
    let pubkey1_bytes =
        hex::decode("0311252930af8ba766b9c7a6580d8dc4bbf9b0befd17a8ef7fabac275bba77ae40").unwrap();
    let pubkey2_bytes =
        hex::decode("02e34d10113f2dd162e8d8614a4afbb8e2eb14eddf4036042b35d12cf5529056a2").unwrap();
    let pubkey3_bytes =
        hex::decode("023e505c48a955e759ce61145dc4a9a7447425290b8483f4e36f05169e7967c86d").unwrap();
    let mut hot_keys = Vec::new();
    let mut cold_keys = Vec::new();
    hot_keys.push(pubkey1_bytes.clone());
    hot_keys.push(pubkey2_bytes.clone());
    hot_keys.push(pubkey3_bytes.clone());
    cold_keys.push(pubkey1_bytes);
    cold_keys.push(pubkey2_bytes);
    cold_keys.push(pubkey3_bytes);
    let hot_addr = create_multi_address(hot_keys);
    let cold_addr = create_multi_address(cold_keys);

    assert_eq!(hot_addr, cold_addr);

    let layout_addr = cold_addr.layout().to_vec();
    let layout = [
        196, 10, 18, 79, 99, 6, 23, 210, 211, 220, 115, 137, 86, 4, 75, 195, 77, 76, 168, 39, 29,
        191, 246, 217, 153,
    ];

    assert_eq!(layout_addr, layout);

    let addr = Address::from_layout(&mut layout_addr.as_slice()).unwrap();

    assert_eq!(cold_addr, addr);

    let pks = [
        169, 20, 10, 18, 79, 99, 6, 23, 210, 211, 220, 115, 137, 86, 4, 75, 195, 77, 76, 168, 39,
        29, 135,
    ];

    let pk = addr.hash.clone().to_vec();
    let mut pubkeys = Vec::new();
    pubkeys.push(Opcode::OP_HASH160 as u8);
    pubkeys.push(Opcode::OP_PUSHBYTES_20 as u8);
    for p in pk {
        pubkeys.push(p)
    }
    pubkeys.push(Opcode::OP_EQUAL as u8);
    assert_eq!(pubkeys, pks);
}

#[test]
fn test_accountid() {
    let acc_btc = H256::from("f4a03666cceb90cb1d50c7d17e87da34fee209550d65c7622c924e82c95aee43");
    let acc_xiaomi = H256::from("10bffec4d267786994ee83bf76f4490ad33ce68f320dbb6403c3d1b1c96eb1ca");
    let acc_zte = H256::from("1d7dcb4d81134b6103d0a21423e7fb7bbd3d17256c195263285cc8457d6cb714");
    let mut init = Vec::new();
    init.push(acc_btc);
    init.push(acc_xiaomi);
    init.push(acc_zte);
    println!("init[0] {:?}", init[0]);
    println!("init {:?}", init);
}
