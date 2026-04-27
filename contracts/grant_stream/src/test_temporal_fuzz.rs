#![cfg(test)]

use super::{GrantStreamContract, GrantStreamContractClient, GrantStatus, SCALING_FACTOR};
use soroban_sdk::{
    testutils::{Address as _, Ledger},
    token, Address, Env, Vec, Symbol,
};
use proptest::prelude::*;
use std::collections::HashMap;

// Constants for temporal fuzz testing
const SECONDS_PER_MINUTE: u64 = 60;
const SECONDS_PER_HOUR: u64 = 3600;
const SECONDS_PER_DAY: u64 = 86400;
const SECONDS_PER_YEAR: u64 = 365 * SECONDS_PER_DAY;
const MAX_TEST_DURATION: u64 = 10 * SECONDS_PER_YEAR; // 10 years
const MIN_TIME_JUMP: u64 = 1; // 1 second

// Test scenarios with different grant configurations
#[derive(Debug, Clone)]
struct TemporalGrantConfig {
    total_amount: i128,
    flow_rate: i128,
    start_time: u64,
    warmup_duration: u64,
    has_validator: bool,
}

#[derive(Debug, Clone)]
struct TemporalTestState {
    env: Env,
    admin: Address,
    grant_token: Address,
    treasury: Address,
    oracle: Address,
    native_token: Address,
    client: GrantStreamContractClient,
    grant_configs: Vec<TemporalGrantConfig>,
    grant_ids: Vec<u64>,
    recipients: Vec<Address>,
    validators: Vec<Option<Address>>,
    initial_contract_balance: i128,
    total_allocated: i128,
}

impl TemporalTestState {
    fn new() -> Self {
        let env = Env::default();
        env.mock_all_auths();
        
        let admin = Address::generate(&env);
        let grant_token_addr = env.register_stellar_asset_contract_v2(admin.clone());
        let native_token_addr = env.register_stellar_asset_contract_v2(admin.clone());
        let treasury = Address::generate(&env);
        let oracle = Address::generate(&env);

        let contract_id = env.register(GrantStreamContract, ());
        let client = GrantStreamContractClient::new(&env, &contract_id);

        client.initialize(&admin, &grant_token_addr.address(), &treasury, &oracle, &native_token_addr.address());

        Self {
            env,
            admin,
            grant_token: grant_token_addr.address(),
            treasury,
            oracle,
            native_token: native_token_addr.address(),
            client,
            grant_configs: Vec::new(&env),
            grant_ids: Vec::new(&env),
            recipients: Vec::new(&env),
            validators: Vec::new(&env),
            initial_contract_balance: 0,
            total_allocated: 0,
        }
    }

    fn setup_grants(&mut self, configs: &[TemporalGrantConfig]) -> Result<(), Box<dyn std::error::Error>> {
        let grant_token = token::Client::new(&self.env, &self.grant_token);
        let grant_token_admin = token::StellarAssetClient::new(&self.env, &self.grant_token);

        // Set initial timestamp
        self.set_timestamp(1000);

        let mut total_needed = 0i128;

        // Create recipients and validators
        for config in configs {
            let recipient = Address::generate(&self.env);
            self.recipients.push_back(recipient);
            
            let validator = if config.has_validator {
                Some(Address::generate(&self.env))
            } else {
                None
            };
            self.validators.push_back(validator);
            
            total_needed += config.total_amount;
        }

        // Mint total amount to contract
        grant_token_admin.mint(&self.client.address, &total_needed);
        self.initial_contract_balance = total_needed;
        self.total_allocated = total_needed;

        // Create grants
        for (i, config) in configs.iter().enumerate() {
            let grant_id = (i + 1) as u64;
            
            self.client.create_grant(
                &grant_id,
                &self.recipients.get(i).unwrap(),
                &config.total_amount,
                &config.flow_rate,
                &config.warmup_duration,
                &self.validators.get(i).unwrap(),
            );
            
            self.grant_ids.push_back(grant_id);
            self.grant_configs.push_back(config.clone());
        }

        Ok(())
    }

    fn set_timestamp(&self, timestamp: u64) {
        self.env.ledger().with_mut(|li| {
            li.timestamp = timestamp;
        });
    }

    fn get_contract_balance(&self) -> i128 {
        let grant_token = token::Client::new(&self.env, &self.grant_token);
        grant_token.balance(&self.client.address)
    }

    fn get_recipient_balance(&self, index: usize) -> i128 {
        let grant_token = token::Client::new(&self.env, &self.grant_token);
        grant_token.balance(self.recipients.get(index).unwrap())
    }

    fn get_validator_balance(&self, index: usize) -> i128 {
        if let Some(validator) = self.validators.get(index).unwrap() {
            let grant_token = token::Client::new(&self.env, &self.grant_token);
            grant_token.balance(validator)
        } else {
            0
        }
    }

    fn verify_temporal_invariant(&self, current_time: u64) -> Result<(), String> {
        let mut total_withdrawn = 0i128;
        let mut total_claimable = 0i128;
        let mut total_validator_withdrawn = 0i128;
        let mut total_validator_claimable = 0i128;

        for (i, grant_id) in self.grant_ids.iter().enumerate() {
            let grant = self.client.get_grant(*grant_id);
            let config = &self.grant_configs.get(i).unwrap();

            // Verify basic invariants
            let total_accounted = grant.withdrawn
                .checked_add(grant.claimable)
                .ok_or("Math overflow in grantee accounting")?
                .checked_add(grant.validator_withdrawn)
                .ok_or("Math overflow in validator accounting")?
                .checked_add(grant.validator_claimable)
                .ok_or("Math overflow in total accounting")?;

            if total_accounted > grant.total_amount {
                return Err(format!(
                    "Grant {}: total accounted {} exceeds total_amount {}",
                    grant_id, total_accounted, grant.total_amount
                ));
            }

            // Verify that withdrawn never exceeds total allocation
            if grant.withdrawn > grant.total_amount {
                return Err(format!(
                    "Grant {}: withdrawn {} exceeds total_amount {}",
                    grant_id, grant.withdrawn, grant.total_amount
                ));
            }

            // Verify temporal boundaries
            if current_time < config.start_time {
                // Before start time, no tokens should have been accrued
                if grant.withdrawn > 0 || grant.claimable > 0 || 
                   grant.validator_withdrawn > 0 || grant.validator_claimable > 0 {
                    return Err(format!(
                        "Grant {}: has activity before start_time {} (current: {})",
                        grant_id, config.start_time, current_time
                    ));
                }
            }

            // Verify that the flow calculation doesn't create extra tokens
            let expected_max_flow = self.calculate_expected_max_flow(config, current_time);
            let actual_total_flow = grant.withdrawn + grant.claimable + 
                                  grant.validator_withdrawn + grant.validator_claimable;
            
            if actual_total_flow > expected_max_flow {
                return Err(format!(
                    "Grant {}: actual flow {} exceeds expected max {} at time {}",
                    grant_id, actual_total_flow, expected_max_flow, current_time
                ));
            }

            total_withdrawn += grant.withdrawn;
            total_claimable += grant.claimable;
            total_validator_withdrawn += grant.validator_withdrawn;
            total_validator_claimable += grant.validator_claimable;
        }

        // Verify global invariant: total tokens never exceed initial allocation
        let contract_balance = self.get_contract_balance();
        let mut total_user_balances = 0i128;
        
        for i in 0..self.recipients.len() {
            total_user_balances += self.get_recipient_balance(i);
            total_user_balances += self.get_validator_balance(i);
        }

        let total_accounted_global = contract_balance + total_user_balances;
        if total_accounted_global > self.initial_contract_balance {
            return Err(format!(
                "Global invariant violation: total accounted {} exceeds initial {}",
                total_accounted_global, self.initial_contract_balance
            ));
        }

        Ok(())
    }

    fn calculate_expected_max_flow(&self, config: &TemporalGrantConfig, current_time: u64) -> i128 {
        if current_time <= config.start_time {
            return 0;
        }

        let elapsed = current_time - config.start_time;
        
        // Calculate base flow
        let base_flow = config.flow_rate.checked_mul(elapsed as i128).unwrap_or(i128::MAX);

        // Apply warmup multiplier
        let warmup_multiplier = if config.warmup_duration == 0 {
            10000 // 100%
        } else {
            let warmup_end = config.start_time + config.warmup_duration;
            if current_time >= warmup_end {
                10000
            } else if current_time <= config.start_time {
                2500 // 25%
            } else {
                let elapsed_warmup = current_time - config.start_time;
                let progress = (elapsed_warmup as i128 * 10000) / (config.warmup_duration as i128);
                2500 + (7500 * progress) / 10000
            }
        };

        let adjusted_flow = base_flow.checked_mul(warmup_multiplier).unwrap_or(i128::MAX) / 10000;

        // Cap at total amount
        adjusted_flow.min(config.total_amount)
    }

    fn simulate_time_jump_with_withdrawals(&mut self, time_jump: u64, withdraw_probability: f64) -> Result<(), String> {
        let current_time = self.env.ledger().timestamp();
        let new_time = current_time + time_jump;
        self.set_timestamp(new_time);

        // Randomly perform withdrawals after time jump
        for (i, grant_id) in self.grant_ids.iter().enumerate() {
            if proptest::sample::Index::new(proptest::rng::Rng::new_from_seed(&[0; 32]), 1).sample(&mut proptest::rng::Rng::new_from_seed(&[0; 32])).unwrap() < (withdraw_probability * 100.0) as usize {
                let claimable = self.client.claimable(*grant_id);
                if claimable > 0 {
                    // Withdraw a random amount up to claimable
                    let withdraw_amount = if claimable > 1 {
                        (proptest::sample::Index::new(proptest::rng::Rng::new_from_seed(&[0; 32]), claimable as usize).sample(&mut proptest::rng::Rng::new_from_seed(&[0; 32])).unwrap() as i128 + 1).min(claimable)
                    } else {
                        claimable
                    };
                    
                    self.client.withdraw(grant_id, &withdraw_amount);
                }
            }

            // Also try validator withdrawals
            if self.validators.get(i).unwrap().is_some() {
                let validator_claimable = self.client.validator_claimable(*grant_id);
                if validator_claimable > 0 {
                    let withdraw_amount = if validator_claimable > 1 {
                        (proptest::sample::Index::new(proptest::rng::Rng::new_from_seed(&[0; 32]), validator_claimable as usize).sample(&mut proptest::rng::Rng::new_from_seed(&[0; 32])).unwrap() as i128 + 1).min(validator_claimable)
                    } else {
                        validator_claimable
                    };
                    
                    self.client.withdraw_validator(grant_id, &withdraw_amount);
                }
            }
        }

        Ok(())
    }
}

// Generate diverse grant configurations for temporal testing
fn generate_grant_configs() -> Vec<TemporalGrantConfig> {
    vec![
        // Standard grant
        TemporalGrantConfig {
            total_amount: 1000000 * SCALING_FACTOR,
            flow_rate: 10 * SCALING_FACTOR,
            start_time: 1000,
            warmup_duration: 0,
            has_validator: false,
        },
        // Grant with warmup
        TemporalGrantConfig {
            total_amount: 2000000 * SCALING_FACTOR,
            flow_rate: 20 * SCALING_FACTOR,
            start_time: 1000,
            warmup_duration: 7 * SECONDS_PER_DAY,
            has_validator: false,
        },
        // Grant with validator
        TemporalGrantConfig {
            total_amount: 1500000 * SCALING_FACTOR,
            flow_rate: 15 * SCALING_FACTOR,
            start_time: 1000,
            warmup_duration: 0,
            has_validator: true,
        },
        // Grant with warmup and validator
        TemporalGrantConfig {
            total_amount: 3000000 * SCALING_FACTOR,
            flow_rate: 30 * SCALING_FACTOR,
            start_time: 1000,
            warmup_duration: 14 * SECONDS_PER_DAY,
            has_validator: true,
        },
        // Micro-stream grant
        TemporalGrantConfig {
            total_amount: 100000 * SCALING_FACTOR,
            flow_rate: 1 * SCALING_FACTOR,
            start_time: 1000,
            warmup_duration: 0,
            has_validator: false,
        },
    ]
}

// Main temporal invariant fuzz test
proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]
    
    #[test]
    fn test_temporal_invariant_random_time_jumps(
        // Random time jumps from 1 second to 10 years
        time_jumps in prop::collection::vec(
            MIN_TIME_JUMP..=MAX_TEST_DURATION,
            10..=50 // 10-50 time jumps per test
        ),
        // Withdrawal probability after each time jump
        withdraw_probability in 0.0f64..1.0f64,
        // Whether to include warmup periods
        include_warmup in prop::bool::ANY,
        // Whether to include validators
        include_validators in prop::bool::ANY
    ) {
        let mut configs = generate_grant_configs();
        
        // Filter configs based on test parameters
        configs.retain(|c| {
            (!include_warmup || c.warmup_duration == 0) && 
            (!include_validators || !c.has_validator)
        });
        
        if configs.is_empty() {
            return; // Skip test if no configs match
        }
        
        let mut state = TemporalTestState::new();
        state.setup_grants(&configs).expect("Failed to setup grants");
        
        let mut current_time = 1000;
        
        // Apply time jumps sequentially
        for (i, time_jump) in time_jumps.iter().enumerate() {
            // Limit total test duration to prevent excessive execution
            if current_time + time_jump > 1000 + MAX_TEST_DURATION {
                break;
            }
            
            state.simulate_time_jump_with_withdrawals(*time_jump, withdraw_probability)
                .unwrap_or_else(|e| panic!("Time jump simulation failed at step {}: {}", i, e));
            
            current_time += time_jump;
            
            // Verify temporal invariants after each time jump
            state.verify_temporal_invariant(current_time)
                .unwrap_or_else(|e| panic!("Temporal invariant violation at time {}: {}", current_time, e));
        }
        
        // Final verification at end of test
        state.verify_temporal_invariant(current_time)
            .unwrap_or_else(|e| panic!("Final temporal invariant violation: {}", e));
    }
}

// Boundary testing for stream Start and End
proptest! {
    #![proptest_config(ProptestConfig::with_cases(50))]
    
    #[test]
    fn test_stream_start_end_boundaries(
        // Test times around stream boundaries
        boundary_offset in -1000i64..=1000i64,
        // Grant duration
        duration_days in 1u64..=365u64,
        // Flow rate
        flow_rate in 1i128..=1000i128,
        // Whether to test start or end boundary
        test_start_boundary in prop::bool::ANY
    ) {
        let config = TemporalGrantConfig {
            total_amount: flow_rate * duration_days as i128 * SECONDS_PER_DAY as i128,
            flow_rate: flow_rate * SCALING_FACTOR,
            start_time: 1000,
            warmup_duration: 0,
            has_validator: false,
        };
        
        let mut state = TemporalTestState::new();
        state.setup_grants(&[config.clone()]).expect("Failed to setup grant");
        
        let test_time = if test_start_boundary {
            // Test around start time
            (config.start_time as i64 + boundary_offset).max(0) as u64
        } else {
            // Test around end time
            let end_time = config.start_time + duration_days * SECONDS_PER_DAY;
            (end_time as i64 + boundary_offset).max(0) as u64
        };
        
        state.set_timestamp(test_time);
        
        // Verify boundary conditions
        state.verify_temporal_invariant(test_time)
            .unwrap_or_else(|e| panic!("Boundary invariant violation at time {}: {}", test_time, e));
        
        // Additional boundary-specific checks
        let grant = state.client.get_grant(1);
        
        if test_time <= config.start_time {
            // Before or at start: no withdrawals should be possible
            assert!(grant.withdrawn == 0, "Withdrawal before start time");
            assert!(grant.claimable == 0, "Claimable before start time");
        }
        
        if test_start_boundary && test_time == config.start_time {
            // Exactly at start time
            let claimable = state.client.claimable(1);
            // Should be minimal (possibly 0 or very small due to warmup)
            assert!(claimable <= config.flow_rate, "Excessive claimable at start time");
        }
    }
}

// Stress test for maximum duration (10 years)
#[test]
fn test_maximum_duration_temporal_invariant() {
    let config = TemporalGrantConfig {
        total_amount: 3650 * SCALING_FACTOR, // 10 years at 1 token/day
        flow_rate: SCALING_FACTOR / SECONDS_PER_DAY, // 1 token per day
        start_time: 1000,
        warmup_duration: 0,
        has_validator: true,
    };
    
    let mut state = TemporalTestState::new();
    state.setup_grants(&[config.clone()]).expect("Failed to setup grant");
    
    // Test at maximum duration
    let max_time = config.start_time + 10 * SECONDS_PER_YEAR;
    state.set_timestamp(max_time);
    
    // Perform full withdrawal
    let claimable = state.client.claimable(1);
    if claimable > 0 {
        state.client.withdraw(&1, &claimable);
    }
    
    let validator_claimable = state.client.validator_claimable(1);
    if validator_claimable > 0 {
        state.client.withdraw_validator(&1, &validator_claimable);
    }
    
    // Verify invariants
    state.verify_temporal_invariant(max_time)
        .expect("Maximum duration temporal invariant violation");
    
    // Verify that total doesn't exceed allocation
    let grant = state.client.get_grant(1);
    let total_accounted = grant.withdrawn + grant.validator_withdrawn;
    assert!(total_accounted <= config.total_amount, 
        "Total exceeded allocation at maximum duration");
}

// Test for mathematical precision over long time periods
#[test]
fn test_long_term_mathematical_precision() {
    let config = TemporalGrantConfig {
        total_amount: i128::MAX / 2, // Very large amount
        flow_rate: 1000000 * SCALING_FACTOR, // High flow rate
        start_time: 1000,
        warmup_duration: 30 * SECONDS_PER_DAY, // 30 day warmup
        has_validator: true,
    };
    
    let mut state = TemporalTestState::new();
    state.setup_grants(&[config.clone()]).expect("Failed to setup grant");
    
    // Test various time points to check for precision loss
    let test_points = vec![
        config.start_time,
        config.start_time + 1, // 1 second after start
        config.start_time + SECONDS_PER_DAY, // 1 day
        config.start_time + 30 * SECONDS_PER_DAY, // End of warmup
        config.start_time + 365 * SECONDS_PER_DAY, // 1 year
        config.start_time + 5 * 365 * SECONDS_PER_DAY, // 5 years
    ];
    
    for test_time in test_points {
        state.set_timestamp(test_time);
        
        // Verify mathematical consistency
        state.verify_temporal_invariant(test_time)
            .unwrap_or_else(|e| panic!("Mathematical precision violation at time {}: {}", test_time, e));
        
        // Check for overflow conditions
        let grant = state.client.get_grant(1);
        assert!(grant.withdrawn >= 0, "Negative withdrawn amount");
        assert!(grant.claimable >= 0, "Negative claimable amount");
        assert!(grant.validator_withdrawn >= 0, "Negative validator withdrawn");
        assert!(grant.validator_claimable >= 0, "Negative validator claimable");
    }
}
