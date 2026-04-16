#![no_std]
use soroban_sdk::{contract, contractimpl, contracttype, token, Address, Env};

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DataKey {
    Admin,
    Token,
    Treasury,
    GrantCounter,
    Grant(u32),
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Grant {
    pub funder: Address,
    pub recipient: Address,
    pub balance: i128,
    pub total_volume: i128,
    pub active: bool,
}

#[contract]
pub struct GrantStreamContract;

const SUSTAINABILITY_TAX_BPS: i128 = 100;
const BPS_DENOMINATOR: i128 = 1_000_000;
const VOLUME_THRESHOLD: i128 = 100_000 * 10_000_000; // Extrapolating 7 decimals

#[contractimpl]
impl GrantStreamContract {
    pub fn init(env: Env, admin: Address, token: Address, treasury: Address) {
        admin.require_auth();
        if env.storage().instance().has(&DataKey::Admin) {
            panic!("Already initialized");
        }
        env.storage().instance().set(&DataKey::Admin, &admin);
        env.storage().instance().set(&DataKey::Token, &token);
        env.storage().instance().set(&DataKey::Treasury, &treasury);
        env.storage().instance().set(&DataKey::GrantCounter, &0u32);
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

        let grant = Grant {
            funder,
            recipient,
            balance: amount,
            total_volume: 0,
            active: true,
        };

        env.storage().persistent().set(&DataKey::Grant(counter), &grant);
        counter
    }

    pub fn claim(env: Env, recipient: Address, grant_id: u32, amount: i128) {
        recipient.require_auth();
        let mut grant: Grant = env.storage().persistent().get(&DataKey::Grant(grant_id)).unwrap();
        
        if !grant.active { panic!("Grant not active"); }
        if grant.recipient != recipient { panic!("Not recipient"); }
        if amount <= 0 || amount > grant.balance { panic!("Invalid amount"); }

        grant.balance -= amount;
        grant.total_volume += amount;

        // compute tax
        let taxable_amount = if grant.total_volume > VOLUME_THRESHOLD {
            let total_before = grant.total_volume - amount;
            if total_before >= VOLUME_THRESHOLD {
                amount
            } else {
                grant.total_volume - VOLUME_THRESHOLD
            }
        } else {
            0
        };

        let tax = (taxable_amount * SUSTAINABILITY_TAX_BPS) / BPS_DENOMINATOR;
        let net = amount - tax;

        let token_addr: Address = env.storage().instance().get(&DataKey::Token).unwrap();
        let token_client = token::Client::new(&env, &token_addr);

        if tax > 0 {
            let treasury: Address = env.storage().instance().get(&DataKey::Treasury).unwrap();
            token_client.transfer(&env.current_contract_address(), &treasury, &tax);
        }
        
        token_client.transfer(&env.current_contract_address(), &recipient, &net);

        env.storage().persistent().set(&DataKey::Grant(grant_id), &grant);
    }
}
