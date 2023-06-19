// SPDX-License-Identifier: MIT
pragma solidity 0.8.13;

import "./SystemWithdraw.sol";

interface IBitcoinAssets {
    function burnFrom(address account, uint256 amount) external;
}

contract ChainXBridge {
    address public cold;
    uint64 public nonce;

    event Withdraw(
        address sender,
        string receiver,
        uint256 amount,
        uint256 estGas,
        address token
    );

    modifier autoIncreaseNonce() {
        nonce = nonce + 1;
        _;
    }

    constructor(address _cold){
        require(_cold != address(0), "InvalidCold");

        cold = _cold;
        nonce = 0;
    }

    function withdrawBTC(
        uint256 value,
        string calldata btcAddr
    ) external autoIncreaseNonce returns (bool) {
        return SystemWithdraw.withdrawBTC(value, btcAddr);
    }

    function withdrawPCX(
        uint256 value,
        bytes32 chainxPubkey
    ) external autoIncreaseNonce returns (bool) {
        return SystemWithdraw.withdrawPCX(value, chainxPubkey);
    }

    function withdraw(
        string calldata receiver,
        address token,
        uint256 amount,
        uint256 estGas
    ) external payable autoIncreaseNonce {
        require((cold != address(0)), "InvalidCold");
        // Less than MIN_BTC_TRANSFER_VALUE as 0
        uint256 MIN_BTC_TRANSFER_VALUE = SystemWithdraw.MIN_BTC_TRANSFER_VALUE;
        estGas = estGas / MIN_BTC_TRANSFER_VALUE * MIN_BTC_TRANSFER_VALUE;
        require(estGas >= MIN_BTC_TRANSFER_VALUE, "InvalidEstGas");
        require(msg.value >= estGas, "ValueErr");

        uint256 old_balance = cold.balance;
        (bool sent, ) = payable(cold).call{value: estGas}("");
        require(sent, "Failed to send Ether");
        uint256 new_balance = cold.balance;
        require(new_balance > old_balance && new_balance == old_balance + estGas, "Unexpect");

        IBitcoinAssets(token).burnFrom(msg.sender, amount);

        emit Withdraw(
            msg.sender,
            receiver,
            amount,
            estGas,
            token
        );
    }
}