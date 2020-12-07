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

use chainx_dev_runtime::constants::{currency::DOLLARS as DEV_DOLLARS, time::DAYS as DEV_DAYS};
use chainx_primitives::{AccountId, AssetId, Balance, ReferralId, Signature};
use chainx_runtime::constants::currency::DOLLARS;
use xp_assets_registrar::Chain;
use xp_protocol::{NetworkType, PCX, PCX_DECIMALS, X_BTC};
use xpallet_gateway_bitcoin::{BtcParams, BtcTxVerifier};
use xpallet_gateway_common::types::TrusteeInfoConfig;

use crate::genesis::assets::{genesis_assets, init_assets, pcx, AssetParams};
use crate::genesis::bitcoin::{btc_genesis_params, BtcGenesisParams, BtcTrusteeParams};

use chainx_dev_runtime as chainx_dev;
use chainx_runtime as chainx;

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

/// Specialized `ChainSpec`. This is a specialization of the general Substrate ChainSpec type.
// pub type ChainSpec = sc_service::GenericChainSpec<GenesisConfig, Extensions>;

pub type ChainXChainSpec = sc_service::GenericChainSpec<chainx::GenesisConfig, Extensions>;
pub type ChainXDevChainSpec = sc_service::GenericChainSpec<chainx_dev::GenesisConfig, Extensions>;

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

pub fn development_config() -> Result<ChainXDevChainSpec, String> {
    let wasm_binary =
        chainx_dev::WASM_BINARY.ok_or("Development wasm binary not available".to_string())?;

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
    Ok(ChainXDevChainSpec::from_genesis(
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
pub fn benchmarks_config() -> Result<ChainXDevChainSpec, String> {
    let wasm_binary =
        chainx_dev::WASM_BINARY.ok_or("Development wasm binary not available".to_string())?;

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
    Ok(ChainXDevChainSpec::from_genesis(
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

pub fn local_testnet_config() -> Result<ChainXDevChainSpec, String> {
    let wasm_binary = chainx_dev::WASM_BINARY
        .ok_or_else(|| "Development wasm binary not available".to_string())?;

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
    Ok(ChainXDevChainSpec::from_genesis(
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

pub fn testnet_config() -> Result<ChainXChainSpec, String> {
    let wasm_binary =
        chainx::WASM_BINARY.ok_or("Development wasm binary not available".to_string())?;

    // 5EWtScne4zWsGaP4gVo8DmLpChVx3MzoQTpKJCEdBTYDA1Dy
    let root_key: AccountId =
        hex!["6c707b1690a6b0e01b5dea252fe1887930a5afc0ec203f96705331749c37ae4a"].into();

    // 5HGZzRCfvLM7LSdkPZF5SzD4tj9BKvCTQuGkJd1jedrcCKFc
    let vesting_key: AccountId =
        hex!["e639a1a8ff3bd1fe15faa922ef2b772b9ee1c8d9cdc63ad36af12ab5ca155d4a"].into();

    let initial_authorities: Vec<AuthorityKeysTuple> = vec![(
        (
            // 5EvXt55kDmAXbBPqrzvNZcbE6bvZ8eWBThxJptoDkwAAyEkw
            hex!["7e7927d030d89585cd66f0d44313de41f4c697da387159786f8b3ed5cd081d4f"].into(),
            b"Validator5".to_vec(),
        ),
        // 5D83WrH4h4rPFxe4m4xGMuC8XuR9jqWHggHBriZQELJ3JneN
        hex!["2ec8253a23695069619df42213106402cffb217bb02c653c11e3435eb047e60d"].unchecked_into(),
        // 5GpHoku58fumTfn9pQZxKFxASvMw6JNTqmFDQw92g7LV1gwj
        hex!["d22ec57d5cdb6f80f0df82590f9999b88e936e9f8c93d9c05cd87dba1b4567ae"].unchecked_into(),
        // 5HCBmPYr7AsXDp4VLu7qh6HRExjy2Nx33YLHqLeQ6i7yFjHt
        hex!["e2e1d5c8eb42aa6b37f71cbdc8e73b67b385e7841d98bbaef3492252a4f3e605"].unchecked_into(),
        // 5HQYBwyf2787MCMhZNBEpswHJcJnXVVNxjrdTa6cVjUf29jy
        hex!["ec4d8806b85969a29214c00ae70b5d239dc65daebf2ea4a43fd47a77e16d9c7c"].unchecked_into(),
    )];

    let constructor = move || {
        // TODO: use mainnet_genesis() or create a new testnet_genesis()?
        testnet_genesis(
            &wasm_binary[..],
            initial_authorities.clone(),
            root_key.clone(),
            vesting_key.clone(),
            genesis_assets(),
            btc_genesis_params(include_str!("res/btc_genesis_params_testnet.json")),
            crate::genesis::bitcoin::mainnet_trustees(),
        )
    };

    Ok(ChainXChainSpec::from_genesis(
        "ChainX TC0",
        "chainx_tc0",
        ChainType::Live,
        constructor,
        bootnodes![
            "/dns/p2p.3.chainx.org/tcp/20223/p2p/12D3KooWRcmKCa1Uo54UNV6umzvVnWAx7TZNFibbZfP87zqPs1DP",
        ],
        Some(
            TelemetryEndpoints::new(vec![
                (CHAINX_TELEMETRY_URL.to_string(), 0),
                (POLKADOT_TELEMETRY_URL.to_string(), 0),
            ])
            .expect("ChainX telemetry url is valid; qed"),
        ),
        Some("pcx-tc0"),
        Some(as_properties(NetworkType::Testnet)),
        Default::default(),
    ))
}

pub fn private_testnet_config() -> Result<ChainXChainSpec, String> {
    let wasm_binary =
        chainx::WASM_BINARY.ok_or("Development wasm binary not available".to_string())?;

    // 5EWtScne4zWsGaP4gVo8DmLpChVx3MzoQTpKJCEdBTYDA1Dy
    let root_key: AccountId =
        hex!["6c707b1690a6b0e01b5dea252fe1887930a5afc0ec203f96705331749c37ae4a"].into();

    // 5HGZzRCfvLM7LSdkPZF5SzD4tj9BKvCTQuGkJd1jedrcCKFc
    let vesting_key: AccountId =
        hex!["e639a1a8ff3bd1fe15faa922ef2b772b9ee1c8d9cdc63ad36af12ab5ca155d4a"].into();

    let initial_authorities: Vec<AuthorityKeysTuple> = vec![(
        (
            // 5DPgEmPRBhXj8fpsHm3aXrNtjXNVv7MHnUYQVLFhzMyzabaN
            hex!["3ab47230dff92003f6f4f79cf7930cfe3f3fd77eedfea55acfde77223ac1a47a"].into(),
            b"Validator1".to_vec(),
        ),
        // 5EPgwcbLknydnWGmHem3rmwhdjk55e9HfoqjF5A5zRQDkWxj
        hex!["66f30ce2de3f23c2383c0ecea1e2a2e0520c18931d3a9bca64be78e3f9f7b62f"].unchecked_into(),
        // 5FTYv429Xnkmn7HR4FCWPxnTpvveqpqjVKng3H6ypaBpXeVN
        hex!["96213e8f2f57edfec52b6ffc260d4e8257e8addabe7797fa197f0aa8f6b7e748"].unchecked_into(),
        // 5G4KNiQahHTY1LSafhfZnwGLtAnB39TMuc9nhpWMerE98RYQ
        hex!["b0a540b56805d5b14df2787360728d72197bec577601ad49e274d3914f8b407a"].unchecked_into(),
        // 5HNR82T2juJY6hx48ExdwUBQcpHQvHc5QPCcEzusdHMtLfMZ
        hex!["eaaf3faeb72a15004fb5f9c68de188310b8cf3fbbbd8e8eb4db8aa9e95c40966"].unchecked_into(),
    )];

    let constructor = move || {
        // TODO: use mainnet_genesis() or create a new testnet_genesis()?
        testnet_genesis(
            &wasm_binary[..],
            initial_authorities.clone(),
            root_key.clone(),
            vesting_key.clone(),
            genesis_assets(),
            btc_genesis_params(include_str!("res/btc_genesis_params_testnet.json")),
            crate::genesis::bitcoin::local_testnet_trustees(),
        )
    };

    Ok(ChainXChainSpec::from_genesis(
        "ChainX PTC0",
        "chainx_ptc0",
        ChainType::Live,
        constructor,
        bootnodes![],
        Some(
            TelemetryEndpoints::new(vec![
                (CHAINX_TELEMETRY_URL.to_string(), 0),
                (POLKADOT_TELEMETRY_URL.to_string(), 0),
            ])
            .expect("ChainX telemetry url is valid; qed"),
        ),
        Some("pcx-ptc0"),
        Some(as_properties(NetworkType::Testnet)),
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

fn chainx_dev_session_keys(
    babe: BabeId,
    grandpa: GrandpaId,
    im_online: ImOnlineId,
    authority_discovery: AuthorityDiscoveryId,
) -> chainx_dev::SessionKeys {
    chainx_dev::SessionKeys {
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
) -> chainx_dev::GenesisConfig {
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

    chainx_dev::GenesisConfig {
        frame_system: Some(chainx_dev::SystemConfig {
            code: wasm_binary.to_vec(),
            changes_trie_config: Default::default(),
        }),
        pallet_babe: Some(Default::default()),
        pallet_grandpa: Some(chainx_dev::GrandpaConfig {
            authorities: vec![],
        }),
        pallet_collective_Instance1: Some(chainx_dev::CouncilConfig::default()),
        pallet_collective_Instance2: Some(chainx_dev::TechnicalCommitteeConfig {
            members: tech_comm_members,
            phantom: Default::default(),
        }),
        pallet_membership_Instance1: Some(Default::default()),
        pallet_democracy: Some(chainx_dev::DemocracyConfig::default()),
        pallet_treasury: Some(Default::default()),
        pallet_elections_phragmen: Some(chainx_dev::ElectionsConfig {
            members: phragmen_members,
        }),
        pallet_im_online: Some(chainx_dev::ImOnlineConfig { keys: vec![] }),
        pallet_authority_discovery: Some(chainx_dev::AuthorityDiscoveryConfig { keys: vec![] }),
        pallet_session: Some(chainx_dev::SessionConfig {
            keys: initial_authorities
                .iter()
                .map(|x| {
                    (
                        (x.0).0.clone(),
                        (x.0).0.clone(),
                        chainx_dev_session_keys(x.1.clone(), x.2.clone(), x.3.clone(), x.4.clone()),
                    )
                })
                .collect::<Vec<_>>(),
        }),
        pallet_balances: Some(chainx_dev::BalancesConfig { balances }),
        pallet_indices: Some(chainx_dev::IndicesConfig { indices: vec![] }),
        pallet_sudo: Some(chainx_dev::SudoConfig { key: root_key }),
        xpallet_system: Some(chainx_dev::XSystemConfig {
            network_props: NetworkType::Testnet,
        }),
        xpallet_assets_registrar: Some(chainx_dev::XAssetsRegistrarConfig { assets }),
        xpallet_assets: Some(chainx_dev::XAssetsConfig {
            assets_restrictions,
            endowed: assets_endowed,
        }),
        xpallet_gateway_common: Some(chainx_dev::XGatewayCommonConfig { trustees }),
        xpallet_gateway_bitcoin: Some(chainx_dev::XGatewayBitcoinConfig {
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
        xpallet_mining_staking: Some(chainx_dev::XStakingConfig {
            validators,
            validator_count: 50,
            sessions_per_era: 12,
            vesting_account,
            glob_dist_ratio: (12, 88), // (Treasury, X-type Asset and Staking) = (12, 88)
            mining_ratio: (10, 90),    // (Asset Mining, Staking) = (10, 90)
            minimum_penalty: 2 * DOLLARS,
            ..Default::default()
        }),
        xpallet_mining_asset: Some(chainx_dev::XMiningAssetConfig {
            claim_restrictions: vec![(X_BTC, (10, DEV_DAYS * 7))],
            mining_power_map: vec![(X_BTC, 400)],
        }),
        xpallet_dex_spot: Some(chainx_dev::XSpotConfig {
            trading_pairs: vec![(PCX, X_BTC, 9, 2, 100000, true)],
        }),
        xpallet_genesis_builder: Some(chainx_dev::XGenesisBuilderConfig {
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
    bitcoin: BtcGenesisParams,
    trustees: Vec<(Chain, TrusteeInfoConfig, Vec<BtcTrusteeParams>)>,
) -> chainx::GenesisConfig {
    // 1000 PCX
    const STAKING_LOCKED: Balance = 100_000 * DOLLARS;
    // 100 PCX
    const ROOT_ENDOWED: Balance = 10_000 * DOLLARS;

    let (assets, assets_restrictions) = init_assets(assets);

    let initial_authorities_len = initial_authorities.len();

    let tech_comm_members: Vec<AccountId> = vec![
        // 5C7VzhPqJsLXcyJmX71ZEt7GdkAMTxHNPwh6BSb8thgBbQU1
        hex!["0221ce7c4a0b771faaf0bbae23c3a1965348cb5257611313a73c3d4a53599509"].into(),
        // 5D7F1AJoDwuCvZZKEggeGk2brxYty9mkamUcFHyshYBnbWs3
        hex!["2e2b928d39b7a9c8688509927e17031001fab604557db093ead5069474e0584e"].into(),
        // 5HG5CswZ6X39BYqt8Dc8e4Cn2HieGnnUiG39ddGn2oq5G36W
        hex!["e5d8bb656b124beb40990ef9346c441f888981ec7e0d4c55c9c72c176aec5290"].into(),
    ];

    let mut balances = initial_authorities
        .iter()
        .map(|((validator, _), _, _, _, _)| validator)
        .cloned()
        .map(|validator| (validator, STAKING_LOCKED))
        .collect::<Vec<_>>();

    // 100 PCX to root account for paying the transaction fee.
    balances.push((root_key.clone(), ROOT_ENDOWED));

    let initial_authorities_endowed = initial_authorities_len as Balance * STAKING_LOCKED;

    let validators = initial_authorities
        .clone()
        .into_iter()
        .map(|((validator, referral_id), _, _, _, _)| (validator, referral_id, STAKING_LOCKED))
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
        pallet_balances: Some(chainx::BalancesConfig { balances }),
        pallet_indices: Some(chainx::IndicesConfig { indices: vec![] }),
        pallet_sudo: Some(chainx::SudoConfig { key: root_key }),
        xpallet_system: Some(chainx::XSystemConfig {
            network_props: NetworkType::Testnet,
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
            validators,
            validator_count: initial_authorities_len as u32, // Start mainnet in PoA
            sessions_per_era: 12,
            vesting_account,
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
            params: crate::genesis::genesis_builder_params(),
            root_endowed: ROOT_ENDOWED,
            initial_authorities_endowed,
        }),
    }
}
