
// @ts-ignore
import { bytecode } from '../artifacts/contracts/core/UniswapV2Pair.sol/UniswapV2Pair.json'
import { keccak256 } from '@ethersproject/solidity'

const COMPUTED_INIT_CODE_HASH = keccak256(['bytes'], [`${bytecode}`])
console.log(COMPUTED_INIT_CODE_HASH)