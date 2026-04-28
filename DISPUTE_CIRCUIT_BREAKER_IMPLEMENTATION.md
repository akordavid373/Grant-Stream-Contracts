# Mass Milestone Dispute Trigger Circuit Breaker

## Overview

This implementation addresses the "Circuit Breaker: Mass Milestone Dispute Trigger" issue to protect the DAO treasury from Sybil-Dispute attacks designed to overwhelm the arbitration panel.

## Problem Statement

If more than 15% of active grants are placed into "Dispute" status within 24 hours, the protocol should halt new grant initializations to prevent coordinated attacks on the arbitration system.

## Implementation Details

### Core Components

#### 1. Circuit Breaker Storage Keys

Added new storage keys in `circuit_breakers.rs`:
- `DisputeWindowStart`: Timestamp when the current 24-hour monitoring window started
- `DisputeAccumulator`: Number of disputes in the current window
- `ActiveGrantsSnapshot`: Number of active grants at window start
- `GrantInitializationHalted`: Flag indicating if new grant creation is halted

#### 2. Constants

- `DISPUTE_WINDOW_SECS`: 24 hours (86,400 seconds)
- `DISPUTE_THRESHOLD_BPS`: 15% (1,500 basis points)

#### 3. Core Functions

##### `record_dispute(env, active_grants_count) -> bool`
- Records a new dispute in the monitoring window
- Automatically resets the window after 24 hours
- Calculates dispute percentage relative to active grants
- Returns `false` if the 15% threshold is breached (trips the circuit breaker)
- Returns `true` if the dispute is recorded normally

##### `is_grant_initialization_halted(env) -> bool`
- Returns whether the circuit breaker is currently active
- Used to block new grant creation

##### `resume_grant_initialization(env, admin)`
- Admin-only function to resume grant initialization
- Resets all monitoring counters
- Should only be called after manual investigation

##### `get_dispute_monitoring_stats(env) -> (u64, u32, u32, bool)`
- Returns current monitoring statistics for transparency
- Tuple: (window_start, dispute_count, active_grants_snapshot, halted)

### Integration Points

#### 1. Grant Creation (`create_grant`)
Added check at the beginning of `create_grant()`:
```rust
if circuit_breakers::is_grant_initialization_halted(&env) {
    return Err(Error::GrantInitializationHalted);
}
```

#### 2. Dispute Trigger (`trigger_grant_dispute`)
New function to be called when a grant enters dispute status:
- Verifies the grant exists
- Counts current active grants
- Records the dispute and checks threshold
- Emits events for transparency

#### 3. Error Handling
Added new error type: `GrantInitializationHalted = 19`

## Usage Flow

### Normal Operation
1. Grant is created normally
2. Grant enters dispute status through arbitration process
3. `trigger_grant_dispute()` is called
4. Dispute is recorded, threshold checked
5. If below 15%, operation continues normally

### Attack Scenario (Sybil-Dispute Attack)
1. Multiple grants rapidly enter dispute status
2. When disputes exceed 15% of active grants in 24 hours:
   - Circuit breaker trips
   - `GrantInitializationHalted` flag is set
   - New grant creation is blocked
   - Event is emitted for transparency

### Recovery
1. Admin investigates the dispute pattern
2. If determined to be a false alarm or resolved:
   - Admin calls `resume_grant_initialization()`
   - Monitoring window is reset
   - Grant creation resumes

## Security Features

### 1. Automatic Window Reset
- Monitoring window automatically resets after 24 hours
- Prevents permanent lockout

### 2. Admin Oversight
- Only admin can resume operations
- Requires manual investigation
- Prevents automatic recovery from genuine attacks

### 3. Transparency
- All actions emit events
- Statistics are publicly readable
- Enables community monitoring

### 4. Graceful Degradation
- Existing grants continue to function
- Only new grant creation is halted
- Minimizes disruption to legitimate users

## Testing

Created comprehensive test suite in `test_dispute_circuit_breaker.rs`:

1. **Basic Functionality Test**
   - Creates grants, triggers disputes
   - Verifies threshold detection
   - Tests grant creation blocking

2. **Interface Integration Test**
   - Tests through main contract interface
   - Verifies statistics reporting

3. **Window Reset Test**
   - Tests automatic window reset after 24 hours
   - Verifies counter reset functionality

## Integration with Arbitration Contract

The dispute monitoring should be integrated with the existing arbitration contract:

```rust
// In arbitration contract when dispute is created
pub fn raise_dispute(env: Env, grant_id: u32, ...) -> u32 {
    // ... existing dispute logic ...
    
    // Trigger monitoring in grant stream contract
    grant_stream_contract.trigger_grant_dispute(grant_id as u64);
    
    // ... rest of dispute logic ...
}
```

## Configuration

The threshold and window duration can be adjusted by modifying the constants:

```rust
const DISPUTE_WINDOW_SECS: u64 = 24 * 60 * 60; // 24 hours
const DISPUTE_THRESHOLD_BPS: i128 = 1_500; // 15%
```

## Monitoring and Alerting

Contract events provide real-time monitoring:
- `dispute_cb`: Emitted when threshold is breached
- `resume_grants`: Emitted when admin resumes operations

Off-chain systems should monitor these events for alerting and governance response.

## Future Enhancements

1. **Dynamic Thresholds**: Allow DAO to adjust thresholds via governance
2. **Graduated Response**: Implement graduated restrictions based on dispute percentage
3. **Whitelist Mechanism**: Allow certain trusted creators to bypass restrictions
4. **Historical Analysis**: Store dispute history for pattern analysis

## Conclusion

This implementation provides robust protection against Sybil-Dispute attacks while maintaining system transparency and allowing for legitimate dispute resolution. The circuit breaker approach ensures rapid response to potential attacks while minimizing disruption to normal operations.
