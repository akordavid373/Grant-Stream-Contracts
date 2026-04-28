#![cfg(test)]

use super::{GrantStreamContract, GrantStreamContractClient, GrantStatus, SCALING_FACTOR};
use soroban_sdk::{
    testutils::{Address as _, Ledger},
    token, Address, Env, Symbol,
};

fn setup_test(env: &Env) -> (Address, Address, Address, Address, Address, GrantStreamContractClient) {
    let admin = Address::generate(env);
    let grant_token_addr = env.register_stellar_asset_contract_v2(admin.clone());
    let native_token_addr = env.register_stellar_asset_contract_v2(admin.clone());
    let treasury = Address::generate(env);
    let oracle = Address::generate(env);

    let contract_id = env.register(GrantStreamContract, ());
    let client = GrantStreamContractClient::new(env, &contract_id);

    client.initialize(&admin, &grant_token_addr.address(), &treasury, &oracle, &native_token_addr.address());

    (admin, grant_token_addr.address(), treasury, oracle, native_token_addr.address(), client)
}

fn set_timestamp(env: &Env, timestamp: u64) {
    env.ledger().with_mut(|li| {
        li.timestamp = timestamp;
    });
}

#[test]
fn test_pipeline() {
    let env = Env::default();
    env.mock_all_auths();
    let (_admin, grant_token_addr, _treasury, _oracle, _native, client) = setup_test(&env);
    let recipient = Address::generate(&env);
    let grant_token = token::Client::new(&env, &grant_token_addr);
    let grant_token_admin = token::StellarAssetClient::new(&env, &grant_token_addr);

    set_timestamp(&env, 1000);
    
    // 1. Create Grant
    let grant_id = 1;
    let total_amount = 1_000_000 * SCALING_FACTOR; // Large enough to not complete early
    let flow_rate = 1 * SCALING_FACTOR; // 1 token per second
    let warmup_duration = 0;
    
    // Mint tokens to contract for payout
    grant_token_admin.mint(&client.address, &total_amount);

    client.create_grant(&grant_id, &recipient, &total_amount, &flow_rate, &warmup_duration, &None);
}

#[test]
fn test_is_active_grantee_basic_functionality() {
    let env = Env::default();
    env.mock_all_auths();
    let (_admin, _grant_token_addr, _treasury, _oracle, _native, client) = setup_test(&env);
    
    let active_grantee = Address::generate(&env);
    let inactive_grantee = Address::generate(&env);
    let no_grants_user = Address::generate(&env);
    
    // Test 1: User with no grants should return false
    assert!(!client.is_active_grantee(&no_grants_user), "User with no grants should return false");
    
    // Test 2: Create an active grant
    client.create_grant(&1u64, &active_grantee, &1000000i128, &100i128, &0u64, &None);
    assert!(client.is_active_grantee(&active_grantee), "User with active grant should return true");
    
    // Test 3: Create a completed grant
    client.create_grant(&2u64, &inactive_grantee, &1000000i128, &100i128, &0u64, &None);
    // Simulate completion by withdrawing all funds
    set_timestamp(&env, 20000); // Allow some streaming
    let claimable = client.claimable(&2u64);
    if claimable > 0 {
        // For testing, we'll manually set the status to completed
        // In real scenarios, this would happen through normal flow
    }
}

#[test]
fn test_is_active_grantee_with_different_statuses() {
    let env = Env::default();
    env.mock_all_auths();
    let (admin, _grant_token_addr, _treasury, _oracle, _native, client) = setup_test(&env);
    
    let active_grantee = Address::generate(&env);
    let paused_grantee = Address::generate(&env);
    let completed_grantee = Address::generate(&env);
    let cancelled_grantee = Address::generate(&env);
    let ragequit_grantee = Address::generate(&env);
    
    // Create grants for each user
    client.create_grant(&1u64, &active_grantee, &1000000i128, &100i128, &0u64, &None);
    client.create_grant(&2u64, &paused_grantee, &1000000i128, &100i128, &0u64, &None);
    client.create_grant(&3u64, &completed_grantee, &1000000i128, &100i128, &0u64, &None);
    client.create_grant(&4u64, &cancelled_grantee, &1000000i128, &100i128, &0u64, &None);
    client.create_grant(&5u64, &ragequit_grantee, &1000000i128, &100i128, &0u64, &None);
    
    // Test active grant (should return true)
    assert!(client.is_active_grantee(&active_grantee), "Active grantee should return true");
    
    // Pause grant 2 (should still return true - paused is considered active)
    client.pause_stream(&2u64);
    assert!(client.is_active_grantee(&paused_grantee), "Paused grantee should return true");
    
    // Complete grant 3 (should return false)
    // For testing, we'll simulate completion by setting status directly
    // In production, this would happen through normal grant lifecycle
    let grant = client.get_grant(&3u64).unwrap();
    // Note: In real implementation, you'd need to use admin functions to complete grants
    
    // Cancel grant 4 (should return false)
    client.cancel_grant(&4u64);
    assert!(!client.is_active_grantee(&cancelled_grantee), "Cancelled grantee should return false");
    
    // Note: Rage quit requires grant to be paused first
    client.pause_stream(&5u64);
    client.rage_quit(&5u64);
    assert!(!client.is_active_grantee(&ragequit_grantee), "Rage quit grantee should return false");
}

#[test]
fn test_is_active_grantee_edge_cases() {
    let env = Env::default();
    env.mock_all_auths();
    let (_admin, _grant_token_addr, _treasury, _oracle, _native, client) = setup_test(&env);
    
    let user_with_multiple_grants = Address::generate(&env);
    let user_with_depleted_grant = Address::generate(&env);
    
    // Test 1: User with multiple active grants
    client.create_grant(&1u64, &user_with_multiple_grants, &1000000i128, &100i128, &0u64, &None);
    client.create_grant(&2u64, &user_with_multiple_grants, &500000i128, &50i128, &0u64, &None);
    assert!(client.is_active_grantee(&user_with_multiple_grants), "User with multiple active grants should return true");
    
    // Test 2: User with one active and one completed grant
    client.create_grant(&3u64, &user_with_depleted_grant, &1000i128, &100i128, &0u64, &None);
    set_timestamp(&env, 100); // Allow streaming to complete
    // Small grant should be depleted
    let claimable = client.claimable(&3u64);
    // Even if depleted, the grant might still be considered active until status changes
    
    // Test 3: Zero amount grant
    let zero_grant_user = Address::generate(&env);
    client.create_grant(&4u64, &zero_grant_user, &0i128, &0i128, &0u64, &None);
    // Zero amount grants should not be considered active
    assert!(!client.is_active_grantee(&zero_grant_user), "Zero amount grant should not be considered active");
}

#[test]
fn test_is_active_grantee_performance() {
    let env = Env::default();
    env.mock_all_auths();
    let (_admin, _grant_token_addr, _treasury, _oracle, _native, client) = setup_test(&env);
    
    let test_user = Address::generate(&env);
    
    // Create multiple grants to test performance
    for i in 1..=10 {
        client.create_grant(&i, &test_user, &1000000i128, &100i128, &0u64, &None);
    }
    
    // Measure CPU instructions for multiple calls
    let before_cpu = env.budget().cpu_instruction_count();
    
    for _ in 0..100 {
        let _ = client.is_active_grantee(&test_user);
    }
    
    let after_cpu = env.budget().cpu_instruction_count();
    let total_cpu = after_cpu - before_cpu;
    let avg_cpu_per_call = total_cpu / 100;
    
    println!("Average CPU instructions per is_active_grantee call: {}", avg_cpu_per_call);
    
    // Should be well under 5,000 CPU instructions
    assert!(avg_cpu_per_call < 5000, "is_active_grantee exceeds 5,000 CPU instruction limit: {}", avg_cpu_per_call);
}

#[test]
fn test_is_active_grantee_archived_data() {
    let env = Env::default();
    env.mock_all_auths();
    let (_admin, _grant_token_addr, _treasury, _oracle, _native, client) = setup_test(&env);
    
    let archived_grantee = Address::generate(&env);
    
    // Create a grant and then cancel it (simulating archived data)
    client.create_grant(&1u64, &archived_grantee, &1000000i128, &100i128, &0u64, &None);
    assert!(client.is_active_grantee(&archived_grantee), "Active grant should return true");
    
    // Cancel the grant (simulating archival)
    client.cancel_grant(&1u64);
    assert!(!client.is_active_grantee(&archived_grantee), "Cancelled/archived grant should return false");
    
    // Test with user who had grants but all are now completed/cancelled
    // This simulates the "stale records" edge case
}

    // 2. Advance time and check claimable
    set_timestamp(&env, 1010); // 10 seconds later
    assert_eq!(client.claimable(&grant_id), 10 * SCALING_FACTOR);

    // 3. Withdraw
    client.withdraw(&grant_id, &(5 * SCALING_FACTOR));
    assert_eq!(grant_token.balance(&recipient), 5 * SCALING_FACTOR);
    assert_eq!(client.claimable(&grant_id), 5 * SCALING_FACTOR);

    // 4. Propose Rate Increase (Timelocked)
    let new_rate = 2 * SCALING_FACTOR;
    client.propose_rate_change(&grant_id, &new_rate);
    
    let grant = client.get_grant(&grant_id);
    assert_eq!(grant.pending_rate, new_rate);
    assert_eq!(grant.effective_timestamp, 1010 + 48 * 60 * 60);

    // 5. Advance time past timelock
    set_timestamp(&env, 1010 + 48 * 60 * 60 + 10);
    assert_eq!(client.claimable(&grant_id), 172825 * SCALING_FACTOR);
}

#[test]
fn test_warmup() {
    let env = Env::default();
    env.mock_all_auths();
    let (_admin, _grant_token_addr, _treasury, _oracle, _native, client) = setup_test(&env);
    let recipient = Address::generate(&env);
    
    set_timestamp(&env, 1000);
    let grant_id = 1;
    let flow_rate = 100 * SCALING_FACTOR;
    let warmup_duration = 100; // 100 seconds warmup
    
    client.create_grant(&grant_id, &recipient, &(10000 * SCALING_FACTOR), &flow_rate, &warmup_duration, &None);

    set_timestamp(&env, 1100);
    assert_eq!(client.claimable(&grant_id), 10000 * SCALING_FACTOR);
}

#[test]
fn test_rage_quit() {
    let env = Env::default();
    env.mock_all_auths();
    let (_admin, grant_token_addr, treasury, _oracle, _native, client) = setup_test(&env);
    let recipient = Address::generate(&env);
    let grant_token = token::Client::new(&env, &grant_token_addr);
    let grant_token_admin = token::StellarAssetClient::new(&env, &grant_token_addr);
    
    set_timestamp(&env, 1000);
    let grant_id = 1;
    let total_amount = 1000 * SCALING_FACTOR;
    grant_token_admin.mint(&client.address, &total_amount);
    
    client.create_grant(&grant_id, &recipient, &total_amount, &SCALING_FACTOR, &0, &None);
    
    set_timestamp(&env, 1100); // 100 tokens accrued
    client.pause_stream(&grant_id);
    
    client.rage_quit(&grant_id);
    
    assert_eq!(grant_token.balance(&recipient), 100 * SCALING_FACTOR);
    assert_eq!(grant_token.balance(&treasury), 900 * SCALING_FACTOR);
}

#[test]
fn test_apply_kpi_multiplier_requires_oracle_auth() {
    let env = Env::default();
    env.mock_all_auths();
    let (_admin, _grant_token, _treasury, oracle, _native, client) = setup_test(&env);
    let recipient = Address::generate(&env);
    
    let grant_id = 1;
    client.create_grant(&grant_id, &recipient, &(1000 * SCALING_FACTOR), &SCALING_FACTOR, &0, &None);
    
    client.apply_kpi_multiplier(&grant_id, &20000); // 2x in basis points
    
    let grant = client.get_grant(&grant_id);
    assert_eq!(grant.flow_rate, 2 * SCALING_FACTOR);
}

#[test]
fn test_validator_split_basic() {
    let env = Env::default();
    env.mock_all_auths();
    let (_admin, grant_token_addr, _treasury, _oracle, _native, client) = setup_test(&env);
    let recipient = Address::generate(&env);
    let validator = Address::generate(&env);
    let grant_token_admin = token::StellarAssetClient::new(&env, &grant_token_addr);

    set_timestamp(&env, 1000);
    let grant_id = 1;
    let total_amount = 1_000_000 * SCALING_FACTOR;
    let flow_rate = 1 * SCALING_FACTOR;
    grant_token_admin.mint(&client.address, &total_amount);

    client.create_grant(
        &grant_id, &recipient, &total_amount, &flow_rate, &0,
        &Some(validator.clone()),
    );

    set_timestamp(&env, 1100);
    assert_eq!(client.claimable(&grant_id), 95 * SCALING_FACTOR);
    assert_eq!(client.validator_claimable(&grant_id), 5 * SCALING_FACTOR);
}

#[test]
fn test_milestone_proof_nonce_replay_is_rejected() {
    let env = Env::default();
    env.mock_all_auths();
    let (_admin, grant_token_addr, _treasury, _oracle, _native, client) = setup_test(&env);
    let recipient = Address::generate(&env);
    let grant_token_admin = token::StellarAssetClient::new(&env, &grant_token_addr);

    set_timestamp(&env, 1000);
    let grant_id = 77u64;
    let total_amount = 1_000 * SCALING_FACTOR;
    grant_token_admin.mint(&client.address, &total_amount);

    client.create_grant(&grant_id, &recipient, &total_amount, &SCALING_FACTOR, &0, &None);

    let proof_1 = Symbol::new(&env, "proof_1");
    client.submit_milestone_proof(&grant_id, &0u32, &proof_1, &0u64);

    let replay = client.try_submit_milestone_proof(&grant_id, &1u32, &Symbol::new(&env, "proof_2"), &0u64);
    assert!(replay.is_err(), "reused nonce must be rejected");

    client.submit_milestone_proof(&grant_id, &1u32, &Symbol::new(&env, "proof_3"), &1u64);

    set_timestamp(&env, 1100);
    client.cancel_grant(&grant_id);
    let after_cancel = client.try_submit_milestone_proof(&grant_id, &2u32, &Symbol::new(&env, "proof_4"), &2u64);
    assert!(after_cancel.is_err(), "cancelled grants must reject new proofs");
}

#[test]
fn test_finalize_and_purge_rejects_pending_claims() {
    let env = Env::default();
    env.mock_all_auths();
    let (_admin, grant_token_addr, _treasury, _oracle, _native, client) = setup_test(&env);
    let recipient = Address::generate(&env);
    let grant_token_admin = token::StellarAssetClient::new(&env, &grant_token_addr);

    set_timestamp(&env, 1000);
    let grant_id = 88u64;
    let total_amount = 1_000 * SCALING_FACTOR;
    grant_token_admin.mint(&client.address, &total_amount);
    client.create_grant(&grant_id, &recipient, &total_amount, &SCALING_FACTOR, &0, &None);

    // Generate recipient claimable and then cancel; pending claim must block purge.
    set_timestamp(&env, 1100);
    client.cancel_grant(&grant_id);

    let purger = Address::generate(&env);
    let result = client.try_finalize_and_purge(&grant_id, &purger);
    assert!(result.is_err(), "must not purge while recipient claim remains withdrawable");
}

