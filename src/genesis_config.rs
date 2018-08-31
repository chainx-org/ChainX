use ed25519;
use chainx_runtime::{GenesisConfig, ConsensusConfig, CouncilConfig, DemocracyConfig,
SessionConfig, StakingConfig, TimestampConfig};

pub fn testnet_genesis() -> GenesisConfig {
    let god_key = hex!("3d866ec8a9190c8343c2fc593d21d8a6d0c5c4763aaab2349de3a6111d64d124");
    GenesisConfig {
        consensus: Some(ConsensusConfig {
            code: include_bytes!(
                      "../runtime/wasm/target/wasm32-unknown-unknown/release/chainx_runtime.compact.wasm"
                  ).to_vec(),
                  authorities: vec![ed25519::Pair::from_seed(&god_key).public().into()],
        }),
        system: None,
        session: Some(SessionConfig {
            validators: vec![god_key.clone().into()],
            session_length: 720, // that's 1 hour per session.
        }),
        staking: Some(StakingConfig {
            current_era: 0,
            intentions: vec![],
            transaction_base_fee: 100,
            transaction_byte_fee: 1,
            transfer_fee: 0,
            creation_fee: 0,
            reclaim_rebate: 0,
            existential_deposit: 500,
            balances: vec![(god_key.clone().into(), 1u128 << 63)]
                .into_iter()
                .collect(),
                validator_count: 12,
                minimum_validator_count: 0,
                sessions_per_era: 24, // 24 hours per era.
                bonding_duration: 90, // 90 days per bond.
                early_era_slash: 10000,
                session_reward: 100,
                offline_slash_grace: 0,
        }),
        democracy: Some(DemocracyConfig {
            launch_period: 120 * 24 * 14, // 2 weeks per public referendum
            voting_period: 120 * 24 * 28, // 4 weeks to discuss & vote on an active referendum
            minimum_deposit: 1000, // 1000 as the minimum deposit for a referendum
        }),
        council: Some(CouncilConfig {
            active_council: vec![],
            candidacy_bond: 1000, // 1000 to become a council candidate
            voter_bond: 100, // 100 down to vote for a candidate
            present_slash_per_voter: 1, // slash by 1 per voter for an invalid presentation.
            carry_count: 24, // carry over the 24 runners-up to the next council election
            presentation_duration: 120 * 24, // one day for presenting winners.
            // one week period between possible council elections.
            approval_voting_period: 7 * 120 * 24,
            term_duration: 180 * 120 * 24, // 180 day term duration for the council.
            // start with no council: we'll raise this once the stake has been dispersed a bit.
            desired_seats: 0,
            // one addition vote should go by before an inactive voter can be reaped.
            inactive_grace_period: 1,
            // 90 day cooling off period if council member vetoes a proposal.
            cooloff_period: 90 * 120 * 24,
            voting_period: 7 * 120 * 24, // 7 day voting period for council members.
        }),
        timestamp: Some(TimestampConfig {
            period: 5,                  // 5 second block time.
        }),
    }
}
