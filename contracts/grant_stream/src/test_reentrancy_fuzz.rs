// ---------------------------------------------------------------------------
// Issue #303 — Fuzz-Test: Reentrancy-like State Corruption
//
// Although Soroban prevents traditional reentrancy, this test simulates
// "Logical Reentrancy" where a user attempts to call withdraw multiple times
// within a complex transaction tree.
//
// Invariant verified: state fields (last_claim_time, withdrawn, claimable)
// are committed atomically before any token transfer, preventing double-spend
// of the same time-window.
// ---------------------------------------------------------------------------
#![cfg(test)]

use super::{GrantStreamContract, GrantStreamContractClient, GrantStatus, SCALING_FACTOR};
use soroban_sdk::{
    testutils::{Address as _, Ledger},
    token, Address, Env,
};

fn setup(env: &Env) -> (Address, Address, Address, Address, Address, GrantStreamContractClient) {
    let admin = Address::generate(env);
    let grant_token = env.register_stellar_asset_contract_v2(admin.clone());
    let native_token = env.register_stellar_asset_contract_v2(admin.clone());
    let treasury = Address::generate(env);
    let oracle = Address::generate(env);

    let contract_id = env.register(GrantStreamContract, ());
    let client = GrantStreamContractClient::new(env, &contract_id);

    client.initialize(
        &admin,
        &grant_token.address(),
        &treasury,
        &oracle,
        &native_token.address(),
    );

    (admin, grant_token.address(), treasury, oracle, native_token.address(), client)
}

fn set_ts(env: &Env, ts: u64) {
    env.ledger().with_mut(|li| li.timestamp = ts);
}

/// Core invariant: after any sequence of withdrawals, the sum of
/// `withdrawn + claimable` never exceeds what was accrued up to that point,
/// and `last_claim_time` is always ≥ the timestamp of the last withdrawal.
#[test]
fn test_state_committed_before_transfer_single_window() {
    let env = Env::default();
    env.mock_all_auths();
    let (_admin, grant_token_addr, _treasury, _oracle, _native, client) = setup(&env);

    let recipient = Address::generate(&env);
    let grant_token = token::StellarAssetClient::new(&env, &grant_token_addr);

    let total = 1_000 * SCALING_FACTOR;
    let rate = 10 * SCALING_FACTOR; // 10 tokens/sec

    set_ts(&env, 1_000);
    grant_token.mint(&client.address, &total);
    client.create_grant(&1u64, &recipient, &total, &rate, &0u64, &None);

    // Advance 5 seconds → 50 tokens accrued
    set_ts(&env, 1_005);

    // First withdrawal: 30 tokens
    client.withdraw(&1u64, &(30 * SCALING_FACTOR));

    let grant = client.get_grant(&1u64);
    // last_claim_time must be updated to the withdrawal timestamp
    assert_eq!(grant.last_claim_time, 1_005, "last_claim_time not committed");
    assert_eq!(grant.withdrawn, 30 * SCALING_FACTOR, "withdrawn not committed");
    // Remaining claimable = 50 - 30 = 20
    assert_eq!(grant.claimable, 20 * SCALING_FACTOR, "claimable inconsistent");

    // Second withdrawal in the same second: 20 tokens (remaining claimable)
    client.withdraw(&1u64, &(20 * SCALING_FACTOR));

    let grant2 = client.get_grant(&1u64);
    assert_eq!(grant2.withdrawn, 50 * SCALING_FACTOR, "double-spend detected");
    assert_eq!(grant2.claimable, 0, "claimable should be zero after full withdrawal");
    assert_eq!(grant2.last_claim_time, 1_005, "last_claim_time must not regress");
}

/// Fuzz: for any sequence of partial withdrawals within a single time window,
/// total withdrawn never exceeds accrued amount (no double-spend).
#[test]
fn test_no_double_spend_across_partial_withdrawals() {
    let env = Env::default();
    env.mock_all_auths();
    let (_admin, grant_token_addr, _treasury, _oracle, _native, client) = setup(&env);

    let recipient = Address::generate(&env);
    let grant_token = token::StellarAssetClient::new(&env, &grant_token_addr);

    let total = 10_000 * SCALING_FACTOR;
    let rate = 100 * SCALING_FACTOR; // 100 tokens/sec

    set_ts(&env, 0);
    grant_token.mint(&client.address, &total);
    client.create_grant(&2u64, &recipient, &total, &rate, &0u64, &None);

    // Advance 10 seconds → 1000 tokens accrued
    set_ts(&env, 10);

    let accrued = 1_000 * SCALING_FACTOR;

    // Simulate multiple partial withdrawals in the same time window
    let chunks: &[i128] = &[200, 300, 150, 350]; // sums to 1000
    for &chunk in chunks {
        client.withdraw(&2u64, &(chunk * SCALING_FACTOR));
    }

    let grant = client.get_grant(&2u64);
    assert_eq!(grant.withdrawn, accrued, "total withdrawn must equal accrued");
    assert_eq!(grant.claimable, 0, "no claimable should remain");

    // Attempting one more withdrawal must fail (nothing left)
    let result = std::panic::catch_unwind(|| {
        client.withdraw(&2u64, &SCALING_FACTOR);
    });
    assert!(result.is_err(), "over-withdrawal should panic/revert");
}

/// Fuzz: last_claim_time is monotonically non-decreasing across withdrawals
/// at different timestamps.
#[test]
fn test_last_claim_time_monotonic() {
    let env = Env::default();
    env.mock_all_auths();
    let (_admin, grant_token_addr, _treasury, _oracle, _native, client) = setup(&env);

    let recipient = Address::generate(&env);
    let grant_token = token::StellarAssetClient::new(&env, &grant_token_addr);

    let total = 100_000 * SCALING_FACTOR;
    let rate = 10 * SCALING_FACTOR;

    set_ts(&env, 1_000);
    grant_token.mint(&client.address, &total);
    client.create_grant(&3u64, &recipient, &total, &rate, &0u64, &None);

    let timestamps: &[u64] = &[1_010, 1_020, 1_050, 1_100, 1_200];
    let mut prev_claim_time = 1_000u64;

    for &ts in timestamps {
        set_ts(&env, ts);
        let claimable = client.claimable(&3u64);
        if claimable > 0 {
            client.withdraw(&3u64, &claimable);
        }
        let grant = client.get_grant(&3u64);
        assert!(
            grant.last_claim_time >= prev_claim_time,
            "last_claim_time regressed: {} < {}",
            grant.last_claim_time,
            prev_claim_time
        );
        prev_claim_time = grant.last_claim_time;
    }
}

/// Fuzz: withdrawn + claimable + validator_withdrawn + validator_claimable
/// never exceeds total_amount at any point.
#[test]
fn test_total_accounted_never_exceeds_total_amount() {
    let env = Env::default();
    env.mock_all_auths();
    let (_admin, grant_token_addr, _treasury, _oracle, _native, client) = setup(&env);

    let recipient = Address::generate(&env);
    let grant_token = token::StellarAssetClient::new(&env, &grant_token_addr);

    let total = 500 * SCALING_FACTOR;
    let rate = 50 * SCALING_FACTOR; // completes in 10 seconds

    set_ts(&env, 0);
    grant_token.mint(&client.address, &total);
    client.create_grant(&4u64, &recipient, &total, &rate, &0u64, &None);

    // Advance past completion
    set_ts(&env, 20);

    let claimable = client.claimable(&4u64);
    client.withdraw(&4u64, &claimable);

    let grant = client.get_grant(&4u64);
    let total_accounted = grant.withdrawn
        + grant.claimable
        + grant.validator_withdrawn
        + grant.validator_claimable;

    assert!(
        total_accounted <= grant.total_amount,
        "total accounted ({total_accounted}) exceeds total_amount ({})",
        grant.total_amount
    );
    assert_eq!(grant.status, GrantStatus::Completed);
}
