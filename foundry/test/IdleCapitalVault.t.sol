// SPDX-License-Identifier: MIT
pragma solidity ^0.8.20;

import "forge-std/Test.sol";
import "../../contracts/IdleCapitalVault.sol";

contract MockYieldRouter is IYieldRouter {
    uint256 private _total;
    mapping(address => uint256) private _bal;

    function deposit(uint256 amount) external payable override returns (uint256) {
        _total += amount;
        _bal[msg.sender] += amount;
        return amount; // 1:1 shares
    }

    function withdraw(uint256 shares) external override returns (uint256) {
        _bal[msg.sender] -= shares;
        _total -= shares;
        payable(msg.sender).transfer(shares);
        return shares;
    }

    function totalAssets() external view override returns (uint256) { return _total; }
    function balanceOf(address a) external view override returns (uint256) { return _bal[a]; }
    receive() external payable {}
}

contract IdleCapitalVaultTest is Test {
    IdleCapitalVault vault;
    MockYieldRouter router;
    address treasury = address(0xBEEF);

    function setUp() public {
        router = new MockYieldRouter();
        vault = new IdleCapitalVault(address(router), treasury);
        vm.deal(address(this), 10 ether);
    }

    function test_RoutesFundsCorrectly() public {
        vault.depositAndRoute{value: 1 ether}();
        // 20% = 0.2 ETH instant, 80% = 0.8 ETH in yield
        assertEq(vault.instantAccessBalance(), 0.2 ether);
    }

    function test_LiquidityThresholdNeverBreached() public {
        vault.depositAndRoute{value: 1 ether}();
        uint256 ratio = vault.liquidityRatio();
        assertGe(ratio, 2000); // always >= 20%
    }

    function test_WithdrawFromInstantAccess() public {
        vault.depositAndRoute{value: 1 ether}();
        vault.withdraw(0.1 ether);
        assertEq(vault.instantAccessBalance(), 0.1 ether);
    }
}