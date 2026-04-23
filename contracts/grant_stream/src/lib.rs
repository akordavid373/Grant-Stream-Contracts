#![no_std]
use soroban_sdk::{contract, contractimpl, contracttype, token, Address, Env, BytesN, Vec};

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DataKey {
    Admin,
    Token,
    Treasury,
    Stablecoin,
    GrantCounter,
    Grant(u32),
    GlobalExchangeRate,
    TotalPoolBalance,
    SignerCount,
    Signer(u32),
    Threshold,
    ZKVerificationKey,
}
}
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Grant {
    pub funder: Address,
    pub recipient: Address,
    pub balance: i128,
    pub total_volume: i128,
    pub active: bool,
    pub start_rate: i128, // Exchange rate snapshot at grant creation
}

#[contract]
pub struct GrantStreamContract;

const SUSTAINABILITY_TAX_BPS: i128 = 100;
const BPS_DENOMINATOR: i128 = 1_000_000;
const VOLUME_THRESHOLD: i128 = 100_000 * 10_000_000; // Extrapolating 7 decimals

#[contractimpl]
impl GrantStreamContract {
    pub fn init(env: Env, admin: Address, token: Address, treasury: Address, stablecoin: Address) {
        admin.require_auth();
        if env.storage().instance().has(&DataKey::Admin) {
            panic!("Already initialized");
        }
        env.storage().instance().set(&DataKey::Admin, &admin);
        env.storage().instance().set(&DataKey::Token, &token);
        env.storage().instance().set(&DataKey::Treasury, &treasury);
        env.storage().instance().set(&DataKey::Stablecoin, &stablecoin);
        env.storage().instance().set(&DataKey::GrantCounter, &0u32);
        env.storage().instance().set(&DataKey::GlobalExchangeRate, &1_000_000i128); // Initial rate: 1.0
        env.storage().instance().set(&DataKey::TotalPoolBalance, &0i128);
        env.storage().instance().set(&DataKey::SignerCount, &0u32);
        env.storage().instance().set(&DataKey::Threshold, &1u32);
    }

    pub fn create_grant(env: Env, funder: Address, recipient: Address, amount: i128) -> u32 {
        funder.require_auth();
        if amount <= 0 { panic!("Amount must be > 0"); }

        let token_addr: Address = env.storage().instance().get(&DataKey::Token).unwrap();
        let token_client = token::Client::new(&env, &token_addr);
        
        token_client.transfer(&funder, &env.current_contract_address(), &amount);

        let mut counter: u32 = env.storage().instance().get(&DataKey::GrantCounter).unwrap();
        counter += 1;
        env.storage().instance().set(&DataKey::GrantCounter, &counter);

        let current_rate: i128 = env.storage().instance().get(&DataKey::GlobalExchangeRate).unwrap();
        let mut total_balance: i128 = env.storage().instance().get(&DataKey::TotalPoolBalance).unwrap();
        total_balance += amount;
        env.storage().instance().set(&DataKey::TotalPoolBalance, &total_balance);

        let grant = Grant {
            funder,
            recipient,
            balance: amount,
            total_volume: 0,
            active: true,
            start_rate: current_rate,
        };

        env.storage().persistent().set(&DataKey::Grant(counter), &grant);
        counter
    }

    pub fn update_exchange_rate(env: Env, admin: Address, new_rate: i128) {
        admin.require_auth();
        let current_admin: Address = env.storage().instance().get(&DataKey::Admin).unwrap();
        if admin != current_admin { panic!("Not admin"); }
        
        env.storage().instance().set(&DataKey::GlobalExchangeRate, &new_rate);
    }

    pub fn claim(env: Env, recipient: Address, grant_id: u32, amount: i128) {
        recipient.require_auth();
        let mut grant: Grant = env.storage().persistent().get(&DataKey::Grant(grant_id)).unwrap();
        
        if !grant.active { panic!("Grant not active"); }
        if grant.recipient != recipient { panic!("Not recipient"); }
        if amount <= 0 || amount > grant.balance { panic!("Invalid amount"); }

        let current_rate: i128 = env.storage().instance().get(&DataKey::GlobalExchangeRate).unwrap();
        // Calculate yield-adjusted amount
        let yield_adjusted_amount = (amount * current_rate) / grant.start_rate;

        grant.balance -= amount;
        grant.total_volume += amount;

        // compute tax
        let taxable_amount = if grant.total_volume > VOLUME_THRESHOLD {
            let total_before = grant.total_volume - amount;
            if total_before >= VOLUME_THRESHOLD {
                yield_adjusted_amount
            } else {
                (grant.total_volume - VOLUME_THRESHOLD) * current_rate / grant.start_rate
            }
        } else {
            0
        };

        let tax = (taxable_amount * SUSTAINABILITY_TAX_BPS) / BPS_DENOMINATOR;
        let net = yield_adjusted_amount - tax;

        let token_addr: Address = env.storage().instance().get(&DataKey::Token).unwrap();
        let token_client = token::Client::new(&env, &token_addr);

        if tax > 0 {
            let treasury: Address = env.storage().instance().get(&DataKey::Treasury).unwrap();
            token_client.transfer(&env.current_contract_address(), &treasury, &tax);
        }
        
        token_client.transfer(&env.current_contract_address(), &recipient, &net);

        env.storage().persistent().set(&DataKey::Grant(grant_id), &grant);
    }

    pub fn withdraw_as_stable(env: Env, recipient: Address, grant_id: u32, amount: i128, min_stable_out: i128) {
        recipient.require_auth();
        let mut grant: Grant = env.storage().persistent().get(&DataKey::Grant(grant_id)).unwrap();
        
        if !grant.active { panic!("Grant not active"); }
        if grant.recipient != recipient { panic!("Not recipient"); }
        if amount <= 0 || amount > grant.balance { panic!("Invalid amount"); }

        let current_rate: i128 = env.storage().instance().get(&DataKey::GlobalExchangeRate).unwrap();
        // Calculate yield-adjusted amount
        let yield_adjusted_amount = (amount * current_rate) / grant.start_rate;

        grant.balance -= amount;
        grant.total_volume += amount;

        // compute tax
        let taxable_amount = if grant.total_volume > VOLUME_THRESHOLD {
            let total_before = grant.total_volume - amount;
            if total_before >= VOLUME_THRESHOLD {
                yield_adjusted_amount
            } else {
                (grant.total_volume - VOLUME_THRESHOLD) * current_rate / grant.start_rate
            }
        } else {
            0
        };

        let tax = (taxable_amount * SUSTAINABILITY_TAX_BPS) / BPS_DENOMINATOR;
        let net = yield_adjusted_amount - tax;

        let token_addr: Address = env.storage().instance().get(&DataKey::Token).unwrap();
        let stablecoin_addr: Address = env.storage().instance().get(&DataKey::Stablecoin).unwrap();
        
        // Perform path payment to swap token to stablecoin
        // This is a simplified implementation - in practice, you'd need proper path finding
        let path: Vec<Address> = Vec::new(&env); // Empty path for direct swap if possible
        
        // Call path_payment_strict_receive on the token's asset contract
        // Arguments: send_asset, send_max, destination, dest_asset, dest_amount, path
        let args = (
            token_addr.clone(), // send_asset (the token we're sending)
            net, // send_max
            recipient.clone(), // destination
            stablecoin_addr, // dest_asset
            min_stable_out, // dest_amount (minimum stablecoin to receive)
            path, // path
        );
        
        env.invoke_contract::<()>(
            &token_addr,
            &soroban_sdk::symbol!("path_payment_strict_receive"),
            args.into_val(&env),
        );

        // Handle tax transfer if any
        if tax > 0 {
            let token_client = token::Client::new(&env, &token_addr);
            let treasury: Address = env.storage().instance().get(&DataKey::Treasury).unwrap();
            token_client.transfer(&env.current_contract_address(), &treasury, &tax);
        }

        env.storage().persistent().set(&DataKey::Grant(grant_id), &grant);
    }

    pub fn add_signer(env: Env, admin: Address, signer: Address) {
        admin.require_auth();
        let current_admin: Address = env.storage().instance().get(&DataKey::Admin).unwrap();
        if admin != current_admin { panic!("Not admin"); }

        let mut count: u32 = env.storage().instance().get(&DataKey::SignerCount).unwrap_or(0);
        count += 1;
        env.storage().instance().set(&DataKey::SignerCount, &count);
        env.storage().instance().set(&DataKey::Signer(count), &signer);
    }

    pub fn set_threshold(env: Env, admin: Address, threshold: u32) {
        admin.require_auth();
        let current_admin: Address = env.storage().instance().get(&DataKey::Admin).unwrap();
        if admin != current_admin { panic!("Not admin"); }

        env.storage().instance().set(&DataKey::Threshold, &threshold);
    }

    pub fn verify_tss_approval(env: Env, message_hash: [u8; 32], signature: Vec<u8>, signer_bitmask: u32) -> bool {
        // This is a simplified TSS verification - in practice, you'd need proper crypto
        // For now, just check that enough signers are represented in the bitmask
        let threshold: u32 = env.storage().instance().get(&DataKey::Threshold).unwrap_or(1);
        let signer_count: u32 = env.storage().instance().get(&DataKey::SignerCount).unwrap_or(0);
        
        let mut approved_count = 0u32;
        for i in 0..signer_count {
            if (signer_bitmask & (1 << i)) != 0 {
                approved_count += 1;
            }
        }
        
    pub fn verify_zk_proof(env: Env, proof: Vec<u8>, public_inputs: Vec<u8>) -> bool {
        // This is a placeholder for ZK-SNARK verification
        // In a real implementation, you'd use a proper ZK verification library
        // and verify against the stored verification key
        
        // For now, just return true if proof is not empty
        !proof.is_empty()
    }
