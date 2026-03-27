#![no_std]

use soroban_sdk::{
    contract, contracterror, contractimpl, contracttype, symbol_short, Address, Env, Map, Vec, 
    token, Symbol, IntoVal, i128, u64,
};

// --- Constants ---
const STAKING_WEIGHT_PRECISION: u128 = 1_000_000; // 1e6 for weight calculations
const BASE_MATCHING_WEIGHT: u128 = 1_000_000; // Base weight (1.0x)
const MAX_BOOST_MULTIPLIER: u128 = 3_000_000; // Maximum 3x boost
const MIN_STAKE_FOR_BOOST: u128 = 100_000; // Minimum stake to get any boost
const BOOST_TIER_1_THRESHOLD: u128 = 100_000; // 0.1 tokens for 1.2x boost
const BOOST_TIER_2_THRESHOLD: u128 = 1_000_000; // 1 token for 1.5x boost
const BOOST_TIER_3_THRESHOLD: u128 = 10_000_000; // 10 tokens for 2.0x boost
const BOOST_TIER_4_THRESHOLD: u128 = 100_000_000; // 100 tokens for 3.0x boost
const VESTING_QUERY_TIMEOUT: u64 = 30; // 30 seconds timeout for vesting queries
const MAX_STAKING_WEIGHT_BPS: u32 = 20000; // Maximum 200% weight (2x base)

// --- Types ---

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[contracttype]
pub enum StakingMultiplierError {
    NotInitialized = 1,
    Unauthorized = 2,
    VestingVaultNotFound = 3,
    VestingQueryFailed = 4,
    InvalidStakeAmount = 5,
    StakeTooLow = 6,
    StakeTooHigh = 7,
    InvalidProjectToken = 8,
    MathOverflow = 9,
    InsufficientMatchingPool = 10,
    InvalidMultiplier = 11,
    QueryTimeout = 12,
}

#[derive(Clone, Debug, Eq, PartialEq)]
#[contracttype]
pub struct StakingInfo {
    pub donor: Address,
    pub project_token: Address,
    pub staked_amount: u128,
    pub vesting_end_time: u64,
    pub lock_duration: u64,
    pub weight_multiplier: u128, // Weight multiplier in basis points (10000 = 1.0x)
    pub last_updated: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
#[contracttype]
pub struct WeightedDonation {
    pub base_donation: Donation,
    pub staking_weight: u128,
    pub weighted_amount: u128,
    pub boost_multiplier: u128, // Actual multiplier applied (e.g., 1200000 for 1.2x)
    pub staking_info: Option<StakingInfo>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
#[contracttype]
pub struct Donation {
    pub donor: Address,
    pub token_address: Address,
    pub amount: u128,
    pub normalized_value: u128, // Value in native token units
    pub timestamp: u64,
    pub round_id: u64,
    pub project_id: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
#[contracttype]
pub struct WeightedProjectDonations {
    pub project_id: u64,
    pub base_total: u128, // Total without weighting
    pub weighted_total: u128, // Total with staking weights applied
    pub unique_donors: u32,
    pub weighted_donations: Vec<WeightedDonation>,
    pub matching_amount: u128,
    pub final_payout: u128,
    pub total_boost_applied: u128, // Total boost multiplier applied
}

#[derive(Clone, Debug, Eq, PartialEq)]
#[contracttype]
pub struct VestingVaultQuery {
    pub donor: Address,
    pub project_token: Address,
    pub query_timestamp: u64,
    pub response_timestamp: Option<u64>,
    pub staked_amount: Option<u128>,
    pub vesting_end_time: Option<u64>,
    pub status: QueryStatus,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[contracttype]
pub enum QueryStatus {
    Pending,
    Success,
    Failed,
    Timeout,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[contracttype]
pub enum StakingMultiplierDataKey {
    Admin,
    VestingVaultContract,
    NextQueryId,
    VestingQuery(u64), // query_id -> VestingVaultQuery
    DonorStakeCache(Address, Address), // donor + project_token -> StakingInfo
    ProjectToken(Address), // project_token -> bool (supported)
    StakingWeights(Address), // donor -> Map<project_token, weight>
    CacheExpiry(u64), // timestamp for cache expiry
}

#[contract]
pub struct StakingMultiplierContract;

#[contractimpl]
impl StakingMultiplierContract {
    /// Initialize the staking multiplier contract
    pub fn initialize(
        env: Env,
        admin: Address,
        vesting_vault_contract: Address,
    ) -> Result<(), StakingMultiplierError> {
        if env.storage().instance().has(&StakingMultiplierDataKey::Admin) {
            return Err(StakingMultiplierError::NotInitialized);
        }

        env.storage().instance().set(&StakingMultiplierDataKey::Admin, &admin);
        env.storage().instance().set(&StakingMultiplierDataKey::VestingVaultContract, &vesting_vault_contract);
        env.storage().instance().set(&StakingMultiplierDataKey::NextQueryId, &1u64);

        env.events().publish(
            (symbol_short!("staking_mult_initialized"),),
            (admin, vesting_vault_contract),
        );

        Ok(())
    }

    /// Add support for a project token
    pub fn add_project_token(
        env: Env,
        admin: Address,
        project_token: Address,
    ) -> Result<(), StakingMultiplierError> {
        Self::require_admin_auth(&env, &admin)?;

        env.storage().instance().set(&StakingMultiplierDataKey::ProjectToken(project_token), &true);

        env.events().publish(
            (symbol_short!("project_token_added"),),
            (project_token,),
        );

        Ok(())
    }

    /// Query vesting vault for donor's staking information
    pub fn query_vesting_stake(
        env: Env,
        donor: Address,
        project_token: Address,
    ) -> Result<u64, StakingMultiplierError> {
        // Check if project token is supported
        if !env.storage().instance().has(&StakingMultiplierDataKey::ProjectToken(project_token.clone())) {
            return Err(StakingMultiplierError::InvalidProjectToken);
        }

        // Check cache first
        if let Some(cached_info) = Self::get_cached_staking_info(&env, &donor, &project_token)? {
            let now = env.ledger().timestamp();
            let cache_expiry = env.storage().instance().get(&StakingMultiplierDataKey::CacheExpiry).unwrap_or(0);
            
            if now < cache_expiry {
                // Cache is still valid
                return Ok(cached_info.weight_multiplier);
            }
        }

        // Create new query
        let query_id = env.storage()
            .instance()
            .get(&StakingMultiplierDataKey::NextQueryId)
            .unwrap_or(1u64);

        let next_id = query_id + 1;
        env.storage().instance().set(&StakingMultiplierDataKey::NextQueryId, &next_id);

        let query = VestingVaultQuery {
            donor: donor.clone(),
            project_token: project_token.clone(),
            query_timestamp: env.ledger().timestamp(),
            response_timestamp: None,
            staked_amount: None,
            vesting_end_time: None,
            status: QueryStatus::Pending,
        };

        env.storage().instance().set(&StakingMultiplierDataKey::VestingQuery(query_id), &query);

        // In a real implementation, this would make an inter-contract call to the vesting vault
        // For now, we'll simulate the response
        let weight_multiplier = Self::simulate_vesting_query(&env, &donor, &project_token)?;

        // Update query with response
        let mut updated_query = query;
        updated_query.response_timestamp = Some(env.ledger().timestamp());
        updated_query.status = QueryStatus::Success;
        updated_query.staked_amount = Some(weight_multiplier);
        env.storage().instance().set(&StakingMultiplierDataKey::VestingQuery(query_id), &updated_query);

        // Cache the result
        let staking_info = StakingInfo {
            donor: donor.clone(),
            project_token: project_token.clone(),
            staked_amount: weight_multiplier,
            vesting_end_time: env.ledger().timestamp() + 86400 * 30, // 30 days
            lock_duration: 86400 * 30,
            weight_multiplier: Self::calculate_boost_multiplier(weight_multiplier),
            last_updated: env.ledger().timestamp(),
        };

        Self::cache_staking_info(&env, &donor, &project_token, &staking_info)?;

        // Set cache expiry (24 hours)
        env.storage().instance().set(&StakingMultiplierDataKey::CacheExpiry, &(env.ledger().timestamp() + 86400));

        env.events().publish(
            (symbol_short!("vesting_queried"),),
            (donor, project_token, weight_multiplier),
        );

        Ok(staking_info.weight_multiplier)
    }

    /// Apply staking weights to donations
    pub fn apply_staking_weights(
        env: Env,
        donations: Vec<Donation>,
        project_token: Address,
    ) -> Result<Vec<WeightedDonation>, StakingMultiplierError> {
        let mut weighted_donations = Vec::new(&env);
        let mut total_boost_applied = 0u128;

        for donation in donations.iter() {
            let staking_weight = Self::get_donor_staking_weight(&env, &donation.donor, &project_token)?;
            let boost_multiplier = Self::calculate_boost_multiplier(staking_weight);
            
            let weighted_amount = donation.normalized_value
                .checked_mul(staking_weight)
                .ok_or(StakingMultiplierError::MathOverflow)?
                .checked_div(STAKING_WEIGHT_PRECISION)
                .ok_or(StakingMultiplierError::MathOverflow)?;

            let weighted_donation = WeightedDonation {
                base_donation: donation.clone(),
                staking_weight,
                weighted_amount,
                boost_multiplier,
                staking_info: Self::get_cached_staking_info(&env, &donation.donor, &project_token)?,
            };

            total_boost_applied += boost_multiplier;
            weighted_donations.push_back(weighted_donation);
        }

        env.events().publish(
            (symbol_short!("weights_applied"),),
            (project_token, weighted_donations.len(), total_boost_applied),
        );

        Ok(weighted_donations)
    }

    /// Calculate weighted quadratic funding
    pub fn calculate_weighted_matching(
        env: Env,
        project_donations: &Map<u64, WeightedProjectDonations>,
        matching_pool: u128,
        quadratic_coefficient: u128,
    ) -> Result<Map<u64, u128>, StakingMultiplierError> {
        let mut results = Map::new(&env);
        let mut total_weighted_sqrt_sum = 0u128;

        // First pass: calculate weighted square roots
        for (project_id, weighted_donations) in project_donations.iter() {
            if weighted_donations.weighted_total > 0 {
                let sqrt_value = Self::integer_square_root(
                    weighted_donations.weighted_total * QUADRATIC_PRECISION
                )?;
                total_weighted_sqrt_sum += sqrt_value;
            }
        }

        if total_weighted_sqrt_sum == 0 {
            return Ok(results);
        }

        // Second pass: calculate matching amounts with solvency protection
        let mut total_matching_allocated = 0u128;

        for (project_id, weighted_donations) in project_donations.iter() {
            if weighted_donations.weighted_total > 0 {
                let sqrt_value = Self::integer_square_root(
                    weighted_donations.weighted_total * QUADRATIC_PRECISION
                )?;

                let matching_amount = (sqrt_value * matching_pool * quadratic_coefficient)
                    .checked_div(total_weighted_sqrt_sum)
                    .ok_or(StakingMultiplierError::MathOverflow)?
                    .checked_div(QUADRATIC_PRECISION)
                    .ok_or(StakingMultiplierError::MathOverflow)?;

                // Solvency protection
                if total_matching_allocated + matching_amount > matching_pool {
                    let remaining_pool = matching_pool - total_matching_allocated;
                    if remaining_pool > 0 {
                        results.set(*project_id, remaining_pool);
                        total_matching_allocated += remaining_pool;
                    }
                    break;
                }

                results.set(*project_id, matching_amount);
                total_matching_allocated += matching_amount;
            }
        }

        env.events().publish(
            (symbol_short!("weighted_matching_calculated"),),
            (matching_pool, total_matching_allocated, results.len()),
        );

        Ok(results)
    }

    /// Get donor's staking weight for a project
    pub fn get_donor_staking_weight(
        env: &Env,
        donor: &Address,
        project_token: &Address,
    ) -> Result<u128, StakingMultiplierError> {
        // Check cache first
        if let Some(cached_info) = Self::get_cached_staking_info(env, donor, project_token)? {
            let now = env.ledger().timestamp();
            let cache_expiry = env.storage().instance().get(&StakingMultiplierDataKey::CacheExpiry).unwrap_or(0);
            
            if now < cache_expiry {
                return Ok(cached_info.weight_multiplier);
            }
        }

        // Query vesting vault if not cached
        Self::query_vesting_stake(env.clone(), donor.clone(), project_token.clone())
    }

    /// Get cached staking information
    pub fn get_cached_staking_info(
        env: &Env,
        donor: &Address,
        project_token: &Address,
    ) -> Result<Option<StakingInfo>, StakingMultiplierError> {
        env.storage()
            .instance()
            .get(&StakingMultiplierDataKey::DonorStakeCache(donor.clone(), project_token.clone()))
    }

    /// Clear expired cache entries
    pub fn clear_expired_cache(env: Env) -> Result<u32, StakingMultiplierError> {
        let now = env.ledger().timestamp();
        let cache_expiry = env.storage().instance().get(&StakingMultiplierDataKey::CacheExpiry).unwrap_or(0);
        
        if now < cache_expiry {
            return Ok(0); // Cache is still valid
        }

        // In a real implementation, this would iterate through cache entries
        // For now, we'll just reset the expiry
        env.storage().instance().remove(&StakingMultiplierDataKey::CacheExpiry);

        Ok(1) // Return number of cleared entries
    }

    // --- Helper Functions ---

    fn require_admin_auth(env: &Env, admin: &Address) -> Result<(), StakingMultiplierError> {
        let stored_admin: Address = env.storage()
            .instance()
            .get(&StakingMultiplierDataKey::Admin)
            .ok_or(StakingMultiplierError::NotInitialized)?;
        
        if stored_admin != *admin {
            return Err(StakingMultiplierError::Unauthorized);
        }
        
        admin.require_auth();
        Ok(())
    }

    fn get_vesting_vault_contract(env: &Env) -> Result<Address, StakingMultiplierError> {
        env.storage()
            .instance()
            .get(&StakingMultiplierDataKey::VestingVaultContract)
            .ok_or(StakingMultiplierError::VestingVaultNotFound)
    }

    fn calculate_boost_multiplier(staked_amount: u128) -> u128 {
        if staked_amount < MIN_STAKE_FOR_BOOST {
            return BASE_MATCHING_WEIGHT; // No boost
        }

        let multiplier = if staked_amount >= BOOST_TIER_4_THRESHOLD {
            MAX_BOOST_MULTIPLIER // 3x boost
        } else if staked_amount >= BOOST_TIER_3_THRESHOLD {
            2_000_000 // 2x boost
        } else if staked_amount >= BOOST_TIER_2_THRESHOLD {
            1_500_000 // 1.5x boost
        } else if staked_amount >= BOOST_TIER_1_THRESHOLD {
            1_200_000 // 1.2x boost
        } else {
            BASE_MATCHING_WEIGHT // 1x boost (base case)
        };

        multiplier
    }

    fn cache_staking_info(
        env: &Env,
        donor: &Address,
        project_token: &Address,
        staking_info: &StakingInfo,
    ) -> Result<(), StakingMultiplierError> {
        env.storage().instance().set(
            &StakingMultiplierDataKey::DonorStakeCache(donor.clone(), project_token.clone()),
            staking_info,
        );
        Ok(())
    }

    fn simulate_vesting_query(
        env: &Env,
        donor: &Address,
        project_token: &Address,
    ) -> Result<u128, StakingMultiplierError> {
        // Simulate vesting vault response
        // In a real implementation, this would query the actual vesting vault contract
        let now = env.ledger().timestamp();
        
        // Simulate different stake amounts based on donor address (for testing)
        let donor_bytes = donor.to_fixed_bytes();
        let simulated_amount = u128::from(donor_bytes[0]) * 10000 + u128::from(donor_bytes[1]) * 1000;
        
        Ok(simulated_amount)
    }

    fn integer_square_root(n: u128) -> Result<u128, StakingMultiplierError> {
        if n == 0 {
            return Ok(0);
        }

        let mut x = n;
        let mut y = (x + 1) / 2;
        
        while y < x {
            x = y;
            y = (x + n / x) / 2;
        }
        
        Ok(x)
    }
}

// --- Supporting Types ---

const QUADRATIC_PRECISION: u128 = 1_000_000; // 1e6 for quadratic calculations
