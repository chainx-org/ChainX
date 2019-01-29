// Copyright 2018 chainpool
extern crate base58;
extern crate chain as btc_chain;
//extern crate cxrml_tokenbalances;
extern crate keys;
extern crate primitives as btc_primitives;
extern crate substrate_keyring;
extern crate substrate_primitives;

use self::base58::FromBase58;
use chainx_runtime::xassets;
use chainx_runtime::GrandpaConfig;

use chainx_runtime::{
    bitcoin,
    xassets::{Asset, Chain, ChainT},
    Runtime,
};
use chainx_runtime::{
    BalancesConfig, ConsensusConfig, GenesisConfig, IndicesConfig, Params, SessionConfig,
    SudoConfig, TimestampConfig, XAssetsConfig, XBridgeOfBTCConfig, XFeeManagerConfig, XSpotConfig,
    XStakingConfig, XSystemConfig, XTokensConfig,
};

use ed25519;
use sr_primitives::Permill;

use self::btc_chain::BlockHeader;
use self::btc_primitives::{compact::Compact, hash::H256};
use self::keys::DisplayLayout;
use self::substrate_keyring::Keyring;

pub enum GenesisSpec {
    Dev,
    Local,
    Multi,
}

pub fn testnet_genesis(genesis_spec: GenesisSpec) -> GenesisConfig {
    let alice = ed25519::Pair::from_seed(b"Alice                           ").public();
    let bob = ed25519::Pair::from_seed(b"Bob                             ").public();
    let charlie = ed25519::Pair::from_seed(b"Charlie                         ").public();
    let dave = ed25519::Pair::from_seed(b"Dave                            ").public();
    let gavin = ed25519::Pair::from_seed(b"Gavin                           ").public();
    let satoshi = ed25519::Pair::from_seed(b"Satoshi                         ").public();

    let auth1 = alice.into();
    let auth2 = bob.into();
    let auth3 = gavin.into();
    let auth4 = satoshi.into();
    let initial_authorities = match genesis_spec {
        GenesisSpec::Dev => vec![auth1],
        GenesisSpec::Local => vec![auth1, auth2],
        GenesisSpec::Multi => vec![auth1, auth2, auth3, auth4, charlie.into(), dave.into()],
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
    };
    //let balances_config_copy = BalancesConfigCopy::create_from_src(&balances_config).src();

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

    let endowed = initial_authorities
        .clone()
        .into_iter()
        .map(|x| (x, 12500))
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
            session_length: 30, // 30 blocks per session
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
                (btc_asset, true, vec![(Keyring::Alice.to_raw_public().into(), 100),(Keyring::Bob.to_raw_public().into(), 100)]),
                // (dot_asset.clone(), false, vec![(Keyring::Alice.to_raw_public().into(), 1_000_000_000),(Keyring::Bob.to_raw_public().into(), 1_000_000_000)]),
                (xdot_asset.clone(), true, vec![(Keyring::Alice.to_raw_public().into(), 100),(Keyring::Bob.to_raw_public().into(), 100)])
            ],
        }),
        xstaking: Some(XStakingConfig {
            validator_count: 7,
            minimum_validator_count: 1,
            sessions_per_era: 1,
            bonding_duration: 30,
            current_era: 0,
            penalty: 100,
            funding: Default::default(),
            intentions: endowed.iter().cloned().map(|(account, balance)| (account.into(), balance)).collect(),
            validator_stake_threshold: 1,
        }),
        xtokens: Some(XTokensConfig {
            token_discount: Permill::from_percent(30),
            _genesis_phantom_data: Default::default(),
        }),
        xspot: Some(XSpotConfig {
            pair_list: vec![(<xassets::Module<Runtime> as ChainT>::TOKEN.to_vec(),<bitcoin::Module<Runtime> as ChainT>::TOKEN.to_vec(),9,2,100000,true),
                 // (<xassets::Module<Runtime> as ChainT>::TOKEN.to_vec(),dot_asset.token().to_vec(),7,2,100000,false),
                 (xdot_asset.token().to_vec(),<xassets::Module<Runtime> as ChainT>::TOKEN.to_vec(),4,2,100000,true)
                ],
            price_volatility: 10,
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
            cert_address: keys::Address::from_layout(&"2N6JXYKYLqN4e2A96FLnY5J1Mjj5MHXhp6b".from_base58().unwrap()).unwrap(),
            cert_redeem_script: b"522102e34d10113f2dd162e8d8614a4afbb8e2eb14eddf4036042b35d12cf5529056a2210311252930af8ba766b9c7a6580d8dc4bbf9b0befd17a8ef7fabac275bba77ae402103ece1a20b5468b12fd7beda3e62ef6b2f6ad9774489e9aff1c8bc684d87d7078053ae".to_vec(),
            trustee_address: keys::Address::from_layout(&"2MtAUgQmdobnz2mu8zRXGSTwUv9csWcNwLU".from_base58().unwrap()).unwrap(),
            trustee_redeem_script: b"52210311252930af8ba766b9c7a6580d8dc4bbf9b0befd17a8ef7fabac275bba77ae402102e34d10113f2dd162e8d8614a4afbb8e2eb14eddf4036042b35d12cf5529056a221023e505c48a955e759ce61145dc4a9a7447425290b8483f4e36f05169e7967c86d53ae".to_vec(),
            _genesis_phantom_data: Default::default(),
        }),
    }
}
