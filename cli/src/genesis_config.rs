// Copyright 2018-2019 Chainpool.

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
    Local,
    Multi,
}
const PCX_PRECISION: u16 = 8;

pub fn testnet_genesis(genesis_spec: GenesisSpec) -> GenesisConfig {
    // Load all sdot address and quantity.
    let sdot_claims = load_sdot_info().unwrap();
    let mut genesis_node_info = load_genesis_node_info().unwrap();

    let initial_authorities_len = match genesis_spec {
        GenesisSpec::Dev => 1,
        GenesisSpec::Local => 4,
        GenesisSpec::Multi => 4,
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
    let sessions_per_era = 2; // update validators set per 12 sessions
    let sessions_per_epoch = sessions_per_era * 10; // update trustees set per 12*10 sessions
    let bonding_duration = blocks_per_session * sessions_per_era; // freeze 150*12 blocks for non-intention
    let intention_bonding_duration = bonding_duration * 10; // freeze 150*12*10 blocks for intention

    let params_info = Params::new(
        520159231,            // max_bits
        2 * 60 * 60,          // block_max_future
        2 * 7 * 24 * 60 * 60, // target_timespan_seconds
        10 * 60,              // target_spacing_seconds
        4,                    // retargeting_factor
    );

    GenesisConfig {
        consensus: Some(ConsensusConfig {
            code: include_bytes!("./chainx_runtime.compact.wasm").to_vec(),
            authorities: genesis_node_info
                .iter()
                .map(|(_, authority_id, _, _, _, _, _, _)| authority_id.clone().into())
                .collect(),
        }),
        system: None,
        timestamp: Some(TimestampConfig {
            minimum_period: CONSENSUS_TIME, // 2 second block time.
        }),
        xsession: Some(SessionConfig {
            validators: genesis_node_info
                .iter()
                .map(|(_, authority_id, balance, _, _, _, _, _)| {
                    (authority_id.clone().into(), *balance)
                })
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
            network_props: (xsystem::NetworkType::Testnet, 44),
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
                        "0000000000000f3b669061e1437c502eda529057c33a115b63abdb328b5b4645",
                    ),
                    merkle_root_hash: h256_from_rev_str(
                        "b954ca2828475be7f5f772a26369b51e6808d853a0e62219af0dcb9f8f9aa0ad",
                    ),
                    time: 1556000895,
                    bits: Compact::new(437247136),
                    nonce: 472822001,
                },
                1511056,
            ),
            genesis_hash: h256_from_rev_str(
                "0000000000000e6b5c9b88cf3b2b89374841769d075c2698cc80c2eac98cdd54",
            ),
            params_info, // retargeting_factor
            network_id: 1,
            confirmation_number: 6,
            reserved_block: 2100,
            btc_withdrawal_fee: 40000,
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
                    min_trustee_count: 4,
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
                .map(|(account_id, _, _, _, _, _, hot_entity, cold_entity)| {
                    (
                        account_id.clone().into(),
                        hot_entity.clone().into(),
                        cold_entity.clone().into(),
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
            authorities: genesis_node_info
                .iter()
                .map(|(_, authority_id, balance, _, _, _, _, _)| {
                    (authority_id.clone().into(), *balance)
                })
                .collect(),
            // xmultisig (include trustees)
            multisig_init_info: genesis_node_info
                .iter()
                .map(|(account, _, _, _, _, _, _, _)| (account.clone().into(), true))
                .collect(),
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
    use rustc_hex::FromHex;

    let mut reader = csv::Reader::from_reader(&include_bytes!("dot_tx.csv")[..]);
    let mut res = Vec::with_capacity(3052);
    for result in reader.deserialize() {
        let record: RecordOfSDOT = result?;
        let mut sdot_addr = [0u8; 20];
        sdot_addr.copy_from_slice(&record.to[2..].from_hex::<Vec<u8>>()?);
        res.push((sdot_addr, (record.quantity * 1000.0).round() as u64));
    }
    Ok(res)
}

#[derive(Debug, Deserialize)]
pub struct RecordOfGenesisNode {
    account_id: String,
    authority_key: String,
    money: f64,
    node_name: String,
    node_url: String,
    memo: String,
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
        Vec<u8>,
        Vec<u8>,
    )>,
    Box<dyn std::error::Error>,
> {
    use hex::FromHex;

    let mut reader = csv::Reader::from_reader(&include_bytes!("genesis_node.csv")[..]);
    let mut res = Vec::with_capacity(29);
    for result in reader.deserialize() {
        let record: RecordOfGenesisNode = result?;
        let mut account_buffer = [0u8; 32];
        account_buffer.copy_from_slice(&Vec::from_hex(&record.account_id).unwrap());
        let account_id = account_buffer.unchecked_into();
        let mut authority_buffer = [0u8; 32];
        authority_buffer.copy_from_slice(&Vec::from_hex(&record.authority_key).unwrap());
        let authority_key = authority_buffer.unchecked_into();
        let money = (record.money * 10_u64.pow(PCX_PRECISION as u32) as f64) as u64;
        let node_name = Vec::from_hex(hex::encode(&record.node_name)).unwrap();
        let node_url = Vec::from_hex(hex::encode(&record.node_url)).unwrap();
        let memo = Vec::from_hex(hex::encode(&record.memo)).unwrap();
        let hot_key = Vec::from_hex(&record.hot_entity).unwrap();
        let cold_key = Vec::from_hex(&record.cold_entity).unwrap();
        res.push((
            account_id,
            authority_key,
            money,
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
