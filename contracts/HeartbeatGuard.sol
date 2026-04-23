// SPDX-License-Identifier: MIT
pragma solidity ^0.8.20;

contract HeartbeatGuard {
    uint256 public constant HEARTBEAT_INTERVAL = 365 days;

    address public admin;
    uint256 public lastPing;

    // donor address => amount they deposited
    mapping(address => uint256) public donorDeposits;

    bool public selfHealingActive;

    event Pinged(address indexed admin, uint256 timestamp);
    event SelfHealingTriggered(uint256 timestamp);
    event FundsReclaimed(address indexed donor, uint256 amount);

    constructor() {
        admin = msg.sender;
        lastPing = block.timestamp;
    }

    // Admin calls this to prove they're alive (at least once every 12 months)
    function ping() external {
        require(msg.sender == admin, "Not admin");
        lastPing = block.timestamp;
        selfHealingActive = false;
        emit Pinged(msg.sender, block.timestamp);
    }

    // Anyone can call this — triggers self-healing if admin has been silent 12 months
    function heartbeat() external {
        require(!selfHealingActive, "Already in self-healing mode");
        require(
            block.timestamp >= lastPing + HEARTBEAT_INTERVAL,
            "Admin is still active"
        );
        selfHealingActive = true;
        emit SelfHealingTriggered(block.timestamp);
    }

    // Donors deposit funds
    function deposit() external payable {
        require(msg.value > 0, "No funds sent");
        donorDeposits[msg.sender] += msg.value;
    }

    // Once self-healing is active, any donor can reclaim their funds permissionlessly
    function reclaim() external {
        require(selfHealingActive, "Not in self-healing mode");
        uint256 amount = donorDeposits[msg.sender];
        require(amount > 0, "Nothing to reclaim");

        donorDeposits[msg.sender] = 0; // zero before transfer (safety)
        payable(msg.sender).transfer(amount);

        emit FundsReclaimed(msg.sender, amount);
    }
}