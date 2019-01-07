// Copyright 2018 chainpool
extern crate base58;
extern crate chain as btc_chain;
//extern crate cxrml_tokenbalances;
extern crate keys;
extern crate primitives as btc_primitives;
extern crate substrate_keyring;
extern crate substrate_primitives;

use self::base58::FromBase58;
use chainx_runtime::GrandpaConfig;
use chainx_runtime::{
    xassets::{Asset, Chain, ChainT},
    xbitcoin, Runtime,
};
use chainx_runtime::{
    BalancesConfig, ConsensusConfig, GenesisConfig, Params, Perbill, Permill, SessionConfig,
    TimestampConfig, XAccountsConfig, XAssetsConfig, XBridgeOfBTCConfig, XFeeManagerConfig,
    XMatchOrderConfig, XPendingOrdersConfig, XStakingConfig, XSystemConfig,
};

use ed25519;
use ed25519::Public;

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

    //    const MILLICENTS: u128 = 1_000_000_000;
    //    const CENTS: u128 = 1_000 * MILLICENTS;	// assume this is worth about a cent.
    //    const DOLLARS: u128 = 100 * CENTS;

    const MILLICENTS: u128 = 1_000_000_000;
    const CENTS: u128 = 1_000 * MILLICENTS; // assume this is worth about a cent.
    const DOLLARS: u128 = 100 * CENTS;

    const SECS_PER_BLOCK: u64 = 3;
    const MINUTES: u64 = 60 / SECS_PER_BLOCK;
    const HOURS: u64 = MINUTES * 60;
    const DAYS: u64 = HOURS * 24;

    let pcx_precision = 3_u16;
    let normalize = |n: u128| n * 10_u128.pow(pcx_precision as u32);
    let balances_config = BalancesConfig {
        transaction_base_fee: 1,
        transaction_byte_fee: 0,
        existential_deposit: 0,
        transfer_fee: 0,
        creation_fee: 0,
        reclaim_rebate: 0,
        balances: vec![(Keyring::Alice.to_raw_public().into(), 1_000_000)],
    };
    //let balances_config_copy = BalancesConfigCopy::create_from_src(&balances_config).src();

    let btc_asset = Asset::new(
        <xbitcoin::Module<Runtime> as ChainT>::TOKEN.to_vec(), // token
        Chain::Bitcoin,
        8, // bitcoin precision
        b"BTC chainx".to_vec(),
    )
    .unwrap();

    GenesisConfig {
        consensus: Some(ConsensusConfig {
            code: include_bytes!(
            "../../runtime/wasm/target/wasm32-unknown-unknown/release/chainx_runtime_wasm.compact.wasm"
            ).to_vec(),
            authorities: initial_authorities.clone(),
        }),
        system: None,
        balances: Some(balances_config),
        timestamp: Some(TimestampConfig {
            period: SECS_PER_BLOCK, // 3 second block time.
        }),
        session: Some(SessionConfig {
            validators: initial_authorities
                .iter()
                .cloned()
                .map(Into::into)
                .collect(),
            session_length: 30, // 30 blocks per session
        }),
        grandpa: Some(GrandpaConfig {
            authorities: initial_authorities
                .clone()
                .into_iter()
                .map(|k| (k, 1))
                .collect(),
        }),
        // chainx runtime module
        xsystem: Some(XSystemConfig {
            death_account: substrate_primitives::H256::zero(),
            burn_account: substrate_primitives::H256::repeat_byte(0x1),
            banned_account: auth1.into(),
        }),
        xaccounts: None,
        fee_manager: Some(XFeeManagerConfig {
            switch: false,
            _genesis_phantom_data: Default::default(),
        }),
        xassets: Some(XAssetsConfig {
            pcx: (pcx_precision, b"PCX onchain token".to_vec()),
            memo_len: 128,
            // asset, is_psedu_intention, init for account
            // Vec<(Asset, bool, Vec<(T::AccountId, u64)>)>;
            asset_list: vec![
                (btc_asset, true, vec![])
            ],
        }),
        xstaking: Some(XStakingConfig {
            validator_count: 7,
            minimum_validator_count: 1,
            sessions_per_era: 10,
            bonding_duration: 10,
            current_era: 0,
            current_offline_slash: 100,
            offline_slash_grace: 0,
            offline_slash: Perbill::from_millionths(0),
            current_session_reward: 100,
            intentions: initial_authorities
                .clone()
                .into_iter()
                .map(|i| i.0.into())
                .collect(),
        }),
        xpendingorders: Some(XPendingOrdersConfig {
            order_fee: 10,
            pair_list: vec![],
            // (OrderPair { first: Runtime::CHAINX_SYMBOL.to_vec(), second: BridgeOfBTC::SYMBOL.to_vec() }, 8)
            max_command_id: 0,
            average_price_len: 10000,
        }),
        xmatchorder: Some(XMatchOrderConfig {
            match_fee: 10,
            fee_precision: 100000,
            maker_match_fee: 50,
            taker_match_fee: 100,
        }),
        xbitcoin: Some(XBridgeOfBTCConfig {
            // start genesis block: (genesis, blocknumber)
            genesis: (BlockHeader {
                version: 536870912,
                previous_header_hash: H256::from_reversed_str("0000000000169686808d64b2c2bb83b1024375f5af10c77bd90ea58db63ec786"),
                merkle_root_hash: H256::from_reversed_str("7b0d2a0d34c92a0b79ece325478260d75d6c51fe07e606ded0945490f9ecc8de"),
                time: 1543471789,
                bits: Compact::new(436289080),
                nonce: 1307552987,
            }, 1445850),
            params_info: Params::new(520159231, // max_bits
                                     2 * 60 * 60,  // block_max_future
                                     64,  // max_fork_route_preset
                                     2 * 7 * 24 * 60 * 60,  // target_timespan_seconds
                                     10 * 60,  // target_spacing_seconds
                                     4), // retargeting_factor
            network_id: 1,
            utxo_len: 0,
            irr_block: 0,
            btc_fee: 1000,
            cert_address: keys::Address::from_layout(&"2N6JXYKYLqN4e2A96FLnY5J1Mjj5MHXhp6b".from_base58().unwrap()).unwrap(),
            cert_redeem_script: b"522102e34d10113f2dd162e8d8614a4afbb8e2eb14eddf4036042b35d12cf5529056a2210311252930af8ba766b9c7a6580d8dc4bbf9b0befd17a8ef7fabac275bba77ae402103ece1a20b5468b12fd7beda3e62ef6b2f6ad9774489e9aff1c8bc684d87d7078053ae".to_vec(),
            trustee_address: keys::Address::from_layout(&"2N8fUxnFttG5UgPUQDDKXmyRJbr5ZkV4kx3".from_base58().unwrap()).unwrap(),
            trustee_redeem_script: b"52210227e54b65612152485a812b8856e92f41f64788858466cc4d8df674939a5538c321020699bf931859cafdacd8ac4d3e055eae7551427487e281e3efba618bdd395f2f2102a83c80e371ddf0a29006096765d060190bb607ec015ba6023b40ace582e13b9953ae".to_vec(),
            fee: 0,
        }),
    }
}
