// Copyright 2018 chainpool
extern crate base58;
extern crate chain as btc_chain;
//extern crate cxrml_tokenbalances;
extern crate keys;
extern crate primitives as btc_primitives;
extern crate substrate_primitives;

use self::base58::FromBase58;
use chainx_runtime::GrandpaConfig;
use chainx_runtime::{GenesisConfig, ConsensusConfig, CouncilVotingConfig, DemocracyConfig,
                     SessionConfig, StakingConfig, TimestampConfig, BalancesConfig, TreasuryConfig,
                     ContractConfig, Permill, Perbill, XFeeManagerConfig, /*TokenBalancesConfig, FinancialRecordsConfig,
                     MultiSigConfig, BalancesConfigCopy, BridgeOfBTCConfig, Params, Token, PendingOrdersConfig, MatchOrderConfig*/};
use ed25519;
use ed25519::Public;

use self::btc_chain::BlockHeader;
use self::btc_primitives::{compact::Compact, hash::H256};
//use self::cxrml_tokenbalances::{TokenT, Trait};
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
    const CENTS: u128 = 1_000 * MILLICENTS;	// assume this is worth about a cent.
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
            "../../runtime/wasm/target/wasm32-unknown-unknown/release/chainx_runtime.compact.wasm"
            ).to_vec(),
            authorities: initial_authorities.clone(),
        }),
        system: None,
        fee_manager: Some(XFeeManagerConfig { switch: false, _genesis_phantom_data: Default::default(), }), 
        balances: Some(balances_config),
        session: Some(SessionConfig {
            validators: initial_authorities
                .iter()
                .cloned()
                .map(Into::into)
                .collect(),
            session_length: 1 * MINUTES, // that's 1 hour per session.
        }),
        staking: Some(StakingConfig {
            current_era: 0,
            intentions: initial_authorities.iter().cloned().map(Into::into).collect(),
            offline_slash: Perbill::from_billionths(1_000_000),
            session_reward: Perbill::from_billionths(2_065),
            current_offline_slash: 0,
            current_session_reward: 0,
            validator_count: 7,
            sessions_per_era: 12,
            bonding_duration: 1 * DAYS,
            offline_slash_grace: 4,
            minimum_validator_count: 4,
        }),
/*
        staking: Some(StakingConfig {
            current_era: 0,
            bonding_duration: 3 * MINUTES, // 3 days per bond.
            intentions: initial_authorities.clone().into_iter().map(|i| i.0.into()).collect(),
            intention_profiles: initial_authorities.clone().into_iter().map(|i| (i.0.into(), b"ChainX".to_vec(), b"chainx.org".to_vec())).collect(),
            minimum_validator_count: 1,
            validator_count: 6,
            reward_per_sec: 3, // 3 PCX per second
            sessions_per_era: 4, // 24 hours per era.
            session_reward: Perbill::from_millionths(10800),
            offline_slash_grace: 0,
            offline_slash: Perbill::from_millionths(0),
            current_offline_slash: 0,
            current_session_reward: 0,
        }),*/
        democracy: Some(DemocracyConfig {
            launch_period: 5 * MINUTES,    // 1 day per public referendum
            voting_period: 5 * MINUTES,    // 3 days to discuss & vote on an active referendum
            minimum_deposit: 50 * DOLLARS,    // 12000 as the minimum deposit for a referendum
            public_delay: 0,
            max_lock_periods: 6,
        }),
        council_voting: Some(CouncilVotingConfig {
            cooloff_period: 4 * DAYS,
            voting_period: 1 * DAYS,
            enact_delay_period: 0,
        }),
        timestamp: Some(TimestampConfig {
            period: SECS_PER_BLOCK,                  // 3 second block time.
        }),
        treasury: Some(TreasuryConfig {
            proposal_bond: Permill::from_percent(5),
            proposal_bond_minimum: 1_000_000,
            spend_period: 1 * DAYS,
            burn: Permill::from_percent(50),
        }),
        contract: Some(ContractConfig {
            contract_fee: 21,
            call_base_fee: 135,
            create_base_fee: 175,
            gas_price: 1,
            max_depth: 1024,
            block_gas_limit: 10_000_000,
            current_schedule: Default::default(),
        }),
/*        cxsystem: Some(CXSystemConfig {
            death_account: substrate_primitives::H256([0; 32]),
            fee_buy_account: substrate_primitives::H256([1; 32]),
        }),
        tokenbalances: Some(TokenBalancesConfig {
            chainx_precision: pcx_precision,
            // token_list: Vec<(Token, Vec<(T::AccountId, T::TokenBalance)>)>
            // e.g. [("btc", [(account1, value), (account2, value)].to_vec()), ("eth", [(account1, value), (account2, value)].to_vec())]
            token_list: vec![
                (Token::new(BridgeOfBTC::SYMBOL.to_vec(), b"BTC Token".to_vec(), 8),
                // [(Keyring::Alice.to_raw_public().into(), 1_000_000), (Keyring::Bob.to_raw_public().into(), 1_000_000)].to_vec())
                vec![])
            ],

            transfer_token_fee: 10,
        }),
        financialrecords: Some(FinancialRecordsConfig {
            withdrawal_fee: 10,
        }),
        multisig: Some(MultiSigConfig {
            genesis_multi_sig: vec![],
            deploy_fee: 0,
            exec_fee: 0,
            confirm_fee: 0,
            balances_config: balances_config_copy,
            _genesis_phantom_data: Default::default(),
        }),
        bridge_btc: Some(BridgeOfBTCConfig {
            // start genesis block: (genesis, blocknumber)
            genesis: (BlockHeader {
                version: 536870912,
                previous_header_hash: H256::from_reversed_str("000000000000012651bf407efcc567df3529049085711572eaee8d243ec815d4"),
                merkle_root_hash: H256::from_reversed_str("ecec3d2eb31c04a844dc18b233c819c64b6a56c2a51bc77078ef4cc8f434bc21"),
                time: 1541642229,
                bits: Compact::new(436299432),
                nonce: 937513642,
            }, 1442480),
            params_info: Params::new(520159231, // max_bits
                                     2 * 60 * 60,  // block_max_future
                                     64,  // max_fork_route_preset
                                     2 * 7 * 24 * 60 * 60,  // target_timespan_seconds
                                     10 * 60,  // target_spacing_seconds
                                     4), // retargeting_factor
            network_id: 1,
            utxo_max_index: 0,
            irr_block: 0,
            btc_fee: 10,
            accounts_max_index: 0,
            receive_address: keys::Address::from_layout(&"2N4C127fBSmqBsNuHeLmAbZEVSPfV6GB2j2".from_base58().unwrap()).unwrap(),
            redeem_script: b"52210257aff1270e3163aaae9d972b3d09a2385e0d4877501dbeca3ee045f8de00d21c2103fd58c689594b87bbe20a9a00091d074dc0d9f49a988a7ad4c2575adeda1b507c2102bb2a5aa53ba7c0d77bdd86bb9553f77dd0971d3a6bb6ad609787aa76eb17b6b653ae".to_vec(),
            fee: 0,
        }),
        pendingorders: Some(PendingOrdersConfig {
            order_fee: 0,
            pair_list: vec![
                OrderPair::new(b"pcx".to_vec(), b"btc".to_vec(), 8)],
            max_command_id: 0,
            _genesis_phantom_data: Default::default(),
        }),
        matchorder: Some(MatchOrderConfig { match_fee: 10, _genesis_phantom_data: Default::default(),}),
        */
        grandpa: Some(GrandpaConfig {
            authorities: initial_authorities.clone().into_iter().map(|k| (k, 1)).collect(),
        })
    }
}
