// SPDX-License-Identifier: MIT
pragma solidity ^0.8.20;

contract VetoPeriod {
    uint256 public constant VETO_WINDOW = 48 hours;
    uint256 public constant VETO_THRESHOLD_BPS = 2000; // 20% of token holders

    address public securityCouncil;
    uint256 public totalTokenHolders;

    enum WithdrawalStatus { Pending, Approved, Vetoed, Executed }

    struct Withdrawal {
        address recipient;
        uint256 amount;
        uint256 createdAt;
        uint256 vetoVotes;
        WithdrawalStatus status;
    }

    mapping(uint256 => Withdrawal) public withdrawals;
    mapping(uint256 => mapping(address => bool)) public hasVetoed;

    uint256 public withdrawalCount;

    event WithdrawalQueued(uint256 indexed id, address recipient, uint256 amount);
    event VetoCast(uint256 indexed id, address voter, uint256 totalVetoVotes);
    event WithdrawalVetoed(uint256 indexed id);
    event WithdrawalExecuted(uint256 indexed id, address recipient, uint256 amount);

    constructor(address _securityCouncil, uint256 _totalTokenHolders) {
        securityCouncil = _securityCouncil;
        totalTokenHolders = _totalTokenHolders;
    }

    // DAO queues a withdrawal — funds enter pending_exit state
    function queueWithdrawal(address recipient, uint256 amount) external payable returns (uint256) {
        require(msg.value == amount, "Must send exact amount");
        require(recipient != address(0), "Invalid recipient");

        uint256 id = withdrawalCount++;

        withdrawals[id] = Withdrawal({
            recipient: recipient,
            amount: amount,
            createdAt: block.timestamp,
            vetoVotes: 0,
            status: WithdrawalStatus.Pending
        });

        emit WithdrawalQueued(id, recipient, amount);
        return id;
    }

    // Token holders cast a veto vote during the 48-hour window
    function castVeto(uint256 id) external {
        Withdrawal storage w = withdrawals[id];

        require(w.status == WithdrawalStatus.Pending, "Not pending");
        require(block.timestamp <= w.createdAt + VETO_WINDOW, "Veto window closed");
        require(!hasVetoed[id][msg.sender], "Already vetoed");

        hasVetoed[id][msg.sender] = true;
        w.vetoVotes += 1;

        emit VetoCast(id, msg.sender, w.vetoVotes);

        // Check if 20% threshold is reached
        uint256 threshold = (totalTokenHolders * VETO_THRESHOLD_BPS) / 10000;
        if (w.vetoVotes >= threshold) {
            w.status = WithdrawalStatus.Vetoed;
            emit WithdrawalVetoed(id);
        }
    }

    // Security council can veto instantly
    function securityCouncilVeto(uint256 id) external {
        require(msg.sender == securityCouncil, "Not security council");

        Withdrawal storage w = withdrawals[id];
        require(w.status == WithdrawalStatus.Pending, "Not pending");
        require(block.timestamp <= w.createdAt + VETO_WINDOW, "Veto window closed");

        w.status = WithdrawalStatus.Vetoed;
        emit WithdrawalVetoed(id);
    }

    // After 48 hours with no veto — anyone can execute
    function executeWithdrawal(uint256 id) external {
        Withdrawal storage w = withdrawals[id];

        require(w.status == WithdrawalStatus.Pending, "Not pending or already vetoed");
        require(block.timestamp > w.createdAt + VETO_WINDOW, "Veto window still open");

        w.status = WithdrawalStatus.Executed;

        payable(w.recipient).transfer(w.amount);

        emit WithdrawalExecuted(id, w.recipient, w.amount);
    }

    // View: time remaining in veto window
    function timeRemaining(uint256 id) external view returns (uint256) {
        Withdrawal storage w = withdrawals[id];
        uint256 deadline = w.createdAt + VETO_WINDOW;
        if (block.timestamp >= deadline) return 0;
        return deadline - block.timestamp;
    }
}