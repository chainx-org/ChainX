// Copyright 2018 chainpool
extern crate chain as btc_chain;
//extern crate cxrml_tokenbalances;
extern crate primitives as btc_primitives;
extern crate rustc_hex;
extern crate substrate_keyring;
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
    XBridgeOfXDOTConfig, XFeeManagerConfig, XSpotConfig, XStakingConfig, XSystemConfig,
    XTokensConfig,
};

use ed25519::{self, Public};
use sr_primitives::Permill;

use self::btc_chain::BlockHeader;
use self::btc_primitives::{compact::Compact, hash::H256};
use self::substrate_keyring::Keyring;

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
        GenesisSpec::Local => vec![auth1, auth2, auth3],
        GenesisSpec::Multi => vec![auth1, auth2, auth3, auth4, gavin.into(), dave.into()],
    };

    const CONSENSUS_TIME: u64 = 1;

    let pcx_precision = 8_u16;
    let balances_config = BalancesConfig {
        transaction_base_fee: 1,
        transaction_byte_fee: 0,
        existential_deposit: 0,
        transfer_fee: 0,
        creation_fee: 0,
        balances: vec![],
        vesting: vec![],
    };

    let btc_asset = Asset::new(
        <bitcoin::Module<Runtime> as ChainT>::TOKEN.to_vec(), // token
        b"Bitcoin".to_vec(),
        Chain::Bitcoin,
        8, // bitcoin precision
        b"BTC ChainX".to_vec(),
    )
    .unwrap();

    // let dot_asset = Asset::new(
    //     b"DOT".to_vec(), // token
    //     b"Polkadot".to_vec(),
    //     Chain::Polkadot,
    //     8, //  precision
    //     b"DOT ChainX".to_vec(),
    // )
    // .unwrap();

    let xdot_asset = Asset::new(
        b"XDOT".to_vec(), // token
        b"XDOT".to_vec(),
        Chain::Ethereum,
        3, //  precision
        b"XDOT ChainX".to_vec(),
    )
    .unwrap();

    let apply_prec = |x| (x * 10_u64.pow(pcx_precision as u32) as f64) as u64;

    let mut full_endowed = vec![
        (
            auth1,                                                                          // auth
            apply_prec(12.5),      // balance
            b"Alice".to_vec(),     // name
            b"Alice.com".to_vec(), // url
            b"03f72c448a0e59f48d4adef86cba7b278214cece8e56ef32ba1d179e0a8129bdba".to_vec(), // hot_entity
            b"02a79800dfed17ad4c78c52797aa3449925692bc8c83de469421080f42d27790ee".to_vec(), // cold_entity
        ),
        (
            auth2,
            apply_prec(12.5),
            b"Bob".to_vec(),
            b"Bob.com".to_vec(),
            b"0306117a360e5dbe10e1938a047949c25a86c0b0e08a0a7c1e611b97de6b2917dd".to_vec(),
            b"03ece1a20b5468b12fd7beda3e62ef6b2f6ad9774489e9aff1c8bc684d87d70780".to_vec(),
        ),
        (
            auth3,
            apply_prec(12.5),
            b"Charlie".to_vec(),
            b"Charlie.com".to_vec(),
            b"0311252930af8ba766b9c7a6580d8dc4bbf9b0befd17a8ef7fabac275bba77ae40".to_vec(),
            b"02e34d10113f2dd162e8d8614a4afbb8e2eb14eddf4036042b35d12cf5529056a2".to_vec(),
        ),
        (
            auth4,
            apply_prec(12.5),
            b"Satoshi".to_vec(),
            b"Satoshi.com".to_vec(),
            b"0227e54b65612152485a812b8856e92f41f64788858466cc4d8df674939a5538c3".to_vec(),
            b"020699bf931859cafdacd8ac4d3e055eae7551427487e281e3efba618bdd395f2f".to_vec(),
        ),
    ];

    full_endowed.truncate(initial_authorities.len());

    let endowed = full_endowed
        .clone()
        .into_iter()
        .map(|(auth, balance, _, _, _, _)| (auth, balance))
        .collect::<Vec<_>>();

    GenesisConfig {
        consensus: Some(ConsensusConfig {
            code: include_bytes!(
            "../../runtime/wasm/target/wasm32-unknown-unknown/release/chainx_runtime_wasm.compact.wasm"
            ).to_vec(),
            authorities: initial_authorities.clone(),
        }),
        system: None,
        indices: Some(IndicesConfig {
            ids: initial_authorities.clone().into_iter().map(|x| x.0.into()).collect(),
        }),
        balances: Some(balances_config),
        timestamp: Some(TimestampConfig {
            period: CONSENSUS_TIME, // 2 second block time.
        }),
        session: Some(SessionConfig {
            validators: endowed.iter().cloned().map(|(account, balance)| (account.into(), balance)).collect(),
            session_length: 150, // 150 blocks per session
        }),
        sudo: Some(SudoConfig {
            key: auth1.into(),
        }),
        grandpa: Some(GrandpaConfig {
            authorities: endowed.clone(),
        }),
        // chainx runtime module
        xsystem: Some(XSystemConfig {
            death_account: substrate_primitives::H256::zero(),
            burn_account: substrate_primitives::H256::repeat_byte(0x1),
            banned_account: auth1.into(),
        }),
        fee_manager: Some(XFeeManagerConfig {
            switch: false,
            producer_fee_proportion: (1, 10),
            _genesis_phantom_data: Default::default(),
        }),
        xassets: Some(XAssetsConfig {
            pcx: (b"PolkadotChainX".to_vec(), pcx_precision, b"PCX onchain token".to_vec()),
            memo_len: 128,
            // asset, is_psedu_intention, init for account
            // Vec<(Asset, bool, Vec<(T::AccountId, u64)>)>;
            asset_list: vec![
                (btc_asset.clone(), true, vec![(Keyring::Alice.to_raw_public().into(), 100_000),(Keyring::Bob.to_raw_public().into(), 100_000)]),
                // (dot_asset.clone(), false, vec![(Keyring::Alice.to_raw_public().into(), 1_000_000_000),(Keyring::Bob.to_raw_public().into(), 1_000_000_000)]),
                (xdot_asset.clone(), true, vec![(Keyring::Alice.to_raw_public().into(), 10_000),(Keyring::Bob.to_raw_public().into(), 10_000)])
            ],
        }),
        xprocess: Some(XAssetsProcessConfig {
            token_black_list: vec![xdot_asset.token()],
            _genesis_phantom_data: Default::default(),
        }),
        xstaking: Some(XStakingConfig {
            initial_reward: apply_prec(50.0),
            validator_count: 100,
            minimum_validator_count: 4,
            sessions_per_era: 12,  // update validators set per 12 sessions
            sessions_per_epoch: 12 * 10, // update trustees set per 120 sessions
            bonding_duration: 150 * 12, // 150 blocks per bonding
            intention_bonding_duration: 150 * 12 * 10,
            current_era: 0,
            penalty: 0,
            funding: Default::default(),
            intentions: full_endowed.clone().into_iter().map(|(who, value, name, url, _, _)| (who.into(), value, name, url)).collect(),
            validator_stake_threshold: 1,
            trustee_intentions: full_endowed.into_iter().map(|(who, _, _, _, hot_entity, cold_entity)| (who.into(), hot_entity, cold_entity)).collect(),
            team_address: Public::from_ss58check("5CSff76SK7qcWYq5MpvoHDVRrjWFwpxurwUu6Bqw25hKPQiy").unwrap().0.into(),
        }),
        xtokens: Some(XTokensConfig {
            token_discount: Permill::from_percent(30),
            endowed_users: vec![
                (btc_asset.token(), vec![(Keyring::Alice.to_raw_public().into(), 100_000),(Keyring::Bob.to_raw_public().into(), 100_000)]),
                (xdot_asset.token(), vec![(Keyring::Alice.to_raw_public().into(), 10_000),(Keyring::Bob.to_raw_public().into(), 10_000)])
            ],
        }),
        xspot: Some(XSpotConfig {
            pair_list: vec![
                    (xassets::Module::<Runtime>::TOKEN.to_vec(), bitcoin::Module::<Runtime>::TOKEN.to_vec(), 9, 2, 100000, true),
                 // (<xassets::Module<Runtime> as ChainT>::TOKEN.to_vec(),dot_asset.token().to_vec(),7,2,100000,false),
                    (xdot_asset.token(), xassets::Module::<Runtime>::TOKEN.to_vec(), 4, 2, 100000, true)
                ],
            price_volatility: 10,
        }),
        xdot: Some(XBridgeOfXDOTConfig {
            claims: vec![(eth_address, 1_000_000),],
        }),
        bitcoin: Some(XBridgeOfBTCConfig {
            // start genesis block: (genesis, blocknumber)
            genesis: (BlockHeader {
                version: 536870912,
                previous_header_hash: H256::from_reversed_str("00000000f1c80c38f9bd6ebf9ca796d92122e5b2a1539ac06e09252a1a7e3d01"),
                merkle_root_hash: H256::from_reversed_str("815ca8bbed88af8afaa6c4995acba6e6e7453e705e0bc7039472aa3b6191a707"),
                time: 1546999089,
                bits: Compact::new(436290411),
                nonce: 562223693,
            }, 1451572),
            params_info: Params::new(520159231, // max_bits
                                     2 * 60 * 60,  // block_max_future
                                     3,  // max_fork_route_preset
                                     2 * 7 * 24 * 60 * 60,  // target_timespan_seconds
                                     10 * 60,  // target_spacing_seconds
                                     4), // retargeting_factor
            network_id: 1,
            irr_block: 3,
            reserved: 2100,
            btc_fee: 1000,
            max_withdraw_amount: 100,
            _genesis_phantom_data: Default::default(),
        }),
    }
}
