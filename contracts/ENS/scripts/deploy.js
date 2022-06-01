const hre = require("hardhat");
const namehash = require('eth-ens-namehash');
const sleep = require('sleep-promise');
const ethers = hre.ethers
const labelhash = (label) => ethers.utils.keccak256(ethers.utils.toUtf8Bytes(label))

const ZERO_HASH = "0x0000000000000000000000000000000000000000000000000000000000000000";

async function main() {
    const ENSRegistry = await ethers.getContractFactory("ENSRegistry")
    const StablePriceOracle = await ethers.getContractFactory("StablePriceOracle")
    const PublicResolver = await ethers.getContractFactory("PublicResolver")
    const BaseRegistrarImplementation = await ethers.getContractFactory("BaseRegistrarImplementation")
    const StringUtils = await ethers.getContractFactory("StringUtils")
    const DefaultReverseResolver = await ethers.getContractFactory("DefaultReverseResolver")
    const ReverseRegistrar = await ethers.getContractFactory("ReverseRegistrar")

    const signers = await ethers.getSigners();
    const accounts = signers.map(s => s.address)
    console.log("deployer",accounts[0])

    const ens = await ENSRegistry.deploy()
    await ens.deployed()
    console.log("ENSRegistry",ens.address)

    //注册10分钟10+5=15个pcx,续租10分钟15个pcx
    const price = await StablePriceOracle.deploy("317097919837","5000000000000000000","475646879756")
    await price.deployed()
    console.log("StablePriceOracle",price.address)

    const resolver = await PublicResolver.deploy(ens.address);
    await resolver.deployed()
    console.log("PublicResolver",resolver.address)

    const resolverNode = namehash.hash("btc");
    const resolverLabel = labelhash("btc");
    await ens.setSubnodeOwner(ZERO_HASH, resolverLabel, accounts[0]);
    await sleep(6000)
    await ens.setResolver(resolverNode, resolver.address);

    const base = await  BaseRegistrarImplementation.deploy(ens.address, namehash.hash("btc"));
    await base.deployed()
    console.log("BaseRegistrarImplementation",base.address)

    const stringUtils = await StringUtils.deploy()
    await stringUtils.deployed()

    const ETHRegistrarController = await ethers.getContractFactory("ETHRegistrarController",{
        libraries: {
            StringUtils: stringUtils.address
        }
    })
    const controller = await  ETHRegistrarController.deploy(base.address, price.address,60,86400);
    await controller.deployed()
    await resolver.setInterface(namehash.hash("btc"),"0x018fac06",controller.address)
    await ens.setSubnodeOwner(ZERO_HASH, labelhash("btc"), base.address);
    await base.addController(controller.address)
    console.log("ETHRegistrarController",controller.address)

    const defaultReverseResolver = await DefaultReverseResolver.deploy(ens.address);
    await defaultReverseResolver.deployed()
    console.log("defaultReverseResolver",defaultReverseResolver.address)
    const reverseRegistrar = await ReverseRegistrar.deploy(ens.address, defaultReverseResolver.address);
    await reverseRegistrar.deployed()
    console.log("reverseRegistrar",reverseRegistrar.address)
    await ens.setSubnodeOwner(ZERO_HASH, labelhash("reverse"), accounts[0]);
    await sleep(6000)
    await ens.setSubnodeOwner(namehash.hash("reverse"), labelhash("addr"), reverseRegistrar.address);
}

// We recommend this pattern to be able to use async/await everywhere
// and properly handle errors.
main()
    .then(() => process.exit(0))
    .catch((error) => {
        console.error(error);
        process.exit(1);
    });