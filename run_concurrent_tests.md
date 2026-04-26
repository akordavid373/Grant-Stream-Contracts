# Concurrent Withdrawal Fuzz Test - Bank Run Scenario

## Overview
This fuzz test simulates a "Bank Run" scenario where 100+ unique addresses attempt to call `withdraw()` in the same ledger sequence. The test verifies that the contract correctly sequences these transactions without hitting storage limits or causing state corruption.

## Test Implementation
The test is implemented in `contracts/grant_stream/src/test_concurrent_withdraw.rs` and includes:

### Key Features:
1. **150 Concurrent Users**: More than the required 100 users
2. **Property-Based Testing**: Uses `proptest` for randomized scenarios
3. **State Validation**: Comprehensive invariants checking
4. **Gas Consumption Analysis**: Ensures later withdrawers aren't blocked
5. **Storage Limit Testing**: Verifies no storage corruption

### Test Components:

#### 1. Property-Based Bank Run Test
```rust
proptest! {
    #[test]
    fn test_concurrent_withdraw_bank_run(
        withdraw_scenarios in prop::collection::vec(
            (user_id, withdraw_amount, timestamp),
            50..200 // number of operations
        )
    )
}
```

#### 2. Maximum Concurrent Stress Test
- Tests with 200 users simultaneously
- Verifies most withdrawals succeed
- Ensures system handles high load

#### 3. Gas Consumption Consistency Test
- Measures gas usage across withdrawal sequence
- Ensures no gas bloat for later transactions
- Validates consistent performance

### Invariants Verified:

1. **Total Balance Invariant**: `contract_balance + sum(user_balances) == total_deposited`
2. **Grant State Consistency**: `withdrawn + claimable <= total_amount` for each grant
3. **User Balance Accuracy**: `user_balance == grant.withdrawn`
4. **Gas Consumption Fairness**: Later transactions don't suffer excessive gas costs

### Key Test Scenarios:

1. **Random Withdrawal Timing**: Users withdraw at different timestamps within the test window
2. **Variable Withdrawal Amounts**: Different amounts to test edge cases
3. **Concurrent Access**: Multiple users accessing the contract simultaneously
4. **State Persistence**: Verification that state remains consistent across operations

## Running the Tests

### Prerequisites:
- Rust toolchain installed
- `proptest` dependency added to `Cargo.toml`

### Commands:
```bash
# Run all concurrent withdrawal tests
cargo test test_concurrent_withdraw --features testutils

# Run specific property-based test
cargo test test_concurrent_withdraw_bank_run --features testutils

# Run stress test
cargo test test_maximum_concurrent_withdrawals --features testutils

# Run gas consistency test
cargo test test_gas_consumption_consistency --features testutils
```

## Expected Results:

1. **All Tests Pass**: No state corruption or invariant violations
2. **High Success Rate**: At least 75% of concurrent withdrawals should succeed
3. **Consistent Gas Usage**: Gas consumption shouldn't vary by more than 50% between early and late transactions
4. **No Storage Limits**: Contract should handle 150+ concurrent users without storage issues

## Test Configuration Constants:

```rust
const NUM_USERS: usize = 150; // More than 100 users as specified
const GRANT_AMOUNT: i128 = 1_000_000 * SCALING_FACTOR;
const FLOW_RATE: i128 = 10 * SCALING_FACTOR; // 10 tokens per second
const TEST_DURATION: u64 = 3600; // 1 hour of streaming
```

## Validation Criteria:

✅ **Concurrency Handling**: Contract correctly sequences 150+ concurrent withdrawals
✅ **State Integrity**: No corruption across concurrent operations  
✅ **Gas Fairness**: Later withdrawers not blocked by gas consumption
✅ **Storage Limits**: No storage exhaustion or corruption
✅ **Invariant Preservation**: All mathematical invariants maintained

## Implementation Details:

The test uses a `TestState` struct that manages:
- Environment setup with 150 unique addresses
- Grant creation for each user
- Balance tracking and validation
- State consistency checks

The property-based test generates random withdrawal scenarios and validates that:
- Total funds are preserved across all operations
- Individual grant states remain consistent
- User balances match contract state
- Gas consumption remains reasonable

This comprehensive test ensures the grant stream contract can handle real-world "bank run" scenarios without failing or corrupting state.
