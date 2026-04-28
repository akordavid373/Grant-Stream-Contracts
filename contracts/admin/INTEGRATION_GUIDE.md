# Governance Activity Monitor Integration Guide

## Overview

The Governance Activity Monitor is a circuit breaker that protects against rapid protocol parameter changes. It automatically enforces a 7-day mandatory timelock when an admin attempts to change more than 3 protocol parameters in a single ledger.

## Key Features

- **Circuit Breaker**: Triggers when >3 parameter changes occur in one ledger
- **Mandatory Timelock**: 7-day timelock when circuit breaker triggers
- **Parameter Tracking**: Monitors fees, thresholds, addresses, and other parameters
- **Configurable Limits**: Admins can adjust thresholds and timelock durations
- **Audit Trail**: Complete history of all parameter changes

## Integration Steps

### 1. Initialize the Monitor

```rust
use crate::admin::governance_activity_monitor::GovernanceActivityMonitor;

// Initialize with admin address
GovernanceActivityMonitor::initialize(env, admin_address)?;
```

### 2. Wrap Admin Functions

For each admin function that changes protocol parameters, wrap it with the monitor:

```rust
// Original function
pub fn update_protocol_fee(env: Env, admin: Address, new_fee: i128) -> Result<(), Error> {
    admin.require_auth();
    // ... existing logic
}

// Enhanced with monitoring
pub fn update_protocol_fee(env: Env, admin: Address, new_fee: i128) -> Result<(), Error> {
    admin.require_auth();
    
    // Get current fee
    let current_fee = get_current_fee(&env)?;
    
    // Record the change attempt
    let change_id = GovernanceActivityMonitor::record_parameter_change(
        env.clone(),
        admin.clone(),
        ParameterType::Fee,
        String::from_str(&env, "protocol_fee"),
        Bytes::from_slice(&env, &current_fee.to_le_bytes()),
        Bytes::from_slice(&env, &new_fee.to_le_bytes()),
        String::from_str(&env, "Update protocol fee for sustainability"),
    )?;
    
    // Check if change is immediately executable (no timelock)
    if change_id == 0 {
        // Monitor disabled, proceed with change
        set_current_fee(&env, new_fee);
        return Ok(());
    }
    
    // Get the change details to check timelock
    let change = GovernanceActivityMonitor::get_parameter_change(env.clone(), change_id)?;
    
    if env.ledger().timestamp() >= change.executable_at {
        // Timelock expired, execute the change
        GovernanceActivityMonitor::execute_parameter_change(env.clone(), admin.clone(), change_id)?;
        set_current_fee(&env, new_fee);
        Ok(())
    } else {
        // Timelock still active, change is queued
        Err(Error::TimelockActive)
    }
}
```

### 3. Parameter Types

Use the appropriate `ParameterType` for different changes:

```rust
use crate::admin::governance_activity_monitor::ParameterType;

match parameter_name {
    "protocol_fee" | "withdrawal_fee" => ParameterType::Fee,
    "quorum_threshold" | "voting_threshold" => ParameterType::Threshold,
    "treasury_address" | "oracle_address" => ParameterType::Address,
    "timelock_duration" => ParameterType::Timelock,
    _ => ParameterType::Other,
}
```

### 4. Batch Operations

For operations that change multiple parameters at once:

```rust
pub fn batch_update_parameters(env: Env, admin: Address, params: Vec<ParameterUpdate>) -> Result<(), Error> {
    admin.require_auth();
    
    let mut change_ids = Vec::new(&env);
    
    // Record all parameter changes
    for update in params.iter() {
        let change_id = GovernanceActivityMonitor::record_parameter_change(
            env.clone(),
            admin.clone(),
            update.parameter_type,
            update.name.clone(),
            update.old_value.clone(),
            update.new_value.clone(),
            update.reason.clone(),
        )?;
        change_ids.push_back(change_id);
    }
    
    // Check if any changes triggered circuit breaker
    let mut has_mandatory_timelock = false;
    for change_id in change_ids.iter() {
        if let Ok(change) = GovernanceActivityMonitor::get_parameter_change(env.clone(), *change_id) {
            if change.status == ChangeStatus::MandatoryTimelock {
                has_mandatory_timelock = true;
                break;
            }
        }
    }
    
    if has_mandatory_timelock {
        return Err(Error::MandatoryTimelockTriggered);
    }
    
    // Execute all changes
    for (i, update) in params.iter().enumerate() {
        let change_id = change_ids.get(i).unwrap();
        let change = GovernanceActivityMonitor::get_parameter_change(env.clone(), *change_id)?;
        
        if env.ledger().timestamp() >= change.executable_at {
            GovernanceActivityMonitor::execute_parameter_change(env.clone(), admin.clone(), *change_id)?;
            // Apply the actual parameter change
            apply_parameter_update(&env, update)?;
        } else {
            return Err(Error::TimelockActive);
        }
    }
    
    Ok(())
}
```

## Configuration

### Default Settings
- **Max changes per ledger**: 3
- **Mandatory timelock**: 7 days (604,800 seconds)
- **Standard timelock**: 24 hours

### Updating Configuration

```rust
// Update limits (admin only)
GovernanceActivityMonitor::update_config(
    env,
    admin,
    Some(5), // Allow 5 changes per ledger
    Some(10 * 24 * 60 * 60), // 10-day mandatory timelock
)?;
```

### Emergency Disable

```rust
// Disable monitoring in emergency (admin only)
GovernanceActivityMonitor::set_enabled(env, admin, false)?;
```

## Monitoring and Oversight

### View Pending Changes

```rust
// Get all pending changes for an admin
let pending = GovernanceActivityMonitor::get_pending_changes(env, admin_address)?;

// Get current ledger activity
let activity = GovernanceActivityMonitor::get_current_ledger_activity(env)?;

// Check if circuit breaker is active
if activity.change_count > 3 {
    // Circuit breaker triggered
}
```

### Event Monitoring

The monitor emits events for all activities:

- `monitor_init`: Monitor initialized
- `param_change`: Normal parameter change recorded
- `breaker_trig`: Circuit breaker triggered
- `breaker_warn`: Warning when breaker triggers
- `param_exec`: Parameter change executed
- `param_cancel`: Parameter change cancelled
- `monitor_toggle`: Monitor enabled/disabled
- `config_update`: Configuration updated

## Best Practices

### 1. Early Validation

Check for pending changes before attempting new ones:

```rust
let pending = GovernanceActivityMonitor::get_pending_changes(env, admin)?;
if !pending.is_empty() {
    return Err(Error::PendingChangesExist);
}
```

### 2. Clear Reasoning

Always provide clear reasons for parameter changes:

```rust
let reason = String::from_str(&env, "Adjust fee to cover increased operational costs");
```

### 3. Regular Monitoring

Regularly check the monitor status:

```rust
let enabled = GovernanceActivityMonitor::is_enabled(&env);
let activity = GovernanceActivityMonitor::get_current_ledger_activity(env)?;
```

### 4. Emergency Procedures

Have clear procedures for emergency parameter changes:

1. Disable monitor if absolutely necessary
2. Document the emergency reason
3. Re-enable monitor as soon as possible
4. Review all emergency changes post-incident

## Integration with Existing Components

### Dead Man's Switch Integration

The Governance Activity Monitor works alongside the Dead Man's Switch:

```rust
// In admin functions, call both
use crate::admin::dead_mans_switch::DeadMansSwitchContract;
use crate::admin::governance_activity_monitor::GovernanceActivityMonitor;

pub fn admin_operation(env: Env, admin: Address) -> Result<(), Error> {
    admin.require_auth();
    
    // Record activity for dead man's switch
    DeadMansSwitchContract::record_activity(&env);
    
    // Record parameter change if applicable
    // ... monitor integration code
    
    Ok(())
}
```

### Governance Integration

Integrate with governance proposals:

```rust
// In governance execution
pub fn execute_governance_proposal(env: Env, proposal_id: u64) -> Result<(), Error> {
    let proposal = get_proposal(&env, proposal_id)?;
    
    // Record as governance-initiated change
    let change_id = GovernanceActivityMonitor::record_parameter_change(
        env,
        proposal.proposer,
        ParameterType::Other,
        proposal.title,
        proposal.old_state,
        proposal.new_state,
        proposal.description,
    )?;
    
    // Execute if timelock allows
    // ... execution logic
    
    Ok(())
}
```

## Testing

The monitor includes comprehensive tests. Key test scenarios:

1. **Normal Operations**: Under limit changes work normally
2. **Circuit Breaker**: 4th change triggers 7-day timelock
3. **Timelock Enforcement**: Changes blocked until timelock expires
4. **Configuration**: Custom limits and timelocks work
5. **Authorization**: Only admins can execute changes
6. **Emergency Disable**: Monitor can be disabled in emergencies

Run tests with:

```bash
cargo test --package admin --lib governance_activity_monitor_test
```

## Security Considerations

1. **Admin Privileges**: The monitor relies on proper admin authentication
2. **State Consistency**: Ensure parameter changes are atomic with monitor updates
3. **Event Logging**: Monitor all events for security auditing
4. **Configuration Limits**: Set reasonable limits for your protocol
5. **Emergency Procedures**: Document when and how to bypass the monitor

## Troubleshooting

### Common Issues

1. **Change Not Recorded**: Check if monitor is enabled
2. **Timelock Not Working**: Verify ledger timestamp advancement
3. **Circuit Breaker Not Triggering**: Check parameter counting logic
4. **Authorization Errors**: Verify admin address is correctly set

### Debug Information

```rust
// Debug current state
let enabled = GovernanceActivityMonitor::is_enabled(&env);
let admin = GovernanceActivityMonitor::get_admin(&env)?;
let activity = GovernanceActivityMonitor::get_current_ledger_activity(env)?;
let all_changes = GovernanceActivityMonitor::get_all_changes(env)?;
```

This integration guide provides everything needed to successfully integrate the Governance Activity Monitor into your protocol's admin functions.
