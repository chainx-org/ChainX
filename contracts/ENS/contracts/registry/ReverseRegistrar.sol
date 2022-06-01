// SPDX-License-Identifier: MIT
pragma solidity >=0.8.4;

import "./ENS.sol";
import "@openzeppelin/contracts/access/Ownable.sol";
import "../root/Controllable.sol";

abstract contract NameResolver {
    function setName(bytes32 node, string memory name) public virtual;
}

bytes32 constant lookup = 0x3031323334353637383961626364656600000000000000000000000000000000;

bytes32 constant ADDR_REVERSE_NODE = 0x91d1777781884d03a6757a803996e38de2a42967fb37eeaca72729271025a9e2;

// namehash('addr.reverse')

contract ReverseRegistrar is Ownable, Controllable {
    ENS public ens;
    NameResolver public defaultResolver;

    event ReverseClaimed(address indexed _addr, bytes32 indexed _node);

    /**
     * @dev Constructor
     * @param _ensAddr The address of the ENS registry.
     * @param _resolverAddr The address of the default reverse resolver.
     */
    constructor(ENS _ensAddr, NameResolver _resolverAddr) {
        ens = _ensAddr;
        defaultResolver = _resolverAddr;

        // Assign ownership of the reverse record to our deployer
        ReverseRegistrar oldRegistrar = ReverseRegistrar(
            ens.owner(ADDR_REVERSE_NODE)
        );
        if (address(oldRegistrar) != address(0x0)) {
            oldRegistrar.claim(msg.sender);
        }
    }

    modifier authorised(address _addr) {
        require(
            _addr == msg.sender ||
                controllers[msg.sender] ||
                ens.isApprovedForAll(_addr, msg.sender) ||
                ownsContract(_addr),
            "Caller is not a controller or authorised by address or the address itself"
        );
        _;
    }

    /**
     * @dev Transfers ownership of the reverse ENS record associated with the
     *      calling account.
     * @param _owner The address to set as the owner of the reverse record in ENS.
     * @return The ENS node hash of the reverse record.
     */
    function claim(address _owner) public returns (bytes32) {
        return _claimWithResolver(msg.sender, _owner, address(0x0));
    }

    /**
     * @dev Transfers ownership of the reverse ENS record associated with the
     *      calling account.
     * @param _addr The reverse record to set
     * @param _owner The address to set as the owner of the reverse record in ENS.
     * @return The ENS node hash of the reverse record.
     */
    function claimForAddr(address _addr, address _owner)
        public
        authorised(_addr)
        returns (bytes32)
    {
        return _claimWithResolver(_addr, _owner, address(0x0));
    }

    /**
     * @dev Transfers ownership of the reverse ENS record associated with the
     *      calling account.
     * @param _owner The address to set as the owner of the reverse record in ENS.
     * @param _resolver The address of the resolver to set; 0 to leave unchanged.
     * @return The ENS node hash of the reverse record.
     */
    function claimWithResolver(address _owner, address _resolver)
        public
        returns (bytes32)
    {
        return _claimWithResolver(msg.sender, _owner, _resolver);
    }

    /**
     * @dev Transfers ownership of the reverse ENS record specified with the
     *      address provided
     * @param _addr The reverse record to set
     * @param _owner The address to set as the owner of the reverse record in ENS.
     * @param _resolver The address of the resolver to set; 0 to leave unchanged.
     * @return The ENS node hash of the reverse record.
     */
    function claimWithResolverForAddr(
        address _addr,
        address _owner,
        address _resolver
    ) public authorised(_addr) returns (bytes32) {
        return _claimWithResolver(_addr, _owner, _resolver);
    }

    /**
     * @dev Sets the `name()` record for the reverse ENS record associated with
     * the calling account. First updates the resolver to the default reverse
     * resolver if necessary.
     * @param _name The name to set for this address.
     * @return The ENS node hash of the reverse record.
     */
    function setName(string memory _name) public returns (bytes32) {
        bytes32 _node = _claimWithResolver(
            msg.sender,
            address(this),
            address(defaultResolver)
        );
        defaultResolver.setName(_node, _name);
        return _node;
    }

    /**
     * @dev Sets the `name()` record for the reverse ENS record associated with
     * the account provided. First updates the resolver to the default reverse
     * resolver if necessary.
     * Only callable by controllers and authorised users
     * @param _addr The reverse record to set
     * @param _owner The owner of the reverse node
     * @param _name The name to set for this address.
     * @return The ENS node hash of the reverse record.
     */
    function setNameForAddr(
        address _addr,
        address _owner,
        string memory _name
    ) public authorised(_addr) returns (bytes32) {
        bytes32 _node = _claimWithResolver(
            _addr,
            address(this),
            address(defaultResolver)
        );
        defaultResolver.setName(_node, _name);
        ens.setSubnodeOwner(ADDR_REVERSE_NODE, sha3HexAddress(_addr), _owner);
        return _node;
    }

    /**
     * @dev Returns the node hash for a given account's reverse records.
     * @param _addr The address to hash
     * @return The ENS node hash.
     */
    function node(address _addr) public pure returns (bytes32) {
        return
            keccak256(
                abi.encodePacked(ADDR_REVERSE_NODE, sha3HexAddress(_addr))
            );
    }

    /**
     * @dev An optimised function to compute the sha3 of the lower-case
     *      hexadecimal representation of an Ethereum address.
     * @param _addr The address to hash
     * @return ret The SHA3 hash of the lower-case hexadecimal encoding of the
     *         input address.
     */
    function sha3HexAddress(address _addr) private pure returns (bytes32 ret) {
        assembly {
            for {
                let i := 40
            } gt(i, 0) {

            } {
                i := sub(i, 1)
                mstore8(i, byte(and(_addr, 0xf), lookup))
                _addr := div(_addr, 0x10)
                i := sub(i, 1)
                mstore8(i, byte(and(_addr, 0xf), lookup))
                _addr := div(_addr, 0x10)
            }

            ret := keccak256(0, 40)
        }
    }

    /* Internal functions */

    function _claimWithResolver(
        address _addr,
        address _owner,
        address _resolver
    ) internal returns (bytes32) {
        bytes32 label = sha3HexAddress(_addr);
        bytes32 _node = keccak256(abi.encodePacked(ADDR_REVERSE_NODE, label));
        address currentResolver = ens.resolver(_node);
        bool shouldUpdateResolver = (_resolver != address(0x0) &&
            _resolver != currentResolver);
        address newResolver = shouldUpdateResolver ? _resolver : currentResolver;

        ens.setSubnodeRecord(ADDR_REVERSE_NODE, label, _owner, newResolver, 0);

        emit ReverseClaimed(_addr, _node);

        return _node;
    }

    function ownsContract(address _addr) internal view returns (bool) {
        try Ownable(_addr).owner() returns (address owner) {
            return owner == msg.sender;
        } catch {
            return false;
        }
    }
}
