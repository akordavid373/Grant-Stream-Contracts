#![cfg(test)]

use super::*;
use soroban_sdk::{testutils::Address as _, Address, Env};

#[test]
fn test_compliance() {
    let env = Env::default();
    env.mock_all_auths();

    let officer = Address::generate(&env);
    let target = Address::generate(&env);

    let contract_id = env.register_contract(None, ComplianceContract);
    let client = ComplianceContractClient::new(&env, &contract_id);

    client.init(&officer);
    
    assert_eq!(client.is_sanctioned(&target), false);
    client.sanction(&target);
    assert_eq!(client.is_sanctioned(&target), true);
    
    client.unsanction(&target);
    assert_eq!(client.is_sanctioned(&target), false);
}
