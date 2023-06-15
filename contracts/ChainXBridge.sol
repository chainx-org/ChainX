// SPDX-License-Identifier: MIT
pragma solidity 0.8.13;

interface IBitcoinAssets {
    function burnFrom(address account, uint256 amount) external;
}

contract ChainXBridge {
    uint constant MIN_BTC_TRANSFER_VALUE = 10_000_000_000;
    address public cold;
    uint64 public chainId;
    uint64 public nonce;

    event Balance(uint256);

    event SwapOut(
        uint64 fromChainId,
        uint64 toChainId,
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

    constructor(
        address _cold,
        uint64 _chainId
    ){
        require(_cold != address(0), "InvalidCold");
        require(_chainId != 0, "InvalidChainId");

        cold = _cold;
        chainId = _chainId;
        nonce = 0;
    }



    function swap_out(
        uint64 toChainId,
        string calldata receiver,
        address token,
        uint256 amount,
        uint256 estGas
    ) external payable autoIncreaseNonce {
        require((cold != address(0)), "InvalidCold");
        // Less than MIN_BTC_TRANSFER_VALUE as 0
        estGas = estGas / MIN_BTC_TRANSFER_VALUE * MIN_BTC_TRANSFER_VALUE;
        emit Balance(estGas);
        require(estGas >= MIN_BTC_TRANSFER_VALUE, "InvalidEstGas");
        require(msg.value >= estGas, "ValueErr");

        uint256 old_balance = cold.balance;
        (bool sent, ) = payable(cold).call{value: estGas}("");
        require(sent, "Failed to send Ether");
        uint256 new_balance = cold.balance;
        require(new_balance > old_balance && new_balance == old_balance + estGas, "Unexpect");

        IBitcoinAssets(token).burnFrom(msg.sender, amount);

        emit SwapOut(
            chainId,
            toChainId,
            msg.sender,
            receiver,
            amount,
            estGas,
            token
        );
    }
}