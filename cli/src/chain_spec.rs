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

pub fn pre_malan_authorities() -> AuthorityKeysTuple {
    // 5E4MThREbKErsna6rPDuRxFWe9hjcg6PV9BE99B3TB8J6Ufo
    let account: AccountId =
        hex!["5833cec38892b33fadba17fe719f9bf89d6fd595e10c64c04b1dfac7bb5e1109"].into();
    let referal: ReferralId = b"Validator1".to_vec();

    // 5EkZWkS8vfkaqjEnR5dSXFvqL1w5KnmogysYawTyKqy953ZM
    let babe: BabeId =
        hex!["76de32534484c8683d37f4ebc18b94f83f3646add3a986790111bd764451d40a"].unchecked_into();
    // 5FaK685niWdJe3dFbV9MdFLZBEoADBkzvah3ePQ44ZqXVj6Y
    let grandpa: GrandpaId =
        hex!["9b496b2073b6d3ab37d1b50e8b37b0bff1c62751e2dbc637a3cdaf8535e6b1e3"].unchecked_into();
    // 5E4QZUuwmhZHjz6n9spLzt1BPN3qRAcFpTtAGnf66xHfptJr
    let imonlie: ImOnlineId =
        hex!["583e3e07179a25c17f879c79b26cd22c6ce0158c65fcf4ff6a256b80791d9e0a"].unchecked_into();
    // 5HdTCBVrSBxCQp4V2VPoTiM3m6Gnw9kWjxnTgAab7NW3VXaj
    let auth: AuthorityDiscoveryId =
        hex!["f626e754de6eda69535466986201cac54378edbf18a29a98728d128c157c8501"].unchecked_into();

    ((account, referal), babe, grandpa, imonlie, auth)
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

macro_rules! build_genesis {
    (
        $runtime: ident,
        $wasm_binary:expr,
        $initial_authorities:expr ,
        $root_key:expr,
        $assets:expr,
        $endowed:expr,
        $bitcoin:expr,
        $trustees:expr,
    ) => {{
        extern crate $runtime;

        let (assets, assets_restrictions) = init_assets($assets);

        let endowed_accounts = $endowed
            .get(&PCX)
            .expect("PCX endowed; qed")
            .iter()
            .cloned()
            .map(|(k, _)| k)
            .collect::<Vec<_>>();

        let num_endowed_accounts = endowed_accounts.len();

        let mut total_endowed = Balance::default();
        let balances = $endowed
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
        let mut assets_endowed = $endowed;
        assets_endowed.remove(&PCX);

        let btc_genesis_trustees = $trustees
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

        let mut config = $runtime::GenesisConfig::default();

        if stringify!($runtime) == "dev" {
            config.sudo = $runtime::SudoConfig { key: $root_key };
        };

        config.system = $runtime::SystemConfig {
            code: $wasm_binary.to_vec(),
            changes_trie_config: Default::default(),
        };

        config.babe = $runtime::BabeConfig {
            authorities: vec![],
            epoch_config: Some($runtime::BABE_GENESIS_EPOCH_CONFIG),
        };

        config.grandpa = $runtime::GrandpaConfig {
            authorities: vec![],
        };

        config.council = $runtime::CouncilConfig::default();

        config.technical_committee = $runtime::TechnicalCommitteeConfig {
            members: tech_comm_members,
            phantom: Default::default(),
        };

        config.technical_membership = Default::default();

        config.democracy = $runtime::DemocracyConfig::default();

        config.treasury = Default::default();

        config.elections = $runtime::ElectionsConfig {
            members: phragmen_members,
        };

        config.im_online = $runtime::ImOnlineConfig { keys: vec![] };

        config.authority_discovery = $runtime::AuthorityDiscoveryConfig { keys: vec![] };

        config.session = $runtime::SessionConfig {
            keys: $initial_authorities
                .iter()
                .map(|x| {
                    (
                        (x.0).0.clone(),
                        (x.0).0.clone(),
                        $runtime::SessionKeys {
                            grandpa: x.2.clone(),
                            babe: x.1.clone(),
                            im_online: x.3.clone(),
                            authority_discovery: x.4.clone(),
                        }
                    )
                })
                .collect::<Vec<_>>(),
        };

        config.balances = $runtime::BalancesConfig { balances };

        config.indices = $runtime::IndicesConfig { indices: vec![] };

        config.x_system = $runtime::XSystemConfig {
            network_props: if stringify!($runtime) == "chainx" {
                NetworkType::Mainnet
            } else {
                NetworkType::Testnet
            },
        };

        config.x_assets_registrar = $runtime::XAssetsRegistrarConfig { assets };

        config.x_assets = $runtime::XAssetsConfig {
            assets_restrictions,
            endowed: assets_endowed,
        };

        config.x_gateway_common = $runtime::XGatewayCommonConfig {
            trustees: $trustees
        };

        config.x_gateway_bitcoin = $runtime::XGatewayBitcoinConfig {
            genesis_trustees: btc_genesis_trustees,
            network_id: $bitcoin.network,
            confirmation_number: $bitcoin.confirmation_number,
            genesis_hash: $bitcoin.hash(),
            genesis_info: ($bitcoin.header(), $bitcoin.height),
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
        };

        config.x_staking = $runtime::XStakingConfig {
            validator_count: 40,
            sessions_per_era: 12,
            glob_dist_ratio: (12, 88), // (Treasury, X-type Asset and Staking) = (12, 88)
            mining_ratio: (10, 90),    // (Asset Mining, Staking) = (10, 90)
            minimum_penalty: 100 * DOLLARS,
            candidate_requirement: (100 * DOLLARS, 1_000 * DOLLARS), // Minimum value (self_bonded, total_bonded) to be a validator candidate
            ..Default::default()
        };

        config.x_mining_asset = $runtime::XMiningAssetConfig {
            claim_restrictions: vec![(X_BTC, (10, DAYS * 7))],
            mining_power_map: vec![(X_BTC, 400)],
        };

        config.x_spot = $runtime::XSpotConfig {
            trading_pairs: vec![(PCX, X_BTC, 9, 2, 100000, true)],
        };

        config.x_genesis_builder = $runtime::XGenesisBuilderConfig {
            params: crate::genesis::genesis_builder_params(),
            initial_authorities: $initial_authorities
                .iter()
                .map(|i| (i.0).1.clone())
                .collect(),
        };

        config
    }}
}

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
        build_genesis!(
            dev_runtime,
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
        build_genesis!(
            dev_runtime,
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
        build_genesis!(
            dev_runtime,
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

pub fn pre_malan_config() -> Result<MalanChainSpec, String> {
    let wasm_binary =
        dev::WASM_BINARY.ok_or_else(|| "Development wasm binary not available".to_string())?;

    let endowed_balance = 50 * DOLLARS;
    let constructor = move || {
        build_genesis!(
            malan_runtime,
            wasm_binary,
            vec![pre_malan_authorities()],
            hex!("74276b30236e3ffc822c0e5ec0ac8b02933dac11fcefc88733c8a61cdaa45a59").into(),
            genesis_assets(),
            endowed![(
                hex!("74276b30236e3ffc822c0e5ec0ac8b02933dac11fcefc88733c8a61cdaa45a59"),
                endowed_balance
            ),],
            btc_genesis_params(include_str!("res/btc_genesis_params_testnet.json")),
            crate::genesis::bitcoin::local_testnet_trustees(),
        )
    };
    Ok(MalanChainSpec::from_genesis(
        "ChainX Malan Testnet",
        "chainx malan",
        ChainType::Live,
        constructor,
        vec![],
        None,
        Some("pcx"),
        Some(as_properties(NetworkType::Testnet)),
        Default::default(),
    ))
}

#[allow(unused)]
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
