// Copyright 2018 chainpool

use chainx_runtime::{GenesisConfig, ConsensusConfig, CouncilConfig, DemocracyConfig,
                     SessionConfig, StakingConfig, TimestampConfig, BalancesConfig, TreasuryConfig,
                     ContractConfig, Permill, Perbill, TokenBalancesConfig};
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
                (Keyring::Alice.to_raw_public().into(), 1000000),
                (Keyring::Bob.to_raw_public().into(), 1000000),
                (Keyring::Charlie.to_raw_public().into(), 1000000),
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
            session_reward: Perbill::from_millionths(100),
            offline_slash_grace: 0,
            offline_slash: Perbill::from_millionths(1000),
            current_offline_slash: 0,
            current_session_reward: 0,
        }),
        democracy: Some(DemocracyConfig {
            launch_period: 120 * 24 * 14, // 2 weeks per public referendum
            voting_period: 120 * 24 * 28, // 4 weeks to discuss & vote on an active referendum
            minimum_deposit: 1000, // 1000 as the minimum deposit for a referendum
        }),
        council: Some(CouncilConfig {
            active_council: vec![],
            candidacy_bond: 10,
            voter_bond: 2,
            present_slash_per_voter: 1,
            carry_count: 4,
            presentation_duration: 10,
            approval_voting_period: 20,
            term_duration: 1000000,
            desired_seats: 0, // start with no council: we'll raise this once the stake has been dispersed a bit.
            inactive_grace_period: 1,
            cooloff_period: 75,
            voting_period: 20,
        }),
        timestamp: Some(TimestampConfig {
            period: 2,                  // 2 second block time.
        }),
        treasury: Some(TreasuryConfig {
            proposal_bond: Permill::from_percent(5),
            proposal_bond_minimum: 1_000_000,
            spend_period: 12 * 60 * 24,
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
    }
}
