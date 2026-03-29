// SPDX-License-Identifier: MIT
pragma solidity ^0.8.24;

import "@openzeppelin/contracts/access/Ownable.sol";
import "@openzeppelin/contracts/utils/ReentrancyGuard.sol";
import "@openzeppelin/contracts/security/Pausable.sol";

/**
 * @title ArbitrationEscrow
 * @notice Dispute Resolution Arbitration Escrow system for Grant Stream contracts
 * @dev When a DAO claims a project was never delivered, funds move to a neutral "Jury"
 *      Funds are held in escrow until released by pre-approved Third-Party Arbitrators
 *      Provides a Web3 Courtroom for high-stakes grants with transparent legal process
 */
contract ArbitrationEscrow is Ownable, ReentrancyGuard, Pausable {
    
    // ─── Enums ────────────────────────────────────────────────────────────────
    
    enum DisputeStatus { None, Pending, InArbitration, Resolved, Rejected }
    enum ArbitrationDecision { None, FavorFunder, FavorGrantee, Split }
    
    // ─── Structs ──────────────────────────────────────────────────────────────
    
    struct Dispute {
        uint256 id;
        uint256 grantId;
        address funder;
        address grantee;
        uint256 disputedAmount;
        DisputeStatus status;
        ArbitrationDecision decision;
        address arbitrator;
        uint256 funderAward;      // Amount awarded to funder
        uint256 granteeAward;      // Amount awarded to grantee
        uint256 arbitrationFee;   // Fee paid to arbitrator
        uint256 createdAt;
        uint256 resolvedAt;
        string evidence;          // IPFS hash or URL of evidence
        string reason;            // Reason for dispute
        bool exists;
    }
    
    struct Arbitrator {
        address addr;
        bool approved;
        uint256 reputationScore;  // 0-1000 scale
        uint256 totalCases;
        uint256 activeCases;
        string name;              // Legal firm or decentralized court name
        string jurisdiction;      // Legal jurisdiction or framework
        bool exists;
    }
    
    // ─── State ────────────────────────────────────────────────────────────────
    
    uint256 public nextDisputeId;
    uint256 public nextArbitratorId;
    
    // Configuration
    uint256 public constant ARBITRATION_FEE_PERCENTAGE = 200; // 2% in basis points (10000 = 100%)
    uint256 public constant MINIMUM_DISPUTE_AMOUNT = 0.1 ether;
    uint256 public constant ARBITRATION_TIMEOUT = 30 days;
    uint256 public constant MAX_EVIDENCE_LENGTH = 1000;
    
    // Mappings
    mapping(uint256 => Dispute) public disputes;
    mapping(uint256 => Arbitrator) public arbitrators;
    mapping(address => uint256[]) public funderToDisputes;
    mapping(address => uint256[]) public granteeToDisputes;
    mapping(address => uint256[]) public arbitratorToCases;
    
    // Total escrowed funds
    uint256 public totalEscrowed;
    
    // GrantStream contract reference (set after deployment)
    address public grantStreamContract;
    
    // ─── Events ───────────────────────────────────────────────────────────────
    
    event DisputeRaised(
        uint256 indexed disputeId,
        uint256 indexed grantId,
        address indexed funder,
        address grantee,
        uint256 disputedAmount,
        string evidence,
        string reason
    );
    
    event DisputeAccepted(
        uint256 indexed disputeId,
        address indexed arbitrator,
        uint256 arbitrationFee
    );
    
    event ArbitrationDecision(
        uint256 indexed disputeId,
        ArbitrationDecision decision,
        uint256 funderAward,
        uint256 granteeAward,
        uint256 arbitrationFee,
        string ruling
    );
    
    event FundsReleased(
        uint256 indexed disputeId,
        address indexed recipient,
        uint256 amount
    );
    
    event ArbitratorRegistered(
        uint256 indexed arbitratorId,
        address indexed arbitrator,
        string name,
        string jurisdiction
    );
    
    event ArbitratorStatusChanged(
        uint256 indexed arbitratorId,
        address indexed arbitrator,
        bool approved
    );
    
    // ─── Modifiers ─────────────────────────────────────────────────────────────
    
    modifier onlyGrantStream() {
        require(msg.sender == grantStreamContract, "ArbitrationEscrow: Only GrantStream can call");
        _;
    }
    
    modifier disputeExists(uint256 _disputeId) {
        require(disputes[_disputeId].exists, "ArbitrationEscrow: Dispute does not exist");
        _;
    }
    
    modifier arbitratorExists(uint256 _arbitratorId) {
        require(arbitrators[_arbitratorId].exists, "ArbitrationEscrow: Arbitrator does not exist");
        _;
    }
    
    modifier onlyApprovedArbitrator(uint256 _arbitratorId) {
        require(arbitrators[_arbitratorId].approved, "ArbitrationEscrow: Arbitrator not approved");
        _;
    }
    
    // ─── Constructor ──────────────────────────────────────────────────────────
    
    constructor() Ownable(msg.sender) {
        nextDisputeId = 1;
        nextArbitratorId = 1;
    }
    
    // ─── External Functions ─────────────────────────────────────────────────────
    
    /**
     * @notice Set the GrantStream contract address (called once after deployment)
     * @param _grantStreamContract Address of the GrantStream contract
     */
    function setGrantStreamContract(address _grantStreamContract) external onlyOwner {
        require(_grantStreamContract != address(0), "ArbitrationEscrow: Zero address");
        grantStreamContract = _grantStreamContract;
    }
    
    /**
     * @notice Register a new third-party arbitrator (legal firm or decentralized court)
     * @param _arbitrator Address of the arbitrator
     * @param _name Name of the arbitrator/firm
     * @param _jurisdiction Legal jurisdiction or framework
     */
    function registerArbitrator(
        address _arbitrator,
        string memory _name,
        string memory _jurisdiction
    ) external onlyOwner returns (uint256 arbitratorId) {
        require(_arbitrator != address(0), "ArbitrationEscrow: Zero address");
        require(bytes(_name).length > 0, "ArbitrationEscrow: Empty name");
        require(bytes(_jurisdiction).length > 0, "ArbitrationEscrow: Empty jurisdiction");
        
        arbitratorId = nextArbitratorId++;
        arbitrators[arbitratorId] = Arbitrator({
            addr: _arbitrator,
            approved: false,
            reputationScore: 500, // Start with neutral score
            totalCases: 0,
            activeCases: 0,
            name: _name,
            jurisdiction: _jurisdiction,
            exists: true
        });
        
        emit ArbitratorRegistered(arbitratorId, _arbitrator, _name, _jurisdiction);
    }
    
    /**
     * @notice Approve or revoke arbitrator status
     * @param _arbitratorId ID of the arbitrator
     * @param _approved Whether to approve the arbitrator
     */
    function setArbitratorApproval(uint256 _arbitratorId, bool _approved) 
        external 
        onlyOwner 
        arbitratorExists(_arbitratorId) 
    {
        arbitrators[_arbitratorId].approved = _approved;
        emit ArbitratorStatusChanged(_arbitratorId, arbitrators[_arbitratorId].addr, _approved);
    }
    
    /**
     * @notice Called by GrantStream when a dispute is raised
     * @param _grantId ID of the grant being disputed
     * @param _funder Address of the funder
     * @param _grantee Address of the grantee
     * @param _disputedAmount Amount being disputed
     * @param _evidence IPFS hash or URL of evidence
     * @param _reason Reason for the dispute
     */
    function raiseDispute(
        uint256 _grantId,
        address _funder,
        address _grantee,
        uint256 _disputedAmount,
        string memory _evidence,
        string memory _reason
    ) external onlyGrantStream nonReentrant returns (uint256 disputeId) {
        require(_disputedAmount >= MINIMUM_DISPUTE_AMOUNT, "ArbitrationEscrow: Amount too low");
        require(bytes(_evidence).length <= MAX_EVIDENCE_LENGTH, "ArbitrationEscrow: Evidence too long");
        require(bytes(_reason).length > 0, "ArbitrationEscrow: Empty reason");
        
        disputeId = nextDisputeId++;
        
        disputes[disputeId] = Dispute({
            id: disputeId,
            grantId: _grantId,
            funder: _funder,
            grantee: _grantee,
            disputedAmount: _disputedAmount,
            status: DisputeStatus.Pending,
            decision: ArbitrationDecision.None,
            arbitrator: address(0),
            funderAward: 0,
            granteeAward: 0,
            arbitrationFee: 0,
            createdAt: block.timestamp,
            resolvedAt: 0,
            evidence: _evidence,
            reason: _reason,
            exists: true
        });
        
        funderToDisputes[_funder].push(disputeId);
        granteeToDisputes[_grantee].push(disputeId);
        
        totalEscrowed += _disputedAmount;
        
        emit DisputeRaised(disputeId, _grantId, _funder, _grantee, _disputedAmount, _evidence, _reason);
    }
    
    /**
     * @notice Accept a dispute case (called by approved arbitrator)
     * @param _disputeId ID of the dispute to accept
     * @param _arbitratorId ID of the arbitrator accepting the case
     */
    function acceptDispute(uint256 _disputeId, uint256 _arbitratorId) 
        external 
        disputeExists(_disputeId)
        arbitratorExists(_arbitratorId)
        onlyApprovedArbitrator(_arbitratorId)
        nonReentrant 
    {
        require(arbitrators[_arbitratorId].addr == msg.sender, "ArbitrationEscrow: Not arbitrator");
        require(disputes[_disputeId].status == DisputeStatus.Pending, "ArbitrationEscrow: Not pending");
        
        disputes[_disputeId].status = DisputeStatus.InArbitration;
        disputes[_disputeId].arbitrator = msg.sender;
        
        uint256 arbitrationFee = (disputes[_disputeId].disputedAmount * ARBITRATION_FEE_PERCENTAGE) / 10000;
        disputes[_disputeId].arbitrationFee = arbitrationFee;
        
        arbitrators[_arbitratorId].activeCases++;
        arbitrators[_arbitratorId].totalCases++;
        arbitratorToCases[msg.sender].push(_disputeId);
        
        emit DisputeAccepted(_disputeId, msg.sender, arbitrationFee);
    }
    
    /**
     * @notice Issue arbitration decision (called by assigned arbitrator)
     * @param _disputeId ID of the dispute
     * @param _decision Final decision
     * @param _funderAward Amount awarded to funder
     * @param _granteeAward Amount awarded to grantee
     * @param _ruling Detailed ruling explanation
     */
    function issueDecision(
        uint256 _disputeId,
        ArbitrationDecision _decision,
        uint256 _funderAward,
        uint256 _granteeAward,
        string memory _ruling
    ) external disputeExists(_disputeId) nonReentrant {
        Dispute storage dispute = disputes[_disputeId];
        require(dispute.status == DisputeStatus.InArbitration, "ArbitrationEscrow: Not in arbitration");
        require(dispute.arbitrator == msg.sender, "ArbitrationEscrow: Not assigned arbitrator");
        require(_funderAward + _granteeAward + dispute.arbitrationFee <= dispute.disputedAmount, 
                "ArbitrationEscrow: Awards exceed disputed amount");
        
        dispute.status = DisputeStatus.Resolved;
        dispute.decision = _decision;
        dispute.funderAward = _funderAward;
        dispute.granteeAward = _granteeAward;
        dispute.resolvedAt = block.timestamp;
        
        // Update arbitrator stats
        for (uint256 i = 1; i < nextArbitratorId; i++) {
            if (arbitrators[i].addr == msg.sender) {
                arbitrators[i].activeCases--;
                // Update reputation based on decision quality (simplified)
                if (_decision != ArbitrationDecision.None) {
                    arbitrators[i].reputationScore = _min(1000, arbitrators[i].reputationScore + 10);
                }
                break;
            }
        }
        
        totalEscrowed -= dispute.disputedAmount;
        
        // Transfer funds
        if (_funderAward > 0) {
            (bool success, ) = dispute.funder.call{value: _funderAward}("");
            require(success, "ArbitrationEscrow: Funder transfer failed");
            emit FundsReleased(_disputeId, dispute.funder, _funderAward);
        }
        
        if (_granteeAward > 0) {
            (bool success, ) = dispute.grantee.call{value: _granteeAward}("");
            require(success, "ArbitrationEscrow: Grantee transfer failed");
            emit FundsReleased(_disputeId, dispute.grantee, _granteeAward);
        }
        
        if (dispute.arbitrationFee > 0) {
            (bool success, ) = dispute.arbitrator.call{value: dispute.arbitrationFee}("");
            require(success, "ArbitrationEscrow: Fee transfer failed");
            emit FundsReleased(_disputeId, dispute.arbitrator, dispute.arbitrationFee);
        }
        
        emit ArbitrationDecision(_disputeId, _decision, _funderAward, _granteeAward, dispute.arbitrationFee, _ruling);
    }
    
    /**
     * @notice Get dispute details
     * @param _disputeId ID of the dispute
     * @return Complete dispute information
     */
    function getDispute(uint256 _disputeId) external view disputeExists(_disputeId) returns (Dispute memory) {
        return disputes[_disputeId];
    }
    
    /**
     * @notice Get arbitrator details
     * @param _arbitratorId ID of the arbitrator
     * @return Complete arbitrator information
     */
    function getArbitrator(uint256 _arbitratorId) external view arbitratorExists(_arbitratorId) returns (Arbitrator memory) {
        return arbitrators[_arbitratorId];
    }
    
    /**
     * @notice Get all disputes for a funder
     * @param _funder Address of the funder
     * @return Array of dispute IDs
     */
    function getFunderDisputes(address _funder) external view returns (uint256[] memory) {
        return funderToDisputes[_funder];
    }
    
    /**
     * @notice Get all disputes for a grantee
     * @param _grantee Address of the grantee
     * @return Array of dispute IDs
     */
    function getGranteeDisputes(address _grantee) external view returns (uint256[] memory) {
        return granteeToDisputes[_grantee];
    }
    
    /**
     * @notice Get all cases handled by an arbitrator
     * @param _arbitrator Address of the arbitrator
     * @return Array of dispute IDs
     */
    function getArbitratorCases(address _arbitrator) external view returns (uint256[] memory) {
        return arbitratorToCases[_arbitrator];
    }
    
    /**
     * @notice Get list of approved arbitrators
     * @return Array of approved arbitrator IDs
     */
    function getApprovedArbitrators() external view returns (uint256[] memory) {
        uint256 count = 0;
        for (uint256 i = 1; i < nextArbitratorId; i++) {
            if (arbitrators[i].approved) {
                count++;
            }
        }
        
        uint256[] memory result = new uint256[](count);
        uint256 index = 0;
        for (uint256 i = 1; i < nextArbitratorId; i++) {
            if (arbitrators[i].approved) {
                result[index] = i;
                index++;
            }
        }
        
        return result;
    }
    
    // ─── Admin Functions ───────────────────────────────────────────────────────
    
    /**
     * @notice Emergency pause function
     */
    function pause() external onlyOwner {
        _pause();
    }
    
    /**
     * @notice Unpause function
     */
    function unpause() external onlyOwner {
        _unpause();
    }
    
    /**
     * @notice Emergency withdraw function for stuck funds
     * @param _amount Amount to withdraw
     */
    function emergencyWithdraw(uint256 _amount) external onlyOwner whenPaused {
        require(_amount <= address(this).balance, "ArbitrationEscrow: Insufficient balance");
        (bool success, ) = owner().call{value: _amount}("");
        require(success, "ArbitrationEscrow: Withdrawal failed");
    }
    
    // ─── Internal Functions ─────────────────────────────────────────────────────
    
    function _min(uint256 a, uint256 b) internal pure returns (uint256) {
        return a < b ? a : b;
    }
    
    // ─── Receive Function ───────────────────────────────────────────────────────
    
    receive() external payable {
        totalEscrowed += msg.value;
    }
    
    fallback() external payable {
        totalEscrowed += msg.value;
    }
}
