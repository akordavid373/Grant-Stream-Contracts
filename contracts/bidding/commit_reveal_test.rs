#[cfg(test)]
mod tests {
    use super::*;
    use soroban_sdk::{testutils::Address as _, Env, Map, Bytes};

    fn setup() -> (Env, CommitRevealContractClient<'static>) {
        let env = Env::default();
        env.mock_all_auths();
        let contract_id = env.register_contract(None, CommitRevealContract);
        let client = CommitRevealContractClient::new(&env, &contract_id);
        (env, client)
    }

    #[test]
    fn test_commit_and_reveal_success() {
        let (env, client) = setup();
        let admin = Address::generate(&env);
        let grantee = Address::generate(&env);

        client.open_bidding(&admin, &(env.ledger().timestamp() + 86400));

        let mut milestone_costs = Map::new(&env);
        milestone_costs.set(1u32, 500u64);
        milestone_costs.set(2u32, 300u64);

        let salt = Bytes::from_array(&env, &[42u8; 16]);
        let bid = RevealedBid {
            grantee: grantee.clone(),
            amount: 1000,
            milestone_costs: milestone_costs.clone(),
            salt: salt.clone(),
        };

        // Compute commitment off-chain (mimic client SDK)
        let commitment = CommitRevealContract::hash_bid(&env, &bid);

        client.commit(&grantee, &commitment);
        client.close_bidding(&admin);
        client.reveal(&grantee, &bid);

        let revealed = client.get_revealed_bid(&grantee);
        assert_eq!(revealed.amount, 1000);
    }

    #[test]
    #[should_panic(expected = "does not match commitment")]
    fn test_tampered_reveal_rejected() {
        let (env, client) = setup();
        let admin = Address::generate(&env);
        let grantee = Address::generate(&env);

        client.open_bidding(&admin, &(env.ledger().timestamp() + 86400));

        let salt = Bytes::from_array(&env, &[1u8; 16]);
        let real_bid = RevealedBid {
            grantee: grantee.clone(),
            amount: 1000,
            milestone_costs: Map::new(&env),
            salt: salt.clone(),
        };
        let commitment = CommitRevealContract::hash_bid(&env, &real_bid);
        client.commit(&grantee, &commitment);
        client.close_bidding(&admin);

        // Attacker tries to reveal a different (lower) amount
        let tampered_bid = RevealedBid {
            grantee: grantee.clone(),
            amount: 1,  // front-run with a lower bid
            milestone_costs: Map::new(&env),
            salt: salt,
        };
        client.reveal(&grantee, &tampered_bid);
    }
}