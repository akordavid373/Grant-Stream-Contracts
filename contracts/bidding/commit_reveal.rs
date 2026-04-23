#![no_std]
use soroban_sdk::{
    contract, contractimpl, contracttype,
    crypto::Hash,
    Address, Bytes, BytesN, Env, Map, String,
};

// ── Storage Keys ────────────────────────────────────────────────────────────

#[contracttype]
pub enum BidKey {
    Commitment(Address),   // grantee -> commitment hash
    Reveal(Address),       // grantee -> revealed bid
    BiddingOpen,           // bool
    RevealDeadline,        // u64 ledger timestamp
}

// ── Data Types ───────────────────────────────────────────────────────────────

#[contracttype]
#[derive(Clone)]
pub struct RevealedBid {
    pub grantee: Address,
    pub amount: u64,
    pub milestone_costs: Map<u32, u64>, // milestone_id -> cost
    pub salt: Bytes,                    // random salt used during commit
}

// ── Contract ─────────────────────────────────────────────────────────────────

#[contract]
pub struct CommitRevealContract;

#[contractimpl]
impl CommitRevealContract {

    /// Admin opens the bidding window.
    pub fn open_bidding(env: Env, admin: Address, reveal_deadline: u64) {
        admin.require_auth();
        env.storage().instance().set(&BidKey::BiddingOpen, &true);
        env.storage().instance().set(&BidKey::RevealDeadline, &reveal_deadline);
    }

    /// Admin closes the bidding window — no more commits accepted.
    pub fn close_bidding(env: Env, admin: Address) {
        admin.require_auth();
        env.storage().instance().set(&BidKey::BiddingOpen, &false);
    }

    /// Grantee submits SHA-256 hash of (amount + milestone_costs + salt).
    /// Commitment = SHA-256(amount || milestone_costs_encoded || salt)
    pub fn commit(env: Env, grantee: Address, commitment: BytesN<32>) {
        grantee.require_auth();

        let is_open: bool = env
            .storage()
            .instance()
            .get(&BidKey::BiddingOpen)
            .unwrap_or(false);

        if !is_open {
            panic!("Bidding window is closed");
        }

        // Prevent overwriting an existing commitment
        if env.storage().persistent().has(&BidKey::Commitment(grantee.clone())) {
            panic!("Commitment already submitted");
        }

        env.storage()
            .persistent()
            .set(&BidKey::Commitment(grantee), &commitment);
    }

    /// Grantee reveals their original bid after the bidding window closes.
    /// Contract verifies SHA-256(reveal) == stored commitment.
    pub fn reveal(env: Env, grantee: Address, bid: RevealedBid) {
        grantee.require_auth();

        // Bidding must be closed before reveals are accepted
        let is_open: bool = env
            .storage()
            .instance()
            .get(&BidKey::BiddingOpen)
            .unwrap_or(false);
        if is_open {
            panic!("Bidding window must be closed before revealing");
        }

        // Reveal deadline must not have passed
        let deadline: u64 = env
            .storage()
            .instance()
            .get(&BidKey::RevealDeadline)
            .unwrap_or(0);
        if env.ledger().timestamp() > deadline {
            panic!("Reveal window has expired");
        }

        // Retrieve stored commitment
        let stored_commitment: BytesN<32> = env
            .storage()
            .persistent()
            .get(&BidKey::Commitment(grantee.clone()))
            .expect("No commitment found for this grantee");

        // Re-hash the revealed bid and compare
        let computed_hash = Self::hash_bid(&env, &bid);
        if computed_hash != stored_commitment {
            panic!("Revealed bid does not match commitment — possible front-running attempt");
        }

        // Store the verified reveal
        env.storage()
            .persistent()
            .set(&BidKey::Reveal(grantee), &bid);
    }

    /// Read a verified revealed bid (only after reveal phase).
    pub fn get_revealed_bid(env: Env, grantee: Address) -> RevealedBid {
        env.storage()
            .persistent()
            .get(&BidKey::Reveal(grantee))
            .expect("No verified reveal found")
    }

    /// Get the raw commitment hash for a grantee.
    pub fn get_commitment(env: Env, grantee: Address) -> BytesN<32> {
        env.storage()
            .persistent()
            .get(&BidKey::Commitment(grantee))
            .expect("No commitment found")
    }

    // ── Internal ─────────────────────────────────────────────────────────────

    /// Deterministically encode and SHA-256 hash a RevealedBid.
    /// Encoding: amount (8 bytes BE) || each (milestone_id 4B + cost 8B) || salt
    fn hash_bid(env: &Env, bid: &RevealedBid) -> BytesN<32> {
        let mut preimage = Bytes::new(env);

        // Encode amount as 8 big-endian bytes
        preimage.append(&Bytes::from_array(env, &bid.amount.to_be_bytes()));

        // Encode each milestone cost deterministically (sorted by id)
        let mut ids: soroban_sdk::Vec<u32> = soroban_sdk::Vec::new(env);
        for key in bid.milestone_costs.keys() {
            ids.push_back(key);
        }
        // Sort ascending for determinism
        let len = ids.len();
        for i in 0..len {
            for j in 0..len - i - 1 {
                if ids.get(j).unwrap() > ids.get(j + 1).unwrap() {
                    let a = ids.get(j).unwrap();
                    let b = ids.get(j + 1).unwrap();
                    ids.set(j, b);
                    ids.set(j + 1, a);
                }
            }
        }
        for id in ids.iter() {
            preimage.append(&Bytes::from_array(env, &id.to_be_bytes()));
            let cost = bid.milestone_costs.get(id).unwrap();
            preimage.append(&Bytes::from_array(env, &cost.to_be_bytes()));
        }

        // Append salt
        preimage.append(&bid.salt);

        env.crypto().sha256(&preimage)
    }
}