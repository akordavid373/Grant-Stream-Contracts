# Grant Stream Contracts

Smart contracts for managing grant streams with milestone completion proof hashing and integrated dispute resolution system.

## Overview

This implementation addresses multiple issues:
- **Issue #203**: Support for Milestone Completion Proof Hashing
- **Issue #201**: Dispute Resolution Arbitration Escrow System

The contracts allow grantees to submit SHA-256 hashes of their deliverables as immutable proof of milestone completion, while providing a comprehensive Web3 courtroom for high-stakes grants with transparent dispute resolution.

## Features

### Core Functionality
- **Grant Creation**: Create grant streams with multiple milestones
- **Milestone Management**: Define milestones with deadlines and amounts
- **Proof Submission**: Submit SHA-256 hashes of deliverables (PDFs, GitHub releases, etc.)
- **Immutable Audit Trail**: Store completion time, proof hash, and metadata on-chain
- **Access Control**: Role-based permissions for creators and grantees

### Dispute Resolution System 🆕
- **Arbitration Escrow**: Secure fund holding during disputes
- **Third-Party Arbitrators**: Pre-approved legal firms and decentralized courts
- **Web3 Courtroom**: Public interface for transparent dispute proceedings
- **Reputation System**: Arbitrator scoring and performance tracking
- **Flexible Decisions**: Support for partial refunds, split awards, and full resolutions

### Hashing Utilities
- SHA-256 hashing for various data types
- File metadata hashing (filename, content, type)
- GitHub release metadata hashing
- Concatenated data hashing
- Proof verification functions

## Key Components

### GrantStream.sol
Main contract implementing the grant stream functionality with proof hashing.

### GrantStreamWithArbitration.sol 🆕
Enhanced grant streaming contract with integrated dispute resolution system.

### ArbitrationEscrow.sol 🆕
Core dispute resolution and escrow management contract.

### Web3Courtroom.sol 🆕
Public interface for transparent dispute proceedings and arbitrator management.

### HashUtils.sol
Library providing SHA-256 hashing utilities for different use cases.

## Usage Examples

### Creating a Grant with Milestones

```solidity
// Create a grant
uint256 grantId = grantStream.createGrant(
    granteeAddress,
    1000 ether,
    "Research Grant",
    "Funding for blockchain research"
);

// Create milestones
uint256 milestone1 = grantStream.createMilestone(
    grantId,
    "Literature Review",
    "Complete comprehensive literature review",
    250 ether,
    deadline1
);

uint256 milestone2 = grantStream.createMilestone(
    grantId,
    "Prototype Development",
    "Build working prototype",
    500 ether,
    deadline2
);
```

### Submitting Proof for Milestone Completion

```solidity
// For a PDF report
bytes memory pdfContent = "..."; // PDF file content
bytes32 proofHash = sha256(pdfContent);

grantStream.submitProof(
    milestoneId,
    proofHash,
    "Q1 Research Report PDF"
);

// For a GitHub release
bytes32 releaseHash = HashUtils.createReleaseHash(
    "https://github.com/user/repo",
    "v1.0.0",
    "abc123def456",
    "Initial release with all features"
);

grantStream.submitProof(
    milestoneId,
    releaseHash,
    "GitHub Release v1.0.0"
);
```

### Verifying Proof

```solidity
// Verify that submitted data matches the stored hash
bool isValid = grantStream.verifyProof(storedHash, originalData);

// Get immutable audit trail
(uint256 completionTime, bytes32 proofHash, string memory metadata) = 
    grantStream.getMilestoneAuditTrail(milestoneId);
```

### Dispute Resolution Workflow 🆕

```solidity
// 1. Create grant with arbitration support
GrantStreamWithArbitration grantStream = new GrantStreamWithArbitration(
    sustainabilityFundAddress,
    arbitrationEscrowAddress
);

uint256 grantId = grantStream.createGrant{value: 10 ether}(grantee);

// 2. Raise dispute if project not delivered
grantStream.raiseDispute{value: 5 ether}(
    grantId,
    5 ether,
    "QmEvidenceHash",
    "Project not delivered as promised"
);

// 3. Arbitrator accepts case (2% fee)
arbitrationEscrow.acceptDispute(disputeId, arbitratorId);

// 4. Issue final decision
arbitrationEscrow.issueDecision(
    disputeId,
    ArbitrationDecision.FavorFunder,
    3 ether,    // Funder award
    1.95 ether, // Grantee award
    "Partial refund granted"
);
```

## Installation

```bash
# Clone the repository
git clone https://github.com/lifewithbigdamz/Grant-Stream-Contracts.git
cd Grant-Stream-Contracts

# Install dependencies
forge install

# Build contracts
forge build

# Run tests
forge test
```

## Testing

The contracts include comprehensive tests covering:

### Milestone System Tests
- Valid proof submission scenarios
- GitHub release and PDF file hashing
- Access control and permission checks
- Deadline enforcement
- Hash verification
- Audit trail functionality
- Multiple milestone management

### Dispute Resolution Tests 🆕
- Dispute raising and escrow movement
- Arbitrator acceptance and decision issuance
- Fund distribution and fee handling
- Access control and authorization
- Emergency scenarios and pause functionality
- Web3 Courtroom integration
- Reputation system functionality

Run tests with:
```bash
# All tests
forge test

# Arbitration system tests
forge test --match-contract ArbitrationEscrowTest

# Test coverage
forge coverage
```

## Security Considerations

### Core System Security
- **Reentrancy Protection**: Uses OpenZeppelin's ReentrancyGuard
- **Access Control**: Role-based permissions for creators, grantees, and arbitrators
- **Input Validation**: Validates all inputs including proof hashes and deadlines
- **Immutable Storage**: Once submitted, proofs cannot be modified
- **Gas Optimization**: Efficient storage patterns for milestone data

### Dispute Resolution Security 🆕
- **Escrow Protection**: Secure fund holding with multi-signature controls
- **Arbitrator Vetting**: Pre-approved arbitrator system with reputation tracking
- **Emergency Controls**: Pause and emergency withdraw functions
- **Audit Trail**: Complete transaction history for all dispute operations
- **Reentrancy Safety**: All external functions protected against reentrancy attacks

## Audit Trail Features

The immutable audit trail provides:

1. **Completion Timestamp**: Exact time when proof was submitted
2. **Proof Hash**: SHA-256 hash of the deliverable
3. **Metadata**: Optional descriptive information about the proof
4. **Grantee Address**: Address that submitted the proof
5. **Milestone Details**: Full context of the completed milestone

This creates a verifiable, tamper-proof record suitable for institutional audit requirements.

## Integration

The contracts are designed to integrate with:

### Milestone System Integration
- **IPFS/Filecoin**: For storing actual deliverable files
- **GitHub API**: For automatic release hash generation
- **Document Management Systems**: For PDF report processing
- **Audit Platforms**: For compliance verification

### Dispute Resolution Integration 🆕
- **Kleros**: Decentralized justice protocol
- **Aragon Court**: Digital dispute resolution platform
- **Traditional Legal Firms**: Established law practice integration
- **DAO Governance**: Community-driven arbitrator approval

## Documentation

- **[Arbitration System Documentation](docs/ARBITRATION_SYSTEM.md)** - Complete technical documentation
- **[API Reference](docs/API.md)** - Frontend integration guide
- **[Security Audit](docs/SECURITY_AUDIT.md)** - Security analysis and recommendations

## Deployment

### Quick Deploy
```bash
# Set environment variables
export PRIVATE_KEY=0x...
export TREASURY_ADDRESS=0x...
export ARBITRATOR_1_ADDRESS=0x...
export ARBITRATOR_2_ADDRESS=0x...

# Deploy arbitration system
forge script script/DeployArbitrationSystem.s.sol --rpc-url <RPC_URL> --broadcast
```

### Individual Contract Deployment
See individual contract documentation for specific deployment requirements.

## License

MIT License - see LICENSE file for details.

## Contributing

Please follow the contribution guidelines and ensure all tests pass before submitting pull requests.

## Issues

For issues related to:
- **Milestone completion proof hashing**: Please reference Issue #203
- **Dispute resolution arbitration**: Please reference Issue #201
- **General questions**: Use GitHub Discussions

## Contributing

Please follow the contribution guidelines and ensure all tests pass before submitting pull requests.

## License

MIT License - see LICENSE file for details.

---

Built with ❤️ for the Web3 grant ecosystem. Supporting transparent milestones and fair dispute resolution.
