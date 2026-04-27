//! Protocol-Succession / DAO Handover (Issue #318)
//!
//! Implements a two-step "root ownership" transfer:
//!   1. Current admin proposes a new DAO address via `initiate_succession`.
//!   2. The new DAO must call `confirm_succession` (proving key control) before
//!      the original admin's permissions are revoked.
//!
//! This prevents "orphaned" protocol states where no one controls the contract.

#![allow(unused)]

use soroban_sdk::{
    contracttype, contracterror, symbol_short, Address, Env,
};

/// Succession state machine.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[contracttype]
pub enum SuccessionStatus {
    /// No pending succession.
    Idle,
    /// Succession initiated; waiting for new DAO to confirm.
    Pending,
    /// Succession completed; new DAO is in control.
    Completed,
}

/// Pending succession record.
#[derive(Clone, Debug)]
#[contracttype]
pub struct SuccessionRecord {
    pub current_admin: Address,
    pub proposed_dao: Address,
    pub initiated_at: u64,
    pub status: SuccessionStatus,
}

#[derive(Clone)]
#[contracttype]
pub enum SuccessionKey {
    /// Current admin address.
    Admin,
    /// Pending succession record (at most one at a time).
    PendingSuccession,
}

#[contracterror]
#[derive(Clone, Copy, Eq, PartialEq, Debug)]
#[repr(u32)]
pub enum SuccessionError {
    NotInitialized = 1,
    NotAuthorized = 2,
    SuccessionAlreadyPending = 3,
    NoSuccessionPending = 4,
    WrongConfirmingAddress = 5,
    SuccessionAlreadyCompleted = 6,
}

// ── Internal helpers ──────────────────────────────────────────────────────────

fn read_admin(env: &Env) -> Result<Address, SuccessionError> {
    env.storage()
        .instance()
        .get(&SuccessionKey::Admin)
        .ok_or(SuccessionError::NotInitialized)
}

// ── Public API ────────────────────────────────────────────────────────────────

/// Initialize the succession module with the initial admin.
/// Should be called once during contract initialization.
pub fn initialize(env: &Env, admin: Address) {
    env.storage().instance().set(&SuccessionKey::Admin, &admin);
}

/// Step 1 – Current admin proposes handing control to `new_dao`.
/// The current admin's permissions remain active until `confirm_succession` is called.
pub fn initiate_succession(
    env: &Env,
    new_dao: Address,
) -> Result<(), SuccessionError> {
    let current_admin = read_admin(env)?;
    current_admin.require_auth();

    // Reject if a succession is already pending
    if env
        .storage()
        .instance()
        .has(&SuccessionKey::PendingSuccession)
    {
        return Err(SuccessionError::SuccessionAlreadyPending);
    }

    let record = SuccessionRecord {
        current_admin: current_admin.clone(),
        proposed_dao: new_dao.clone(),
        initiated_at: env.ledger().timestamp(),
        status: SuccessionStatus::Pending,
    };
    env.storage()
        .instance()
        .set(&SuccessionKey::PendingSuccession, &record);

    env.events().publish(
        (symbol_short!("suc_ini"),),
        (current_admin, new_dao),
    );
    Ok(())
}

/// Step 2 – New DAO confirms succession by signing this transaction.
/// This proves the new DAO controls the proposed address.
/// Only after this call are the original admin's permissions revoked.
pub fn confirm_succession(env: &Env, confirming_dao: Address) -> Result<(), SuccessionError> {
    // The new DAO must sign this transaction
    confirming_dao.require_auth();

    let record: SuccessionRecord = env
        .storage()
        .instance()
        .get(&SuccessionKey::PendingSuccession)
        .ok_or(SuccessionError::NoSuccessionPending)?;

    if record.proposed_dao != confirming_dao {
        return Err(SuccessionError::WrongConfirmingAddress);
    }

    // Transfer admin rights to the new DAO
    env.storage()
        .instance()
        .set(&SuccessionKey::Admin, &confirming_dao);

    // Remove pending record
    env.storage()
        .instance()
        .remove(&SuccessionKey::PendingSuccession);

    env.events().publish(
        (symbol_short!("suc_done"),),
        (record.current_admin, confirming_dao),
    );
    Ok(())
}

/// Cancel a pending succession. Only the current admin can cancel.
pub fn cancel_succession(env: &Env) -> Result<(), SuccessionError> {
    let current_admin = read_admin(env)?;
    current_admin.require_auth();

    if !env
        .storage()
        .instance()
        .has(&SuccessionKey::PendingSuccession)
    {
        return Err(SuccessionError::NoSuccessionPending);
    }

    env.storage()
        .instance()
        .remove(&SuccessionKey::PendingSuccession);

    env.events().publish(
        (symbol_short!("suc_cxl"),),
        (current_admin,),
    );
    Ok(())
}

/// Returns the current admin address.
pub fn get_admin(env: &Env) -> Result<Address, SuccessionError> {
    read_admin(env)
}

/// Returns the pending succession record, if any.
pub fn get_pending_succession(env: &Env) -> Option<SuccessionRecord> {
    env.storage()
        .instance()
        .get(&SuccessionKey::PendingSuccession)
}
