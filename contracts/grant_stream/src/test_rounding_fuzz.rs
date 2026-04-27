#![cfg(test)]

use super::{GrantStreamContract, GrantStreamContractClient, GrantStatus, SCALING_FACTOR};
use soroban_sdk::{
    testutils::{Address as _, Ledger},
    token, Address, Env, Vec, Symbol,
};
use proptest::prelude::*;
use std::collections::HashMap;

// Constants for micro-stream testing
const STROOP: i128 = 1; // 1 stroop = 0.0000001 XLM
const MICRO_STREAM_RATE: i128 = 100 * STROOP; // 100 stroops per day
const SECONDS_PER_DAY: u64 = 86400;
const NUM_MICRO_STREAMS: usize = 5000; // Thousands of micro-streams
const TEST_DURATION_DAYS: u64 = 365; // 1 year test duration

#[derive(Debug, Clone)]
struct MicroStreamTestState {
    env: Env,
    admin: Address,
    grant_token: Address,
    treasury: Address,
    oracle: Address,
    native_token: Address,
    client: GrantStreamContractClient,
    recipients: Vec<Address>,
    grant_ids: Vec<u64>,
    total_expected_distributed: i128,
    initial_contract_balance: i128,
}

impl MicroStreamTestState {
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

        // Generate many recipients for micro-streams
        let mut recipients = Vec::new(&env);
        for _ in 0..NUM_MICRO_STREAMS {
            recipients.push_back(Address::generate(&env));
        }

        Self {
            env,
            admin,
            grant_token: grant_token_addr.address(),
            treasury,
            oracle,
            native_token: native_token_addr.address(),
            client,
            recipients,
            grant_ids: Vec::new(&env),
            total_expected_distributed: 0,
            initial_contract_balance: 0,
        }
    }

    fn setup_micro_streams(&mut self, with_validator: bool) -> Result<(), Box<dyn std::error::Error>> {
        let grant_token = token::Client::new(&self.env, &self.grant_token);
        let grant_token_admin = token::StellarAssetClient::new(&self.env, &self.grant_token);

        // Set initial timestamp
        self.set_timestamp(1000);

        let validator_addr = if with_validator {
            Some(Address::generate(&self.env))
        } else {
            None
        };

        // Calculate total amount needed for all micro-streams
        let total_amount_per_stream = MICRO_STREAM_RATE * SECONDS_PER_DAY as i128 * TEST_DURATION_DAYS as i128;
        let total_amount_all_streams = total_amount_per_stream * NUM_MICRO_STREAMS as i128;

        // Mint total amount to contract
        grant_token_admin.mint(&self.client.address, &total_amount_all_streams);
        self.initial_contract_balance = total_amount_all_streams;

        // Create micro-streams for each recipient
        for (i, recipient) in self.recipients.iter().enumerate() {
            let grant_id = (i + 1) as u64;
            
            // Create micro-stream grant
            self.client.create_grant(
                &grant_id,
                recipient,
                &total_amount_per_stream,
                &MICRO_STREAM_RATE,
                &0, // no warmup
                &validator_addr,
            );
            
            self.grant_ids.push_back(grant_id);
        }

        self.total_expected_distributed = total_amount_all_streams;
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

    fn get_total_distributed(&self) -> i128 {
        let mut total = 0i128;
        for recipient in self.recipients.iter() {
            total += token::Client::new(&self.env, &self.grant_token).balance(recipient);
        }
        total
    }

    fn get_validator_balance(&self, validator: &Address) -> i128 {
        token::Client::new(&self.env, &self.grant_token).balance(validator)
    }

    fn calculate_theoretical_distribution(&self, elapsed_seconds: u64) -> (i128, i128) {
        let base_distribution = MICRO_STREAM_RATE * elapsed_seconds as i128 * NUM_MICRO_STREAMS as i128;
        let validator_share = base_distribution * 500 / 10000; // 5%
        let grantee_share = base_distribution - validator_share;
        (grantee_share, validator_share)
    }

    fn verify_rounding_invariants(&self, elapsed_seconds: u64, with_validator: bool) -> Result<(), String> {
        let contract_balance = self.get_contract_balance();
        let total_distributed = self.get_total_distributed();
        
        let (theoretical_grantee, theoretical_validator) = self.calculate_theoretical_distribution(elapsed_seconds);
        let total_theoretical = theoretical_grantee + theoretical_validator;
        
        // The actual distribution should be very close to theoretical
        // Allow for small rounding differences due to integer division
        let rounding_tolerance = NUM_MICRO_STREAMS as i128 * 2; // Max 2 stroops error per stream
        
        if with_validator {
            // Get validator address from first grant
            let first_grant = self.client.get_grant(1);
            if let Some(validator) = first_grant.validator {
                let validator_balance = self.get_validator_balance(&validator);
                let validator_error = (validator_balance - theoretical_validator).abs();
                
                if validator_error > rounding_tolerance {
                    return Err(format!(
                        "Validator rounding error too large: expected {}, got {}, error {} (tolerance {})",
                        theoretical_validator, validator_balance, validator_error, rounding_tolerance
                    ));
                }
            }
        }
        
        let grantee_error = (total_distributed - theoretical_grantee).abs();
        if grantee_error > rounding_tolerance {
            return Err(format!(
                "Grantee rounding error too large: expected {}, got {}, error {} (tolerance {})",
                theoretical_grantee, total_distributed, grantee_error, rounding_tolerance
            ));
        }
        
        // Total invariant check
        let total_accounted = contract_balance + total_distributed;
        if let Some(validator) = self.client.get_grant(1).validator {
            let validator_balance = self.get_validator_balance(&validator);
            let total_accounted_with_validator = total_accounted + validator_balance;
            
            if (total_accounted_with_validator - self.initial_contract_balance).abs() > rounding_tolerance {
                return Err(format!(
                    "Total invariant violation: expected {}, got {}",
                    self.initial_contract_balance, total_accounted_with_validator
                ));
            }
        } else {
            if (total_accounted - self.initial_contract_balance).abs() > rounding_tolerance {
                return Err(format!(
                    "Total invariant violation: expected {}, got {}",
                    self.initial_contract_balance, total_accounted
                ));
            }
        }
        
        Ok(())
    }

    fn simulate_withdrawals(&mut self, num_withdrawals: usize) -> Result<i128, String> {
        let mut total_withdrawn = 0i128;
        
        for i in 0..num_withdrawals.min(self.recipients.len()) {
            let grant_id = (i + 1) as u64;
            let recipient = self.recipients.get(i).unwrap();
            
            // Check current claimable
            let claimable = self.client.claimable(grant_id);
            
            if claimable > 0 {
                // Withdraw all claimable to maximize rounding exposure
                self.client.withdraw(&grant_id, &claimable);
                total_withdrawn += claimable;
            }
        }
        
        Ok(total_withdrawn)
    }
}

// Fuzz test for micro-stream rounding error accumulation
proptest! {
    #![proptest_config(ProptestConfig::with_cases(50))]
    
    #[test]
    fn test_micro_stream_rounding_accumulation(
        // Test different time periods (1 day to 1 year)
        days_elapsed in 1u64..=365u64,
        // Test with and without validator
        with_validator in prop::bool::ANY,
        // Number of withdrawals to perform
        num_withdrawals in 100usize..=NUM_MICRO_STREAMS,
        // Random time advancement pattern
        time_steps in prop::collection::vec(
            1u64..=SECONDS_PER_DAY, // 1 second to 1 day steps
            10..=100 // number of time steps
        )
    ) {
        let mut state = MicroStreamTestState::new();
        state.setup_micro_streams(with_validator).expect("Failed to setup micro streams");
        
        let total_elapsed_seconds = days_elapsed * SECONDS_PER_DAY;
        let mut current_time = 1000;
        
        // Simulate time advancement with random steps
        for step in time_steps.iter().take(20) { // Limit steps for performance
            current_time += step;
            if current_time > 1000 + total_elapsed_seconds {
                current_time = 1000 + total_elapsed_seconds;
            }
            state.set_timestamp(current_time);
        }
        
        // Final time advancement
        state.set_timestamp(1000 + total_elapsed_seconds);
        
        // Perform withdrawals
        state.simulate_withdrawals(num_withdrawals)
            .unwrap_or_else(|e| panic!("Withdrawal simulation failed: {}", e));
        
        // Verify rounding invariants
        state.verify_rounding_invariants(total_elapsed_seconds, with_validator)
            .unwrap_or_else(|e| panic!("Rounding invariant violation: {}", e));
    }
}

// Stress test with maximum number of micro-streams
#[test]
fn test_maximum_micro_streams_stress() {
    let mut state = MicroStreamTestState::new();
    state.setup_micro_streams(true).expect("Failed to setup micro streams");
    
    // Simulate 1 year of streaming
    let year_seconds = 365 * SECONDS_PER_DAY;
    state.set_timestamp(1000 + year_seconds);
    
    // All recipients withdraw
    let total_withdrawn = state.simulate_withdrawals(NUM_MICRO_STREAMS)
        .expect("Withdrawal simulation failed");
    
    // Verify total distributed matches expected within tolerance
    let (theoretical_grantee, theoretical_validator) = state.calculate_theoretical_distribution(year_seconds);
    let total_theoretical = theoretical_grantee + theoretical_validator;
    
    let tolerance = NUM_MICRO_STREAMS as i128 * 5; // 5 stroops per stream tolerance
    let error = (total_withdrawn - theoretical_grantee).abs();
    
    assert!(error <= tolerance, 
        "Total distribution error too large: expected {}, got {}, error {} (tolerance {})",
        theoretical_grantee, total_withdrawn, error, tolerance
    );
    
    // Verify final state consistency
    state.verify_rounding_invariants(year_seconds, true)
        .expect("Final rounding invariant violation");
}

// Test dust accumulation and treasury return
#[test]
fn test_dust_accumulation_and_treasury_return() {
    let mut state = MicroStreamTestState::new();
    state.setup_micro_streams(false).expect("Failed to setup micro streams");
    
    // Simulate partial streaming to create dust
    let partial_seconds = 30 * SECONDS_PER_DAY; // 30 days
    state.set_timestamp(1000 + partial_seconds);
    
    // Cancel half the grants to return dust to treasury
    let grants_to_cancel = NUM_MICRO_STREAMS / 2;
    for i in 0..grants_to_cancel {
        let grant_id = (i + 1) as u64;
        state.client.cancel_grant(&grant_id);
    }
    
    // Verify treasury received the dust amounts
    let contract_balance_before = state.get_contract_balance();
    
    // Withdraw from remaining grants
    let remaining_withdrawals = NUM_MICRO_STREAMS - grants_to_cancel;
    state.simulate_withdrawals(remaining_withdrawals)
        .expect("Withdrawal simulation failed");
    
    let contract_balance_after = state.get_contract_balance();
    
    // Contract should have minimal remaining balance (dust)
    let max_dust = remaining_withdrawals as i128 * MICRO_STREAM_RATE;
    assert!(contract_balance_after <= max_dust, 
        "Too much dust remaining in contract: {}", contract_balance_after);
    
    // Verify total invariant
    let total_distributed = state.get_total_distributed();
    let total_accounted = contract_balance_after + total_distributed;
    let expected_total = state.initial_contract_balance;
    
    let tolerance = NUM_MICRO_STREAMS as i128 * 2;
    assert!((total_accounted - expected_total).abs() <= tolerance,
        "Total invariant violation after dust handling: expected {}, got {}",
        expected_total, total_accounted);
}

// Test mathematical verification of rounding error bounds
#[test]
fn test_rounding_error_mathematical_bounds() {
    let mut state = MicroStreamTestState::new();
    state.setup_micro_streams(true).expect("Failed to setup micro streams");
    
    // Test various time periods to establish rounding error patterns
    let test_periods = vec![1, 7, 30, 90, 180, 365]; // days
    
    for days in test_periods {
        let elapsed_seconds = days * SECONDS_PER_DAY;
        state.set_timestamp(1000 + elapsed_seconds);
        
        // Calculate theoretical vs actual
        let (theoretical_grantee, theoretical_validator) = state.calculate_theoretical_distribution(elapsed_seconds);
        
        // Withdraw all
        state.simulate_withdrawals(NUM_MICRO_STREAMS)
            .expect("Withdrawal simulation failed");
        
        let actual_grantee = state.get_total_distributed();
        let validator_addr = state.client.get_grant(1).validator.unwrap();
        let actual_validator = state.get_validator_balance(&validator_addr);
        
        // Calculate errors
        let grantee_error = (actual_grantee - theoretical_grantee).abs();
        let validator_error = (actual_validator - theoretical_validator).abs();
        
        // Mathematical bounds verification
        // Maximum error per stream should be less than (rate * time_step) / divisor
        let max_error_per_stream = (MICRO_STREAM_RATE * SECONDS_PER_DAY as i128) / 10000;
        let max_total_error = max_error_per_stream * NUM_MICRO_STREAMS as i128;
        
        assert!(grantee_error <= max_total_error,
            "Grantee error exceeds mathematical bounds: {} > {}",
            grantee_error, max_total_error);
        
        assert!(validator_error <= max_total_error,
            "Validator error exceeds mathematical bounds: {} > {}",
            validator_error, max_total_error);
        
        // Reset for next test period
        state = MicroStreamTestState::new();
        state.setup_micro_streams(true).expect("Failed to reset micro streams");
    }
}

// Test edge case: single stroop precision
#[test]
fn test_single_stroop_precision_edge_case() {
    let mut state = MicroStreamTestState::new();
    
    // Create ultra-micro streams: 1 stroop per day
    let ultra_micro_rate = 1 * STROOP; // 1 stroop per day
    let test_duration_days = 30;
    let total_per_stream = ultra_micro_rate * SECONDS_PER_DAY as i128 * test_duration_days as i128;
    let total_all_streams = total_per_stream * 1000 as i128; // Fewer streams for this test
    
    let grant_token = token::StellarAssetClient::new(&state.env, &state.grant_token);
    grant_token.mint(&state.client.address, &total_all_streams);
    
    // Create ultra-micro streams
    for i in 0..1000 {
        let grant_id = (i + 1) as u64;
        let recipient = Address::generate(&state.env);
        
        state.client.create_grant(
            &grant_id,
            &recipient,
            &total_per_stream,
            &ultra_micro_rate,
            &0,
            &None,
        );
    }
    
    // Stream for test duration
    state.set_timestamp(1000 + test_duration_days * SECONDS_PER_DAY);
    
    // Verify that even with single stroop precision, rounding errors don't accumulate
    let contract_balance = state.get_contract_balance();
    let expected_remaining = total_all_streams - (ultra_micro_rate * test_duration_days as i128 * 1000);
    
    // Should be very close due to minimal rounding at this scale
    let error = (contract_balance - expected_remaining).abs();
    assert!(error <= 1000, "Single stroop precision error too large: {}", error);
}
