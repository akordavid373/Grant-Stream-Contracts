# Implement #417 Milestone-Reward-Clawback and #419 Asset-Trustline-Check Features

## Summary
This PR implements two critical security features for the Grant Stream Contracts system to enhance milestone-based release security and handle stream cancellation edge cases.

## Features Implemented

### #417: Milestone-Reward-Clawback for Post-Payout Fraud Detection
- **Community-driven clawback mechanism** with 66% approval threshold and 50% minimum participation
- **30-day voting period** with evidence requirements for fraud detection
- **Automatic fund recovery** system that transfers funds back to contract upon approval
- **Complete audit trail** and status tracking for all clawback activities

### #419: Asset-Trustline-Check for Grantee before Stream Start
- **Automatic trustline verification** before fund distribution begins
- **7-day grace period** for grantees to establish required trustlines
- **Re-checking capability** for resolving trustline issues
- **Comprehensive status tracking** and reporting system

## Implementation Details

### New Functions Added
```rust
// #417: Milestone-Reward-Clawback
pub fn propose_milestone_clawback(env, grant_id, milestone_claim_id, amount, reason, evidence)
pub fn vote_milestone_clawback(env, clawback_id, vote_for)

// #419: Asset-Trustline-Check
pub fn check_grantee_trustline(env, grant_id)
pub fn recheck_trustline(env, check_id)
pub fn get_trustline_check_status(env, check_id)
```

### New Data Structures
```rust
// #417: MilestoneClawbackRequest, ClawbackStatus
// #419: TrustlineCheckRecord, TrustlineStatus
```

### New Storage Keys
- `MilestoneClawbackRequest(u64)` - Maps clawback_id to request details
- `MilestoneClawbackRequests(u64)` - Maps grant_id to list of clawback IDs
- `ClawbackVotes(u64, Address)` - Maps clawback_id + voter to their vote
- `TrustlineCheckRecord(u64)` - Maps check_id to trustline check record
- `TrustlineCheckRecords(u64)` - Maps grant_id to list of trustline check IDs

### Security Considerations
- **Voting Thresholds**: 66% approval required with 50% minimum participation
- **Time Constraints**: 30-day voting period for clawbacks, 7-day grace period for trustlines
- **Evidence Requirements**: Detailed evidence required for clawback proposals
- **Amount Validation**: Cannot clawback more than original milestone amount
- **Status Validation**: Only paid milestones can be clawed back

## Testing
Comprehensive test suite added in `test_milestone_clawback_trustline.rs` covering:
- Normal workflow scenarios for both features
- Error conditions and edge cases
- Authorization and permission checks
- Time-based behaviors (timeouts, grace periods)
- State transitions and validation

## Files Changed
- `contracts/grant_contracts/src/lib.rs` - Main implementation (1,300+ lines added)
- `contracts/grant_contracts/src/test_milestone_clawback_trustline.rs` - Test suite (new)
- `MILESTONE_CLAWBACK_TRUSTLINE_IMPLEMENTATION.md` - Documentation (new)

## Usage Examples

### Milestone Clawback
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

### Trustline Check
```rust
// Check if grantee has trustline for the asset
let check_id = contract.check_grantee_trustline(env, grant_id)?;

// If failed, grantee can establish trustline and then re-check
contract.recheck_trustline(env, check_id)?;

// Get current status
let status = contract.get_trustline_check_status(env, check_id)?;
```

## Breaking Changes
None - this implementation is fully backward compatible.

## Dependencies
No new dependencies added.

## Checklist
- [x] Code implements the requested features
- [x] Comprehensive test suite added
- [x] Documentation provided
- [x] Error handling implemented
- [x] Security considerations addressed
- [x] Backward compatibility maintained

## Related Issues
- Closes #417: Milestone-Reward-Clawback for Post-Payout Fraud Detection
- Closes #419: Asset-Trustline-Check for Grantee before Stream Start
