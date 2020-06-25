use std::collections::BTreeMap;

use chainx_runtime::{
    h256_conv_endian_from_str, AccountId, Asset, AssetId, AssetRestriction, AssetRestrictions,
    AuraConfig, BTCHeader, BTCNetwork, BTCParams, Balance, Chain, Compact, ContractsSchedule,
    GenesisConfig, GrandpaConfig, Runtime, Signature, SudoConfig, SystemConfig, XAssetsConfig,
    XBridgeBitcoinConfig, XContractsConfig, WASM_BINARY,
};
use sc_service::ChainType;
use sp_consensus_aura::sr25519::AuthorityId as AuraId;
use sp_core::{sr25519, Pair, Public};
use sp_finality_grandpa::AuthorityId as GrandpaId;
use sp_runtime::traits::{IdentifyAccount, Verify};

// Note this is the URL for the telemetry server
//const STAGING_TELEMETRY_URL: &str = "wss://telemetry.polkadot.io/submit/";

/// Specialized `ChainSpec`. This is a specialization of the general Substrate ChainSpec type.
pub type ChainSpec = sc_service::GenericChainSpec<GenesisConfig>;

/// Helper function to generate a crypto pair from seed
pub fn get_from_seed<TPublic: Public>(seed: &str) -> <TPublic::Pair as Pair>::Public {
    TPublic::Pair::from_string(&format!("//{}", seed), None)
        .expect("static values are valid; qed")
        .public()
}

type AccountPublic = <Signature as Verify>::Signer;

/// Helper function to generate an account ID from seed
pub fn get_account_id_from_seed<TPublic: Public>(seed: &str) -> AccountId
where
    AccountPublic: From<<TPublic::Pair as Pair>::Public>,
{
    AccountPublic::from(get_from_seed::<TPublic>(seed)).into_account()
}

/// Helper function to generate an authority key for Aura
pub fn authority_keys_from_seed(s: &str) -> (AuraId, GrandpaId) {
    (get_from_seed::<AuraId>(s), get_from_seed::<GrandpaId>(s))
}

#[inline]
fn balance(input: Balance, precision: u8) -> Balance {
    input * 10_u128.pow(precision as u32)
}

/// A small macro for generating the info of PCX endowed accounts.
macro_rules! endowed_gen {
  ( $( ($seed:expr, $value:expr), )+ ) => {
    {
        let mut endowed = BTreeMap::new();
        let pcx_id = pcx().0;
        let endowed_info = vec![
            $((get_account_id_from_seed::<sr25519::Public>($seed), balance($value, PCX_PRECISION)),)+
        ];
        endowed.insert(pcx_id, endowed_info);
        endowed
    }
  }
}

pub fn development_config() -> ChainSpec {
    let constructor = || {
        testnet_genesis(
            vec![authority_keys_from_seed("Alice")],
            get_account_id_from_seed::<sr25519::Public>("Alice"),
            testnet_assets(),
            endowed_gen![
                ("Alice", 100000),
                ("Bob", 100000),
                ("Alice//stash", 100000),
                ("Bob//stash", 100000),
            ],
            true,
        )
    };
    ChainSpec::from_genesis(
        "Development",
        "dev",
        ChainType::Development,
        constructor,
        vec![],
        None,
        None,
        None,
        None,
    )
}

pub fn local_testnet_config() -> ChainSpec {
    let constructor = || {
        testnet_genesis(
            vec![
                authority_keys_from_seed("Alice"),
                authority_keys_from_seed("Bob"),
            ],
            get_account_id_from_seed::<sr25519::Public>("Alice"),
            testnet_assets(),
            endowed_gen![
                ("Alice", 100000),
                ("Bob", 100000),
                ("Charlie", 100000),
                ("Dave", 100000),
                ("Eve", 100000),
                ("Ferdie", 100000),
                ("Alice//stash", 100000),
                ("Bob//stash", 100000),
                ("Charlie//stash", 100000),
                ("Dave//stash", 100000),
                ("Eve//stash", 100000),
                ("Ferdie//stash", 100000),
            ],
            true,
        )
    };
    ChainSpec::from_genesis(
        "Local Testnet",
        "local_testnet",
        ChainType::Local,
        constructor,
        vec![],
        None,
        None,
        None,
        None,
    )
}

const PCX_PRECISION: u8 = 8;
fn pcx() -> (AssetId, Asset, AssetRestrictions) {
    (
        xrml_protocol::PCX,
        Asset::new::<Runtime>(
            b"PCX".to_vec(),
            b"Polkadot ChainX".to_vec(),
            Chain::ChainX,
            PCX_PRECISION,
            b"ChainX's crypto currency in Polkadot ecology".to_vec(),
        )
        .unwrap(),
        AssetRestriction::Deposit
            | AssetRestriction::Withdraw
            | AssetRestriction::DestroyWithdrawal
            | AssetRestriction::DestroyFree,
    )
}

fn testnet_assets() -> Vec<(AssetId, Asset, AssetRestrictions, bool, bool)> {
    let pcx = pcx();
    let assets = vec![(pcx.0, pcx.1, pcx.2, true, true)];
    assets
}

fn testnet_genesis(
    initial_authorities: Vec<(AuraId, GrandpaId)>,
    root_key: AccountId,
    assets: Vec<(AssetId, Asset, AssetRestrictions, bool, bool)>,
    endowed: BTreeMap<AssetId, Vec<(AccountId, Balance)>>,
    enable_println: bool,
) -> GenesisConfig {
    GenesisConfig {
        frame_system: Some(SystemConfig {
            code: WASM_BINARY.to_vec(),
            changes_trie_config: Default::default(),
        }),
        pallet_aura: Some(AuraConfig {
            authorities: initial_authorities.iter().map(|x| (x.0.clone())).collect(),
        }),
        pallet_grandpa: Some(GrandpaConfig {
            authorities: initial_authorities
                .iter()
                .map(|x| (x.1.clone(), 1))
                .collect(),
        }),
        pallet_sudo: Some(SudoConfig {
            key: root_key.clone(),
        }),
        xrml_assets: Some(XAssetsConfig {
            assets,
            endowed,
            memo_len: 128,
        }),
        xrml_bridge_bitcoin: Some(XBridgeBitcoinConfig {
            genesis_header_and_height: (
                BTCHeader {
                    version: 536870912,
                    previous_header_hash: h256_conv_endian_from_str(
                        "0000000000000000000a4adf6c5192128535d4dcb56cfb5753755f8d392b26bf",
                    ),
                    merkle_root_hash: h256_conv_endian_from_str(
                        "1d21e60acb0b12e5cfd3f775edb647f982a2d666f9886b2f61ea5e72577b0f5e",
                    ),
                    time: 1558168296,
                    bits: Compact::new(388627269),
                    nonce: 1439505020,
                },
                576576,
            ),
            genesis_hash: h256_conv_endian_from_str(
                "0000000000000000001721f58deb88b0710295a02551f0dde1e2e231a15f1882",
            ),
            params_info: BTCParams::new(
                486604799,            // max_bits
                2 * 60 * 60,          // block_max_future
                2 * 7 * 24 * 60 * 60, // target_timespan_seconds
                10 * 60,              // target_spacing_seconds
                4,                    // retargeting_factor
            ), // retargeting_factor
            network_id: BTCNetwork::Mainnet,
            confirmation_number: 4,
            reserved_block: 2100,
            btc_withdrawal_fee: 500000,
            max_withdrawal_count: 100,
        }),
        xrml_contracts: Some(XContractsConfig {
            current_schedule: ContractsSchedule {
                enable_println, // this should only be enabled on development chains
                ..Default::default()
            },
        }),
    }
}
