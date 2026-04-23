#![cfg(test)]

use super::*;
use soroban_sdk::{testutils::Address as _, Address, Env};
use soroban_sdk::token::Client as TokenClient;

#[test]
fn test_arbitration() {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let funder = Address::generate(&env);
    let grantee = Address::generate(&env);
    let arbitrator = Address::generate(&env);
    
    let token_admin = Address::generate(&env);
    let token_addr = env.register_stellar_asset_contract(token_admin);
    let token_client = token::StellarAssetClient::new(&env, &token_addr);
    token_client.mint(&funder, &1000);

    let contract_id = env.register_contract(None, ArbitrationContract);
    let client = ArbitrationContractClient::new(&env, &contract_id);

    client.init(&admin, &token_addr);
    let dispute_id = client.raise_dispute(&1, &funder, &grantee, &1000, &arbitrator);
    
    // Resolve dispute
    client.resolve_dispute(&dispute_id, &500, &500);
    
    let real_token = token::Client::new(&env, &token_addr);
    assert_eq!(real_token.balance(&funder), 500);
    assert_eq!(real_token.balance(&grantee), 500);
}
