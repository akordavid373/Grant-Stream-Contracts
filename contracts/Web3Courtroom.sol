// SPDX-License-Identifier: MIT
pragma solidity ^0.8.24;

import "@openzeppelin/contracts/access/Ownable.sol";
import "./ArbitrationEscrow.sol";

/**
 * @title Web3Courtroom
 * @notice Transparent interface for dispute resolution proceedings
 * @dev Provides a public interface for viewing and participating in arbitration cases
 *      Implements a "Web3 Courtroom" where anyone can view ongoing cases and evidence
 */
contract Web3Courtroom is Ownable {
    
    // ─── Structs ──────────────────────────────────────────────────────────────
    
    struct CourtCase {
        uint256 disputeId;
        uint256 grantId;
        address funder;
        address grantee;
        address arbitrator;
        string title;                 // Case title derived from dispute reason
        string description;          // Full case description
        string evidence;             // Evidence IPFS hash/URL
        uint256 filingDate;
        uint256 lastUpdate;
        ArbitrationEscrow.DisputeStatus status;
        uint256 stakeAmount;         // Amount in dispute
        bool isPublic;               // Whether case details are public
        string[] publicEvidence;     // Array of publicly available evidence
        string[] rulings;            // Historical rulings/comments
        mapping(address => bool) hasAccess;  // Who can view private details
    }
    
    struct ArbitratorProfile {
        address arbitratorAddress;
        string name;
        string jurisdiction;
        uint256 reputationScore;
        uint256 totalCases;
        uint256 wonCases;
        uint256 averageResolutionTime; // In hours
        string specialization;        // e.g., "Software Development", "DeFi"
        bool isVerified;
        string website;
        string contactInfo;
    }
    
    // ─── State ────────────────────────────────────────────────────────────────
    
    ArbitrationEscrow public immutable arbitrationEscrow;
    
    mapping(uint256 => CourtCase) public courtCases;
    mapping(address => ArbitratorProfile) public arbitratorProfiles;
    
    uint256[] public activeCases;
    uint256[] public resolvedCases;
    uint256[] public pendingCases;
    
    // Statistics
    uint256 public totalCases;
    uint256 public totalValueDisputed;
    uint256 public averageResolutionTime;
    uint256 public publicCasesCount;
    
    // ─── Events ───────────────────────────────────────────────────────────────
    
    event CaseFiled(
        uint256 indexed disputeId,
        uint256 indexed grantId,
        address indexed funder,
        string title,
        uint256 stakeAmount
    );
    
    caseUpdated(uint256 indexed disputeId, string update, address updatedBy);
    evidenceSubmitted(uint256 indexed disputeId, string evidence, address submittedBy);
    rulingIssued(uint256 indexed disputeId, string ruling, address arbitrator);
    arbitratorRegistered(address indexed arbitrator, string name, string specialization);
    caseMadePublic(uint256 indexed disputeId, address madePublicBy);
    
    // ─── Constructor ──────────────────────────────────────────────────────────
    
    constructor(address _arbitrationEscrow) Ownable(msg.sender) {
        require(_arbitrationEscrow != address(0), "Web3Courtroom: zero address");
        arbitrationEscrow = ArbitrationEscrow(_arbitrationEscrow);
    }
    
    // ─── External Functions ─────────────────────────────────────────────────────
    
    /**
     * @notice Register an arbitrator profile in the courtroom
     * @param _arbitrator Address of the arbitrator
     * @param _name Display name
     * @param _jurisdiction Legal jurisdiction
     * @param _specialization Area of expertise
     * @param _website Website URL
     * @param _contactInfo Contact information
     */
    function registerArbitratorProfile(
        address _arbitrator,
        string memory _name,
        string memory _jurisdiction,
        string memory _specialization,
        string memory _website,
        string memory _contactInfo
    ) external {
        require(_arbitrator != address(0), "Web3Courtroom: zero address");
        require(bytes(_name).length > 0, "Web3Courtroom: empty name");
        
        // Verify arbitrator is approved in ArbitrationEscrow
        uint256[] memory approvedArbitrators = arbitrationEscrow.getApprovedArbitrators();
        bool isApproved = false;
        for (uint256 i = 0; i < approvedArbitrators.length; i++) {
            ArbitrationEscrow.Arbitrator memory arb = arbitrationEscrow.getArbitrator(approvedArbitrators[i]);
            if (arb.addr == _arbitrator) {
                isApproved = true;
                break;
            }
        }
        require(isApproved, "Web3Courtroom: arbitrator not approved");
        
        arbitratorProfiles[_arbitrator] = ArbitratorProfile({
            arbitratorAddress: _arbitrator,
            name: _name,
            jurisdiction: _jurisdiction,
            reputationScore: 500, // Start with neutral score
            totalCases: 0,
            wonCases: 0,
            averageResolutionTime: 0,
            specialization: _specialization,
            isVerified: true,
            website: _website,
            contactInfo: _contactInfo
        });
        
        emit arbitratorRegistered(_arbitrator, _name, _specialization);
    }
    
    /**
     * @notice Create a court case from a dispute
     * @param _disputeId ID of the dispute
     * @param _title Case title
     * @param _description Case description
     * @param _isPublic Whether the case should be public
     */
    function createCourtCase(
        uint256 _disputeId,
        string memory _title,
        string memory _description,
        bool _isPublic
    ) external {
        ArbitrationEscrow.Dispute memory dispute = arbitrationEscrow.getDispute(_disputeId);
        require(dispute.exists, "Web3Courtroom: dispute does not exist");
        require(courtCases[_disputeId].disputeId == 0, "Web3Courtroom: case already exists");
        
        CourtCase storage courtCase = courtCases[_disputeId];
        courtCase.disputeId = _disputeId;
        courtCase.grantId = dispute.grantId;
        courtCase.funder = dispute.funder;
        courtCase.grantee = dispute.grantee;
        courtCase.arbitrator = dispute.arbitrator;
        courtCase.title = _title;
        courtCase.description = _description;
        courtCase.evidence = dispute.evidence;
        courtCase.filingDate = dispute.createdAt;
        courtCase.lastUpdate = block.timestamp;
        courtCase.status = dispute.status;
        courtCase.stakeAmount = dispute.disputedAmount;
        courtCase.isPublic = _isPublic;
        
        // Grant access to involved parties
        courtCase.hasAccess[dispute.funder] = true;
        courtCase.hasAccess[dispute.grantee] = true;
        if (dispute.arbitrator != address(0)) {
            courtCase.hasAccess[dispute.arbitrator] = true;
        }
        
        // Update statistics
        totalCases++;
        totalValueDisputed += dispute.disputedAmount;
        if (_isPublic) {
            publicCasesCount++;
        }
        
        // Add to appropriate case lists
        if (dispute.status == ArbitrationEscrow.DisputeStatus.Pending) {
            pendingCases.push(_disputeId);
        } else if (dispute.status == ArbitrationEscrow.DisputeStatus.InArbitration) {
            activeCases.push(_disputeId);
        } else if (dispute.status == ArbitrationEscrow.DisputeStatus.Resolved) {
            resolvedCases.push(_disputeId);
        }
        
        emit CaseFiled(_disputeId, dispute.grantId, dispute.funder, _title, dispute.disputedAmount);
    }
    
    /**
     * @notice Submit additional evidence to a case
     * @param _disputeId ID of the dispute/case
     * @param _evidence IPFS hash or URL of evidence
     */
    function submitEvidence(uint256 _disputeId, string memory _evidence) external {
        CourtCase storage courtCase = courtCases[_disputeId];
        require(courtCase.disputeId != 0, "Web3Courtroom: case does not exist");
        require(courtCase.hasAccess[msg.sender], "Web3Courtroom: no access");
        require(bytes(_evidence).length > 0, "Web3Courtroom: empty evidence");
        
        courtCase.publicEvidence.push(_evidence);
        courtCase.lastUpdate = block.timestamp;
        
        emit evidenceSubmitted(_disputeId, _evidence, msg.sender);
    }
    
    /**
     * @notice Add a ruling or comment to a case
     * @param _disputeId ID of the dispute/case
     * @param _ruling Ruling text or comment
     */
    function addRuling(uint256 _disputeId, string memory _ruling) external {
        CourtCase storage courtCase = courtCases[_disputeId];
        require(courtCase.disputeId != 0, "Web3Courtroom: case does not exist");
        require(courtCase.hasAccess[msg.sender], "Web3Courtroom: no access");
        require(bytes(_ruling).length > 0, "Web3Courtroom: empty ruling");
        
        courtCase.rulings.push(_ruling);
        courtCase.lastUpdate = block.timestamp;
        
        emit rulingIssued(_disputeId, _ruling, msg.sender);
    }
    
    /**
     * @notice Make a case public
     * @param _disputeId ID of the dispute/case
     */
    function makeCasePublic(uint256 _disputeId) external {
        CourtCase storage courtCase = courtCases[_disputeId];
        require(courtCase.disputeId != 0, "Web3Courtroom: case does not exist");
        require(
            msg.sender == courtCase.funder || 
            msg.sender == courtCase.grantee || 
            msg.sender == courtCase.arbitrator ||
            msg.sender == owner(),
            "Web3Courtroom: unauthorized"
        );
        
        if (!courtCase.isPublic) {
            courtCase.isPublic = true;
            publicCasesCount++;
            emit caseMadePublic(_disputeId, msg.sender);
        }
    }
    
    /**
     * @notice Grant access to a case for a specific address
     * @param _disputeId ID of the dispute/case
     * @param _address Address to grant access to
     */
    function grantCaseAccess(uint256 _disputeId, address _address) external {
        CourtCase storage courtCase = courtCases[_disputeId];
        require(courtCase.disputeId != 0, "Web3Courtroom: case does not exist");
        require(
            msg.sender == courtCase.funder || 
            msg.sender == courtCase.grantee || 
            msg.sender == courtCase.arbitrator ||
            msg.sender == owner(),
            "Web3Courtroom: unauthorized"
        );
        
        courtCase.hasAccess[_address] = true;
    }
    
    // ─── View Functions ─────────────────────────────────────────────────────────
    
    /**
     * @notice Get public case information
     * @param _disputeId ID of the dispute/case
     * @return Public case details
     */
    function getPublicCase(uint256 _disputeId) external view returns (
        uint256 disputeId,
        uint256 grantId,
        string memory title,
        string memory description,
        uint256 filingDate,
        uint256 lastUpdate,
        ArbitrationEscrow.DisputeStatus status,
        uint256 stakeAmount,
        bool isPublic
    ) {
        CourtCase storage courtCase = courtCases[_disputeId];
        require(courtCase.disputeId != 0, "Web3Courtroom: case does not exist");
        require(courtCase.isPublic, "Web3Courtroom: case not public");
        
        return (
            courtCase.disputeId,
            courtCase.grantId,
            courtCase.title,
            courtCase.description,
            courtCase.filingDate,
            courtCase.lastUpdate,
            courtCase.status,
            courtCase.stakeAmount,
            courtCase.isPublic
        );
    }
    
    /**
     * @notice Get full case information (requires access)
     * @param _disputeId ID of the dispute/case
     * @return Complete case details
     */
    function getFullCase(uint256 _disputeId) external view returns (CourtCase memory) {
        CourtCase storage courtCase = courtCases[_disputeId];
        require(courtCase.disputeId != 0, "Web3Courtroom: case does not exist");
        require(courtCase.isPublic || courtCase.hasAccess[msg.sender], "Web3Courtroom: no access");
        
        return courtCase;
    }
    
    /**
     * @notice Get all active cases
     * @return Array of active dispute IDs
     */
    function getActiveCases() external view returns (uint256[] memory) {
        return activeCases;
    }
    
    /**
     * @notice Get all resolved cases
     * @return Array of resolved dispute IDs
     */
    function getResolvedCases() external view returns (uint256[] memory) {
        return resolvedCases;
    }
    
    /**
     * @notice Get all pending cases
     * @return Array of pending dispute IDs
     */
    function getPendingCases() external view returns (uint256[] memory) {
        return pendingCases;
    }
    
    /**
     * @notice Get arbitrator profile
     * @param _arbitrator Address of the arbitrator
     * @return Arbitrator profile information
     */
    function getArbitratorProfile(address _arbitrator) external view returns (ArbitratorProfile memory) {
        return arbitratorProfiles[_arbitrator];
    }
    
    /**
     * @notice Get courtroom statistics
     * @return Various statistics about the courtroom
     */
    function getStatistics() external view returns (
        uint256 _totalCases,
        uint256 _totalValueDisputed,
        uint256 _averageResolutionTime,
        uint256 _publicCasesCount,
        uint256 activeCasesCount,
        uint256 resolvedCasesCount,
        uint256 pendingCasesCount
    ) {
        return (
            totalCases,
            totalValueDisputed,
            averageResolutionTime,
            publicCasesCount,
            activeCases.length,
            resolvedCases.length,
            pendingCases.length
        );
    }
    
    /**
     * @notice Check if an address has access to a case
     * @param _disputeId ID of the dispute/case
     * @param _address Address to check
     * @return Whether the address has access
     */
    function hasCaseAccess(uint256 _disputeId, address _address) external view returns (bool) {
        CourtCase storage courtCase = courtCases[_disputeId];
        return courtCase.isPublic || courtCase.hasAccess[_address];
    }
    
    // ─── Internal Functions ─────────────────────────────────────────────────────
    
    /**
     * @notice Update case status when dispute status changes
     * @param _disputeId ID of the dispute
     * @param _newStatus New dispute status
     */
    function _updateCaseStatus(uint256 _disputeId, ArbitrationEscrow.DisputeStatus _newStatus) internal {
        CourtCase storage courtCase = courtCases[_disputeId];
        if (courtCase.disputeId == 0) return;
        
        ArbitrationEscrow.DisputeStatus oldStatus = courtCase.status;
        courtCase.status = _newStatus;
        courtCase.lastUpdate = block.timestamp;
        
        // Move case between lists based on status change
        _moveCaseBetweenLists(_disputeId, oldStatus, _newStatus);
        
        // Update statistics if case was resolved
        if (_newStatus == ArbitrationEscrow.DisputeStatus.Resolved && oldStatus != ArbitrationEscrow.DisputeStatus.Resolved) {
            uint256 resolutionTime = (block.timestamp - courtCase.filingDate) / 3600; // Convert to hours
            _updateAverageResolutionTime(resolutionTime);
        }
    }
    
    function _moveCaseBetweenLists(
        uint256 _disputeId,
        ArbitrationEscrow.DisputeStatus _oldStatus,
        ArbitrationEscrow.DisputeStatus _newStatus
    ) internal {
        // Remove from old list
        if (_oldStatus == ArbitrationEscrow.DisputeStatus.Pending) {
            _removeFromArray(pendingCases, _disputeId);
        } else if (_oldStatus == ArbitrationEscrow.DisputeStatus.InArbitration) {
            _removeFromArray(activeCases, _disputeId);
        } else if (_oldStatus == ArbitrationEscrow.DisputeStatus.Resolved) {
            _removeFromArray(resolvedCases, _disputeId);
        }
        
        // Add to new list
        if (_newStatus == ArbitrationEscrow.DisputeStatus.Pending) {
            pendingCases.push(_disputeId);
        } else if (_newStatus == ArbitrationEscrow.DisputeStatus.InArbitration) {
            activeCases.push(_disputeId);
        } else if (_newStatus == ArbitrationEscrow.DisputeStatus.Resolved) {
            resolvedCases.push(_disputeId);
        }
    }
    
    function _removeFromArray(uint256[] storage array, uint256 value) internal {
        for (uint256 i = 0; i < array.length; i++) {
            if (array[i] == value) {
                array[i] = array[array.length - 1];
                array.pop();
                break;
            }
        }
    }
    
    function _updateAverageResolutionTime(uint256 newResolutionTime) internal {
        if (resolvedCases.length == 1) {
            averageResolutionTime = newResolutionTime;
        } else {
            averageResolutionTime = (averageResolutionTime * (resolvedCases.length - 1) + newResolutionTime) / resolvedCases.length;
        }
    }
}
