#![cfg(test)]

use soroban_sdk::{symbol_short, Address, Env, Vec, Map, u128};
use crate::staking_multiplier::{
    StakingMultiplierContract, StakingMultiplierContractClient, StakingMultiplierError, StakingMultiplierDataKey,
    StakingInfo, WeightedDonation, WeightedProjectDonations, VestingVaultQuery, QueryStatus,
    STAKING_WEIGHT_PRECISION, BASE_MATCHING_WEIGHT, MAX_BOOST_MULTIPLIER, MIN_STAKE_FOR_BOOST,
    BOOST_TIER_1_THRESHOLD, BOOST_TIER_2_THRESHOLD, BOOST_TIER_3_THRESHOLD, BOOST_TIER_4_THRESHOLD
};

#[test]
fn test_staking_multiplier_initialization() {
    let env = Env::default();
    let admin = Address::generate(&env);
    let vesting_vault = Address::generate(&env);
    
    let contract_id = env.register_contract(None, StakingMultiplierContract);
    let client = StakingMultiplierClient::new(&env, &contract_id);
    
    // Test successful initialization
    client.initialize(&admin, &vesting_vault);
    
    // Verify admin is set
    let stored_admin = env.storage().instance().get(&StakingMultiplierDataKey::Admin).unwrap();
    assert_eq!(stored_admin, admin);
    
    // Verify vesting vault is set
    let stored_vault = env.storage().instance().get(&StakingMultiplierDataKey::VestingVaultContract).unwrap();
    assert_eq!(stored_vault, vesting_vault);
    
    // Verify next query ID is initialized
    let next_query_id = env.storage().instance().get(&StakingMultiplierDataKey::NextQueryId).unwrap();
    assert_eq!(next_query_id, 1);
}

#[test]
fn test_staking_multiplier_double_initialization() {
    let env = Env::default();
    let admin = Address::generate(&env);
    let vesting_vault = Address::generate(&env);
    
    let contract_id = env.register_contract(None, StakingMultiplierContract);
    let client = StakingMultiplierClient::new(&env, &contract_id);
    
    // First initialization should succeed
    client.initialize(&admin, &vesting_vault);
    
    // Second initialization should fail
    let result = client.try_initialize(&admin, &vesting_vault);
    assert_eq!(result, Err(StakingMultiplierError::NotInitialized));
}

#[test]
fn test_add_project_token() {
    let env = Env::default();
    let admin = Address::generate(&env);
    let vesting_vault = Address::generate(&env);
    let project_token = Address::generate(&env);
    
    let contract_id = env.register_contract(None, StakingMultiplierContract);
    let client = StakingMultiplierClient::new(&env, &contract_id);
    
    client.initialize(&admin, &vesting_vault);
    
    // Add project token
    client.add_project_token(&admin, &project_token);
    
    // Verify project token is supported
    let is_supported = env.storage().instance().get(&StakingMultiplierDataKey::ProjectToken(project_token)).unwrap();
    assert!(is_supported);
}

#[test]
fn test_query_vesting_stake_with_cache() {
    let env = Env::default();
    let admin = Address::generate(&env);
    let vesting_vault = Address::generate(&env);
    let donor = Address::generate(&env);
    let project_token = Address::generate(&env);
    
    let contract_id = env.register_contract(None, StakingMultiplierContract);
    let client = StakingMultiplierClient::new(&env, &contract_id);
    
    client.initialize(&admin, &vesting_vault);
    client.add_project_token(&admin, &project_token);
    
    // First query should hit vesting vault
    let weight1 = client.query_vesting_stake(&donor, &project_token);
    assert!(weight1.is_ok());
    
    // Second query should use cache
    let weight2 = client.query_vesting_stake(&donor, &project_token);
    assert!(weight2.is_ok());
    assert_eq!(weight1.unwrap(), weight2.unwrap());
    
    // Verify cache was set
    let cached_info = client.get_cached_staking_info(&donor, &project_token);
    assert!(cached_info.is_ok());
    assert!(cached_info.unwrap().is_some());
}

#[test]
fn test_query_vesting_stake_unsupported_token() {
    let env = Env::default();
    let admin = Address::generate(&env);
    let vesting_vault = Address::generate(&env);
    let donor = Address::generate(&env);
    let unsupported_token = Address::generate(&env);
    
    let contract_id = env.register_contract(None, StakingMultiplierContract);
    let client = StakingMultiplierClient::new(&env, &contract_id);
    
    client.initialize(&admin, &vesting_vault);
    
    // Try query with unsupported token
    let result = client.try_query_vesting_stake(&donor, &unsupported_token);
    assert_eq!(result, Err(StakingMultiplierError::InvalidProjectToken));
}

#[test]
fn test_apply_staking_weights() {
    let env = Env::default();
    let admin = Address::generate(&env);
    let vesting_vault = Address::generate(&env);
    let donor1 = Address::generate(&env);
    let donor2 = Address::generate(&env);
    let project_token = Address::generate(&env);
    let usdc = Address::generate(&env);
    
    let contract_id = env.register_contract(None, StakingMultiplierContract);
    let client = StakingMultiplierClient::new(&env, &contract_id);
    
    client.initialize(&admin, &vesting_vault);
    client.add_project_token(&admin, &project_token);
    
    // Create test donations
    let donation1 = crate::multi_token_matching::Donation {
        donor: donor1.clone(),
        token_address: usdc,
        amount: 10000u128,
        normalized_value: 10000000000000u128, // 10,000 * 1,000,000 precision
        timestamp: env.ledger().timestamp(),
        round_id: 1,
        project_id: 123,
    };
    
    let donation2 = crate::multi_token_matching::Donation {
        donor: donor2.clone(),
        token_address: usdc,
        amount: 5000u128,
        normalized_value: 5000000000000u128, // 5,000 * 1,000,000 precision
        timestamp: env.ledger().timestamp(),
        round_id: 1,
        project_id: 123,
    };
    
    let donations = Vec::from_array(&env, [donation1, donation2]);
    
    // Apply staking weights
    let weighted_donations = client.apply_staking_weights(donations, project_token);
    assert!(weighted_donations.is_ok());
    
    let weighted = weighted_donations.unwrap();
    assert_eq!(weighted.len(), 2);
    
    // Verify weights were applied
    for weighted_donation in weighted.iter() {
        assert!(weighted_donation.weighted_amount >= weighted_donation.base_donation.normalized_value);
        assert!(weighted_donation.boost_multiplier >= BASE_MATCHING_WEIGHT);
    }
}

#[test]
fn test_calculate_boost_multiplier() {
    // Test different stake amounts and their corresponding boost multipliers
    let test_cases = vec![
        (0u128, BASE_MATCHING_WEIGHT),          // 0 stake = 1x boost
        (50000u128, BASE_MATCHING_WEIGHT),      // Below minimum = 1x boost
        (100000u128, 1200000u128),             // Tier 1 = 1.2x boost
        (500000u128, 1200000u128),             // Tier 1 = 1.2x boost
        (1000000u128, 1500000u128),            // Tier 2 = 1.5x boost
        (5000000u128, 1500000u128),            // Tier 2 = 1.5x boost
        (10000000u128, 2000000u128),           // Tier 3 = 2.0x boost
        (50000000u128, 2000000u128),           // Tier 3 = 2.0x boost
        (100000000u128, 3000000u128),         // Tier 4 = 3.0x boost (max)
        (500000000u128, 3000000u128),         // Tier 4 = 3.0x boost (max)
    ];
    
    for (stake_amount, expected_multiplier) in test_cases {
        let multiplier = StakingMultiplierContract::calculate_boost_multiplier(stake_amount);
        assert_eq!(multiplier, expected_multiplier, "Failed for stake amount: {}", stake_amount);
    }
}

#[test]
fn test_weighted_quadratic_matching() {
    let env = Env::default();
    
    // Create test weighted project donations
    let project1_donations = WeightedProjectDonations {
        project_id: 1,
        base_total: 1000000u128, // Base donations: 1M
        weighted_total: 1500000u128, // Weighted donations: 1.5M (due to staking)
        unique_donors: 3,
        weighted_donations: Vec::new(&env),
        matching_amount: 0,
        final_payout: 0,
        total_boost_applied: 1500000u128, // 1.5x total boost
    };
    
    let project2_donations = WeightedProjectDonations {
        project_id: 2,
        base_total: 500000u128,  // Base donations: 0.5M
        weighted_total: 500000u128,  // Weighted donations: 0.5M (no staking)
        unique_donors: 2,
        weighted_donations: Vec::new(&env),
        matching_amount: 0,
        final_payout: 0,
        total_boost_applied: 1000000u128, // 1.0x total boost
    };
    
    let project_donations = Map::from_array(&env, [
        (1u64, project1_donations),
        (2u64, project2_donations),
    ]);
    
    let matching_pool = 1000000u128; // 1M matching pool
    let quadratic_coefficient = 1000000u128; // 1.0 coefficient
    
    // Calculate weighted matching
    let results = StakingMultiplierContract::calculate_weighted_matching(
        &env,
        &project_donations,
        matching_pool,
        quadratic_coefficient,
    );
    
    assert!(results.is_ok());
    
    let matching_results = results.unwrap();
    assert_eq!(matching_results.len(), 2);
    
    // Project 1 should get more matching due to higher weighted total
    let project1_matching = matching_results.get(1u64).unwrap();
    let project2_matching = matching_results.get(2u64).unwrap();
    
    assert!(project1_matching > project2_matching);
    
    // Total matching should not exceed pool
    let total_matching = project1_matching + project2_matching;
    assert!(total_matching <= matching_pool);
}

#[test]
fn test_solvency_protection() {
    let env = Env::default();
    
    // Create project donations that would exceed matching pool
    let project_donations = WeightedProjectDonations {
        project_id: 1,
        base_total: 5000000u128, // Base donations: 5M
        weighted_total: 10000000u128, // Weighted donations: 10M (very high)
        unique_donors: 10,
        weighted_donations: Vec::new(&env),
        matching_amount: 0,
        final_payout: 0,
        total_boost_applied: 2000000u128, // 2x boost
    };
    
    let project_donations = Map::from_array(&env, [(1u64, project_donations)]);
    
    let matching_pool = 1000000u128; // Only 1M matching pool
    let quadratic_coefficient = 1000000u128;
    
    // Calculate weighted matching with solvency protection
    let results = StakingMultiplierContract::calculate_weighted_matching(
        &env,
        &project_donations,
        matching_pool,
        quadratic_coefficient,
    );
    
    assert!(results.is_ok());
    
    let matching_results = results.unwrap();
    assert_eq!(matching_results.len(), 1);
    
    // Should only allocate what's available in the pool
    let allocated_matching = matching_results.get(1u64).unwrap();
    assert!(allocated_matching <= matching_pool);
}

#[test]
fn test_integer_square_root() {
    let test_values = vec![
        (0u128, 0u128),
        (1u128, 1u128),
        (4u128, 2u128),
        (9u128, 3u128),
        (16u128, 4u128),
        (25u128, 5u128),
        (100u128, 10u128),
        (10000u128, 100u128),
        (1000000u128, 1000u128),
        (100000000u128, 10000u128),
    ];
    
    for (input, expected) in test_values {
        let result = StakingMultiplierContract::integer_square_root(input).unwrap();
        assert_eq!(result, expected, "Failed for input: {}", input);
    }
}

#[test]
fn test_get_donor_staking_weight() {
    let env = Env::default();
    let admin = Address::generate(&env);
    let vesting_vault = Address::generate(&env);
    let donor = Address::generate(&env);
    let project_token = Address::generate(&env);
    
    let contract_id = env.register_contract(None, StakingMultiplierContract);
    let client = StakingMultiplierClient::new(&env, &contract_id);
    
    client.initialize(&admin, &vesting_vault);
    client.add_project_token(&admin, &project_token);
    
    // Query staking weight
    let weight = client.get_donor_staking_weight(&donor, &project_token);
    assert!(weight.is_ok());
    
    // Weight should be at least base weight
    let weight_value = weight.unwrap();
    assert!(weight_value >= BASE_MATCHING_WEIGHT);
}

#[test]
fn test_get_cached_staking_info() {
    let env = Env::default();
    let admin = Address::generate(&env);
    let vesting_vault = Address::generate(&env);
    let donor = Address::generate(&env);
    let project_token = Address::generate(&env);
    
    let contract_id = env.register_contract(None, StakingMultiplierContract);
    let client = StakingMultiplierClient::new(&env, &contract_id);
    
    client.initialize(&admin, &vesting_vault);
    client.add_project_token(&admin, &project_token);
    
    // Initially no cache
    let cached_info = client.get_cached_staking_info(&donor, &project_token);
    assert!(cached_info.is_ok());
    assert!(cached_info.unwrap().is_none());
    
    // Query to populate cache
    client.query_vesting_stake(&donor, &project_token).unwrap();
    
    // Now cache should exist
    let cached_info = client.get_cached_staking_info(&donor, &project_token);
    assert!(cached_info.is_ok());
    assert!(cached_info.unwrap().is_some());
    
    let info = cached_info.unwrap().unwrap();
    assert_eq!(info.donor, donor);
    assert_eq!(info.project_token, project_token);
    assert!(info.weight_multiplier >= BASE_MATCHING_WEIGHT);
}

#[test]
fn test_clear_expired_cache() {
    let env = Env::default();
    let admin = Address::generate(&env);
    let vesting_vault = Address::generate(&env);
    let donor = Address::generate(&env);
    let project_token = Address::generate(&env);
    
    let contract_id = env.register_contract(None, StakingMultiplierContract);
    let client = StakingMultiplierClient::new(&env, &contract_id);
    
    client.initialize(&admin, &vesting_vault);
    client.add_project_token(&admin, &project_token);
    
    // Query to populate cache
    client.query_vesting_stake(&donor, &project_token).unwrap();
    
    // Verify cache exists
    let cached_info = client.get_cached_staking_info(&donor, &project_token);
    assert!(cached_info.unwrap().is_some());
    
    // Clear cache
    let cleared_count = client.clear_expired_cache();
    assert!(cleared_count.is_ok());
    assert_eq!(cleared_count.unwrap(), 1);
    
    // Cache should be expired now
    let cached_info = client.get_cached_staking_info(&donor, &project_token);
    assert!(cached_info.unwrap().is_none());
}

#[test]
fn test_edge_cases() {
    let env = Env::default();
    let admin = Address::generate(&env);
    let vesting_vault = Address::generate(&env);
    
    let contract_id = env.register_contract(None, StakingMultiplierContract);
    let client = StakingMultiplierClient::new(&env, &contract_id);
    
    client.initialize(&admin, &vesting_vault);
    
    // Test with empty donations
    let empty_donations = Vec::new(&env);
    let project_token = Address::generate(&env);
    
    let weighted_donations = client.apply_staking_weights(empty_donations, project_token);
    assert!(weighted_donations.is_ok());
    assert_eq!(weighted_donations.unwrap().len(), 0);
    
    // Test with empty project donations
    let empty_project_donations = Map::new(&env);
    let matching_pool = 1000000u128;
    let quadratic_coefficient = 1000000u128;
    
    let results = client.calculate_weighted_matching(&env, &empty_project_donations, matching_pool, quadratic_coefficient);
    assert!(results.is_ok());
    assert_eq!(results.unwrap().len(), 0);
    
    // Test integer square root edge cases
    assert_eq!(StakingMultiplierContract::integer_square_root(0).unwrap(), 0);
    assert_eq!(StakingMultiplierContract::integer_square_root(1).unwrap(), 1);
    assert_eq!(StakingMultiplierContract::integer_square_root(u128::MAX).unwrap(), u128::MAX);
}

#[test]
fn test_boost_multiplier_limits() {
    // Test that boost multiplier never exceeds maximum
    let extreme_stake = u128::MAX;
    let multiplier = StakingMultiplierContract::calculate_boost_multiplier(extreme_stake);
    assert_eq!(multiplier, MAX_BOOST_MULTIPLIER);
    
    // Test minimum stake threshold
    let below_minimum = MIN_STAKE_FOR_BOOST - 1;
    let multiplier = StakingMultiplierContract::calculate_boost_multiplier(below_minimum);
    assert_eq!(multiplier, BASE_MATCHING_WEIGHT);
    
    // Test exactly at minimum
    let at_minimum = MIN_STAKE_FOR_BOOST;
    let multiplier = StakingMultiplierContract::calculate_boost_multiplier(at_minimum);
    assert_eq!(multiplier, 1200000u128); // 1.2x boost
}

#[test]
fn test_multiple_donors_same_project() {
    let env = Env::default();
    let admin = Address::generate(&env);
    let vesting_vault = Address::generate(&env);
    let donor1 = Address::generate(&env);
    let donor2 = Address::generate(&env);
    let donor3 = Address::generate(&env);
    let project_token = Address::generate(&env);
    let usdc = Address::generate(&env);
    
    let contract_id = env.register_contract(None, StakingMultiplierContract);
    let client = StakingMultiplierClient::new(&env, &contract_id);
    
    client.initialize(&admin, &vesting_vault);
    client.add_project_token(&admin, &project_token);
    
    // Create donations from multiple donors
    let donation1 = crate::multi_token_matching::Donation {
        donor: donor1.clone(),
        token_address: usdc,
        amount: 10000u128,
        normalized_value: 10000000000000u128,
        timestamp: env.ledger().timestamp(),
        round_id: 1,
        project_id: 123,
    };
    
    let donation2 = crate::multi_token_matching::Donation {
        donor: donor2.clone(),
        token_address: usdc,
        amount: 5000u128,
        normalized_value: 5000000000000u128,
        timestamp: env.ledger().timestamp(),
        round_id: 1,
        project_id: 123,
    };
    
    let donation3 = crate::multi_token_matching::Donation {
        donor: donor3.clone(),
        token_address: usdc,
        amount: 2000u128,
        normalized_value: 2000000000000u128,
        timestamp: env.ledger().timestamp(),
        round_id: 1,
        project_id: 123,
    };
    
    let donations = Vec::from_array(&env, [donation1, donation2, donation3]);
    
    // Apply staking weights
    let weighted_donations = client.apply_staking_weights(donations, project_token.clone());
    assert!(weighted_donations.is_ok());
    
    let weighted = weighted_donations.unwrap();
    assert_eq!(weighted.len(), 3);
    
    // Verify each donation has appropriate weight
    for weighted_donation in weighted.iter() {
        assert!(weighted_donation.weighted_amount >= weighted_donation.base_donation.normalized_value);
        assert!(weighted_donation.boost_multiplier >= BASE_MATCHING_WEIGHT);
        assert!(weighted_donation.boost_multiplier <= MAX_BOOST_MULTIPLIER);
    }
    
    // Calculate weighted matching for the project
    let mut project_donations = Map::new(&env);
    
    let project_weighted = WeightedProjectDonations {
        project_id: 123,
        base_total: 17000000000000u128, // 17,000 total
        weighted_total: weighted.iter().map(|d| d.weighted_amount).sum(),
        unique_donors: 3,
        weighted_donations: weighted.clone(),
        matching_amount: 0,
        final_payout: 0,
        total_boost_applied: weighted.iter().map(|d| d.boost_multiplier).sum(),
    };
    
    project_donations.set(123u64, project_weighted);
    
    let matching_pool = 500000u128;
    let quadratic_coefficient = 1000000u128;
    
    let results = client.calculate_weighted_matching(&env, &project_donations, matching_pool, quadratic_coefficient);
    assert!(results.is_ok());
    
    let matching_results = results.unwrap();
    assert_eq!(matching_results.len(), 1);
    
    let project_matching = matching_results.get(123u64).unwrap();
    assert!(project_matching <= matching_pool);
}
