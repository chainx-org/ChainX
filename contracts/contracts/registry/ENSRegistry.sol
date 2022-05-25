// SPDX-License-Identifier: MIT
pragma solidity >=0.8.4;

import "./ENS.sol";

/**
 * The ENS registry contract.
 */
contract ENSRegistry is ENS {

    struct Record {
        address owner;
        address resolver;
        uint64 ttl;
    }

    mapping (bytes32 => Record) records;
    mapping (address => mapping(address => bool)) operators;

    // Permits modifications only by the owner of the specified node.
    modifier authorised(bytes32 _node) {
        address currOwner = records[_node].owner;
        require(currOwner == msg.sender || operators[currOwner][msg.sender]);
        _;
    }

    /**
     * @dev Constructs a new ENS registrar.
     */
    constructor() {
        records[0x0].owner = msg.sender;
    }

    /**
     * @dev Sets the record for a node.
     * @param _node The node to update.
     * @param _owner The address of the new owner.
     * @param _resolver The address of the resolver.
     * @param _ttl The TTL in seconds.
     */
    function setRecord(bytes32 _node, address _owner, address _resolver, uint64 _ttl) external virtual override {
        setOwner(_node, _owner);
        _setResolverAndTTL(_node, _resolver, _ttl);
    }

    /**
     * @dev Sets the record for a subnode.
     * @param _node The parent node.
     * @param _label The hash of the label specifying the subnode.
     * @param _owner The address of the new owner.
     * @param _resolver The address of the resolver.
     * @param _ttl The TTL in seconds.
     */
    function setSubnodeRecord(bytes32 _node, bytes32 _label, address _owner, address _resolver, uint64 _ttl) external virtual override {
        bytes32 subnode = setSubnodeOwner(_node, _label, _owner);
        _setResolverAndTTL(subnode, _resolver, _ttl);
    }

    /**
     * @dev Transfers ownership of a node to a new address. May only be called by the current _owner of the node.
     * @param _node The node to transfer ownership of.
     * @param _owner The address of the new _owner.
     */
    function setOwner(bytes32 _node, address _owner) public virtual override authorised(_node) {
        _setOwner(_node, _owner);
        emit Transfer(_node, _owner);
    }

    /**
     * @dev Transfers ownership of a subnode keccak256(_node, _label) to a new address. May only be called by the _owner of the parent _node.
     * @param _node The parent _node.
     * @param _label The hash of the _label specifying the subnode.
     * @param _owner The address of the new _owner.
     */
    function setSubnodeOwner(bytes32 _node, bytes32 _label, address _owner) public virtual override authorised(_node) returns(bytes32) {
        bytes32 subnode = keccak256(abi.encodePacked(_node, _label));
        _setOwner(subnode, _owner);
        emit NewOwner(_node, _label, _owner);
        return subnode;
    }

    /**
     * @dev Sets the _resolver address for the specified _node.
     * @param _node The _node to update.
     * @param _resolver The address of the _resolver.
     */
    function setResolver(bytes32 _node, address _resolver) public virtual override authorised(_node) {
        emit NewResolver(_node, _resolver);
        records[_node].resolver = _resolver;
    }

    /**
     * @dev Sets the TTL for the specified _node.
     * @param _node The _node to update.
     * @param _ttl The TTL in seconds.
     */
    function setTTL(bytes32 _node, uint64 _ttl) public virtual override authorised(_node) {
        emit NewTTL(_node, _ttl);
        records[_node].ttl = _ttl;
    }

    /**
     * @dev Enable or disable approval for a third party ("operator") to manage
     *  all of `msg.sender`'s ENS records. Emits the ApprovalForAll event.
     * @param operator Address to add to the set of authorized operators.
     * @param approved True if the operator is approved, false to revoke approval.
     */
    function setApprovalForAll(address operator, bool approved) external virtual override {
        operators[msg.sender][operator] = approved;
        emit ApprovalForAll(msg.sender, operator, approved);
    }

    /**
     * @dev Returns the address that owns the specified _node.
     * @param _node The specified _node.
     * @return address of the _owner.
     */
    function owner(bytes32 _node) public virtual override view returns (address) {
        address addr = records[_node].owner;
        if (addr == address(this)) {
            return address(0x0);
        }

        return addr;
    }

    /**
     * @dev Returns the address of the _resolver for the specified _node.
     * @param _node The specified _node.
     * @return address of the _resolver.
     */
    function resolver(bytes32 _node) public virtual override view returns (address) {
        return records[_node].resolver;
    }

    /**
     * @dev Returns the TTL of a _node, and any records associated with it.
     * @param _node The specified _node.
     * @return ttl of the _node.
     */
    function ttl(bytes32 _node) public virtual override view returns (uint64) {
        return records[_node].ttl;
    }

    /**
     * @dev Returns whether a record has been imported to the registry.
     * @param _node The specified _node.
     * @return Bool if record exists
     */
    function recordExists(bytes32 _node) public virtual override view returns (bool) {
        return records[_node].owner != address(0x0);
    }

    /**
     * @dev Query if an address is an authorized operator for another address.
     * @param _owner The address that owns the records.
     * @param operator The address that acts on behalf of the _owner.
     * @return True if `operator` is an approved operator for `_owner`, false otherwise.
     */
    function isApprovedForAll(address _owner, address operator) external virtual override view returns (bool) {
        return operators[_owner][operator];
    }

    function _setOwner(bytes32 _node, address _owner) internal virtual {
        records[_node].owner = _owner;
    }

    function _setResolverAndTTL(bytes32 _node, address _resolver, uint64 _ttl) internal {
        if(_resolver != records[_node].resolver) {
            records[_node].resolver = _resolver;
            emit NewResolver(_node, _resolver);
        }

        if(_ttl != records[_node].ttl) {
            records[_node].ttl = _ttl;
            emit NewTTL(_node, _ttl);
        }
    }
}
