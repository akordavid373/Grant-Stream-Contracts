use soroban_sdk::{Address, Env, Symbol, testutils::{Accounts as _, Ledger as _}, token};
use crate::{
    test::{self, GrantContract},
    Grant, GrantStatus, MilestoneClaim, MilestoneStatus, 
    MilestoneClawbackRequest, ClawbackStatus, TrustlineCheckRecord, TrustlineStatus,
    DataKey, MILESTONE_CLAWBACK_CHALLENGE_PERIOD, TRUSTLINE_CHECK_TIMEOUT
};

#[test]
fn test_milestone_clawback_proposal() {
    let env = Env::default();
    env.mock_all_auths();
    
    let admin = Address::generate(&env);
    let grantee = Address::generate(&env);
    let clawbacker = Address::generate(&env);
    let token = Address::generate(&env);
    
    let contract = GrantContract::new(&env, &admin);
    contract.initialize(&env, &admin, &token, &admin, &admin, &token);
    
    // Create a grant
    let grant_id = 1;
    contract.create_grant(
        &env,
        grant_id,
        &grantee,
        1000,
        100,
        0,
    ).unwrap();
    
    // Create a paid milestone claim
    let milestone_claim_id = 1;
    let claim = MilestoneClaim {
        claim_id: milestone_claim_id,
        grant_id,
        claimer: grantee.clone(),
        milestone_number: 1,
        amount: 500,
        claimed_at: env.ledger().timestamp(),
        challenge_deadline: env.ledger().timestamp() + 86400,
        status: MilestoneStatus::Paid,
        evidence: "Milestone completed".to_string(),
        challenger: None,
        challenge_reason: None,
        challenged_at: None,
    };
    
    env.storage().instance().set(&DataKey::MilestoneClaim(milestone_claim_id), &claim);
    
    // Propose clawback
    let clawback_id = contract.propose_milestone_clawback(
        &env,
        grant_id,
        milestone_claim_id,
        300,
        "Fraud detected".to_string(),
        "Evidence of fraud".to_string(),
    ).unwrap();
    
    // Verify clawback request was created
    let clawback_request: MilestoneClawbackRequest = env.storage().instance()
        .get(&DataKey::MilestoneClawbackRequest(clawback_id))
        .unwrap();
    
    assert_eq!(clawback_request.clawback_id, clawback_id);
    assert_eq!(clawback_request.grant_id, grant_id);
    assert_eq!(clawback_request.milestone_claim_id, milestone_claim_id);
    assert_eq!(clawback_request.clawbacker, clawbacker);
    assert_eq!(clawback_request.grantee, grantee);
    assert_eq!(clawback_request.amount, 300);
    assert_eq!(clawback_request.status, ClawbackStatus::Proposed);
    assert_eq!(clawback_request.reason, "Fraud detected");
    assert_eq!(clawback_request.evidence, "Evidence of fraud");
}

#[test]
fn test_milestone_clawback_voting() {
    let env = Env::default();
    env.mock_all_auths();
    
    let admin = Address::generate(&env);
    let grantee = Address::generate(&env);
    let voter1 = Address::generate(&env);
    let voter2 = Address::generate(&env);
    let token = Address::generate(&env);
    
    let contract = GrantContract::new(&env, &admin);
    contract.initialize(&env, &admin, &token, &admin, &admin, &token);
    
    // Create a grant
    let grant_id = 1;
    contract.create_grant(
        &env,
        grant_id,
        &grantee,
        1000,
        100,
        0,
    ).unwrap();
    
    // Create a paid milestone claim
    let milestone_claim_id = 1;
    let claim = MilestoneClaim {
        claim_id: milestone_claim_id,
        grant_id,
        claimer: grantee.clone(),
        milestone_number: 1,
        amount: 500,
        claimed_at: env.ledger().timestamp(),
        challenge_deadline: env.ledger().timestamp() + 86400,
        status: MilestoneStatus::Paid,
        evidence: "Milestone completed".to_string(),
        challenger: None,
        challenge_reason: None,
        challenged_at: None,
    };
    
    env.storage().instance().set(&DataKey::MilestoneClaim(milestone_claim_id), &claim);
    
    // Propose clawback
    let clawback_id = contract.propose_milestone_clawback(
        &env,
        grant_id,
        milestone_claim_id,
        300,
        "Fraud detected".to_string(),
        "Evidence of fraud".to_string(),
    ).unwrap();
    
    // Vote for clawback
    contract.vote_milestone_clawback(&env, clawback_id, true).unwrap();
    
    // Verify vote was recorded
    let vote_record: bool = env.storage().instance()
        .get(&DataKey::ClawbackVotes(clawback_id, voter1))
        .unwrap();
    assert!(vote_record);
    
    // Vote against clawback
    contract.vote_milestone_clawback(&env, clawback_id, false).unwrap();
    
    // Verify second vote was recorded
    let vote_record2: bool = env.storage().instance()
        .get(&DataKey::ClawbackVotes(clawback_id, voter2))
        .unwrap();
    assert!(!vote_record2);
}

#[test]
fn test_trustline_check_success() {
    let env = Env::default();
    env.mock_all_auths();
    
    let admin = Address::generate(&env);
    let grantee = Address::generate(&env);
    let token = Address::generate(&env);
    
    let contract = GrantContract::new(&env, &admin);
    contract.initialize(&env, &admin, &token, &admin, &admin, &token);
    
    // Create a grant
    let grant_id = 1;
    contract.create_grant(
        &env,
        grant_id,
        &grantee,
        1000,
        100,
        0,
    ).unwrap();
    
    // Mock successful trustline check by setting a balance
    let token_client = token::Client::new(&env, &token);
    token_client.mint(&grantee, &100);
    
    // Check trustline
    let check_id = contract.check_grantee_trustline(&env, grant_id).unwrap();
    
    // Verify trustline check record
    let check_record: TrustlineCheckRecord = env.storage().instance()
        .get(&DataKey::TrustlineCheckRecord(check_id))
        .unwrap();
    
    assert_eq!(check_record.check_id, check_id);
    assert_eq!(check_record.grant_id, grant_id);
    assert_eq!(check_record.grantee, grantee);
    assert_eq!(check_record.asset_address, token);
    assert_eq!(check_record.status, TrustlineStatus::Verified);
    assert!(check_record.failure_reason.is_none());
    assert!(check_record.resolved_at.is_some());
}

#[test]
fn test_trustline_check_failure() {
    let env = Env::default();
    env.mock_all_auths();
    
    let admin = Address::generate(&env);
    let grantee = Address::generate(&env);
    let token = Address::generate(&env);
    
    let contract = GrantContract::new(&env, &admin);
    contract.initialize(&env, &admin, &token, &admin, &admin, &token);
    
    // Create a grant
    let grant_id = 1;
    contract.create_grant(
        &env,
        grant_id,
        &grantee,
        1000,
        100,
        0,
    ).unwrap();
    
    // Don't set up trustline - this will cause the check to fail
    
    // Check trustline
    let check_id = contract.check_grantee_trustline(&env, grant_id).unwrap();
    
    // Verify trustline check record
    let check_record: TrustlineCheckRecord = env.storage().instance()
        .get(&DataKey::TrustlineCheckRecord(check_id))
        .unwrap();
    
    assert_eq!(check_record.check_id, check_id);
    assert_eq!(check_record.grant_id, grant_id);
    assert_eq!(check_record.grantee, grantee);
    assert_eq!(check_record.asset_address, token);
    assert_eq!(check_record.status, TrustlineStatus::Failed);
    assert!(check_record.failure_reason.is_some());
    assert!(check_record.resolved_at.is_none());
}

#[test]
fn test_trustline_recheck_success() {
    let env = Env::default();
    env.mock_all_auths();
    
    let admin = Address::generate(&env);
    let grantee = Address::generate(&env);
    let token = Address::generate(&env);
    
    let contract = GrantContract::new(&env, &admin);
    contract.initialize(&env, &admin, &token, &admin, &admin, &token);
    
    // Create a grant
    let grant_id = 1;
    contract.create_grant(
        &env,
        grant_id,
        &grantee,
        1000,
        100,
        0,
    ).unwrap();
    
    // Initial trustline check (should fail)
    let check_id = contract.check_grantee_trustline(&env, grant_id).unwrap();
    
    // Now set up trustline
    let token_client = token::Client::new(&env, &token);
    token_client.mint(&grantee, &100);
    
    // Re-check trustline
    contract.recheck_trustline(&env, check_id).unwrap();
    
    // Verify trustline check record was updated
    let check_record: TrustlineCheckRecord = env.storage().instance()
        .get(&DataKey::TrustlineCheckRecord(check_id))
        .unwrap();
    
    assert_eq!(check_record.status, TrustlineStatus::Resolved);
    assert!(check_record.failure_reason.is_none());
    assert!(check_record.resolved_at.is_some());
}

#[test]
fn test_trustline_check_timeout() {
    let env = Env::default();
    env.mock_all_auths();
    
    let admin = Address::generate(&env);
    let grantee = Address::generate(&env);
    let token = Address::generate(&env);
    
    let contract = GrantContract::new(&env, &admin);
    contract.initialize(&env, &admin, &token, &admin, &admin, &token);
    
    // Create a grant
    let grant_id = 1;
    contract.create_grant(
        &env,
        grant_id,
        &grantee,
        1000,
        100,
        0,
    ).unwrap();
    
    // Initial trustline check (should fail)
    let check_id = contract.check_grantee_trustline(&env, grant_id).unwrap();
    
    // Advance time beyond timeout
    env.ledger().set_timestamp(env.ledger().timestamp() + TRUSTLINE_CHECK_TIMEOUT + 1);
    
    // Try to re-check trustline (should fail due to timeout)
    let result = contract.recheck_trustline(&env, check_id);
    assert!(result.is_err());
    
    // Verify trustline check record was updated to expired
    let check_record: TrustlineCheckRecord = env.storage().instance()
        .get(&DataKey::TrustlineCheckRecord(check_id))
        .unwrap();
    
    assert_eq!(check_record.status, TrustlineStatus::Expired);
}

#[test]
fn test_get_trustline_check_status() {
    let env = Env::default();
    env.mock_all_auths();
    
    let admin = Address::generate(&env);
    let grantee = Address::generate(&env);
    let token = Address::generate(&env);
    
    let contract = GrantContract::new(&env, &admin);
    contract.initialize(&env, &admin, &token, &admin, &admin, &token);
    
    // Create a grant
    let grant_id = 1;
    contract.create_grant(
        &env,
        grant_id,
        &grantee,
        1000,
        100,
        0,
    ).unwrap();
    
    // Check trustline
    let check_id = contract.check_grantee_trustline(&env, grant_id).unwrap();
    
    // Get trustline check status
    let status = contract.get_trustline_check_status(&env, check_id).unwrap();
    
    assert_eq!(status.check_id, check_id);
    assert_eq!(status.grant_id, grant_id);
    assert_eq!(status.grantee, grantee);
    assert_eq!(status.asset_address, token);
}
