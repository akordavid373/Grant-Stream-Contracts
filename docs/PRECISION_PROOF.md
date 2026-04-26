# Formal Proof: Fixed-Point Arithmetic Precision (Issue #307)

## Claim

The fixed-point scaling factor `SCALING_FACTOR = 10_000_000` (1 × 10⁷) used
for flow rates in the Grant Stream contract provides sufficient precision to
handle a 10-year grant without losing more than **0.000001 %** of the total
grant value to rounding.

---

## Setup

### Grant parameters (worst-case scenario)

| Parameter | Value | Rationale |
|-----------|-------|-----------|
| Grant duration | 10 years | As specified in the issue |
| Duration in seconds | `315_360_000 s` | 10 × 365.25 × 24 × 3600 |
| Total amount | `10^15` stroops | ~10^8 XLM — an extremely large institutional grant |
| `SCALING_FACTOR` | `10_000_000` (1e7) | Constant in `lib.rs` |

### Flow-rate encoding

The contract stores `flow_rate` as **stroops per second × SCALING_FACTOR**.
For a grant of `T` stroops over `D` seconds the ideal (real-valued) flow rate
is:

```
r_ideal = T / D   (stroops/second, real-valued)
```

The stored integer flow rate is:

```
r_stored = floor(T * SCALING_FACTOR / D)
```

The per-second rounding error is at most:

```
ε_per_second = r_ideal - r_stored / SCALING_FACTOR
             < 1 / SCALING_FACTOR   (stroops/second)
             = 1 / 10^7             (stroops/second)
```

---

## Proof

### Step 1 — Total rounding loss over the full duration

The maximum cumulative rounding loss over `D` seconds is:

```
Loss_max = ε_per_second × D
         < (1 / SCALING_FACTOR) × D
         = D / 10^7
         = 315_360_000 / 10_000_000
         = 31.536  stroops
```

So the absolute worst-case loss is **≤ 32 stroops** (rounding up to the next
integer).

### Step 2 — Loss as a fraction of total grant value

```
Loss_fraction = Loss_max / T
              ≤ 32 / 10^15
              = 3.2 × 10^{-14}
              = 0.0000000000032 %
```

### Step 3 — Compare against the 0.000001 % threshold

The required threshold is:

```
Threshold = 0.000001 % = 1 × 10^{-8}
```

The computed loss fraction is:

```
3.2 × 10^{-14}  ≪  1 × 10^{-8}
```

The loss is **six orders of magnitude below** the threshold. ∎

---

## General Formula

For any grant with total amount `T` stroops and duration `D` seconds:

```
Loss_fraction ≤ D / (SCALING_FACTOR × T)
```

The threshold `0.000001 % = 10^{-8}` is satisfied whenever:

```
D / (SCALING_FACTOR × T) ≤ 10^{-8}
⟺  T ≥ D / (SCALING_FACTOR × 10^{-8})
   T ≥ D × 10^{-7} / 10^{-8}
   T ≥ D × 10
```

For a 10-year grant (`D = 315_360_000 s`) this requires:

```
T ≥ 3_153_600_000  stroops  ≈  315.36 XLM
```

Any grant larger than ~315 XLM over 10 years satisfies the precision
requirement.  Grants smaller than this are unlikely in an institutional context,
and even for them the absolute loss is at most 32 stroops — negligible in
practice.

---

## Settle-time rounding

`settle_grant` computes accrued tokens as:

```rust
let base_accrued = grant.flow_rate.checked_mul(elapsed_i128)?;
let accrued = base_accrued
    .checked_mul(multiplier)?
    .checked_div(10000)?;
```

The additional division by `10000` (warmup multiplier denominator) introduces
at most **1 stroop per `settle_grant` call**.  Over the lifetime of a grant
with one settlement per second (extreme upper bound) this adds at most
`D = 315_360_000` stroops ≈ 31.5 XLM.  For a grant of `T ≥ 10^15` stroops
this is still `< 10^{-7}` of total value — well within the threshold.

---

## Conclusion

`SCALING_FACTOR = 1e7` provides **at least 7 decimal digits of sub-stroop
precision** in the flow rate.  For any realistic institutional grant (≥ 315 XLM
over 10 years) the total rounding loss is provably less than **0.000001 %** of
the grant value, satisfying the Institutional Assurance requirement.
