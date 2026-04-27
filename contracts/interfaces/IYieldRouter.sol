// SPDX-License-Identifier: MIT
pragma solidity ^0.8.20;

interface IYieldRouter {
    function deposit(uint256 amount) external returns (uint256 shares);
    function withdraw(uint256 shares) external returns (uint256 amount);
    function totalAssets() external view returns (uint256);
    function balanceOf(address account) external view returns (uint256);
}