// Copyright 2018 chainpool
extern crate chain as btc_chain;
//extern crate cxrml_tokenbalances;
extern crate primitives as btc_primitives;
extern crate rustc_hex;
extern crate substrate_primitives;

use self::rustc_hex::FromHex;
use chainx_runtime::xassets;
use chainx_runtime::GrandpaConfig;

use chainx_runtime::{
    bitcoin,
    xassets::{Asset, Chain, ChainT},
    Runtime,
};
use chainx_runtime::{
    BalancesConfig, ConsensusConfig, GenesisConfig, IndicesConfig, Params, SessionConfig,
    SudoConfig, TimestampConfig, XAssetsConfig, XAssetsProcessConfig, XBridgeOfBTCConfig,
    XBridgeOfSDOTConfig, XFeeManagerConfig, XSpotConfig, XStakingConfig, XSystemConfig,
    XTokensConfig,
};

use ed25519::{self, Public};

use self::btc_chain::BlockHeader;
use self::btc_primitives::{compact::Compact, hash::H256};

pub enum GenesisSpec {
    Dev,
    Local,
    Multi,
}

pub fn testnet_genesis(genesis_spec: GenesisSpec) -> GenesisConfig {
    let tmp_eth_address = "004927472a848c6015f5eb02defc13272937d2d5"
        .from_hex::<Vec<_>>()
        .unwrap();
    let mut eth_address: [u8; 20] = [0u8; 20];
    eth_address.copy_from_slice(&tmp_eth_address);

    // account pub and pri key
    let alice = ed25519::Pair::from_seed(b"Alice                           ").public();
    let bob = ed25519::Pair::from_seed(b"Bob                             ").public();
    let charlie = ed25519::Pair::from_seed(b"Charlie                         ").public();
    let dave = ed25519::Pair::from_seed(b"Dave                            ").public();
    let gavin = ed25519::Pair::from_seed(b"Gavin                           ").public();
    let satoshi = ed25519::Pair::from_seed(b"Satoshi                         ").public();

    let auth1 = alice.into();
    let auth2 = bob.into();
    let auth3 = charlie.into();
    let auth4 = satoshi.into();
    let initial_authorities = match genesis_spec {
        GenesisSpec::Dev => vec![auth1],
        GenesisSpec::Local => vec![auth1, auth2, auth3, auth4],
        GenesisSpec::Multi => vec![auth1, auth2, auth3, auth4, gavin.into(), dave.into()],
    };

    const CONSENSUS_TIME: u64 = 1;

    let pcx_precision = 8_u16;

    let btc_asset = Asset::new(
        <bitcoin::Module<Runtime> as ChainT>::TOKEN.to_vec(), // token
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

    let apply_prec = |x| (x * 10_u64.pow(pcx_precision as u32) as f64) as u64;

    let mut full_endowed = vec![
        (
            auth1,                 // auth
            apply_prec(12.5),      // balance
            b"Alice".to_vec(),     // name
            b"Alice.com".to_vec(), // url
            "03f72c448a0e59f48d4adef86cba7b278214cece8e56ef32ba1d179e0a8129bdba"
                .from_hex()
                .unwrap(), // hot_entity
            "02a79800dfed17ad4c78c52797aa3449925692bc8c83de469421080f42d27790ee"
                .from_hex()
                .unwrap(), // cold_entity
        ),
        (
            auth2,
            apply_prec(12.5),
            b"Bob".to_vec(),
            b"Bob.com".to_vec(),
            "0306117a360e5dbe10e1938a047949c25a86c0b0e08a0a7c1e611b97de6b2917dd"
                .from_hex()
                .unwrap(),
            "03ece1a20b5468b12fd7beda3e62ef6b2f6ad9774489e9aff1c8bc684d87d70780"
                .from_hex()
                .unwrap(),
        ),
        (
            auth3,
            apply_prec(12.5),
            b"Charlie".to_vec(),
            b"Charlie.com".to_vec(),
            "0311252930af8ba766b9c7a6580d8dc4bbf9b0befd17a8ef7fabac275bba77ae40"
                .from_hex()
                .unwrap(),
            "02e34d10113f2dd162e8d8614a4afbb8e2eb14eddf4036042b35d12cf5529056a2"
                .from_hex()
                .unwrap(),
        ),
        (
            auth4,
            apply_prec(12.5),
            b"Satoshi".to_vec(),
            b"Satoshi.com".to_vec(),
            "0227e54b65612152485a812b8856e92f41f64788858466cc4d8df674939a5538c3"
                .from_hex()
                .unwrap(),
            "020699bf931859cafdacd8ac4d3e055eae7551427487e281e3efba618bdd395f2f"
                .from_hex()
                .unwrap(),
        ),
    ];

    full_endowed.truncate(initial_authorities.len());

    let endowed = full_endowed
        .clone()
        .into_iter()
        .map(|(auth, balance, _, _, _, _)| (auth, balance))
        .collect::<Vec<_>>();

    let blocks_per_session = 150; // 150 blocks per session
    let sessions_per_era = 12; // update validators set per 12 sessions
    let sessions_per_epoch = sessions_per_era * 10; // update trustees set per 12*10 sessions
    let bonding_duration = blocks_per_session * sessions_per_era; // freeze 150*12 blocks for non-intention
    let intention_bonding_duration = bonding_duration * 10; // freeze 150*12*10 blocks for intention

    GenesisConfig {
        consensus: Some(ConsensusConfig {
            code: include_bytes!("./chainx_runtime_wasm.compact.wasm").to_vec(),
            authorities: initial_authorities.clone(),
        }),
        system: None,
        indices: Some(IndicesConfig {
            ids: initial_authorities
                .clone()
                .into_iter()
                .map(|x| x.0.into())
                .collect(),
        }),
        balances: Some(BalancesConfig {
            transaction_base_fee: 10000,
            transaction_byte_fee: 100,
            existential_deposit: 0,
            transfer_fee: 0,
            creation_fee: 0,
            balances: vec![],
            vesting: vec![],
        }),
        timestamp: Some(TimestampConfig {
            period: CONSENSUS_TIME, // 2 second block time.
        }),
        session: Some(SessionConfig {
            validators: endowed
                .iter()
                .cloned()
                .map(|(account, balance)| (account.into(), balance))
                .collect(),
            session_length: blocks_per_session,
        }),
        sudo: Some(SudoConfig { key: auth1.into() }),
        grandpa: Some(GrandpaConfig {
            authorities: endowed.clone(),
        }),
        // chainx runtime module
        xsystem: Some(XSystemConfig {
            death_account: substrate_primitives::H256::zero(),
            burn_account: substrate_primitives::H256::repeat_byte(0x1),
        }),
        fee_manager: Some(XFeeManagerConfig {
            producer_fee_proportion: (1, 10),
            _genesis_phantom_data: Default::default(),
        }),
        xassets: Some(XAssetsConfig {
            pcx: (
                b"Polkadot ChainX".to_vec(),
                pcx_precision,
                b"ChainX's crypto currency in Polkadot ecology".to_vec(),
            ),
            memo_len: 128,
            // asset, is_online, is_psedu_intention, init for account
            // Vec<(Asset, bool, Vec<(T::AccountId, u64)>)>;
            asset_list: vec![
                (btc_asset.clone(), true, true, vec![]),
                (sdot_asset.clone(), true, true, vec![]),
            ],
        }),
        xprocess: Some(XAssetsProcessConfig {
            token_black_list: vec![sdot_asset.token()],
            _genesis_phantom_data: Default::default(),
        }),
        xstaking: Some(XStakingConfig {
            initial_reward: apply_prec(50.0),
            validator_count: 30,
            minimum_validator_count: 4,
            trustee_count: 4,
            minimum_trustee_count: 4,
            sessions_per_era: sessions_per_era,
            sessions_per_epoch: sessions_per_epoch,
            bonding_duration: bonding_duration,
            intention_bonding_duration: intention_bonding_duration,
            current_era: 0,
            penalty: 50 * 100_000_000 / 150, // 1 per block reward
            intentions: full_endowed
                .clone()
                .into_iter()
                .map(|(who, value, name, url, _, _)| (who.into(), value, name, url))
                .collect(),
            validator_stake_threshold: 1,
            trustee_intentions: full_endowed
                .into_iter()
                .map(|(who, _, _, _, hot_entity, cold_entity)| {
                    (who.into(), hot_entity, cold_entity)
                })
                .collect(),
            council_address: Default::default(),
            team_address: Public::from_ss58check(
                "5CSff76SK7qcWYq5MpvoHDVRrjWFwpxurwUu6Bqw25hKPQiy",
            )
            .unwrap()
            .0
            .into(),
        }),
        xtokens: Some(XTokensConfig {
            token_discount: 50,
            endowed_users: vec![(btc_asset.token(), vec![]), (sdot_asset.token(), vec![])],
        }),
        xspot: Some(XSpotConfig {
            pair_list: vec![
                (
                    xassets::Module::<Runtime>::TOKEN.to_vec(),
                    bitcoin::Module::<Runtime>::TOKEN.to_vec(),
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
            price_volatility: 10,
        }),
        sdot: Some(XBridgeOfSDOTConfig {
            claims: vec![(eth_address, 1_000_000)],
        }),
        bitcoin: Some(XBridgeOfBTCConfig {
            // start genesis block: (genesis, blocknumber)
            genesis: (
                BlockHeader {
                    version: 536870912,
                    previous_header_hash: H256::from_reversed_str(
                        "0000000000005a693961608af8c00d25fa71bde2d9e3eae4494c10baaeed4070",
                    ),
                    merkle_root_hash: H256::from_reversed_str(
                        "9e7add48fd35513b37309fed6c0b9e116621de9385548aee5c4bb313476ff30a",
                    ),
                    time: 1550490136,
                    bits: Compact::new(453049348),
                    nonce: 3012999283,
                },
                1474333,
            ),
            params_info: Params::new(
                520159231,            // max_bits
                2 * 60 * 60,          // block_max_future
                2 * 7 * 24 * 60 * 60, // target_timespan_seconds
                10 * 60,              // target_spacing_seconds
                4,
            ), // retargeting_factor
            network_id: 1,
            irr_block: 6,
            reserved: 2100,
            btc_fee: 40000,
            max_withdraw_amount: 10,
            _genesis_phantom_data: Default::default(),
        }),
    }
}
