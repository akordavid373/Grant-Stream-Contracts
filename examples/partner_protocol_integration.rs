// Example: Partner Protocol Integration with Grant-Stream is_active_grantee Adapter
// This demonstrates how a DeFi protocol can offer builder discounts to active grantees

#![no_std]
use soroban_sdk::{
    contract, contracterror, contractimpl, contracttype, symbol_short, token, Address, Env,
    Symbol, Vec, Map, String,
};

// --- Contract Errors ---
#[contracterror]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u32)]
pub enum Error {
    NotInitialized = 1,
    Unauthorized = 2,
    InsufficientBalance = 3,
    InvalidAmount = 4,
    AccessDenied = 5,
}

// --- Contract Storage ---
#[derive(Clone)]
#[contracttype]
pub struct Config {
    pub admin: Address,
    pub grant_stream_contract: Address,
    pub base_fee_rate: i128, // in basis points (10000 = 100%)
    pub grantee_discount_rate: i128, // discount in basis points
}

#[derive(Clone)]
#[contracttype]
pub enum DataKey {
    Config,
    UserTier(Address),
}

// --- Contract Implementation ---
#[contract]
pub struct PartnerProtocol;

#[contractimpl]
impl PartnerProtocol {
    /// Initialize the partner protocol contract
    pub fn initialize(
        env: Env,
        admin: Address,
        grant_stream_contract: Address,
        base_fee_rate: i128,
        grantee_discount_rate: i128,
    ) -> Result<(), Error> {
        if env.storage().instance().has(&DataKey::Config) {
            return Err(Error::NotInitialized);
        }

        let config = Config {
            admin: admin.clone(),
            grant_stream_contract,
            base_fee_rate,
            grantee_discount_rate,
        };

        env.storage().instance().set(&DataKey::Config, &config);
        Ok(())
    }

    /// Check if a user is an active grantee and apply appropriate pricing
    pub fn calculate_fee(env: Env, user: Address, base_amount: i128) -> i128 {
        let config = Self::read_config(&env);
        
        // Query Grant-Stream contract for active grantee status
        let is_grantee = env.invoke_contract::<bool>(
            &config.grant_stream_contract,
            &Symbol::new(&env, "is_active_grantee"),
            (user.clone(),)
        );

        if is_grantee {
            // Apply builder discount for active grantees
            let discount_amount = base_amount * config.grantee_discount_rate / 10000;
            base_amount - discount_amount
        } else {
            // Standard pricing for non-grantees
            base_amount
        }
    }

    /// Get user tier based on grantee status
    pub fn get_user_tier(env: Env, user: Address) -> String {
        let config = Self::read_config(&env);
        
        let is_grantee = env.invoke_contract::<bool>(
            &config.grant_stream_contract,
            &Symbol::new(&env, "is_active_grantee"),
            (user.clone(),)
        );

        if is_grantee {
            String::from_str(&env, "Builder")
        } else {
            String::from_str(&env, "Standard")
        }
    }

    /// Example function that requires premium access
    pub fn access_premium_feature(env: Env, user: Address) -> Result<(), Error> {
        let config = Self::read_config(&env);
        
        let is_grantee = env.invoke_contract::<bool>(
            &config.grant_stream_contract,
            &Symbol::new(&env, "is_active_grantee"),
            (user.clone(),)
        );

        if is_grantee {
            // Grant access to premium features for active grantees
            env.events().publish(
                (symbol_short!("premium_access"),),
                (user, "Granted")
            );
            Ok(())
        } else {
            Err(Error::AccessDenied)
        }
    }

    /// Example lending function with reduced rates for grantees
    pub fn calculate_borrowing_rate(env: Env, user: Address, base_rate: i128) -> i128 {
        let config = Self::read_config(&env);
        
        let is_grantee = env.invoke_contract::<bool>(
            &config.grant_stream_contract,
            &Symbol::new(&env, "is_active_grantee"),
            (user.clone(),)
        );

        if is_grantee {
            // 20% reduced borrowing rate for active grantees
            base_rate * 80 / 100
        } else {
            base_rate
        }
    }

    /// Batch check multiple users for grantee status (efficient for large operations)
    pub fn batch_check_grantee_status(env: Env, users: Vec<Address>) -> Vec<bool> {
        let config = Self::read_config(&env);
        let mut results = Vec::new(&env);

        for user in users.iter() {
            let is_grantee = env.invoke_contract::<bool>(
                &config.grant_stream_contract,
                &Symbol::new(&env, "is_active_grantee"),
                (user.clone(),)
            );
            results.push_back(is_grantee);
        }

        results
    }

    /// Update configuration (admin only)
    pub fn update_config(
        env: Env,
        admin: Address,
        new_grant_stream_contract: Option<Address>,
        new_base_fee_rate: Option<i128>,
        new_grantee_discount_rate: Option<i128>,
    ) -> Result<(), Error> {
        let mut config = Self::read_config(&env);
        
        if config.admin != admin {
            return Err(Error::Unauthorized);
        }

        if let Some(new_contract) = new_grant_stream_contract {
            config.grant_stream_contract = new_contract;
        }
        if let Some(new_rate) = new_base_fee_rate {
            config.base_fee_rate = new_rate;
        }
        if let Some(new_discount) = new_grantee_discount_rate {
            config.grantee_discount_rate = new_discount;
        }

        env.storage().instance().set(&DataKey::Config, &config);
        Ok(())
    }

    /// Get current configuration
    pub fn get_config(env: Env) -> Config {
        Self::read_config(&env)
    }

    // --- Internal Helper Functions ---
    fn read_config(env: &Env) -> Config {
        env.storage()
            .instance()
            .get(&DataKey::Config)
            .unwrap_or_else(|| panic!("Contract not initialized"))
    }
}

// --- Test Cases ---
#[cfg(test)]
mod test {
    use super::*;
    use soroban_sdk::testutils::{Address as _, Ledger};

    fn setup_test(env: &Env) -> (Address, Address, PartnerProtocolClient) {
        let admin = Address::generate(env);
        let grant_stream_contract = Address::generate(env); // Mock Grant-Stream contract
        
        let contract_id = env.register(PartnerProtocol, ());
        let client = PartnerProtocolClient::new(env, &contract_id);

        client.initialize(
            &admin,
            &grant_stream_contract,
            &10000i128,  // 100% base fee rate
            &2500i128,   // 25% discount for grantees
        );

        (admin, grant_stream_contract, client)
    }

    #[test]
    fn test_fee_calculation() {
        let env = Env::default();
        env.mock_all_auths();
        let (admin, grant_stream_contract, client) = setup_test(&env);

        let user = Address::generate(&env);
        let base_amount = 1000i128;

        // Mock the Grant-Stream contract response for non-grantee
        env.mock_contract_call(
            &grant_stream_contract,
            &Symbol::new(&env, "is_active_grantee"),
            (user.clone(),),
            Ok(false.into())
        );

        let fee = client.calculate_fee(&user, &base_amount);
        assert_eq!(fee, base_amount); // No discount for non-grantee

        // Mock the Grant-Stream contract response for grantee
        env.mock_contract_call(
            &grant_stream_contract,
            &Symbol::new(&env, "is_active_grantee"),
            (user.clone(),),
            Ok(true.into())
        );

        let discounted_fee = client.calculate_fee(&user, &base_amount);
        assert_eq!(discounted_fee, 750i128); // 25% discount applied
    }

    #[test]
    fn test_user_tier() {
        let env = Env::default();
        env.mock_all_auths();
        let (admin, grant_stream_contract, client) = setup_test(&env);

        let user = Address::generate(&env);

        // Mock non-grantee response
        env.mock_contract_call(
            &grant_stream_contract,
            &Symbol::new(&env, "is_active_grantee"),
            (user.clone(),),
            Ok(false.into())
        );

        let tier = client.get_user_tier(&user);
        assert_eq!(tier.to_string(&env), "Standard");

        // Mock grantee response
        env.mock_contract_call(
            &grant_stream_contract,
            &Symbol::new(&env, "is_active_grantee"),
            (user.clone(),),
            Ok(true.into())
        );

        let grantee_tier = client.get_user_tier(&user);
        assert_eq!(grantee_tier.to_string(&env), "Builder");
    }

    #[test]
    fn test_premium_access() {
        let env = Env::default();
        env.mock_all_auths();
        let (admin, grant_stream_contract, client) = setup_test(&env);

        let grantee = Address::generate(&env);
        let non_grantee = Address::generate(&env);

        // Mock grantee response
        env.mock_contract_call(
            &grant_stream_contract,
            &Symbol::new(&env, "is_active_grantee"),
            (grantee.clone(),),
            Ok(true.into())
        );

        // Grantee should have access
        assert!(client.access_premium_feature(&grantee).is_ok());

        // Mock non-grantee response
        env.mock_contract_call(
            &grant_stream_contract,
            &Symbol::new(&env, "is_active_grantee"),
            (non_grantee.clone(),),
            Ok(false.into())
        );

        // Non-grantee should be denied
        assert_eq!(
            client.access_premium_feature(&non_grantee),
            Err(Error::AccessDenied)
        );
    }
}
