#![cfg(test)]

use super::*;
use soroban_sdk::{testutils::Address as _, Address, Env};

#[test]
fn test_zk_kyc() {
    let env = Env::default();
    env.mock_all_auths();

    let verifier = Address::generate(&env);
    let user = Address::generate(&env);

    let contract_id = env.register_contract(None, ZKKYCContract);
    let client = ZKKYCContractClient::new(&env, &contract_id);

    client.init(&verifier);
    
    assert_eq!(client.is_verified(&user), false);
    client.verify_user(&user);
    assert_eq!(client.is_verified(&user), true);
    
    client.revoke_user(&user);
    assert_eq!(client.is_verified(&user), false);
}
