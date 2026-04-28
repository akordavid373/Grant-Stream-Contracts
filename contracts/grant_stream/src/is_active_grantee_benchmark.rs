#![cfg(test)]

use soroban_sdk::{testutils::{Ledger, LedgerInfo}, Address, Env, Vec};
use crate::{GrantStreamContract, GrantStatus, DataKey, Grant};

/// Benchmark for is_active_grantee function to ensure it executes under 5,000 CPU instructions
pub fn benchmark_is_active_grantee() -> (u64, u64, u64) {
    let env = Env::default();
    env.mock_all_auths();
    
    let contract_id = env.register_contract(None, GrantStreamContract);
    let client = GrantStreamContractClient::new(&env, &contract_id);
    
    // Initialize contract
    let admin = Address::generate(&env);
    let grant_token = Address::generate(&env);
    let treasury = Address::generate(&env);
    let oracle = Address::generate(&env);
    let native_token = Address::generate(&env);
    
    client.initialize(&admin, &grant_token, &treasury, &oracle, &native_token);
    
    // Create test addresses
    let active_grantee = Address::generate(&env);
    let inactive_grantee = Address::generate(&env);
    
    // Setup grants for testing
    // Active grant for active_grantee
    client.create_grant(&1u64, &active_grantee, &1000000i128, &100i128, &0u64, &None);
    
    // Completed grant for inactive_grantee
    client.create_grant(&2u64, &inactive_grantee, &1000000i128, &100i128, &0u64, &None);
    // Simulate completion by updating status directly (for benchmark purposes)
    let grant_key = DataKey::Grant(2u64);
    if let Some(mut grant) = env.storage().instance().get::<_, Grant>(&grant_key) {
        grant.status = GrantStatus::Completed;
        env.storage().instance().set(&grant_key, &grant);
    }
    
    let before_cpu = env.budget().cpu_instruction_count();
    
    // Test queries - worst case scenario: checking multiple grants
    let _result1 = client.is_active_grantee(&active_grantee);  // Should return true
    let _result2 = client.is_active_grantee(&inactive_grantee); // Should return false
    let _result3 = client.is_active_grantee(&admin); // Should return false (no grants)
    
    let cpu_cost = (env.budget().cpu_instruction_count() - before_cpu) as u64;
    
    // The function should average under 5,000 CPU instructions per call
    let avg_cpu_per_call = cpu_cost / 3;
    assert!(avg_cpu_per_call < 5000, "is_active_grantee exceeds 5,000 CPU instruction limit: {}", avg_cpu_per_call);
    
    (0, 0, cpu_cost)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_active_grantee_performance() {
        let (gas_used, storage_cost, cpu_cost) = benchmark_is_active_grantee();
        println!("is_active_grantee benchmark results:");
        println!("  Gas used: {}", gas_used);
        println!("  Storage cost: {}", storage_cost);
        println!("  CPU cost: {} (avg per call: {})", cpu_cost, cpu_cost / 3);
        
        // Verify performance requirements
        assert!(cpu_cost / 3 < 5000, "Function exceeds 5,000 CPU instruction limit");
    }

    #[test]
    fn test_is_active_grantee_functionality() {
        let env = Env::default();
        env.mock_all_auths();
        
        let contract_id = env.register_contract(None, GrantStreamContract);
        let client = GrantStreamContractClient::new(&env, &contract_id);
        
        // Initialize contract
        let admin = Address::generate(&env);
        let grant_token = Address::generate(&env);
        let treasury = Address::generate(&env);
        let oracle = Address::generate(&env);
        let native_token = Address::generate(&env);
        
        client.initialize(&admin, &grant_token, &treasury, &oracle, &native_token);
        
        // Create test addresses
        let active_grantee = Address::generate(&env);
        let inactive_grantee = Address::generate(&env);
        let no_grants_user = Address::generate(&env);
        
        // Test 1: User with no grants should return false
        assert!(!client.is_active_grantee(&no_grants_user));
        
        // Test 2: Create an active grant
        client.create_grant(&1u64, &active_grantee, &1000000i128, &100i128, &0u64, &None);
        assert!(client.is_active_grantee(&active_grantee));
        
        // Test 3: Create a completed grant
        client.create_grant(&2u64, &inactive_grantee, &1000000i128, &100i128, &0u64, &None);
        // Simulate completion
        let grant_key = DataKey::Grant(2u64);
        if let Some(mut grant) = env.storage().instance().get::<_, Grant>(&grant_key) {
            grant.status = GrantStatus::Completed;
            env.storage().instance().set(&grant_key, &grant);
        }
        assert!(!client.is_active_grantee(&inactive_grantee));
        
        // Test 4: Test with paused grant (should be active)
        let paused_grantee = Address::generate(&env);
        client.create_grant(&3u64, &paused_grantee, &1000000i128, &100i128, &0u64, &None);
        // Simulate pause
        let grant_key = DataKey::Grant(3u64);
        if let Some(mut grant) = env.storage().instance().get::<_, Grant>(&grant_key) {
            grant.status = GrantStatus::Paused;
            env.storage().instance().set(&grant_key, &grant);
        }
        assert!(client.is_active_grantee(&paused_grantee));
        
        // Test 5: Test with cancelled grant (should not be active)
        let cancelled_grantee = Address::generate(&env);
        client.create_grant(&4u64, &cancelled_grantee, &1000000i128, &100i128, &0u64, &None);
        // Simulate cancellation
        let grant_key = DataKey::Grant(4u64);
        if let Some(mut grant) = env.storage().instance().get::<_, Grant>(&grant_key) {
            grant.status = GrantStatus::Cancelled;
            env.storage().instance().set(&grant_key, &grant);
        }
        assert!(!client.is_active_grantee(&cancelled_grantee));
        
        println!("All functionality tests passed!");
    }
}
