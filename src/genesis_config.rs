// Copyright 2018 chainpool

use chainx_runtime::{GenesisConfig, ConsensusConfig, CouncilVotingConfig, DemocracyConfig,
                     SessionConfig, StakingConfig, TimestampConfig, BalancesConfig, TreasuryConfig,
                     ContractConfig, Permill, Perbill, TokenBalancesConfig, FinancialRecordsConfig,
                     MultiSigConfig, BalancesConfigCopy};
use super::cli::ChainSpec;
use keyring::Keyring;
use ed25519;


pub fn testnet_genesis(chainspec: ChainSpec) -> GenesisConfig {
    let alice = ed25519::Pair::from_seed(b"Alice                           ").public();
    let bob = ed25519::Pair::from_seed(b"Bob                             ").public();
    let gavin = ed25519::Pair::from_seed(b"Gavin                           ").public();
    let satoshi = ed25519::Pair::from_seed(b"Satoshi                         ").public();

    let auth1 = alice.into();
    let auth2 = bob.into();
    let auth3 = gavin.into();
    let auth4 = satoshi.into();
    let initial_authorities = match chainspec {
        ChainSpec::Dev => vec![auth1],
        ChainSpec::Local => vec![auth1, auth2],
        ChainSpec::Multi => vec![auth1, auth2, auth3, auth4],
    };


//    const MILLICENTS: u128 = 1_000_000_000;
//    const CENTS: u128 = 1_000 * MILLICENTS;	// assume this is worth about a cent.
//    const DOLLARS: u128 = 100 * CENTS;

    const SECS_PER_BLOCK: u64 = 3;
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
            name_of_intention: initial_authorities.clone().into_iter().map(|i| (i.0.into(), b"chainx".to_vec())).collect(),
            url_of_intention: initial_authorities.into_iter().map(|i| (i.0.into(), b"chainx".to_vec())).collect(),
            minimum_validator_count: 1,
            validator_count: 4,
            candidate_count: 4 * 4,
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
            token_list: vec![],
            transfer_token_fee: 10,
        }),
        financialrecords: Some(FinancialRecordsConfig {
            deposit_fee: 10,
            withdrawal_fee: 10,
        }),
        multisig: Some(MultiSigConfig {
            genesis_multi_sig: vec![],
            deploy_fee: 0,
            exec_fee: 0,
            confirm_fee: 0,
            balances_config: balances_config_copy,
        }),
    }
}
