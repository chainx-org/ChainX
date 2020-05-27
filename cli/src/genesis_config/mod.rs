// Copyright 2018-2019 Chainpool.

mod bitcoin;
mod chainx;
mod sdot;

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

const PCX_PRECISION: u16 = 8;
const CONSENSUS_TIME: u64 = 1;

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

pub fn genesis(genesis_spec: GenesisSpec) -> GenesisConfig {
    let (code, mut genesis_node_info, team_and_council, network_props, bitcoin) = match genesis_spec {
        GenesisSpec::Dev => (
            include_bytes!("../../../runtime/wasm/target/wasm32-unknown-unknown/release/chainx_runtime.compact.wasm").to_vec(), // dev genesis runtime version is 6
            chainx::load_genesis_node(&include_bytes!("res/dev_genesis_node.csv")[..]).unwrap(),
            chainx::load_team_council(&include_bytes!("res/dev_team_council.csv")[..]).unwrap(),
            (xsystem::NetworkType::Testnet, 42),
            bitcoin::testnet_mohism(), // dev use a newer bitcoin header
        ),
        GenesisSpec::Testnet => (
            include_bytes!("res/wasm/chainx_runtime.compact.wasm").to_vec(), // testnet genesis runtime version is 6
            chainx::load_genesis_node(&include_bytes!("res/testnet_genesis_node.csv")[..]).unwrap(),
            chainx::load_team_council(&include_bytes!("res/testnet_team_council.csv")[..]).unwrap(),
            (xsystem::NetworkType::Testnet, 42),
            bitcoin::testnet_taoism(),
        ),
        GenesisSpec::TestnetMohism => (
            include_bytes!("res/wasm/testnet_mohism_chainx_runtime.compact.wasm").to_vec(), // testnet genesis runtime version is 6
            chainx::load_genesis_node(&include_bytes!("res/testnet_genesis_node.csv")[..]).unwrap(),
            chainx::load_team_council(&include_bytes!("res/testnet_team_council.csv")[..]).unwrap(),
            (xsystem::NetworkType::Testnet, 42),
            bitcoin::testnet_mohism(),
        ),
        GenesisSpec::Mainnet => (
            include_bytes!("res/wasm/mainnet_chainx_runtime.compact.wasm").to_vec(), // mainnet genesis runtime version is 0
            chainx::load_genesis_node(&include_bytes!("res/mainnet_genesis_node.csv")[..]).unwrap(),
            chainx::load_team_council(&include_bytes!("res/mainnet_team_council.csv")[..]).unwrap(),
            (xsystem::NetworkType::Mainnet, 44),
            bitcoin::mainnet(),
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

    assert_eq!(team_and_council.len(), 8);

    let team_account = team_and_council[..3].to_vec();
    let council_account = team_and_council[3..8].to_vec();

    let initial_authorities_len = match genesis_spec {
        GenesisSpec::Dev => 1,
        GenesisSpec::Testnet => genesis_node_info.len(),
        GenesisSpec::TestnetMohism => genesis_node_info.len(),
        GenesisSpec::Mainnet => genesis_node_info.len(),
    };

    genesis_node_info.truncate(initial_authorities_len);

    // Load all sdot address and quantity.
    let sdot_claims = sdot::load_genesis().unwrap();

    let btc_asset = bitcoin::create_asset();
    let sdot_asset = sdot::create_asset();

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
            intentions: chainx::bootstrap_intentions_config(&genesis_node_info),
            trustee_intentions: chainx::bootstrap_trustee_intentions_config(&genesis_node_info),
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
