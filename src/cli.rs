// Copyright 2018 chainpool

use clap::{Arg, App, SubCommand, ArgMatches};
use std::net::SocketAddr;

#[derive(Clone, Debug)]
pub enum ChainSpec {
    Dev, // One validator mode.
    Local, // Two validator mode.
    Multi, // Four validator mode.
}

pub fn build_cli() -> App<'static, 'static> {
    App::new("chainx")
        .version("0.1.0")
        .about("    Cross-Chain Asset Manager")
        .arg(
            Arg::with_name("chainspec")
            .long("chainspec")
            .value_name("CHAINSPEC")
            .help("Run in [dev|local|multi] mode, dev---single validator, local---2 validator testnet, multi---4 validator testnet")
            .takes_value(true)
        )
        .arg(
            Arg::with_name("port")
            .long("port")
            .value_name("PORT")
            .help("Specify p2p protocol TCP port")
            .takes_value(true),
            )
        .arg(
            Arg::with_name("bootnodes")
            .long("bootnodes")
            .value_name("URL")
            .help("Specify a list of bootnodes")
            .takes_value(true)
            .multiple(true),
            )
        .arg(
            Arg::with_name("db-path")
            .long("db-path")
            .value_name("DB")
            .help("Specify the database directory path")
            .takes_value(true),
            )
        .arg(
            Arg::with_name("rpc-port")
            .long("rpc-port")
            .value_name("PORT")
            .help("Specify HTTP RPC server TCP port")
            .takes_value(true),
            )
        .arg(
            Arg::with_name("ws-port")
            .long("ws-port")
            .value_name("PORT")
            .help("Specify WebSockets RPC server TCP port")
            .takes_value(true),
            )
        .subcommand(SubCommand::with_name("validator").about(
                "Enable validator mode",
                ).arg(
                    Arg::with_name("auth")
                    .long("auth")
                    .value_name("Validator")
                    .help("Select validator one of [alice|bob|gavin|satoshi]")
                    .takes_value(true)
                )
        )
}

pub fn parse_address(
    default: &str,
    port_param: &str,
    matches: &ArgMatches,
    ) -> Result<SocketAddr, String> {
    let mut address: SocketAddr = default.parse().ok().ok_or_else(|| {
        format!("Invalid address specified for --{}.", port_param)
    })?;
    if let Some(port) = matches.value_of(port_param) {
        let port: u16 = port.parse().ok().ok_or_else(|| {
            format!("Invalid port for --{} specified.", port_param)
        })?;
        address.set_port(port);
    }

    Ok(address)
}
