// Copyright 2019 Chainpool

use chainx_runtime::GenesisConfig;
use genesis_config::{testnet_genesis, GenesisSpec};
use substrate_service;

const STAGING_TELEMETRY_URL: &str = "wss://telemetry.polkadot.io/submit/";

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
        Some(STAGING_TELEMETRY_URL.into()),
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
        None,
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
    ChainSpec::from_genesis(
        "ChainX V0.9.3",
        "chainx_testnet",
        local_testnet_genesis,
        vec![],
        Some(STAGING_TELEMETRY_URL.into()),
        None,
        None,
        None,
    )
}
