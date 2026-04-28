#![no_std]
use soroban_sdk::{
    contract, contractimpl, contracttype, token, Address, Env, Symbol, BytesN, Vec, Map,
    panic_with_error,
};

use crate::error::ContractError;

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PaymentStatus {
    Active,
    PausedDueToFreeze,
    Cancelled,
    Completed,
}

#[contracttype]
#[derive(Clone)]
pub struct Milestone {
    pub amount: i128,
    pub description: Symbol,
    pub deliverable_hash: BytesN<32>,
    pub approvals_received: u32,
    pub required_approvals: u32,
    pub is_approved: bool,
    pub is_released: bool,
    pub start_time: u64,
    pub deadline: u64,
}

#[contracttype]
#[derive(Clone)]
pub struct Grant {
    pub recipient: Address,
    pub sponsor: Address,
    pub asset: Address,
    pub total_amount: i128,
    pub released_amount: i128,
    pub stream_cap_per_ledger: i128,
    pub milestones: Vec<Milestone>,
    pub reviewers: Vec<Address>,
    pub required_approvals: u32,
    pub is_cancelled: bool,
    pub is_frozen: bool,
    pub freeze_reason: Option<Symbol>,
    pub start_time: u64,
    pub end_time: u64,                    // For time-based prorated calculation
    pub payment_status: PaymentStatus,
    pub last_settled_ledger: u32,
}

#[contracttype]
pub enum DataKey {
    Grant(u32),
    GrantCount,
}

#[contract]
pub struct GrantStream;

#[contractimpl]
impl GrantStream {

    // ====================== CANCEL STREAM WITH AUTOMATIC PRORATED PAYOUT ======================

    pub fn cancel_stream(env: Env, grant_id: u32) -> Result<i128, ContractError> {
        let mut grant: Grant = env.storage()
            .instance()
            .get(&DataKey::Grant(grant_id))
            .ok_or(ContractError::GrantNotFound)?;

        require!(!grant.is_cancelled, ContractError::AlreadyCancelled);

        let current_time = env.ledger().timestamp();
        let prorated_amount = Self::calculate_prorated_amount(&grant, current_time);

        // Prevent over-payment
        let payable_amount = prorated_amount.min(grant.total_amount - grant.released_amount);

        if payable_amount > 0 {
            // Transfer the prorated amount to recipient
            let token_client = token::Client::new(&env, &grant.asset);
            token_client.transfer(
                &env.current_contract_address(),
                &grant.recipient,
                &payable_amount,
            );

            grant.released_amount += payable_amount;

            env.events().publish(
                (Symbol::new(&env, "prorated_payout"), grant_id),
                (payable_amount, current_time),
            );
        }

        // Mark as cancelled
        grant.is_cancelled = true;
        grant.payment_status = PaymentStatus::Cancelled;
        grant.end_time = current_time;

        env.storage().instance().set(&DataKey::Grant(grant_id), &grant);

        env.events().publish(
            (Symbol::new(&env, "stream_cancelled"), grant_id),
            (payable_amount, current_time),
        );

        Ok(payable_amount)
    }

    /// Calculates the prorated amount based on time elapsed
    fn calculate_prorated_amount(grant: &Grant, current_time: u64) -> i128 {
        if grant.end_time > 0 && current_time >= grant.end_time {
            return 0;
        }

        let total_duration = grant.end_time - grant.start_time;
        if total_duration == 0 {
            return 0;
        }

        let elapsed = current_time.saturating_sub(grant.start_time);

        // Prorated = (elapsed / total_duration) * (total_amount - released_amount)
        let remaining = grant.total_amount - grant.released_amount;
        
        if elapsed >= total_duration {
            remaining
        } else {
            (remaining * elapsed as i128) / total_duration as i128
        }
    }

    // ====================== EXISTING FUNCTIONS (with integration) ======================

    pub fn release_milestone(...) { /* your existing logic with freeze + stream cap checks */ }

    pub fn freeze_grant(...) { /* from #422 */ }

    pub fn unfreeze_grant(...) { /* from #422 */ }

    // Admin can cancel with manual override (optional)
    pub fn force_cancel_stream(env: Env, grant_id: u32) -> Result<i128, ContractError> {
        // Only admin / sponsor
        // ... access control
        Self::cancel_stream(env, grant_id)
    }
}