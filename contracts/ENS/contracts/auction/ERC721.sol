// SPDX-License-Identifier: MIT
pragma solidity ^0.8.0;

 abstract contract ERC721 {
    // Function
    function totalSupply() public view virtual returns (uint256 _totalSupply);
    function balanceOf(address _owner) public view virtual returns (uint256 _balance);
    function ownerOf(uint _tokenId) public view virtual returns (address _owner);
    function approve(address _to, uint _tokenId) public virtual;
    function transferFrom(address _from, address _to, uint _tokenId) public virtual;
    function transfer(address _to, uint _tokenId) public virtual;
    function implementsERC721() public view virtual returns (bool _implementsERC721);

    // Events
    event Transfer(address indexed _from, address indexed _to, uint256 _tokenId);
    event Approval(address indexed _owner, address indexed _approved, uint256 _tokenId);
}