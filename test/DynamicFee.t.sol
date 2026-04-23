// SPDX-License-Identifier: MIT
pragma solidity ^0.8.20;

import "forge-std/Test.sol";
import "../contracts/DynamicFee.sol";

contract DynamicFeeTest is Test {
    DynamicFee fee;

    function setUp() public {
        fee = new DynamicFee();
    }

    function test_BaseFeeLowTVL() public {
        // No TVL set → should return BASE_FEE (100 BPS)
        uint256 f = fee.getCurrentFee();
        assertEq(f, 100);
    }

    function test_FeeDecreasesAsHighTVL() public {
        // Low TVL fee
        fee.setTVL(1_000 ether);
        uint256 lowTVLFee = fee.getCurrentFee();

        // High TVL fee
        fee.setTVL(100_000 ether);
        uint256 highTVLFee = fee.getCurrentFee();

        // Higher TVL should produce a lower fee
        assertLt(highTVLFee, lowTVLFee);
    }

    function test_VolatilityIncreaseFee() public {
        fee.setTVL(1_000 ether);
        uint256 normalFee = fee.getCurrentFee();

        fee.setVolatilityIndex(100); // max volatility
        uint256 volatileFee = fee.getCurrentFee();

        assertGt(volatileFee, normalFee);
    }

    function test_HighLoadIncreasesFee() public {
        fee.setTVL(1_000 ether);
        uint256 normalFee = fee.getCurrentFee();

        fee.setTransactionLoad(5000); // 5000 txns → +50 BPS surcharge
        uint256 highLoadFee = fee.getCurrentFee();

        assertGt(highLoadFee, normalFee);
    }

    function test_FeeAlwaysWithinBounds() public {
        // Extreme low TVL + max volatility + max load
        fee.setTVL(0);
        fee.setVolatilityIndex(100);
        fee.setTransactionLoad(100_000);
        uint256 maxFee = fee.getCurrentFee();
        assertLe(maxFee, 500); // never exceeds MAX (500 BPS)

        // Extreme high TVL + no volatility + no load
        fee.setTVL(10_000_000 ether);
        fee.setVolatilityIndex(0);
        fee.setTransactionLoad(0);
        uint256 minFee = fee.getCurrentFee();
        assertGe(minFee, 10); // never below MIN (10 BPS)
    }

    function test_ApplyFeeDeductsCorrectly() public {
        // Base fee = 100 BPS = 1%
        // 1 ETH → feeCharged = 0.01 ETH, afterFee = 0.99 ETH
        (uint256 afterFee, uint256 feeCharged) = fee.applyFee(1 ether);
        assertEq(feeCharged, 0.01 ether);
        assertEq(afterFee,   0.99 ether);
    }
}