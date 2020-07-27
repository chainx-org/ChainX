# Scripts for genesis generation

## `btc_genesis_header_gen.py`

Generate specific Bitcoin block header given the Bitcoin block height.

Currently this script uses the API of https://blockstream.info/.

### Usage

```bash
$ ./btc_genesis_header_gen.py -h
```

Example:

```bash
# Generate block header from Bitcoin mainnet at height 576576.
$ ./btc_genesis_header_gen.py 576576

# Generate block header from Bitcoin testnet at height 576576.
# ./btc_genesis_header_gen.py 576576 --network Testnet
```
