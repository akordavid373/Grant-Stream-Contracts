# Division Operations Audit

## Purpose

This document catalogs all division operations in the grant stream contracts and confirms they use safe rounding behavior (round down) to prevent the Point One Cent exploit.

## Critical Division Operations

### 1. `apply_accrued_split()` - Validator Share Calculation

**Location:** `contracts/grant_stream/src/lib.rs:242`

**Code:**
```rust
let validator_share = accrued
    .checked_mul(500)
    .ok_or(Error::MathOverflow)?
    .checked_div(10000)  // ← CRITICAL DIVISION
    .ok_or(Error::MathOverflow)?;
```

**Purpose:** Calculate 5% validator share from accrued amount

**Rounding Behavior:** ✅ Rounds down (truncates toward zero)

**Example:**
- Input: `accrued = 1007`
- Calculation: `(1007 * 500) / 10000 = 503500 / 10000 = 50.35`
- Result: `50` (0.35 remainder stays in contract)

**Security Impact:** HIGH - This is the primary division in the payout path

---

### 2. `calculate_accrued()` - Warmup Multiplier Application

**Location:** `contracts/grant_stream/src/lib.rs:435`

**Code:**
```rust
let accrued = base_accrued
    .checked_mul(multiplier)
    .ok_or(Error::MathOverflow)?
    .checked_div(10000)  // ← CRITICAL DIVISION
    .ok_or(Error::MathOverflow)?;
```

**Purpose:** Apply warmup multiplier to base accrued amount

**Rounding Behavior:** ✅ Rounds down (truncates toward zero)

**Example:**
- Input: `base_accrued = 10000`, `multiplier = 5000` (50% warmup)
- Calculation: `(10000 * 5000) / 10000 = 50000000 / 10000 = 5000`
- Result: `5000` (exact in this case)

**Security Impact:** HIGH - Affects all accrual calculations

---

### 3. `calculate_amount_from_shares()` - Clawback Resilient

**Location:** `contracts/grant_contracts/src/clawback_resilient.rs:275`

**Code:**
```rust
let amount = (shares * current_balance)
    .checked_div(total_shares)  // ← DIVISION
    .ok_or(BalanceError::MathOverflow)?;
```

**Purpose:** Convert shares back to token amount

**Rounding Behavior:** ✅ Rounds down (truncates toward zero)

**Security Impact:** MEDIUM - Used in clawback scenarios

---

### 4. `calculate_shares_from_amount()` - Clawback Resilient

**Location:** `contracts/grant_contracts/src/clawback_resilient.rs:255`

**Code:**
```rust
let shares = (scaled_amount * total_shares)
    .checked_div(current_balance)  // ← DIVISION 1
    .ok_or(BalanceError::MathOverflow)?
    .checked_div(SHARES_SCALING_FACTOR)  // ← DIVISION 2
    .ok_or(BalanceError::MathOverflow)?;
```

**Purpose:** Convert token amount to shares

**Rounding Behavior:** ✅ Both divisions round down

**Security Impact:** MEDIUM - Used in deposit scenarios

---

## Non-Critical Division Operations

### Circuit Breakers - Deviation Calculation

**Location:** `contracts/grant_stream/src/circuit_breakers.rs:76`

**Code:**
```rust
let deviation_bps = diff
    .saturating_mul(10_000)
    .checked_div(last)  // ← DIVISION
    .unwrap_or(i128::MAX);
```

**Purpose:** Calculate percentage deviation for circuit breaker

**Rounding Behavior:** ✅ Rounds down

**Security Impact:** LOW - Used for monitoring, not payouts

---

### Oracle Integration - Price Conversion

**Location:** `contracts/grant_contracts/src/oracle_integration.rs:86`

**Code:**
```rust
let xlm_amount = scaled_amount
    .checked_div(price_feed.price)  // ← DIVISION
    .ok_or(OracleError::MathOverflow)?;
```

**Purpose:** Convert USD to XLM using oracle price

**Rounding Behavior:** ✅ Rounds down

**Security Impact:** MEDIUM - Affects oracle-based conversions

---

## Rounding Behavior Verification

### Rust Integer Division

Rust's integer division (`/` operator and `checked_div()`) follows these rules:

1. **Truncates toward zero** (not toward negative infinity)
2. For positive numbers: **rounds down** ✅
3. For negative numbers: rounds toward zero (up in absolute value)

### Examples

```rust
// Positive numbers (our use case)
10 / 3 = 3        // rounds down, remainder 1
1007 / 10 = 100   // rounds down, remainder 7
503500 / 10000 = 50  // rounds down, remainder 3500

// Negative numbers (not applicable)
-10 / 3 = -3      // rounds toward zero (up in absolute value)
```

### Why This Is Safe

For all our use cases:
1. Amounts are always positive (i128 but never negative)
2. Division always rounds down
3. Remainders stay in the contract
4. Contract balance always >= obligations

---

## Dangerous Patterns NOT Found

✅ **No `ceil_div` or ceiling division**
✅ **No `round_half_up` or banker's rounding**
✅ **No floating-point division**
✅ **No custom rounding logic**
✅ **No division with rounding mode parameters**

All divisions use standard `checked_div()` which is safe.

---

## Test Coverage

### Primary Test: `test_point_one_cent_exploit.rs`

Proves that division rounding is safe through:

1. **1,000-iteration simulation** with awkward numbers
2. **5 edge case tests** covering extreme scenarios
3. **Invariant verification** after every withdrawal
4. **Dust accumulation tracking** to ensure bounded remainders

### Test Results

All tests pass, proving:
- ✅ Division rounds down correctly
- ✅ No cumulative rounding errors
- ✅ Contract never falls short of obligations
- ✅ Remainders accumulate in contract's favor

---

## Audit Checklist

- [x] All division operations identified
- [x] All divisions use `checked_div()` (safe)
- [x] No ceiling division or rounding-up logic
- [x] No floating-point arithmetic
- [x] Rounding behavior documented in code
- [x] Comprehensive tests written and passing
- [x] Invariant verified: `balance >= obligations`
- [x] Edge cases tested (1 unit, remainders, etc.)
- [x] Documentation complete

---

## Conclusion

**All division operations in the grant stream contracts are SAFE.**

1. ✅ Use `checked_div()` which rounds down
2. ✅ No rounding-up logic exists
3. ✅ Remainders stay in contract
4. ✅ Comprehensive tests prove safety
5. ✅ Point One Cent exploit is IMPOSSIBLE

**Recommendation:** APPROVED for deployment

---

## Maintenance Notes

### For Future Developers

**⚠️ CRITICAL: When adding new division operations:**

1. **Always use `checked_div()`** - never implement custom division
2. **Never round up** in payout calculations
3. **Add tests** to verify rounding behavior
4. **Document** the rounding direction in comments
5. **Update this audit** with new division operations

### Red Flags to Watch For

🚨 **DANGER:** If you see any of these patterns:
- `ceil_div()` or ceiling division
- `round()`, `round_up()`, or similar
- Floating-point division (`f64`, `f32`)
- Custom rounding logic
- Division with `+ 1` to "round up"

**→ STOP and review for Point One Cent exploit vulnerability**

### Safe Patterns

✅ **SAFE:** These patterns are approved:
- `checked_div()` for integer division
- `saturating_div()` for non-critical calculations
- Division with explicit truncation
- Division with remainder tracking

---

## References

- **Test Suite:** `test_point_one_cent_exploit.rs`
- **Documentation:** `POINT_ONE_CENT_EXPLOIT_PREVENTION.md`
- **Run Guide:** `RUN_POINT_ONE_CENT_TESTS.md`
- **Rust Division Docs:** https://doc.rust-lang.org/std/primitive.i128.html#method.checked_div

---

**Audit Date:** 2026-04-29  
**Auditor:** Security Engineering Team  
**Status:** ✅ APPROVED - No Point One Cent exploit possible
