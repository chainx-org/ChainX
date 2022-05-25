// SPDX-License-Identifier: MIT
pragma solidity ^0.8.4;

import "./StringUtils.sol";

contract NameHash {
    using StringUtils for *;
    function btc_bash() pure public returns(bytes32) {
        bytes32 btc_label = keccak256(bytes("btc"));
        return keccak256(abi.encodePacked(bytes32(0), btc_label));
    }

    function valid(string memory name) public pure returns(bool) {
        (uint nameUint,bool result) = name.strToUint();
        return name.strlen() == 5 && nameUint > 9999 && nameUint < 100000 && result;
    }

    function name_hash(string memory name) pure public returns(bytes32) {
        require(valid(name), "Only support [10000, 99999]");

        bytes32 label = keccak256(bytes(name));
        return keccak256(abi.encodePacked(btc_bash(), label));
    }
}
