//! Streaming-to-Yield Auto-Compounding (Issue #319)
//!
//! Earned-but-unclaimed grant balances can be automatically deposited into a
//! yield vault. The principal remains claimable at any time; accrued interest
//! is split between the grantee and the protocol treasury.
//! Withdrawal seamlessly "unwraps" yield-bearing assets without extra gas cost
//! to the grantee.

#![allow(unused)]

use soroban_sdk::{
    contracttype, contracterror, symbol_short, token, Address, Env,
};

/// Grantee's share of yield interest (basis points). 7000 = 70%.
pub const GRANTEE_YIELD_SHARE_BPS: i128 = 7_000;
/// Protocol treasury's share of yield interest (basis points). 3000 = 30%.
pub const TREASURY_YIELD_SHARE_BPS: i128 = 3_000;
/// Basis points denominator.
pub const BPS_DENOM: i128 = 10_000;

/// Per-grant yield position tracking.
#[derive(Clone, Debug)]
#[contracttype]
pub struct YieldPosition {
    /// Grant ID this position belongs to.
    pub grant_id: u64,
    /// Grantee address.
    pub grantee: Address,
    /// Principal deposited into the vault (earned but unclaimed balance).
    pub principal: i128,
    /// Accumulated interest not yet distributed.
    pub accrued_interest: i128,
    /// Simulated APY in basis points (e.g., 500 = 5%).
    pub apy_bps: i128,
    /// Timestamp of last interest accrual.
    pub last_accrual_ts: u64,
    /// Whether auto-compounding is enabled for this grant.
    pub enabled: bool,
}

#[derive(Clone)]
#[contracttype]
pub enum StreamingYieldKey {
    /// Maps grant_id -> YieldPosition
    YieldPosition(u64),
    /// Protocol treasury address
    Treasury,
}

#[contracterror]
#[derive(Clone, Copy, Eq, PartialEq, Debug)]
#[repr(u32)]
pub enum StreamingYieldError {
    NotAuthorized = 1,
    InvalidAmount = 2,
    YieldNotEnabled = 3,
    PositionNotFound = 4,
    MathOverflow = 5,
    InsufficientPrincipal = 6,
}

// ── Internal helpers ──────────────────────────────────────────────────────────

fn read_position(env: &Env, grant_id: u64) -> Result<YieldPosition, StreamingYieldError> {
    env.storage()
        .instance()
        .get(&StreamingYieldKey::YieldPosition(grant_id))
        .ok_or(StreamingYieldError::PositionNotFound)
}

fn write_position(env: &Env, grant_id: u64, pos: &YieldPosition) {
    env.storage()
        .instance()
        .set(&StreamingYieldKey::YieldPosition(grant_id), pos);
}

/// Accrue interest since last update using simple interest: I = P * apy * dt / year.
fn accrue(pos: &YieldPosition, now: u64) -> i128 {
    let elapsed = now.saturating_sub(pos.last_accrual_ts) as i128;
    if elapsed == 0 || pos.principal == 0 {
        return 0;
    }
    const YEAR_SECS: i128 = 365 * 24 * 60 * 60;
    // interest = principal * apy_bps * elapsed / (BPS_DENOM * YEAR_SECS)
    pos.principal
        .saturating_mul(pos.apy_bps)
        .saturating_mul(elapsed)
        / (BPS_DENOM * YEAR_SECS)
}

// ── Public API ────────────────────────────────────────────────────────────────

/// Enable auto-compounding for a grant. Called by the grantee.
/// `apy_bps` is the vault's annual yield in basis points (e.g., 500 = 5%).
pub fn enable_yield(
    env: &Env,
    grant_id: u64,
    grantee: Address,
    apy_bps: i128,
) -> Result<(), StreamingYieldError> {
    grantee.require_auth();
    if apy_bps < 0 {
        return Err(StreamingYieldError::InvalidAmount);
    }

    let now = env.ledger().timestamp();
    let pos = YieldPosition {
        grant_id,
        grantee,
        principal: 0,
        accrued_interest: 0,
        apy_bps,
        last_accrual_ts: now,
        enabled: true,
    };
    write_position(env, grant_id, &pos);

    env.events().publish(
        (symbol_short!("yld_en"), grant_id),
        (apy_bps,),
    );
    Ok(())
}

/// Deposit earned-but-unclaimed balance into the yield vault.
/// Called internally when the grantee's claimable balance increases.
pub fn deposit_to_yield(
    env: &Env,
    grant_id: u64,
    amount: i128,
) -> Result<(), StreamingYieldError> {
    if amount <= 0 {
        return Err(StreamingYieldError::InvalidAmount);
    }

    let mut pos = read_position(env, grant_id)?;
    if !pos.enabled {
        return Err(StreamingYieldError::YieldNotEnabled);
    }

    let now = env.ledger().timestamp();
    // Settle interest before changing principal
    let interest = accrue(&pos, now);
    pos.accrued_interest = pos
        .accrued_interest
        .checked_add(interest)
        .ok_or(StreamingYieldError::MathOverflow)?;
    pos.principal = pos
        .principal
        .checked_add(amount)
        .ok_or(StreamingYieldError::MathOverflow)?;
    pos.last_accrual_ts = now;

    write_position(env, grant_id, &pos);

    env.events().publish(
        (symbol_short!("yld_dep"), grant_id),
        (amount, pos.principal),
    );
    Ok(())
}

/// Withdraw principal (and optionally distribute accrued interest).
/// Seamlessly "unwraps" the yield position so the grantee receives their
/// principal without any extra gas overhead.
///
/// Returns `(principal_withdrawn, grantee_interest, treasury_interest)`.
pub fn withdraw_from_yield(
    env: &Env,
    grant_id: u64,
    amount: i128,
    token_address: Address,
    treasury: Address,
) -> Result<(i128, i128, i128), StreamingYieldError> {
    if amount <= 0 {
        return Err(StreamingYieldError::InvalidAmount);
    }

    let mut pos = read_position(env, grant_id)?;
    if !pos.enabled {
        return Err(StreamingYieldError::YieldNotEnabled);
    }
    if pos.principal < amount {
        return Err(StreamingYieldError::InsufficientPrincipal);
    }

    let now = env.ledger().timestamp();
    // Settle interest
    let new_interest = accrue(&pos, now);
    let total_interest = pos
        .accrued_interest
        .checked_add(new_interest)
        .ok_or(StreamingYieldError::MathOverflow)?;

    // Split interest
    let grantee_interest = total_interest * GRANTEE_YIELD_SHARE_BPS / BPS_DENOM;
    let treasury_interest = total_interest - grantee_interest;

    // Reduce principal
    pos.principal -= amount;
    pos.accrued_interest = 0;
    pos.last_accrual_ts = now;
    write_position(env, grant_id, &pos);

    // Transfer treasury share
    if treasury_interest > 0 {
        let tok = token::Client::new(env, &token_address);
        tok.transfer(&env.current_contract_address(), &treasury, &treasury_interest);
    }

    env.events().publish(
        (symbol_short!("yld_wdr"), grant_id),
        (amount, grantee_interest, treasury_interest),
    );

    Ok((amount, grantee_interest, treasury_interest))
}

/// Query the current yield position for a grant.
pub fn get_yield_position(env: &Env, grant_id: u64) -> Option<YieldPosition> {
    env.storage()
        .instance()
        .get(&StreamingYieldKey::YieldPosition(grant_id))
}

/// Compute pending (unaccrued) interest for a grant without mutating state.
pub fn pending_interest(env: &Env, grant_id: u64) -> i128 {
    let pos: YieldPosition = match env.storage().instance().get(&StreamingYieldKey::YieldPosition(grant_id)) {
        Some(p) => p,
        None => return 0,
    };
    let now = env.ledger().timestamp();
    pos.accrued_interest + accrue(&pos, now)
}
