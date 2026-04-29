# Running Point One Cent Exploit Prevention Tests

## Quick Start

```bash
# From the Grant-Stream-Contracts/contracts/grant_stream directory
cargo test test_point_one_cent_exploit -- --nocapture
```

## Individual Tests

### 1. Main Exploit Simulation (1,000 withdrawals)
```bash
cargo test test_rounding_never_drains_contract -- --nocapture
```

**What it tests:** 1,000 partial withdrawals across 5 beneficiaries with awkward share allocations

**Expected output:**
```
Initial balance: 1000000007
Initial obligations: 1000000007
...
✓ All 1,000 withdrawals completed without invariant violation
✓ Rounding always favors the contract
✓ No Point One Cent exploit possible
```

### 2. Single Unit Edge Case
```bash
cargo test test_single_unit_allocation -- --nocapture
```

**What it tests:** Contract with exactly 1 base unit

### 3. Remainder Retention
```bash
cargo test test_max_shares_with_remainder -- --nocapture
```

**What it tests:** 10 units split among 3 beneficiaries (1 unit remainder)

### 4. Repeated Withdrawals
```bash
cargo test test_repeated_same_beneficiary_withdrawals -- --nocapture
```

**What it tests:** Same beneficiary withdraws 1,000 times

### 5. Zero Remainder
```bash
cargo test test_zero_remainder_case -- --nocapture
```

**What it tests:** Cleanly divisible amounts (no rounding needed)

## Run All Tests

```bash
cargo test test_point_one_cent_exploit
```

## Expected Results

All tests should pass with output showing:
- ✓ Invariant holds after every withdrawal
- ✓ No deficit occurs
- ✓ Dust accumulation is bounded
- ✓ Contract balance always >= obligations

## Troubleshooting

### Test Fails with "EXPLOIT DETECTED"

If you see:
```
EXPLOIT DETECTED at withdrawal X: balance=Y < obligations=Z, deficit=W
```

This means:
1. The division logic is rounding UP instead of DOWN
2. The contract is being drained beyond its obligations
3. **CRITICAL SECURITY ISSUE** - do not deploy

### Test Fails with "Dust accumulation unbounded"

If dust grows beyond `num_beneficiaries * 100`:
1. Rounding errors are accumulating incorrectly
2. Check the division logic in `apply_accrued_split()`

### Test Passes

If all tests pass:
- ✅ Division logic rounds down correctly
- ✅ Contract retains fractional remainders
- ✅ No Point One Cent exploit possible
- ✅ Safe to deploy

## Performance

- Main test (1,000 iterations): ~5-10 seconds
- All 5 tests combined: ~15-20 seconds
- Tests are deterministic and reproducible

## Integration with CI/CD

Add to your CI pipeline:

```yaml
- name: Run Point One Cent Exploit Tests
  run: |
    cd contracts/grant_stream
    cargo test test_point_one_cent_exploit -- --nocapture
```

## What These Tests Prove

1. **Rounding Direction:** Division always rounds down (truncates)
2. **Invariant Maintenance:** `contract_balance >= obligations` after every withdrawal
3. **No Cumulative Drain:** 1,000 withdrawals don't accumulate rounding errors
4. **Edge Case Safety:** Even extreme cases (1 unit, prime numbers) are safe
5. **Bounded Dust:** Rounding remainders stay in contract, don't grow unbounded

## What These Tests Don't Cover

- Reentrancy attacks (see `test_reentrancy_fuzz.rs`)
- Overflow/underflow (see `test_security_invariants.rs`)
- Temporal attacks (see `test_temporal_fuzz.rs`)
- Global invariants (see `test_global_invariant_fuzz.rs`)

This test suite focuses specifically on **division rounding behavior** and the Point One Cent exploit.
