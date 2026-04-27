//! Upgrade-Manager for Seamless Protocol Evolution (Issue #317)
//!
//! Provides a secure `upgrade_contract` function gated by:
//!   - A 14-day timelock from proposal to execution.
//!   - A 75% DAO supermajority vote.
//!
//! State migration metadata is recorded on-chain so that all existing streams,
//! balances, and milestone hashes can be mapped to the new contract version.

#![allow(unused)]

use soroban_sdk::{
    contracttype, contracterror, symbol_short, Address, Bytes, BytesN, Env, String, Vec,
};

/// Timelock duration: 14 days in seconds.
pub const UPGRADE_TIMELOCK_SECS: u64 = 14 * 24 * 60 * 60;
/// Required DAO supermajority in basis points (7500 = 75%).
pub const UPGRADE_MAJORITY_BPS: u32 = 7_500;
/// Basis points denominator.
pub const BPS_DENOM: u32 = 10_000;

/// Status of an upgrade proposal.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[contracttype]
pub enum UpgradeStatus {
    /// Proposal created; voting open.
    Proposed,
    /// Voting passed; waiting for timelock to expire.
    Approved,
    /// Timelock expired; upgrade executed.
    Executed,
    /// Proposal rejected or cancelled.
    Cancelled,
}

/// An upgrade proposal.
#[derive(Clone, Debug)]
#[contracttype]
pub struct UpgradeProposal {
    pub proposal_id: u64,
    pub proposer: Address,
    /// SHA-256 / WASM hash of the new contract bytecode.
    pub new_wasm_hash: BytesN<32>,
    /// Human-readable description of changes.
    pub description: String,
    pub proposed_at: u64,
    /// Earliest timestamp at which the upgrade can be executed (proposed_at + 14 days).
    pub executable_after: u64,
    pub status: UpgradeStatus,
    pub votes_for: i128,
    pub votes_against: i128,
    pub total_voting_power: i128,
    pub voting_deadline: u64,
}

/// Migration manifest recorded on-chain for auditability.
#[derive(Clone, Debug)]
#[contracttype]
pub struct MigrationManifest {
    pub proposal_id: u64,
    pub old_wasm_hash: BytesN<32>,
    pub new_wasm_hash: BytesN<32>,
    pub migrated_at: u64,
    /// Number of active grants at migration time.
    pub active_grants: u32,
    /// Total locked balance at migration time.
    pub total_locked: i128,
    /// Milestone claim IDs preserved.
    pub milestone_claim_ids: Vec<u64>,
}

#[derive(Clone)]
#[contracttype]
pub enum UpgradeKey {
    /// Next proposal ID counter.
    NextProposalId,
    /// Maps proposal_id -> UpgradeProposal.
    Proposal(u64),
    /// List of all proposal IDs.
    ProposalIds,
    /// Maps (proposal_id, voter) -> bool (has voted).
    Vote(u64, Address),
    /// Latest migration manifest.
    LatestMigration,
}

#[contracterror]
#[derive(Clone, Copy, Eq, PartialEq, Debug)]
#[repr(u32)]
pub enum UpgradeError {
    NotInitialized = 1,
    NotAuthorized = 2,
    ProposalNotFound = 3,
    VotingEnded = 4,
    VotingStillActive = 5,
    AlreadyVoted = 6,
    TimelockNotExpired = 7,
    MajorityNotReached = 8,
    ProposalNotApproved = 9,
    MathOverflow = 10,
    InvalidVotingPower = 11,
}

// ── Internal helpers ──────────────────────────────────────────────────────────

fn next_proposal_id(env: &Env) -> u64 {
    let id: u64 = env
        .storage()
        .instance()
        .get(&UpgradeKey::NextProposalId)
        .unwrap_or(1);
    env.storage()
        .instance()
        .set(&UpgradeKey::NextProposalId, &(id + 1));
    id
}

fn read_proposal(env: &Env, proposal_id: u64) -> Result<UpgradeProposal, UpgradeError> {
    env.storage()
        .instance()
        .get(&UpgradeKey::Proposal(proposal_id))
        .ok_or(UpgradeError::ProposalNotFound)
}

fn write_proposal(env: &Env, proposal: &UpgradeProposal) {
    env.storage()
        .instance()
        .set(&UpgradeKey::Proposal(proposal.proposal_id), proposal);
}

// ── Public API ────────────────────────────────────────────────────────────────

/// Propose a contract upgrade. Any DAO member with voting power can propose.
/// `voting_period_secs` is how long voting stays open before the timelock starts.
pub fn propose_upgrade(
    env: &Env,
    proposer: Address,
    new_wasm_hash: BytesN<32>,
    description: String,
    total_voting_power: i128,
    voting_period_secs: u64,
) -> Result<u64, UpgradeError> {
    proposer.require_auth();
    if total_voting_power <= 0 {
        return Err(UpgradeError::InvalidVotingPower);
    }

    let now = env.ledger().timestamp();
    let proposal_id = next_proposal_id(env);
    let voting_deadline = now + voting_period_secs;

    let proposal = UpgradeProposal {
        proposal_id,
        proposer: proposer.clone(),
        new_wasm_hash: new_wasm_hash.clone(),
        description,
        proposed_at: now,
        executable_after: voting_deadline + UPGRADE_TIMELOCK_SECS,
        status: UpgradeStatus::Proposed,
        votes_for: 0,
        votes_against: 0,
        total_voting_power,
        voting_deadline,
    };
    write_proposal(env, &proposal);

    // Track proposal IDs
    let mut ids: Vec<u64> = env
        .storage()
        .instance()
        .get(&UpgradeKey::ProposalIds)
        .unwrap_or_else(|| Vec::new(env));
    ids.push_back(proposal_id);
    env.storage()
        .instance()
        .set(&UpgradeKey::ProposalIds, &ids);

    env.events().publish(
        (symbol_short!("upg_prop"), proposal_id),
        (proposer, new_wasm_hash, voting_deadline),
    );
    Ok(proposal_id)
}

/// Cast a vote on an upgrade proposal.
/// `voting_power` is the voter's token-weighted power.
pub fn vote_on_upgrade(
    env: &Env,
    proposal_id: u64,
    voter: Address,
    approve: bool,
    voting_power: i128,
) -> Result<(), UpgradeError> {
    voter.require_auth();
    if voting_power <= 0 {
        return Err(UpgradeError::InvalidVotingPower);
    }

    let mut proposal = read_proposal(env, proposal_id)?;
    let now = env.ledger().timestamp();

    if now > proposal.voting_deadline {
        return Err(UpgradeError::VotingEnded);
    }

    let vote_key = UpgradeKey::Vote(proposal_id, voter.clone());
    if env.storage().instance().has(&vote_key) {
        return Err(UpgradeError::AlreadyVoted);
    }
    env.storage().instance().set(&vote_key, &true);

    if approve {
        proposal.votes_for = proposal
            .votes_for
            .checked_add(voting_power)
            .ok_or(UpgradeError::MathOverflow)?;
    } else {
        proposal.votes_against = proposal
            .votes_against
            .checked_add(voting_power)
            .ok_or(UpgradeError::MathOverflow)?;
    }
    write_proposal(env, &proposal);

    env.events().publish(
        (symbol_short!("upg_vote"), proposal_id),
        (voter, approve, voting_power),
    );
    Ok(())
}

/// Finalize voting: mark proposal as Approved if 75% supermajority reached.
/// Must be called after voting_deadline has passed.
pub fn finalize_vote(env: &Env, proposal_id: u64) -> Result<(), UpgradeError> {
    let mut proposal = read_proposal(env, proposal_id)?;
    let now = env.ledger().timestamp();

    if now <= proposal.voting_deadline {
        return Err(UpgradeError::VotingStillActive);
    }
    if proposal.status != UpgradeStatus::Proposed {
        return Err(UpgradeError::ProposalNotFound);
    }

    let total_cast = proposal.votes_for + proposal.votes_against;
    let majority_met = if total_cast > 0 {
        // votes_for / total_cast >= 75%
        (proposal.votes_for as u128 * BPS_DENOM as u128)
            >= (total_cast as u128 * UPGRADE_MAJORITY_BPS as u128)
    } else {
        false
    };

    proposal.status = if majority_met {
        UpgradeStatus::Approved
    } else {
        UpgradeStatus::Cancelled
    };
    write_proposal(env, &proposal);

    env.events().publish(
        (symbol_short!("upg_fin"), proposal_id),
        (majority_met, proposal.votes_for, proposal.votes_against),
    );
    Ok(())
}

/// Execute the upgrade after the 14-day timelock has expired.
/// Records a MigrationManifest for state auditability.
/// The actual WASM upgrade (`env.deployer().update_current_contract_wasm`) must
/// be called by the host after this function succeeds.
pub fn execute_upgrade(
    env: &Env,
    proposal_id: u64,
    admin: Address,
    active_grants: u32,
    total_locked: i128,
    milestone_claim_ids: Vec<u64>,
    old_wasm_hash: BytesN<32>,
) -> Result<BytesN<32>, UpgradeError> {
    admin.require_auth();

    let mut proposal = read_proposal(env, proposal_id)?;
    let now = env.ledger().timestamp();

    if proposal.status != UpgradeStatus::Approved {
        return Err(UpgradeError::ProposalNotApproved);
    }
    if now < proposal.executable_after {
        return Err(UpgradeError::TimelockNotExpired);
    }

    proposal.status = UpgradeStatus::Executed;
    write_proposal(env, &proposal);

    // Record migration manifest for auditability
    let manifest = MigrationManifest {
        proposal_id,
        old_wasm_hash: old_wasm_hash.clone(),
        new_wasm_hash: proposal.new_wasm_hash.clone(),
        migrated_at: now,
        active_grants,
        total_locked,
        milestone_claim_ids,
    };
    env.storage()
        .instance()
        .set(&UpgradeKey::LatestMigration, &manifest);

    env.events().publish(
        (symbol_short!("upg_exec"), proposal_id),
        (old_wasm_hash, proposal.new_wasm_hash.clone(), now),
    );

    Ok(proposal.new_wasm_hash)
}

/// Returns an upgrade proposal by ID.
pub fn get_proposal(env: &Env, proposal_id: u64) -> Option<UpgradeProposal> {
    env.storage()
        .instance()
        .get(&UpgradeKey::Proposal(proposal_id))
}

/// Returns the latest migration manifest.
pub fn get_latest_migration(env: &Env) -> Option<MigrationManifest> {
    env.storage()
        .instance()
        .get(&UpgradeKey::LatestMigration)
}
