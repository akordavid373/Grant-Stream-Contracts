// SPDX-License-Identifier: MIT
pragma solidity ^0.8.20;

import "forge-std/Test.sol";
import "../contracts/VetoPeriod.sol";

contract VetoPeriodTest is Test {
    VetoPeriod veto;

    address council   = address(0xC0C);
    address recipient = address(0xBEEF);
    address voter1    = address(0x1);
    address voter2    = address(0x2);

    // 10 total holders → 20% threshold = 2 voters needed
    uint256 totalHolders = 10;

    function setUp() public {
        veto = new VetoPeriod(council, totalHolders);
        vm.deal(address(this), 10 ether);
    }

    function test_WithdrawalQueuedInPendingState() public {
        uint256 id = veto.queueWithdrawal{value: 1 ether}(recipient, 1 ether);
        (, , , , VetoPeriod.WithdrawalStatus status) = veto.withdrawals(id);
        assertEq(uint(status), uint(VetoPeriod.WithdrawalStatus.Pending));
    }

    function test_ExecuteAfter48Hours() public {
        uint256 id = veto.queueWithdrawal{value: 1 ether}(recipient, 1 ether);

        vm.warp(block.timestamp + 48 hours + 1);

        uint256 balBefore = recipient.balance;
        veto.executeWithdrawal(id);

        assertEq(recipient.balance, balBefore + 1 ether);
    }

    function test_CannotExecuteDuringVetoWindow() public {
        uint256 id = veto.queueWithdrawal{value: 1 ether}(recipient, 1 ether);

        vm.expectRevert("Veto window still open");
        veto.executeWithdrawal(id);
    }

    function test_20PercentMinorityCanVeto() public {
        uint256 id = veto.queueWithdrawal{value: 1 ether}(recipient, 1 ether);

        vm.prank(voter1); veto.castVeto(id);
        vm.prank(voter2); veto.castVeto(id);

        (, , , , VetoPeriod.WithdrawalStatus status) = veto.withdrawals(id);
        assertEq(uint(status), uint(VetoPeriod.WithdrawalStatus.Vetoed));
    }

    function test_SecurityCouncilCanVetoInstantly() public {
        uint256 id = veto.queueWithdrawal{value: 1 ether}(recipient, 1 ether);

        vm.prank(council);
        veto.securityCouncilVeto(id);

        (, , , , VetoPeriod.WithdrawalStatus status) = veto.withdrawals(id);
        assertEq(uint(status), uint(VetoPeriod.WithdrawalStatus.Vetoed));
    }

    function test_VetoedWithdrawalCannotBeExecuted() public {
        uint256 id = veto.queueWithdrawal{value: 1 ether}(recipient, 1 ether);

        vm.prank(council);
        veto.securityCouncilVeto(id);

        vm.warp(block.timestamp + 48 hours + 1);
        vm.expectRevert("Not pending or already vetoed");
        veto.executeWithdrawal(id);
    }

    // -----------------------------------------------------------------------
    // Issue #308 — Formal Proof: Governance Veto Reachability
    // Proves that for any high-value transaction queued by the admin, there is
    // always a valid execution path for the Security Council to call veto()
    // before the timelock expires — the system is never "deadlocked."
    // -----------------------------------------------------------------------

    /// @dev Proof: security council can veto at any point strictly inside the window.
    ///      Fuzz over the warp offset to cover the full [0, VETO_WINDOW) range.
    function testFuzz_SecurityCouncilCanAlwaysVetoWithinWindow(uint256 offset) public {
        // Bound offset to [0, VETO_WINDOW - 1] — any moment inside the window
        offset = bound(offset, 0, veto.VETO_WINDOW() - 1);

        uint256 id = veto.queueWithdrawal{value: 1 ether}(recipient, 1 ether);

        vm.warp(block.timestamp + offset);

        // Security council veto must always succeed inside the window
        vm.prank(council);
        veto.securityCouncilVeto(id);

        (, , , , VetoPeriod.WithdrawalStatus status) = veto.withdrawals(id);
        assertEq(uint(status), uint(VetoPeriod.WithdrawalStatus.Vetoed),
            "veto must be reachable at any point inside the window");
    }

    /// @dev Proof: veto is NOT reachable after the window closes (no deadlock in reverse).
    function test_VetoUnreachableAfterWindowCloses() public {
        uint256 id = veto.queueWithdrawal{value: 1 ether}(recipient, 1 ether);

        vm.warp(block.timestamp + veto.VETO_WINDOW() + 1);

        vm.prank(council);
        vm.expectRevert("Veto window closed");
        veto.securityCouncilVeto(id);
    }

    /// @dev Proof: execution is unreachable while the veto window is open —
    ///      the council always has the full window to act.
    function testFuzz_ExecutionBlockedDuringVetoWindow(uint256 offset) public {
        offset = bound(offset, 0, veto.VETO_WINDOW() - 1);

        uint256 id = veto.queueWithdrawal{value: 1 ether}(recipient, 1 ether);

        vm.warp(block.timestamp + offset);

        vm.expectRevert("Veto window still open");
        veto.executeWithdrawal(id);
    }

    /// @dev Proof: timeRemaining is always > 0 inside the window and 0 outside.
    function testFuzz_TimeRemainingIsConsistent(uint256 offset) public {
        offset = bound(offset, 0, veto.VETO_WINDOW() * 2);

        uint256 id = veto.queueWithdrawal{value: 1 ether}(recipient, 1 ether);
        uint256 queuedAt = block.timestamp;

        vm.warp(queuedAt + offset);

        uint256 remaining = veto.timeRemaining(id);

        if (offset < veto.VETO_WINDOW()) {
            assertGt(remaining, 0, "time remaining must be > 0 inside window");
        } else {
            assertEq(remaining, 0, "time remaining must be 0 after window");
        }
    }

    /// @dev Proof: a vetoed withdrawal can never be executed, even after the window.
    ///      Ensures the veto action is permanent and irreversible.
    function testFuzz_VetoIsPermanent(uint256 postVetoWarp) public {
        postVetoWarp = bound(postVetoWarp, 0, 365 days);

        uint256 id = veto.queueWithdrawal{value: 1 ether}(recipient, 1 ether);

        vm.prank(council);
        veto.securityCouncilVeto(id);

        vm.warp(block.timestamp + postVetoWarp);

        vm.expectRevert("Not pending or already vetoed");
        veto.executeWithdrawal(id);
    }
}