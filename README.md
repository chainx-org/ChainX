# ChainX

<!-- TOC GFM -->

* [Introduction](#introduction)
* [Features](#features)
* [Roadmap](#roadmap)
    * [1.0: Independent Chain](#10-independent-chain)
    * [2.0: Polkadot Parachain](#20-polkadot-parachain)
    * [3.0: Polkadot level 2 multi-chain system](#30-polkadot-level-2-multi-chain-system)
* [Installation](#installation)
    * [Building from source](#building-from-source)
        * [Requirement](#requirement)
        * [Build the code](#build-the-code)
* [Testnet](#testnet)
    * [Validating on PoC 3](#validating-on-poc-3)
* [Development](#development)
    * [Run a local testnet](#run-a-local-testnet)
* [License](#license)

<!-- /TOC -->

❗️ To avoid distractions and move quickly, we make the hard choice of developing ChainX in a private codebase. Once it's been fully tested and audited with great care, we'll put it public again. Stay tuned!

----

## Introduction

For the time being the goal of [ChainX](https://github.com/chainx-org/ChainX) is to build a cross-chain digital asset management platform on the strength of [substrate](https://github.com/paritytech/substrate) which is next-generation framework for blockchain created by [paritytech](https://github.com/paritytech). The long-term vision of ChainX is to evolve as a general blockchain infrastructure platform.

<p align="center">
    <a href="http://chainx.org" target="_blank">
        <img width="800" alt="transparent" src="http://chainx.org/static/media/section2.0347a5e3.png">
    </a>
</p>

ChainX is still at a very early stage and in an active development. The instruction as followed is not stable and may change in the future.

## Features

- Built-in light client of existing blockchains.

- Built-in Coin DEX.

- Progressive staking and election machanism.

- And more.

## Roadmap

### 1.0: Independent Chain

ChainX 1.0 will operate as an independent chain at the very beginning, supporting the Coin DEX between the system currency PCX and BTC powed by BTC cross-chain transaction from the relay. At this stage, ChainX will continue to relay BCH, LTC, ZEC, ETH, DAI, ERC20, ERC721, ADA, EOS and other chains for Coin DEX.

### 2.0: Polkadot Parachain

ChainX 2.0 will begin at Q3 2019 when Polkadot releases v1. It will connect into Polkadot and transform into a para-chain, adding new applications such as decentralized stable currency collateralized by BTC and derivatives exchanges.

### 3.0: Polkadot level 2 multi-chain system

ChainX 3.0 will begin at 2020 when Polkadot releases v2, splited into a multi-chain architecture operating as Polkadot's level 2 relay network.

## Installation

### Building from source

#### Requirement

Refer to [Hacking on Substrate](https://github.com/paritytech/substrate#61-hacking-on-substrate) as well.

Ensure you have [Rust](https://www.rust-lang.org/) and the support software installed:

```bash
$ curl https://sh.rustup.rs -sSf | sh
$ rustup update nightly
$ rustup target add wasm32-unknown-unknown --toolchain nightly
$ rustup update stable
$ cargo install --git https://github.com/alexcrichton/wasm-gc
```

Ubuntu:

```bash
$ sudo apt install cmake pkg-config libssl-dev git
```

Mac:

```bash
$ brew install cmake pkg-config openssl git
```

#### Build the code

```bash
# Get the source code
$ git clone https://github.com/chainx-org/ChainX ~/ChainX
$ cd ~/ChainX

# Note: build ChainX with nightly
$ cargo +nightly build --release
```

When you succeed to build the project with `cargo build --release`, the `chainx` binary should be present in `target/release/chainx`.

## Testnet

Connect to the public testnet of ChainX:

```bash
# display status of your node on https://telemetry.polkadot.io/ via `--telemetry`
# customize your name on the telemetry page via `--name` 
$ RUST_LOG=info ./chainx --chainspec=dev --telemetry --name=YOUR_NAME --bootnodes=/ip4/47.93.16.189/tcp/20222/p2p/QmRdBJk8eVPjkHcxZvRAUZdWtTq96mWivJFc7tpJ8fUEGU --db-path=/tmp/chainx
```

### Validating on PoC 3

If you have succeeded to connect to our testnet, being a validator is not hard:

1. follow the instruction above to start a node util it is synced.

2. Create an account using [our web-based wallet](http://wallet.chainx.org). Save your seed and take note of your account's address.

3. File an application to be a validator in our telegram [chainx_org](https://t.me/chainx_org), which should include your name, url and ChainX address of validator account.

4. Rerestart your node with `--key=<seed> validator`:

    ```bash
    RUST_LOG=info ./chainx --chainspec=dev --telemetry --name=YOUR_NAME --bootnodes=/ip4/47.93.16.189/tcp/20222/p2p/QmRdBJk8eVPjkHcxZvRAUZdWtTq96mWivJFc7tpJ8fUEGU --db-path=/tmp/chainx --key=<seed> validator
    ```

## Development

We assume `chainx` is in your `$PATH` in the following sections. Run this command so that `chainx` could be found in `$PATH`:

```bash
$ export PATH=$(pwd)/target/release:$PATH
```

### Run a local testnet

Start a local chainx testnet by running:

```bash
$ chainx --chainspec=dev --key=Alice validator
```

## License

[GPL v3](LICENSE)
