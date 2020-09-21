// Copyright 2019-2020 ChainX Project Authors. Licensed under GPL-3.0.

use std::collections::BTreeMap;
use std::convert::TryInto;

use hex_literal::hex;
use serde::{Deserialize, Serialize};
use serde_json::json;

use chainx_runtime::{
    constants::currency::DOLLARS, AssetInfo, AssetRestrictions, BtcParams,
    BtcTxVerifier, Chain, NetworkType, TrusteeInfoConfig,
};
use chainx_runtime::{AccountId, AssetId, Balance, ReferralId, Runtime, Signature, WASM_BINARY};
use chainx_runtime::{
    AuraConfig, AuthorityDiscoveryConfig, BalancesConfig, CouncilConfig, DemocracyConfig,
    ElectionsConfig, GenesisConfig, GrandpaConfig, ImOnlineConfig, IndicesConfig, SessionConfig,
    SessionKeys, SocietyConfig, SudoConfig, SystemConfig, TechnicalCommitteeConfig, XAssetsConfig,
    XAssetsRegistrarConfig, XGatewayBitcoinConfig, XGatewayCommonConfig, XMiningAssetConfig,
    XSpotConfig, XStakingConfig, XSystemConfig,
};

use pallet_im_online::sr25519::AuthorityId as ImOnlineId;
use sc_chain_spec::ChainSpecExtension;
use sc_service::{ChainType, Properties};
use sp_authority_discovery::AuthorityId as AuthorityDiscoveryId;
use sp_consensus_aura::sr25519::AuthorityId as AuraId;
use sp_core::{crypto::UncheckedInto, sr25519, Pair, Public};
use sp_finality_grandpa::AuthorityId as GrandpaId;
use sp_runtime::traits::{IdentifyAccount, Verify};

use crate::genesis::trustees::TrusteeParams;
use crate::res::BitcoinParams;
use sc_service::config::TelemetryEndpoints;

// Note this is the URL for the telemetry server
//const STAGING_TELEMETRY_URL: &str = "wss://telemetry.polkadot.io/submit/";
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
    AuraId,
    GrandpaId,
    ImOnlineId,
    AuthorityDiscoveryId,
);

/// Helper function to generate an authority key for Aura
pub fn authority_keys_from_seed(seed: &str) -> AuthorityKeysTuple {
    (
        (
            get_account_id_from_seed::<sr25519::Public>(seed),
            seed.as_bytes().to_vec(),
        ),
        get_account_id_from_seed::<sr25519::Public>(&format!("{}//blockauthor", seed)),
        get_from_seed::<AuraId>(seed),
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

pub fn development_config() -> Result<ChainSpec, String> {
    let wasm_binary = WASM_BINARY.ok_or("Development wasm binary not available".to_string())?;

    let endowed_balance = 50 * DOLLARS;
    let constructor = move || {
        testnet_genesis(
            wasm_binary,
            vec![authority_keys_from_seed("Alice")],
            get_account_id_from_seed::<sr25519::Public>("Alice"),
            get_account_id_from_seed::<sr25519::Public>("vesting"),
            testnet_assets(),
            endowed_gen![
                ("Alice", endowed_balance),
                ("Bob", endowed_balance),
                ("Alice//stash", endowed_balance),
                ("Bob//stash", endowed_balance),
            ],
            crate::res::load_mainnet_btc_genesis_header_info,
            crate::genesis::trustees::local_testnet_trustees(),
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
        testnet_genesis(
            wasm_binary,
            vec![authority_keys_from_seed("Alice")],
            get_account_id_from_seed::<sr25519::Public>("Alice"),
            get_account_id_from_seed::<sr25519::Public>("vesting"),
            testnet_assets(),
            endowed_gen![
                ("Alice", endowed_balance),
                ("Bob", endowed_balance),
                ("Alice//stash", endowed_balance),
                ("Bob//stash", endowed_balance),
            ],
            crate::res::load_mainnet_btc_genesis_header_info,
            crate::genesis::trustees::benchmarks_trustees(),
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
        testnet_genesis(
            wasm_binary,
            vec![
                authority_keys_from_seed("Alice"),
                authority_keys_from_seed("Bob"),
            ],
            get_account_id_from_seed::<sr25519::Public>("Alice"),
            get_account_id_from_seed::<sr25519::Public>("vesting"),
            testnet_assets(),
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
            crate::res::load_mainnet_btc_genesis_header_info,
            crate::genesis::trustees::local_testnet_trustees(),
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
    // for i in 1 2 3; do for j in aura; do subkey inspect-key --scheme sr25519  --uri "$SECRET//$i//$j"; done; done
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

    // aura
    // 5EZ47mio3fjhb1iwGSLKZGmgYvhZRJakfGmPfAemMAMBAA7e
    let aura1: AuraId =
        hex!["6e178a72736139a91e32dadeb57c2822501690e9d8f1516a04b18372cd981831"].unchecked_into();
    // 5EpnwHC4QjhHXq9tGV4FE94GG17JBDDBXfBALPu5VQTVqbyp
    let aura2: AuraId =
        hex!["7a185d241085c938fda96b54059632f885866befb1183aa4dd456f8a406db70c"].unchecked_into();
    // 5CV7jA56wV3mjzLi4JMg4oXATNpwKfcet61NwYJqAAiRsEH9
    let aura3: AuraId =
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
            aura1,
            grandpa1,
            im_online1,
            authority_discovery1,
        ),
        (
            (validator2, b"Validator2".to_vec()),
            blockauthor2,
            aura2,
            grandpa2,
            im_online2,
            authority_discovery2,
        ),
        (
            (validator3, b"Validator3".to_vec()),
            blockauthor3,
            aura3,
            grandpa3,
            im_online3,
            authority_discovery3,
        ),
    ];

    let assets = testnet_assets();
    let endowed_balance = 50 * DOLLARS;
    let mut endowed = BTreeMap::new();
    let pcx_id = pcx().0;
    let endowed_info = initial_authorities
        .iter()
        .map(|i| ((i.0).0.clone(), endowed_balance))
        .collect::<Vec<_>>();
    endowed.insert(pcx_id, endowed_info);

    let constructor = move || {
        testnet_genesis(
            wasm_binary,
            initial_authorities.clone(),
            root_key.clone(),
            root_key.clone(), // use root key as vesting_account
            assets.clone(),
            endowed.clone(),
            crate::res::load_testnet_btc_genesis_header_info,
            crate::genesis::trustees::staging_testnet_trustees(),
        )
    };
    Ok(ChainSpec::from_genesis(
        "ChainX Staging Testnet",
        "chainx_staging_testnet",
        ChainType::Live,
        constructor,
        vec![
            "/dns/p2p.staging-1.chainx.org/tcp/30333/p2p/12D3KooWQq7h1cqwRqFaRnp7LxcWmBAzJtizS4uckJrxyK5KHron".to_string().try_into().expect("must be valid bootnode"),
            "/dns/p2p.staging-2.chainx.org/tcp/30334/p2p/12D3KooWNKCPciz7iAJ6DBqSygsfzHCVdoMCMWoBgo1EgHMrTpDN".to_string().try_into().expect("must be valid bootnode"),
            "/dns/p2p.staging-3.chainx.org/tcp/30335/p2p/12D3KooWLuxACVFoeddQ4ja68C7Y4qNrXtpBC9gx7akRPacnvoJe".to_string().try_into().expect("must be valid bootnode"),
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
    // 5ERUBzfWtZzB59HM2qekCKzPm9sFo433z3V4rGgJXd7ugWNv
    let root_key: AccountId =
        hex!["684e9d27ae6b5ab3a673616de27bd3e455062c83090de607ab49a2f7396b5a19"].into();
    // bash:
    // for i in 1 2 3; do for j in validator blockauthor; do subkey inspect-key --uri "$SECRET//$i//$j"; done; done
    // for i in 1 2 3; do for j in aura; do subkey inspect-key --scheme sr25519  --uri "$SECRET//$i//$j"; done; done
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

    // aura
    // 5EZ47mio3fjhb1iwGSLKZGmgYvhZRJakfGmPfAemMAMBAA7e
    let aura1: AuraId =
        hex!["6e178a72736139a91e32dadeb57c2822501690e9d8f1516a04b18372cd981831"].unchecked_into();
    // 5EpnwHC4QjhHXq9tGV4FE94GG17JBDDBXfBALPu5VQTVqbyp
    let aura2: AuraId =
        hex!["7a185d241085c938fda96b54059632f885866befb1183aa4dd456f8a406db70c"].unchecked_into();
    // 5CV7jA56wV3mjzLi4JMg4oXATNpwKfcet61NwYJqAAiRsEH9
    let aura3: AuraId =
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
            aura1,
            grandpa1,
            im_online1,
            authority_discovery1,
        ),
        (
            (validator2, b"Validator2".to_vec()),
            blockauthor2,
            aura2,
            grandpa2,
            im_online2,
            authority_discovery2,
        ),
        (
            (validator3, b"Validator3".to_vec()),
            blockauthor3,
            aura3,
            grandpa3,
            im_online3,
            authority_discovery3,
        ),
    ];

    let assets = testnet_assets();
    let endowed_balance = 50 * DOLLARS;
    let mut endowed = BTreeMap::new();
    let pcx_id = pcx().0;
    let endowed_info = initial_authorities
        .iter()
        .map(|i| ((i.0).0.clone(), endowed_balance))
        .collect::<Vec<_>>();
    endowed.insert(pcx_id, endowed_info);

    let constructor = move || {
        testnet_genesis(
            &wasm_binary[..],
            initial_authorities.clone(),
            root_key.clone(),
            root_key.clone(), // use root key as vesting_account
            assets.clone(),
            endowed.clone(),
            crate::res::load_testnet_btc_genesis_header_info,
            crate::genesis::trustees::staging_testnet_trustees(),
        )
    };
    Ok(ChainSpec::from_genesis(
        "ChainX Testnet",
        "chainx_testnet",
        ChainType::Live,
        constructor,
        vec![
            "/dns/p2p.testnet-1.chainx.org/tcp/30333/p2p/12D3KooWQq7h1cqwRqFaRnp7LxcWmBAzJtizS4uckJrxyK5KHron".to_string().try_into().expect("must be valid bootnode"),
            "/dns/p2p.testnet-2.chainx.org/tcp/30334/p2p/12D3KooWNKCPciz7iAJ6DBqSygsfzHCVdoMCMWoBgo1EgHMrTpDN".to_string().try_into().expect("must be valid bootnode"),
            "/dns/p2p.testnet-3.chainx.org/tcp/30335/p2p/12D3KooWLuxACVFoeddQ4ja68C7Y4qNrXtpBC9gx7akRPacnvoJe".to_string().try_into().expect("must be valid bootnode"),
        ],
        Some(TelemetryEndpoints::new(vec![(CHAINX_TELEMETRY_URL.to_string(), 0)]).expect("Testnet telemetry url is valid; qed")),
        Some("chainx-testnet"),
        Some(as_properties(NetworkType::Testnet)),
        Default::default(),
    ))
}

const PCX_DECIMALS: u8 = 8;
const BTC_DECIMALS: u8 = 8;
fn pcx() -> (AssetId, AssetInfo, AssetRestrictions) {
    (
        xpallet_protocol::PCX,
        AssetInfo::new::<Runtime>(
            b"PCX".to_vec(),
            b"Polkadot ChainX".to_vec(),
            Chain::ChainX,
            PCX_DECIMALS,
            b"ChainX's crypto currency in Polkadot ecology".to_vec(),
        )
        .unwrap(),
        AssetRestrictions::Deposit
            | AssetRestrictions::Withdraw
            | AssetRestrictions::DestroyWithdrawal
            | AssetRestrictions::DestroyUsable,
    )
}

fn xbtc() -> (AssetId, AssetInfo, AssetRestrictions) {
    (
        xpallet_protocol::X_BTC,
        AssetInfo::new::<Runtime>(
            b"XBTC".to_vec(),
            b"ChainX Bitcoin".to_vec(),
            Chain::Bitcoin,
            BTC_DECIMALS,
            b"ChainX's Cross-chain Bitcoin".to_vec(),
        )
        .unwrap(),
        AssetRestrictions::DestroyUsable,
    )
}

// asset_id, asset_info, asset_restrictions, is_online, has_mining_rights
fn testnet_assets() -> Vec<(AssetId, AssetInfo, AssetRestrictions, bool, bool)> {
    let pcx = pcx();
    let btc = xbtc();
    let assets = vec![
        (pcx.0, pcx.1, pcx.2, true, false),
        (btc.0, btc.1, btc.2, true, true),
    ];
    assets
}

fn session_keys(
    aura: AuraId,
    grandpa: GrandpaId,
    im_online: ImOnlineId,
    authority_discovery: AuthorityDiscoveryId,
) -> SessionKeys {
    SessionKeys {
        grandpa,
        aura,
        im_online,
        authority_discovery,
    }
}

type AssetParams = (AssetId, AssetInfo, AssetRestrictions, bool, bool);
fn init_assets(
    assets: Vec<AssetParams>,
) -> (
    Vec<(AssetId, AssetInfo, bool, bool)>,
    Vec<(AssetId, AssetRestrictions)>,
) {
    let mut init_assets = vec![];
    let mut assets_restrictions = vec![];
    for (a, b, c, d, e) in assets {
        init_assets.push((a, b, d, e));
        assets_restrictions.push((a, c))
    }
    (init_assets, assets_restrictions)
}

fn testnet_genesis<F>(
    wasm_binary: &[u8],
    initial_authorities: Vec<AuthorityKeysTuple>,
    root_key: AccountId,
    vesting_account: AccountId,
    assets: Vec<AssetParams>,
    endowed: BTreeMap<AssetId, Vec<(AccountId, Balance)>>,
    bitcoin_info: F,
    trustees: Vec<(Chain, TrusteeInfoConfig, Vec<TrusteeParams>)>,
) -> GenesisConfig
where
    F: FnOnce() -> BitcoinParams,
{
    const ENDOWMENT: Balance = 10_000_000 * DOLLARS;
    const STASH: Balance = 100 * DOLLARS;
    const STAKING_LOCKED: Balance = 1_000 * DOLLARS;
    let (assets, assets_restrictions) = init_assets(assets);

    let endowed_accounts = endowed
        .get(&xpallet_protocol::PCX)
        .expect("PCX endowed; qed")
        .iter()
        .cloned()
        .map(|(k, _)| k)
        .collect::<Vec<_>>();

    let num_endowed_accounts = endowed_accounts.len();

    let balances = endowed
        .get(&xpallet_protocol::PCX)
        .expect("PCX endowed; qed")
        .iter()
        .cloned()
        .map(|(k, _)| (k, ENDOWMENT))
        .collect::<Vec<_>>();

    // The value of STASH balance will be reserved per phragmen member.
    let phragmen_members = endowed_accounts
        .iter()
        .take((num_endowed_accounts + 1) / 2)
        .cloned()
        .map(|member| (member, STASH))
        .collect();

    // PCX only reserves the native asset id in assets module,
    // the actual native fund management is handled by pallet_balances.
    let mut assets_endowed = endowed;
    assets_endowed.remove(&xpallet_protocol::PCX);

    let validators = initial_authorities
        .clone()
        .into_iter()
        .map(|((val, referral_id), _, _, _, _, _)| (val, referral_id, STAKING_LOCKED))
        .collect::<Vec<_>>();

    GenesisConfig {
        frame_system: Some(SystemConfig {
            code: wasm_binary.to_vec(),
            changes_trie_config: Default::default(),
        }),
        pallet_aura: Some(AuraConfig {
            authorities: vec![],
        }),
        pallet_grandpa: Some(GrandpaConfig {
            authorities: vec![],
        }),
        pallet_collective_Instance1: Some(CouncilConfig::default()),
        pallet_collective_Instance2: Some(TechnicalCommitteeConfig {
            members: endowed_accounts
                .iter()
                .take((num_endowed_accounts + 1) / 2)
                .cloned()
                .collect(),
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
        pallet_society: Some(SocietyConfig {
            members: endowed_accounts
                .iter()
                .take((num_endowed_accounts + 1) / 2)
                .cloned()
                .collect(),
            pot: 0,
            max_members: 999,
        }),
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
        xpallet_gateway_bitcoin: {
            let BitcoinParams {
                genesis_info,
                genesis_hash,
                network,
                confirmed_count,
            } = bitcoin_info(); // crate::res::load_mainnet_btc_genesis_header_info();
            Some(XGatewayBitcoinConfig {
                genesis_info,
                genesis_hash,
                network_id: network,
                params_info: BtcParams::new(
                    486604799,            // max_bits
                    2 * 60 * 60,          // block_max_future
                    2 * 7 * 24 * 60 * 60, // target_timespan_seconds
                    10 * 60,              // target_spacing_seconds
                    4,                    // retargeting_factor
                ), // retargeting_factor
                verifier: BtcTxVerifier::Recover,
                confirmation_number: confirmed_count,
                reserved_block: 2100,
                btc_withdrawal_fee: 500000,
                max_withdrawal_count: 100,
            })
        },
        xpallet_mining_staking: Some(XStakingConfig {
            validators,
            validator_count: 50,
            sessions_per_era: 12,
            vesting_account,
            glob_dist_ratio: (12, 88), // (Treasury, X-type Asset and Staking) = (12, 88)
            mining_ratio: (10, 90),    // (Asset Mining, Staking) = (10, 90)
            minimum_penalty: 2 * DOLLARS,
            offence_severity: 2,
            ..Default::default()
        }),
        xpallet_mining_asset: Some(XMiningAssetConfig {
            claim_restrictions: vec![(xpallet_protocol::X_BTC, (10, chainx_runtime::DAYS * 7))],
            mining_power_map: vec![(xpallet_protocol::X_BTC, 400)],
        }),
        xpallet_dex_spot: Some(XSpotConfig {
            trading_pairs: vec![(
                xpallet_protocol::PCX,
                xpallet_protocol::X_BTC,
                9,
                2,
                100000,
                true,
            )],
        }),
    }
}
