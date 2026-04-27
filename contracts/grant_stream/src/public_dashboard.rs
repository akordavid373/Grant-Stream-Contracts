/// Public-Dashboard Event (Issue #324)
///
/// Emits a heartbeat event every 24 hours (or on significant balance changes)
/// containing Total_TVL, Active_Stream_Count, Disputed_Amount, and
/// Available_Liquidity for community bots to consume.

use soroban_sdk::{symbol_short, Env};

const HEARTBEAT_INTERVAL_SECS: u64 = 24 * 60 * 60; // 24 hours
const SIGNIFICANT_CHANGE_BPS: i128 = 500; // 5% change triggers early heartbeat

#[derive(Clone)]
#[soroban_sdk::contracttype]
pub struct DashboardSnapshot {
    pub total_tvl: i128,
    pub active_stream_count: u32,
    pub disputed_amount: i128,
    pub available_liquidity: i128,
    pub timestamp: u64,
}

#[derive(Clone)]
#[soroban_sdk::contracttype]
pub enum DashboardKey {
    LastHeartbeat,
    LastTvl,
}

/// Emit a heartbeat event if 24 hours have elapsed or TVL changed by ≥5%.
/// Returns `true` when an event was emitted.
pub fn heartbeat_emit(
    env: &Env,
    total_tvl: i128,
    active_stream_count: u32,
    disputed_amount: i128,
    available_liquidity: i128,
) -> bool {
    let now = env.ledger().timestamp();

    let last_heartbeat: u64 = env
        .storage()
        .instance()
        .get(&DashboardKey::LastHeartbeat)
        .unwrap_or(0);

    let last_tvl: i128 = env
        .storage()
        .instance()
        .get(&DashboardKey::LastTvl)
        .unwrap_or(0);

    let time_elapsed = now.saturating_sub(last_heartbeat) >= HEARTBEAT_INTERVAL_SECS;

    // Detect significant TVL change (≥5%)
    let significant_change = if last_tvl > 0 {
        let delta = (total_tvl - last_tvl).abs();
        // delta / last_tvl >= 5% => delta * 10000 / last_tvl >= 500
        delta.saturating_mul(10000) / last_tvl >= SIGNIFICANT_CHANGE_BPS
    } else {
        total_tvl > 0
    };

    if !time_elapsed && !significant_change {
        return false;
    }

    let snapshot = DashboardSnapshot {
        total_tvl,
        active_stream_count,
        disputed_amount,
        available_liquidity,
        timestamp: now,
    };

    env.events().publish(
        (symbol_short!("heartbeat"),),
        snapshot,
    );

    env.storage()
        .instance()
        .set(&DashboardKey::LastHeartbeat, &now);
    env.storage()
        .instance()
        .set(&DashboardKey::LastTvl, &total_tvl);

    true
}
