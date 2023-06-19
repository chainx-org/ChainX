# Deploy Uniswap V2 to ChainX

This is a Hardhat setup to deploy the necessary contracts of Uniswap.
Forked from [moonbeam-uniswap](https://github.com/PureStake/moonbeam-uniswap)

## Get Started

Clone repo:

```
git clone https://github.com/chainx-org/ChainX.git
cd contracts/uniswap-contracts
```

Set PRIVKEY env:
```
export PRIVKEY=0x.....
```

Update hardhat.config.js:
default url
```
url: 'http://127.0.0.1:8546'
```


Install packages:

```
npm i
```

Modify the private keys as you wish in the `hardhat.config.js` file.

### Deploy the contracts (Standalone)

To deploy the contracts in a Standalone node you can run:

#### Script
##### calc init_code_hash
```bash
export PRIVKEY= Your privateKey

npx hardhat compile
ts-node scripts/init_code_hash.ts --resolveJsonModule
```

##### deploy
```
export PRIVKEY= Your privateKey
npx hardhat run --network dev scripts/deploy-factory.js 
```

#### Remix

To collect to localhost

```
bash ./script/remix.sh 
```

setting
* evmVersion: istanbul
* Enable optimization: true

