# Scripts for genesis generation

## `btc_genesis_params_gen.py`

Generate specific Bitcoin block header params given the Bitcoin block height.

Currently this script uses the API of https://blockstream.info/.

### Usage

```bash
$ ./btc_genesis_params_gen.py -h
```

Example:

```bash
# Generate block params from Bitcoin mainnet at height 576576.
$ ./btc_genesis_params_gen.py 576576

# Generate block params from Bitcoin testnet at height 576576.
$ ./btc_genesis_params_gen.py 576576 --network Testnet
```

## `generate_keys.sh`

This script can be used for generating the various keys for the multiple genesis validators, e.g., babe, grandpa. It's useful when you plan to start a brand new chain.

Note: If the output format of command `subkey key inspect-key` changes, this script probably should be updated accordingly, it's only guaranteed to work with the following output of `subkey key inspect-key`:

```
Secret phrase `bottom drive obey lake curtain smoke basket hold race lonely fit walk` is account:
  Secret seed:      0xfac7959dbfe72f052e5a0c3c8d6530f202b02fd8f9f5ca3580ec8deb7797479e
  Public key (hex): 0x46ebddef8cd9bb167dc30878d7113b7e168e6f0646beffd77d69d39bad76b47a
  Account ID:       0x46ebddef8cd9bb167dc30878d7113b7e168e6f0646beffd77d69d39bad76b47a
  SS58 Address:     5DfhGyQdFobKM8NsWvEeAKk5EQQgYe9AydgJ7rMB6E1EqRzV
```

### Usage

```bash
$ cargo build --release
$ cd scripts/genesis
$ export SECRET="YOUR SECRET"
$ bash generate_keys.sh
```
