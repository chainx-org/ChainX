// SPDX-License-Identifier: MIT
pragma solidity >=0.8.4;

import "../registry/ENS.sol";
import "../registry/ReverseRegistrar.sol";

/**
 * @dev Provides a default implementation of a resolver for reverse records,
 * which permits only the owner to update it.
 */
contract DefaultReverseResolver {

    ENS public ens;
    mapping (bytes32 => string) public name;

    /**
     * @dev Only permits calls by the reverse registrar.
     * @param node The node permission is required for.
     */
    modifier onlyOwner(bytes32 node) {
        require(msg.sender == ens.owner(node));
        _;
    }

    /**
     * @dev Constructor
     * @param ensAddr The address of the ENS registry.
     */
    constructor(ENS ensAddr) {
        ens = ensAddr;

        // Assign ownership of the reverse record to our deployer
        ReverseRegistrar registrar = ReverseRegistrar(ens.owner(ADDR_REVERSE_NODE));
        if (address(registrar) != address(0x0)) {
            registrar.claim(msg.sender);
        }
    }

    /**
     * @dev Sets the name for a node.
     * @param node The node to update.
     * @param _name The name to set.
     */
    function setName(bytes32 node, string memory _name) public onlyOwner(node) {
        name[node] = _name;
    }
}
