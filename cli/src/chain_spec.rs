// Copyright 2019-2020 ChainX Project Authors. Licensed under GPL-3.0.

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

use chainx_primitives::{AccountId, AssetId, Balance, ReferralId, Signature};
use chainx_runtime::constants::currency::DOLLARS;
use dev_runtime::constants::{currency::DOLLARS as DEV_DOLLARS, time::DAYS as DEV_DAYS};
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
}

/// The `ChainSpec` parameterised for the chainx mainnet runtime.
pub type ChainXChainSpec = sc_service::GenericChainSpec<chainx::GenesisConfig, Extensions>;
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
    // ChainXChainSpec::from_json_bytes(&include_bytes!("./res/chainx.json")[..])
    build_mainnet_config()
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

fn build_genesis(
    wasm_binary: &[u8],
    initial_authorities: Vec<AuthorityKeysTuple>,
    root_key: AccountId,
    assets: Vec<AssetParams>,
    endowed: BTreeMap<AssetId, Vec<(AccountId, Balance)>>,
    bitcoin: BtcGenesisParams,
    trustees: Vec<(Chain, TrusteeInfoConfig, Vec<BtcTrusteeParams>)>,
) -> dev::GenesisConfig {
    const ENDOWMENT: Balance = 10_000_000 * DEV_DOLLARS;
    const STASH: Balance = 100 * DEV_DOLLARS;
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
        xpallet_mining_staking: Some(dev::XStakingConfig {
            validator_count: 50,
            sessions_per_era: 12,
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
        }),
    }
}

macro_rules! bootnodes {
    ( $( $bootnode:expr, )* ) => {
        vec![
            $($bootnode.to_string().try_into().expect("The bootnode is invalid"),)*
        ]
    }
}

pub fn build_mainnet_config() -> Result<ChainXChainSpec, String> {
    let wasm_binary =
        chainx::WASM_BINARY.ok_or("Development wasm binary not available".to_string())?;

    let initial_authorities: Vec<AuthorityKeysTuple> = vec![
        (
            (
                // 5QkGjd5rsczm4qVgVpzRSdBe2SrhLvKrPrYFeAAtw4qbdRPh
                hex!["31000d19a3e9607d92b3697a661a6e7e9fbb65361846680d968cfc86c9561103"].into(),
                b"Hotbit".to_vec(),
            ),
            hex!["8aca6544728dfcfabb2579527b8e24675450bfb52a8a53b6b24d27366faf0b16"]
                .unchecked_into(),
            hex!["233869639205a2469ab45e7c8059206e1089d980d2309a6cd9ff581d7b4e4961"]
                .unchecked_into(),
            hex!["4801006d94d3612b582f132295c7864dcce0a5de267cce8dd0fd88524091e129"]
                .unchecked_into(),
            hex!["f4f6124a5abf4fcd95ef5920585733f88b66754325fe4a4a88608b884dd0a178"]
                .unchecked_into(),
        ),
        (
            (
                // 5StNFoeSmLXr7SfDuwJqHR5CyKV2o4BD2yU36GGay3GVFhtt
                hex!["8fa51087d1a7327c90da45f8e369e31037606427f07ef77007a41036227a3a5b"].into(),
                b"Web3".to_vec(),
            ),
            hex!["464a78ec0b6e44452c18f1fd363d32cd5e591d18d0e0a2fbb6935b246f995409"]
                .unchecked_into(),
            hex!["fb41135c2b072cb02ee8bb6bc7a1e0db101a8ff2a01b30ed8b2aa2785bd51d1e"]
                .unchecked_into(),
            hex!["1e0fd63785309233efc25966a329315943d713ed490561002065b5f869bad315"]
                .unchecked_into(),
            hex!["9cccb586deffae2a9145f5fda338bc687f5e7aec9235ad099f00b901ab338366"]
                .unchecked_into(),
        ),
        (
            (
                // 5RaxFQc7E4ACr4FVoHj2SMA6MGMqT8Ck9mDV5byGZtPPUw8f
                hex!["5620d851190bda61acb08f1a06defcdd5a3c7da3c33819643e7d6ee232ee78bf"].into(),
                b"ChainY".to_vec(),
            ),
            hex!["1ac6ee27ae8f7a5fb2cd583a47db9ece87cd7fb293cfaeb89ad7d403eff15c31"]
                .unchecked_into(),
            hex!["5151c37e77ed9b89d50d33da305f9c7f11fce661d6e9e423805ce7b54be3344c"]
                .unchecked_into(),
            hex!["ea3fd82e2ca5a5f62f3e427c4bd1014fd4fe417ff949ccf213f671d3ec43f825"]
                .unchecked_into(),
            hex!["06186f8d4d12563b7376083eceacd0308c881ca791fb72d2fa081bfd3c66dd34"]
                .unchecked_into(),
        ),
        (
            (
                // 5RXaXGcz84KQ1XZeAMbjbexXRPNAaPdt3st5QDG3H6VegwD6
                hex!["538df88774a48e4ec759cfb3d25f12e343d8048a4bfa92643070b73cbb4be843"].into(),
                b"BiHODL".to_vec(),
            ),
            hex!["840ce40e301797fedfe99183f66f20d40077fae376865c26d231883e793e564e"]
                .unchecked_into(),
            hex!["83b45e42e2fb2439c62e8d986f97f80fa98a7d0a0f7ed6296a858245306681d1"]
                .unchecked_into(),
            hex!["706bd686bf48ebe137f0caddd0505ac07063815cfe29451c460991b2f7ae9520"]
                .unchecked_into(),
            hex!["0e973e483a3a978f9c4e0c60f768fcc744639faa4104541a7bef159e7f728909"]
                .unchecked_into(),
        ),
        (
            (
                // 5RAZf8UHcbS5RBRpP9zptQJm84tpfnxcJ64ctSyxNJeLLxtq
                hex!["4386e83d66fbdf9ebe72af81d453f41fb8f877287f04823665fc81b58cab6e6b"].into(),
                b"XPool".to_vec(),
            ),
            hex!["660098bd0806bb14799ce6dae47bcf8fe0d84b82dcb866191f3b3a591e01ff51"]
                .unchecked_into(),
            hex!["08307f0c94748fde0eb634dca36a69e511eb5860bc6355b7ac4ee85c8c8d3ee8"]
                .unchecked_into(),
            hex!["b84a635d7c6e3cb23eecccc4a0580214a5dc030ee27151e2bf16b911dbad685b"]
                .unchecked_into(),
            hex!["ee88d728d4fbe4f4532fe87879d378968f9350207d4c31a5e45abcaff1efea15"]
                .unchecked_into(),
        ),
    ];
    let constructor = move || {
        mainnet_genesis(
            &wasm_binary[..],
            initial_authorities.clone(),
            genesis_assets(),
            btc_genesis_params(include_str!("res/btc_genesis_params_mainnet.json")), //TODO: update mainnet btc genesis
            crate::genesis::bitcoin::mainnet_trustees(),
        )
    };
    Ok(ChainXChainSpec::from_genesis(
        "ChainX",
        "chainx",
        ChainType::Live,
        constructor,
        // TODO: make sure the bootnodes
        bootnodes![
            "/dns/p2p.1.chainx.org/tcp/20222/p2p/12D3KooWMMGD6eyLDgoTPnmGrawn9gkjtsZGLACJXqVCUbe6R6bD",
            "/dns/p2p.2.chainx.org/tcp/20222/p2p/12D3KooWC1tFLBFVw47S2nfD7Nzhg5hBMUvsnz4nqpr82zfTYWaH",
            "/dns/p2p.3.chainx.org/tcp/20222/p2p/12D3KooWPthFY8xDDyM5X9PWZwNfioqP5EShiTKyVv5899H22WBT",
        ],
        Some(
            TelemetryEndpoints::new(vec![
                (CHAINX_TELEMETRY_URL.to_string(), 0),
                (POLKADOT_TELEMETRY_URL.to_string(), 0),
            ])
            .expect("ChainX telemetry url is valid; qed"),
        ),
        Some("pcx1"),
        Some(as_properties(NetworkType::Mainnet)),
        Default::default(),
    ))
}

fn chainx_session_keys(
    babe: BabeId,
    grandpa: GrandpaId,
    im_online: ImOnlineId,
    authority_discovery: AuthorityDiscoveryId,
) -> chainx::SessionKeys {
    chainx::SessionKeys {
        grandpa,
        babe,
        im_online,
        authority_discovery,
    }
}

fn mainnet_genesis(
    wasm_binary: &[u8],
    initial_authorities: Vec<AuthorityKeysTuple>,
    assets: Vec<AssetParams>,
    bitcoin: BtcGenesisParams,
    trustees: Vec<(Chain, TrusteeInfoConfig, Vec<BtcTrusteeParams>)>,
) -> chainx::GenesisConfig {
    use chainx_runtime::constants::time::DAYS;

    let (assets, assets_restrictions) = init_assets(assets);
    // TODO: update the tech commitees
    let tech_comm_members: Vec<AccountId> = vec![
        // 5C7VzhPqJsLXcyJmX71ZEt7GdkAMTxHNPwh6BSb8thgBbQU1
        hex!["0221ce7c4a0b771faaf0bbae23c3a1965348cb5257611313a73c3d4a53599509"].into(),
        // 5D7F1AJoDwuCvZZKEggeGk2brxYty9mkamUcFHyshYBnbWs3
        hex!["2e2b928d39b7a9c8688509927e17031001fab604557db093ead5069474e0584e"].into(),
        // 5HG5CswZ6X39BYqt8Dc8e4Cn2HieGnnUiG39ddGn2oq5G36W
        hex!["e5d8bb656b124beb40990ef9346c441f888981ec7e0d4c55c9c72c176aec5290"].into(),
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

    chainx::GenesisConfig {
        frame_system: Some(chainx::SystemConfig {
            code: wasm_binary.to_vec(),
            changes_trie_config: Default::default(),
        }),
        pallet_babe: Some(Default::default()),
        pallet_grandpa: Some(chainx::GrandpaConfig {
            authorities: vec![],
        }),
        pallet_collective_Instance1: Some(chainx::CouncilConfig::default()),
        pallet_collective_Instance2: Some(chainx::TechnicalCommitteeConfig {
            members: tech_comm_members,
            phantom: Default::default(),
        }),
        pallet_membership_Instance1: Some(Default::default()),
        pallet_democracy: Some(chainx::DemocracyConfig::default()),
        pallet_treasury: Some(Default::default()),
        pallet_elections_phragmen: Some(chainx::ElectionsConfig::default()),
        pallet_im_online: Some(chainx::ImOnlineConfig { keys: vec![] }),
        pallet_authority_discovery: Some(chainx::AuthorityDiscoveryConfig { keys: vec![] }),
        pallet_session: Some(chainx::SessionConfig {
            keys: initial_authorities
                .iter()
                .map(|x| {
                    (
                        (x.0).0.clone(),
                        (x.0).0.clone(),
                        chainx_session_keys(x.1.clone(), x.2.clone(), x.3.clone(), x.4.clone()),
                    )
                })
                .collect::<Vec<_>>(),
        }),
        pallet_balances: Some(chainx::BalancesConfig::default()),
        pallet_indices: Some(chainx::IndicesConfig { indices: vec![] }),
        xpallet_system: Some(chainx::XSystemConfig {
            network_props: NetworkType::Mainnet,
        }),
        xpallet_assets_registrar: Some(chainx::XAssetsRegistrarConfig { assets }),
        xpallet_assets: Some(chainx::XAssetsConfig {
            assets_restrictions,
            endowed: Default::default(),
        }),
        xpallet_gateway_common: Some(chainx::XGatewayCommonConfig { trustees }),
        xpallet_gateway_bitcoin: Some(chainx::XGatewayBitcoinConfig {
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
        xpallet_mining_staking: Some(chainx::XStakingConfig {
            validator_count: 40, // TODO: figure it out
            sessions_per_era: 12,
            glob_dist_ratio: (12, 88), // (Treasury, X-type Asset and Staking) = (12, 88)
            mining_ratio: (10, 90),    // (Asset Mining, Staking) = (10, 90)
            minimum_penalty: 100 * DOLLARS,
            candidate_requirement: (100 * DOLLARS, 1_000 * DOLLARS), // Minimum value (self_bonded, total_bonded) to be a validator candidate
            ..Default::default()
        }),
        xpallet_mining_asset: Some(chainx::XMiningAssetConfig {
            claim_restrictions: vec![(X_BTC, (10, DAYS * 7))],
            mining_power_map: vec![(X_BTC, 400)],
        }),
        xpallet_dex_spot: Some(chainx::XSpotConfig {
            trading_pairs: vec![(PCX, X_BTC, 9, 2, 100000, true)],
        }),
        xpallet_genesis_builder: Some(chainx::XGenesisBuilderConfig {
            params: crate::genesis::genesis_builder_params(), // TODO initial authorities should not be chilled.
        }),
    }
}
