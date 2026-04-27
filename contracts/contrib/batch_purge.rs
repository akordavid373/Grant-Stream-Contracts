use soroban_sdk::{Env, Address, panic};

#[derive(Clone)]
pub struct ArchivedGrant {
    pub grant_id: String,
    pub final_state_hash: String,
    pub purge_timestamp: u64,
    pub purged_by: Address,
}

pub fn batch_purge(env: &Env) {
    let caller = env.invoker();
    let now = env.ledger().timestamp();

    // Retrieve list of grants
    let grants: Option<Vec<String>> = env.storage().get("grants");
    if grants.is_none() {
        return;
    }

    let mut purged_count = 0;

    for grant_id in grants.unwrap().iter() {
        // Check grant metadata
        let completed_ts: Option<u64> = env.storage().get(&format!("grant:{}:completed_ts", grant_id));
        let cancelled_ts: Option<u64> = env.storage().get(&format!("grant:{}:cancelled_ts", grant_id));

        let expiry_ts = completed_ts.or(cancelled_ts);
        if let Some(ts) = expiry_ts {
            if now > ts + 90 * 24 * 60 * 60 {
                // Archive final state hash
                let final_hash: String = env.storage().get(&format!("grant:{}:final_hash", grant_id)).unwrap_or("".to_string());
                let archive = ArchivedGrant {
                    grant_id: grant_id.clone(),
                    final_state_hash: final_hash,
                    purge_timestamp: now,
                    purged_by: caller.clone(),
                };
                env.storage().set(&format!("archived:{}", grant_id), &archive);

                // Delete granular milestone data
                env.storage().remove(&format!("grant:{}:milestones", grant_id));
                env.storage().remove(&format!("grant:{}:details", grant_id));

                purged_count += 1;
            }
        }
    }

    // Incentivize caller with Gas Bounty (reclaimed rent)
    let bounty = purged_count * 10; // Example: 10 units per purged grant
    credit_bounty(env, &caller, bounty);

    env.events().publish(
        (["grant", "batch_purge"],),
        (caller, purged_count, bounty),
    );
}

fn credit_bounty(env: &Env, caller: &Address, bounty: i128) {
    let mut balance: i128 = env.storage().get(&format!("balance:{}", caller)).unwrap_or(0);
    balance += bounty;
    env.storage().set(&format!("balance:{}", caller), &balance);
}
