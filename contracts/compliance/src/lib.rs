#![no_std]
use soroban_sdk::{contract, contractimpl, contracttype, Address, Env};

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DataKey {
    Officer,
    Sanctioned(Address),
    Flagged(Address),
}

#[contract]
pub struct ComplianceContract;

#[contractimpl]
impl ComplianceContract {
    pub fn init(env: Env, officer: Address) {
        if env.storage().instance().has(&DataKey::Officer) {
            panic!("Already initialized");
        }
        env.storage().instance().set(&DataKey::Officer, &officer);
    }

    pub fn sanction(env: Env, target: Address) {
        let officer: Address = env.storage().instance().get(&DataKey::Officer).unwrap();
        officer.require_auth();
        env.storage().persistent().set(&DataKey::Sanctioned(target), &true);
    }

    pub fn unsanction(env: Env, target: Address) {
        let officer: Address = env.storage().instance().get(&DataKey::Officer).unwrap();
        officer.require_auth();
        env.storage().persistent().remove(&DataKey::Sanctioned(target));
    }

    pub fn is_sanctioned(env: Env, target: Address) -> bool {
        env.storage().persistent().get(&DataKey::Sanctioned(target)).unwrap_or(false)
    }

    pub fn flag_address(env: Env, target: Address) {
        let officer: Address = env.storage().instance().get(&DataKey::Officer).unwrap();
        officer.require_auth();
        env.storage().persistent().set(&DataKey::Flagged(target), &true);
    }

    pub fn is_flagged(env: Env, target: Address) -> bool {
        env.storage().persistent().get(&DataKey::Flagged(target)).unwrap_or(false)
    }
}
