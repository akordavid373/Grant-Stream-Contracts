# Formal Proof: Monotonic Time-Flow Property (Issue #305)

## Claim

Let `C(t)` denote the value of `grant.claimable` immediately after calling
`settle_grant(grant, t)` on an **Active** grant that has never been withdrawn
from.  Then for all `t₁ ≤ t₂`:

```
C(t₁) ≤ C(t₂)
```

That is, `current_claimable` is a **monotonically non-decreasing** function of
time.

---

## Definitions

| Symbol | Meaning |
|--------|---------|
| `r` | `grant.flow_rate` (tokens per second, scaled by `SCALING_FACTOR = 1e7`) |
| `t` | Current ledger timestamp (seconds since Unix epoch) |
| `t₀` | `grant.last_update_ts` at the start of a `settle_grant` call |
| `elapsed` | `t - t₀ ≥ 0` |
| `m(t)` | Warmup multiplier at time `t` (basis points, range `[2500, 10000]`) |
| `accrued(elapsed, t)` | `r * elapsed * m(t) / 10000` |
| `C(t)` | `grant.claimable` after `settle_grant(grant, t)` |

---

## Proof

### Lemma 1 — `accrued` is non-negative

```
accrued(elapsed, t) = r * elapsed * m(t) / 10000
```

- `r ≥ 0` (enforced by `create_grant`: `flow_rate ≥ 0`).
- `elapsed = t - t₀ ≥ 0` (enforced by the guard `if now < grant.last_update_ts { return Err }` in `settle_grant`).
- `m(t) ∈ [2500, 10000]` (see `calculate_warmup_multiplier`; minimum is 25% = 2500 bps).

Therefore `accrued ≥ 0`. ∎

### Lemma 2 — `settle_grant` only adds to `claimable`

Inside `settle_grant`, the only mutation of `claimable` is:

```rust
grant.claimable = grant.claimable
    .checked_add(grantee_share)   // grantee_share ≥ 0  (Lemma 1)
    .ok_or(Error::MathOverflow)?;
```

(The validator path is analogous.)  No code path inside `settle_grant`
*subtracts* from `claimable`; the cap applied when `total_accounted >=
total_amount` only prevents `claimable` from exceeding the remaining budget —
it never reduces a value that was already within budget.

Therefore, for any two calls `settle_grant(grant, t₁)` and
`settle_grant(grant, t₂)` with `t₁ ≤ t₂` and no intervening `withdraw`:

```
C(t₂) = C(t₁) + accrued(t₂ - t₁, t₂) ≥ C(t₁)
```

∎

### Lemma 3 — Pending-rate switch preserves monotonicity

When a pending rate increase becomes effective at `switch_ts ∈ [t₀, t]`:

```
C(t) = C(t₀)
     + accrued(switch_ts - t₀, switch_ts)   // old rate segment ≥ 0
     + accrued(t - switch_ts, t)             // new rate segment ≥ 0
```

Both segments are non-negative (Lemma 1), so `C(t) ≥ C(t₀)`. ∎

### Main Theorem

By induction on the sequence of `settle_grant` calls:

- **Base case**: `C(t₀) = 0` (grant just created, `claimable = 0`).
- **Inductive step**: For any `t > t_prev`, `C(t) = C(t_prev) + Δ` where
  `Δ ≥ 0` (Lemmas 1–3).

Therefore `C` is monotonically non-decreasing in `t`. ∎

---

## Corollary — No "Time-Reversal" Bug

The contract enforces `now ≥ grant.last_update_ts` at the top of
`settle_grant`.  Combined with the monotonicity proof above, it is impossible
for a grantee's claimable balance to decrease due to the passage of time alone.
A decrease can only occur through an explicit `withdraw` call, which is an
intentional, authorised action by the grantee.

---

## Scope and Assumptions

1. The proof holds for **Active** grants.  Paused grants do not accrue (the
   `if grant.status == GrantStatus::Active` guard in `settle_grant`), so their
   `claimable` is constant — trivially non-decreasing.
2. The proof assumes no integer overflow.  Overflow is guarded by
   `checked_add` / `checked_mul` throughout; any overflow returns
   `Error::MathOverflow` rather than silently wrapping.
3. The proof does not cover `cancel_grant` or `rage_quit`, which are
   intentional state-termination paths, not time-flow paths.
