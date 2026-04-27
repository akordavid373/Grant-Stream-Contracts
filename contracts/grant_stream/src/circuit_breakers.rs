/// Circuit Breakers: Oracle Price Deviation Guard (#312) and TVL Velocity Limit (#311).
///
/// # Issue #312 — Oracle Price Deviation Guard
/// If the XLM price reported by the oracle changes by more than 50% relative to
/// the previously stored price in a single ledger update, all price-dependent
/// operations (swaps, price-dependent withdrawals) are frozen.  The freeze is
/// lifted only after a designated "sanity-check" oracle confirms the new price.
///
/// # Issue #311 — Sudden TVL Drain (Velocity Limit)
/// Tracks the total amount withdrawn from the protocol within any rolling 6-hour
/// window.  If cumulative withdrawals exceed 20% of the total protocol liquidity
/// snapshot, the contract enters `SoftPause` mode.  An admin must explicitly call
/// `resume_after_velocity_check` to resume normal operations.

use soroban_sdk::{contracttype, Address, Env};

// ── Constants ─────────────────────────────────────────────────────────────────

/// 50% deviation threshold (in basis points: 5000 / 10000 = 50%).
const PRICE_DEVIATION_BPS: i128 = 5_000;
/// 20% TVL drain threshold (in basis points: 2000 / 10000 = 20%).
const TVL_DRAIN_BPS: i128 = 2_000;
/// 6-hour rolling window in seconds.
const VELOCITY_WINDOW_SECS: u64 = 6 * 60 * 60;
/// 48-hour oracle heartbeat interval.
const ORACLE_HEARTBEAT_INTERVAL_SECS: u64 = 48 * 60 * 60;

// ── Storage Keys ──────────────────────────────────────────────────────────────

#[contracttype]
pub enum CircuitBreakerKey {
    /// Last confirmed oracle price (i128, scaled by SCALING_FACTOR).
    LastOraclePrice,
    /// Address of the sanity-check oracle that can confirm a suspicious price.
    SanityOracle,
    /// Whether the oracle price circuit breaker is currently tripped.
    OracleFrozen,
    /// Total liquidity snapshot used as the denominator for velocity checks (i128).
    TvlSnapshot,
    /// Timestamp when the current velocity window started (u64).
    VelocityWindowStart,
    /// Cumulative withdrawals in the current velocity window (i128).
    VelocityAccumulator,
    /// Whether the contract is in SoftPause due to a velocity-limit breach.
    SoftPaused,
    /// Last oracle heartbeat timestamp.
    OracleLastHeartbeat,
    /// Whether oracle is frozen due to no heartbeat.
    OracleFrozenDueToNoHeartbeat,
    /// Manual exchange rate set by DAO.
    ManualExchangeRate,
}

// ── Oracle Price Guard (Issue #312) ───────────────────────────────────────────

/// Record a new oracle price ping.  Returns `true` if the price was accepted
/// normally, or `false` if the deviation exceeded 50% and the guard was tripped.
///
/// When the guard is tripped the caller should prevent any price-dependent
/// operations until `confirm_oracle_price` is called by the sanity oracle.
pub fn record_oracle_price(env: &Env, new_price: i128) -> bool {
    // Ping the heartbeat on price update
    ping_oracle_heartbeat(env);

    let last: i128 = env
        .storage()
        .instance()
        .get(&CircuitBreakerKey::LastOraclePrice)
        .unwrap_or(0);

    if last > 0 {
        // Calculate absolute deviation in basis points.
        let diff = if new_price > last { new_price - last } else { last - new_price };
        let deviation_bps = diff
            .saturating_mul(10_000)
            .checked_div(last)
            .unwrap_or(i128::MAX);

        if deviation_bps >= PRICE_DEVIATION_BPS {
            // Trip the circuit breaker — do NOT update the stored price yet.
            env.storage()
                .instance()
                .set(&CircuitBreakerKey::OracleFrozen, &true);
            return false; // price rejected; guard tripped
        }
    }

    // Price is within acceptable range — store it and ensure guard is clear.
    env.storage()
        .instance()
        .set(&CircuitBreakerKey::LastOraclePrice, &new_price);
    env.storage()
        .instance()
        .set(&CircuitBreakerKey::OracleFrozen, &false);
    true
}

/// Called by the sanity-check oracle to confirm a suspicious price after the
/// guard has been tripped.  Clears the freeze and stores the confirmed price.
pub fn confirm_oracle_price(env: &Env, caller: &Address, confirmed_price: i128) {
    let sanity_oracle: Address = env
        .storage()
        .instance()
        .get(&CircuitBreakerKey::SanityOracle)
        .expect("SanityOracle not configured");
    if *caller != sanity_oracle {
        panic!("confirm_oracle_price: caller is not the sanity oracle");
    }
    caller.require_auth();

    env.storage()
        .instance()
        .set(&CircuitBreakerKey::LastOraclePrice, &confirmed_price);
    env.storage()
        .instance()
        .set(&CircuitBreakerKey::OracleFrozen, &false);
}

/// Returns `true` when the oracle price circuit breaker is active (frozen).
pub fn is_oracle_frozen(env: &Env) -> bool {
    env.storage()
        .instance()
        .get(&CircuitBreakerKey::OracleFrozen)
        .unwrap_or(false) ||
    env.storage()
        .instance()
        .get(&CircuitBreakerKey::OracleFrozenDueToNoHeartbeat)
        .unwrap_or(false)
}

/// Set (or update) the sanity-check oracle address.  Must be called by the
/// contract admin before the guard can be cleared via `confirm_oracle_price`.
pub fn set_sanity_oracle(env: &Env, sanity_oracle: &Address) {
    env.storage()
        .instance()
        .set(&CircuitBreakerKey::SanityOracle, sanity_oracle);
}

// ── Oracle Heartbeat and Manual Revert (Emergency Manual Revert for Oracle Freeze) ───────────────────────────────────────────

/// Ping the oracle heartbeat, resetting the freeze if active.
pub fn ping_oracle_heartbeat(env: &Env) {
    let now = env.ledger().timestamp();
    env.storage()
        .instance()
        .set(&CircuitBreakerKey::OracleLastHeartbeat, &now);
    env.storage()
        .instance()
        .set(&CircuitBreakerKey::OracleFrozenDueToNoHeartbeat, &false);
}

/// Check if oracle heartbeat is still valid. If not, freeze.
pub fn check_oracle_heartbeat(env: &Env) -> bool {
    let last: u64 = env
        .storage()
        .instance()
        .get(&CircuitBreakerKey::OracleLastHeartbeat)
        .unwrap_or(0);
    let current = env.ledger().timestamp();
    if current.saturating_sub(last) > ORACLE_HEARTBEAT_INTERVAL_SECS {
        env.storage()
            .instance()
            .set(&CircuitBreakerKey::OracleFrozenDueToNoHeartbeat, &true);
        false
    } else {
        true
    }
}

/// Set manual exchange rate via DAO vote, clearing the freeze.
pub fn set_manual_exchange_rate(env: &Env, rate: i128) {
    env.storage()
        .instance()
        .set(&CircuitBreakerKey::ManualExchangeRate, &rate);
    env.storage()
        .instance()
        .set(&CircuitBreakerKey::OracleFrozenDueToNoHeartbeat, &false);
}

/// Get the manual exchange rate if set.
pub fn get_manual_exchange_rate(env: &Env) -> Option<i128> {
    env.storage()
        .instance()
        .get(&CircuitBreakerKey::ManualExchangeRate)
}

// ── TVL Velocity Limit (Issue #311) ───────────────────────────────────────────

/// Update the TVL snapshot used as the denominator for velocity checks.
/// Should be called whenever the total protocol liquidity changes materially
/// (e.g., after a large deposit or at initialisation).
pub fn update_tvl_snapshot(env: &Env, total_liquidity: i128) {
    env.storage()
        .instance()
        .set(&CircuitBreakerKey::TvlSnapshot, &total_liquidity);
}

/// Record a withdrawal of `amount` tokens and check whether the rolling 6-hour
/// velocity limit has been breached.
///
/// Returns `true` if the withdrawal is within limits, `false` if the velocity
/// limit was just breached (SoftPause has been engaged).
///
/// Panics if the contract is already in SoftPause — callers must check
/// `is_soft_paused` before calling this.
pub fn record_withdrawal_velocity(env: &Env, amount: i128) -> bool {
    if is_soft_paused(env) {
        panic!("Contract is in SoftPause — admin verification required");
    }

    let now: u64 = env.ledger().timestamp();
    let window_start: u64 = env
        .storage()
        .instance()
        .get(&CircuitBreakerKey::VelocityWindowStart)
        .unwrap_or(now);
    let mut accumulator: i128 = env
        .storage()
        .instance()
        .get(&CircuitBreakerKey::VelocityAccumulator)
        .unwrap_or(0);

    // Reset window if it has expired.
    if now.saturating_sub(window_start) >= VELOCITY_WINDOW_SECS {
        env.storage()
            .instance()
            .set(&CircuitBreakerKey::VelocityWindowStart, &now);
        accumulator = 0;
    }

    accumulator = accumulator.saturating_add(amount);
    env.storage()
        .instance()
        .set(&CircuitBreakerKey::VelocityAccumulator, &accumulator);

    let tvl: i128 = env
        .storage()
        .instance()
        .get(&CircuitBreakerKey::TvlSnapshot)
        .unwrap_or(0);

    if tvl > 0 {
        let drain_bps = accumulator
            .saturating_mul(10_000)
            .checked_div(tvl)
            .unwrap_or(i128::MAX);

        if drain_bps >= TVL_DRAIN_BPS {
            env.storage()
                .instance()
                .set(&CircuitBreakerKey::SoftPaused, &true);
            return false; // velocity limit breached; SoftPause engaged
        }
    }

    true
}

/// Returns `true` when the contract is in SoftPause due to a velocity breach.
pub fn is_soft_paused(env: &Env) -> bool {
    env.storage()
        .instance()
        .get(&CircuitBreakerKey::SoftPaused)
        .unwrap_or(false)
}

/// Admin-only: clear SoftPause after manual verification.
pub fn resume_after_velocity_check(env: &Env, admin: &Address) {
    admin.require_auth();
    env.storage()
        .instance()
        .set(&CircuitBreakerKey::SoftPaused, &false);
    // Reset the velocity window so the 6-hour clock starts fresh.
    let now: u64 = env.ledger().timestamp();
    env.storage()
        .instance()
        .set(&CircuitBreakerKey::VelocityWindowStart, &now);
    env.storage()
        .instance()
        .set(&CircuitBreakerKey::VelocityAccumulator, &0_i128);
}
