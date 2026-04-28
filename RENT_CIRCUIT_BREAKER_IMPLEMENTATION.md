# Storage Rent Depletion Circuit Breaker Implementation

## Overview

This implementation adds a circuit breaker that monitors the contract's native XLM balance to ensure sufficient funds for storage rent. If the balance falls below a 3-month rent buffer, non-essential functions are disabled to preserve funds for storage maintenance.

## Features

### 1. Rent Buffer Calculation
- **Monthly Rent Estimate**: 1 XLM per month (conservative estimate)
- **Buffer Period**: 3 months
- **Total Buffer**: 3 XLM (30,000,000 stroops)

### 2. Storage Keys Added
```rust
RentPreservationMode,     // Whether contract is in rent preservation mode
LastRentCheckTimestamp,    // Last time rent balance was checked
RentWarningThreshold,       // Balance threshold when warning was triggered
```

### 3. Core Functions

#### `check_rent_balance(env: &Env) -> bool`
- Monitors contract's native XLM balance
- Engages rent preservation mode if balance < 3 XLM
- Returns `true` if healthy, `false` if preservation mode engaged

#### `is_rent_preservation_mode(env: &Env) -> bool`
- Returns whether contract is currently in rent preservation mode

#### `is_function_allowed(env: &Env, is_essential: bool) -> bool`
- Essential functions (like depositing XLM) are always allowed
- Non-essential functions are blocked during rent preservation mode

#### `disable_rent_preservation_mode(env: &Env, admin: &Address)`
- Admin-only function to manually disable preservation mode
- Verifies sufficient balance before disabling

### 4. Integration Points

#### Modified Functions
- `create_grant()`: Blocks grant creation during rent preservation
- `withdraw()`: Checks rent balance after withdrawal operations

#### Added Admin Functions
- `check_rent_balance()`: Manual rent balance check
- `is_rent_preservation_mode()`: Get current preservation status
- `get_current_xlm_balance()`: Get contract's XLM balance
- `get_rent_buffer_threshold()`: Get 3-month buffer amount
- `disable_rent_preservation_mode()`: Manual recovery

### 5. Error Handling
Added new error variant:
```rust
RentPreservationMode = 19,  // Returned when non-essential functions are blocked
```

## Testing

### Test Coverage
The implementation includes comprehensive tests in `test_rent_circuit_breaker.rs`:

1. **Healthy Balance Test**: Verifies normal operation when balance > 3 XLM
2. **Depleted Balance Test**: Triggers rent preservation mode when balance < 3 XLM
3. **Function Blocking Test**: Ensures non-essential functions are blocked
4. **Essential Function Test**: Verifies essential functions still work
5. **Manual Recovery Test**: Tests admin recovery functionality
6. **Withdraw Integration Test**: Ensures rent checks happen after withdrawals
7. **Edge Cases**: Tests exact threshold boundaries

### Running Tests
```bash
# From the grant_stream directory
cargo test test_rent_circuit_breaker

# Or run all tests
cargo test
```

## Usage Examples

### Checking Rent Status
```rust
// Check if contract is in rent preservation mode
let is_preserved = client.is_rent_preservation_mode();

// Get current balance and threshold
let current_balance = client.get_current_xlm_balance();
let threshold = client.get_rent_buffer_threshold();
```

### Admin Recovery
```rust
// After adding sufficient XLM to contract
if client.is_rent_preservation_mode() {
    client.disable_rent_preservation_mode()?;
}
```

### Event Monitoring
The implementation emits events for monitoring:
- `("rent_warning", "low_balance")`: When preservation mode engages
- `("rent_recovery", "mode_disabled")`: When admin disables preservation mode

## Security Considerations

1. **Conservative Estimates**: Uses 1 XLM/month rent estimate (likely higher than actual)
2. **3-Month Buffer**: Provides substantial safety margin
3. **Essential Function Protection**: Critical operations (like adding funds) always work
4. **Admin Controls**: Admin can manually recover after adding funds
5. **Automatic Monitoring**: Rent checks happen automatically on withdrawals

## Integration with Existing Circuit Breakers

This rent depletion circuit breaker works alongside existing circuit breakers:
- **Oracle Price Guard**: Blocks price-dependent operations during oracle issues
- **TVL Velocity Limit**: Prevents rapid fund drainage
- **Rent Depletion Warning**: Preserves funds for storage maintenance

All circuit breakers must be satisfied for operations to proceed.

## Future Enhancements

Potential improvements:
1. **Dynamic Rent Calculation**: Calculate actual rent based on storage usage
2. **Graduated Response**: Different levels of restrictions based on balance
3. **Automatic Recovery**: Automatically disable when sufficient funds are detected
4. **Rent Estimation API**: Provide rent cost estimates to users
5. **Alert System**: External notifications when rent preservation mode engages
