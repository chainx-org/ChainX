// SPDX-License-Identifier: MIT
pragma solidity >=0.8.4;

import "./SafeMath.sol";
import "./StringUtils.sol";
import "@openzeppelin/contracts/access/Ownable.sol";

// StablePriceOracle sets a price in USD, based on an oracle.
contract StablePriceOracle is Ownable {
    using SafeMath for *;
    using StringUtils for *;

    // Rent in base price units by length. Element 0 is for 1-length names, and so on.
    uint256 public rentPrices;
    uint256 public premiumPrices;
    uint256 public renewPrice;

    event RentPriceChanged(uint rentPrices,uint premiumPrices,uint renewPrices);

    bytes4 constant private INTERFACE_META_ID = bytes4(keccak256("supportsInterface(bytes4)"));
    bytes4 constant private ORACLE_ID = bytes4(keccak256("registerPrice(uint256)"));

    constructor(uint256  _rentPrices,uint256 _premiumPrices,uint256 _renewPrices) {
        setPrices(_rentPrices,_premiumPrices,_renewPrices);
    }

    function registerPrice(uint256 duration) external view returns(uint) {
        uint basePrice = rentPrices.mul(duration);
        basePrice = basePrice.add(premiumPrices);
        return basePrice;
    }

    function setPrices(uint256 _rentPrices,uint256 _premiumPrices,uint256 _renewPrices) public onlyOwner {
        rentPrices = _rentPrices;
        premiumPrices = _premiumPrices;
        renewPrice = _renewPrices;
        emit RentPriceChanged(_rentPrices,_premiumPrices,_renewPrices);
    }

    function supportsInterface(bytes4 interfaceID) public view virtual returns (bool) {
        return interfaceID == INTERFACE_META_ID || interfaceID == ORACLE_ID;
    }
}
