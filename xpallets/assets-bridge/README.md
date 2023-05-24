# [deprecated]AssetsBridge
A bridge from [substrate assets(wasm)](../assets) into [ERC20 tokens(evm)](../../contracts/deprecated_AssetsBridgeErc20.sol).

## Overview

[Substrate & EVM address on ChainX](../../docs/substrate_and_evm_address_on_chainx.md)

In AssetsBridge
- substrate assets `->` erc20 tokens: `deposit`
- substrate assets `<-` erc20 tokens: `withdraw`
- native currency(wasm) `<->` eth(evm): `teleport`

## Dispatchable Functions
- for user:
  - `claim_account`: bond substrate account and evm address, will reserve some currency.
  - `dissolve`: unbond substrate account and evm address, will unreserve some currency.
  - `deposit`: move substrate assets into erc20 tokens.
  - `withdraw`: move back substrate assets from erc20 tokens.
  - `teleport`: transfer native currency between substrate account and evm address.
- for admin:
  - `register`: bond substrate assets and erc20 contract address.
  - `pause`: pause `deposit`, `withdraw` and `teleport(BackForeign)` when in emergency.
  - `unpause`: unpause the `paused` state.
- for sudo:
  - `set_admin`: set new the admin of `AssetsBridge`.
  - `force_unregister`: force unbond substrate assets and erc20 contract address.

In the production environment, the admin of assets-bridge must audits whether the erc20 contract
implements `IAssetsBridge` interface and whether it has the `AssetsBridgeAdmin` modifier.


## Work Flow

- (1) bond `Assets(wasm)` and `Tokens(evm)`: admin call `register`.
- (2) bond `Account(wasm)` and `Address(evm)`: user call `claim_account`.
- (3) move assets(wasm and evm):
  - `deposit`: burn from wasm and mint into evm.
  - `withdraw`: burn from evm and mint into wasm.
  - `teleport`: transfer in wasm.
- (4) maintenance：
  - for `sudo`: `set_admin`, `force_unregister`.
  - for `admin`: `pause`, `unpause`.
  - for `user`: `dissolve`.

## Eth Signed Data Format

```txt
"evm:" + substrate_pubkey_hex_without_0x
```
example:

```txt
substrate account: 5USGSZK3raH3LD4uxvNTa23HN5VULnYrkXonRktyizTJUYg9
it's pubkey(hex): 0xd43593c715fdd31c61141abd04a99fd6822c8558854ccde39a5684e7a56da27d
the sign data: "evm:d43593c715fdd31c61141abd04a99fd6822c8558854ccde39a5684e7a56da27d"
```

## companion with `relay`
- `Foreign assets`(on other chain) -> `ChainX assets` flow:
  - (1) `user` need `transfer` to the account which under the control of `assets-bridge admin` on `foreign chain`
  - (2) `mint` to `user` by `assets-bridge admin` on `ChainX`

- `ChainX assets` -> `Foreign assets`(on other chain) flow:
  - (1) `user` need `teleport` with `BackForeign(asset_id)` on `ChainX`. 
  - (2) the account which under the control of `assets-bridge admin` on `foreign chain` `transfer` to `user`

- `maintenance` by `admin`: `back_foreign` add or remove `asset_id` which can back foreign chain.
## Note

For safety, AssetsBridge now only allows dependent 
[AssetsBridge assets(wasm)](../assets) and 
[AssetsBridge tokens(evm)](../../contracts/deprecated_AssetsBridgeErc20.sol).
