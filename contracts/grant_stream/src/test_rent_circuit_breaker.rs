#![cfg(test)]

use super::{
    circuit_breakers, GrantStreamContract, GrantStreamContractClient, Error, SCALING_FACTOR,
};
use soroban_sdk::{
    testutils::{Address as _, Ledger},
    token, Address, Env,
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
fn test_rent_balance_healthy() {
    let env = Env::default();
    env.mock_all_auths();
    let (admin, _grant_token_addr, _treasury, _oracle, _native, client) = setup_test(&env);

    // Give contract a healthy balance (more than 3 XLM rent buffer)
    let contract_address = client.address.clone();
    env.ledger().with_mut(|li| {
        li.balance.insert(contract_address.clone(), 5_000_0000); // 5 XLM in stroops
    });

    // Check rent balance should return true (healthy)
    assert!(client.check_rent_balance().unwrap());
    
    // Rent preservation mode should be false
    assert!(!client.is_rent_preservation_mode());
    
    // Current balance should be above threshold
    assert!(client.get_current_xlm_balance() > client.get_rent_buffer_threshold());
}

#[test]
fn test_rent_balance_depleted() {
    let env = Env::default();
    env.mock_all_auths();
    let (admin, _grant_token_addr, _treasury, _oracle, _native, client) = setup_test(&env);

    // Give contract a low balance (less than 3 XLM rent buffer)
    let contract_address = client.address.clone();
    env.ledger().with_mut(|li| {
        li.balance.insert(contract_address.clone(), 1_000_0000); // 1 XLM in stroops
    });

    // Check rent balance should return false (depleted)
    assert!(!client.check_rent_balance().unwrap());
    
    // Rent preservation mode should be true
    assert!(client.is_rent_preservation_mode());
    
    // Current balance should be below threshold
    assert!(client.get_current_xlm_balance() < client.get_rent_buffer_threshold());
}

#[test]
fn test_rent_preservation_blocks_non_essential_functions() {
    let env = Env::default();
    env.mock_all_auths();
    let (admin, grant_token_addr, _treasury, _oracle, _native, client) = setup_test(&env);
    let recipient = Address::generate(&env);

    // Set low balance to trigger rent preservation mode
    let contract_address = client.address.clone();
    env.ledger().with_mut(|li| {
        li.balance.insert(contract_address.clone(), 1_000_0000); // 1 XLM in stroops
    });

    // Trigger rent preservation mode
    client.check_rent_balance().unwrap();
    assert!(client.is_rent_preservation_mode());

    // Try to create a grant (non-essential function) - should fail
    let grant_id = 1;
    let total_amount = 1_000_000 * SCALING_FACTOR;
    let flow_rate = 1 * SCALING_FACTOR;
    let warmup_duration = 0;

    let result = client.try_create_grant(&grant_id, &recipient, &total_amount, &flow_rate, &warmup_duration, &None);
    assert_eq!(result, Err(Ok(Error::RentPreservationMode)));
}

#[test]
fn test_rent_preservation_allows_essential_functions() {
    let env = Env::default();
    env.mock_all_auths();
    let (admin, _grant_token_addr, _treasury, _oracle, _native, client) = setup_test(&env);

    // Set low balance to trigger rent preservation mode
    let contract_address = client.address.clone();
    env.ledger().with_mut(|li| {
        li.balance.insert(contract_address.clone(), 1_000_0000); // 1 XLM in stroops
    });

    // Trigger rent preservation mode
    client.check_rent_balance().unwrap();
    assert!(client.is_rent_preservation_mode());

    // Essential functions like checking rent balance should still work
    assert!(!client.check_rent_balance().unwrap());
    assert!(client.is_rent_preservation_mode());
    
    // Getting balance info should work
    let current_balance = client.get_current_xlm_balance();
    let threshold = client.get_rent_buffer_threshold();
    assert!(current_balance < threshold);
}

#[test]
fn test_disable_rent_preservation_mode() {
    let env = Env::default();
    env.mock_all_auths();
    let (admin, _grant_token_addr, _treasury, _oracle, _native, client) = setup_test(&env);

    // Set low balance to trigger rent preservation mode
    let contract_address = client.address.clone();
    env.ledger().with_mut(|li| {
        li.balance.insert(contract_address.clone(), 1_000_0000); // 1 XLM in stroops
    });

    // Trigger rent preservation mode
    client.check_rent_balance().unwrap();
    assert!(client.is_rent_preservation_mode());

    // Try to disable with insufficient balance - should fail
    let result = client.try_disable_rent_preservation_mode();
    assert!(result.is_err()); // Should panic due to insufficient balance

    // Now add sufficient balance
    env.ledger().with_mut(|li| {
        li.balance.insert(contract_address.clone(), 5_000_0000); // 5 XLM in stroops
    });

    // Should be able to disable rent preservation mode now
    client.disable_rent_preservation_mode().unwrap();
    assert!(!client.is_rent_preservation_mode());
}

#[test]
fn test_withdraw_triggers_rent_check() {
    let env = Env::default();
    env.mock_all_auths();
    let (admin, grant_token_addr, _treasury, _oracle, _native, client) = setup_test(&env);
    let recipient = Address::generate(&env);
    let grant_token = token::Client::new(&env, &grant_token_addr);
    let grant_token_admin = token::StellarAssetClient::new(&env, &grant_token_addr);

    set_timestamp(&env, 1000);
    
    // Create a grant
    let grant_id = 1;
    let total_amount = 1_000_000 * SCALING_FACTOR;
    let flow_rate = 1 * SCALING_FACTOR;
    let warmup_duration = 0;
    
    // Mint tokens to contract for payout
    grant_token_admin.mint(&client.address, &total_amount);

    client.create_grant(&grant_id, &recipient, &total_amount, &flow_rate, &warmup_duration, &None);

    // Advance time and create claimable amount
    set_timestamp(&env, 1010); // 10 seconds later
    assert_eq!(client.claimable(&grant_id), 10 * SCALING_FACTOR);

    // Set low balance before withdrawal
    let contract_address = client.address.clone();
    env.ledger().with_mut(|li| {
        li.balance.insert(contract_address.clone(), 1_000_0000); // 1 XLM in stroops
    });

    // Withdraw should trigger rent check and work (rent check happens after withdrawal)
    client.withdraw(&grant_id, &(5 * SCALING_FACTOR));
    
    // After withdrawal, rent preservation mode should be engaged
    assert!(client.is_rent_preservation_mode());
}

#[test]
fn test_rent_buffer_constants() {
    let env = Env::default();
    env.mock_all_auths();
    let (_admin, _grant_token_addr, _treasury, _oracle, _native, client) = setup_test(&env);

    // Rent buffer should be 3 XLM (3 * 10^7 stroops)
    let expected_buffer = 3 * 10i128.pow(7); // 3 XLM in stroops
    assert_eq!(client.get_rent_buffer_threshold(), expected_buffer);
}

#[test]
fn test_rent_check_updates_timestamp() {
    let env = Env::default();
    env.mock_all_auths();
    let (admin, _grant_token_addr, _treasury, _oracle, _native, client) = setup_test(&env);

    set_timestamp(&env, 1000);

    // Give contract a healthy balance
    let contract_address = client.address.clone();
    env.ledger().with_mut(|li| {
        li.balance.insert(contract_address.clone(), 5_000_0000); // 5 XLM in stroops
    });

    // Check rent balance
    client.check_rent_balance().unwrap();

    // Advance time
    set_timestamp(&env, 2000);

    // Check rent balance again
    client.check_rent_balance().unwrap();

    // The timestamp should be updated to the latest check time
    // (We can't directly access the timestamp storage, but we can verify the function works)
    assert!(!client.is_rent_preservation_mode());
}

#[test]
fn test_edge_case_exact_threshold_balance() {
    let env = Env::default();
    env.mock_all_auths();
    let (admin, _grant_token_addr, _treasury, _oracle, _native, client) = setup_test(&env);

    // Set balance exactly at the threshold (3 XLM)
    let contract_address = client.address.clone();
    env.ledger().with_mut(|li| {
        li.balance.insert(contract_address.clone(), 3_000_0000); // Exactly 3 XLM in stroops
    });

    // Should be healthy (>= threshold)
    assert!(client.check_rent_balance().unwrap());
    assert!(!client.is_rent_preservation_mode());

    // Reduce by 1 stroop to be below threshold
    env.ledger().with_mut(|li| {
        li.balance.insert(contract_address.clone(), 2_999_9999); // Just below 3 XLM
    });

    // Should trigger rent preservation mode
    assert!(!client.check_rent_balance().unwrap());
    assert!(client.is_rent_preservation_mode());
}
