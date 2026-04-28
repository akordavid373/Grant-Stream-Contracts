// contracts/grant_stream/src/lib.rs

#![no_std]
use soroban_sdk::{contract, contractimpl, contracttype, token, Address, Env, Symbol, BytesN, Vec};

#[contracttype]
#[derive(Clone)]
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
    pub required_approvals: u32,   // N in N-of-M for this milestone
    pub is_approved: bool,
    pub is_released: bool,
    pub deadline: u64,
}

#[contracttype]
#[derive(Clone)]
pub struct Grant {
    pub recipient: Address,
    pub sponsor: Address,
    pub asset: Address,                    // Regulated asset (can be frozen)
    pub total_amount: i128,
    pub released_amount: i128,
    pub stream_cap_per_ledger: i128,       // From #421
    pub milestones: Vec<Milestone>,
    pub reviewers: Vec<Address>,
    pub required_approvals: u32,           // Global N-of-M (can be overridden per milestone)
    pub is_cancelled: bool,
    pub is_frozen: bool,                   // From #422
    pub freeze_reason: Option<Symbol>,
    pub last_checked_ledger: u32,
    pub payment_status: PaymentStatus,
}

#[contract]
pub struct GrantStream;

#[contractimpl]
impl GrantStream {
    pub fn create_grant(
        env: Env,
        recipient: Address,
        asset: Address,
        total_amount: i128,
        stream_cap_per_ledger: i128,
        reviewers: Vec<Address>,
        required_approvals: u32,
    ) {
        assert!(stream_cap_per_ledger > 0 && stream_cap_per_ledger <= total_amount, "Invalid stream cap");
        assert!(reviewers.len() >= required_approvals as u32 && required_approvals > 0, "Invalid N-of-M");

        let grant = Grant {
            recipient,
            sponsor: env.current_contract_address(),
            asset,
            total_amount,
            released_amount: 0,
            stream_cap_per_ledger,
            milestones: Vec::new(&env),
            reviewers,
            required_approvals,
            is_cancelled: false,
            is_frozen: false,
            freeze_reason: None,
            last_checked_ledger: env.ledger().sequence(),
            payment_status: PaymentStatus::Active,
        };

        // Save grant logic (use storage map with grant_id)
        let grant_id = Self::increment_grant_count(&env);
        env.storage().instance().set(&grant_id, &grant);

        env.events().publish(
            (Symbol::new(&env, "grant_created"), grant_id),
            (recipient, total_amount, stream_cap_per_ledger),
        );
    }

    pub fn release_milestone(
        env: Env,
        grant_id: u32,
        milestone_index: u32,
        deliverable_proof: BytesN<32>,
    ) {
        let mut grant: Grant = env.storage().instance().get(&grant_id).unwrap();

        // ====================== ASSET FREEZE CHECK (SEP-8) ======================
        if grant.is_frozen || Self::is_asset_frozen(&env, &grant.asset, &grant.recipient) {
            grant.is_frozen = true;
            grant.freeze_reason = Some(Symbol::new(&env, "ASSET_FROZEN"));
            grant.payment_status = PaymentStatus::PausedDueToFreeze;
            env.storage().instance().set(&grant_id, &grant);
            panic_with_error!(env, ContractError::AssetFrozen);
        }

        // ====================== STREAM CAP CHECK (#421) ======================
        let milestone = grant.milestones.get(milestone_index).unwrap();
        if milestone.amount > grant.stream_cap_per_ledger {
            panic_with_error!(env, ContractError::ExceedsStreamCap);
        }

        // ====================== N-OF-M CONSENSUS CHECK (#420) ======================
        if !milestone.is_approved {
            panic_with_error!(env, ContractError::MilestoneNotApproved);
        }

        require!(!milestone.is_released, "Milestone already released");
        require!(!grant.is_cancelled, "Grant is cancelled");

        // Perform transfer
        let token_client = token::Client::new(&env, &grant.asset);
        token_client.transfer(
            &env.current_contract_address(),
            &grant.recipient,
            &milestone.amount,
        );

        // Update state
        let mut updated_milestone = milestone.clone();
        updated_milestone.is_released = true;

        grant.released_amount += milestone.amount;
        grant.milestones.set(milestone_index, updated_milestone);

        env.storage().instance().set(&grant_id, &grant);

        env.events().publish(
            (Symbol::new(&env, "milestone_released"), grant_id, milestone_index),
            milestone.amount,
        );
    }

    fn is_asset_frozen(env: &Env, asset: &Address, recipient: &Address) -> bool {
        let token_client = token::Client::new(env, asset);
        !token_client.authorized(recipient)   // Stellar native freeze check
    }

    pub fn freeze_grant(env: Env, grant_id: u32, reason: Symbol) {
        // Access control: only admin or compliance
        let mut grant: Grant = env.storage().instance().get(&grant_id).unwrap();
        grant.is_frozen = true;
        grant.freeze_reason = Some(reason);
        grant.payment_status = PaymentStatus::PausedDueToFreeze;
        env.storage().instance().set(&grant_id, &grant);

        env.events().publish((Symbol::new(&env, "grant_frozen"), grant_id), ());
    }

    pub fn unfreeze_grant(env: Env, grant_id: u32) {
        let mut grant: Grant = env.storage().instance().get(&grant_id).unwrap();
        grant.is_frozen = false;
        grant.freeze_reason = None;
        grant.payment_status = PaymentStatus::Active;
        env.storage().instance().set(&grant_id, &grant);

        env.events().publish((Symbol::new(&env, "grant_unfrozen"), grant_id), ());
    }
}