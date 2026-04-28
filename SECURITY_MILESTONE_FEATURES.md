# Security and Milestone Features Implementation

This document describes the implementation of four critical security and milestone-based features for the Grant Stream Contracts system.

## Overview

The following features have been implemented to enhance security, provide emergency recovery mechanisms, improve milestone approval workflows, and handle complex multi-grantor scenarios:

1. **#415: Authorized Grantee Change Logic for Team Migrations**
2. **#416: Emergency Grace Period for Stream Resumption post-Cancellation**
3. **#414: Staged Approval Workflow (Reviewer → Admin)**
4. **#408: Partial Funding Cancellation for Multi-Grantor Pools**

---

## #415: Authorized Grantee Change Logic for Team Migrations

### Purpose
Enables secure transfer of grant ownership when team members change, preventing unauthorized access while allowing legitimate team migrations.

### Key Features
- **Authorization Required**: Only current grantee or admin can propose changes
- **Admin Authorization**: All changes must be authorized by contract admin
- **Cooldown Period**: 7-day cooldown between changes to prevent abuse
- **Audit Trail**: Complete history of all grantee changes with reasons
- **Time-Bound Authorization**: 30-day authorization window for security

### Functions
- `propose_grantee_change(grant_id, proposed_grantee, reason)` - Propose a change
- `authorize_grantee_change(request_id, authorized, rejection_reason)` - Admin authorization
- `execute_grantee_change(request_id)` - Execute authorized change

### Security Measures
- Dual authorization (grantee + admin)
- Request expiration after 30 days
- Cooldown period between changes
- Comprehensive audit logging

---

## #416: Emergency Grace Period for Stream Resumption post-Cancellation

### Purpose
Provides a safety net for grants that were cancelled in error or due to temporary issues, allowing emergency resumption within a grace period.

### Key Features
- **3-Day Grace Period**: Window for emergency resumption after cancellation
- **Fee-Based Recovery**: 10 XLM fee to prevent abuse
- **Admin Approval**: All resumptions require admin approval
- **Reason Validation**: Detailed justification required
- **Automatic Expiration**: Requests expire if not processed in time

### Functions
- `request_emergency_resumption(grant_id, reason)` - Request resumption
- `pay_emergency_resumption_fee(request_id)` - Pay recovery fee
- `approve_emergency_resumption(request_id, approved, rejection_reason)` - Admin decision

### Security Measures
- Economic disincentive via fee
- Time-limited grace period
- Admin oversight required
- Detailed reason tracking

---

## #414: Staged Approval Workflow (Reviewer → Admin)

### Purpose
Implements a two-stage approval process for milestone claims, ensuring proper review before final admin approval.

### Key Features
- **Sequential Approval**: Reviewer approval required before admin approval
- **14-Day Timeout**: Automatic expiration if not processed
- **Reason Tracking**: Detailed feedback from both reviewer and admin
- **Milestone Integration**: Works with existing milestone system
- **Flexible Roles**: Configurable reviewer and admin addresses

### Functions
- `create_staged_approval(grant_id, milestone_claim_id, reviewer, admin)` - Create approval
- `reviewer_approve(approval_id, approved, reason)` - Reviewer decision
- `admin_approve(approval_id, approved, reason)` - Final admin decision

### Security Measures
- Enforced approval sequence
- Time-based expiration
- Role-based access control
- Comprehensive audit trail

---

## #408: Partial Funding Cancellation for Multi-Grantor Pools

### Purpose
Enables partial cancellation of funding when multiple grantors are involved, with proportional approval requirements.

### Key Features
- **Proportional Approval**: Requires >50% approval by share weight
- **Challenge Period**: 5-day window for objections
- **Multi-Grantor Support**: Up to 10 grantors per grant
- **Share-Based Voting**: Approval weighted by contribution percentage
- **Partial Amounts**: Can cancel portions without affecting entire grant

### Functions
- `propose_partial_cancellation(grant_id, cancellation_amount, reason)` - Propose cancellation
- `approve_partial_cancellation(request_id, approved, reason)` - Grantor approval

### Security Measures
- Majority approval required
- Challenge period for objections
- Share-weighted voting
- Minimum share requirements

---

## Implementation Details

### New Data Types

#### GranteeChangeRequest
```rust
pub struct GranteeChangeRequest {
    pub request_id: u64,
    pub grant_id: u64,
    pub current_grantee: Address,
    pub proposed_grantee: Address,
    pub authorizer: Address,
    pub reason: String,
    pub created_at: u64,
    pub authorization_deadline: u64,
    pub status: GranteeChangeStatus,
    pub authorized_at: Option<u64>,
    pub executed_at: Option<u64>,
    pub rejection_reason: Option<String>,
}
```

#### EmergencyResumptionRequest
```rust
pub struct EmergencyResumptionRequest {
    pub request_id: u64,
    pub grant_id: u64,
    pub requester: Address,
    pub reason: String,
    pub created_at: u64,
    pub grace_period_end: u64,
    pub status: EmergencyResumptionStatus,
    pub approved_at: Option<u64>,
    pub approved_by: Option<Address>,
    pub rejection_reason: Option<String>,
    pub fee_paid: bool,
}
```

#### StagedApproval
```rust
pub struct StagedApproval {
    pub approval_id: u64,
    pub grant_id: u64,
    pub milestone_claim_id: u64,
    pub reviewer: Address,
    pub admin: Address,
    pub reviewer_approval: bool,
    pub admin_approval: bool,
    pub reviewer_reason: Option<String>,
    pub admin_reason: Option<String>,
    pub reviewer_approved_at: Option<u64>,
    pub admin_approved_at: Option<u64>,
    pub created_at: u64,
    pub deadline: u64,
    pub status: StagedApprovalStatus,
}
```

#### PartialCancellationRequest
```rust
pub struct PartialCancellationRequest {
    pub request_id: u64,
    pub grant_id: u64,
    pub requesting_grantor: Address,
    pub all_grantors: Vec<Address>,
    pub grantor_shares: Map<Address, u32>,
    pub cancellation_amount: i128,
    pub reason: String,
    pub created_at: u64,
    pub challenge_deadline: u64,
    pub status: PartialCancellationStatus,
    pub approvals: Map<Address, bool>,
    pub executed_at: Option<u64>,
    pub rejection_reasons: Map<Address, String>,
}
```

### New Constants

```rust
// #415: Authorized Grantee Change constants
const GRANTEE_CHANGE_AUTHORIZATION_PERIOD: u64 = 30 * 24 * 60 * 60; // 30 days
const MAX_GRANTEE_CHANGE_REASON_LENGTH: u32 = 500;
const GRANTEE_CHANGE_COOLDOWN: u64 = 7 * 24 * 60 * 60; // 7 days

// #416: Emergency Grace Period constants
const EMERGENCY_GRACE_PERIOD: u64 = 3 * 24 * 60 * 60; // 3 days
const MAX_EMERGENCY_REASON_LENGTH: u32 = 1000;
const EMERGENCY_RESUMPTION_FEE: i128 = 100_000_000; // 10 XLM

// #414: Staged Approval Workflow constants
const STAGED_APPROVAL_TIMEOUT: u64 = 14 * 24 * 60 * 60; // 14 days
const MAX_REVIEWER_REASON_LENGTH: u32 = 800;

// #408: Partial Funding Cancellation constants
const MIN_GRANTOR_SHARE_PERCENTAGE: u32 = 1000; // 10%
const MAX_GRANTORS_FOR_PARTIAL_CANCELLATION: u32 = 10;
const PARTIAL_CANCELLATION_CHALLENGE_PERIOD: u64 = 5 * 24 * 60 * 60; // 5 days
```

### Storage Keys

New storage keys have been added for each feature:

```rust
// #415: Authorized Grantee Change keys
GranteeChangeRequest(u64),
GranteeChangeRequests(u64),
NextGranteeChangeRequestId,
GranteeChangeIds,

// #416: Emergency Grace Period keys
EmergencyResumptionRequest(u64),
EmergencyResumptionRequests(u64),
NextEmergencyResumptionRequestId,
EmergencyResumptionIds,

// #414: Staged Approval Workflow keys
StagedApproval(u64),
StagedApprovals(u64),
NextStagedApprovalId,
StagedApprovalIds,

// #408: Partial Funding Cancellation keys
PartialCancellationRequest(u64),
PartialCancellationRequests(u64),
NextPartialCancellationRequestId,
PartialCancellationIds,
```

### Error Handling

Comprehensive error types have been added for each feature:

- `GranteeChangeRequestNotFound`, `GranteeChangeCooldownActive`, `GranteeChangeAuthorizationExpired`
- `EmergencyGracePeriodExpired`, `EmergencyResumptionFeeNotPaid`, `GrantNotCancelled`
- `StagedApprovalTimeout`, `StagedApprovalSequenceError`, `ReviewerApprovalRequired`
- `PartialCancellationChallengeActive`, `InsufficientGrantorShare`, `TooManyGrantors`

---

## Usage Examples

### Grantee Change Example
```rust
// Propose change (by current grantee or admin)
let request_id = GrantContract::propose_grantee_change(
    env,
    grant_id,
    new_grantee_address,
    "Team migration: original developer left project".to_string(),
)?;

// Authorize change (admin only)
GrantContract::authorize_grantee_change(
    env,
    request_id,
    true, // authorized
    None, // no rejection reason
)?;

// Execute change
GrantContract::execute_grantee_change(env, request_id)?;
```

### Emergency Resumption Example
```rust
// Request emergency resumption
let request_id = GrantContract::request_emergency_resumption(
    env,
    grant_id,
    "Critical bug fixed, need to resume development".to_string(),
)?;

// Pay recovery fee
GrantContract::pay_emergency_resumption_fee(env, request_id)?;

// Admin approval
GrantContract::approve_emergency_resumption(
    env,
    request_id,
    true, // approved
    None, // no rejection reason
)?;
```

### Staged Approval Example
```rust
// Create staged approval
let approval_id = GrantContract::create_staged_approval(
    env,
    grant_id,
    milestone_claim_id,
    reviewer_address,
    admin_address,
)?;

// Reviewer approval
GrantContract::reviewer_approve(
    env,
    approval_id,
    true, // approved
    Some("Milestone completed successfully".to_string()),
)?;

// Admin final approval
GrantContract::admin_approve(
    env,
    approval_id,
    true, // approved
    Some("Final admin confirmation".to_string()),
)?;
```

### Partial Cancellation Example
```rust
// Propose partial cancellation
let request_id = GrantContract::propose_partial_cancellation(
    env,
    grant_id,
    500000, // cancel half the funding
    "Budget constraints require reduction".to_string(),
)?;

// Grantor approval (each grantor calls this)
GrantContract::approve_partial_cancellation(
    env,
    request_id,
    true, // approved
    Some("Agree to partial reduction".to_string()),
)?;
```

---

## Testing

Comprehensive tests have been implemented in `test_security_milestone_features.rs` covering:

- Normal workflow scenarios for each feature
- Error conditions and edge cases
- Authorization and permission checks
- Time-based behaviors (timeouts, grace periods)
- State transitions and validation

### Running Tests
```bash
cargo test --package grant_contracts test_security_milestone_features
```

---

## Security Considerations

1. **Authorization**: All sensitive operations require proper authorization
2. **Time Constraints**: Requests have expiration dates to prevent stale issues
3. **Economic Disincentives**: Fees prevent abuse of emergency mechanisms
4. **Audit Trails**: Complete history maintained for all operations
5. **Validation**: Comprehensive input validation and state checking
6. **Access Control**: Role-based permissions enforced throughout

---

## Future Enhancements

1. **Multi-Sig Support**: Extend to multi-signature requirements for critical operations
2. **Delegation**: Allow delegation of approval authority
3. **Batch Operations**: Support for batch processing of multiple requests
4. **Notifications**: Event-based notification system for status changes
5. **Integration**: Deeper integration with external governance systems

---

## Conclusion

These four features significantly enhance the Grant Stream Contracts system by:

- Providing secure mechanisms for team changes and emergencies
- Implementing robust approval workflows for milestone claims
- Supporting complex multi-grantor scenarios
- Maintaining comprehensive audit trails and security controls

The implementation follows best practices for smart contract security, including proper authorization, time-based constraints, economic incentives, and comprehensive error handling.
