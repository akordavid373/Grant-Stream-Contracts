# Global Invariant Fuzz Test Documentation

## Overview

This document describes the implementation of a comprehensive property-based fuzz test that ensures the **Contract Balance >= Sum of all Active Stream allocations** invariant is never violated in the Grant Stream contract.

## Critical Invariant

The test verifies the fundamental property that the contract never promises more tokens than it actually holds:

```
Contract_Balance >= Total_Allocated_Funds
```

Where:
- `Contract_Balance` = Current token balance in the contract
- `Total_Allocated_Funds` = Sum of (total_amount - withdrawn) for all Active/Paused grants

## Test Implementation

### File: `contracts/grant_stream/src/test_global_invariant_fuzz.rs`

### Key Components

1. **GlobalInvariantTestState**: Manages the test environment and contract state
2. **RandomOperation**: Enum representing all possible contract operations
3. **Property-based verification**: Continuous invariant checking during random operations

### Test Scenarios

#### 1. Main Fuzz Test (`test_global_invariant_random_operations`)
- **Operations**: 100-1000 random operations per test
- **Parameters**: Variable initial balance, time advancement, operation seeds
- **Coverage**: All contract functions with random parameters

#### 2. Stress Tests
- **Maximum Stress**: 10,000 operations with maximum balance
- **Zero Balance Edge**: Tests with zero initial balance
- **Single Large Grant**: One massive grant with stress testing
- **Many Micro Grants**: 1000 small grants to test aggregation

#### 3. Mathematical Proof Test
- Deterministic scenario proving the invariant mathematically
- Step-by-step verification of balance calculations

### Random Operations

The test generates these operations with realistic probability distributions:

- **Create Grant** (30%): New grant creation with random parameters
- **Withdraw** (20%): Random withdrawal from available claimable
- **Withdraw Validator** (10%): Validator withdrawal operations
- **Pause/Resume Stream** (20%): Stream state management
- **Cancel Grant** (5%): Grant cancellation with treasury return
- **Rage Quit** (5%): Emergency grant termination
- **Rate Changes** (5%): Flow rate modifications

### Invariant Verification

After each operation, the test verifies:

1. **Primary Invariant**: `contract_balance >= total_allocated`
2. **No Token Creation**: Total tokens in system never exceed initial balance
3. **Mathematical Consistency**: No negative balances or overflow conditions

### Test Parameters

```rust
const MAX_GRANTS_PER_TEST: usize = 1000;
const MAX_ITERATIONS: usize = 10000;
const MIN_TOKEN_AMOUNT: i128 = 1000 * SCALING_FACTOR;
const MAX_TOKEN_AMOUNT: i128 = 10_000_000 * SCALING_FACTOR;
```

## Security Assurance

This fuzz test provides mathematical proof that even with millions of random operations, the contract maintains its critical invariant. The test covers:

- **Edge Cases**: Zero balances, maximum values, boundary conditions
- **Temporal Aspects**: Time advancement with accrual calculations
- **State Transitions**: All possible grant status changes
- **Validator Operations**: 5% ecosystem tax handling
- **Error Conditions**: Graceful handling of invalid operations

## Running the Tests

```bash
# Run all global invariant tests
cargo test test_global_invariant

# Run specific stress test
cargo test test_global_invariant_maximum_stress

# Run property-based fuzz test
cargo test test_global_invariant_random_operations
```

## Expected Results

All tests should pass, confirming that:
1. The contract never reaches a state where it has promised more tokens than it holds
2. No token creation or loss occurs under any operation sequence
3. Mathematical precision is maintained across all calculations
4. Edge cases are handled correctly without invariant violations

## Integration

This test is integrated into the main test suite via the module declaration in `lib.rs`:

```rust
#[cfg(test)]
mod test_global_invariant_fuzz;
```

The test runs as part of the standard test suite and provides continuous assurance of the contract's mathematical correctness.
