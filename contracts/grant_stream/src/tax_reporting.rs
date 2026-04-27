/// Tax-Reporting Export Hook (Issue #323)
///
/// Provides `get_historical_flow` which returns the time-weighted average
/// (TWA) of funds received by a recipient over a specific ledger range.
/// Institutional users can consume this to generate "Web3 1099-MISC"
/// equivalent reports.

use soroban_sdk::{symbol_short, Address, Env, Vec};

#[derive(Clone)]
#[soroban_sdk::contracttype]
pub struct FlowRecord {
    /// Ledger timestamp when this record was written.
    pub timestamp: u64,
    /// Cumulative amount received by the recipient up to this timestamp.
    pub cumulative_amount: i128,
}

#[derive(Clone)]
#[soroban_sdk::contracttype]
pub enum TaxKey {
    /// Vec<FlowRecord> keyed by recipient address.
    FlowHistory(Address),
}

/// Append a new flow record for `recipient`.  Call this whenever a withdrawal
/// is processed so the history stays up-to-date.
pub fn record_flow(env: &Env, recipient: &Address, cumulative_amount: i128) {
    let key = TaxKey::FlowHistory(recipient.clone());
    let mut history: Vec<FlowRecord> = env
        .storage()
        .instance()
        .get(&key)
        .unwrap_or_else(|| Vec::new(env));

    history.push_back(FlowRecord {
        timestamp: env.ledger().timestamp(),
        cumulative_amount,
    });

    env.storage().instance().set(&key, &history);
}

/// Return the time-weighted average flow rate (tokens per second) for
/// `recipient` between `start_ts` and `end_ts`.
///
/// The TWA is computed as:
///   (cumulative_at_end - cumulative_at_start) / (end_ts - start_ts)
///
/// Returns `(total_received, twa_per_second)`.
pub fn get_historical_flow(
    env: &Env,
    recipient: &Address,
    start_ts: u64,
    end_ts: u64,
) -> (i128, i128) {
    if end_ts <= start_ts {
        return (0, 0);
    }

    let key = TaxKey::FlowHistory(recipient.clone());
    let history: Vec<FlowRecord> = env
        .storage()
        .instance()
        .get(&key)
        .unwrap_or_else(|| Vec::new(env));

    if history.is_empty() {
        return (0, 0);
    }

    // Find the last record at or before start_ts and end_ts respectively.
    let mut cumulative_at_start: i128 = 0;
    let mut cumulative_at_end: i128 = 0;

    for i in 0..history.len() {
        let record = history.get(i).unwrap();
        if record.timestamp <= start_ts {
            cumulative_at_start = record.cumulative_amount;
        }
        if record.timestamp <= end_ts {
            cumulative_at_end = record.cumulative_amount;
        }
    }

    let total_received = cumulative_at_end.saturating_sub(cumulative_at_start);
    let duration = (end_ts - start_ts) as i128;
    let twa_per_second = if duration > 0 {
        total_received / duration
    } else {
        0
    };

    env.events().publish(
        (symbol_short!("taxreport"), recipient.clone()),
        (start_ts, end_ts, total_received, twa_per_second),
    );

    (total_received, twa_per_second)
}
