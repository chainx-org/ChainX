// Copyright 2018-2019 Chainpool.

use telemetry::TelemetryEndpoints;

use chainx_runtime::GenesisConfig;

use super::genesis_config::{testnet_genesis, GenesisSpec};

const STAGING_TELEMETRY_URL: &str = "wss://telemetry.polkadot.io/submit/";
const CHAINX_TELEMETRY_URL: &str = "wss://stats.chainx.org/submit/";

/// Specialised `ChainSpec`.
pub type ChainSpec = substrate_service::ChainSpec<GenesisConfig>;

/// Staging testnet config.
pub fn staging_testnet_config() -> ChainSpec {
    let boot_nodes = vec![];
    ChainSpec::from_genesis(
        "ChainX Staging Testnet",
        "chainx_staging_testnet",
        staging_testnet_config_genesis,
        boot_nodes,
        Some(TelemetryEndpoints::new(vec![
            (STAGING_TELEMETRY_URL.to_string(), 0),
            (CHAINX_TELEMETRY_URL.to_string(), 0),
        ])),
        None,
        None,
        None,
    )
}

fn staging_testnet_config_genesis() -> GenesisConfig {
    testnet_genesis(GenesisSpec::Multi)
}

fn development_config_genesis() -> GenesisConfig {
    testnet_genesis(GenesisSpec::Dev)
}

/// Development config (single validator Alice)
pub fn development_config() -> ChainSpec {
    ChainSpec::from_genesis(
        "Development",
        "dev",
        development_config_genesis,
        vec![],
        Some(TelemetryEndpoints::new(vec![(
            CHAINX_TELEMETRY_URL.to_string(),
            0,
        )])),
        None,
        None,
        None,
    )
}

fn local_testnet_genesis() -> GenesisConfig {
    testnet_genesis(GenesisSpec::Local)
}

/// Local testnet config (multivalidator Alice + Bob)
pub fn local_testnet_config() -> ChainSpec {
    let boot_nodes = vec![
    //        "/ip4/47.96.134.203/tcp/31126/p2p/QmRZ2URFsxZ2uCjr4cdrP9QvHr8qQg5gXMBhLsVchgNi4S".into(),
    //        "/ip4/47.96.97.52/tcp/31127/p2p/QmeCNS75ZRHAUsAUtH6DjcaxVFWwkBaT5KQtx91WvAh9i5".into(),
    //        "/ip4/47.110.232.108/tcp/31129/p2p/QmQ7vca7aum3q1toVPUf8T6SiUPdhFuMPpyJgYAxeoUXtf".into(),
        ];
    ChainSpec::from_genesis(
        "ChainX Local V0.9.8",
        "chainx_testnet",
        local_testnet_genesis,
        boot_nodes,
        Some(TelemetryEndpoints::new(vec![
            (STAGING_TELEMETRY_URL.to_string(), 0),
            (CHAINX_TELEMETRY_URL.to_string(), 0),
        ])),
        None,
        None,
        None,
    )
}
