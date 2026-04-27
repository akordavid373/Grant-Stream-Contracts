/// Multi-Threshold Signature Logic for Treasury Rescues (Issue #321)
///
/// Implements a tiered recovery system:
///   - Standard operations  : 3-of-5 signers required
///   - Emergency Migration  : 7-of-10 signers required
///
/// Workflow:
///   1. Any registered signer calls `propose_rescue` to open a proposal.
///   2. Other signers call `approve_rescue` to add their signature.
///   3. Once the threshold is met, `execute_rescue` can be called to
///      transfer funds to the designated rescue address.

use soroban_sdk::{symbol_short, Address, Bytes, Env, Vec};

// ── Thresholds ────────────────────────────────────────────────────────────────

pub const STANDARD_THRESHOLD: u32 = 3;
pub const STANDARD_SIGNERS: u32 = 5;
pub const EMERGENCY_THRESHOLD: u32 = 7;
pub const EMERGENCY_SIGNERS: u32 = 10;

// ── Types ─────────────────────────────────────────────────────────────────────

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[soroban_sdk::contracttype]
pub enum RescueKind {
    /// Standard 3-of-5 operation.
    Standard,
    /// Emergency 7-of-10 treasury migration.
    Emergency,
}

#[derive(Clone, Debug, Eq, PartialEq)]
#[soroban_sdk::contracttype]
pub enum RescueStatus {
    Pending,
    Executed,
    Cancelled,
}

#[derive(Clone)]
#[soroban_sdk::contracttype]
pub struct RescueProposal {
    pub id: u64,
    pub kind: RescueKind,
    pub proposer: Address,
    /// Destination address for the rescued funds.
    pub rescue_to: Address,
    /// Amount to transfer.
    pub amount: i128,
    /// Addresses that have approved this proposal.
    pub approvals: Vec<Address>,
    pub status: RescueStatus,
    pub created_at: u64,
}

#[derive(Clone)]
#[soroban_sdk::contracttype]
pub enum RescueKey {
    /// Vec<Address> – the registered signer set (up to EMERGENCY_SIGNERS).
    Signers,
    /// RescueProposal keyed by proposal id.
    Proposal(u64),
    /// u64 – monotonically increasing proposal counter.
    ProposalCounter,
}

// ── Helpers ───────────────────────────────────────────────────────────────────

fn read_signers(env: &Env) -> Vec<Address> {
    env.storage()
        .instance()
        .get(&RescueKey::Signers)
        .unwrap_or_else(|| Vec::new(env))
}

fn is_signer(env: &Env, addr: &Address) -> bool {
    let signers = read_signers(env);
    for i in 0..signers.len() {
        if signers.get(i).unwrap() == *addr {
            return true;
        }
    }
    false
}

fn threshold_for(kind: RescueKind) -> u32 {
    match kind {
        RescueKind::Standard => STANDARD_THRESHOLD,
        RescueKind::Emergency => EMERGENCY_THRESHOLD,
    }
}

fn next_proposal_id(env: &Env) -> u64 {
    let id: u64 = env
        .storage()
        .instance()
        .get(&RescueKey::ProposalCounter)
        .unwrap_or(0)
        .saturating_add(1);
    env.storage()
        .instance()
        .set(&RescueKey::ProposalCounter, &id);
    id
}

// ── Public API ────────────────────────────────────────────────────────────────

/// Register the signer set.  Must be called once during contract setup.
/// `signers` must contain exactly `EMERGENCY_SIGNERS` (10) addresses.
pub fn initialize_signers(env: &Env, signers: Vec<Address>) {
    env.storage()
        .instance()
        .set(&RescueKey::Signers, &signers);
}

/// Open a new rescue proposal.  The caller must be a registered signer.
/// Returns the new proposal id.
pub fn propose_rescue(
    env: &Env,
    proposer: Address,
    kind: RescueKind,
    rescue_to: Address,
    amount: i128,
) -> u64 {
    proposer.require_auth();
    assert!(is_signer(env, &proposer), "not a registered signer");
    assert!(amount > 0, "amount must be positive");

    let id = next_proposal_id(env);
    let mut approvals: Vec<Address> = Vec::new(env);
    approvals.push_back(proposer.clone()); // proposer auto-approves

    let proposal = RescueProposal {
        id,
        kind,
        proposer: proposer.clone(),
        rescue_to: rescue_to.clone(),
        amount,
        approvals,
        status: RescueStatus::Pending,
        created_at: env.ledger().timestamp(),
    };

    env.storage()
        .instance()
        .set(&RescueKey::Proposal(id), &proposal);

    env.events().publish(
        (symbol_short!("rescprop"), proposer),
        (id, kind, rescue_to, amount),
    );

    id
}

/// Add an approval to an existing pending proposal.
/// The caller must be a registered signer who has not already approved.
pub fn approve_rescue(env: &Env, signer: Address, proposal_id: u64) {
    signer.require_auth();
    assert!(is_signer(env, &signer), "not a registered signer");

    let mut proposal: RescueProposal = env
        .storage()
        .instance()
        .get(&RescueKey::Proposal(proposal_id))
        .expect("proposal not found");

    assert!(
        proposal.status == RescueStatus::Pending,
        "proposal not pending"
    );

    // Prevent duplicate approvals
    for i in 0..proposal.approvals.len() {
        assert!(
            proposal.approvals.get(i).unwrap() != signer,
            "already approved"
        );
    }

    proposal.approvals.push_back(signer.clone());

    env.storage()
        .instance()
        .set(&RescueKey::Proposal(proposal_id), &proposal);

    env.events().publish(
        (symbol_short!("rescappr"), signer),
        (proposal_id, proposal.approvals.len()),
    );
}

/// Execute a rescue proposal once the required threshold is met.
/// Returns the amount transferred.
///
/// The caller must be a registered signer.  Actual token transfer is
/// delegated to the contract caller via the returned amount – the
/// contract's `rescue_funds` entry-point should perform the transfer.
pub fn execute_rescue(env: &Env, caller: Address, proposal_id: u64) -> (Address, i128) {
    caller.require_auth();
    assert!(is_signer(env, &caller), "not a registered signer");

    let mut proposal: RescueProposal = env
        .storage()
        .instance()
        .get(&RescueKey::Proposal(proposal_id))
        .expect("proposal not found");

    assert!(
        proposal.status == RescueStatus::Pending,
        "proposal not pending"
    );

    let required = threshold_for(proposal.kind);
    assert!(
        proposal.approvals.len() >= required,
        "insufficient approvals"
    );

    proposal.status = RescueStatus::Executed;
    env.storage()
        .instance()
        .set(&RescueKey::Proposal(proposal_id), &proposal);

    env.events().publish(
        (symbol_short!("rescexec"), caller),
        (proposal_id, proposal.kind, proposal.rescue_to.clone(), proposal.amount),
    );

    (proposal.rescue_to, proposal.amount)
}

/// Cancel a pending proposal.  Only the original proposer may cancel.
pub fn cancel_rescue(env: &Env, proposer: Address, proposal_id: u64) {
    proposer.require_auth();

    let mut proposal: RescueProposal = env
        .storage()
        .instance()
        .get(&RescueKey::Proposal(proposal_id))
        .expect("proposal not found");

    assert!(proposal.proposer == proposer, "not the proposer");
    assert!(
        proposal.status == RescueStatus::Pending,
        "proposal not pending"
    );

    proposal.status = RescueStatus::Cancelled;
    env.storage()
        .instance()
        .set(&RescueKey::Proposal(proposal_id), &proposal);

    env.events().publish(
        (symbol_short!("resccanc"), proposer),
        proposal_id,
    );
}

/// Return a proposal by id.
pub fn get_proposal(env: &Env, proposal_id: u64) -> Option<RescueProposal> {
    env.storage()
        .instance()
        .get(&RescueKey::Proposal(proposal_id))
}

/// Return the current approval count for a proposal.
pub fn approval_count(env: &Env, proposal_id: u64) -> u32 {
    let proposal: Option<RescueProposal> = env
        .storage()
        .instance()
        .get(&RescueKey::Proposal(proposal_id));
    proposal.map(|p| p.approvals.len()).unwrap_or(0)
}
