# ChainX

<!-- TOC GFM -->

* [Introduction](#introduction)
* [Installation](#installation)
    * [Building from source](#building-from-source)
        * [Requirement](#requirement)
        * [Build the code](#build-the-code)
* [Development](#development)
    * [Validator node](#validator-node)
        * [Local single validator](#local-single-validator)
        * [Local two validator node](#local-two-validator-node)
        * [Multiple validator node](#multiple-validator-node)
    * [Sync node](#sync-node)
        * [Public testnet](#public-testnet)
        * [Development](#development-1)
* [License](#license)

<!-- /TOC -->

## Introduction

For the time being the goal of [ChainX](https://github.com/chainx-org/ChainX) is to build a corss-chain digital asset management platform on the strength of [substrate](https://github.com/paritytech/substrate) which is next-generation framework for blockchain created by [paritytech](https://github.com/paritytech). The long-term vision of ChainX is to evolve as a general blockchain infrastrcutre platform.

ChainX is still at a very early stage and in an active development. The instruction as followed is not stable and may change in the future.

:tada: Run this command to connect to our public testnet:

```bash
$ chainx --chainspec=multi --telemetry --bootnodes=/ip4/47.105.73.172/tcp/30333/p2p/QmW7aJxigxGFXLmn966nJBBCexZA4nfSiydeg1JfmGFC9q --db-path=/tmp/chainx
```

## Installation

### Building from source

#### Requirement

Ensure you have [Rust](https://www.rust-lang.org/) and the support software installed:

Ubuntu:

```bash
$ curl https://sh.rustup.rs -sSf | sh
$ rustup update nightly
$ rustup target add wasm32-unknown-unknown --toolchain nightly
$ rustup update stable
$ cargo install --git https://github.com/alexcrichton/wasm-gc
$ sudo apt install cmake pkg-config libssl-dev git
```

#### Build the code

```bash
# Get the source code
$ git clone https://github.com/chainx-org/ChainX ~/ChainX
$ cd ~/ChainX

# Build all native code
$ cargo build
```

## Development

When you succeed to build the project with `cargo build`, the `chainx` binary should be present in `target/debug/chainx`.

We assume `chainx` is in your `$PATH` in the following sections. Run this command so that `chainx` could be found in `$PATH`:

```bash
$ export PATH=$(pwd)/target/debug:$PATH
```

See all the avaliable options and commands via `chainx -h`.

### Validator node

#### Local single validator

You can run a simple single-node development _network_ on your machine by running in a terminal:

```bash
$ RUST_LOG=info chainx --chainspec=dev --db-path=/tmp/dev-alice validator --auth=alice
```

Don't forget to run with `RUST_LOG=info` to see the logs, or you prefer to `export RUST_LOG=info` to avoid specifying every time.

```bash
$ export RUST_LOG=info
```

#### Local two validator node

Run the first node:

```bash
$ chainx --chainspec=local --db-path=/tmp/local-alice validator --auth=alice
INFO 2018-09-11T05:09:59Z: chainx: Chainspec is local mode
INFO 2018-09-11T05:09:59Z: substrate_client::client: Initialising Genesis block/state (state: 0x1529…4159, header-hash: 0xbcf4…9a00)
INFO 2018-09-11T05:09:59Z: substrate_network_libp2p::service: Local node address is: /ip4/127.0.0.1/tcp/20222/p2p/Qmevv1ggYD5dLf3MwAJ5zKeRGtnjfV7i85cPAsYwNaVW2o
INFO 2018-09-11T05:10:00Z: chainx: Auth is alice
......
```

Run the second node with option `bootnodes` from the address of first node:

```bash
$ chainx --chainspec=local --db-path=/tmp/local-bob --bootnodes=/ip4/127.0.0.1/tcp/20222/p2p/Qmevv1ggYD5dLf3MwAJ5zKeRGtnjfV7i85cPAsYwNaVW2o validator --auth=bob
```

#### Multiple validator node

Run the first node:

```bash
$ chainx --chainspec=multi --db-path=/tmp/multi-alice validator --auth=alice
```

Run the second node:

```bash
$ chainx --chainspec=multi --db-path=/tmp/multi-bob --bootnodes=/ip4/127.0.0.1/tcp/20222/p2p/QmWrZEJcYn3m8HeiHsYDVH1apitFF1h4ojyRYu9AjFkTuH validator --auth=bob
```

Run the third node:

```bash
$ chainx --chainspec=multi --db-path=/tmp/multi-gavin --bootnodes=/ip4/127.0.0.1/tcp/20222/p2p/QmWrZEJcYn3m8HeiHsYDVH1apitFF1h4ojyRYu9AjFkTuH validator --auth=gavin
```

These nodes won't be able to produce blocks until the number of validators is no less than 3.

We can add one more validator:

```bash
$ chainx --chainspec=multi --db-path=/tmp/multi-satoshi --bootnodes=/ip4/127.0.0.1/tcp/20222/p2p/QmWrZEJcYn3m8HeiHsYDVH1apitFF1h4ojyRYu9AjFkTuH validator --auth=satoshi
```

### Sync node

#### Public testnet

Run the following command to connect to our public testnet:

```bash
$ chainx --chainspec=multi --telemetry --bootnodes=/ip4/47.105.73.172/tcp/30333/p2p/QmW7aJxigxGFXLmn966nJBBCexZA4nfSiydeg1JfmGFC9q --db-path=/tmp/chainx
```

#### Development

Running `chainx` without `validator` subcommand is to synchronise to the chain, e.g., synchronise to a node in local mode:

```bash
$ chainx --chainspec=local --db-path=/tmp/local-sync --bootnodes=/ip4/127.0.0.1/tcp/20222/p2p/Qmevv1ggYD5dLf3MwAJ5zKeRGtnjfV7i85cPAsYwNaVW2o
```

## License

[GPL v3](LICENSE)
