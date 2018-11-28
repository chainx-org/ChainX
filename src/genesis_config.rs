// Copyright 2018 chainpool
extern crate base58;
extern crate chain as btc_chain;
extern crate cxrml_exchange_pendingorders;
extern crate cxrml_tokenbalances;
extern crate keys;
extern crate primitives as btc_primitives;
extern crate substrate_primitives;

use self::base58::FromBase58;
use self::cxrml_exchange_pendingorders::OrderPair;
use chainx_runtime::{
    AssociationsConfig, BalancesConfig, BalancesConfigCopy, BridgeOfBTC, BridgeOfBTCConfig,
    CXSystemConfig, ConsensusConfig, ContractConfig, CouncilVotingConfig, DemocracyConfig,
    GenesisConfig, MatchOrderConfig, MultiSigConfig, Params, PendingOrdersConfig, Perbill, Permill,
    Runtime, SessionConfig, StakingConfig, TimestampConfig, Token, TokenBalancesConfig,
    TokenStakingConfig, TreasuryConfig, WithdrawalConfig,
};

use super::cli::ChainSpec;
use ed25519;
use keyring::Keyring;

use self::btc_chain::BlockHeader;
use self::btc_primitives::{compact::Compact, hash::H256};
use self::cxrml_tokenbalances::{TokenT, Trait};
use self::keys::DisplayLayout;

pub fn testnet_genesis(chainspec: ChainSpec) -> GenesisConfig {
    let alice = ed25519::Pair::from_seed(b"Alice                           ").public();
    let bob = ed25519::Pair::from_seed(b"Bob                             ").public();
    let _charlie = ed25519::Pair::from_seed(b"Charlie                         ").public();
    let _dave = ed25519::Pair::from_seed(b"Dave                            ").public();
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
        cxsystem: Some(CXSystemConfig {
            death_account: substrate_primitives::H256([0; 32]),
            fee_buy_account: substrate_primitives::H256([1; 32]),
        }),
        tokenbalances: Some(TokenBalancesConfig {
            chainx_precision: 8,
            // token_list: Vec<(Token, Vec<(T::AccountId, T::TokenBalance)>)>
            // e.g. [("btc", [(account1, value), (account2, value)].to_vec()), ("eth", [(account1, value), (account2, value)].to_vec())]
            token_list: vec![
                (Token::new(BridgeOfBTC::SYMBOL.to_vec(), b"btc token".to_vec(), 8),
                // [(Keyring::Alice.to_raw_public().into(), 1_000_000), (Keyring::Bob.to_raw_public().into(), 1_000_000)].to_vec())
                vec![])
            ],

            transfer_token_fee: 10,
        }),
        multisig: Some(MultiSigConfig {
            genesis_multi_sig: vec![],
            deploy_fee: 0,
            exec_fee: 0,
            confirm_fee: 0,
            balances_config: balances_config_copy,
        }),
        associations: Some(AssociationsConfig {
            init_fee: 10,
        }),
        staking: Some(StakingConfig {
            current_era: 0,
            bonding_duration: 3 * MINUTES, // 3 days per bond.
            intentions: initial_authorities.clone().into_iter().map(|i| i.0.into()).collect(),
            intention_profiles: initial_authorities.clone().into_iter().map(|i| (i.0.into(), b"ChainX".to_vec(), b"chainx.org".to_vec())).collect(),
            minimum_validator_count: 1,
            validator_count: 6,
            sessions_per_era: 4, // 24 hours per era.
            shares_per_cert: 45,
            activation_per_share: 100000,
            maximum_cert_owner_count: 200,
            intention_threshold: 9000,
            offline_slash_grace: 0,
            offline_slash: Perbill::from_millionths(0),
            current_offline_slash: 0,
            current_session_reward: 0,
            cert_owner: auth1.0.into(),
            register_fee: 1,
            claim_fee: 1,
            stake_fee: 1,
            unstake_fee: 1,
            activate_fee: 1,
            deactivate_fee: 1,
            nominate_fee: 1,
            unnominate_fee: 1,
        }),
        tokenstaking: Some(TokenStakingConfig {
            fee: 10
        }),
        withdrawal: Some(WithdrawalConfig {
            withdrawal_fee: 10,
        }),
        bridge_btc: Some(BridgeOfBTCConfig {
            // start genesis block: (genesis, blocknumber)
            genesis: (BlockHeader {
                version: 545259520,
                previous_header_hash: H256::from_reversed_str("0000000000000119cb529b757340de5a642b21938930646c646241f19ab10789"),
                merkle_root_hash: H256::from_reversed_str("53a5a6865051ad1f350de8a8bd65700f9929808bae3c7e82c792647e767c2eab"),
                time: 1542972675,
                bits: Compact::new(436296509),
                nonce: 1691442840,
            }, 1444850),
            params_info: Params::new(520159231, // max_bits
                                     2 * 60 * 60,  // block_max_future
                                     64,  // max_fork_route_preset
                                     2 * 7 * 24 * 60 * 60,  // target_timespan_seconds
                                     10 * 60,  // target_spacing_seconds
                                     4), // retargeting_factor
            network_id: 1,
            utxo_max_index: 0,
            irr_block: 0,
            btc_fee: 1000,
            accounts_max_index: 0,
            cert_address: keys::Address::from_layout(&"2N6JXYKYLqN4e2A96FLnY5J1Mjj5MHXhp6b".from_base58().unwrap()).unwrap(),
            cert_redeem_script: b"522102e34d10113f2dd162e8d8614a4afbb8e2eb14eddf4036042b35d12cf5529056a2210311252930af8ba766b9c7a6580d8dc4bbf9b0befd17a8ef7fabac275bba77ae402103ece1a20b5468b12fd7beda3e62ef6b2f6ad9774489e9aff1c8bc684d87d7078053ae".to_vec(),
            receive_address: keys::Address::from_layout(&"2N8tR484JD32i1DY2FnRPLwBVaNuXSfzoAv".from_base58().unwrap()).unwrap(),
            redeem_script: b"52210306117a360e5dbe10e1938a047949c25a86c0b0e08a0a7c1e611b97de6b2917dd2102a79800dfed17ad4c78c52797aa3449925692bc8c83de469421080f42d27790ee2103f72c448a0e59f48d4adef86cba7b278214cece8e56ef32ba1d179e0a8129bdba53ae".to_vec(),
            fee: 0,
        }),
        pendingorders: Some(PendingOrdersConfig {
            order_fee: 0,
            pair_list: vec![
                (OrderPair { first: Runtime::CHAINX_SYMBOL.to_vec(), second: BridgeOfBTC::SYMBOL.to_vec() }, 8)
            ],
            max_command_id: 0,
            average_price_len:10000,
        }),
        matchorder: Some(MatchOrderConfig { match_fee: 10 ,maker_match_fee:50,taker_match_fee:100,fee_precision:100000}),

    }
}
