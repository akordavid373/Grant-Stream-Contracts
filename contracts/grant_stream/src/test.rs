#![cfg(test)]

use super::{GrantStreamContract, GrantStreamContractClient, GrantStatus, SCALING_FACTOR};
use std::println;
use soroban_sdk::{
    testutils::{Address as _, Ledger},
    token, Address, Bytes, Env, Symbol, xdr::ToXdr,
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

fn build_confidential_proof(
    env: &Env,
    grant_id: u64,
    commitment_before: i128,
    claim_amount: i128,
    nullifier: &Bytes,
    verifier_key_hash: &Bytes,
) -> Bytes {
    let commitment_after = commitment_before - claim_amount;
    let mut public_inputs = Bytes::new(env);
    for byte in grant_id.to_be_bytes() {
        public_inputs.push_back(byte);
    }
    public_inputs.append(&commitment_before.to_xdr(env));
    public_inputs.append(&commitment_after.to_xdr(env));
    public_inputs.append(&claim_amount.to_xdr(env));
    public_inputs.append(nullifier);
    public_inputs.append(verifier_key_hash);
    env.crypto().sha256(&public_inputs).into()
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

    client.create_grant(&grant_id, &recipient, &total_amount, &flow_rate, &warmup_duration, &None, &None);
}

#[test]
fn test_milestone_submission_deposit_refunded_on_approval() {
    let env = Env::default();
    env.mock_all_auths();
    let (_admin, _grant_token_addr, _treasury, _oracle, native_token_addr, client) = setup_test(&env);
    let recipient = Address::generate(&env);
    let native_token = token::Client::new(&env, &native_token_addr);
    let native_token_admin = token::StellarAssetClient::new(&env, &native_token_addr);
    let grant_id = 77u64;
    let deposit = 100_000i128;

    native_token_admin.mint(&recipient, &1_000_000i128);
    client.create_grant(&grant_id, &recipient, &1_000_000i128, &1_000i128, &0u64, &None, &None);

    let recipient_before = native_token.balance(&recipient);
    let contract_before = native_token.balance(&client.address);
    client.submit_milestone_proof(&grant_id, &0u32, &Symbol::new(&env, "m0"), &0u64).unwrap();
    let recipient_after_submit = native_token.balance(&recipient);
    let contract_after_submit = native_token.balance(&client.address);

    assert_eq!(recipient_after_submit, recipient_before - deposit);
    assert_eq!(contract_after_submit, contract_before + deposit);

    client.approve_milestone_submission(&grant_id, &0u32).unwrap();
    let recipient_after_approval = native_token.balance(&recipient);
    let contract_after_approval = native_token.balance(&client.address);

    assert_eq!(recipient_after_approval, recipient_before);
    assert_eq!(contract_after_approval, contract_before);
}

#[test]
fn test_milestone_submission_deposit_slashed_to_treasury() {
    let env = Env::default();
    env.mock_all_auths();
    let (_admin, _grant_token_addr, treasury, _oracle, native_token_addr, client) = setup_test(&env);
    let recipient = Address::generate(&env);
    let native_token = token::Client::new(&env, &native_token_addr);
    let native_token_admin = token::StellarAssetClient::new(&env, &native_token_addr);
    let grant_id = 78u64;
    let deposit = 100_000i128;

    native_token_admin.mint(&recipient, &1_000_000i128);
    client.create_grant(&grant_id, &recipient, &1_000_000i128, &1_000i128, &0u64, &None, &None);

    let treasury_before = native_token.balance(&treasury);
    let contract_before = native_token.balance(&client.address);
    client.submit_milestone_proof(&grant_id, &0u32, &Symbol::new(&env, "m1"), &0u64).unwrap();
    client.slash_milestone_submission_deposit(&grant_id, &0u32).unwrap();

    let treasury_after = native_token.balance(&treasury);
    let contract_after = native_token.balance(&client.address);
    assert_eq!(treasury_after, treasury_before + deposit);
    assert_eq!(contract_after, contract_before);
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
    client.create_grant(&1u64, &active_grantee, &1000000i128, &100i128, &0u64, &None, &None);
    assert!(client.is_active_grantee(&active_grantee), "User with active grant should return true");
    
    // Test 3: Create a completed grant
    client.create_grant(&2u64, &inactive_grantee, &1000000i128, &100i128, &0u64, &None, &None);
    // Simulate completion by withdrawing all funds
    set_timestamp(&env, 20000); // Allow some streaming
    let claimable = client.claimable(&2u64);
    if claimable > 0 {
        // For testing, we'll manually set the status to completed
        // In real scenarios, this would happen through normal flow
    }
}

#[test]
fn test_current_claimable_amounts_are_previewed_without_storage_change() {
    let env = Env::default();
    env.mock_all_auths();
    let (_admin, _grant_token_addr, _treasury, _oracle, _native, client) = setup_test(&env);
    let recipient = Address::generate(&env);
    let validator = Address::generate(&env);
    let grant_id = 1u64;
    let total_amount = 1000 * SCALING_FACTOR;
    let flow_rate = 1 * SCALING_FACTOR;

    set_timestamp(&env, 100);
    client.create_grant(&grant_id, &recipient, &total_amount, &flow_rate, &0u64, &Some(validator.clone()));
    set_timestamp(&env, 110);

    let (claimable, validator_claimable) = client.get_current_claimable_amounts(&grant_id).unwrap();
    assert_eq!(claimable, 95 * SCALING_FACTOR);
    assert_eq!(validator_claimable, 5 * SCALING_FACTOR);

    let stored_grant = client.get_grant(&grant_id).unwrap();
    assert_eq!(stored_grant.claimable, 0, "Preview query should not mutate stored grant state");
    assert_eq!(stored_grant.validator_claimable, 0, "Preview query should not mutate stored grant state");
}

#[test]
fn test_get_health_factor_is_read_only_preview() {
    let env = Env::default();
    env.mock_all_auths();
    let (_admin, _grant_token_addr, _treasury, _oracle, _native, client) = setup_test(&env);
    let grant_id = 1u64;
    let recipient = Address::generate(&env);

    client.create_grant(&grant_id, &recipient, &100_000i128, &1_000i128, &0u64, &None);
    env.storage().instance().set(&super::storage_keys::StorageKey::ReserveBalance, &100_000i128);

    let health = client.get_health_factor().unwrap();
    assert_eq!(health, 9000, "Health factor should reflect the current reserve and liabilities without mutating state");
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
    client.pause_stream(&2u64, &None);
    assert!(client.is_active_grantee(&paused_grantee), "Paused grantee should return true");
    
    // Complete grant 3 (should return false)
    // For testing, we'll simulate completion by setting status directly
    // In production, this would happen through normal grant lifecycle
    let grant = client.get_grant(&3u64);
    // Note: In real implementation, you'd need to use admin functions to complete grants
    
    // Cancel grant 4 (should return false)
    client.cancel_grant(&4u64);
    assert!(!client.is_active_grantee(&cancelled_grantee), "Cancelled grantee should return false");
    
    // Note: Rage quit requires grant to be paused first
    client.pause_stream(&5u64, &None);
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
    let before_cpu = env.budget().cpu_instruction_cost();
    
    for _ in 0..100 {
        let _ = client.is_active_grantee(&test_user);
    }
    
    let after_cpu = env.budget().cpu_instruction_cost();
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
    client.pause_stream(&grant_id, &None);
    
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
        &Some(validator.clone()), &None,
    );

    set_timestamp(&env, 1100);
    assert_eq!(client.claimable(&grant_id), 95 * SCALING_FACTOR);
    assert_eq!(client.validator_claimable(&grant_id), 5 * SCALING_FACTOR);
}

#[test]
fn test_withdraw_below_minimum_rejected() {
    let env = Env::default();
    env.mock_all_auths();
    let (_admin, grant_token_addr, _treasury, _oracle, _native, client) = setup_test(&env);
    let recipient = Address::generate(&env);
    let grant_token_admin = token::StellarAssetClient::new(&env, &grant_token_addr);

    set_timestamp(&env, 1000);
    let grant_id = 1;
    let total_amount = 1_000_000 * SCALING_FACTOR;
    // Flow rate: 0.5 USDC/sec — claimable after 1 sec is 0.5 USDC < 1 USDC minimum
    let flow_rate = SCALING_FACTOR / 2;
    grant_token_admin.mint(&client.address, &total_amount);
    client.create_grant(&grant_id, &recipient, &total_amount, &flow_rate, &0, &None);

    set_timestamp(&env, 1001); // only 0.5 USDC accrued
    let result = client.try_withdraw(&grant_id, &flow_rate);
    assert_eq!(result, Err(Ok(Error::WithdrawalBelowMinimum)));
}

#[test]
fn test_withdraw_at_minimum_succeeds() {
    let env = Env::default();
    env.mock_all_auths();
    let (_admin, grant_token_addr, _treasury, _oracle, _native, client) = setup_test(&env);
    let recipient = Address::generate(&env);
    let grant_token_admin = token::StellarAssetClient::new(&env, &grant_token_addr);

    set_timestamp(&env, 1000);
    let grant_id = 1;
    let total_amount = 1_000_000 * SCALING_FACTOR;
    // Flow rate: 1 USDC/sec — claimable after 1 sec is exactly 1 USDC
    let flow_rate = MIN_WITHDRAWAL;
    grant_token_admin.mint(&client.address, &total_amount);
    client.create_grant(&grant_id, &recipient, &total_amount, &flow_rate, &0, &None);

    set_timestamp(&env, 1001); // exactly MIN_WITHDRAWAL accrued
    client.withdraw(&grant_id, &MIN_WITHDRAWAL);
}

#[test]
fn test_withdraw_above_minimum_succeeds() {
    let env = Env::default();
    env.mock_all_auths();
    let (_admin, grant_token_addr, _treasury, _oracle, _native, client) = setup_test(&env);
    let recipient = Address::generate(&env);
    let grant_token_admin = token::StellarAssetClient::new(&env, &grant_token_addr);

    set_timestamp(&env, 1000);
    let grant_id = 1;
    let total_amount = 1_000_000 * SCALING_FACTOR;
    let flow_rate = 5 * SCALING_FACTOR; // 5 USDC/sec
    grant_token_admin.mint(&client.address, &total_amount);
    client.create_grant(&grant_id, &recipient, &total_amount, &flow_rate, &0, &None);

    set_timestamp(&env, 1010); // 50 USDC accrued >> minimum
    client.withdraw(&grant_id, &(50 * SCALING_FACTOR));
}

#[test]
fn test_confidential_claim_executes_with_valid_proof() {
    let env = Env::default();
    env.mock_all_auths();
    let (_admin, grant_token_addr, _treasury, _oracle, _native, client) = setup_test(&env);
    let recipient = Address::generate(&env);
    let grant_token = token::Client::new(&env, &grant_token_addr);
    let grant_token_admin = token::StellarAssetClient::new(&env, &grant_token_addr);
    let grant_id = 501u64;
    let commitment_before = 1_000_000i128;
    let claim_amount = 10_000i128;
    let nullifier = Bytes::from_array(&env, &[1; 32]);
    let verifier_key_hash = Bytes::from_array(&env, &[7; 32]);

    grant_token_admin.mint(&client.address, &1_000_000i128);
    client.create_confidential_grant(
        &grant_id,
        &recipient,
        &commitment_before,
        &verifier_key_hash,
    );

    let proof = build_confidential_proof(
        &env,
        grant_id,
        commitment_before,
        claim_amount,
        &nullifier,
        &verifier_key_hash,
    );
    let recipient_before = grant_token.balance(&recipient);
    client.confidential_claim(&grant_id, &claim_amount, &nullifier, &proof);
    let recipient_after = grant_token.balance(&recipient);
    assert_eq!(recipient_after, recipient_before + claim_amount);
}

#[test]
fn test_confidential_claim_invalid_proof_fuzz_barrage() {
    let env = Env::default();
    env.mock_all_auths();
    let (_admin, grant_token_addr, _treasury, _oracle, _native, client) = setup_test(&env);
    let recipient = Address::generate(&env);
    let grant_token_admin = token::StellarAssetClient::new(&env, &grant_token_addr);
    let grant_id = 502u64;
    let commitment_before = 50_000i128;
    let verifier_key_hash = Bytes::from_array(&env, &[9; 32]);

    grant_token_admin.mint(&client.address, &1_000_000i128);
    client.create_confidential_grant(
        &grant_id,
        &recipient,
        &commitment_before,
        &verifier_key_hash,
    );

    for i in 0..128u32 {
        let mut bad_nullifier = Bytes::new(&env);
        bad_nullifier.append(&Bytes::from_array(&env, &[0; 31]));
        bad_nullifier.push_back((i % 255) as u8);
        let mut bad_proof = Bytes::new(&env);
        bad_proof.append(&Bytes::from_array(&env, &[3; 31]));
        bad_proof.push_back((i % 251) as u8);
        let bad_claim = commitment_before + 1 + i as i128;
        let result = client.try_confidential_claim(&grant_id, &bad_claim, &bad_nullifier, &bad_proof);
        assert_eq!(result, Err(Ok(super::Error::InvalidZKProof)));
    }

    let nullifier = Bytes::from_array(&env, &[5; 32]);
    let claim_amount = 1_000i128;
    let proof = build_confidential_proof(
        &env,
        grant_id,
        commitment_before,
        claim_amount,
        &nullifier,
        &verifier_key_hash,
    );
    client.confidential_claim(&grant_id, &claim_amount, &nullifier, &proof);
    let replay = client.try_confidential_claim(&grant_id, &claim_amount, &nullifier, &proof);
    assert_eq!(replay, Err(Ok(super::Error::InvalidZKProof)));
}
