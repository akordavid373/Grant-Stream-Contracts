# Security Invariants Manifest for Grant-Stream Contracts

## Overview

This document outlines all security invariants that must never be violated in the Grant-Stream smart contract system. These invariants serve as the fundamental security guarantees of the protocol and are essential for auditors to verify the correctness of the implementation.

## Core Financial Invariants

### 1. Total Supply Invariant
**Rule:** The sum of all distributed funds can never exceed the initial grant amount.

**Mathematical Representation:**
```
Σ(withdrawn_i + claimable_i + validator_withdrawn_i + validator_claimable_i) ≤ total_amount_i
```

**Verification Points:**
- `settle_grant()` function must cap accruals to prevent overflow
- `withdraw()` function must validate sufficient claimable balance
- `rage_quit()` and `cancel_grant()` must return remaining funds to treasury
- Emergency functions must preserve total allocated funds

### 2. Non-Negative Balance Invariant
**Rule:** All token balances must remain non-negative at all times.

**Applicable Fields:**
- `grant.withdrawn` ≥ 0
- `grant.claimable` ≥ 0  
- `grant.validator_withdrawn` ≥ 0
- `grant.validator_claimable` ≥ 0
- `grant.flow_rate` ≥ 0

**Verification Points:**
- All arithmetic operations must use checked math
- Underflow protection in all subtraction operations
- Rate changes must validate non-negative values

### 3. Accrual Rate Invariant
**Rule:** Token accrual rate must never exceed the configured flow rate.

**Constraints:**
- `accrued_tokens ≤ flow_rate × elapsed_time`
- Warmup multiplier may reduce but never increase base rate
- KPI multipliers apply to base rate only

**Verification Points:**
- `calculate_accrued()` function enforces rate limits
- Warmup multiplier bounded between 25% and 100%
- Oracle price updates cannot increase accrual beyond base rate

## State Consistency Invariants

### 4. Status Transition Invariant
**Rule:** Grant status transitions must follow the defined state machine.

**Valid Transitions:**
- Active → Paused (admin only)
- Paused → Active (admin only)
- Active/Paused → Completed (when fully claimed)
- Active/Paused → Cancelled (admin only)
- Paused → RageQuitted (grantee only)

**Invalid Transitions:**
- Completed → Any other state
- RageQuitted → Any other state
- Cancelled → Active/Paused

**Verification Points:**
- All state-changing functions validate current status
- Status changes are atomic with associated state updates
- No bypass of state machine through emergency functions

### 5. Timestamp Consistency Invariant
**Rule:** All timestamps must be monotonically increasing and within reasonable bounds.

**Constraints:**
- `last_update_ts` ≤ `current_ledger_timestamp`
- `rate_updated_at` ≤ `last_update_ts`
- `effective_timestamp` ≥ `current_ledger_timestamp` for pending changes

**Verification Points:**
- `settle_grant()` validates timestamp ordering
- Rate changes set future effective timestamps
- No retroactive timestamp modifications

### 6. Authorization Invariant
**Rule:** All protected operations must require proper authentication.

**Protected Operations:**
- Admin functions: `pause_stream()`, `cancel_grant()`, `rescue_tokens()`
- Grantee functions: `withdraw()`, `rage_quit()`
- Oracle functions: `apply_kpi_multiplier()`, `submit_oracle_price()`

**Verification Points:**
- `require_admin_auth()` for admin operations
- `grant.recipient.require_auth()` for grantee operations
- Multi-sig verification for critical operations

## Access Control Invariants

### 7. Multi-Sig Threshold Invariant
**Rule:** Multi-signature operations must meet required thresholds before execution.

**Threshold Requirements:**
- Standard operations: 3-of-5 signers
- Emergency operations: 7-of-10 signers
- No single signer can execute critical operations alone

**Verification Points:**
- `execute_rescue()` validates approval count
- Duplicate approvals are rejected
- Signer set is immutable after initialization

### 8. Council Authority Invariant
**Rule:** Only registered council members can execute governance proposals.

**Constraints:**
- Council membership verified via byte comparison
- Proposal execution requires council member authentication
- Council updates require existing council member approval

**Verification Points:**
- `require_council_auth()` function
- Optimized signer verification using pre-serialized bytes
- No bypass of council authentication

## Economic Security Invariants

### 9. Validator Share Invariant
**Rule:** Validator rewards are exactly 5% of all accruals when validator is set.

**Calculation:**
```
validator_share = accrued × 500 / 10000
grantee_share = accrued - validator_share
```

**Verification Points:**
- `apply_accrued_split()` function enforces 5% split
- No validator fees when no validator address is set
- Validator withdrawals tracked separately

### 10. Rescue Funds Invariant
**Rule:** Rescue operations cannot violate allocated fund constraints.

**Constraints:**
- `rescue_tokens()` cannot transfer allocated grant funds
- Treasury rescue only affects unallocated balances
- Emergency preserves grantee claimable amounts

**Verification Points:**
- `total_allocated_funds()` calculation
- Balance validation before rescue transfers
- Treasury-only rescue destinations

## Circuit Breaker Invariants

### 11. Oracle Price Invariant
**Rule:** Oracle price deviations beyond 50% trigger automatic freeze.

**Trigger Conditions:**
- `|new_price - last_price| / last_price > 0.5`
- Frozen oracle blocks price-dependent operations
- Manual confirmation required to unfreeze

**Verification Points:**
- `record_oracle_price()` function
- `is_oracle_frozen()` status check
- Price-dependent operations validate oracle status

### 12. Withdrawal Velocity Invariant
**Rule:** Excessive withdrawal velocity triggers soft pause protection.

**Thresholds:**
- Velocity calculations based on TVL snapshots
- Soft pause blocks non-essential withdrawals
- Emergency operations remain available

**Verification Points:**
- `record_withdrawal_velocity()` function
- `is_soft_paused()` status validation
- Velocity limit configuration

## Legal Compliance Invariants

### 13. Legal Signature Invariant
**Rule:** Grants requiring legal signatures cannot accrue tokens until signed.

**Conditions:**
- `requires_legal_signature = true` blocks accruals
- `is_legal_signed = false` prevents withdrawals
- Legal hash must be set before signature requirement

**Verification Points:**
- `settle_grant()` checks legal signature status
- Withdrawal functions validate legal compliance
- Legal document hash immutability

### 14. Tax Reporting Invariant
**Rule:** All taxable events must be recorded for compliance.

**Recorded Events:**
- Grant withdrawals
- Validator reward distributions
- Treasury rescue operations

**Verification Points:**
- `record_flow()` function calls
- Audit trail completeness
- Event emission for all transfers

## Gas and Resource Invariants

### 15. Gas Buffer Invariant
**Rule:** Critical multi-sig operations reserve sufficient gas for completion.

**Buffer Requirements:**
- Standard operations: 5M gas units buffer
- Emergency operations: 10M gas units buffer
- Gas usage monitoring and optimization

**Verification Points:**
- `get_gas_buffer()` function
- Gas buffer configuration
- Priority gas for emergency operations

### 16. Storage Rent Invariant
**Rule:** Contract must maintain sufficient XLM for storage rent payments.

**Requirements:**
- Minimum reserve: 5 XLM
- Rent preservation mode during low balance
- Non-essential operations blocked when rent is critical

**Verification Points:**
- `check_rent_balance()` function
- Rent preservation mode activation
- Balance monitoring and alerts

## Emergency Response Invariants

### 17. Pause Reason Invariant
**Rule:** All pause operations must include transparent reason strings.

**Requirements:**
- Grant-level pauses stored with grant
- Protocol-level pauses stored globally
- Reasons emitted in ProtocolPaused events

**Verification Points:**
- `pause_stream()` includes reason parameter
- `emergency_pause()` stores global reason
- Event emission for transparency

### 18. Recovery Access Invariant
**Rule:** Emergency recovery mechanisms must always remain accessible.

**Guaranteed Operations:**
- Emergency pause functionality
- Multi-sig treasury rescue
- Oracle price confirmation

**Verification Points:**
- No circuit breaker can block emergency functions
- Gas buffer ensures execution capability
- Admin authentication bypassed in emergencies

## Audit Verification Checklist

### For Each Invariant, Verify:
- [ ] Implementation matches mathematical specification
- [ ] Edge cases are properly handled
- [ ] Error conditions prevent invariant violations
- [ ] Test coverage includes invariant scenarios
- [ ] Gas optimization doesn't break invariants
- [ ] Emergency preserves maintain invariants

### Cross-Invariant Dependencies:
- [ ] Financial invariants remain consistent during state changes
- [ ] Access control invariants protect economic invariants
- [ ] Circuit breakers preserve core financial guarantees
- [ ] Emergency responses maintain system integrity

## Testing Requirements

### Unit Tests:
- Each invariant tested with boundary conditions
- Error path testing for violation attempts
- State transition validation

### Integration Tests:
- Multi-contract invariant preservation
- Cross-function interaction testing
- Emergency scenario validation

### Fuzz Testing:
- Random operation sequences preserve invariants
- Edge case discovery through fuzzing
- Long-running state consistency

## Conclusion

These invariants represent the fundamental security properties of the Grant-Stream protocol. Any modification to the contract must preserve all invariants, and auditors should use this document as the primary reference for security verification.

**Document Version:** 1.0  
**Last Updated:** 2026-04-28  
**Next Review:** 2026-07-28
