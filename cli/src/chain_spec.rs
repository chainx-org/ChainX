// Copyright 2019-2020 ChainX Project Authors. Licensed under GPL-3.0.

use std::collections::BTreeMap;

use hex_literal::hex;
use serde::{Deserialize, Serialize};
use serde_json::json;

use sc_chain_spec::ChainSpecExtension;
use sc_service::{config::TelemetryEndpoints, ChainType, Properties};

use sp_authority_discovery::AuthorityId as AuthorityDiscoveryId;
use sp_consensus_babe::AuthorityId as BabeId;
use sp_core::{crypto::UncheckedInto, sr25519, Pair, Public};
use sp_finality_grandpa::AuthorityId as GrandpaId;
use sp_runtime::traits::{IdentifyAccount, Verify};

use pallet_im_online::sr25519::AuthorityId as ImOnlineId;

use xp_protocol::{PCX, PCX_DECIMALS, X_BTC};

use chainx_runtime::{
    constants::{currency::DOLLARS, time::DAYS},
    AccountId, AssetId, Balance, BtcParams, BtcTxVerifier, Chain, NetworkType, ReferralId,
    SessionKeys, Signature, TrusteeInfoConfig, WASM_BINARY,
};
use chainx_runtime::{
    AuthorityDiscoveryConfig, BalancesConfig, CouncilConfig, DemocracyConfig, ElectionsConfig,
    GenesisConfig, GrandpaConfig, ImOnlineConfig, IndicesConfig, SessionConfig, SudoConfig,
    SystemConfig, TechnicalCommitteeConfig, XAssetsConfig, XAssetsRegistrarConfig,
    XGatewayBitcoinConfig, XGatewayCommonConfig, XGenesisBuilderConfig, XMiningAssetConfig,
    XSpotConfig, XStakingConfig, XSystemConfig,
};

use crate::genesis::assets::{genesis_assets, init_assets, pcx, AssetParams};
use crate::genesis::bitcoin::{btc_genesis_params, BtcGenesisParams, BtcTrusteeParams};

// Note this is the URL for the telemetry server
const POLKADOT_TELEMETRY_URL: &str = "wss://telemetry.polkadot.io/submit/";
const CHAINX_TELEMETRY_URL: &str = "ws://stats.chainx.org:1024/submit/";

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
pub type ChainSpec = sc_service::GenericChainSpec<GenesisConfig, Extensions>;

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

macro_rules! bootnodes {
    ( $( $bootnode:expr, )* ) => {
        vec![
            $($bootnode.to_string().try_into().expect("The bootnode is invalid"),)*
        ]
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

pub fn development_config() -> Result<ChainSpec, String> {
    let wasm_binary = WASM_BINARY.ok_or("Development wasm binary not available".to_string())?;

    let endowed_balance = 50 * DOLLARS;
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
    Ok(ChainSpec::from_genesis(
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
pub fn benchmarks_config() -> Result<ChainSpec, String> {
    let wasm_binary = WASM_BINARY.ok_or("Development wasm binary not available".to_string())?;

    let endowed_balance = 50 * DOLLARS;
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
    Ok(ChainSpec::from_genesis(
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

pub fn local_testnet_config() -> Result<ChainSpec, String> {
    let wasm_binary =
        WASM_BINARY.ok_or_else(|| "Development wasm binary not available".to_string())?;

    let endowed_balance = 50 * DOLLARS;
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
    Ok(ChainSpec::from_genesis(
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

#[allow(unused)]
pub fn mainnet_config() -> Result<ChainSpec, String> {
    let wasm_binary = WASM_BINARY.ok_or("Development wasm binary not available".to_string())?;

    // 5HNeqQYeyqcaBTHHjSbFnEvhCeg6jKcRrV2zeHgXQhvjK8XY
    let root_key: AccountId =
        hex!["eadd6992f5b27027c9424be83d460fcd71550aa8ba3c322ff25548565ca6395d"].into();

    // 5E9upTw5KfKuVa9nA5E9sfiC3S6pToZmm5NKkdWnfaArA1zD
    let vesting_key: AccountId =
        hex!["5c70f62d9ac4bc0a314100d5d3b74d127dbcf8628329ffa799361ae69e104768"].into();

    // export SECRET="YOUR SECRET"
    // cd scripts/genesis/generate_keys.sh && bash generate_keys.sh

    let initial_authorities: Vec<AuthorityKeysTuple> = vec![
        (
            (
                // 5DPgEmPRBhXj8fpsHm3aXrNtjXNVv7MHnUYQVLFhzMyzabaN
                hex!["3ab47230dff92003f6f4f79cf7930cfe3f3fd77eedfea55acfde77223ac1a47a"].into(),
                b"Validator1".to_vec(),
            ),
            // 5EPgwcbLknydnWGmHem3rmwhdjk55e9HfoqjF5A5zRQDkWxj
            hex!["66f30ce2de3f23c2383c0ecea1e2a2e0520c18931d3a9bca64be78e3f9f7b62f"]
                .unchecked_into(),
            // 5FTYv429Xnkmn7HR4FCWPxnTpvveqpqjVKng3H6ypaBpXeVN
            hex!["96213e8f2f57edfec52b6ffc260d4e8257e8addabe7797fa197f0aa8f6b7e748"]
                .unchecked_into(),
            // 5G4KNiQahHTY1LSafhfZnwGLtAnB39TMuc9nhpWMerE98RYQ
            hex!["b0a540b56805d5b14df2787360728d72197bec577601ad49e274d3914f8b407a"]
                .unchecked_into(),
            // 5HNR82T2juJY6hx48ExdwUBQcpHQvHc5QPCcEzusdHMtLfMZ
            hex!["eaaf3faeb72a15004fb5f9c68de188310b8cf3fbbbd8e8eb4db8aa9e95c40966"]
                .unchecked_into(),
        ),
        (
            (
                // 5HmRZ2viuTVEcxD1DgF89DvnHptVsqYyN27DvPwiSMUoiFhB
                hex!["fc3b583310fe9f091e35a8be64ac9508b9d3c088fdcef51b747113fa8fe87f44"].into(),
                b"Validator2".to_vec(),
            ),
            // 5FLeZHeNy7UPyNusqLRT4EUqxBxUVEpSbma6u9By6DzC3o13
            hex!["90dd84e695fd85888d9060ded9a79a7b6bca69206cf10d5366f10c12822c9424"]
                .unchecked_into(),
            // 5Hi3u6dHDtemnL18s6gr6CduTbXEFJg4Jx4LX5YWVc4HAwnC
            hex!["f9a8b6d66d3efbd77b304feea7c142b801cd04422e250d548f063750c890ff1a"]
                .unchecked_into(),
            // 5FbwQEszvNLmcJzbVaSGUccLpri3waR49xFD6Zt3fNd48wqF
            hex!["9c86e6f5ac9ef21d27500d3bfdc9900c17911a4dba1a29ccd32268c208d6c24f"]
                .unchecked_into(),
            // 5ENpkyLe4xNRz1siYybtQnhEfvgJc6zsb2Go5dZLHTSdGZNt
            hex!["664a1e1eccab98fb1f7c1b20c16b1f010ed5a1ec13648b6c57f336ebb44d865a"]
                .unchecked_into(),
        ),
        (
            (
                // 5FEMghsWkqKi8wR7r95jPg6DnGErv9w7AfZF7nMkjUqHy421
                hex!["8c114008f432dd0f10dc74a78d1f6cdef6778bb30c3d5abfcf60f5f1570a6b43"].into(),
                b"Validator3".to_vec(),
            ),
            // 5Grs98EMvxsCrmKMKGCx3xemYcMhAPNF8aa6uRYttWVKJLBg
            hex!["d4257968d6f6775c356c9bac1fcce8d420cd1e03e704dc5c4c5c33b0182c471a"]
                .unchecked_into(),
            // 5EaDXXayhSHc49gXk7i7frh8tFwmtWAL2Sb9zjtaGhBCbNib
            hex!["6efa76335d099850aa2ef10a15f20e9d305f7eec9b0079f8196c7aa9deff07ed"]
                .unchecked_into(),
            // 5GBQRvZ4FXZi1ttKCMHQN99X4qtegoQ3aRAyqjxyKuRZ8qnX
            hex!["b60cfd05f854c298b4fdaac6d1b2d58e94f870ce0ce3f98d1863e56479d19776"]
                .unchecked_into(),
            // 5CB4Smxj5szwZGr1n1ebzEW4HHu8aPqndiZ2oAZGXdDnLFsK
            hex!["04d8c72b0f7cfe7ce77da151a4b3f6d68295f4740cd4f0ce00d1d197d4758e42"]
                .unchecked_into(),
        ),
        (
            (
                // 5EhdQUJkAcaBMmDpAmc2vMhLoRTjUxHgbXmsun7CSawXu6To
                hex!["74a18faa6693cf68b988c479799b9e5fff4c131beb055acdab30c11570df0978"].into(),
                b"Validator4".to_vec(),
            ),
            // 5CJPSYjvaCi2M7izUXwg5pqkQqhaJiHgDX9gz1JoKQrY18jo
            hex!["0a6f713ac4f6773b688be912174a67b6e708386dc2466391c5ba23037ce69951"]
                .unchecked_into(),
            // 5EoaYFrxhUBpvUh1u4EqBRbXkAEZ7mnn45tqz93fvGz3mFxd
            hex!["792b62f72b315be07d5ba5b1492ec2010b69612e03ea89635448eefa0aaff722"]
                .unchecked_into(),
            // 5Cr1Jnhp8p3BQfez9GxZkMaMemPb6PpaqbqkwWWdcwj7W3Qc
            hex!["228cc94abf939b2e38dc932fb87e970fdcdb04b906e29ccf2669ef7cda56836a"]
                .unchecked_into(),
            // 5DNpp5jtdht5JBBUoUtFc46YrgBLeJwP5QyLtQ6fGyii4nkV
            hex!["3a0e10920f8f5f1880d25a1b9d04ce936d64c4a84060428133347cc1c83e8a64"]
                .unchecked_into(),
        ),
        (
            (
                // 5EvXt55kDmAXbBPqrzvNZcbE6bvZ8eWBThxJptoDkwAAyEkw
                hex!["7e7927d030d89585cd66f0d44313de41f4c697da387159786f8b3ed5cd081d4f"].into(),
                b"Validator5".to_vec(),
            ),
            // 5D83WrH4h4rPFxe4m4xGMuC8XuR9jqWHggHBriZQELJ3JneN
            hex!["2ec8253a23695069619df42213106402cffb217bb02c653c11e3435eb047e60d"]
                .unchecked_into(),
            // 5GpHoku58fumTfn9pQZxKFxASvMw6JNTqmFDQw92g7LV1gwj
            hex!["d22ec57d5cdb6f80f0df82590f9999b88e936e9f8c93d9c05cd87dba1b4567ae"]
                .unchecked_into(),
            // 5HCBmPYr7AsXDp4VLu7qh6HRExjy2Nx33YLHqLeQ6i7yFjHt
            hex!["e2e1d5c8eb42aa6b37f71cbdc8e73b67b385e7841d98bbaef3492252a4f3e605"]
                .unchecked_into(),
            // 5HQYBwyf2787MCMhZNBEpswHJcJnXVVNxjrdTa6cVjUf29jy
            hex!["ec4d8806b85969a29214c00ae70b5d239dc65daebf2ea4a43fd47a77e16d9c7c"]
                .unchecked_into(),
        ),
    ];

    let constructor = move || {
        mainnet_genesis(
            &wasm_binary[..],
            initial_authorities.clone(),
            root_key.clone(),
            vesting_key.clone(),
            genesis_assets(),
            // FIXME update btc mainnet header
            btc_genesis_params(include_str!("res/btc_genesis_params_mainnet.json")),
            crate::genesis::bitcoin::mainnet_trustees(),
        )
    };

    Ok(ChainSpec::from_genesis(
        "ChainX",
        "chainx",
        ChainType::Live,
        constructor,
        bootnodes![], // FIXME Add mainnet bootnodes
        Some(
            TelemetryEndpoints::new(vec![
                (CHAINX_TELEMETRY_URL.to_string(), 0),
                (POLKADOT_TELEMETRY_URL.to_string(), 0),
            ])
            .expect("ChainX telemetry url is valid; qed"),
        ),
        Some("pcx"),
        Some(as_properties(NetworkType::Mainnet)),
        Default::default(),
    ))
}

fn session_keys(
    babe: BabeId,
    grandpa: GrandpaId,
    im_online: ImOnlineId,
    authority_discovery: AuthorityDiscoveryId,
) -> SessionKeys {
    SessionKeys {
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
) -> GenesisConfig {
    const ENDOWMENT: Balance = 10_000_000 * DOLLARS;
    const STASH: Balance = 100 * DOLLARS;
    const STAKING_LOCKED: Balance = 1_000 * DOLLARS;
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

    let validators = initial_authorities
        .clone()
        .into_iter()
        .map(|((validator, referral), _, _, _, _)| (validator, referral, STAKING_LOCKED))
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

    GenesisConfig {
        frame_system: Some(SystemConfig {
            code: wasm_binary.to_vec(),
            changes_trie_config: Default::default(),
        }),
        pallet_babe: Some(Default::default()),
        pallet_grandpa: Some(GrandpaConfig {
            authorities: vec![],
        }),
        pallet_collective_Instance1: Some(CouncilConfig::default()),
        pallet_collective_Instance2: Some(TechnicalCommitteeConfig {
            members: tech_comm_members,
            phantom: Default::default(),
        }),
        pallet_membership_Instance1: Some(Default::default()),
        pallet_democracy: Some(DemocracyConfig::default()),
        pallet_treasury: Some(Default::default()),
        pallet_elections_phragmen: Some(ElectionsConfig {
            members: phragmen_members,
        }),
        pallet_im_online: Some(ImOnlineConfig { keys: vec![] }),
        pallet_authority_discovery: Some(AuthorityDiscoveryConfig { keys: vec![] }),
        pallet_session: Some(SessionConfig {
            keys: initial_authorities
                .iter()
                .map(|x| {
                    (
                        (x.0).0.clone(),
                        (x.0).0.clone(),
                        session_keys(x.1.clone(), x.2.clone(), x.3.clone(), x.4.clone()),
                    )
                })
                .collect::<Vec<_>>(),
        }),
        pallet_balances: Some(BalancesConfig { balances }),
        pallet_indices: Some(IndicesConfig { indices: vec![] }),
        pallet_sudo: Some(SudoConfig { key: root_key }),
        xpallet_system: Some(XSystemConfig {
            network_props: NetworkType::Testnet,
        }),
        xpallet_assets_registrar: Some(XAssetsRegistrarConfig { assets }),
        xpallet_assets: Some(XAssetsConfig {
            assets_restrictions,
            endowed: assets_endowed,
        }),
        xpallet_gateway_common: Some(XGatewayCommonConfig { trustees }),
        xpallet_gateway_bitcoin: Some(XGatewayBitcoinConfig {
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
        xpallet_mining_staking: Some(XStakingConfig {
            validators,
            validator_count: 50,
            sessions_per_era: 12,
            vesting_account,
            glob_dist_ratio: (12, 88), // (Treasury, X-type Asset and Staking) = (12, 88)
            mining_ratio: (10, 90),    // (Asset Mining, Staking) = (10, 90)
            minimum_penalty: 2 * DOLLARS,
            ..Default::default()
        }),
        xpallet_mining_asset: Some(XMiningAssetConfig {
            claim_restrictions: vec![(X_BTC, (10, DAYS * 7))],
            mining_power_map: vec![(X_BTC, 400)],
        }),
        xpallet_dex_spot: Some(XSpotConfig {
            trading_pairs: vec![(PCX, X_BTC, 9, 2, 100000, true)],
        }),
        xpallet_genesis_builder: Some(XGenesisBuilderConfig {
            params: crate::genesis::genesis_builder_params(),
            total_endowed,
        }),
    }
}

fn mainnet_genesis(
    wasm_binary: &[u8],
    initial_authorities: Vec<AuthorityKeysTuple>,
    root_key: AccountId,
    vesting_account: AccountId,
    assets: Vec<AssetParams>,
    bitcoin: BtcGenesisParams,
    trustees: Vec<(Chain, TrusteeInfoConfig, Vec<BtcTrusteeParams>)>,
) -> GenesisConfig {
    // 1000 PCX
    const STAKING_LOCKED: Balance = 100_000 * DOLLARS;

    let (assets, assets_restrictions) = init_assets(assets);

    let initial_authorities_len = initial_authorities.len();

    let tech_comm_members = initial_authorities
        .iter()
        .map(|((validator, _), _, _, _, _)| validator)
        .take((initial_authorities_len + 1) / 2)
        .cloned()
        .collect::<Vec<_>>();

    let balances = initial_authorities
        .iter()
        .map(|((validator, _), _, _, _, _)| validator)
        .cloned()
        .map(|validator| (validator, STAKING_LOCKED))
        .collect::<Vec<_>>();

    let total_endowed = initial_authorities_len as Balance * STAKING_LOCKED;

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

    GenesisConfig {
        frame_system: Some(SystemConfig {
            code: wasm_binary.to_vec(),
            changes_trie_config: Default::default(),
        }),
        pallet_babe: Some(Default::default()),
        pallet_grandpa: Some(GrandpaConfig {
            authorities: vec![],
        }),
        pallet_collective_Instance1: Some(CouncilConfig::default()),
        pallet_collective_Instance2: Some(TechnicalCommitteeConfig {
            members: tech_comm_members,
            phantom: Default::default(),
        }),
        pallet_membership_Instance1: Some(Default::default()),
        pallet_democracy: Some(DemocracyConfig::default()),
        pallet_treasury: Some(Default::default()),
        pallet_elections_phragmen: Some(ElectionsConfig::default()),
        pallet_im_online: Some(ImOnlineConfig { keys: vec![] }),
        pallet_authority_discovery: Some(AuthorityDiscoveryConfig { keys: vec![] }),
        pallet_session: Some(SessionConfig {
            keys: initial_authorities
                .iter()
                .map(|x| {
                    (
                        (x.0).0.clone(),
                        (x.0).0.clone(),
                        session_keys(x.1.clone(), x.2.clone(), x.3.clone(), x.4.clone()),
                    )
                })
                .collect::<Vec<_>>(),
        }),
        pallet_balances: Some(BalancesConfig { balances }),
        pallet_indices: Some(IndicesConfig { indices: vec![] }),
        pallet_sudo: Some(SudoConfig { key: root_key }),
        xpallet_system: Some(XSystemConfig {
            network_props: NetworkType::Mainnet,
        }),
        xpallet_assets_registrar: Some(XAssetsRegistrarConfig { assets }),
        xpallet_assets: Some(XAssetsConfig {
            assets_restrictions,
            endowed: Default::default(),
        }),
        xpallet_gateway_common: Some(XGatewayCommonConfig { trustees }),
        xpallet_gateway_bitcoin: Some(XGatewayBitcoinConfig {
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
        xpallet_mining_staking: Some(XStakingConfig {
            validators,
            validator_count: initial_authorities_len as u32, // Start mainnet in PoA
            sessions_per_era: 12,
            vesting_account,
            glob_dist_ratio: (12, 88), // (Treasury, X-type Asset and Staking) = (12, 88)
            mining_ratio: (10, 90),    // (Asset Mining, Staking) = (10, 90)
            minimum_penalty: 2 * DOLLARS,
            ..Default::default()
        }),
        xpallet_mining_asset: Some(XMiningAssetConfig {
            claim_restrictions: vec![(X_BTC, (10, DAYS * 7))],
            mining_power_map: vec![(X_BTC, 400)],
        }),
        xpallet_dex_spot: Some(XSpotConfig {
            trading_pairs: vec![(PCX, X_BTC, 9, 2, 100000, true)],
        }),
        xpallet_genesis_builder: Some(XGenesisBuilderConfig {
            params: crate::genesis::genesis_builder_params(),
            total_endowed,
        }),
    }
}
