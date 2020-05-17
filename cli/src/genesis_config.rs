// Copyright 2018-2019 Chainpool.

use hex::FromHex;
use serde_derive::Deserialize;
use substrate_primitives::{crypto::UncheckedInto, ed25519::Public as AuthorityId};

use chainx_primitives::AccountId;
use chainx_runtime::{
    xassets::{self, Asset, Chain, ChainT},
    xbitcoin::{self, Params},
    xbridge_common::types::TrusteeInfoConfig,
    Runtime,
};
use chainx_runtime::{
    xcontracts, ChainSpec, ConsensusConfig, GenesisConfig, SessionConfig, TimestampConfig,
    XAssetsConfig, XBootstrapConfig, XBridgeFeaturesConfig, XBridgeOfBTCConfig,
    XBridgeOfSDOTConfig, XContractsConfig, XFeeManagerConfig, XSpotConfig, XStakingConfig,
    XSystemConfig, XTokensConfig,
};

use btc_chain::BlockHeader;
use btc_primitives::{h256_from_rev_str, Compact};

#[derive(Copy, Clone)]
pub enum GenesisSpec {
    Dev,
    Testnet,
    TestnetMohism,
    Mainnet,
}
impl Into<ChainSpec> for GenesisSpec {
    fn into(self) -> ChainSpec {
        match self {
            GenesisSpec::Dev => ChainSpec::Dev,
            GenesisSpec::Testnet => ChainSpec::Testnet,
            GenesisSpec::TestnetMohism => ChainSpec::Testnet,
            GenesisSpec::Mainnet => ChainSpec::Mainnet,
        }
    }
}

const PCX_PRECISION: u16 = 8;

fn hex(account: &str) -> [u8; 32] {
    <[u8; 32] as FromHex>::from_hex(account).unwrap()
}

pub fn genesis(genesis_spec: GenesisSpec) -> GenesisConfig {
    // Load all sdot address and quantity.
    let sdot_claims = load_sdot_info().unwrap();
    let testnet_taoism_bitcoin = (
        (
            BlockHeader {
                version: 536870912,
                previous_header_hash: h256_from_rev_str(
                    "00000000000b494dc8ec94e46e2d111c6b9a317e7300494544a147e15371ff58",
                ),
                merkle_root_hash: h256_from_rev_str(
                    "670cd88ba0dd51650a444c744b8088653dba381bf091366ecc41dba0e1b483ff",
                ),
                time: 1573628044,
                bits: Compact::new(436469756),
                nonce: 3891368516,
            },
            1608246,
        ),
        h256_from_rev_str("00000000000000927abc8c28ddd2c0ee46cc47dadb4c45ee14ff2a0307e1b896"),
        1, // bitcoin testnet
    );
    let testnet_mohism_bitcoin = (
        (
            BlockHeader {
                version: 0x20400000,
                previous_header_hash: h256_from_rev_str(
                    "00000000747224aab97a80577eb3fefcc6e182ccb916a2d9f16b3cdee6ac46bc",
                ),
                merkle_root_hash: h256_from_rev_str(
                    "e3b332dfe87440c2e9c106fa32de1eb63adde90748a7f6e9eff7c23e09926690",
                ),
                time: 1589721767,
                bits: Compact::new(0x1a0ffff0),
                nonce: 0x6ba03668,
            },
            1745290,
        ),
        h256_from_rev_str("0000000000000afef24ac300f11b64115335471fa46dd8f8a8b4f9fe575ad38b"),
        1, // bitcoin testnet
    );
    let mainnet_bitcoin = (
        (
            BlockHeader {
                version: 536870912,
                previous_header_hash: h256_from_rev_str(
                    "0000000000000000000a4adf6c5192128535d4dcb56cfb5753755f8d392b26bf",
                ),
                merkle_root_hash: h256_from_rev_str(
                    "1d21e60acb0b12e5cfd3f775edb647f982a2d666f9886b2f61ea5e72577b0f5e",
                ),
                time: 1558168296,
                bits: Compact::new(388627269),
                nonce: 1439505020,
            },
            576576,
        ),
        h256_from_rev_str("0000000000000000001721f58deb88b0710295a02551f0dde1e2e231a15f1882"),
        0, // bitcoin mainnet
    );

    let (code, mut genesis_node_info, team_council, network_props, bitcoin) = match genesis_spec {
        GenesisSpec::Dev => (
            include_bytes!("../../runtime/wasm/target/wasm32-unknown-unknown/release/chainx_runtime.compact.wasm").to_vec(), // dev genesis runtime version is 6
            load_genesis_node_info(&include_bytes!("dev_genesis_node.csv")[..]).unwrap(),
            load_team_council_info(&include_bytes!("dev_team_council.csv")[..]).unwrap(),
            (xsystem::NetworkType::Testnet, 42),
            testnet_mohism_bitcoin, // dev use a newer bitcoin header
        ),
        GenesisSpec::Testnet => (
            include_bytes!("./chainx_runtime.compact.wasm").to_vec(), // testnet genesis runtime version is 6
            load_genesis_node_info(&include_bytes!("testnet_genesis_node.csv")[..]).unwrap(),
            load_team_council_info(&include_bytes!("testnet_team_council.csv")[..]).unwrap(),
            (xsystem::NetworkType::Testnet, 42),
            testnet_taoism_bitcoin,
        ),
        GenesisSpec::TestnetMohism => (
            include_bytes!("./testnet_mohism_chainx_runtime.compact.wasm").to_vec(), // testnet genesis runtime version is 6
            load_genesis_node_info(&include_bytes!("testnet_genesis_node.csv")[..]).unwrap(),
            load_team_council_info(&include_bytes!("testnet_team_council.csv")[..]).unwrap(),
            (xsystem::NetworkType::Testnet, 42),
            testnet_mohism_bitcoin,
        ),
        GenesisSpec::Mainnet => (
            include_bytes!("./mainnet_chainx_runtime.compact.wasm").to_vec(), // mainnet genesis runtime version is 0
            load_genesis_node_info(&include_bytes!("mainnet_genesis_node.csv")[..]).unwrap(),
            load_team_council_info(&include_bytes!("mainnet_team_council.csv")[..]).unwrap(),
            (xsystem::NetworkType::Mainnet, 44),
            mainnet_bitcoin,
        ),
    };
    let contracts_config = match genesis_spec {
        GenesisSpec::Dev => Some(XContractsConfig {
            current_schedule: xcontracts::Schedule {
                enable_println: true, // this should only be enabled on development chains
                ..Default::default()
            },
            gas_price: 5,
        }),
        _ => None,
    };

    assert_eq!(team_council.len(), 8);

    let team_account = team_council[..3].to_vec();
    let council_account = team_council[3..8].to_vec();

    let initial_authorities_len = match genesis_spec {
        GenesisSpec::Dev => 1,
        GenesisSpec::Testnet => genesis_node_info.len(),
        GenesisSpec::TestnetMohism => genesis_node_info.len(),
        GenesisSpec::Mainnet => genesis_node_info.len(),
    };

    const CONSENSUS_TIME: u64 = 1;
    let btc_asset = Asset::new(
        <xbitcoin::Module<Runtime> as ChainT>::TOKEN.to_vec(), // token
        b"X-BTC".to_vec(),
        Chain::Bitcoin,
        8, // bitcoin precision
        b"ChainX's Cross-chain Bitcoin".to_vec(),
    )
    .unwrap();

    let sdot_asset = Asset::new(
        b"SDOT".to_vec(), // token
        b"Shadow DOT".to_vec(),
        Chain::Ethereum,
        3, //  precision
        b"ChainX's Shadow Polkadot from Ethereum".to_vec(),
    )
    .unwrap();

    genesis_node_info.truncate(initial_authorities_len);

    let blocks_per_session = 150; // 150 blocks per session
    let sessions_per_era = 12; // update validators set per 12 sessions
    let sessions_per_epoch = sessions_per_era * 10; // update trustees set per 12*10 sessions
    let bonding_duration = blocks_per_session * sessions_per_era * 72; // freeze 150*12*72 blocks for non-intention
    let intention_bonding_duration = bonding_duration * 10; // freeze 150*12*10 blocks for intention

    let params_info = Params::new(
        486604799,            // max_bits
        2 * 60 * 60,          // block_max_future
        2 * 7 * 24 * 60 * 60, // target_timespan_seconds
        10 * 60,              // target_spacing_seconds
        4,                    // retargeting_factor
    );

    let active_genesis_nodes = genesis_node_info
        .iter()
        .filter(|(_, _, balance, _, _, _, _, _)| *balance > 0)
        .collect::<Vec<_>>();

    assert!((active_genesis_nodes.len() == 4) | (genesis_spec as u8 == GenesisSpec::Dev as u8));

    GenesisConfig {
        consensus: Some(ConsensusConfig {
            code,
            authorities: active_genesis_nodes
                .iter()
                .map(|(_, authority_id, _, _, _, _, _, _)| authority_id.clone().into())
                .collect(),
        }),
        system: None,
        timestamp: Some(TimestampConfig {
            minimum_period: CONSENSUS_TIME, // 2 second block time.
        }),
        xsession: Some(SessionConfig {
            validators: active_genesis_nodes
                .iter()
                .map(|(account, _, balance, _, _, _, _, _)| (account.clone().into(), *balance))
                .collect(),
            session_length: blocks_per_session,
            keys: genesis_node_info
                .iter()
                .map(|(account, authority_id, _, _, _, _, _, _)| {
                    (account.clone().into(), authority_id.clone().into())
                })
                .collect(),
        }),
        // chainx runtime module
        xsystem: Some(XSystemConfig {
            network_props,
            _genesis_phantom_data: Default::default(),
        }),
        xfee_manager: Some(XFeeManagerConfig {
            producer_fee_proportion: (1, 10),
            transaction_base_fee: 10000,
            transaction_byte_fee: 100,
        }),

        xassets: Some(XAssetsConfig {
            memo_len: 128,
            _genesis_phantom_data: Default::default(),
        }),
        // bugfix: due to naming error in XAssetsProcess `decl_storage`, thus affect the genesis data.
        // we move token_black_list init into xbootstrap module, and use `mainnet` flag to mark
        // current network state(mainnet/testnet). if current state is mainnet, use old key to init it.
        // if current state is testnet, use new key to init it.
        // xprocess: Some(XAssetsProcessConfig {
        //     token_black_list: vec![sdot_asset.token()],
        //     _genesis_phantom_data: Default::default(),
        // }),
        xstaking: Some(XStakingConfig {
            initial_reward: ((50 as f64) * 10_u64.pow(PCX_PRECISION as u32) as f64) as u64,
            validator_count: 100,
            minimum_validator_count: 4,
            sessions_per_era,
            sessions_per_epoch,
            bonding_duration,
            intention_bonding_duration,
            current_era: 0,
            minimum_penalty: 1_000_000, // 0.01 PCX by default
            missed_blocks_severity: 3,
            maximum_intention_count: 1000,
        }),
        xtokens: Some(XTokensConfig {
            token_discount: vec![
                (xbitcoin::Module::<Runtime>::TOKEN.to_vec(), 50),
                (sdot_asset.token(), 10),
            ],
            _genesis_phantom_data: Default::default(),
        }),
        xspot: Some(XSpotConfig {
            price_volatility: 10,
            _genesis_phantom_data: Default::default(),
        }),
        xbitcoin: Some(XBridgeOfBTCConfig {
            // start genesis block: (genesis, blocknumber)
            genesis: bitcoin.0,
            genesis_hash: bitcoin.1,
            params_info, // retargeting_factor
            network_id: bitcoin.2,
            confirmation_number: 4,
            reserved_block: 2100,
            btc_withdrawal_fee: 100000,
            max_withdrawal_count: 100,
            _genesis_phantom_data: Default::default(),
        }),
        xsdot: Some(XBridgeOfSDOTConfig {
            claims: sdot_claims,
        }),
        xbridge_features: Some(XBridgeFeaturesConfig {
            trustee_info_config: vec![(
                Chain::Bitcoin,
                TrusteeInfoConfig {
                    min_trustee_count: 3,
                    max_trustee_count: 15,
                },
            )],
            _genesis_phantom_data: Default::default(),
        }),
        xcontracts: contracts_config,
        xbootstrap: Some(XBootstrapConfig {
            // xassets
            pcx: (
                b"Polkadot ChainX".to_vec(),
                PCX_PRECISION,
                b"ChainX's crypto currency in Polkadot ecology".to_vec(),
            ),
            // asset, is_online, is_psedu_intention
            // Vec<(Asset, bool, bool)>;
            asset_list: vec![
                (btc_asset.clone(), true, true),
                (sdot_asset.clone(), true, true),
            ],
            // xstaking
            intentions: genesis_node_info
                .iter()
                .map(|(account_id, authority_id, value, name, url, memo, _, _)| {
                    (
                        account_id.clone(),
                        authority_id.clone(),
                        *value,
                        name.clone(),
                        url.clone(),
                        memo.clone(),
                    )
                })
                .collect(),
            trustee_intentions: genesis_node_info
                .iter()
                .filter(|(_, _, _, _, _, _, hot_entity, cold_entity)| {
                    hot_entity.is_some() && cold_entity.is_some()
                })
                .map(|(account_id, _, _, _, _, _, hot_entity, cold_entity)| {
                    (
                        account_id.clone().into(),
                        hot_entity.clone().unwrap().into(),
                        cold_entity.clone().unwrap().into(),
                    )
                })
                .collect(),
            // xtokens
            endowed_users: vec![(btc_asset.token(), vec![]), (sdot_asset.token(), vec![])],
            // xspot
            pair_list: vec![
                (
                    xassets::Module::<Runtime>::TOKEN.to_vec(),
                    xbitcoin::Module::<Runtime>::TOKEN.to_vec(),
                    9,
                    2,
                    100000,
                    true,
                ),
                (
                    sdot_asset.token(),
                    xassets::Module::<Runtime>::TOKEN.to_vec(),
                    4,
                    2,
                    100000,
                    true,
                ),
            ],
            // xgrandpa
            authorities: active_genesis_nodes
                .iter()
                .map(|(_, authority_id, balance, _, _, _, _, _)| {
                    (authority_id.clone().into(), *balance)
                })
                .collect(),
            // xmultisig (include trustees)
            multisig_init_info: (team_account, council_account),
            chain_spec: genesis_spec.into(),
        }),
    }
}

#[derive(Debug, Deserialize)]
pub struct RecordOfSDOT {
    tx_hash: String,
    block_number: u64,
    unix_timestamp: u64,
    date_time: String,
    from: String,
    to: String,
    quantity: f64,
}

fn load_sdot_info() -> Result<Vec<([u8; 20], u64)>, Box<dyn std::error::Error>> {
    let mut reader = csv::Reader::from_reader(&include_bytes!("dot_tx.csv")[..]);
    let mut res = Vec::with_capacity(3052);
    for result in reader.deserialize() {
        let record: RecordOfSDOT = result?;
        let sdot_addr = <[u8; 20] as FromHex>::from_hex(&record.to[2..])?;
        res.push((sdot_addr, (record.quantity * 1000.0).round() as u64));
    }
    Ok(res)
}

#[derive(Debug, Deserialize)]
pub struct RecordOfGenesisNode {
    account_id: String,
    session_key: String,
    endowed: f64,
    name: String,
    url: String,
    about: String,
    hot_entity: String,
    cold_entity: String,
}

fn load_genesis_node_info(
    csv: &[u8],
) -> Result<
    Vec<(
        AccountId,
        AuthorityId,
        u64,
        Vec<u8>,
        Vec<u8>,
        Vec<u8>,
        Option<Vec<u8>>,
        Option<Vec<u8>>,
    )>,
    Box<dyn std::error::Error>,
> {
    let mut reader = csv::Reader::from_reader(csv);
    let mut res = Vec::with_capacity(29);
    for result in reader.deserialize() {
        let record: RecordOfGenesisNode = result?;

        let account_id = hex(&record.account_id).unchecked_into();
        let authority_key = hex(&record.session_key).unchecked_into();

        let endowed = (record.endowed * 10_u64.pow(PCX_PRECISION as u32) as f64) as u64;
        let node_name = record.name.into_bytes();
        let node_url = record.url.into_bytes();
        let memo = record.about.into_bytes();
        let get_entity = |entity: String| {
            if entity.is_empty() {
                None
            } else {
                Some(Vec::from_hex(&entity).unwrap())
            }
        };
        let hot_key = get_entity(record.hot_entity);
        let cold_key = get_entity(record.cold_entity);
        res.push((
            account_id,
            authority_key,
            endowed,
            node_name,
            node_url,
            memo,
            hot_key,
            cold_key,
        ));
    }
    Ok(res)
}

#[derive(Debug, Deserialize)]
pub struct RecordOfTeamCouncil {
    account_id: String,
}

fn load_team_council_info(csv: &[u8]) -> Result<Vec<AccountId>, Box<dyn std::error::Error>> {
    let mut reader = csv::Reader::from_reader(csv);
    let mut res = Vec::with_capacity(7);
    for result in reader.deserialize() {
        let record: RecordOfTeamCouncil = result?;
        let account_id = hex(&record.account_id).unchecked_into();
        res.push(account_id);
    }
    Ok(res)
}

#[test]
fn test_quantity_sum() {
    let res = load_sdot_info().unwrap();
    let sum: u64 = res.iter().map(|(_, quantity)| *quantity).sum();
    assert_eq!(sum, 4999466375u64);
}
