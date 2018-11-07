// Copyright 2018 chainpool
extern crate primitives as btc_primitives;
extern crate chain as btc_chain;
extern crate base58;

use self::base58::FromBase58;
use chainx_runtime::{GenesisConfig, ConsensusConfig, CouncilVotingConfig, DemocracyConfig,
                     SessionConfig, StakingConfig, TimestampConfig, BalancesConfig, TreasuryConfig,
                     ContractConfig, Permill, Perbill, TokenBalancesConfig, FinancialRecordsConfig,
                     MultiSigConfig, BalancesConfigCopy, BridgeOfBTCConfig, Params, Token};
use super::cli::ChainSpec;
use keyring::Keyring;
use ed25519;

use self::btc_primitives::{hash::H256, compact::Compact};
use self::btc_chain::BlockHeader;

pub fn testnet_genesis(chainspec: ChainSpec) -> GenesisConfig {
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
    let initial_authorities = match chainspec {
        ChainSpec::Dev => vec![auth1],
        ChainSpec::Local => vec![auth1, auth2],
        ChainSpec::Multi => vec![auth1, auth2, auth3, auth4, charlie.into(), dave.into()],
    };


//    const MILLICENTS: u128 = 1_000_000_000;
//    const CENTS: u128 = 1_000 * MILLICENTS;	// assume this is worth about a cent.
//    const DOLLARS: u128 = 100 * CENTS;

    const SECS_PER_BLOCK: u64 = 1;
    const MINUTES: u64 = 60 / SECS_PER_BLOCK;
    const HOURS: u64 = MINUTES * 60;
    const DAYS: u64 = HOURS * 24;

    let balances_config = BalancesConfig {
        transaction_base_fee: 1,
        transaction_byte_fee: 0,
        existential_deposit: 0,
        transfer_fee: 0,
        creation_fee: 0,
        reclaim_rebate: 0,
        balances: vec![
            (Keyring::Alice.to_raw_public().into(), 1_000_000),
            (Keyring::Bob.to_raw_public().into(), 1_000_000),
            (Keyring::Charlie.to_raw_public().into(), 1_000_000),
            (Keyring::Dave.to_raw_public().into(), 1_000_000),
            (Keyring::Ferdie.to_raw_public().into(), 996_000_000),
        ],
    };
    let balances_config_copy = BalancesConfigCopy::create_from_src(&balances_config).src();

    GenesisConfig {
        consensus: Some(ConsensusConfig {
            code: include_bytes!(
            "../runtime/wasm/target/wasm32-unknown-unknown/release/chainx_runtime.compact.wasm"
            ).to_vec(),
            authorities: initial_authorities.clone(),
        }),
        system: None,
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
            bonding_duration: 3 * MINUTES, // 3 days per bond.
            intentions: initial_authorities.clone().into_iter().map(|i| i.0.into()).collect(),
            name_of_intention: initial_authorities.clone().into_iter().map(|i| (i.0.into(), b"ChainX".to_vec())).collect(),
            url_of_intention: initial_authorities.into_iter().map(|i| (i.0.into(), b"chainx.org".to_vec())).collect(),
            minimum_validator_count: 1,
            validator_count: 6,
            candidate_count: 6 * 4,
            reward_per_sec: 3, // 3 PCX per second
            sessions_per_era: 4, // 24 hours per era.
            session_reward: Perbill::from_millionths(10800),
            offline_slash_grace: 0,
            offline_slash: Perbill::from_millionths(0),
            current_offline_slash: 0,
            current_session_reward: 0,
        }),
        democracy: Some(DemocracyConfig {
            launch_period: 120 * 24 * 14, // 2 weeks per public referendum
            voting_period: 120 * 24 * 28, // 4 weeks to discuss & vote on an active referendum
            minimum_deposit: 1000, // 1000 as the minimum deposit for a referendum
        }),
        council_voting: Some(CouncilVotingConfig {
            cooloff_period: 4 * DAYS,
            voting_period: 1 * DAYS,
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
        }),
        tokenbalances: Some(TokenBalancesConfig {
            // token_list: Vec<(Token, Vec<(T::AccountId, T::TokenBalance)>)>
            // e.g. [("btc", [(account1, value), (account2, value)].to_vec()), ("eth", [(account1, value), (account2, value)].to_vec())]
            token_list: vec![(Token::new(b"x-btc".to_vec(), b"btc token".to_vec(), 8), vec![]),],
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
        }),
        bridge_btc: Some(BridgeOfBTCConfig {
            // start genesis block: (genesis, blocknumber)
            genesis: (BlockHeader {
                version: 536870912,
                previous_header_hash: H256::from_reversed_str("000000000000837bcdb53e7a106cf0e74bab6ae8bc96481243d31bea3e6b8c92"),
                merkle_root_hash: H256::from_reversed_str("8beab73ba2318e4cbdb1c65624496bc3214d6ba93204e049fb46293a41880b9a"),
                time: 1506023937,
                bits: Compact::new(453021074),
                nonce: 2001025151,
            }, 1200000),
            params_info: Params::new(520159231, // max_bits
                                     2 * 60 * 60,  // block_max_future
                                     64,  // max_fork_route_preset
                                     2 * 7 * 24 * 60 * 60,  // target_timespan_seconds
                                     10 * 60,  // target_spacing_seconds
                                     4), // retargeting_factor
            network_id: 1,
            utxo_max_index: 0,
            irr_block: 6,
            accounts_max_index: 0,
            receive_address: "mjKE11gjVN4JaC9U8qL6ZB5vuEBgmwik7b".from_base58().unwrap(),
            redeem_script: b"52210257aff1270e3163aaae9d972b3d09a2385e0d4877501dbeca3ee045f8de00d21c2103fd58c689594b87bbe20a9a00091d074dc0d9f49a988a7ad4c2575adeda1b507c2102bb2a5aa53ba7c0d77bdd86bb9553f77dd0971d3a6bb6ad609787aa76eb17b6b653ae".to_vec(),
            fee: 0,
        }),
    }
}
