#![cfg(test)]

use super::*;
use soroban_sdk::{testutils::Address as _, Address, Env};

#[test]
fn test_create_and_claim() {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let funder = Address::generate(&env);
    let recipient = Address::generate(&env);
    let treasury = Address::generate(&env);
    
    // Register mock token
    let token_admin = Address::generate(&env);
    let token_addr = env.register_stellar_asset_contract(token_admin);
    let token_client = token::StellarAssetClient::new(&env, &token_addr);
    token_client.mint(&funder, &2000000000000000);

    // Register mock stablecoin
    let stable_admin = Address::generate(&env);
    let stable_addr = env.register_stellar_asset_contract(stable_admin);

    let contract_id = env.register_contract(None, GrantStreamContract);
    let client = GrantStreamContractClient::new(&env, &contract_id);

    client.init(&admin, &token_addr, &treasury, &stable_addr);
    
    let grant_id = client.create_grant(&funder, &recipient, &500000000000000);
    assert_eq!(grant_id, 1);
    
    client.claim(&recipient, &grant_id, &10000000000000);
    
    let real_token = token::Client::new(&env, &token_addr);
    assert_eq!(real_token.balance(&recipient), 10000000000000);
}
