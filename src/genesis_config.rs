// Copyright 2018 chainpool

use chainx_runtime::{GenesisConfig, ConsensusConfig, DemocracyConfig, TreasuryConfig,
                     SessionConfig, StakingConfig, TimestampConfig, BalancesConfig, Permill};
use super::cli::ChainSpec;
use keyring::Keyring;
use ed25519;


pub fn testnet_genesis(chainspec: ChainSpec) -> GenesisConfig {
    let auth1 = ed25519::Pair::from_seed(b"Alice                           ")
        .public()
        .into();
    let auth2 = ed25519::Pair::from_seed(b"Bob                             ")
        .public()
        .into();
    let auth3 = ed25519::Pair::from_seed(b"Gavin                           ")
        .public()
        .into();
    let auth4 = ed25519::Pair::from_seed(b"Satoshi                         ")
        .public()
        .into();
    let initial_authorities = match chainspec {
        ChainSpec::Dev => vec![auth1],
        ChainSpec::Local => vec![auth1, auth2],
        ChainSpec::Multi => vec![auth1, auth2, auth3, auth4],
    };
    GenesisConfig {
        consensus: Some(ConsensusConfig {
            code: include_bytes!(
                "../runtime/wasm/target/wasm32-unknown-unknown/release/chainx_runtime.compact.wasm"
            ).to_vec(),
            authorities: initial_authorities.clone(),
        }),
        system: None,
        balances: Some(BalancesConfig {
            transaction_base_fee: 1,
            transaction_byte_fee: 0,
            existential_deposit: 500,
            transfer_fee: 0,
            creation_fee: 0,
            reclaim_rebate: 0,
            balances: vec![
                (Keyring::Alice.to_raw_public().into(), 10000),
                (Keyring::Bob.to_raw_public().into(), 10000),
                (Keyring::Charlie.to_raw_public().into(), 10000),
            ],
        }),

        session: Some(SessionConfig {
            validators: initial_authorities
                .iter()
                .cloned()
                .map(Into::into)
                .collect(),
            session_length: 720, // that's 1 hour per session.
        }),
        staking: Some(StakingConfig {
            current_era: 0,
            bonding_duration: 90, // 90 days per bond.
            intentions: vec![],
            minimum_validator_count: 1,
            validator_count: 2,
            sessions_per_era: 24, // 24 hours per era.
            session_reward: 100,
            offline_slash_grace: 0,
            offline_slash: 10000,
        }),
        democracy: Some(DemocracyConfig {
            launch_period: 120 * 24 * 14, // 2 weeks per public referendum
            voting_period: 120 * 24 * 28, // 4 weeks to discuss & vote on an active referendum
            minimum_deposit: 1000, // 1000 as the minimum deposit for a referendum
        }),
        timestamp: Some(TimestampConfig {
            period: 5,                  // 5 second block time.
        }),
        treasury: Some(TreasuryConfig {
            proposal_bond: Permill::from_percent(5),
            proposal_bond_minimum: 1_000_000,
            spend_period: 12 * 60 * 24,
            burn: Permill::from_percent(50),
        }),
    }
}
