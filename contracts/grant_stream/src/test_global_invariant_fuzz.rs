#![cfg(test)]

use super::{GrantStreamContract, GrantStreamContractClient, GrantStatus, SCALING_FACTOR};
use soroban_sdk::{
    testutils::{Address as _, Ledger},
    token, Address, Env, Vec, Symbol,
};
use proptest::prelude::*;
use std::collections::HashMap;

// Constants for global invariant fuzz testing
const SECONDS_PER_DAY: u64 = 86400;
const MAX_GRANTS_PER_TEST: usize = 1000;
const MAX_ITERATIONS: usize = 10000;
const MIN_TOKEN_AMOUNT: i128 = 1000 * SCALING_FACTOR;
const MAX_TOKEN_AMOUNT: i128 = 10_000_000 * SCALING_FACTOR;
const MIN_FLOW_RATE: i128 = 1 * SCALING_FACTOR / SECONDS_PER_DAY; // 1 token per day
const MAX_FLOW_RATE: i128 = 1000 * SCALING_FACTOR; // 1000 tokens per second

#[derive(Debug, Clone)]
struct GlobalInvariantTestState {
    env: Env,
    admin: Address,
    grant_token: Address,
    treasury: Address,
    oracle: Address,
    native_token: Address,
    client: GrantStreamContractClient,
    recipients: Vec<Address>,
    validators: Vec<Option<Address>>,
    grant_ids: Vec<u64>,
    initial_contract_balance: i128,
    operations_log: Vec<String>,
}

#[derive(Debug, Clone)]
enum RandomOperation {
    CreateGrant {
        grant_id: u64,
        recipient_index: usize,
        total_amount: i128,
        flow_rate: i128,
        warmup_duration: u64,
        has_validator: bool,
    },
    Withdraw {
        grant_id: u64,
        amount: i128,
    },
    WithdrawValidator {
        grant_id: u64,
        amount: i128,
    },
    PauseStream {
        grant_id: u64,
    },
    ResumeStream {
        grant_id: u64,
    },
    CancelGrant {
        grant_id: u64,
    },
    RageQuit {
        grant_id: u64,
    },
    ProposeRateChange {
        grant_id: u64,
        new_rate: i128,
    },
    ApplyKpiMultiplier {
        grant_id: u64,
        multiplier: i128,
    },
}

impl GlobalInvariantTestState {
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
            recipients: Vec::new(&env),
            validators: Vec::new(&env),
            grant_ids: Vec::new(&env),
            initial_contract_balance: 0,
            operations_log: Vec::new(&env),
        }
    }

    fn setup_initial_balance(&mut self, total_amount: i128) {
        let grant_token_admin = token::StellarAssetClient::new(&self.env, &self.grant_token);
        grant_token_admin.mint(&self.client.address, &total_amount);
        self.initial_contract_balance = total_amount;
    }

    fn generate_recipient(&mut self) -> usize {
        let recipient = Address::generate(&self.env);
        self.recipients.push_back(recipient);
        self.validators.push_back(None);
        self.recipients.len() - 1
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

    fn get_total_allocated_funds(&self) -> i128 {
        let mut total = 0_i128;
        // Use the same logic as the contract's total_allocated_funds function
        for i in 0..self.grant_ids.len() {
            let grant_id = self.grant_ids.get(i).unwrap();
            if let Ok(grant) = self.client.get_grant(grant_id) {
                if grant.status == GrantStatus::Active || grant.status == GrantStatus::Paused {
                    let remaining = grant.total_amount
                        .checked_sub(grant.withdrawn)
                        .unwrap_or(0);
                    total = total.checked_add(remaining).unwrap_or(i128::MAX);
                }
            }
        }
        total
    }

    fn verify_global_invariant(&self) -> Result<(), String> {
        let contract_balance = self.get_contract_balance();
        let total_allocated = self.get_total_allocated_funds();
        
        // The critical invariant: contract balance must be >= total allocated funds
        if contract_balance < total_allocated {
            return Err(format!(
                "GLOBAL INVARIANT VIOLATION: Contract balance {} < Total allocated {} (deficit: {})",
                contract_balance, total_allocated, total_allocated - contract_balance
            ));
        }
        
        // Additional invariant: total tokens in system should never exceed initial balance
        let mut total_user_balances = 0i128;
        for (i, recipient) in self.recipients.iter().enumerate() {
            total_user_balances += token::Client::new(&self.env, &self.grant_token).balance(recipient);
            if let Some(validator) = self.validators.get(i).unwrap() {
                total_user_balances += token::Client::new(&self.env, &self.grant_token).balance(validator);
            }
        }
        
        let total_accounted = contract_balance + total_user_balances;
        if total_accounted > self.initial_contract_balance {
            return Err(format!(
                "TOKEN CREATION DETECTED: Total accounted {} > Initial balance {} (excess: {})",
                total_accounted, self.initial_contract_balance, total_accounted - self.initial_contract_balance
            ));
        }
        
        Ok(())
    }

    fn execute_random_operation(&mut self, operation: RandomOperation) -> Result<(), String> {
        match operation {
            RandomOperation::CreateGrant {
                grant_id,
                recipient_index,
                total_amount,
                flow_rate,
                warmup_duration,
                has_validator,
            } => {
                let validator_addr = if has_validator {
                    let validator = Address::generate(&self.env);
                    self.validators.set(recipient_index, Some(validator.clone()));
                    Some(validator)
                } else {
                    None
                };
                
                self.client.create_grant(
                    &grant_id,
                    &self.recipients.get(recipient_index).unwrap(),
                    &total_amount,
                    &flow_rate,
                    &warmup_duration,
                    &validator_addr,
                );
                
                self.grant_ids.push_back(grant_id);
                self.operations_log.push_back(format!("Create grant {} for recipient {}", grant_id, recipient_index));
            }
            
            RandomOperation::Withdraw { grant_id, amount } => {
                let claimable = self.client.claimable(grant_id);
                let withdraw_amount = amount.min(claimable);
                if withdraw_amount > 0 {
                    self.client.withdraw(&grant_id, &withdraw_amount);
                    self.operations_log.push_back(format!("Withdraw {} from grant {}", withdraw_amount, grant_id));
                }
            }
            
            RandomOperation::WithdrawValidator { grant_id, amount } => {
                let validator_claimable = self.client.validator_claimable(grant_id);
                let withdraw_amount = amount.min(validator_claimable);
                if withdraw_amount > 0 {
                    self.client.withdraw_validator(&grant_id, &withdraw_amount);
                    self.operations_log.push_back(format!("Validator withdraw {} from grant {}", withdraw_amount, grant_id));
                }
            }
            
            RandomOperation::PauseStream { grant_id } => {
                if let Ok(grant) = self.client.get_grant(grant_id) {
                    if grant.status == GrantStatus::Active {
                        self.client.pause_stream(&grant_id);
                        self.operations_log.push_back(format!("Pause grant {}", grant_id));
                    }
                }
            }
            
            RandomOperation::ResumeStream { grant_id } => {
                if let Ok(grant) = self.client.get_grant(grant_id) {
                    if grant.status == GrantStatus::Paused {
                        self.client.resume_stream(&grant_id);
                        self.operations_log.push_back(format!("Resume grant {}", grant_id));
                    }
                }
            }
            
            RandomOperation::CancelGrant { grant_id } => {
                if let Ok(grant) = self.client.get_grant(grant_id) {
                    if grant.status != GrantStatus::Completed && grant.status != GrantStatus::RageQuitted {
                        self.client.cancel_grant(&grant_id);
                        self.operations_log.push_back(format!("Cancel grant {}", grant_id));
                    }
                }
            }
            
            RandomOperation::RageQuit { grant_id } => {
                if let Ok(grant) = self.client.get_grant(grant_id) {
                    if grant.status == GrantStatus::Paused {
                        self.client.rage_quit(&grant_id);
                        self.operations_log.push_back(format!("Rage quit grant {}", grant_id));
                    }
                }
            }
            
            RandomOperation::ProposeRateChange { grant_id, new_rate } => {
                if let Ok(grant) = self.client.get_grant(grant_id) {
                    if grant.status == GrantStatus::Active && new_rate >= 0 {
                        self.client.propose_rate_change(&grant_id, &new_rate);
                        self.operations_log.push_back(format!("Propose rate change for grant {} to {}", grant_id, new_rate));
                    }
                }
            }
            
            RandomOperation::ApplyKpiMultiplier { grant_id, multiplier } => {
                if let Ok(grant) = self.client.get_grant(grant_id) {
                    if grant.status == GrantStatus::Active && multiplier > 0 {
                        self.client.apply_kpi_multiplier(&grant_id, &multiplier);
                        self.operations_log.push_back(format!("Apply KPI multiplier {} to grant {}", multiplier, grant_id));
                    }
                }
            }
        }
        
        // Verify invariant after each operation
        self.verify_global_invariant()
    }

    fn generate_random_operations(&mut self, num_operations: usize, seed: u32) -> Vec<RandomOperation> {
        let mut operations = Vec::new();
        let mut next_grant_id = 1u64;
        let mut rng_state = seed;
        
        for i in 0..num_operations {
            // Simple pseudo-random number generator
            rng_state = rng_state.wrapping_mul(1103515245).wrapping_add(12345);
            let operation_type = (rng_state % 10) as usize;
            
            let operation = match operation_type {
                0..=2 => { // Create grant (30% probability)
                    if self.grant_ids.len() < MAX_GRANTS_PER_TEST {
                        let recipient_index = self.generate_recipient();
                        
                        // Generate random values
                        rng_state = rng_state.wrapping_mul(1103515245).wrapping_add(12345);
                        let amount_range = (MAX_TOKEN_AMOUNT - MIN_TOKEN_AMOUNT) as u64;
                        let total_amount = MIN_TOKEN_AMOUNT + (rng_state % amount_range) as i128;
                        
                        rng_state = rng_state.wrapping_mul(1103515245).wrapping_add(12345);
                        let flow_range = (MAX_FLOW_RATE - MIN_FLOW_RATE) as u64;
                        let flow_rate = MIN_FLOW_RATE + (rng_state % flow_range) as i128;
                        
                        rng_state = rng_state.wrapping_mul(1103515245).wrapping_add(12345);
                        let warmup_duration = (rng_state % (30 * SECONDS_PER_DAY));
                        
                        rng_state = rng_state.wrapping_mul(1103515245).wrapping_add(12345);
                        let has_validator = (rng_state % 2) == 1;
                        
                        RandomOperation::CreateGrant {
                            grant_id: next_grant_id,
                            recipient_index,
                            total_amount,
                            flow_rate,
                            warmup_duration,
                            has_validator,
                        }
                    } else {
                        continue; // Skip if max grants reached
                    }
                }
                3..=4 => { // Withdraw (20% probability)
                    if !self.grant_ids.is_empty() {
                        rng_state = rng_state.wrapping_mul(1103515245).wrapping_add(12345);
                        let grant_index = (rng_state % self.grant_ids.len() as u32) as usize;
                        let grant_id = self.grant_ids.get(grant_index).unwrap();
                        
                        rng_state = rng_state.wrapping_mul(1103515245).wrapping_add(12345);
                        let amount = 1 * SCALING_FACTOR + (rng_state % (1000 * SCALING_FACTOR as u32)) as i128;
                        
                        RandomOperation::Withdraw { grant_id, amount }
                    } else {
                        continue;
                    }
                }
                5 => { // Withdraw validator (10% probability)
                    if !self.grant_ids.is_empty() {
                        rng_state = rng_state.wrapping_mul(1103515245).wrapping_add(12345);
                        let grant_index = (rng_state % self.grant_ids.len() as u32) as usize;
                        let grant_id = self.grant_ids.get(grant_index).unwrap();
                        
                        rng_state = rng_state.wrapping_mul(1103515245).wrapping_add(12345);
                        let amount = 1 * SCALING_FACTOR + (rng_state % (1000 * SCALING_FACTOR as u32)) as i128;
                        
                        RandomOperation::WithdrawValidator { grant_id, amount }
                    } else {
                        continue;
                    }
                }
                6 => { // Pause stream (10% probability)
                    if !self.grant_ids.is_empty() {
                        rng_state = rng_state.wrapping_mul(1103515245).wrapping_add(12345);
                        let grant_index = (rng_state % self.grant_ids.len() as u32) as usize;
                        let grant_id = self.grant_ids.get(grant_index).unwrap();
                        RandomOperation::PauseStream { grant_id }
                    } else {
                        continue;
                    }
                }
                7 => { // Resume stream (10% probability)
                    if !self.grant_ids.is_empty() {
                        rng_state = rng_state.wrapping_mul(1103515245).wrapping_add(12345);
                        let grant_index = (rng_state % self.grant_ids.len() as u32) as usize;
                        let grant_id = self.grant_ids.get(grant_index).unwrap();
                        RandomOperation::ResumeStream { grant_id }
                    } else {
                        continue;
                    }
                }
                8 => { // Cancel grant (5% probability)
                    if !self.grant_ids.is_empty() {
                        rng_state = rng_state.wrapping_mul(1103515245).wrapping_add(12345);
                        let grant_index = (rng_state % self.grant_ids.len() as u32) as usize;
                        let grant_id = self.grant_ids.get(grant_index).unwrap();
                        RandomOperation::CancelGrant { grant_id }
                    } else {
                        continue;
                    }
                }
                9 => { // Rage quit (5% probability)
                    if !self.grant_ids.is_empty() {
                        rng_state = rng_state.wrapping_mul(1103515245).wrapping_add(12345);
                        let grant_index = (rng_state % self.grant_ids.len() as u32) as usize;
                        let grant_id = self.grant_ids.get(grant_index).unwrap();
                        RandomOperation::RageQuit { grant_id }
                    } else {
                        continue;
                    }
                }
                _ => continue,
            };
            
            operations.push(operation);
            if operation_type <= 2 {
                next_grant_id += 1;
            }
        }
        
        operations
    }

    fn run_fuzz_test(&mut self, num_operations: usize, time_advancement: bool, seed: u32) -> Result<(), String> {
        let operations = self.generate_random_operations(num_operations, seed);
        let mut current_time = 1000u64;
        let mut rng_state = seed;
        
        for (i, operation) in operations.into_iter().enumerate() {
            // Random time advancement
            if time_advancement && i % 10 == 0 {
                rng_state = rng_state.wrapping_mul(1103515245).wrapping_add(12345);
                let time_jump = (rng_state % SECONDS_PER_DAY) + 1; // At least 1 second
                current_time += time_jump;
                self.set_timestamp(current_time);
            }
            
            self.execute_random_operation(operation)
                .map_err(|e| format!("Operation {} failed: {}", i, e))?;
        }
        
        // Final verification
        self.verify_global_invariant()
    }
}

// Main global invariant fuzz test
proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]
    
    #[test]
    fn test_global_invariant_random_operations(
        // Number of random operations to perform
        num_operations in 100usize..=1000,
        // Initial contract balance
        initial_balance in MIN_TOKEN_AMOUNT..=MAX_TOKEN_AMOUNT * 10,
        // Whether to advance time during test
        time_advancement in prop::bool::ANY,
        // Test with different operation distributions
        operation_seed in 0u32..=100u32
    ) {
        let mut state = GlobalInvariantTestState::new();
        state.setup_initial_balance(initial_balance);
        
        state.run_fuzz_test(num_operations, time_advancement, operation_seed)
            .unwrap_or_else(|e| panic!("Global invariant fuzz test failed: {}", e));
    }
}

// Stress test with maximum number of operations
#[test]
fn test_global_invariant_maximum_stress() {
    let mut state = GlobalInvariantTestState::new();
    state.setup_initial_balance(MAX_TOKEN_AMOUNT * 100);
    
    // Run maximum number of operations
    state.run_fuzz_test(MAX_ITERATIONS, true, 42)
        .expect("Maximum stress test should maintain global invariant");
}

// Test edge case: zero balance edge
#[test]
fn test_global_invariant_zero_balance_edge() {
    let mut state = GlobalInvariantTestState::new();
    state.setup_initial_balance(0);
    
    // Even with zero balance, invariant should hold
    state.run_fuzz_test(100, false, 123)
        .expect("Zero balance edge case should maintain invariant");
}

// Test edge case: single large grant
#[test]
fn test_global_invariant_single_large_grant() {
    let mut state = GlobalInvariantTestState::new();
    let large_amount = MAX_TOKEN_AMOUNT;
    state.setup_initial_balance(large_amount);
    
    // Create single large grant
    let recipient_index = state.generate_recipient();
    let operation = RandomOperation::CreateGrant {
        grant_id: 1,
        recipient_index,
        total_amount: large_amount,
        flow_rate: MAX_FLOW_RATE,
        warmup_duration: 0,
        has_validator: false,
    };
    
    state.execute_random_operation(operation)
        .expect("Single large grant creation should maintain invariant");
    
    // Run additional operations
    state.run_fuzz_test(500, true, 456)
        .expect("Single large grant stress test should maintain invariant");
}

// Test edge case: many micro grants
#[test]
fn test_global_invariant_many_micro_grants() {
    let mut state = GlobalInvariantTestState::new();
    let micro_amount = MIN_TOKEN_AMOUNT;
    let total_micro_amount = micro_amount * 1000;
    state.setup_initial_balance(total_micro_amount);
    
    // Create many micro grants
    for i in 0..1000 {
        let recipient_index = state.generate_recipient();
        let operation = RandomOperation::CreateGrant {
            grant_id: (i + 1) as u64,
            recipient_index,
            total_amount: micro_amount,
            flow_rate: MIN_FLOW_RATE,
            warmup_duration: 0,
            has_validator: false,
        };
        
        state.execute_random_operation(operation)
            .expect("Micro grant creation should maintain invariant");
    }
    
    // Run random operations on micro grants
    state.run_fuzz_test(1000, true, 789)
        .expect("Many micro grants test should maintain invariant");
}

// Test mathematical proof of invariant
#[test]
fn test_global_invariant_mathematical_proof() {
    let mut state = GlobalInvariantTestState::new();
    let initial_balance = MAX_TOKEN_AMOUNT * 10;
    state.setup_initial_balance(initial_balance);
    
    // Create deterministic scenario
    let recipient_index = state.generate_recipient();
    
    // Create grant
    let grant_amount = initial_balance / 2;
    state.execute_random_operation(RandomOperation::CreateGrant {
        grant_id: 1,
        recipient_index,
        total_amount: grant_amount,
        flow_rate: SCALING_FACTOR,
        warmup_duration: 0,
        has_validator: false,
    }).expect("Grant creation should maintain invariant");
    
    // Verify invariant: contract_balance >= allocated
    let contract_balance = state.get_contract_balance();
    let allocated = state.get_total_allocated_funds();
    assert!(contract_balance >= allocated, 
        "Invariant violated immediately after grant creation: {} >= {}",
        contract_balance, allocated);
    
    // Advance time and withdraw
    state.set_timestamp(2000);
    let claimable = state.client.claimable(1);
    if claimable > 0 {
        state.execute_random_operation(RandomOperation::Withdraw { grant_id: 1, amount: claimable })
            .expect("Withdrawal should maintain invariant");
    }
    
    // Final verification
    state.verify_global_invariant()
        .expect("Mathematical proof test should maintain invariant");
}

// Test rescue tokens doesn't violate invariant
#[test]
fn test_global_invariant_rescue_tokens() {
    let mut state = GlobalInvariantTestState::new();
    let initial_balance = MAX_TOKEN_AMOUNT * 5;
    state.setup_initial_balance(initial_balance);
    
    // Create grant that uses half the balance
    let recipient_index = state.generate_recipient();
    let grant_amount = initial_balance / 2;
    state.execute_random_operation(RandomOperation::CreateGrant {
        grant_id: 1,
        recipient_index,
        total_amount: grant_amount,
        flow_rate: SCALING_FACTOR,
        warmup_duration: 0,
        has_validator: false,
    }).expect("Grant creation should maintain invariant");
    
    // Calculate rescuable amount (balance - allocated)
    let contract_balance = state.get_contract_balance();
    let allocated = state.get_total_allocated_funds();
    let rescuable = contract_balance - allocated;
    
    if rescuable > 0 {
        // Rescue tokens should not violate invariant
        let rescue_amount = rescuable / 2;
        let rescue_to = Address::generate(&state.env);
        
        state.client.rescue_tokens(&state.grant_token, &rescue_amount, &rescue_to)
            .expect("Rescue should succeed");
        
        // Verify invariant still holds
        state.verify_global_invariant()
            .expect("Rescue operation should maintain invariant");
    }
}
