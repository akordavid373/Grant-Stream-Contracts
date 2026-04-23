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
}