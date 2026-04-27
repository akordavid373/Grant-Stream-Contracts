//! Flash-Loan Defense for Matching Pool Deposits (Issue #320)
//!
//! Prevents flash-loan attacks on Quadratic Funding matching pools by enforcing
//! a 48-hour deposit age requirement before funds count toward matching weight.

#![allow(unused)]

use soroban_sdk::{
    contracttype, contracterror, symbol_short, token, Address, Env, Map,
};

/// Minimum deposit age in seconds before funds count toward matching weight (48 hours)
pub const DEPOSIT_AGE_REQUIREMENT_SECS: u64 = 48 * 60 * 60;

/// Deposit ledger entry tracking when funds were deposited
#[derive(Clone, Debug)]
#[contracttype]
pub struct DepositRecord {
    pub depositor: Address,
    pub pool_token: Address,
    pub amount: i128,
    pub deposited_at: u64,
}

#[derive(Clone)]
#[contracttype]
pub enum FlashLoanDefenseKey {
    /// Maps (pool_token, depositor) -> DepositRecord
    DepositLedger(Address, Address),
    /// Maps pool_token -> total mature balance (deposits older than 48h)
    MatureBalance(Address),
    /// Maps pool_token -> total pending balance (deposits younger than 48h)
    PendingBalance(Address),
}

#[contracterror]
#[derive(Clone, Copy, Eq, PartialEq, Debug)]
#[repr(u32)]
pub enum FlashLoanDefenseError {
    NotInitialized = 1,
    NotAuthorized = 2,
    InvalidAmount = 3,
    DepositTooYoung = 4,      // Deposit has not aged 48h yet
    DepositNotFound = 5,
    InsufficientBalance = 6,
    MathOverflow = 7,
}

/// Record a new deposit into the matching pool.
/// The deposit is tracked with its timestamp; it will not count toward
/// matching weight until 48 hours have elapsed.
pub fn record_deposit(
    env: &Env,
    pool_token: Address,
    depositor: Address,
    amount: i128,
) -> Result<(), FlashLoanDefenseError> {
    if amount <= 0 {
        return Err(FlashLoanDefenseError::InvalidAmount);
    }

    let now = env.ledger().timestamp();

    // If depositor already has a record, accumulate (reset age to now for new funds)
    let key = FlashLoanDefenseKey::DepositLedger(pool_token.clone(), depositor.clone());
    let existing: Option<DepositRecord> = env.storage().instance().get(&key);

    let (new_amount, deposit_at) = if let Some(rec) = existing {
        // Merge: new deposit resets the clock on the combined amount
        let merged = rec.amount.checked_add(amount).ok_or(FlashLoanDefenseError::MathOverflow)?;
        (merged, now)
    } else {
        (amount, now)
    };

    let record = DepositRecord {
        depositor: depositor.clone(),
        pool_token: pool_token.clone(),
        amount: new_amount,
        deposited_at: deposit_at,
    };

    env.storage().instance().set(&key, &record);

    // Update pending balance
    let pending_key = FlashLoanDefenseKey::PendingBalance(pool_token.clone());
    let pending: i128 = env.storage().instance().get(&pending_key).unwrap_or(0);
    let new_pending = pending.checked_add(amount).ok_or(FlashLoanDefenseError::MathOverflow)?;
    env.storage().instance().set(&pending_key, &new_pending);

    env.events().publish(
        (symbol_short!("dep_rec"), pool_token),
        (depositor, amount, deposit_at),
    );

    Ok(())
}

/// Compute the matching weight for a depositor.
/// Returns 0 if the deposit is younger than 48 hours (flash-loan defense).
/// Returns the full deposited amount if the deposit has matured.
pub fn get_matching_weight(
    env: &Env,
    pool_token: Address,
    depositor: Address,
) -> i128 {
    let key = FlashLoanDefenseKey::DepositLedger(pool_token, depositor);
    let record: DepositRecord = match env.storage().instance().get(&key) {
        Some(r) => r,
        None => return 0,
    };

    let now = env.ledger().timestamp();
    let age = now.saturating_sub(record.deposited_at);

    if age >= DEPOSIT_AGE_REQUIREMENT_SECS {
        record.amount
    } else {
        0
    }
}

/// Withdraw from the matching pool.
/// Enforces that only mature deposits (>= 48h old) can be withdrawn.
pub fn withdraw_from_pool(
    env: &Env,
    pool_token: Address,
    depositor: Address,
    amount: i128,
) -> Result<(), FlashLoanDefenseError> {
    if amount <= 0 {
        return Err(FlashLoanDefenseError::InvalidAmount);
    }

    let key = FlashLoanDefenseKey::DepositLedger(pool_token.clone(), depositor.clone());
    let record: DepositRecord = env
        .storage()
        .instance()
        .get(&key)
        .ok_or(FlashLoanDefenseError::DepositNotFound)?;

    let now = env.ledger().timestamp();
    let age = now.saturating_sub(record.deposited_at);

    if age < DEPOSIT_AGE_REQUIREMENT_SECS {
        return Err(FlashLoanDefenseError::DepositTooYoung);
    }

    if record.amount < amount {
        return Err(FlashLoanDefenseError::InsufficientBalance);
    }

    let remaining = record.amount - amount;
    if remaining == 0 {
        env.storage().instance().remove(&key);
    } else {
        let updated = DepositRecord {
            amount: remaining,
            ..record
        };
        env.storage().instance().set(&key, &updated);
    }

    // Update mature balance
    let mature_key = FlashLoanDefenseKey::MatureBalance(pool_token.clone());
    let mature: i128 = env.storage().instance().get(&mature_key).unwrap_or(0);
    let new_mature = mature.saturating_sub(amount);
    env.storage().instance().set(&mature_key, &new_mature);

    env.events().publish(
        (symbol_short!("dep_wdr"), pool_token),
        (depositor, amount),
    );

    Ok(())
}

/// Settle pending deposits: move deposits that have aged >= 48h from pending to mature.
/// Should be called periodically (e.g., before computing matching weights).
pub fn settle_pending_deposits(
    env: &Env,
    pool_token: Address,
    depositors: soroban_sdk::Vec<Address>,
) -> Result<i128, FlashLoanDefenseError> {
    let now = env.ledger().timestamp();
    let mut newly_matured: i128 = 0;

    for depositor in depositors.iter() {
        let key = FlashLoanDefenseKey::DepositLedger(pool_token.clone(), depositor.clone());
        let record: DepositRecord = match env.storage().instance().get(&key) {
            Some(r) => r,
            None => continue,
        };

        let age = now.saturating_sub(record.deposited_at);
        if age >= DEPOSIT_AGE_REQUIREMENT_SECS {
            newly_matured = newly_matured
                .checked_add(record.amount)
                .ok_or(FlashLoanDefenseError::MathOverflow)?;
        }
    }

    // Update mature balance
    let mature_key = FlashLoanDefenseKey::MatureBalance(pool_token.clone());
    let mature: i128 = env.storage().instance().get(&mature_key).unwrap_or(0);
    let new_mature = mature.checked_add(newly_matured).ok_or(FlashLoanDefenseError::MathOverflow)?;
    env.storage().instance().set(&mature_key, &new_mature);

    // Reduce pending balance
    let pending_key = FlashLoanDefenseKey::PendingBalance(pool_token.clone());
    let pending: i128 = env.storage().instance().get(&pending_key).unwrap_or(0);
    let new_pending = pending.saturating_sub(newly_matured);
    env.storage().instance().set(&pending_key, &new_pending);

    Ok(newly_matured)
}

/// Returns the total mature balance for a pool (eligible for matching weight calculation).
pub fn get_mature_pool_balance(env: &Env, pool_token: Address) -> i128 {
    env.storage()
        .instance()
        .get(&FlashLoanDefenseKey::MatureBalance(pool_token))
        .unwrap_or(0)
}

/// Returns the total pending balance for a pool (not yet eligible).
pub fn get_pending_pool_balance(env: &Env, pool_token: Address) -> i128 {
    env.storage()
        .instance()
        .get(&FlashLoanDefenseKey::PendingBalance(pool_token))
        .unwrap_or(0)
}
