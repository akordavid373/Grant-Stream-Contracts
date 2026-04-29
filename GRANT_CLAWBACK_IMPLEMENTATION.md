# Grant Clawback Implementation

## Overview

This implementation adds a comprehensive grant clawback mechanism to the Grant Stream Contracts, allowing donors or DAO multi-sig to recover unearned funds when grantees fail to meet milestones or violate grant terms.

## Key Features

### 1. **Access Control**
- **Donor Authorization**: Original donor who funded the grant can trigger clawback
- **DAO Multi-sig**: If no donor is specified, admin (DAO multi-sig) can trigger clawback
- **Authorization Verification**: Uses `require_donor_or_multisig_auth()` helper function

### 2. **Precise Proration Math**
- **Millisecond Precision**: Calculates vested amount up to the exact second of clawback
- **Unearned Balance Formula**: `Total_Grant - Amount_Already_Streamed_To_Date`
- **Checkpoint System**: Prevents double-spending during clawback execution

### 3. **Dispute Escrow System**
- **Contested Clawbacks**: Funds can be moved to dispute escrow instead of donor's wallet
- **Resolution Mechanism**: Admin can resolve disputes by releasing funds to appropriate party
- **Escrow Tracking**: Separate storage for disputed funds

### 4. **Event Emission**
- **GrantClawbackExecuted Event**: Emitted with amount, reason, and contest status
- **Transparency**: All clawback actions are publicly logged

## Core Functions

### `trigger_grant_clawback(env, grant_id, reason, contested)`

**Purpose**: Main entry point for clawback execution

**Parameters**:
- `grant_id`: ID of the grant to clawback
- `reason`: Human-readable reason for clawback
- `contested`: Whether the clawback is disputed (moves to escrow if true)

**Process**:
1. Validates grant state (must be Active or Paused)
2. Checks authorization (donor or admin)
3. Sets checkpoint to prevent double-spending
4. Settles grant up to exact millisecond of clawback
5. Calculates unearned balance
6. Transfers funds based on contest status
7. Updates grant status to Clawbacked
8. Emits event

### `resolve_disputed_clawback(env, grant_id, release_to_donor)`

**Purpose**: Resolves disputed clawbacks by releasing escrowed funds

**Parameters**:
- `grant_id`: ID of the disputed grant
- `release_to_donor`: true = release to donor, false = resume grant

**Process**:
1. Validates clawbacked status
2. Retrieves escrow amount
3. Transfers funds to appropriate party
4. Clears escrow storage
5. Emits resolution event

### `get_dispute_escrow_balance(env, grant_id)`

**Purpose**: Returns current escrow balance for a disputed grant

## Security Features

### 1. **Double-Spending Prevention**
- **Checkpoint System**: Sets timestamp checkpoint before clawback execution
- **State Validation**: Checks if clawback already executed
- **Atomic Operations**: All state changes happen in single transaction

### 2. **Access Control**
- **Donor Verification**: Only original donor can trigger clawback
- **Admin Fallback**: DAO multi-sig can act when no donor specified
- **Authorization Checks**: Uses Stellar's built-in auth system

### 3. **Mathematical Precision**
- **Overflow Protection**: All math operations check for overflow
- **Basis Points**: Uses SCALING_FACTOR for precise calculations
- **Temporal Accuracy**: Settles up to exact millisecond of execution

## Data Structures

### New Grant Fields
```rust
pub donor: Option<Address>,              // Original donor for authorization
pub clawback_checkpoint: Option<u64>,   // Prevents double-spending
```

### New Storage Keys
```rust
GrantDonor(u64),           // Donor information
ClawbackCheckpoint(u64),  // Checkpoint timestamp
DisputeEscrow(u64),       // Escrow balance
```

### New Grant Status
```rust
Clawbacked,  // Grant has been clawed back
```

## Acceptance Criteria

### ✅ Acceptance 1: Donor Capital Recovery
- Donors can recover unearned capital if project is abandoned
- Works for both legal violations and milestone failures
- Returns funds to donor's vault instantly

### ✅ Acceptance 2: Grantee Payment Protection
- Grantees are mathematically guaranteed payment for work done
- Proration calculated to millisecond precision
- Vested funds remain claimable after clawback

### ✅ Acceptance 3: Balance Invariant Maintenance
- Perfect balance invariant between treasury, donor, and grantee
- No funds can be lost or double-spent
- All transfers are atomic and verifiable

## Testing

### Comprehensive Test Suite
1. **Basic Functionality**: Verify end-to-end clawback process
2. **Dispute Escrow**: Test contested clawback flow
3. **Proration Math**: Validate millisecond precision
4. **Access Control**: Test authorization mechanisms
5. **Double-Spending Prevention**: Verify checkpoint system
6. **Balance Invariants**: Ensure no funds lost
7. **Validator Support**: Test with validator rewards
8. **Edge Cases**: Immediate clawback, post-completion clawback

### Test Coverage
- **Happy Paths**: Normal clawback scenarios
- **Error Cases**: Invalid states, unauthorized access
- **Edge Conditions**: Zero amounts, completed grants
- **Security**: Double-spending attempts, overflow checks

## Integration Points

### Existing Functions Modified
- `create_grant()`: Added donor parameter
- `Grant` struct: Added donor and checkpoint fields
- Error enum: Added clawback-specific errors

### New Dependencies
- Storage keys for clawback data
- Event emissions for transparency
- Helper functions for calculations

## Usage Examples

### Basic Clawback (Donor)
```rust
// Donor triggers clawback for abandoned project
let reason = String::from_str(&env, "Project abandoned - no progress for 90 days");
contract.trigger_grant_clawback(&grant_id, &reason, &false);
```

### Contested Clawback
```rust
// Donor triggers contested clawback
let reason = String::from_str(&env, "Milestone disputes - quality issues");
contract.trigger_grant_clawback(&grant_id, &reason, &true);

// Later, DAO resolves in favor of donor
contract.resolve_disputed_clawback(&grant_id, &true);
```

### DAO Multi-sig Clawback
```rust
// Admin triggers clawback when no donor specified
let reason = String::from_str(&env, "Legal compliance violation");
contract.trigger_grant_clawback(&grant_id, &reason, &false);
```

## Event Schema

### GrantClawbackExecuted
```
Topics: [symbol_short!("clawback"), recipient, admin, token_address, grant_id]
Data: [amount, reason, contested]
```

### ClawbackResolved
```
Topics: [symbol_short!("claw_resolve"), grant_id, admin]
Data: [amount, release_to_donor]
```

## Gas Optimization

### Efficient Storage
- Minimal additional storage per grant
- Reuses existing storage patterns
- Cleanup of escrow after resolution

### Optimized Math
- Basis point calculations for precision
- Overflow checks with early returns
- Efficient settlement calculations

## Future Enhancements

### Potential Extensions
1. **Partial Clawbacks**: Clawback specific percentage
2. **Time-Locked Escrow**: Time-based dispute resolution
3. **Multi-Donor Support**: Grants with multiple donors
4. **Automated Milestone Clawbacks**: Trigger based on milestone failures

### Upgrade Path
- Storage migration support included
- Backward compatibility maintained
- Feature flags for gradual rollout

## Security Considerations

### Threat Mitigations
1. **Reentrancy**: Checkpoint system prevents reentrancy attacks
2. **Front-Running**: Authorization checks prevent front-running
3. **Overflow**: All math operations include overflow checks
4. **State Corruption**: Atomic updates prevent inconsistent state

### Audit Recommendations
1. Verify all authorization paths
2. Test edge cases with maximum values
3. Validate temporal precision under load
4. Check gas limits for large grants

## Conclusion

This implementation provides a robust, secure, and transparent grant clawback mechanism that meets all acceptance criteria while maintaining the protocol's security guarantees and mathematical precision.
