// Copyright 2019-2020 ChainX Project Authors. Licensed under GPL-3.0.

use std::collections::BTreeMap;
use std::convert::TryInto;

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
use crate::genesis::bitcoin::{
    btc_genesis_params, local_testnet_trustees, mainnet_trustees, staging_testnet_trustees,
    BtcGenesisParams, BtcTrusteeParams,
};

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
    AccountId,               // (SessionKey)
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
        get_account_id_from_seed::<sr25519::Public>(&format!("{}//blockauthor", seed)),
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
            local_testnet_trustees(),
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
            btc_genesis_params(include_str!("res/btc_genesis_params_mainnet.json")),
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
            local_testnet_trustees(),
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

pub fn staging_testnet_config() -> Result<ChainSpec, String> {
    let wasm_binary =
        WASM_BINARY.ok_or_else(|| "Staging Testnet wasm not available".to_string())?;
    // subkey inspect-key --uri "$SECRET"
    // 5ERUBzfWtZzB59HM2qekCKzPm9sFo433z3V4rGgJXd7ugWNv
    let root_key: AccountId =
        hex!["684e9d27ae6b5ab3a673616de27bd3e455062c83090de607ab49a2f7396b5a19"].into();
    // bash:
    // for i in 1 2 3; do for j in validator blockauthor; do subkey inspect-key --uri "$SECRET//$i//$j"; done; done
    // for i in 1 2 3; do for j in babe; do subkey inspect-key --scheme sr25519  --uri "$SECRET//$i//$j"; done; done
    // for i in 1 2 3; do for j in grandpa; do subkey inspect-key --scheme ed25519 --uri "$SECRET//$i//$j"; done; done
    // for i in 1 2 3; do for j in im_online; do subkey inspect-key --scheme sr25519 --uri "$SECRET//$i//$j"; done; done
    // for i in 1 2 3; do for j in authority_discovery; do subkey inspect-key --scheme sr25519 --uri "$SECRET//$i//$j"; done; done

    // validator & blockauthor
    let (validator1, blockauthor1): (AccountId, AccountId) = (
        // 5Ca46gRUa2oS6GukzKph8qFfn4WdhP5yhuRaTuzaXsKjfGgM
        hex!["16624186f2ea93a21f34e00ae622959e40d841231b26e625be93f75137b2a10d"].into(),
        // 5Ca1ayQB2GfVb6tPjB849aViHF4vVgBs6USiNVqGeKorPwMw
        hex!["1659cc960f00d5c82662bd97b516330caf9759d7fa7b98fee45005765a19287c"].into(),
    );
    let (validator2, blockauthor2): (AccountId, AccountId) = (
        // 5DV17DNeRCidmacaP1MdhD8YV8A94PmVyr4eRcKq8tG6Q17C
        hex!["3ec431c8b3ae28095ad652f5531a770ef21e59779d4a3a46e0217baa4c614624"].into(),
        // 5FWYBfwLKQhGVqbUZevjkXpM9EqS79tYRkRAwUvnDT2QdJa8
        hex!["9868855492e0bbf55034b9eb52f0200ede9a0e47b5388074163c0fdc7251cd43"].into(),
    );
    let (validator3, blockauthor3): (AccountId, AccountId) = (
        // 5ERY5k4cDMhhE7B8PRA26fCs1VbHNZJAhHoiuZhzP18cxq8T
        hex!["685bb75b531394c4d522003784cc62fa15fcab8fe16c19c3f4a1eeae308afa4f"].into(),
        // 5FCPo3uswynCs1rPvCpnjFykhN3jmeUH51ocMqfpzPq9jVwc
        hex!["8a91dc3768bdba8bba11da5c3b2ae954eede9591a6b7a2d156637d84aee5623c"].into(),
    );

    // babe
    // 5EZ47mio3fjhb1iwGSLKZGmgYvhZRJakfGmPfAemMAMBAA7e
    let babe1: BabeId =
        hex!["6e178a72736139a91e32dadeb57c2822501690e9d8f1516a04b18372cd981831"].unchecked_into();
    // 5EpnwHC4QjhHXq9tGV4FE94GG17JBDDBXfBALPu5VQTVqbyp
    let babe2: BabeId =
        hex!["7a185d241085c938fda96b54059632f885866befb1183aa4dd456f8a406db70c"].unchecked_into();
    // 5CV7jA56wV3mjzLi4JMg4oXATNpwKfcet61NwYJqAAiRsEH9
    let babe3: BabeId =
        hex!["129e3eb4543ed8188d67df20122bb73add3f0ea5fdbd480fdbb9f6b4c14dd872"].unchecked_into();

    // grandpa
    // 5EntNNUQB97ui1F2g1aT9tTBUHsUY3Zi6noVLH5uVfoFadYR
    let grandpa1: GrandpaId =
        hex!["78a4292a2fbccbedc19663a787d13ad5e1af9b1aa4cc7d28adb10c239965eaf5"].unchecked_into();
    // 5CYecRFedCR6rjCe3d6AwLi9AsArdga8fdzUPfCR11bp45Ax
    let grandpa2: GrandpaId =
        hex!["154ff203b637f4dd8d3e186e6820414bb43ccddf0022f3d1754c3862decd3696"].unchecked_into();
    // 5Hj97jQ5SE4TWbpJX1w8CtjZftK9ZzvHUtQWtuiunc1hfTG2
    let grandpa3: GrandpaId =
        hex!["fa7d863e427ebb01df0c66d05cfbbb043ff8abb964786a4ee8d2eceda2b43fef"].unchecked_into();

    // im-online
    // 5GmSNWiRT6GMptZsb97kAMC3eqRikMP4uA8m96JQgCdv5vKf
    let im_online1: ImOnlineId =
        hex!["d001dde321a31457fc615210754a49f9793d22d282e3bb7153ed4257dd238777"].unchecked_into();
    // 5G25Rj3gBQG1bd9sSGzXNdD6c1zTm7W1srBWusjTrA1V6paZ
    let im_online2: ImOnlineId =
        hex!["aeefcefafc41d8b69327cc61e5d9961769851f5238f4cf8ce7f149bf9c9cc85d"].unchecked_into();
    // 5EtJ2KYfVdCscuBrBrV6KVvq9eqhajS9MpHPY9BoEWrhxGCw
    let im_online3: ImOnlineId =
        hex!["7cc403ead4673f243779bb77041e8791f85fc42ebfa2dbffd7ddcc68e6321807"].unchecked_into();

    // authority-discovery
    // 5F4kvJLWoKr9ikn3pEXpTCfLDnfpLAVUf2itbFuJM1NdLuUM
    let authority_discovery1: AuthorityDiscoveryId =
        hex!["84bf028f518c5039c30400da70909f41346c2078ae32d406eb7b74829f13904f"].unchecked_into();
    // 5EhrqABtJXMpzVXu2oy6AQTUAoAAUBikDhXVxCYSfvnox2eQ
    let authority_discovery2: AuthorityDiscoveryId =
        hex!["74cec1864e320408617c7276e98fe2aa75c1f552f2a5621ead78d6c43b390a28"].unchecked_into();
    // 5GTRgcMghrEz92uKLvQLX9opt5SnonYNF9fEqAHooAss2TNq
    let authority_discovery3: AuthorityDiscoveryId =
        hex!["c245222eed6474d094baf1db1225a18dae39567fa16dd7ab0e181e5770d73e26"].unchecked_into();

    let initial_authorities: Vec<AuthorityKeysTuple> = vec![
        (
            (validator1, b"Validator1".to_vec()),
            blockauthor1,
            babe1,
            grandpa1,
            im_online1,
            authority_discovery1,
        ),
        (
            (validator2, b"Validator2".to_vec()),
            blockauthor2,
            babe2,
            grandpa2,
            im_online2,
            authority_discovery2,
        ),
        (
            (validator3, b"Validator3".to_vec()),
            blockauthor3,
            babe3,
            grandpa3,
            im_online3,
            authority_discovery3,
        ),
    ];

    let assets = genesis_assets();
    let endowed_balance = 50 * DOLLARS;
    let mut endowed = BTreeMap::new();
    let pcx_id = pcx().0;
    let endowed_info = initial_authorities
        .iter()
        .map(|i| ((i.0).0.clone(), endowed_balance))
        .collect::<Vec<_>>();
    endowed.insert(pcx_id, endowed_info);

    let constructor = move || {
        build_genesis(
            wasm_binary,
            initial_authorities.clone(),
            root_key.clone(),
            root_key.clone(), // use root key as vesting_account
            assets.clone(),
            endowed.clone(),
            btc_genesis_params(include_str!("res/btc_genesis_params_testnet.json")),
            staging_testnet_trustees(),
        )
    };
    Ok(ChainSpec::from_genesis(
        "ChainX Staging Testnet",
        "chainx_staging_testnet",
        ChainType::Live,
        constructor,
        bootnodes![
            "/dns/p2p.staging-1.chainx.org/tcp/30333/p2p/12D3KooWQq7h1cqwRqFaRnp7LxcWmBAzJtizS4uckJrxyK5KHron",
            "/dns/p2p.staging-2.chainx.org/tcp/30334/p2p/12D3KooWNKCPciz7iAJ6DBqSygsfzHCVdoMCMWoBgo1EgHMrTpDN",
            "/dns/p2p.staging-3.chainx.org/tcp/30335/p2p/12D3KooWLuxACVFoeddQ4ja68C7Y4qNrXtpBC9gx7akRPacnvoJe",
        ],
        None,
        Some("chainx-staging-testnet"),
        Some(as_properties(NetworkType::Testnet)),
        Default::default(),
    ))
}

pub fn testnet_config() -> Result<ChainSpec, String> {
    let wasm_binary = include_bytes!("./wasm/chainx_runtime_testnet.compact.wasm");
    // subkey inspect-key --uri "$SECRET"
    // 5DevKrCXVnGtJ5epm19VCQwdbjXVGLvDVVe86b67sRweMh8P
    let root_key: AccountId =
        hex!["46548ce2fca0244d9ca8bc2b82d599458d340d0da3c13078689cf4f17bbb3017"].into();
    // bash:
    // for i in 1 2 3; do for j in validator blockauthor; do subkey inspect-key --uri "$SECRET//$i//$j"; done; done
    // for i in 1 2 3; do for j in babe; do subkey inspect-key --scheme sr25519  --uri "$SECRET//$i//$j"; done; done
    // for i in 1 2 3; do for j in grandpa; do subkey inspect-key --scheme ed25519 --uri "$SECRET//$i//$j"; done; done
    // for i in 1 2 3; do for j in im_online; do subkey inspect-key --scheme sr25519 --uri "$SECRET//$i//$j"; done; done
    // for i in 1 2 3; do for j in authority_discovery; do subkey inspect-key --scheme sr25519 --uri "$SECRET//$i//$j"; done; done

    // validator & blockauthor
    let (validator1, blockauthor1): (AccountId, AccountId) = (
        // 5Ca46gRUa2oS6GukzKph8qFfn4WdhP5yhuRaTuzaXsKjfGgM
        hex!["16624186f2ea93a21f34e00ae622959e40d841231b26e625be93f75137b2a10d"].into(),
        // 5Ca1ayQB2GfVb6tPjB849aViHF4vVgBs6USiNVqGeKorPwMw
        hex!["1659cc960f00d5c82662bd97b516330caf9759d7fa7b98fee45005765a19287c"].into(),
    );
    let (validator2, blockauthor2): (AccountId, AccountId) = (
        // 5DV17DNeRCidmacaP1MdhD8YV8A94PmVyr4eRcKq8tG6Q17C
        hex!["3ec431c8b3ae28095ad652f5531a770ef21e59779d4a3a46e0217baa4c614624"].into(),
        // 5FWYBfwLKQhGVqbUZevjkXpM9EqS79tYRkRAwUvnDT2QdJa8
        hex!["9868855492e0bbf55034b9eb52f0200ede9a0e47b5388074163c0fdc7251cd43"].into(),
    );
    let (validator3, blockauthor3): (AccountId, AccountId) = (
        // 5ERY5k4cDMhhE7B8PRA26fCs1VbHNZJAhHoiuZhzP18cxq8T
        hex!["685bb75b531394c4d522003784cc62fa15fcab8fe16c19c3f4a1eeae308afa4f"].into(),
        // 5FCPo3uswynCs1rPvCpnjFykhN3jmeUH51ocMqfpzPq9jVwc
        hex!["8a91dc3768bdba8bba11da5c3b2ae954eede9591a6b7a2d156637d84aee5623c"].into(),
    );

    // babe
    // 5EZ47mio3fjhb1iwGSLKZGmgYvhZRJakfGmPfAemMAMBAA7e
    let babe1: BabeId =
        hex!["6e178a72736139a91e32dadeb57c2822501690e9d8f1516a04b18372cd981831"].unchecked_into();
    // 5EpnwHC4QjhHXq9tGV4FE94GG17JBDDBXfBALPu5VQTVqbyp
    let babe2: BabeId =
        hex!["7a185d241085c938fda96b54059632f885866befb1183aa4dd456f8a406db70c"].unchecked_into();
    // 5CV7jA56wV3mjzLi4JMg4oXATNpwKfcet61NwYJqAAiRsEH9
    let babe3: BabeId =
        hex!["129e3eb4543ed8188d67df20122bb73add3f0ea5fdbd480fdbb9f6b4c14dd872"].unchecked_into();

    // grandpa
    // 5EntNNUQB97ui1F2g1aT9tTBUHsUY3Zi6noVLH5uVfoFadYR
    let grandpa1: GrandpaId =
        hex!["78a4292a2fbccbedc19663a787d13ad5e1af9b1aa4cc7d28adb10c239965eaf5"].unchecked_into();
    // 5CYecRFedCR6rjCe3d6AwLi9AsArdga8fdzUPfCR11bp45Ax
    let grandpa2: GrandpaId =
        hex!["154ff203b637f4dd8d3e186e6820414bb43ccddf0022f3d1754c3862decd3696"].unchecked_into();
    // 5Hj97jQ5SE4TWbpJX1w8CtjZftK9ZzvHUtQWtuiunc1hfTG2
    let grandpa3: GrandpaId =
        hex!["fa7d863e427ebb01df0c66d05cfbbb043ff8abb964786a4ee8d2eceda2b43fef"].unchecked_into();

    // im-online
    // 5GmSNWiRT6GMptZsb97kAMC3eqRikMP4uA8m96JQgCdv5vKf
    let im_online1: ImOnlineId =
        hex!["d001dde321a31457fc615210754a49f9793d22d282e3bb7153ed4257dd238777"].unchecked_into();
    // 5G25Rj3gBQG1bd9sSGzXNdD6c1zTm7W1srBWusjTrA1V6paZ
    let im_online2: ImOnlineId =
        hex!["aeefcefafc41d8b69327cc61e5d9961769851f5238f4cf8ce7f149bf9c9cc85d"].unchecked_into();
    // 5EtJ2KYfVdCscuBrBrV6KVvq9eqhajS9MpHPY9BoEWrhxGCw
    let im_online3: ImOnlineId =
        hex!["7cc403ead4673f243779bb77041e8791f85fc42ebfa2dbffd7ddcc68e6321807"].unchecked_into();

    // authority-discovery
    // 5F4kvJLWoKr9ikn3pEXpTCfLDnfpLAVUf2itbFuJM1NdLuUM
    let authority_discovery1: AuthorityDiscoveryId =
        hex!["84bf028f518c5039c30400da70909f41346c2078ae32d406eb7b74829f13904f"].unchecked_into();
    // 5EhrqABtJXMpzVXu2oy6AQTUAoAAUBikDhXVxCYSfvnox2eQ
    let authority_discovery2: AuthorityDiscoveryId =
        hex!["74cec1864e320408617c7276e98fe2aa75c1f552f2a5621ead78d6c43b390a28"].unchecked_into();
    // 5GTRgcMghrEz92uKLvQLX9opt5SnonYNF9fEqAHooAss2TNq
    let authority_discovery3: AuthorityDiscoveryId =
        hex!["c245222eed6474d094baf1db1225a18dae39567fa16dd7ab0e181e5770d73e26"].unchecked_into();

    let initial_authorities: Vec<AuthorityKeysTuple> = vec![
        (
            (validator1, b"Validator1".to_vec()),
            blockauthor1,
            babe1,
            grandpa1,
            im_online1,
            authority_discovery1,
        ),
        (
            (validator2, b"Validator2".to_vec()),
            blockauthor2,
            babe2,
            grandpa2,
            im_online2,
            authority_discovery2,
        ),
        (
            (validator3, b"Validator3".to_vec()),
            blockauthor3,
            babe3,
            grandpa3,
            im_online3,
            authority_discovery3,
        ),
    ];

    let assets = genesis_assets();
    let endowed_balance = 50 * DOLLARS;
    let mut endowed = BTreeMap::new();
    let pcx_id = pcx().0;
    let endowed_info = initial_authorities
        .iter()
        .map(|i| ((i.0).0.clone(), endowed_balance))
        .collect::<Vec<_>>();
    endowed.insert(pcx_id, endowed_info);

    let constructor = move || {
        build_genesis(
            &wasm_binary[..],
            initial_authorities.clone(),
            root_key.clone(),
            root_key.clone(), // use root key as vesting_account
            assets.clone(),
            endowed.clone(),
            btc_genesis_params(include_str!("res/btc_genesis_params_testnet.json")),
            staging_testnet_trustees(),
        )
    };
    Ok(ChainSpec::from_genesis(
        "ChainX Testnet",
        "chainx_testnet",
        ChainType::Live,
        constructor,
        bootnodes![
            "/dns/p2p.testnet-1.chainx.org/tcp/30333/p2p/12D3KooWQq7h1cqwRqFaRnp7LxcWmBAzJtizS4uckJrxyK5KHron",
            "/dns/p2p.testnet-2.chainx.org/tcp/30334/p2p/12D3KooWNKCPciz7iAJ6DBqSygsfzHCVdoMCMWoBgo1EgHMrTpDN",
            "/dns/p2p.testnet-3.chainx.org/tcp/30335/p2p/12D3KooWLuxACVFoeddQ4ja68C7Y4qNrXtpBC9gx7akRPacnvoJe",
        ],
        Some(
            TelemetryEndpoints::new(vec![
                (CHAINX_TELEMETRY_URL.to_string(), 0),
                (POLKADOT_TELEMETRY_URL.to_string(), 0),
            ])
            .expect("Testnet telemetry url is valid; qed"),
        ),
        Some("chainx-testnet"),
        Some(as_properties(NetworkType::Testnet)),
        Default::default(),
    ))
}

pub fn mainnet_pre_config() -> Result<ChainSpec, String> {
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
            // 5GgaYZcQyHMk75VxvYQVQR3b2gxsPawsgGXjx8vhCiiTabLR
            hex!["cc4d27469504538539515615deb495c2901a774abd1a13655336b615d4014a3c"].into(),
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
            // 5E4ayu9boQK88fkBRCb5D2KeFHzqko3xeBsjidgnVBZrEKvM
            hex!["5861528f602af4e6237e8c0724885db5aa2a8c2faa44ba539d2436021537be04"].into(),
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
            // 5FeZHG9fpPKxzjMikjX39FXvUdG4JJ2yhHpA2YBGywtWzCcK
            hex!["9e862cbf1bb26bed7660f725317f53a5c40dd50aa4e36d81263681651a6f2931"].into(),
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
            // 5ERRsAvNXWhZmqG4tdjcGEgJHCkqYSDSdsdqAq3GGoRGpu9m
            hex!["6846c9ee8d47c20bda6d4458bef2677e75a2dc2df93a506e36c03cb6d53e801f"].into(),
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
            // 5FyRpivr8qN9sK1PU9Qhutv4QwVnHomFGgMn93khwCbDCBXq
            hex!["aceabbac700af582fe4ff6084ba1c778339fbadccb1c182d82817aacdfcb5345"].into(),
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
            mainnet_trustees(),
        )
    };

    Ok(ChainSpec::from_genesis(
        "ChainX 2.0 Pre",
        "chainx-pre",
        ChainType::Live,
        constructor,
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
        Some("pcx-pre"),
        Some(as_properties(NetworkType::Testnet)),
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
        .map(|((validator, referral), _, _, _, _, _)| (validator, referral, STAKING_LOCKED))
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
                        session_keys(x.2.clone(), x.3.clone(), x.4.clone(), x.5.clone()),
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
        .map(|((validator, _), _, _, _, _, _)| validator)
        .take((initial_authorities_len + 1) / 2)
        .cloned()
        .collect::<Vec<_>>();

    let balances = initial_authorities
        .iter()
        .map(|((validator, _), _, _, _, _, _)| validator)
        .cloned()
        .map(|validator| (validator, STAKING_LOCKED))
        .collect::<Vec<_>>();

    let total_endowed = initial_authorities_len as Balance * STAKING_LOCKED;

    let validators = initial_authorities
        .clone()
        .into_iter()
        .map(|((validator, referral_id), _, _, _, _, _)| (validator, referral_id, STAKING_LOCKED))
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
                        session_keys(x.2.clone(), x.3.clone(), x.4.clone(), x.5.clone()),
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
