// SPDX-License-Identifier: MIT
pragma solidity ^0.8.0;

import "./ClockAuctionBase.sol";
import "./Pausable.sol";

contract ClockAuction is Pausable, ClockAuctionBase {

    /// @dev Constructor creates a reference to the NFT ownership contract
    ///  and verifies the owner cut is in the valid range.
    /// @param _cut - percent cut the owner takes on each auction, must be
    ///  between 0-10,000.
    constructor(uint256 _cut) {
        require(_cut <= 10000);
        ownerCut = _cut;
    }

    /// @dev Remove all Ether from the contract, which is the owner's cuts
    ///  as well as any Ether sent directly to the contract address.
    ///  Always transfers to the NFT contract, but can be called either by
    ///  the owner or the NFT contract.
    function withdrawBalance() external {
        require(
            msg.sender == owner()
        );
        payable(msg.sender).transfer(address(this).balance);
    }

    /// @dev Creates and begins a new auction.
    /// @param _nftAddress - address of a deployed contract implementing
    ///  the Nonfungible Interface.
    /// @param _tokenId - ID of token to auction, sender must be owner.
    /// @param _startingPrice - Price of item (in wei) at beginning of auction.
    /// @param _endingPrice - Price of item (in wei) at end of auction.
    /// @param _duration - Length of time to move between starting
    ///  price and ending price (in seconds).
    /// @param _seller - Seller, if not the message sender
    function createAuction(
        address _nftAddress,
        uint256 _tokenId,
        uint256 _startingPrice,
        uint256 _endingPrice,
        uint256 _duration,
        address _seller
    )
    public
    whenNotPaused
    canBeStoredWith128Bits(_startingPrice)
    canBeStoredWith128Bits(_endingPrice)
    canBeStoredWith64Bits(_duration)
    {
        require(_owns(_nftAddress, msg.sender, _tokenId));
        _escrow(_nftAddress, msg.sender, _tokenId);
        Auction memory auction = Auction(
            _nftAddress,
            _seller,
            uint128(_startingPrice),
            uint128(_endingPrice),
            uint64(_duration),
            uint64(block.timestamp)
        );
        _addAuction(_nftAddress, _tokenId, auction);
    }

    /// @dev Bids on an open auction, completing the auction and transferring
    ///  ownership of the NFT if enough Ether is supplied.
    /// @param _nftAddress - address of a deployed contract implementing
    ///  the Nonfungible Interface.
    /// @param _tokenId - ID of token to bid on.
    function bid(address _nftAddress, uint256 _tokenId)
    public
    virtual
    payable
    whenNotPaused
    {
        // _bid will throw if the bid or funds transfer fails
        _bid(_nftAddress, _tokenId, msg.value);
        _transfer(_nftAddress, msg.sender, _tokenId);
    }

    /// @dev Cancels an auction that hasn't been won yet.
    ///  Returns the NFT to original owner.
    /// @notice This is a state-modifying function that can
    ///  be called while the contract is paused.
    /// @param _nftAddress - Address of the NFT.
    /// @param _tokenId - ID of token on auction
    function cancelAuction(address _nftAddress, uint256 _tokenId)
    public
    {
        Auction storage auction = nftToTokenIdToAuction[_nftAddress][_tokenId];
        require(_isOnAuction(auction));
        address seller = auction.seller;
        require(msg.sender == seller);
        _cancelAuction(_nftAddress, _tokenId, seller);
    }

    /// @dev Cancels an auction when the contract is paused.
    ///  Only the owner may do this, and NFTs are returned to
    ///  the seller. This should only be used in emergencies.
    /// @param _nftAddress - Address of the NFT.
    /// @param _tokenId - ID of the NFT on auction to cancel.
    function cancelAuctionWhenPaused(address _nftAddress, uint256 _tokenId)
    whenPaused
    onlyOwner
    public
    {
        Auction storage auction = nftToTokenIdToAuction[_nftAddress][_tokenId];
        require(_isOnAuction(auction));
        _cancelAuction(_nftAddress, _tokenId, auction.seller);
    }

    /// @dev Returns auction info for an NFT on auction.
    /// @param _nftAddress - Address of the NFT.
    /// @param _tokenId - ID of NFT on auction.
    function getAuction(address _nftAddress, uint256 _tokenId)
    public
    view
    returns
    (
        address seller,
        uint256 startingPrice,
        uint256 endingPrice,
        uint256 duration,
        uint256 startedAt
    ) {
        Auction storage auction = nftToTokenIdToAuction[_nftAddress][_tokenId];
        require(_isOnAuction(auction));
        return (
        auction.seller,
        auction.startingPrice,
        auction.endingPrice,
        auction.duration,
        auction.startedAt
        );
    }

    /// @dev Returns the current price of an auction.
    /// @param _nftAddress - Address of the NFT.
    /// @param _tokenId - ID of the token price we are checking.
    function getCurrentPrice(address _nftAddress, uint256 _tokenId)
    public
    view
    returns (uint256)
    {
        Auction storage auction = nftToTokenIdToAuction[_nftAddress][_tokenId];
        require(_isOnAuction(auction));
        return _currentPrice(auction);
    }

}

/// @title Clock auction modified for sale of kitties
contract SaleClockAuction is ClockAuction {

    // Delegate constructor
    constructor(uint256 _cut) ClockAuction(_cut) {}

    /// @dev Creates and begins a new auction.
    /// @param _nftAddress - The address of the NFT.
    /// @param _tokenId - ID of token to auction, sender must be owner.
    /// @param _startingPrice - Price of item (in wei) at beginning of auction.
    /// @param _endingPrice - Price of item (in wei) at end of auction.
    /// @param _duration - Length of auction (in seconds).
    function createAuction(
        address _nftAddress,
        uint256 _tokenId,
        uint256 _startingPrice,
        uint256 _endingPrice,
        uint256 _duration
    )
    public
    canBeStoredWith128Bits(_startingPrice)
    canBeStoredWith128Bits(_endingPrice)
    canBeStoredWith64Bits(_duration)
    {
        address seller = msg.sender;
        _escrow(_nftAddress, seller, _tokenId);
        Auction memory auction = Auction(
            _nftAddress,
            seller,
            uint128(_startingPrice),
            uint128(_endingPrice),
            uint64(_duration),
            uint64(block.timestamp)
        );
        _addAuction(_nftAddress, _tokenId, auction);
    }

    /// @dev Updates lastSalePrice if seller is the nft contract
    /// Otherwise, works the same as default bid method.
    function bid(address _nftAddress, uint256 _tokenId)
    public
    override
    payable
    {
        // _bid verifies token ID size
        _bid(_nftAddress, _tokenId, msg.value);
        _transfer(_nftAddress, msg.sender, _tokenId);
    }
}
