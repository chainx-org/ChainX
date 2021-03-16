// Copyright 2019-2020 ChainX Project Authors. Licensed under GPL-3.0.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use serde_json::json;

use sc_chain_spec::ChainSpecExtension;
use sc_service::{ChainType, Properties};

use sp_authority_discovery::AuthorityId as AuthorityDiscoveryId;
use sp_consensus_babe::AuthorityId as BabeId;
use sp_core::{sr25519, Pair, Public};
use sp_finality_grandpa::AuthorityId as GrandpaId;
use sp_runtime::traits::{IdentifyAccount, Verify};

use pallet_im_online::sr25519::AuthorityId as ImOnlineId;

use chainx_primitives::{AccountId, AssetId, Balance, ReferralId, Signature};
use chainx_runtime::constants::currency::DOLLARS;
use dev_runtime::constants::{currency::DOLLARS as DEV_DOLLARS, time::DAYS as DEV_DAYS};
use xp_assets_registrar::Chain;
use xp_protocol::{NetworkType, PCX, PCX_DECIMALS, X_BTC};
use xpallet_gateway_bitcoin::{BtcParams, BtcTxVerifier};
use xpallet_gateway_bitcoin_v2::types::TradingPrice;
use xpallet_gateway_common::types::TrusteeInfoConfig;

use crate::genesis::assets::{genesis_assets, init_assets, pcx, AssetParams};
use crate::genesis::bitcoin::{btc_genesis_params, BtcGenesisParams, BtcTrusteeParams};

use chainx_runtime as chainx;
use dev_runtime as dev;
use malan_runtime as malan;
use testnet_runtime as testnet;

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
}

/// The `ChainSpec` parameterised for the chainx mainnet runtime.
pub type ChainXChainSpec = sc_service::GenericChainSpec<chainx::GenesisConfig, Extensions>;

pub type TestnetChainSpec = sc_service::GenericChainSpec<testnet::GenesisConfig, Extensions>;
/// The `ChainSpec` parameterised for the chainx testnet runtime.
pub type DevChainSpec = sc_service::GenericChainSpec<dev::GenesisConfig, Extensions>;
/// The `ChainSpec` parameterised for the chainx development runtime.
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

    let endowed_balance = 50 * DEV_DOLLARS;
    let constructor = move || {
        build_genesis(
            wasm_binary,
            vec![authority_keys_from_seed("Alice")],
            get_account_id_from_seed::<sr25519::Public>("Alice"),
            get_account_id_from_seed::<sr25519::Public>("vesting"),
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

    let endowed_balance = 50 * DEV_DOLLARS;
    let constructor = move || {
        build_genesis(
            wasm_binary,
            vec![authority_keys_from_seed("Alice")],
            get_account_id_from_seed::<sr25519::Public>("Alice"),
            get_account_id_from_seed::<sr25519::Public>("vesting"),
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

    let endowed_balance = 50 * DEV_DOLLARS;
    let constructor = move || {
        build_genesis(
            wasm_binary,
            vec![
                authority_keys_from_seed("Alice"),
                authority_keys_from_seed("Bob"),
            ],
            get_account_id_from_seed::<sr25519::Public>("Alice"),
            get_account_id_from_seed::<sr25519::Public>("vesting"),
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
        "Local Testnet",
        "local_testnet",
        ChainType::Local,
        constructor,
        vec![],
        None,
        Some("chainx-local-testnet"),
        Some(as_properties(NetworkType::Testnet)),
        Default::default(),
    ))
}

pub fn mainnet_config() -> Result<ChainXChainSpec, String> {
    ChainXChainSpec::from_json_bytes(&include_bytes!("./res/chainx.json")[..])
}

pub fn testnet_config() -> Result<TestnetChainSpec, String> {
    use hex_literal::hex;
    use sc_telemetry::TelemetryEndpoints;
    use sp_core::crypto::UncheckedInto;

    let wasm_binary =
        testnet::WASM_BINARY.ok_or_else(|| "Development wasm binary not available".to_string())?;
    let endowed_balance = 50 * DEV_DOLLARS;

    let root_key: AccountId =
        hex!["ecdfeb7930b5d59a026e234e7e43f8c0a21a2aa9a20b136f66f55b65a402330b"].into();

    let group1: AuthorityKeysTuple = (
        (
            // 5HgEo72erMXVvriHBQWHeTKvDTjemYSDTawfSNx4jEzcLUgp
            hex!["f846e7a559fddbf68704051c728b99b561fe206f4c8e684dcbf76c97cefe5c5f"].into(),
            b"Validator1".to_vec(),
        ),
        // 5HQBX6RTXZskjTjAcq6q8M6xV9a1oBeKspmguwqDbnYnJEY8
        hex!["ec07f343ea88ac2639c1890adac7473616c138440c939f7ccadf1152f940ae15"].unchecked_into(),
        // 5DyoG8Qm5wCrpDTShTuHQmCae77oj9vdVeLphfEd5CNC5Ji5
        hex!["54ba6ed80edca6c3233dc0a8880c41f0f08ccf9d38812d5ef62af07d1a30f66d"].unchecked_into(),
        // 5Fet39XDWTutm3nYXE5gZ4V31en8k46GTi8MuPua1t6BHRPz
        hex!["9ec550b7c7eecf83a4791f4be18d447f30c5a67359d4e551952f4179aef58b57"].unchecked_into(),
        // 5Ew3YegPqSYdnepKox6ZF2Xd6yqJuzCL6oedxVoRA76yJWRR
        hex!["7edd04710c4d69cae9b21df79fa3b445b513fe135373bb25a6b794903b741358"].unchecked_into(),
    );

    let group2: AuthorityKeysTuple = (
        (
            // 5CFmnqTfRBbTWrUTe2UiJWzdbpdCRL9xLgXZBA9mL5U9pHFm
            hex!["0870f1350d5f139b5adaec90c812b22fb5d03c43250631511a1ca0fbbfe0102e"].into(),
            b"Validator2".to_vec(),
        ),
        // 5FHufR5mRkrs4Y1WXHr8vFq974ffURp9z7N563KxyfZjesz6
        hex!["8ec6a213a4cbb23dd4f0c3875a34f6e51a445b2ddc290056425b3f2380428e1f"].unchecked_into(),
        // 5FZKz5gNfCDiRNReyTY3RfZ1JZXdewSrQy2Ecf2BVi3M7obg
        hex!["9a89304fb713266fcbbb5359d5dfa6474e87d41fbc08147b441b1009442d1ae6"].unchecked_into(),
        // 5HYW4dBmrvnVg94rvydDvVDbvBkuHs1qySKN64pTKmDHvMme
        hex!["f260563f74e4ee46f25fc81250b15082ef1d5ac07a843eaaa0f3cb661500f962"].unchecked_into(),
        // 5E4gdBEfbcpe2aKx7Qq3x7uC9DwdCah2TbgfceM9UFeJxG1z
        hex!["5874514c0563d4990916293e0059dfb2379d657e8df40936f1da24c7af8b7c64"].unchecked_into(),
    );

    let initial_authorities: Vec<AuthorityKeysTuple> = vec![group1, group2];

    let mut endowed = BTreeMap::new();
    let endowed_info = initial_authorities
        .iter()
        .map(|i| ((i.0).0.clone(), endowed_balance))
        .collect::<Vec<_>>();
    endowed.insert(pcx().0, endowed_info);

    let constructor = move || {
        testnet_genesis(
            wasm_binary,
            initial_authorities.clone(),
            root_key.clone(),
            root_key.clone(),
            genesis_assets(),
            endowed.clone(),
            btc_genesis_params(include_str!("res/btc_genesis_params_testnet.json")),
            crate::genesis::bitcoin::local_testnet_trustees(),
        )
    };
    Ok(TestnetChainSpec::from_genesis(
        "ChainX Testnet",
        "chainx_testnet",
        ChainType::Live,
        constructor,
        vec![
            // "/dns/p2p.testnet-1.chainx.org/tcp/30333/p2p/12D3KooWQq7h1cqwRqFaRnp7LxcWmBAzJtizS4uckJrxyK5KHron".to_string().try_into().expect("must be valid bootnode"),
            // "/dns/p2p.testnet-2.chainx.org/tcp/30334/p2p/12D3KooWNKCPciz7iAJ6DBqSygsfzHCVdoMCMWoBgo1EgHMrTpDN".to_string().try_into().expect("must be valid bootnode"),
            // "/dns/p2p.testnet-3.chainx.org/tcp/30335/p2p/12D3KooWLuxACVFoeddQ4ja68C7Y4qNrXtpBC9gx7akRPacnvoJe".to_string().try_into().expect("must be valid bootnode"),
        ],
        Some(
            TelemetryEndpoints::new(vec![(CHAINX_TELEMETRY_URL.to_string(), 0)])
                .expect("Testnet telemetry url is valid; qed"),
        ),
        Some("chainx-testnet"),
        Some(as_properties(NetworkType::Testnet)),
        Default::default(),
    ))
}

pub fn malan_config() -> Result<MalanChainSpec, String> {
    MalanChainSpec::from_json_bytes(&include_bytes!("./res/malan.json")[..])
}

fn dev_session_keys(
    babe: BabeId,
    grandpa: GrandpaId,
    im_online: ImOnlineId,
    authority_discovery: AuthorityDiscoveryId,
) -> dev::SessionKeys {
    dev::SessionKeys {
        grandpa,
        babe,
        im_online,
        authority_discovery,
    }
}

fn testnet_session_keys(
    babe: BabeId,
    grandpa: GrandpaId,
    im_online: ImOnlineId,
    authority_discovery: AuthorityDiscoveryId,
) -> testnet::SessionKeys {
    testnet::SessionKeys {
        grandpa,
        babe,
        im_online,
        authority_discovery,
    }
}

fn build_genesis(
    wasm_binary: &[u8],
    initial_authorities: Vec<AuthorityKeysTuple>,
    root_key: AccountId,
    vesting_account: AccountId,
    assets: Vec<AssetParams>,
    endowed: BTreeMap<AssetId, Vec<(AccountId, Balance)>>,
    bitcoin: BtcGenesisParams,
    trustees: Vec<(Chain, TrusteeInfoConfig, Vec<BtcTrusteeParams>)>,
) -> dev::GenesisConfig {
    const ENDOWMENT: Balance = 10_000_000 * DEV_DOLLARS;
    const STASH: Balance = 100 * DEV_DOLLARS;
    const STAKING_LOCKED: Balance = 1_000 * DEV_DOLLARS;
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

    let mut initial_authorities_endowed = Balance::default();
    let validators = initial_authorities
        .clone()
        .into_iter()
        .map(|((validator, referral), _, _, _, _)| {
            initial_authorities_endowed += STAKING_LOCKED;
            (validator, referral, STAKING_LOCKED)
        })
        .collect::<Vec<_>>();
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
        frame_system: Some(dev::SystemConfig {
            code: wasm_binary.to_vec(),
            changes_trie_config: Default::default(),
        }),
        pallet_babe: Some(Default::default()),
        pallet_grandpa: Some(dev::GrandpaConfig {
            authorities: vec![],
        }),
        pallet_collective_Instance1: Some(dev::CouncilConfig::default()),
        pallet_collective_Instance2: Some(dev::TechnicalCommitteeConfig {
            members: tech_comm_members,
            phantom: Default::default(),
        }),
        pallet_membership_Instance1: Some(Default::default()),
        pallet_democracy: Some(dev::DemocracyConfig::default()),
        pallet_treasury: Some(Default::default()),
        pallet_elections_phragmen: Some(dev::ElectionsConfig {
            members: phragmen_members,
        }),
        pallet_im_online: Some(dev::ImOnlineConfig { keys: vec![] }),
        pallet_authority_discovery: Some(dev::AuthorityDiscoveryConfig { keys: vec![] }),
        pallet_session: Some(dev::SessionConfig {
            keys: initial_authorities
                .iter()
                .map(|x| {
                    (
                        (x.0).0.clone(),
                        (x.0).0.clone(),
                        dev_session_keys(x.1.clone(), x.2.clone(), x.3.clone(), x.4.clone()),
                    )
                })
                .collect::<Vec<_>>(),
        }),
        pallet_balances: Some(dev::BalancesConfig { balances }),
        pallet_indices: Some(dev::IndicesConfig { indices: vec![] }),
        pallet_sudo: Some(dev::SudoConfig { key: root_key }),
        xpallet_system: Some(dev::XSystemConfig {
            network_props: NetworkType::Testnet,
        }),
        xpallet_assets_registrar: Some(dev::XAssetsRegistrarConfig { assets }),
        xpallet_assets: Some(dev::XAssetsConfig {
            assets_restrictions,
            endowed: assets_endowed,
        }),
        xpallet_gateway_common: Some(dev::XGatewayCommonConfig { trustees }),
        xpallet_gateway_bitcoin: Some(dev::XGatewayBitcoinConfig {
            genesis_trustees: btc_genesis_trustees,
            network_id: bitcoin.network,
            confirmation_number: bitcoin.confirmation_number,
            genesis_hash: bitcoin.hash(),
            genesis_info: (bitcoin.header(), bitcoin.height),
            params_info: BtcParams::new(
                486604799,            // max_bits
                2 * 60 * 60,          // block_max_future
                2 * 7 * 24 * 60 * 60, // target_timespan_seconds
                10 * 60,              // target_spacing_seconds
                4,                    // retargeting_factor
            ), // retargeting_factor
            btc_withdrawal_fee: 500000,
            max_withdrawal_count: 100,
            verifier: BtcTxVerifier::Recover,
        }),
        xpallet_gateway_bitcoin_v2_pallet: Some(dev::XGatewayBitcoinV2Config {
            exchange_rate: TradingPrice {
                price: 1,
                decimal: 3,
            },
            oracle_accounts: Default::default(),
            liquidator_id: get_account_id_from_seed::<sr25519::Public>("liquidator"),
            issue_griefing_fee: 10,
            redeem_fee: 0u32.into(),
        }),
        xpallet_mining_staking: Some(dev::XStakingConfig {
            validators,
            validator_count: 50,
            sessions_per_era: 12,
            vesting_account,
            glob_dist_ratio: (12, 88), // (Treasury, X-type Asset and Staking) = (12, 88)
            mining_ratio: (10, 90),    // (Asset Mining, Staking) = (10, 90)
            minimum_penalty: 2 * DOLLARS,
            ..Default::default()
        }),
        xpallet_mining_asset: Some(dev::XMiningAssetConfig {
            claim_restrictions: vec![(X_BTC, (10, DEV_DAYS * 7))],
            mining_power_map: vec![(X_BTC, 400)],
        }),
        xpallet_dex_spot: Some(dev::XSpotConfig {
            trading_pairs: vec![(PCX, X_BTC, 9, 2, 100000, true)],
        }),
        xpallet_genesis_builder: Some(dev::XGenesisBuilderConfig {
            params: crate::genesis::genesis_builder_params(),
            initial_authorities_endowed,
            root_endowed: 0,
        }),
    }
}

fn testnet_genesis(
    wasm_binary: &[u8],
    initial_authorities: Vec<AuthorityKeysTuple>,
    root_key: AccountId,
    vesting_account: AccountId,
    assets: Vec<AssetParams>,
    endowed: BTreeMap<AssetId, Vec<(AccountId, Balance)>>,
    bitcoin: BtcGenesisParams,
    trustees: Vec<(Chain, TrusteeInfoConfig, Vec<BtcTrusteeParams>)>,
) -> testnet::GenesisConfig {
    const ENDOWMENT: Balance = 10_000_000 * DEV_DOLLARS;
    const STASH: Balance = 100 * DEV_DOLLARS;
    const STAKING_LOCKED: Balance = 1_000 * DEV_DOLLARS;
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

    let mut initial_authorities_endowed = Balance::default();
    let validators = initial_authorities
        .clone()
        .into_iter()
        .map(|((validator, referral), _, _, _, _)| {
            initial_authorities_endowed += STAKING_LOCKED;
            (validator, referral, STAKING_LOCKED)
        })
        .collect::<Vec<_>>();
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

    testnet::GenesisConfig {
        frame_system: Some(testnet::SystemConfig {
            code: wasm_binary.to_vec(),
            changes_trie_config: Default::default(),
        }),
        pallet_babe: Some(Default::default()),
        pallet_grandpa: Some(testnet::GrandpaConfig {
            authorities: vec![],
        }),
        pallet_collective_Instance1: Some(testnet::CouncilConfig::default()),
        pallet_collective_Instance2: Some(testnet::TechnicalCommitteeConfig {
            members: tech_comm_members,
            phantom: Default::default(),
        }),
        pallet_membership_Instance1: Some(Default::default()),
        pallet_democracy: Some(testnet::DemocracyConfig::default()),
        pallet_treasury: Some(Default::default()),
        pallet_elections_phragmen: Some(testnet::ElectionsConfig {
            members: phragmen_members,
        }),
        pallet_im_online: Some(testnet::ImOnlineConfig { keys: vec![] }),
        pallet_authority_discovery: Some(testnet::AuthorityDiscoveryConfig { keys: vec![] }),
        pallet_session: Some(testnet::SessionConfig {
            keys: initial_authorities
                .iter()
                .map(|x| {
                    (
                        (x.0).0.clone(),
                        (x.0).0.clone(),
                        testnet_session_keys(x.1.clone(), x.2.clone(), x.3.clone(), x.4.clone()),
                    )
                })
                .collect::<Vec<_>>(),
        }),
        pallet_balances: Some(testnet::BalancesConfig { balances }),
        pallet_indices: Some(testnet::IndicesConfig { indices: vec![] }),
        pallet_sudo: Some(testnet::SudoConfig { key: root_key }),
        xpallet_system: Some(testnet::XSystemConfig {
            network_props: NetworkType::Testnet,
        }),
        xpallet_assets_registrar: Some(testnet::XAssetsRegistrarConfig { assets }),
        xpallet_assets: Some(testnet::XAssetsConfig {
            assets_restrictions,
            endowed: assets_endowed,
        }),
        xpallet_gateway_common: Some(testnet::XGatewayCommonConfig { trustees }),
        xpallet_gateway_bitcoin: Some(testnet::XGatewayBitcoinConfig {
            genesis_trustees: btc_genesis_trustees,
            network_id: bitcoin.network,
            confirmation_number: bitcoin.confirmation_number,
            genesis_hash: bitcoin.hash(),
            genesis_info: (bitcoin.header(), bitcoin.height),
            params_info: BtcParams::new(
                486604799,            // max_bits
                2 * 60 * 60,          // block_max_future
                2 * 7 * 24 * 60 * 60, // target_timespan_seconds
                10 * 60,              // target_spacing_seconds
                4,                    // retargeting_factor
            ), // retargeting_factor
            btc_withdrawal_fee: 500000,
            max_withdrawal_count: 100,
            verifier: BtcTxVerifier::Recover,
        }),
        xpallet_gateway_bitcoin_v2_pallet: Some(testnet::XGatewayBitcoinV2Config {
            exchange_rate: TradingPrice {
                price: 1,
                decimal: 3,
            },
            oracle_accounts: Default::default(),
            liquidator_id: get_account_id_from_seed::<sr25519::Public>("liquidator"),
            issue_griefing_fee: 10,
            redeem_fee: 0u32.into(),
        }),
        xpallet_mining_staking: Some(testnet::XStakingConfig {
            validators,
            validator_count: 50,
            sessions_per_era: 12,
            vesting_account,
            glob_dist_ratio: (12, 88), // (Treasury, X-type Asset and Staking) = (12, 88)
            mining_ratio: (10, 90),    // (Asset Mining, Staking) = (10, 90)
            minimum_penalty: 2 * DOLLARS,
            ..Default::default()
        }),
        xpallet_mining_asset: Some(testnet::XMiningAssetConfig {
            claim_restrictions: vec![(X_BTC, (10, DEV_DAYS * 7))],
            mining_power_map: vec![(X_BTC, 400)],
        }),
        xpallet_dex_spot: Some(testnet::XSpotConfig {
            trading_pairs: vec![(PCX, X_BTC, 9, 2, 100000, true)],
        }),
        xpallet_genesis_builder: Some(testnet::XGenesisBuilderConfig {
            params: crate::genesis::genesis_builder_params(),
            initial_authorities_endowed,
            root_endowed: 0,
        }),
    }
}
