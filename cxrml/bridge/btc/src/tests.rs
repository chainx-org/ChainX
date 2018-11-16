extern crate srml_consensus as consensus;

use substrate_primitives::{Blake2Hasher, H256 as S_H256};

use self::base58::FromBase58;
use super::*;
use runtime_io;
use runtime_io::with_externalities;
use self::keys::DisplayLayout;
use runtime_primitives::testing::{Digest, DigestItem, Header};
use runtime_primitives::traits::BlakeTwo256;
use runtime_primitives::BuildStorage;
use runtime_support::StorageValue;

impl_outer_origin! {
    pub enum Origin for Test {}
}

pub type AccountId = u64;

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
}

impl balances::Trait for Test {
    type Balance = u64;
    type AccountIndex = u64;
    type OnFreeBalanceZero = ();
    type EnsureAccountLiquid = ();
    type Event = ();
}

impl consensus::Trait for Test {
    const NOTE_OFFLINE_POSITION: u32 = 1;
    type Log = DigestItem;
    type SessionKey = u64;
    type OnOfflineValidator = ();
}

impl timestamp::Trait for Test {
    const TIMESTAMP_SET_POSITION: u32 = 0;
    type Moment = u64;
}

impl cxsystem::Trait for Test {}

impl associations::Trait for Test {
    type OnCalcFee = cxsupport::Module<Test>;
    type Event = ();
}

impl cxsupport::Trait for Test {}

impl tokenbalances::Trait for Test {
    const CHAINX_SYMBOL: tokenbalances::SymbolString = b"pcx";
    const CHAINX_TOKEN_DESC: tokenbalances::DescString = b"this is pcx for mock";
    type TokenBalance = u128;
    type Event = ();
}

impl financial_records::Trait for Test {
    type Event = ();
}

impl Trait for Test {
    type Event = ();
}

pub fn new_test_ext() -> runtime_io::TestExternalities<Blake2Hasher> {
    let mut r = system::GenesisConfig::<Test>::default()
        .build_storage()
        .unwrap();

    // bridge btc
    r.extend(
        GenesisConfig::<Test> {
            // start genesis block: (genesis, blocknumber)
            genesis: (
                BlockHeader {
                    version: 1,
                    previous_header_hash: Default::default(),
                    merkle_root_hash: H256::from_reversed_str(
                        "4a5e1e4baab89f3a32518a88c31bc87f618f76673e2cc77ab2127b7afdeda33b",
                    ),
                    time: 1296688602,
                    bits: Compact::new(486604799),
                    nonce: 414098458,
                },
                0,
            ),
            params_info: Params::new(
                520159231, // max_bits
                2 * 60 * 60, // block_max_future
                64, // max_fork_route_preset
                2 * 7 * 24 * 60 * 60, // target_timespan_seconds
                10 * 60, // target_spacing_seconds
                4,
            ), // retargeting_factor
            network_id: 1,
            utxo_max_index: 0,
            irr_block: 6,
            btc_fee: 10,
            accounts_max_index: 0,
            receive_address: keys::Address::from_layout(&"2N4C127fBSmqBsNuHeLmAbZEVSPfV6GB2j2".from_base58().unwrap()).unwrap(),
            redeem_script: b"52210257aff1270e3163aaae9d972b3d09a2385e0d4877501dbeca3ee045f8de00d21c2103fd58c689594b87bbe20a9a00091d074dc0d9f49a988a7ad4c2575adeda1b507c2102bb2a5aa53ba7c0d77bdd86bb9553f77dd0971d3a6bb6ad609787aa76eb17b6b653ae".to_vec(),
            fee: 0,
        }.build_storage()
            .unwrap(),
    );
    r.into()
}

pub fn new_test_ext_err_genesisblock() -> runtime_io::TestExternalities<Blake2Hasher> {
    let mut r = system::GenesisConfig::<Test>::default()
        .build_storage()
        .unwrap();

    // bridge btc
    r.extend(
        GenesisConfig::<Test> {
            // start genesis block: (genesis, blocknumber)
            genesis: (
                BlockHeader {
                    version: 1,
                    previous_header_hash: Default::default(),
                    merkle_root_hash: H256::from_reversed_str(
                        "4a5e1e4baab89f3a32518a88c31bc87f618f76673e2cc77ab2127b7afdeda33b",
                    ),
                    time: 1296688602,
                    bits: Compact::new(486604799),
                    nonce: 414098458,
                },
                5,
            ),
            params_info: Params::new(
                520159231, // max_bits
                2 * 60 * 60, // block_max_future
                64, // max_fork_route_preset
                2 * 7 * 24 * 60 * 60, // target_timespan_seconds
                10 * 60, // target_spacing_seconds
                4,
            ), // retargeting_factor
            network_id: 0,
            utxo_max_index: 0,
            irr_block: 6,
            btc_fee: 10,
            accounts_max_index: 0,
            receive_address: keys::Address::from_layout(&"2N4C127fBSmqBsNuHeLmAbZEVSPfV6GB2j2".from_base58().unwrap()).unwrap(),
            redeem_script: b"52210257aff1270e3163aaae9d972b3d09a2385e0d4877501dbeca3ee045f8de00d21c2103fd58c689594b87bbe20a9a00091d074dc0d9f49a988a7ad4c2575adeda1b507c2102bb2a5aa53ba7c0d77bdd86bb9553f77dd0971d3a6bb6ad609787aa76eb17b6b653ae".to_vec(),
            fee: 0,
        }.build_storage()
            .unwrap(),
    );
    r.into()
}

type BridgeOfBTC = Module<Test>;
type Timestamp = timestamp::Module<Test>;

fn generate_blocks() -> (Vec<BlockHeader>, Vec<BlockHeader>) {
    let b0: BlockHeader = BlockHeader {
        version: 1,
        previous_header_hash: Default::default(),
        merkle_root_hash: H256::from_reversed_str(
            "4a5e1e4baab89f3a32518a88c31bc87f618f76673e2cc77ab2127b7afdeda33b",
        ),
        time: 1296688602,
        bits: Compact::new(486604799),
        nonce: 414098458,
    };

    let b1: BlockHeader = BlockHeader {
        version: 1,
        previous_header_hash: H256::from_reversed_str(
            "000000000933ea01ad0ee984209779baaec3ced90fa3f408719526f8d77f4943",
        ),
        merkle_root_hash: H256::from_reversed_str(
            "f0315ffc38709d70ad5647e22048358dd3745f3ce3874223c80a7c92fab0c8ba",
        ),
        time: 1296688928,
        bits: Compact::new(486604799),
        nonce: 1924588547,
    };

    let b2: BlockHeader = BlockHeader {
        version: 1,
        previous_header_hash: H256::from_reversed_str(
            "00000000b873e79784647a6c82962c70d228557d24a747ea4d1b8bbe878e1206",
        ),
        merkle_root_hash: H256::from_reversed_str(
            "20222eb90f5895556926c112bb5aa0df4ab5abc3107e21a6950aec3b2e3541e2",
        ),
        time: 1296688946,
        bits: Compact::new(486604799),
        nonce: 875942400,
    };

    let b3: BlockHeader = BlockHeader {
        version: 1,
        previous_header_hash: H256::from_reversed_str(
            "000000006c02c8ea6e4ff69651f7fcde348fb9d557a06e6957b65552002a7820",
        ),
        merkle_root_hash: H256::from_reversed_str(
            "71241692d7adc0980c018e764a50974f59e1657ba88a1b1503ae2a53fc5aba41",
        ),
        time: 1296689030,
        bits: Compact::new(486604799),
        nonce: 3066203397,
    };

    let b4: BlockHeader = BlockHeader {
        version: 1,
        previous_header_hash: H256::from_reversed_str(
            "000000008b896e272758da5297bcd98fdc6d97c9b765ecec401e286dc1fdbe10",
        ),
        merkle_root_hash: H256::from_reversed_str(
            "2d1f657e970f724c4cd690494152a83bd297cd10e86ed930daa2dd76576d974c",
        ),
        time: 1296689066,
        bits: Compact::new(486604799),
        nonce: 1081518338,
    };

    let b1_fork: BlockHeader = BlockHeader {
        version: 536870912,
        previous_header_hash: H256::from_reversed_str(
            "000000000933ea01ad0ee984209779baaec3ced90fa3f408719526f8d77f4943",
        ),
        merkle_root_hash: H256::from_reversed_str(
            "4cb0721431b48ebbc38f3b5fc10825b4c43a4a2680b5e60cad6487a3f66e24a1",
        ),
        time: 1540346221,
        bits: Compact::new(520159231),
        nonce: 11914,
    };

    (vec![b0.clone(), b1, b2, b3, b4], vec![b0, b1_fork])
}

pub fn new_test_mock_ext() -> runtime_io::TestExternalities<Blake2Hasher> {
    let (c1, _) = generate_mock_blocks();
    let mut r = system::GenesisConfig::<Test>::default()
        .build_storage()
        .unwrap();

    // bridge btc
    r.extend(
        GenesisConfig::<Test> {
            // start genesis block: (genesis, blocknumber)
            genesis: (c1.get(0).unwrap().clone(), 0),
            //        genesis: Default::default(),
            params_info: Params::new(
                520159231, // max_bits
                2 * 60 * 60, // block_max_future
                64, // max_fork_route_preset
                2 * 7 * 24 * 60 * 60, // target_timespan_seconds
                10 * 60, // target_spacing_seconds
                4,
            ), // retargeting_factor
            network_id: 1,
            utxo_max_index: 0,
            irr_block: 6,
            btc_fee: 10,
            accounts_max_index: 0,
            receive_address: keys::Address::from_layout(&"2N4C127fBSmqBsNuHeLmAbZEVSPfV6GB2j2".from_base58().unwrap()).unwrap(),
            redeem_script: b"52210257aff1270e3163aaae9d972b3d09a2385e0d4877501dbeca3ee045f8de00d21c2103fd58c689594b87bbe20a9a00091d074dc0d9f49a988a7ad4c2575adeda1b507c2102bb2a5aa53ba7c0d77bdd86bb9553f77dd0971d3a6bb6ad609787aa76eb17b6b653ae".to_vec(),
            fee: 0,
        }.build_storage()
            .unwrap(),
    );
    r.into()
}

fn generate_mock_blocks() -> (Vec<BlockHeader>, Vec<BlockHeader>) {
    let b0: BlockHeader = BlockHeader {
        version: 1,
        previous_header_hash: Default::default(),
        merkle_root_hash: H256::from_reversed_str(
            "c710fae3c9e56fe3ca026250a89b036df51deaa90e623e9117c728da3016f507",
        ),
        time: 1540289095,
        bits: Compact::new(520159231),
        nonce: 6515,
    };

    let b1: BlockHeader = BlockHeader {
        version: 1,
        previous_header_hash: H256::from_reversed_str(
            "0305b6acb0feee5bd7f5f74606190c35877299b881691db2e56a53452e3929f9",
        ),
        merkle_root_hash: H256::from_reversed_str(
            "ae8c8aef64ad8dda4370bfac1a4e31da0885d3ab32bfe40a5b031bf0c5fd387d",
        ),
        time: 1540290095,
        bits: Compact::new(520159231),
        nonce: 19020,
    };

    let b2: BlockHeader = BlockHeader {
        version: 1,
        previous_header_hash: H256::from_reversed_str(
            "000097386320b37dd82e9f79484f72b8adb548e4320e65b68a744435109c511b",
        ),
        merkle_root_hash: H256::from_reversed_str(
            "c8987a87716b1fcb187dfa12efefa7cfd44ffa4dbcd7fef97385172f91897a12",
        ),
        time: 1540291095,
        bits: Compact::new(520159231),
        nonce: 99402,
    };

    let b3: BlockHeader = BlockHeader {
        version: 1,
        previous_header_hash: H256::from_reversed_str(
            "00000620f148f387d1e723f42128288ba3daea01df62546f45046afb761ac29b",
        ),
        merkle_root_hash: H256::from_reversed_str(
            "de35b4bfa5b0c40afc9750e828c95f7f8e279361d6818449a9ac30e8c8ceb39b",
        ),
        time: 1540292095,
        bits: Compact::new(520159231),
        nonce: 1684,
    };

    let b4: BlockHeader = BlockHeader {
        version: 1,
        previous_header_hash: H256::from_reversed_str(
            "0000b9bfc9a7d24d664bcf24107162619017e9a4959c0d48dbb63b193a30ffed",
        ),
        merkle_root_hash: H256::from_reversed_str(
            "4cf189a478806ac29874f721c83ce6c932eb061bbb62e011b34207fb093b2e4f",
        ),
        time: 1540293095,
        bits: Compact::new(520159231),
        nonce: 25207,
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
        "000000000933ea01ad0ee984209779baaec3ced90fa3f408719526f8d77f4943"
    );
    assert_eq!(
        format!("{:?}", c1.get(1).unwrap().hash().reversed()).to_string(),
        "00000000b873e79784647a6c82962c70d228557d24a747ea4d1b8bbe878e1206"
    );
    assert_eq!(
        format!("{:?}", c1.get(2).unwrap().hash().reversed()).to_string(),
        "000000006c02c8ea6e4ff69651f7fcde348fb9d557a06e6957b65552002a7820"
    );
    assert_eq!(
        format!("{:?}", c1.get(3).unwrap().hash().reversed()).to_string(),
        "000000008b896e272758da5297bcd98fdc6d97c9b765ecec401e286dc1fdbe10"
    );
    assert_eq!(
        format!("{:?}", c1.get(4).unwrap().hash().reversed()).to_string(),
        "000000008b5d0af9ffb1741e38b17b193bd12d7683401cecd2fd94f548b6e5dd"
    );
}

#[test]
fn test_init_mock_blocks() {
    let (c1, c2) = generate_mock_blocks();
    assert_eq!(
        format!("{:?}", c1.get(0).unwrap().hash().reversed()).to_string(),
        "0305b6acb0feee5bd7f5f74606190c35877299b881691db2e56a53452e3929f9"
    );
    println!("{:?}", ser::serialize(c1.get(1).unwrap()));
    assert_eq!(
        format!("{:?}", c1.get(1).unwrap().hash().reversed()).to_string(),
        "000097386320b37dd82e9f79484f72b8adb548e4320e65b68a744435109c511b"
    );
    assert_eq!(
        format!("{:?}", c1.get(2).unwrap().hash().reversed()).to_string(),
        "00000620f148f387d1e723f42128288ba3daea01df62546f45046afb761ac29b"
    );
    assert_eq!(
        format!("{:?}", c1.get(3).unwrap().hash().reversed()).to_string(),
        "0000b9bfc9a7d24d664bcf24107162619017e9a4959c0d48dbb63b193a30ffed"
    );
    assert_eq!(
        format!("{:?}", c1.get(4).unwrap().hash().reversed()).to_string(),
        "000004c5b1a732e06f9e589c35f8ba117d20c94d3c72f1994ce24c54a95da9de"
    );

    assert_eq!(
        format!("{:?}", c2.get(1).unwrap().hash().reversed()).to_string(),
        "0000b7b52e51d3b424d349e9b277e35c69c5ac46856e60a6abe65c052238d429"
    );
    assert_eq!(
        format!("{:?}", c2.get(2).unwrap().hash().reversed()).to_string(),
        "00005594feab27dfa44581d158f5bd0fa2940567ff7d140b90afeabc00701b47"
    );
}

#[test]
fn test_genesis() {
    with_externalities(&mut new_test_ext(), || {
        let (header, num) = BridgeOfBTC::genesis_info();
        let _r = <GenesisInfo<Test>>::get();
        let h = header.hash().reversed();
        assert_eq!(
            format!("{:?}", h).to_string(),
            "000000000933ea01ad0ee984209779baaec3ced90fa3f408719526f8d77f4943"
        );
        assert_eq!(num, 0);

        assert_eq!(
            header.hash(),
            *BridgeOfBTC::hashs_for_num(0).get(0).unwrap()
        );
        assert_eq!(BridgeOfBTC::num_for_hash(header.hash()).unwrap(), 0);

        let hh = BridgeOfBTC::hashs_for_num(0).get(0).unwrap().clone();
        if let Some((a, b, _)) = BridgeOfBTC::block_header_for(&hh) {
            assert_eq!(b, AccountId::default());
            assert_eq!(a.hash(), header.hash());
        } else {
            panic!("should not hit this branch");
        }

        let best = BridgeOfBTC::best_index();
        assert_eq!(best.hash, header.hash());
    })
}

#[test]
#[should_panic]
fn test_err_genesis_startnumber() {
    with_externalities(&mut new_test_ext_err_genesisblock(), || {})
}

#[test]
fn test_normal() {
    with_externalities(&mut new_test_ext(), || {
        Timestamp::set_timestamp(current_time());
        let (c1, _) = generate_blocks();
        assert_err!(
            BridgeOfBTC::process_header(c1.get(0).unwrap().clone(), &1),
            "already store this header"
        );
        assert_ok!(BridgeOfBTC::process_header(c1.get(1).unwrap().clone(), &2));
        assert_ok!(BridgeOfBTC::process_header(c1.get(2).unwrap().clone(), &3));

        let best = BridgeOfBTC::best_index();
        assert_eq!(best.hash, c1.get(2).unwrap().hash());
        assert_eq!(best.number, 2);

        assert_eq!(
            BridgeOfBTC::hashs_for_num(1),
            [c1.get(1).unwrap().hash()].to_vec()
        );
        assert_eq!(
            BridgeOfBTC::hashs_for_num(2),
            [c1.get(2).unwrap().hash()].to_vec()
        );
    })
}

#[test]
fn test_fork() {
    with_externalities(&mut new_test_mock_ext(), || {
        Timestamp::set_timestamp(current_time());
        let (c1, c2) = generate_mock_blocks();
        assert_err!(
            BridgeOfBTC::process_header(c1.get(0).unwrap().clone(), &1),
            "already store this header"
        );
        // insert normal
        assert_ok!(BridgeOfBTC::process_header(c1.get(1).unwrap().clone(), &2));

        let best = BridgeOfBTC::best_index();
        assert_eq!(best.hash, c1.get(1).unwrap().hash());
        assert_eq!(best.number, 1);

        // insert fork same height
        assert_ok!(BridgeOfBTC::process_header(c2.get(1).unwrap().clone(), &2));
        let best = BridgeOfBTC::best_index();
        assert_eq!(best.hash, c1.get(1).unwrap().hash());
        assert_eq!(best.number, 1);
        assert_eq!(
            BridgeOfBTC::hashs_for_num(1),
            [c1.get(1).unwrap().hash()].to_vec()
        );

        assert_eq!(
            BridgeOfBTC::num_for_hash(c1.get(1).unwrap().hash()),
            Some(1)
        );
        assert_eq!(BridgeOfBTC::num_for_hash(c2.get(1).unwrap().hash()), None);

        // insert fork
        assert_ok!(BridgeOfBTC::process_header(c2.get(2).unwrap().clone(), &3));
        // switch
        let best = BridgeOfBTC::best_index();
        assert_eq!(best.hash, c2.get(2).unwrap().hash());
        assert_eq!(best.number, 2);

        // number 1 hash is the fork chain hash
        assert_eq!(
            BridgeOfBTC::hashs_for_num(1),
            [c1.get(1).unwrap().hash(), c2.get(1).unwrap().hash()].to_vec()
        );

        // insert source 2,3, switch to source
        assert_ok!(BridgeOfBTC::process_header(c1.get(2).unwrap().clone(), &2));
        assert_ok!(BridgeOfBTC::process_header(c1.get(3).unwrap().clone(), &2));

        let best = BridgeOfBTC::best_index();
        assert_eq!(best.hash, c1.get(3).unwrap().hash());
        assert_eq!(best.number, 3);

        assert_eq!(
            BridgeOfBTC::hashs_for_num(1),
            [c1.get(1).unwrap().hash(), c2.get(1).unwrap().hash()].to_vec()
        );
        assert_eq!(
            BridgeOfBTC::hashs_for_num(2),
            [c2.get(2).unwrap().hash(), c1.get(2).unwrap().hash()].to_vec()
        );
    })
}

#[test]
fn test_orphan() {
    with_externalities(&mut new_test_ext(), || {
        Timestamp::set_timestamp(current_time());
        let (c1, _) = generate_blocks();
        assert_err!(
            BridgeOfBTC::process_header(c1.get(2).unwrap().clone(), &3),
            "can't find the prev header in ChainX, may be a orphan block"
        );
        assert_ok!(BridgeOfBTC::process_header(c1.get(1).unwrap().clone(), &2));
        assert_ok!(BridgeOfBTC::process_header(c1.get(2).unwrap().clone(), &3));
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
        assert_ok!(BridgeOfBTC::push_header(origin, v));
        let h = BridgeOfBTC::hashs_for_num(1).get(0).unwrap().clone();
        let (_, who, _) = BridgeOfBTC::block_header_for(&h).unwrap();
        assert_eq!(who, 99);
    })
}

pub fn new_test_ext2() -> runtime_io::TestExternalities<Blake2Hasher> {
    let mut r = system::GenesisConfig::<Test>::default()
        .build_storage()
        .unwrap();

    // bridge btc
    r.extend(
        GenesisConfig::<Test> {
            // start genesis block: (genesis, blocknumber)
            genesis: (
                BlockHeader {
                    version: 1,
                    previous_header_hash: H256::from_reversed_str(
                        "00000000b873e79784647a6c82962c70d228557d24a747ea4d1b8bbe878e1206",
                    ),
                    merkle_root_hash: H256::from_reversed_str(
                        "20222eb90f5895556926c112bb5aa0df4ab5abc3107e21a6950aec3b2e3541e2",
                    ),
                    time: 1296688946,
                    bits: Compact::new(486604799),
                    nonce: 875942400,
                },
                2,
            ),
            //        genesis: Default::default(),
            params_info: Params::new(
                486604799, // max_bits
                2 * 60 * 60, // block_max_future
                64, // max_fork_route_preset
                2 * 7 * 24 * 60 * 60, // target_timespan_seconds
                10 * 60, // target_spacing_seconds
                4,
            ), // retargeting_factor
            network_id: 1,
            utxo_max_index: 0,
            irr_block: 6,
            btc_fee: 10,
            accounts_max_index: 0,
            receive_address: keys::Address::from_layout(&"2N4C127fBSmqBsNuHeLmAbZEVSPfV6GB2j2".from_base58().unwrap()).unwrap(),
            redeem_script: b"52210257aff1270e3163aaae9d972b3d09a2385e0d4877501dbeca3ee045f8de00d21c2103fd58c689594b87bbe20a9a00091d074dc0d9f49a988a7ad4c2575adeda1b507c2102bb2a5aa53ba7c0d77bdd86bb9553f77dd0971d3a6bb6ad609787aa76eb17b6b653ae".to_vec(),
            fee: 0,
        }.build_storage()
            .unwrap(),
    );
    r.into()
}

#[test]
fn test_genesis2() {
    with_externalities(&mut new_test_ext2(), || {
        Timestamp::set_timestamp(current_time());
        let (c1, _) = generate_blocks();
        let best = BridgeOfBTC::best_index();
        assert_eq!(best.number, 2);
        assert_err!(
            BridgeOfBTC::process_header(c1.get(0).unwrap().clone(), &1),
            "can't find the prev header in ChainX, may be a orphan block"
        );
        assert_err!(
            BridgeOfBTC::process_header(c1.get(1).unwrap().clone(), &1),
            "can't find the prev header in ChainX, may be a orphan block"
        );
        assert_err!(
            BridgeOfBTC::process_header(c1.get(2).unwrap().clone(), &1),
            "already store this header"
        );
        assert_ok!(BridgeOfBTC::process_header(c1.get(3).unwrap().clone(), &1));

        let best = BridgeOfBTC::best_index();
        assert_eq!(best.number, 3);
    })
}

pub fn new_test_ext3() -> runtime_io::TestExternalities<Blake2Hasher> {
    let mut r = system::GenesisConfig::<Test>::default()
        .build_storage()
        .unwrap();

    // bridge btc
    r.extend(
        GenesisConfig::<Test> {
            // start genesis block: (genesis, blocknumber)
            genesis: (
                BlockHeader {
                    version: 1,
                    previous_header_hash: H256::from_reversed_str(
                        "00000000b873e79784647a6c82962c70d228557d24a747ea4d1b8bbe878e1206",
                    ),
                    merkle_root_hash: H256::from_reversed_str(
                        "20222eb90f5895556926c112bb5aa0df4ab5abc3107e21a6950aec3b2e3541e2",
                    ),
                    time: 1296688946,
                    bits: Compact::new(486604799),
                    nonce: 875942400,
                },
                2,
            ),
            //        genesis: Default::default(),
            params_info: Params::new(
                486604799, // max_bits
                2 * 60 * 60, // block_max_future
                64, // max_fork_route_preset
                2 * 7 * 24 * 60 * 60, // target_timespan_seconds
                10 * 60, // target_spacing_seconds
                4,
            ), // retargeting_factor
            network_id: 1,
            utxo_max_index: 0,
            irr_block: 6,
            btc_fee: 10,
            accounts_max_index: 0,
            receive_address: keys::Address::from_layout(&"2N4C127fBSmqBsNuHeLmAbZEVSPfV6GB2j2".from_base58().unwrap()).unwrap(),
            redeem_script: b"52210257aff1270e3163aaae9d972b3d09a2385e0d4877501dbeca3ee045f8de00d21c2103fd58c689594b87bbe20a9a00091d074dc0d9f49a988a7ad4c2575adeda1b507c2102bb2a5aa53ba7c0d77bdd86bb9553f77dd0971d3a6bb6ad609787aa76eb17b6b653ae".to_vec(),
            fee: 0,
        }.build_storage()
            .unwrap(),
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
    BridgeOfBTC::verify_btc_address(&"mjKE11gjVN4JaC9U8qL6ZB5vuEBgmwik7b".from_base58().unwrap())
        .unwrap();
}
