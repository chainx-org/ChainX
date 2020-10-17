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
