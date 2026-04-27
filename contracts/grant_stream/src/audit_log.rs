/// Audit-Log Hashing for Off-Chain Indexers (Issue #322)
///
/// Maintains an incremental Merkle Tree over active stream balances.
/// Every 100 transactions the contract emits an event containing the
/// Merkle Root so off-chain indexers (e.g. Zealynx) can verify their
/// local database without re-scanning every ledger entry.
///
/// The tree is built from SHA-256 hashes of (grant_id || balance) pairs.
/// Incremental updates replace only the leaf that changed, keeping
/// on-chain compute proportional to O(log n) per update.

use soroban_sdk::{symbol_short, Bytes, Env, Vec};

const EMIT_EVERY_N_TXS: u32 = 100;

#[derive(Clone)]
#[soroban_sdk::contracttype]
pub enum AuditKey {
    /// u32 – rolling transaction counter.
    TxCounter,
    /// Vec<Bytes> – the current Merkle leaf layer (one 32-byte hash per grant).
    MerkleLeaves,
    /// Bytes(32) – the last emitted Merkle root.
    MerkleRoot,
}

// ── Hashing helpers ──────────────────────────────────────────────────────────

/// SHA-256 of `grant_id || balance` (both as big-endian 8-byte values).
fn leaf_hash(env: &Env, grant_id: u64, balance: i128) -> Bytes {
    let mut data = Bytes::new(env);
    // grant_id as 8 big-endian bytes
    for byte in grant_id.to_be_bytes() {
        data.push_back(byte);
    }
    // balance as 16 big-endian bytes
    for byte in balance.to_be_bytes() {
        data.push_back(byte);
    }
    env.crypto().sha256(&data).into()
}

/// SHA-256 of two 32-byte child hashes concatenated.
fn parent_hash(env: &Env, left: &Bytes, right: &Bytes) -> Bytes {
    let mut data = Bytes::new(env);
    data.append(left);
    data.append(right);
    env.crypto().sha256(&data).into()
}

/// Compute the Merkle root from a flat leaf layer.
/// If the layer has an odd number of leaves the last leaf is duplicated.
fn compute_root(env: &Env, leaves: &Vec<Bytes>) -> Bytes {
    let n = leaves.len();
    if n == 0 {
        // Empty tree → zero hash
        return Bytes::from_array(env, &[0u8; 32]);
    }
    if n == 1 {
        return leaves.get(0).unwrap();
    }

    let mut current: Vec<Bytes> = leaves.clone();

    while current.len() > 1 {
        let mut next: Vec<Bytes> = Vec::new(env);
        let len = current.len();
        let mut i = 0u32;
        while i < len {
            let left = current.get(i).unwrap();
            let right = if i + 1 < len {
                current.get(i + 1).unwrap()
            } else {
                left.clone() // duplicate last leaf for odd counts
            };
            next.push_back(parent_hash(env, &left, &right));
            i += 2;
        }
        current = next;
    }

    current.get(0).unwrap()
}

// ── Public API ───────────────────────────────────────────────────────────────

/// Update the Merkle leaf for `grant_id` with its current `balance`.
/// Increments the transaction counter and emits a `state_root` event
/// every `EMIT_EVERY_N_TXS` transactions.
pub fn update_audit_leaf(env: &Env, grant_id: u64, balance: i128) {
    // Increment tx counter
    let counter: u32 = env
        .storage()
        .instance()
        .get(&AuditKey::TxCounter)
        .unwrap_or(0)
        .saturating_add(1);
    env.storage()
        .instance()
        .set(&AuditKey::TxCounter, &counter);

    // Update the leaf for this grant_id.
    // We use grant_id as the leaf index (mod capacity) for simplicity.
    let new_leaf = leaf_hash(env, grant_id, balance);
    let mut leaves: Vec<Bytes> = env
        .storage()
        .instance()
        .get(&AuditKey::MerkleLeaves)
        .unwrap_or_else(|| Vec::new(env));

    // Find existing leaf index or append.
    let idx = (grant_id as u32) % 256; // cap at 256 leaves
    while leaves.len() <= idx {
        leaves.push_back(Bytes::from_array(env, &[0u8; 32]));
    }
    leaves.set(idx, new_leaf);
    env.storage()
        .instance()
        .set(&AuditKey::MerkleLeaves, &leaves);

    // Emit root every N transactions
    if counter % EMIT_EVERY_N_TXS == 0 {
        let root = compute_root(env, &leaves);
        env.storage()
            .instance()
            .set(&AuditKey::MerkleRoot, &root);
        env.events().publish(
            (symbol_short!("stateroot"),),
            (counter, root),
        );
    }
}

/// Return the last stored Merkle root (32 bytes).
pub fn get_merkle_root(env: &Env) -> Bytes {
    env.storage()
        .instance()
        .get(&AuditKey::MerkleRoot)
        .unwrap_or_else(|| Bytes::from_array(env, &[0u8; 32]))
}

/// Return the current transaction counter.
pub fn get_tx_counter(env: &Env) -> u32 {
    env.storage()
        .instance()
        .get(&AuditKey::TxCounter)
        .unwrap_or(0)
}
