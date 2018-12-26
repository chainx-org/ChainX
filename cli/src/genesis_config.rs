// Copyright 2018 chainpool
extern crate base58;
extern crate chain as btc_chain;
//extern crate cxrml_tokenbalances;
extern crate keys;
extern crate primitives as btc_primitives;
extern crate substrate_primitives;

use self::base58::FromBase58;
use chainx_runtime::GrandpaConfig;
use chainx_runtime::{
    BalancesConfig, ConsensusConfig, GenesisConfig, Perbill, Permill, SessionConfig,
    TimestampConfig, XAssetsConfig, XFeeManagerConfig, XSystemConfig, XAccountsConfig,XPendingOrdersConfig,XMatchOrderConfig
};
use ed25519;
use ed25519::Public;

use self::btc_chain::BlockHeader;
use self::btc_primitives::{compact::Compact, hash::H256};
use self::keys::DisplayLayout;

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

    const SECS_PER_BLOCK: u64 = 1;
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
        balances: vec![],
    };
    //let balances_config_copy = BalancesConfigCopy::create_from_src(&balances_config).src();

    GenesisConfig {
        consensus: Some(ConsensusConfig {
            code: include_bytes!(
            "../../runtime/wasm/target/wasm32-unknown-unknown/release/chainx_runtime_wasm.compact.wasm"
            )
            .to_vec(),
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
            session_length: 1 * MINUTES, // that's 1 hour per session.
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
        }),
        xaccounts: None,
        fee_manager: Some(XFeeManagerConfig {
            switch: false,
            _genesis_phantom_data: Default::default(),
        }),
        xassets: Some(XAssetsConfig {
            pcx: (pcx_precision, b"PCX onchain token".to_vec()),
            remark_len: 128,
            asset_list: vec![],
        }),
        xpendingorders:Some(XPendingOrdersConfig{
            order_fee: 10,
            pair_list: vec![],
            // (OrderPair { first: Runtime::CHAINX_SYMBOL.to_vec(), second: BridgeOfBTC::SYMBOL.to_vec() }, 8)
            max_command_id: 0,
            average_price_len: 10000,
        }),
        xmatchorder:Some(XMatchOrderConfig{
            match_fee: 10,
            fee_precision: 100000,
            maker_match_fee: 50,
            taker_match_fee: 100,
        })
    }
}
