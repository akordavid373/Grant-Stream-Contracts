# Milestone-Reward-Clawback and Asset-Trustline-Check Implementation

This document describes the implementation of two critical security features for the Grant Stream Contracts system:

1. **#417: Milestone-Reward-Clawback for Post-Payout Fraud Detection**
2. **#419: Asset-Trustline-Check for Grantee before Stream Start**

---

## #417: Milestone-Reward-Clawback for Post-Payout Fraud Detection

### Purpose
Enables the community to claw back milestone rewards if fraud is detected after payout. This provides a safety mechanism for protecting grant funds from fraudulent claims.

### Key Features
- **Post-Payout Protection**: Allows clawback after milestone funds have been released
- **Community Voting**: Uses democratic voting to determine clawback validity
- **Evidence-Based**: Requires detailed evidence for clawback proposals
- **Time-Bound**: 30-day challenge period for voting
- **Transparent**: Complete audit trail of all clawback activities

### Functions
- `propose_milestone_clawback(grant_id, milestone_claim_id, amount, reason, evidence)` - Propose clawback
- `vote_milestone_clawback(clawback_id, vote_for)` - Vote on clawback proposal

### Data Structures
```rust
pub struct MilestoneClawbackRequest {
    pub clawback_id: u64,
    pub grant_id: u64,
    pub milestone_claim_id: u64,
    pub clawbacker: Address,
    pub grantee: Address,
    pub amount: i128,
    pub reason: String,
    pub evidence: String,
    pub created_at: u64,
    pub challenge_deadline: u64,
    pub status: ClawbackStatus,
    pub votes_for: i128,
    pub votes_against: i128,
    pub total_voting_power: i128,
    pub executed_at: Option<u64>,
    pub clawback_reason: Option<String>,
}

pub enum ClawbackStatus {
    Proposed,        // Clawback proposed, voting period open
    Approved,        // Clawback approved, ready for execution
    Rejected,        // Clawback rejected by vote
    Executed,        // Clawback successfully executed
    Expired,         // Voting period expired
    Cancelled,       // Request cancelled by proposer
}
```

### Constants
```rust
const MILESTONE_CLAWBACK_CHALLENGE_PERIOD: u64 = 30 * 24 * 60 * 60; // 30 days
const MAX_CLAWBACK_REASON_LENGTH: u32 = 1000;
const MAX_CLAWBACK_EVIDENCE_LENGTH: u32 = 2000;
const CLAWBACK_VOTING_THRESHOLD: u32 = 6600; // 66% approval required
const MIN_CLAWBACK_VOTING_PARTICIPATION: u32 = 5000; // 50% minimum participation
```

### Usage Example
```rust
// Propose clawback for fraudulent milestone
let clawback_id = contract.propose_milestone_clawback(
    env,
    grant_id,
    milestone_claim_id,
    500, // amount to clawback
    "Fraudulent work submission".to_string(),
    "Evidence: Plagiarized code found".to_string(),
)?;

// Vote on the clawback
contract.vote_milestone_clawback(env, clawback_id, true)?;
```

---

## #419: Asset-Trustline-Check for Grantee before Stream Start

### Purpose
Verifies that grantees have established proper trustlines for the grant asset before stream starts, preventing failed payments and ensuring smooth fund distribution.

### Key Features
- **Pre-Stream Validation**: Checks trustlines before funds start flowing
- **Automatic Detection**: Automatically detects missing trustlines
- **Grace Period**: 7-day window for grantees to establish trustlines
- **Re-Checking**: Allows re-checking after trustline issues are resolved
- **Status Tracking**: Complete tracking of trustline verification status

### Functions
- `check_grantee_trustline(grant_id)` - Check if grantee has trustline
- `recheck_trustline(check_id)` - Re-check after fixing issues
- `get_trustline_check_status(check_id)` - Get current status

### Data Structures
```rust
pub struct TrustlineCheckRecord {
    pub check_id: u64,
    pub grant_id: u64,
    pub grantee: Address,
    pub asset_address: Address,
    pub checked_at: u64,
    pub status: TrustlineStatus,
    pub failure_reason: Option<String>,
    pub resolved_at: Option<u64>,
}

pub enum TrustlineStatus {
    Pending,         // Trustline check pending
    Verified,        // Trustline verified and active
    Failed,          // Trustline check failed
    Resolved,        // Trustline issue resolved
    Expired,         // Check timeout expired
}
```

### Constants
```rust
const TRUSTLINE_CHECK_TIMEOUT: u64 = 7 * 24 * 60 * 60; // 7 days
const MAX_TRUSTLINE_REASON_LENGTH: u32 = 500;
```

### Usage Example
```rust
// Check if grantee has trustline for the asset
let check_id = contract.check_grantee_trustline(env, grant_id)?;

// If failed, grantee can establish trustline and then re-check
contract.recheck_trustline(env, check_id)?;

// Get current status
let status = contract.get_trustline_check_status(env, check_id)?;
```

---

## Implementation Details

### Storage Keys
New storage keys have been added for both features:

```rust
// #417: Milestone-Reward-Clawback keys
MilestoneClawbackRequest(u64), // Maps clawback_id to clawback request details
MilestoneClawbackRequests(u64), // Maps grant_id to list of clawback request IDs
NextMilestoneClawbackRequestId, // Next available clawback request ID
MilestoneClawbackIds,          // List of all clawback request IDs
ClawbackVotes(u64, Address),   // Maps clawback_id + voter to their vote

// #419: Asset-Trustline-Check keys
TrustlineCheckRecord(u64),     // Maps check_id to trustline check record
TrustlineCheckRecords(u64),    // Maps grant_id to list of trustline check IDs
NextTrustlineCheckId,          // Next available trustline check ID
TrustlineCheckIds,             // List of all trustline check IDs
```

### Error Handling
Comprehensive error types have been added for both features:

#### #417 Errors
- `ClawbackRequestNotFound` - Clawback request not found
- `ClawbackAlreadyExecuted` - Clawback already executed
- `ClawbackVotingExpired` - Voting period expired
- `ClawbackNotApproved` - Clawback not approved
- `InvalidClawbackAmount` - Invalid clawback amount
- `ClawbackChallengePeriodActive` - Challenge period still active

#### #419 Errors
- `TrustlineCheckNotFound` - Trustline check not found
- `TrustlineCheckExpired` - Trustline check expired
- `TrustlineNotEstablished` - Trustline not established
- `TrustlineVerificationFailed` - Trustline verification failed
- `AssetAddressInvalid` - Asset address invalid

### Security Considerations

#### #417 Security
1. **Voting Thresholds**: 66% approval required with 50% minimum participation
2. **Time Constraints**: 30-day voting period prevents stale issues
3. **Evidence Requirements**: Detailed evidence required for proposals
4. **Amount Validation**: Cannot clawback more than original milestone amount
5. **Status Validation**: Only paid milestones can be clawed back

#### #419 Security
1. **Pre-Validation**: Checks trustlines before any funds flow
2. **Grace Period**: 7-day window allows time for trustline establishment
3. **Automatic Detection**: Uses token balance checks to detect trustline issues
4. **Status Tracking**: Complete audit trail of verification attempts
5. **Timeout Protection**: Prevents indefinite pending states

### Integration Points

#### #417 Integration
- Integrates with existing milestone system
- Uses existing token transfer mechanisms
- Leverages existing voting infrastructure
- Compatible with existing grant lifecycle

#### #419 Integration
- Integrates with grant creation process
- Uses existing token client for balance checks
- Compatible with multi-asset grants
- Works with existing grant status system

### Testing

Comprehensive tests have been implemented in `test_milestone_clawback_trustline.rs` covering:

#### #417 Tests
- Normal clawback proposal workflow
- Voting mechanism and threshold validation
- Error conditions and edge cases
- Status transitions and validation
- Execution of approved clawbacks

#### #419 Tests
- Successful trustline verification
- Failed trustline detection
- Re-checking after resolution
- Timeout handling
- Status retrieval functionality

### Running Tests
```bash
cargo test --package grant_contracts test_milestone_clawback_trustline
```

---

## Future Enhancements

### #417 Enhancements
1. **Multi-Sig Support**: Require multiple signatures for clawback execution
2. **Delegation**: Allow delegation of voting authority
3. **Batch Processing**: Support for batch clawback proposals
4. **Escrow Integration**: Hold funds in escrow during voting period
5. **Appeal Process**: Add appeal mechanism for rejected clawbacks

### #419 Enhancements
1. **Multi-Asset Support**: Check trustlines for multiple assets
2. **Automatic Notifications**: Notify grantees of trustline issues
3. **Pre-Flight Checks**: Integrate with grant creation flow
4. **Trustline Suggestions**: Suggest trustline establishment steps
5. **Integration with Wallets**: Direct integration with wallet providers

---

## Conclusion

These two features significantly enhance the Grant Stream Contracts system by:

- **#417**: Providing a robust mechanism for post-payout fraud detection and recovery
- **#419**: Ensuring smooth fund distribution by pre-validating trustlines

The implementation follows best practices for smart contract security, including proper authorization, time-based constraints, comprehensive error handling, and thorough testing. Both features are designed to be backward compatible and integrate seamlessly with the existing codebase.
