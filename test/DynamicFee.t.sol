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

    // -----------------------------------------------------------------------
    // Issue #302 — Fuzz-Test: Dynamic Fee Calculation Stability
    // Simulates various protocol TVL levels and verifies dynamic_fee stays
    // within allowed bounds [MIN_FEE_BPS, MAX_FEE_BPS] under all inputs.
    // -----------------------------------------------------------------------

    /// @dev Fuzz: fee is always within [MIN_FEE_BPS, MAX_FEE_BPS] for any TVL,
    ///      volatility index (0–100), and transaction load.
    function testFuzz_FeeAlwaysWithinBounds(
        uint256 tvl,
        uint8   volatilityIndex,
        uint256 txLoad
    ) public {
        // Bound volatility to valid range [0, 100]
        uint256 vi = bound(volatilityIndex, 0, 100);

        fee.setTVL(tvl);
        fee.setVolatilityIndex(vi);
        fee.setTransactionLoad(txLoad);

        uint256 f = fee.getCurrentFee();

        assertGe(f, fee.MIN_FEE_BPS(), "fee below MIN");
        assertLe(f, fee.MAX_FEE_BPS(), "fee above MAX");
    }

    /// @dev Fuzz: applyFee never overflows and afterFee + feeCharged == amount.
    function testFuzz_ApplyFeeConservation(uint128 amount) public {
        // Use uint128 to avoid overflow in fee arithmetic (amount * 500 / 10000)
        (uint256 afterFee, uint256 feeCharged) = fee.applyFee(uint256(amount));
        assertEq(afterFee + feeCharged, uint256(amount), "fee conservation violated");
    }

    /// @dev Fuzz: fee is monotonically non-increasing as TVL grows
    ///      (holding volatility and load constant at zero).
    function testFuzz_FeeDecreasesWithTVL(uint256 tvlLow, uint256 tvlHigh) public {
        // Ensure tvlHigh > tvlLow and both are multiples of 1e18 for meaningful log steps
        tvlLow  = bound(tvlLow,  0,          1_000_000 ether);
        tvlHigh = bound(tvlHigh, tvlLow + 1 ether, 10_000_000 ether);

        fee.setVolatilityIndex(0);
        fee.setTransactionLoad(0);

        fee.setTVL(tvlLow);
        uint256 feeLow = fee.getCurrentFee();

        fee.setTVL(tvlHigh);
        uint256 feeHigh = fee.getCurrentFee();

        assertLe(feeHigh, feeLow, "fee should not increase with higher TVL");
    }

    /// @dev Fuzz: volatility surcharge is always in [0, VOLATILITY_SCALE_MAX].
    function testFuzz_VolatilitySurchargeInBounds(uint8 vi) public {
        uint256 bounded = bound(vi, 0, 100);
        fee.setVolatilityIndex(bounded);
        uint256 surcharge = fee.computeVolatilitySurcharge();
        assertLe(surcharge, fee.VOLATILITY_SCALE_MAX(), "volatility surcharge exceeds max");
    }

    /// @dev Fuzz: load surcharge is always capped at 100 BPS.
    function testFuzz_LoadSurchargeCapped(uint256 load) public {
        fee.setTransactionLoad(load);
        uint256 surcharge = fee.computeLoadSurcharge();
        assertLe(surcharge, 100, "load surcharge exceeds 100 BPS cap");
    }
}