use chainx_runtime::GenesisConfig;
use substrate_service;
use genesis_config::{testnet_genesis, GenesisSpec};

const STAGING_TELEMETRY_URL: &str = "ws://stats.chainx.org/submit/";

/// Specialised `ChainSpec`.
pub type ChainSpec = substrate_service::ChainSpec<GenesisConfig>;

/// Staging testnet config.
pub fn staging_testnet_config() -> ChainSpec {
	let boot_nodes = vec![
	];
	ChainSpec::from_genesis(
		"Staging Testnet",
		"staging_testnet",
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
	ChainSpec::from_genesis("Development", "development", development_config_genesis, vec![], None, None, None, None)
}

fn local_testnet_genesis() -> GenesisConfig {
	testnet_genesis(GenesisSpec::Local)
}

/// Local testnet config (multivalidator Alice + Bob)
pub fn local_testnet_config() -> ChainSpec {
	ChainSpec::from_genesis("Local Testnet", "local_testnet", local_testnet_genesis, vec![], None, None, None, None)
}

