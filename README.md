# BEVM
<img width="800" alt="WechatIMG475" src="https://github.com/btclayer2/BEVM/assets/9285062/eca6798f-b52c-45d1-8e7a-8d4c5c64890c">

## A community-driven BTC Layer2 project.
### Technical features:
- ***EVM:*** Fully compatible with EVM ecology, wallets such as metamask, development frameworks such as truffle/hardhat, and solidity programming language.
- ***BTC native gas:*** Use native BTC as the gas fee for EVM. Similar to ETH layer2 OP/Starknet, ETH is used as the gas fee of Layer2.
- ***Taproot Threshold Signature:*** On-chain POS nodes to ensure the decentralization of threshold signature verifiers. singal privacy communication protocol to ensure the security of the aggregated schnorr signature pubkey/msg.
- ***bitcoin light node:*** Light nodes on the BTC chain that support the Wasm version (no_std).
- ***Signal Privacy Distributed Protocol:*** [Signal protocol](https://en.wikipedia.org/wiki/Signal_Protocol) to ensure the privacy and security of message communication between nodes when schnorr aggregate signature and Mast contract combined threshold signature. 
- ***zkstark ultra-light node:*** To optimize the light nodes mentioned above, zkstark technology can be used to realize the ultra-light nodes of BTC.

### Taproot Threshold Signature
Musig2 is a multi-signature protocol that only needs two rounds of communication to complete communication. It is a continuation and upgrade of Musig, and its practicability is greatly improved. This repo fully reproduces the multi-signature scheme proposed by [Musig2](https://eprint.iacr.org/2020/1261) Paper which the version is `20210706:150749`.At the same time, we implemented versions for secp256k1 and sr25519, respectively, so that we can use the Musig2 protocol in BTC  and Polka.

### secp256k1

The naming of the functions, states, and variables are aligned with that of the protocol. At the same time, it is compatible with the schnorr signature process proposed by Bitcoin [bip340](https://github.com/bitcoin/bips/blob/master/bip-0340.mediawiki), making it applicable to the Bitcoin network.

### sr25519

Implements musig2 protocol on [Ristretto](https://ristretto.group/) compressed Ed25519 points.

## Contribution
Any kinds of contribution are highly welcome. Feel free to submit an issue if you have any question or run into any issues.

## Metamask config for BTC
```
Network name: BEVM
RPC URL: https://mainnet.bevm.io/rpc
Chain ID: 1501
Currency symbol: BTC
Block explorer URL (Optional): https://evm.bevm.io/
```

## License

[GPL v3](LICENSE)

# References

- [schnorrkel](https://github.com/w3f/schnorrkel)
- [multi-party-schnorr](https://github.com/ZenGo-X/multi-party-schnorr)
- [musig2](https://eprint.iacr.org/2020/1261)

