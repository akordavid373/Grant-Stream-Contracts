// SPDX-License-Identifier: MIT
pragma solidity ^0.8.24;

import "@openzeppelin/contracts/access/AccessControl.sol";
import "@openzeppelin/contracts/security/ReentrancyGuard.sol";

contract GrantStream is AccessControl, ReentrancyGuard {
    bytes32 public constant ADMIN_ROLE = keccak256("ADMIN_ROLE");
    bytes32 public constant REVIEWER_ROLE = keccak256("REVIEWER_ROLE");

    struct Milestone {
        uint256 amount;
        string description;
        bytes32 deliverableHash;
        uint256 approvalsReceived;
        bool isApproved;
        bool isReleased;
        uint256 deadline;
        mapping(address => bool) hasApproved;
    }

    struct Grant {
        address recipient;
        address sponsor;
        uint256 totalAmount;
        uint256 releasedAmount;
        uint256 streamCapPerLedger;     // ← NEW: Stream Cap
        Milestone[] milestones;
        address[] reviewers;
        uint256 requiredApprovals;
        bool isCancelled;
    }

    mapping(uint256 => Grant) public grants;
    uint256 public grantCount;

    // Events
    event GrantCreated(uint256 indexed grantId, address recipient, uint256 streamCapPerLedger);
    event MilestoneReleased(uint256 indexed grantId, uint256 milestoneIndex, uint256 amount);
    event StreamCapUpdated(uint256 indexed grantId, uint256 newCap);

    constructor() {
        _grantRole(DEFAULT_ADMIN_ROLE, msg.sender);
        _grantRole(ADMIN_ROLE, msg.sender);
    }

    /* ==================== GRANT CREATION WITH STREAM CAP ==================== */

    function createGrant(
        address _recipient,
        uint256 _totalAmount,
        uint256 _streamCapPerLedger,      // New parameter
        address[] calldata _reviewers,
        uint256 _requiredApprovals
    ) external onlyRole(ADMIN_ROLE) {
        require(_recipient != address(0), "Invalid recipient");
        require(_streamCapPerLedger > 0 && _streamCapPerLedger <= _totalAmount, "Invalid stream cap");
        require(_reviewers.length >= _requiredApprovals && _requiredApprovals > 0, "Invalid N-of-M");

        Grant storage grant = grants[grantCount];
        grant.recipient = _recipient;
        grant.sponsor = msg.sender;
        grant.totalAmount = _totalAmount;
        grant.streamCapPerLedger = _streamCapPerLedger;
        grant.reviewers = _reviewers;
        grant.requiredApprovals = _requiredApprovals;
        grant.isCancelled = false;

        emit GrantCreated(grantCount, _recipient, _streamCapPerLedger);
        grantCount++;
    }

    /* ==================== MILESTONE RELEASE WITH STREAM CAP ENFORCEMENT ==================== */

    function releaseMilestone(uint256 grantId, uint256 milestoneIndex) 
        external 
        onlyRole(ADMIN_ROLE) 
        nonReentrant 
    {
        Grant storage grant = grants[grantId];
        require(!grant.isCancelled, "Grant is cancelled");
        require(milestoneIndex < grant.milestones.length, "Invalid milestone");

        Milestone storage milestone = grant.milestones[milestoneIndex];
        require(milestone.isApproved, "Milestone not approved by N-of-M reviewers");
        require(!milestone.isReleased, "Already released");

        // ====================== STREAM CAP CHECK ======================
        require(milestone.amount <= grant.streamCapPerLedger, 
            "Milestone amount exceeds stream cap per ledger");

        // Optional: Track total released in current ledger (more advanced protection)
        // For now, we enforce per-milestone cap as "per-ledger" equivalent

        milestone.isReleased = true;
        grant.releasedAmount += milestone.amount;

        // Transfer funds
        payable(grant.recipient).transfer(milestone.amount);

        emit MilestoneReleased(grantId, milestoneIndex, milestone.amount);
    }

    /* ==================== ADMIN FUNCTIONS ==================== */

    function updateStreamCap(uint256 grantId, uint256 newCap) 
        external 
        onlyRole(ADMIN_ROLE) 
    {
        Grant storage grant = grants[grantId];
        require(grant.recipient != address(0), "Grant does not exist");
        require(newCap > 0, "Cap must be > 0");

        grant.streamCapPerLedger = newCap;
        emit StreamCapUpdated(grantId, newCap);
    }

    function cancelStream(uint256 grantId) external onlyRole(ADMIN_ROLE) {
        Grant storage grant = grants[grantId];
        grant.isCancelled = true;
        // Remaining logic for refund/arbitration can be added here
    }

    // View function
    function getStreamCap(uint256 grantId) external view returns (uint256) {
        return grants[grantId].streamCapPerLedger;
    }
}