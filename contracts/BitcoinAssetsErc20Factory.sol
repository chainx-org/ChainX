// SPDX-License-Identifier: MIT

pragma solidity ^0.8.0;

import "./BitcoinAssetsErc20.sol";

contract BitcoinAssetsErc20Factory is Ownable {
    event BitcoinAsset(BitcoinAssetsErc20 token);

    function create(
        string memory name_,
        string memory symbol_,
        uint8 decimals_,
        address owner_,
        string memory protocol_,
        address admin_
    ) external onlyOwner {
        BitcoinAssetsErc20 token = new BitcoinAssetsErc20(
            name_,
            symbol_,
            decimals_,
            owner_,
            protocol_,
            admin_
        );

        emit BitcoinAsset(token);
    }
}