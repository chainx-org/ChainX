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
    ConsensusConfig, GenesisConfig, SessionConfig, TimestampConfig, XAssetsConfig,
    XAssetsProcessConfig, XBootstrapConfig, XBridgeFeaturesConfig, XBridgeOfBTCConfig,
    XBridgeOfSDOTConfig, XFeeManagerConfig, XSpotConfig, XStakingConfig, XSystemConfig,
    XTokensConfig,
};

use btc_chain::BlockHeader;
use btc_primitives::{h256_from_rev_str, Compact};

pub enum GenesisSpec {
    Dev,
    Testnet,
    Mainnet,
}
const PCX_PRECISION: u16 = 8;

fn hex(account: &str) -> [u8; 32] {
    <[u8; 32] as FromHex>::from_hex(account).unwrap()
}

pub fn genesis(genesis_spec: GenesisSpec) -> GenesisConfig {
    // Load all sdot address and quantity.
    let sdot_claims = load_sdot_info().unwrap();
    let mut genesis_node_info = load_genesis_node_info().unwrap();

    let initial_authorities_len = match genesis_spec {
        GenesisSpec::Dev => 1,
        GenesisSpec::Testnet => genesis_node_info.len(),
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

    let team_account = vec![
        hex("a5b74e024ed2823e5dc4d4e77313c0601393f107c7fa62b9e8ca54930b12d545"),
        hex("bf40736f7157faf64411ef36de9b6dae8133be3edf460a50d9e84cc05829dc21"),
        hex("120bdbc81e1172e17becc965a51dc1bf3e782162eadee54b5d94fec8a0288c83"),
    ];

    let council_account = vec![
        hex("1595e186c3a915cfbd4f601b23a88bbaab873bfefbb09d231483e424633093e7"),
        hex("a4e99224b97dee6798f3fb90b835d63e3f4059f334f09a44e23420ca993e45f0"),
        hex("7ad04497564c5da319794aa8c99375d61878f471124dbc83dcc5a3cd6418af11"),
        hex("b16a5254fff78ab974abd25c64430ae5944e201916d003807226b6e2a0fcd1f1"),
        hex("041b0452b3defb8bdcaab8f4786fa634ae6f841cbe1ee9e1959bd94eaa021f7f"),
    ];

    let blocks_per_session = 150; // 150 blocks per session
    let sessions_per_era = 12; // update validators set per 12 sessions
    let sessions_per_epoch = sessions_per_era * 10; // update trustees set per 12*10 sessions
    let bonding_duration = blocks_per_session * sessions_per_era; // freeze 150*12 blocks for non-intention
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

    assert!(active_genesis_nodes.len() == 4);

    GenesisConfig {
        consensus: Some(ConsensusConfig {
            code: include_bytes!("./chainx_runtime.compact.wasm").to_vec(),
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
            network_props: (xsystem::NetworkType::Mainnet, 44),
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
        xprocess: Some(XAssetsProcessConfig {
            token_black_list: vec![sdot_asset.token()],
            _genesis_phantom_data: Default::default(),
        }),
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
            genesis: (
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
            genesis_hash: h256_from_rev_str(
                "0000000000000000001721f58deb88b0710295a02551f0dde1e2e231a15f1882",
            ),
            params_info, // retargeting_factor
            network_id: 0,
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
            multisig_init_info: (
                team_account
                    .iter()
                    .map(|&account| account.unchecked_into())
                    .collect(),
                council_account
                    .iter()
                    .map(|&account| account.unchecked_into())
                    .collect(),
            ),
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

fn load_genesis_node_info() -> Result<
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
    let mut reader = csv::Reader::from_reader(&include_bytes!("genesis_node.csv")[..]);
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

#[test]
fn test_quantity_sum() {
    let res = load_sdot_info().unwrap();
    let sum: u64 = res.iter().map(|(_, quantity)| *quantity).sum();
    assert_eq!(sum, 4999466375u64);
}
