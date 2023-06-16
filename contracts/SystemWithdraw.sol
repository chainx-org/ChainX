// SPDX-License-Identifier: MIT

pragma solidity ^0.8.0;

library SystemWithdraw {
    address constant private precompile = address(0x403);
    uint256 constant MIN_BTC_TRANSFER_VALUE = 10_000_000_000;

    event WithdrawBTC(address from, uint256 amount, string to);
    event WithdrawPCX(address from, uint256 amount, bytes32 to);

    function withdrawBTC(
        uint256 value,
        string calldata btcAddr
    ) public returns (bool) {
        (bool success, bytes memory returnData) = precompile.delegatecall(abi.encodePacked(false, value, btcAddr));

        require(success, string(returnData));

        emit WithdrawBTC(msg.sender, value / MIN_BTC_TRANSFER_VALUE, btcAddr);

        return success;
    }

    function withdrawPCX(
        uint256 value,
        bytes32 chainxPubkey
    ) public returns (bool) {
        (bool success, bytes memory returnData) = precompile.delegatecall(abi.encodePacked(true, value, chainxPubkey));

        require(success, string(returnData));

        emit WithdrawPCX(msg.sender, value, chainxPubkey);

        return success;
    }
}