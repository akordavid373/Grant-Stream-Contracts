// ============================================================================
// FILE: contracts/grant_stream/src/reentrancy.rs
//
// Manual non-reentrant guard for Soroban cross-contract calls.
//
// Design decisions
// ────────────────
// • Uses `env.storage().temporary()` so the flag is scoped to the current
//   ledger sequence and costs the minimum possible ledger-entry rent.
//   Temporary storage entries expire automatically, so a flag left set by a
//   panicking transaction cannot permanently brick the contract.
// • A second, persistent `TemporaryGuard::Locked` entry (via `instance()`)
//   is NOT used — temporary storage is sufficient because Soroban transactions
//   are atomic: if the outer call panics the temp entry is rolled back.
// • The guard is zero-cost when the protected function completes normally:
//   `exit()` deletes the temporary entry rather than writing `false`, saving
//   one ledger write.
// ============================================================================

use soroban_sdk::{contracttype, panic_with_error, Env};

// ---------------------------------------------------------------------------
// Storage key
// ---------------------------------------------------------------------------

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum GuardKey {
    /// Single reentrancy lock shared across all protected entry-points.
    /// Using one key means a re-entrant call via *any* guarded function is
    /// blocked, not just re-entry into the same function.
    NonReentrant,
}

// ---------------------------------------------------------------------------
// Error code
// ---------------------------------------------------------------------------

/// Numeric error emitted when re-entry is detected.
/// Wire into your contract's top-level `#[contracterror]` enum with this
/// discriminant (or choose an unused one):
///
/// ```rust
/// #[contracterror]
/// #[derive(Copy, Clone, Debug, Eq, PartialEq)]
/// #[repr(u32)]
/// pub enum GrantStreamError {
///     // … existing variants …
///     Reentrant = 100,
/// }
/// ```
pub const REENTRANT_ERROR_CODE: u32 = 100;

// ---------------------------------------------------------------------------
// Guard implementation
// ---------------------------------------------------------------------------

/// Checks that no reentrant call is in progress, then sets the lock.
///
/// Call this as the **first** statement in every protected entry-point.
/// Pair with [`reentrancy_exit`] as the **last** statement (before returning
/// the value, or in every early-return path).
///
/// # Gas / ledger optimisation
/// The flag is stored in `temporary` storage so it:
///   - Costs the minimum ledger-entry rent (expires after the ledger sequence).
///   - Is automatically rolled back if the transaction panics, preventing
///     a permanently-locked contract.
///   - Is deleted (not overwritten with `false`) on exit, saving one write.
///
/// # Panics
/// Panics with `REENTRANT_ERROR_CODE` if the lock is already set.
#[inline]
pub fn reentrancy_enter(env: &Env) {
    let storage = env.storage().temporary();

    // Check ──────────────────────────────────────────────────────────────────
    if storage.has(&GuardKey::NonReentrant) {
        panic_with_error!(env, REENTRANT_ERROR_CODE);
    }

    // Lock ───────────────────────────────────────────────────────────────────
    // TTL of 1 ledger is enough: Soroban transactions are single-ledger.
    // The entry is deleted on normal exit via reentrancy_exit(); if the
    // transaction aborts, the entire write set is rolled back automatically.
    storage.set(&GuardKey::NonReentrant, &true);
    storage.extend_ttl(&GuardKey::NonReentrant, 0, 1);
}

/// Releases the reentrancy lock.
///
/// Must be called before every `return` in a guarded function.
/// Prefer [`nonreentrant!`] macro to avoid accidentally forgetting this call.
///
/// Deletes the temporary entry rather than writing `false` — this saves one
/// ledger write and keeps the ledger footprint at zero between calls.
#[inline]
pub fn reentrancy_exit(env: &Env) {
    env.storage().temporary().remove(&GuardKey::NonReentrant);
}

// ---------------------------------------------------------------------------
// Convenience macro — preferred over calling enter/exit manually
// ---------------------------------------------------------------------------

/// Wraps a block in an enter/exit reentrancy guard.
///
/// ```rust
/// use crate::reentrancy::nonreentrant;
///
/// pub fn claim_milestone_funds(env: Env, grant_id: u64) -> i128 {
///     nonreentrant!(env, {
///         // … all logic here …
///         transferred_amount
///     })
/// }
/// ```
///
/// The macro captures the block's return value, calls `reentrancy_exit`, then
/// returns it — so the lock is always released even on early returns within
/// the block.  Panics inside the block still roll back the temp storage entry
/// automatically (Soroban atomicity).
#[macro_export]
macro_rules! nonreentrant {
    ($env:expr, $body:block) => {{
        $crate::reentrancy::reentrancy_enter(&$env);
        let __result = (|| $body)();
        $crate::reentrancy::reentrancy_exit(&$env);
        __result
    }};
}