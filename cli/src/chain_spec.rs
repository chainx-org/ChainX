use std::collections::BTreeMap;
use std::convert::TryFrom;

use chainx_runtime::{
    constants, trustees, AssetInfo, AssetRestriction, AssetRestrictions, BtcParams, BtcTxVerifier,
    Chain, ContractsSchedule, NetworkType, TrusteeInfoConfig,
};
use chainx_runtime::{AccountId, AssetId, Balance, Runtime, Signature, WASM_BINARY};
use chainx_runtime::{
    AuraConfig, BalancesConfig, CouncilConfig, ElectionsConfig, GenesisConfig, GrandpaConfig,
    ImOnlineConfig, SessionConfig, SessionKeys, SudoConfig, SystemConfig, TechnicalCommitteeConfig,
    XAssetsConfig, XContractsConfig, XGatewayBitcoinConfig, XGatewayCommonConfig,
    XMiningAssetConfig, XSpotConfig, XStakingConfig, XSystemConfig,
};

use pallet_im_online::sr25519::AuthorityId as ImOnlineId;
use sc_service::ChainType;
use sp_consensus_aura::sr25519::AuthorityId as AuraId;
use sp_core::{sr25519, Pair, Public};
use sp_finality_grandpa::AuthorityId as GrandpaId;
use sp_runtime::traits::{IdentifyAccount, Verify};

// Note this is the URL for the telemetry server
//const STAGING_TELEMETRY_URL: &str = "wss://telemetry.polkadot.io/submit/";

/// Specialized `ChainSpec`. This is a specialization of the general Substrate ChainSpec type.
pub type ChainSpec = sc_service::GenericChainSpec<GenesisConfig>;

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

/// Helper function to generate an authority key for Aura
pub fn authority_keys_from_seed(
    seed: &str,
) -> (AccountId, AccountId, AuraId, GrandpaId, ImOnlineId) {
    (
        get_account_id_from_seed::<sr25519::Public>(seed),
        get_account_id_from_seed::<sr25519::Public>(&format!("{}//blockauthor", seed)),
        get_from_seed::<AuraId>(seed),
        get_from_seed::<GrandpaId>(seed),
        get_from_seed::<ImOnlineId>(seed),
    )
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
    let endowed_balance = 50 * constants::currency::DOLLARS;
    let constructor = move || {
        testnet_genesis(
            vec![authority_keys_from_seed("Alice")],
            get_account_id_from_seed::<sr25519::Public>("Alice"),
            testnet_assets(),
            endowed_gen![
                ("Alice", endowed_balance),
                ("Bob", endowed_balance),
                ("Alice//stash", endowed_balance),
                ("Bob//stash", endowed_balance),
            ],
            testnet_trustees(),
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
    let endowed_balance = 50 * constants::currency::DOLLARS;
    let constructor = move || {
        testnet_genesis(
            vec![
                authority_keys_from_seed("Alice"),
                authority_keys_from_seed("Bob"),
            ],
            get_account_id_from_seed::<sr25519::Public>("Alice"),
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
            testnet_trustees(),
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
const BTC_PRECISION: u8 = 8;
fn pcx() -> (AssetId, AssetInfo, AssetRestrictions) {
    (
        xpallet_protocol::PCX,
        AssetInfo::new::<Runtime>(
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

fn xbtc() -> (AssetId, AssetInfo, AssetRestrictions) {
    (
        xpallet_protocol::X_BTC,
        AssetInfo::new::<Runtime>(
            b"XBTC".to_vec(),
            b"ChainX Bitcoin".to_vec(),
            Chain::Bitcoin,
            BTC_PRECISION,
            b"ChainX's Cross-chain Bitcoin".to_vec(),
        )
        .unwrap(),
        AssetRestriction::DestroyFree.into(),
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

fn testnet_trustees() -> Vec<(
    Chain,
    TrusteeInfoConfig,
    Vec<(AccountId, Vec<u8>, Vec<u8>, Vec<u8>)>,
)> {
    let alice = get_account_id_from_seed::<sr25519::Public>("Alice");
    let bob = get_account_id_from_seed::<sr25519::Public>("Bob");
    let charlie = get_account_id_from_seed::<sr25519::Public>("Charlie");
    let btc = Chain::Bitcoin;
    let alice_hot = trustees::bitcoin::BtcTrusteeType::try_from(
        hex::decode("035b8fb240f808f4d3d0d024fdf3b185b942e984bba81b6812b8610f66d59f3a84")
            .expect(""),
    )
    .expect("");
    let alice_cold = trustees::bitcoin::BtcTrusteeType::try_from(
        hex::decode("0227e54b65612152485a812b8856e92f41f64788858466cc4d8df674939a5538c3")
            .expect(""),
    )
    .expect("");
    let bob_hot = trustees::bitcoin::BtcTrusteeType::try_from(
        hex::decode("02a79800dfed17ad4c78c52797aa3449925692bc8c83de469421080f42d27790ee")
            .expect(""),
    )
    .expect("");
    let bob_cold = trustees::bitcoin::BtcTrusteeType::try_from(
        hex::decode("020699bf931859cafdacd8ac4d3e055eae7551427487e281e3efba618bdd395f2f")
            .expect(""),
    )
    .expect("");
    let charlie_hot = trustees::bitcoin::BtcTrusteeType::try_from(
        hex::decode("0306117a360e5dbe10e1938a047949c25a86c0b0e08a0a7c1e611b97de6b2917dd")
            .expect(""),
    )
    .expect("");
    let charlie_cold = trustees::bitcoin::BtcTrusteeType::try_from(
        hex::decode("02a83c80e371ddf0a29006096765d060190bb607ec015ba6023b40ace582e13b99")
            .expect(""),
    )
    .expect("");

    let about = b"".to_vec();
    let collection = vec![
        (alice, about.clone(), alice_hot.into(), alice_cold.into()),
        (bob, about.clone(), bob_hot.into(), bob_cold.into()),
        (
            charlie,
            about.clone(),
            charlie_hot.into(),
            charlie_cold.into(),
        ),
    ];
    let config = TrusteeInfoConfig {
        min_trustee_count: 3,
        max_trustee_count: 15,
    };
    vec![(btc, config, collection)]
}

fn session_keys(aura: AuraId, grandpa: GrandpaId, im_online: ImOnlineId) -> SessionKeys {
    SessionKeys {
        grandpa,
        aura,
        im_online,
    }
}

fn testnet_genesis(
    initial_authorities: Vec<(AccountId, AccountId, AuraId, GrandpaId, ImOnlineId)>,
    root_key: AccountId,
    assets: Vec<(AssetId, AssetInfo, AssetRestrictions, bool, bool)>,
    endowed: BTreeMap<AssetId, Vec<(AccountId, Balance)>>,
    trustees: Vec<(
        Chain,
        TrusteeInfoConfig,
        Vec<(AccountId, Vec<u8>, Vec<u8>, Vec<u8>)>,
    )>,
    enable_println: bool,
) -> GenesisConfig {
    const ENDOWMENT: Balance = 10_000_000 * constants::currency::DOLLARS;
    const STASH: Balance = 100 * constants::currency::DOLLARS;

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

    let validators = {
        let staking_authorities = initial_authorities
            .iter()
            .map(|(s, _, _, _, _)| s)
            .collect::<Vec<_>>();
        balances
            .clone()
            .into_iter()
            .filter(|(v, _)| staking_authorities.contains(&v))
            .collect()
    };

    GenesisConfig {
        frame_system: Some(SystemConfig {
            code: WASM_BINARY.to_vec(),
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
        pallet_treasury: Some(Default::default()),
        pallet_elections_phragmen: Some(ElectionsConfig {
            members: endowed_accounts
                .iter()
                .take((num_endowed_accounts + 1) / 2)
                .cloned()
                .map(|member| (member, STASH))
                .collect(),
        }),
        pallet_im_online: Some(ImOnlineConfig { keys: vec![] }),
        pallet_session: Some(SessionConfig {
            keys: initial_authorities
                .iter()
                .map(|x| {
                    (
                        x.0.clone(),
                        x.0.clone(),
                        session_keys(x.2.clone(), x.3.clone(), x.4.clone()),
                    )
                })
                .collect::<Vec<_>>(),
        }),
        pallet_balances: Some(BalancesConfig { balances }),
        pallet_sudo: Some(SudoConfig { key: root_key }),
        xpallet_system: Some(XSystemConfig {
            network_props: NetworkType::Testnet,
        }),
        xpallet_assets: Some(XAssetsConfig {
            assets,
            endowed: endowed.clone(),
            memo_len: 128,
        }),
        xpallet_gateway_common: Some(XGatewayCommonConfig { trustees }),
        xpallet_gateway_bitcoin: {
            let (genesis_info, genesis_hash, network_id) =
                crate::res::load_mainnet_btc_genesis_header_info();
            Some(XGatewayBitcoinConfig {
                genesis_info,
                genesis_hash,
                network_id,
                params_info: BtcParams::new(
                    486604799,            // max_bits
                    2 * 60 * 60,          // block_max_future
                    2 * 7 * 24 * 60 * 60, // target_timespan_seconds
                    10 * 60,              // target_spacing_seconds
                    4,                    // retargeting_factor
                ), // retargeting_factor
                verifier: BtcTxVerifier::Recover,
                confirmation_number: 4,
                reserved_block: 2100,
                btc_withdrawal_fee: 500000,
                max_withdrawal_count: 100,
            })
        },
        xpallet_mining_staking: Some(XStakingConfig {
            validators,
            validator_count: 1000,
            minimum_validator_count: 4,
            sessions_per_era: 12,
            vesting_account: get_account_id_from_seed::<sr25519::Public>("vesting"),
            glob_dist_ratio: (12, 88), // (Treasury, X-type Asset and Staking) = (12, 88)
            mining_ratio: (10, 90),    // (Asset Mining, Staking) = (10, 90)
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
            ..Default::default()
        }),
        xpallet_contracts: Some(XContractsConfig {
            current_schedule: ContractsSchedule {
                enable_println, // this should only be enabled on development chains
                ..Default::default()
            },
        }),
    }
}
