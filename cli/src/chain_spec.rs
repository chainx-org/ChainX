// Copyright 2019-2020 ChainX Project Authors. Licensed under GPL-3.0.

#![allow(unused)]
use std::collections::BTreeMap;
use std::convert::TryInto;

use hex_literal::hex;
use serde::{Deserialize, Serialize};
use serde_json::json;

use sc_chain_spec::ChainSpecExtension;
use sc_service::config::TelemetryEndpoints;
use sc_service::{ChainType, Properties};

use sp_authority_discovery::AuthorityId as AuthorityDiscoveryId;
use sp_consensus_babe::AuthorityId as BabeId;
use sp_core::{crypto::UncheckedInto, sr25519, Pair, Public};
use sp_finality_grandpa::AuthorityId as GrandpaId;
use sp_runtime::traits::{IdentifyAccount, Verify};

use pallet_im_online::sr25519::AuthorityId as ImOnlineId;
use sp_core::crypto::AccountId32;

use chainx_primitives::{AccountId, AssetId, Balance, ReferralId, Signature};
use chainx_runtime::constants::{currency::DOLLARS, time::DAYS};
use xp_assets_registrar::Chain;
use xp_protocol::{NetworkType, PCX, PCX_DECIMALS, X_BTC};
use xpallet_gateway_bitcoin::{BtcParams, BtcTxVerifier};
use xpallet_gateway_common::types::TrusteeInfoConfig;

use crate::genesis::assets::{genesis_assets, init_assets, pcx, AssetParams};
use crate::genesis::bitcoin::{btc_genesis_params, BtcGenesisParams, BtcTrusteeParams};

use chainx_runtime as chainx;
use dev_runtime as dev;
use malan_runtime as malan;

// Note this is the URL for the telemetry server
#[allow(unused)]
const POLKADOT_TELEMETRY_URL: &str = "wss://telemetry.polkadot.io/submit/";
#[allow(unused)]
const CHAINX_TELEMETRY_URL: &str = "wss://telemetry.chainx.org/submit/";

/// Node `ChainSpec` extensions.
///
/// Additional parameters for some Substrate core modules,
/// customizable from the chain spec.
#[derive(Default, Clone, Serialize, Deserialize, ChainSpecExtension)]
#[serde(rename_all = "camelCase")]
pub struct Extensions {
    /// Block numbers with known hashes.
    pub fork_blocks: sc_client_api::ForkBlocks<chainx_primitives::Block>,
    /// Known bad block hashes.
    pub bad_blocks: sc_client_api::BadBlocks<chainx_primitives::Block>,
    /// This value will be set by the `sync-state rpc` implementation.
    pub light_sync_state: sc_sync_state_rpc::LightSyncStateExtension,
}

/// The `ChainSpec` parameterised for the chainx mainnet runtime.
pub type ChainXChainSpec = sc_service::GenericChainSpec<chainx::GenesisConfig, Extensions>;
/// The `ChainSpec` parameterised for the chainx development runtime.
pub type DevChainSpec = sc_service::GenericChainSpec<dev::GenesisConfig, Extensions>;
/// The `ChainSpec` parameterised for the chainx testnet runtime.
pub type MalanChainSpec = sc_service::GenericChainSpec<malan::GenesisConfig, Extensions>;

type AccountPublic = <Signature as Verify>::Signer;

/// Helper function to generate a crypto pair from seed
pub fn get_from_seed<TPublic: Public>(seed: &str) -> <TPublic::Pair as Pair>::Public {
    TPublic::Pair::from_string(&format!("//{}", seed), None)
        .expect("static values are valid; qed")
        .public()
}

/// Helper function to generate an account ID from seed
pub fn get_account_id_from_seed<TPublic: Public>(seed: &str) -> AccountId
where
    AccountPublic: From<<TPublic::Pair as Pair>::Public>,
{
    AccountPublic::from(get_from_seed::<TPublic>(seed)).into_account()
}

type AuthorityKeysTuple = (
    (AccountId, ReferralId), // (Staking ValidatorId, ReferralId)
    BabeId,
    GrandpaId,
    ImOnlineId,
    AuthorityDiscoveryId,
);

/// Helper function to generate an authority key for babe
pub fn authority_keys_from_seed(seed: &str) -> AuthorityKeysTuple {
    (
        (
            get_account_id_from_seed::<sr25519::Public>(seed),
            seed.as_bytes().to_vec(),
        ),
        get_from_seed::<BabeId>(seed),
        get_from_seed::<GrandpaId>(seed),
        get_from_seed::<ImOnlineId>(seed),
        get_from_seed::<AuthorityDiscoveryId>(seed),
    )
}

#[inline]
fn balance(input: Balance, decimals: u8) -> Balance {
    input * 10_u128.pow(decimals as u32)
}

/// A small macro for generating the info of PCX endowed accounts.
macro_rules! endowed_gen {
    ( $( ($seed:expr, $value:expr), )+ ) => {
        {
            let mut endowed = BTreeMap::new();
            let pcx_id = pcx().0;
            let endowed_info = vec![
                $((get_account_id_from_seed::<sr25519::Public>($seed), balance($value, PCX_DECIMALS)),)+
            ];
            endowed.insert(pcx_id, endowed_info);
            endowed
        }
    }
}

macro_rules! endowed {
    ( $( ($pubkey:expr, $value:expr), )+ ) => {
        {
            let mut endowed = BTreeMap::new();
            let pcx_id = pcx().0;
            let endowed_info = vec![
                $((($pubkey).into(), balance($value, PCX_DECIMALS)),)+
            ];
            endowed.insert(pcx_id, endowed_info);
            endowed
        }
    }
}

const ENDOWMENT: Balance = 10_000_000 * DOLLARS;
const STASH: Balance = 100 * DOLLARS;

/// Helper function to generate the network properties.
fn as_properties(network: NetworkType) -> Properties {
    json!({
        "ss58Format": network.ss58_addr_format_id(),
        "network": network,
        "tokenDecimals": PCX_DECIMALS,
        "tokenSymbol": "PCX"
    })
    .as_object()
    .expect("network properties generation can not fail; qed")
    .to_owned()
}

pub fn development_config() -> Result<DevChainSpec, String> {
    let wasm_binary =
        dev::WASM_BINARY.ok_or_else(|| "Development wasm binary not available".to_string())?;

    let endowed_balance = 50 * DOLLARS;
    let constructor = move || {
        build_dev_genesis(
            wasm_binary,
            vec![authority_keys_from_seed("Alice")],
            get_account_id_from_seed::<sr25519::Public>("Alice"),
            genesis_assets(),
            endowed_gen![
                ("Alice", endowed_balance),
                ("Bob", endowed_balance),
                ("Alice//stash", endowed_balance),
                ("Bob//stash", endowed_balance),
            ],
            btc_genesis_params(include_str!("res/btc_genesis_params_testnet.json")),
            crate::genesis::bitcoin::local_testnet_trustees(),
        )
    };
    Ok(DevChainSpec::from_genesis(
        "Development",
        "dev",
        ChainType::Development,
        constructor,
        vec![],
        None,
        Some("chainx-dev"),
        Some(as_properties(NetworkType::Testnet)),
        Default::default(),
    ))
}

#[cfg(feature = "runtime-benchmarks")]
pub fn benchmarks_config() -> Result<DevChainSpec, String> {
    let wasm_binary =
        dev::WASM_BINARY.ok_or_else(|| "Development wasm binary not available".to_string())?;

    let endowed_balance = 50 * DOLLARS;
    let constructor = move || {
        build_dev_genesis(
            wasm_binary,
            vec![authority_keys_from_seed("Alice")],
            get_account_id_from_seed::<sr25519::Public>("Alice"),
            genesis_assets(),
            endowed_gen![
                ("Alice", endowed_balance),
                ("Bob", endowed_balance),
                ("Alice//stash", endowed_balance),
                ("Bob//stash", endowed_balance),
            ],
            btc_genesis_params(include_str!("res/btc_genesis_params_benchmarks.json")),
            crate::genesis::bitcoin::benchmarks_trustees(),
        )
    };
    Ok(DevChainSpec::from_genesis(
        "Benchmarks",
        "dev",
        ChainType::Development,
        constructor,
        vec![],
        None,
        Some("chainx-dev"),
        Some(as_properties(NetworkType::Testnet)),
        Default::default(),
    ))
}

pub fn local_testnet_config() -> Result<DevChainSpec, String> {
    let wasm_binary =
        dev::WASM_BINARY.ok_or_else(|| "Development wasm binary not available".to_string())?;

    let endowed_balance = 50 * DOLLARS;
    let constructor = move || {
        build_dev_genesis(
            wasm_binary,
            vec![
                authority_keys_from_seed("Alice"),
                authority_keys_from_seed("Bob"),
            ],
            get_account_id_from_seed::<sr25519::Public>("Alice"),
            genesis_assets(),
            endowed_gen![
                ("Alice", endowed_balance),
                ("Bob", endowed_balance),
                ("Charlie", endowed_balance),
                ("Dave", endowed_balance),
                ("Eve", endowed_balance),
                ("Ferdie", endowed_balance),
                ("Alice//stash", endowed_balance),
                ("Bob//stash", endowed_balance),
                ("Charlie//stash", endowed_balance),
                ("Dave//stash", endowed_balance),
                ("Eve//stash", endowed_balance),
                ("Ferdie//stash", endowed_balance),
            ],
            btc_genesis_params(include_str!("res/btc_genesis_params_testnet.json")),
            crate::genesis::bitcoin::local_testnet_trustees(),
        )
    };
    Ok(DevChainSpec::from_genesis(
        "ChainX Local Testnet",
        "dev",
        ChainType::Local,
        constructor,
        vec![],
        None,
        Some("pcx"),
        Some(as_properties(NetworkType::Testnet)),
        Default::default(),
    ))
}

pub fn mainnet_config() -> Result<ChainXChainSpec, String> {
    ChainXChainSpec::from_json_bytes(&include_bytes!("./res/chainx_regenesis.json")[..])
}

pub fn malan_config() -> Result<MalanChainSpec, String> {
    MalanChainSpec::from_json_bytes(&include_bytes!("./res/malan.json")[..])
}

pub fn new_malan_config() -> Result<MalanChainSpec, String> {
    let wasm_binary =
        malan::WASM_BINARY.ok_or_else(|| "ChainX wasm binary not available".to_string())?;

    let initial_authorities: Vec<AuthorityKeysTuple> = vec![
        (
            (
                // 5QkGjd5rsczm4qVgVpzRSdBe2SrhLvKrPrYFeAAtw4qbdRPh
                hex!["31000d19a3e9607d92b3697a661a6e7e9fbb65361846680d968cfc86c9561103"].into(),
                b"Hotbit".to_vec(),
            ),
            // 5DwEF6ek2uYzQEeW1Mx4YjFcrBVNvQyUHEUBWq7sXE9XbzEe
            hex!["52c4cb6299ef78711dd1025b7bfc91655abed0f028bbf04145c2e249b1454909"]
                .unchecked_into(),
            // 5EjGLje6XHExxPyuijBg8c8MGTbG6A3fLKGzEMFV8qHKZNjN
            hex!["75e1249435a447adc812cc418c01fb5719488025add677bcc931d36a2338848a"]
                .unchecked_into(),
            // 5GNFfpS8wy2bjHnR43AyRpYVyHsKHKwzDRPiJ83XvxeojrxX
            hex!["be533292b9da99f2d03eb1ef7c4c9dfe3dbe26bf2fca75562d2618fbc7870b24"]
                .unchecked_into(),
            // 5DSEUDH8scoC2XbeAuAxjki2zfJEHhg38HcseSWLJuJrzfj5
            hex!["3ca7705b612b2bd56a50a6284b8095bb23c71805e9ca047256f630589944f815"]
                .unchecked_into(),
        ),
        (
            (
                // 5StNFoeSmLXr7SfDuwJqHR5CyKV2o4BD2yU36GGay3GVFhtt
                hex!["8fa51087d1a7327c90da45f8e369e31037606427f07ef77007a41036227a3a5b"].into(),
                b"Web3".to_vec(),
            ),
            // 5GjmArSffr9wwZ1gJMfU7yAfguJzrpbrDdjMx9yTvi6zeQeK
            hex!["cebaa8ae0af251cbf2aa5e397a6186d440b1c9e4f930388b209d0b5f93dbbf70"]
                .unchecked_into(),
            // 5Hcq9FpiRhJywuVuvYWRSMakeB8dWwb1yKVkMzuCUxhMLxPG
            hex!["f5ad8c0b2806effb7a77234b2955860c95fad100ea706fa60ceb7274fd399e63"]
                .unchecked_into(),
            // 5C4rKSUr5p3Gc2bySXtqQcmsT1pRPUJiB33jgZ3YivXC9WqC
            hex!["001c7bf4abd047bc97a1fb3c201d6a785e1eb3c818c838b5f2f0be98121f586c"]
                .unchecked_into(),
            // 5Da114jPuaKkcFh8BTUKUmG9qD6oMYC5XXQAEFKo6gGudY3Z
            hex!["42941089ea8a4353e2dab6905d27260735526a1a408274fc6c0d233b1a9e311e"]
                .unchecked_into(),
        ),
        (
            (
                // 5RaxFQc7E4ACr4FVoHj2SMA6MGMqT8Ck9mDV5byGZtPPUw8f
                hex!["5620d851190bda61acb08f1a06defcdd5a3c7da3c33819643e7d6ee232ee78bf"].into(),
                b"ChainY".to_vec(),
            ),
            // 5FuwXy3d71LYWtgiECH2CCe6xqfzTQrjo8zv1T9EsxkBzVXx
            hex!["aa41c49785e1f4bc9079f3c2af7b9f43ff88545e9777b6bb291574982a5a9169"]
                .unchecked_into(),
            // 5Cq6tNkVFHZe1nrdtB58QXgAEwnX5q9a4QFzhPHs87noRBng
            hex!["21dc525f93a2afcb7abf0cb094c26ab807af5f89590269f0dd5fbaa2b91eb754"]
                .unchecked_into(),
            // 5DaTSiRZzAVJ4fqJc1eGA1yBz9TwFcWX4JwiJmqWgMSRBtaC
            hex!["42ed13bde38b21f479448b8ed9d155a9e7318acfafcb06f4e50d3098c1304c11"]
                .unchecked_into(),
            // 5H3DD5sSD2r6d79Kw3b78NGegEmqT1eVkE1e1waEvTmceHSv
            hex!["dc097bedcd2c06e87054f644a4cbe7f78470687a03fe019af6fffe775390d641"]
                .unchecked_into(),
        ),
        (
            (
                // 5RXaXGcz84KQ1XZeAMbjbexXRPNAaPdt3st5QDG3H6VegwD6
                hex!["538df88774a48e4ec759cfb3d25f12e343d8048a4bfa92643070b73cbb4be843"].into(),
                b"BiHODL".to_vec(),
            ),
            // 5CyFAnrP5nrgtrFL5nV8L78u2wRRzqebftcHgLkmqSNHkYEX
            hex!["28122b0c7781c8c151348a71981e82095dddca65a04c97394be7a9cbfb24cc55"]
                .unchecked_into(),
            // 5Gv4i3TnzM6LrJTS4ssBRHn1PhggyvJEdwXapiuA4eDP2jNE
            hex!["d696267a82fad34996ff9b8e9de1495e0fdceb3516f7f61a2b7452f81bcbc236"]
                .unchecked_into(),
            // 5Dr5Jt5zKMfCAaHazn7vr3ft4VpUxyrn7YQAwWSRqykimBTp
            hex!["4ed67d86aaf12d7b9e05ecd1e7f5f3406f8ee9537a2bf1ffa7513bd9ecba3e0d"]
                .unchecked_into(),
            // 5Gjzu9kCd37jdZkyNLRAvDnfAvVx1fAMkGELTGz2cKGvJXNc
            hex!["cee8e033890610bfdae1145469d93a81300fe0f92ae5a4b54deb3f6da958a467"]
                .unchecked_into(),
        ),
        (
            (
                // 5RAZf8UHcbS5RBRpP9zptQJm84tpfnxcJ64ctSyxNJeLLxtq
                hex!["4386e83d66fbdf9ebe72af81d453f41fb8f877287f04823665fc81b58cab6e6b"].into(),
                b"XPool".to_vec(),
            ),
            // 5Co3EQJfM5tnTHDjxzKAWDiLUtiJu2g86mp6vjGXopbzfJjA
            hex!["20498732449e249d32c27f415083004cf045cb33e740f0e8e9a2b656e11cba73"]
                .unchecked_into(),
            // 5FeNZC89WyB3KW6an2WMpyYgpMmDoyvuHKiqQuuDMJPsUG1E
            hex!["9e6211f1cb9cadc180c189bcce9e70158dc9ee1df15a0bfb147c6c91fe432655"]
                .unchecked_into(),
            // 5HGJjjAQSfXvyyjMADjmZVq6bP6ouHPQPsfXQ9VMX4r6AZXy
            hex!["e60648cbf567f22f5ca7b5c9897f4869e6015413ecdc4bca325f8773439efd63"]
                .unchecked_into(),
            // 5Ey51L18oBfwepK6XCKPd48G433jZN9pSMzEcgH62rEYLUTp
            hex!["80686c3f3b6b83143ec462269e63a4cefd86d2bed6ad1717610e5ba965ca0a5a"]
                .unchecked_into(),
        ),
    ];
    let constructor = move || {
        malan_genesis(
            wasm_binary,
            initial_authorities.clone(),
            genesis_assets(),
            btc_genesis_params(include_str!("res/btc_genesis_params_testnet.json")),
            crate::genesis::bitcoin::mainnet_trustees(),
        )
    };

    let bootnodes = Default::default();

    Ok(MalanChainSpec::from_genesis(
        "ChainX-Malan",
        "chainx-malan",
        ChainType::Live,
        constructor,
        bootnodes,
        Some(
            TelemetryEndpoints::new(vec![(CHAINX_TELEMETRY_URL.to_string(), 0)])
                .expect("ChainX telemetry url is valid; qed"),
        ),
        Some("pcx1"),
        Some(as_properties(NetworkType::Testnet)),
        Default::default(),
    ))
}

fn malan_session_keys(
    babe: BabeId,
    grandpa: GrandpaId,
    im_online: ImOnlineId,
    authority_discovery: AuthorityDiscoveryId,
) -> malan::SessionKeys {
    malan::SessionKeys {
        grandpa,
        babe,
        im_online,
        authority_discovery,
    }
}

fn malan_genesis(
    wasm_binary: &[u8],
    initial_authorities: Vec<AuthorityKeysTuple>,
    assets: Vec<AssetParams>,
    bitcoin: BtcGenesisParams,
    trustees: Vec<(Chain, TrusteeInfoConfig, Vec<BtcTrusteeParams>)>,
) -> malan::GenesisConfig {
    use malan_runtime::constants::time::DAYS;

    let (assets, assets_restrictions) = init_assets(assets);
    let tech_comm_members: Vec<AccountId> = vec![
        // 5QChfn7eDn96LDSy79WZHZYNWpjjNWuSUFxAwuZVmGmCpXfb
        hex!["2a077c909d0c5dcb3748cc11df2fb406ab8f35901b1a93010b78353e4a2bde0d"].into(),
        // 5GxS3YuwjhZZtmPmLEJuGPuz14gEJsunabqNLYTthXfThRwG
        hex!["d86477344ad5c27a45c4c178c7cca1b7b111380a4fbe7e23b3488a42ce56ca30"].into(),
        // 5DhacpyA2Ykpjx4AUJGbF7qa8tPqFELEVQYXQsxXQSauPb9r
        hex!["485bf22c979d4a61643f57a2006ff4fb7447a2a8ed905997c5f6b0230f39b860"].into(),
    ];

    let btc_genesis_trustees = trustees
        .iter()
        .find_map(|(chain, _, trustee_params)| {
            if *chain == Chain::Bitcoin {
                Some(
                    trustee_params
                        .iter()
                        .map(|i| (i.0).clone())
                        .collect::<Vec<_>>(),
                )
            } else {
                None
            }
        })
        .expect("bitcoin trustees generation can not fail; qed");

    malan::GenesisConfig {
        sudo: malan::SudoConfig {
            key: hex!["b0ca18cce5c51f51655acf683453aa1ff319e3c3edd00b43b36a686a3ae34341"].into(),
        },
        system: malan::SystemConfig {
            code: wasm_binary.to_vec(),
            changes_trie_config: Default::default(),
        },
        babe: Default::default(),
        grandpa: malan::GrandpaConfig {
            authorities: vec![],
        },
        council: malan::CouncilConfig::default(),
        technical_committee: malan::TechnicalCommitteeConfig {
            members: tech_comm_members,
            phantom: Default::default(),
        },
        technical_membership: Default::default(),
        democracy: malan::DemocracyConfig::default(),
        treasury: Default::default(),
        elections: Default::default(),
        im_online: malan::ImOnlineConfig { keys: vec![] },
        authority_discovery: malan::AuthorityDiscoveryConfig { keys: vec![] },
        session: malan::SessionConfig {
            keys: initial_authorities
                .iter()
                .map(|x| {
                    (
                        (x.0).0.clone(),
                        (x.0).0.clone(),
                        malan::SessionKeys {
                            grandpa: x.2.clone(),
                            babe: x.1.clone(),
                            im_online: x.3.clone(),
                            authority_discovery: x.4.clone(),
                        },
                    )
                })
                .collect::<Vec<_>>(),
        },
        balances: Default::default(),
        indices: malan::IndicesConfig { indices: vec![] },
        x_system: malan::XSystemConfig {
            network_props: NetworkType::Testnet,
        },
        x_assets_registrar: malan::XAssetsRegistrarConfig { assets },
        x_assets: malan::XAssetsConfig {
            assets_restrictions,
            endowed: Default::default(),
        },
        x_gateway_common: malan::XGatewayCommonConfig { trustees },
        x_gateway_bitcoin: malan::XGatewayBitcoinConfig {
            genesis_trustees: btc_genesis_trustees,
            network_id: bitcoin.network,
            confirmation_number: bitcoin.confirmation_number,
            genesis_hash: bitcoin.hash(),
            genesis_info: (bitcoin.header(), bitcoin.height),
            params_info: BtcParams::new(
                // for signet and regtest
                545259519,            // max_bits
                2 * 60 * 60,          // block_max_future
                2 * 7 * 24 * 60 * 60, // target_timespan_seconds
                10 * 60,              // target_spacing_seconds
                4,                    // retargeting_factor
            ), // retargeting_factor
            btc_withdrawal_fee: 500000,
            max_withdrawal_count: 100,
            verifier: BtcTxVerifier::Recover,
        },
        x_staking: malan::XStakingConfig {
            validator_count: 40,
            sessions_per_era: 12,
            glob_dist_ratio: (12, 88), // (Treasury, X-type Asset and Staking) = (12, 88)
            mining_ratio: (10, 90),    // (Asset Mining, Staking) = (10, 90)
            minimum_penalty: 100 * DOLLARS,
            candidate_requirement: (100 * DOLLARS, 1_000 * DOLLARS), // Minimum value (self_bonded, total_bonded) to be a validator candidate
            minimum_validator_count: 4,
            maximum_validator_count: 5,
            ..Default::default()
        },
        x_mining_asset: malan::XMiningAssetConfig {
            claim_restrictions: vec![(X_BTC, (10, DAYS * 7))],
            mining_power_map: vec![(X_BTC, 400)],
        },
        x_spot: malan::XSpotConfig {
            trading_pairs: vec![(PCX, X_BTC, 9, 2, 100000, true)],
        },
        x_genesis_builder: malan::XGenesisBuilderConfig {
            params: crate::genesis::genesis_builder_params(),
            initial_authorities: initial_authorities
                .iter()
                .map(|i| (i.0).1.clone())
                .collect(),
        },
    }
}

fn build_dev_genesis(
    wasm_binary: &[u8],
    initial_authorities: Vec<AuthorityKeysTuple>,
    root_key: AccountId,
    assets: Vec<AssetParams>,
    endowed: BTreeMap<AssetId, Vec<(AccountId, Balance)>>,
    bitcoin: BtcGenesisParams,
    trustees: Vec<(Chain, TrusteeInfoConfig, Vec<BtcTrusteeParams>)>,
) -> dev::GenesisConfig {
    const ENDOWMENT: Balance = 10_000_000 * DOLLARS;
    const STASH: Balance = 100 * DOLLARS;
    let (assets, assets_restrictions) = init_assets(assets);

    let endowed_accounts = endowed
        .get(&PCX)
        .expect("PCX endowed; qed")
        .iter()
        .cloned()
        .map(|(k, _)| k)
        .collect::<Vec<_>>();

    let num_endowed_accounts = endowed_accounts.len();

    let mut total_endowed = Balance::default();
    let balances = endowed
        .get(&PCX)
        .expect("PCX endowed; qed")
        .iter()
        .cloned()
        .map(|(k, _)| {
            total_endowed += ENDOWMENT;
            (k, ENDOWMENT)
        })
        .collect::<Vec<_>>();

    // The value of STASH balance will be reserved per phragmen member.
    let phragmen_members = endowed_accounts
        .iter()
        .take((num_endowed_accounts + 1) / 2)
        .cloned()
        .map(|member| (member, STASH))
        .collect();

    let tech_comm_members = endowed_accounts
        .iter()
        .take((num_endowed_accounts + 1) / 2)
        .cloned()
        .collect::<Vec<_>>();

    // PCX only reserves the native asset id in assets module,
    // the actual native fund management is handled by pallet_balances.
    let mut assets_endowed = endowed;
    assets_endowed.remove(&PCX);

    let btc_genesis_trustees = trustees
        .iter()
        .find_map(|(chain, _, trustee_params)| {
            if *chain == Chain::Bitcoin {
                Some(
                    trustee_params
                        .iter()
                        .map(|i| (i.0).clone())
                        .collect::<Vec<_>>(),
                )
            } else {
                None
            }
        })
        .expect("bitcoin trustees generation can not fail; qed");

    dev::GenesisConfig {
        sudo: dev::SudoConfig { key: root_key },
        system: dev::SystemConfig {
            code: wasm_binary.to_vec(),
            changes_trie_config: Default::default(),
        },
        babe: dev::BabeConfig {
            authorities: vec![],
            epoch_config: Some(dev::BABE_GENESIS_EPOCH_CONFIG),
        },
        grandpa: dev::GrandpaConfig {
            authorities: vec![],
        },
        council: dev::CouncilConfig::default(),
        technical_committee: dev::TechnicalCommitteeConfig {
            members: tech_comm_members,
            phantom: Default::default(),
        },
        technical_membership: Default::default(),
        democracy: dev::DemocracyConfig::default(),
        treasury: Default::default(),
        elections: dev::ElectionsConfig {
            members: phragmen_members,
        },
        im_online: dev::ImOnlineConfig { keys: vec![] },
        authority_discovery: dev::AuthorityDiscoveryConfig { keys: vec![] },
        session: dev::SessionConfig {
            keys: initial_authorities
                .iter()
                .map(|x| {
                    (
                        (x.0).0.clone(),
                        (x.0).0.clone(),
                        dev::SessionKeys {
                            grandpa: x.2.clone(),
                            babe: x.1.clone(),
                            im_online: x.3.clone(),
                            authority_discovery: x.4.clone(),
                        },
                    )
                })
                .collect::<Vec<_>>(),
        },
        balances: dev::BalancesConfig { balances },
        indices: dev::IndicesConfig { indices: vec![] },
        x_system: dev::XSystemConfig {
            network_props: NetworkType::Testnet,
        },
        x_assets_registrar: dev::XAssetsRegistrarConfig { assets },
        x_assets: dev::XAssetsConfig {
            assets_restrictions,
            endowed: assets_endowed,
        },
        x_gateway_common: dev::XGatewayCommonConfig { trustees },
        x_gateway_bitcoin: dev::XGatewayBitcoinConfig {
            genesis_trustees: btc_genesis_trustees,
            network_id: bitcoin.network,
            confirmation_number: bitcoin.confirmation_number,
            genesis_hash: bitcoin.hash(),
            genesis_info: (bitcoin.header(), bitcoin.height),
            params_info: BtcParams::new(
                // for signet and regtest
                545259519,            // max_bits
                2 * 60 * 60,          // block_max_future
                2 * 7 * 24 * 60 * 60, // target_timespan_seconds
                10 * 60,              // target_spacing_seconds
                4,                    // retargeting_factor
            ), // retargeting_factor
            btc_withdrawal_fee: 500000,
            max_withdrawal_count: 100,
            verifier: BtcTxVerifier::Recover,
        },
        x_staking: dev::XStakingConfig {
            validator_count: 40,
            sessions_per_era: 12,
            glob_dist_ratio: (12, 88), // (Treasury, X-type Asset and Staking) = (12, 88)
            mining_ratio: (10, 90),    // (Asset Mining, Staking) = (10, 90)
            minimum_penalty: 100 * DOLLARS,
            candidate_requirement: (100 * DOLLARS, 1_000 * DOLLARS), // Minimum value (self_bonded, total_bonded) to be a validator candidate
            ..Default::default()
        },
        x_mining_asset: dev::XMiningAssetConfig {
            claim_restrictions: vec![(X_BTC, (10, DAYS * 7))],
            mining_power_map: vec![(X_BTC, 400)],
        },
        x_spot: dev::XSpotConfig {
            trading_pairs: vec![(PCX, X_BTC, 9, 2, 100000, true)],
        },
        x_genesis_builder: dev::XGenesisBuilderConfig {
            params: crate::genesis::genesis_builder_params(),
            initial_authorities: initial_authorities
                .iter()
                .map(|i| (i.0).1.clone())
                .collect(),
        },
    }
}
