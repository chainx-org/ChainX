# Deploy Uniswap V2 to Moonbase Alpha (or Standalone node)

This is a Hardhat setup to deploy the necessary contracts of Uniswap.

## Get Started

Clone repo:

```
git clone https://github.com/albertov19/uniswap-contracts-moonbeam/
cd uniswap-contracts-moonbeam
```

Install packages:

```
npm i
```

Modify the private keys as you wish in the `hardhat.config.js` file.

### Deploy the contracts (Standalone)

To deploy the contracts in a Standalone node you can run:

```
npx hardhat run --network dev scripts/deploy-uniswap.js
```

Contracts will be deployed if a Standalone node is running (default 9933 port is used).

**Note: the interface will only work if the contracts are deployed in a fresh instance. As contacts addressess are saved so that they match that order of deployment**

### Deploy the contracts (Moonbase Alpha):

To deploy the contracts in Moonbase Alpha you can run:

```
npx hardhat run --network moonbase scripts/deploy-uniswap.js
```

**Note: the interface works on Moonbase Alpha with the contracts address baked in the SDK. To make sure that the interface works with your deployment you need to modify both the Interface and SDK repos**
