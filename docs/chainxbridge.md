# ChainXBridge

Through ChainXBridge, assets in BEVM(formerly called `chainx-evm`) and Bitcoin can circulate freely

![chainx-v5](https://github.com/chainx-org/ChainX/assets/8869892/9ba9afdc-c738-4392-8f05-2d9d91d3b3bd)

## 1. Deposit

### 1.1 BTC(bitcoin -> bevm)
TODO

### 1.2 BRC20(bitcoin -> bevm)
TODO

### 1.3 PCX(chainx -> bevm)
ChainX wallet: https://dapp.chainx.org/#/chainstate/extrinsics

xAssetsBridge -> depositPcxToEvm

100000000 means 1 PCX

![deposit-pcx](./deposit-pcx.png)

PCX erc20 address: `0xf3607524cAB05762cB5F0cAb17e4cA3A0F0b4E87`

metamask wallet

![pcx-metamask](./pcx-metamask.png)

### 1.4 XBTC => BTC(chainx -> bevm)
ChainX wallet: https://dapp.chainx.org/#/chainstate/extrinsics

xAssetsBridge -> swapXbtcToBtc

1000 means 0.00001000 XBTC

![btc-to-evm](./btc-to-evm.png)

### 1.5 BTC(chainx -> bevm)
ChainX wallet: https://dapp.chainx.org/#/chainstate/extrinsics

xAssetsBridge -> transferBtcToEvm

1000 means 0.00001000 BTC

![transfer-btc-to-evm](./transfer-btc-to-evm.png)

## 2. Withdraw

### 2.1 BTC(bevm -> bitcoin)
TODO

### 2.2 BRC20(bevm -> bitcoin)
TODO

### 2.3 PCX(bevm -> chainx)
TODO

## 3. Query balance

### 3.1 BTC(bevm)

![btc-evm](./btc-evm.png)

### 3.2 BTC(wasm)

`0x6CfE5574639Ba46d74b6b67D2651d1470E10BA9a` mapping to `5UKCmTuj5V4wmGnmZYm6RSDbT2mqwf2FUMTJy7phjwmwE51H`

https://evm.chainx.org/tools/SS58

![evm-to-chainx](./evm-to-chainx.png)

https://dapp.chainx.org/#/chainstate/chainstate

![btc-balance](./btc-balance.png)

### 3.3 PCX(bevm)

![pcx-evm](./pcx-evm.png)

### 3.4 PCX(wasm)

https://dapp.chainx.org/#/chainstate/chainstate

![pcx-balance](./pcx-balance.png)