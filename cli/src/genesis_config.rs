// Copyright 2018-2019 Chainpool.

use hex_literal::{hex, hex_impl};
use rustc_hex::FromHex;
use serde_derive::Deserialize;
use substrate_primitives::{crypto::UncheckedInto, ed25519::Public as AuthorityId};

use chainx_primitives::AccountId;
use chainx_runtime::{
    xaccounts::TrusteeInfoConfig,
    xassets::{self, Asset, Chain, ChainT},
    xbitcoin::{self, Params},
    Runtime,
};
use chainx_runtime::{
    ConsensusConfig, GenesisConfig, SessionConfig, SudoConfig, TimestampConfig, XAccountsConfig,
    XAssetsConfig, XAssetsProcessConfig, XBootstrapConfig, XBridgeOfBTCConfig, XBridgeOfSDOTConfig,
    XFeeManagerConfig, XSpotConfig, XStakingConfig, XTokensConfig,
};

use btc_chain::BlockHeader;
use btc_primitives::{h256_from_rev_str, Compact};

pub enum GenesisSpec {
    Dev,
    Local,
    Multi,
}

pub fn testnet_genesis(genesis_spec: GenesisSpec) -> GenesisConfig {
    // Load all sdot address and quantity.
    let sdot_claims = load_sdot_info().unwrap();

    // account pub and pri key
    let alice = hex!["471af9e69d41ee06426940fd302454662742405cb9dcc5bc68ceb7bec979e5e4"];
    let bob = hex!["806a491666670aa087e04770c025d64b2ecebfd91a74efdc4f4329642de32365"];
    let charlie = hex!["1cf70f57bf2a2036661819501164458bd6d94642d81b5e8f1d9bdad93bad49bb"];
    let satoshi = hex!["09a6acd8a6f4394c6ba8b5ea93ae0d473880823f357dd3fdfd5ff4ccf1fcad99"];
    //    let funding = hex!["c4387fd74bc774db3f9a2f6ea37b99218b1412677f20e25df4ff9043ed54e9ce"].into();
    let sudo_address: AccountId =
        hex!["c4387fd74bc774db3f9a2f6ea37b99218b1412677f20e25df4ff9043ed54e9ce"].unchecked_into();

    let auth1: AccountId = alice.unchecked_into();
    let auth2: AccountId = bob.unchecked_into();
    let auth3: AccountId = charlie.unchecked_into();
    let auth4: AccountId = satoshi.unchecked_into();
    let initial_authorities_len = match genesis_spec {
        GenesisSpec::Dev => 1,
        GenesisSpec::Local => 4,
        GenesisSpec::Multi => 4,
    };

    const CONSENSUS_TIME: u64 = 1;

    let pcx_precision = 8_u16;

    let btc_asset = Asset::new(
        <xbitcoin::Module<Runtime> as ChainT>::TOKEN.to_vec(), // token
        b"X-BTC".to_vec(),
        Chain::Bitcoin,
        8, // bitcoin precision
        b"ChainX's Cross-chain Bitcoin".to_vec(),
    )
    .unwrap();

    let sdot_asset = Asset::new(
        b"SDOT".to_vec(), // token
        b"Shadow DOT".to_vec(),
        Chain::Ethereum,
        3, //  precision
        b"ChainX's Shadow Polkadot from Ethereum".to_vec(),
    )
    .unwrap();

    let apply_prec = |x| (x * 10_u64.pow(pcx_precision as u32) as f64) as u64;

    let mut full_endowed = vec![
        (
            auth1,                 // auth
            apply_prec(12.5),      // balance
            b"Alice".to_vec(),     // name
            b"Alice.com".to_vec(), // url
            "03f72c448a0e59f48d4adef86cba7b278214cece8e56ef32ba1d179e0a8129bdba"
                .from_hex()
                .unwrap(), // hot_entity
            "02a79800dfed17ad4c78c52797aa3449925692bc8c83de469421080f42d27790ee"
                .from_hex()
                .unwrap(), // cold_entity
        ),
        (
            auth2,
            apply_prec(12.5),
            b"Bob".to_vec(),
            b"Bob.com".to_vec(),
            "0306117a360e5dbe10e1938a047949c25a86c0b0e08a0a7c1e611b97de6b2917dd"
                .from_hex()
                .unwrap(),
            "03ece1a20b5468b12fd7beda3e62ef6b2f6ad9774489e9aff1c8bc684d87d70780"
                .from_hex()
                .unwrap(),
        ),
        (
            auth3,
            apply_prec(12.5),
            b"Charlie".to_vec(),
            b"Charlie.com".to_vec(),
            "0311252930af8ba766b9c7a6580d8dc4bbf9b0befd17a8ef7fabac275bba77ae40"
                .from_hex()
                .unwrap(),
            "02e34d10113f2dd162e8d8614a4afbb8e2eb14eddf4036042b35d12cf5529056a2"
                .from_hex()
                .unwrap(),
        ),
        (
            auth4,
            apply_prec(12.5),
            b"Satoshi".to_vec(),
            b"Satoshi.com".to_vec(),
            "0227e54b65612152485a812b8856e92f41f64788858466cc4d8df674939a5538c3"
                .from_hex()
                .unwrap(),
            "020699bf931859cafdacd8ac4d3e055eae7551427487e281e3efba618bdd395f2f"
                .from_hex()
                .unwrap(),
        ),
    ];

    full_endowed.truncate(initial_authorities_len);

    let endowed = full_endowed
        .clone()
        .into_iter()
        .map(|(auth, balance, _, _, _, _)| (auth, balance))
        .collect::<Vec<(AuthorityId, _)>>();

    let blocks_per_session = 150; // 150 blocks per session
    let sessions_per_era = 2; // update validators set per 12 sessions
    let sessions_per_epoch = sessions_per_era * 10; // update trustees set per 12*10 sessions
    let bonding_duration = blocks_per_session * sessions_per_era; // freeze 150*12 blocks for non-intention
    let intention_bonding_duration = bonding_duration * 10; // freeze 150*12*10 blocks for intention

    let btc_genesis = (
        BlockHeader {
            version: 536870912,
            previous_header_hash: h256_from_rev_str(
                "00000000000000f51b45a318afa95726b947aeb5154e65fd1bde2e2a798e2cc6",
            ),
            merkle_root_hash: h256_from_rev_str(
                "c9212ace69e32dcfe2d09383c7b8fea8da1151cd70af58ea2b7155afa5e92f47",
            ),
            time: 1553231960,
            bits: Compact::new(436271905),
            nonce: 4083801566,
        },
        1485555,
    );

    let params_info = Params::new(
        520159231,            // max_bits
        2 * 60 * 60,          // block_max_future
        2 * 7 * 24 * 60 * 60, // target_timespan_seconds
        10 * 60,              // target_spacing_seconds
        4,                    // retargeting_factor
    );

    GenesisConfig {
        consensus: Some(ConsensusConfig {
            code: include_bytes!("./chainx_runtime.compact.wasm").to_vec(),
            authorities: endowed
                .iter()
                .cloned()
                .map(|(account, _)| account.into())
                .collect(),
        }),
        system: None,
        timestamp: Some(TimestampConfig {
            minimum_period: CONSENSUS_TIME, // 2 second block time.
        }),
        xsession: Some(SessionConfig {
            validators: endowed
                .iter()
                .cloned()
                .map(|(account, balance)| (account.into(), balance))
                .collect(),
            session_length: blocks_per_session,
            keys: endowed
                .iter()
                .cloned()
                .map(|(account, _)| (account.clone().into(), account.into()))
                .collect(),
        }),
        sudo: Some(SudoConfig { key: sudo_address }),
        // chainx runtime module
        xfee_manager: Some(XFeeManagerConfig {
            producer_fee_proportion: (1, 10),
            transaction_base_fee: 10000,
            transaction_byte_fee: 100,
        }),
        xaccounts: Some(XAccountsConfig {
            trustee_info_config: vec![(
                Chain::Bitcoin,
                TrusteeInfoConfig {
                    min_trustee_count: 4,
                    max_trustee_count: 15,
                },
            )],
            _genesis_phantom_data: Default::default(),
        }),
        xassets: Some(XAssetsConfig {
            memo_len: 128,
            _genesis_phantom_data: Default::default(),
        }),
        xprocess: Some(XAssetsProcessConfig {
            token_black_list: vec![sdot_asset.token()],
            _genesis_phantom_data: Default::default(),
        }),
        xstaking: Some(XStakingConfig {
            initial_reward: apply_prec(50.0),
            validator_count: 100,
            minimum_validator_count: 4,
            sessions_per_era,
            sessions_per_epoch,
            bonding_duration,
            intention_bonding_duration,
            current_era: 0,
            minimum_penalty: 10_000_000, // 0.1 PCX by default
        }),
        xtokens: Some(XTokensConfig {
            token_discount: vec![
                (xbitcoin::Module::<Runtime>::TOKEN.to_vec(), 50),
                (sdot_asset.token(), 100),
            ],
            _genesis_phantom_data: Default::default(),
        }),
        xspot: Some(XSpotConfig {
            price_volatility: 10,
            _genesis_phantom_data: Default::default(),
        }),
        xsdot: Some(XBridgeOfSDOTConfig {
            claims: sdot_claims,
        }),
        xbitcoin: Some(XBridgeOfBTCConfig {
            // start genesis block: (genesis, blocknumber)
            genesis: btc_genesis.clone(),
            params_info: params_info.clone(), // retargeting_factor
            confirmation_number: 6,
            reserved_block: 2100,
            btc_withdrawal_fee: 40000,
            max_withdrawal_count: 10,
            _genesis_phantom_data: Default::default(),
        }),
        xbootstrap: Some(XBootstrapConfig {
            // xassets
            pcx: (
                b"Polkadot ChainX".to_vec(),
                pcx_precision,
                b"ChainX's crypto currency in Polkadot ecology".to_vec(),
            ),
            // asset, is_online, is_psedu_intention, init for account
            // Vec<(Asset, bool, Vec<(T::AccountId, u64)>)>;
            asset_list: vec![
                (btc_asset.clone(), true, true, vec![]),
                (sdot_asset.clone(), true, true, vec![]),
            ],
            // xstaking
            intentions: full_endowed
                .clone()
                .into_iter()
                .map(|(who, value, name, url, _, _)| (who.into(), value, name, url))
                .collect(),
            trustee_intentions: full_endowed
                .into_iter()
                .map(|(who, _, _, _, hot_entity, cold_entity)| {
                    (who.into(), hot_entity, cold_entity)
                })
                .collect(),
            // xtokens
            endowed_users: vec![(btc_asset.token(), vec![]), (sdot_asset.token(), vec![])],
            // xspot
            pair_list: vec![
                (
                    xassets::Module::<Runtime>::TOKEN.to_vec(),
                    xbitcoin::Module::<Runtime>::TOKEN.to_vec(),
                    9,
                    2,
                    100000,
                    true,
                ),
                (
                    sdot_asset.token(),
                    xassets::Module::<Runtime>::TOKEN.to_vec(),
                    4,
                    2,
                    100000,
                    true,
                ),
            ],
            // xgrandpa
            authorities: endowed.clone(),
            // xbitcoin
            genesis: btc_genesis,
            params_info,
            network_id: 1,
            multisig_init_info: (
                endowed
                    .iter()
                    .cloned()
                    .map(|(account, _)| (account.into(), true))
                    .collect(),
                3,
            ),
        }),
    }
}

#[derive(Debug, Deserialize)]
pub struct RecordOfSDOT {
    tx_hash: String,
    block_number: u64,
    unix_timestamp: u64,
    date_time: String,
    from: String,
    to: String,
    quantity: f64,
}

fn load_sdot_info() -> Result<Vec<([u8; 20], u64)>, Box<dyn std::error::Error>> {
    let mut reader = csv::Reader::from_reader(&include_bytes!("dot_tx.csv")[..]);
    let mut res = Vec::with_capacity(3053);
    for result in reader.deserialize() {
        let record: RecordOfSDOT = result?;
        let mut sdot_addr = [0u8; 20];
        sdot_addr.copy_from_slice(&record.to[2..].from_hex::<Vec<u8>>()?);
        res.push((sdot_addr, (record.quantity * 1000.0).round() as u64));
    }
    Ok(res)
}

#[test]
fn test_quantity_sum() {
    let res = load_sdot_info().unwrap();
    let sum: u64 = res.iter().map(|(_, quantity)| *quantity).sum();
    assert_eq!(sum, 4999466375u64 + 5 * 20 * 1000);
}
