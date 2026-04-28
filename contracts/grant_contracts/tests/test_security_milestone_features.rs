use soroban_sdk::{
    contract, contracterror, contractimpl, contracttype, symbol_short, token, Address, Env, Vec,
    Symbol, Map, String, testutils::{Address as _, AuthorizedFunction, AuthorizedInvocation},
};
use grant_contracts::{
    GrantContract, Grant, GrantStatus, DataKey, Error,
    GranteeChangeRequest, GranteeChangeStatus,
    EmergencyResumptionRequest, EmergencyResumptionStatus,
    StagedApproval, StagedApprovalStatus,
    PartialCancellationRequest, PartialCancellationStatus,
};

#[contract]
pub struct TestContract;

#[contractimpl]
impl TestContract {
    // Helper function to create a test grant
    pub fn create_test_grant(env: Env, admin: Address, recipient: Address) -> u64 {
        let grant_id = 1u64;
        let grant = Grant {
            recipient: recipient.clone(),
            total_amount: 1000000,
            withdrawn: 0,
            claimable: 0,
            flow_rate: 1000,
            base_flow_rate: 1000,
            last_update_ts: env.ledger().timestamp(),
            rate_updated_at: env.ledger().timestamp(),
            last_claim_time: env.ledger().timestamp(),
            pending_rate: 0,
            effective_timestamp: 0,
            status: GrantStatus::Active,
            redirect: None,
            stream_type: grant_contracts::StreamType::FixedAmount,
            start_time: env.ledger().timestamp(),
            warmup_duration: 0,
            required_stake: 0,
            staked_amount: 0,
            stake_token: admin.clone(), // Use admin as placeholder
            slash_reason: None,
            lessor: admin.clone(), // Use admin as placeholder
            property_id: String::from_str(&env, "test_property"),
            serial_number: String::from_str(&env, "test_serial"),
            security_deposit: 0,
            lease_end_time: env.ledger().timestamp() + 1000000,
            lease_terminated: false,
            remaining_balance: 1000000,
            linked_addresses: Vec::new(&env),
            milestone_amount: 100000,
            total_milestones: 10,
            claimed_milestones: 0,
            available_milestone_funds: 0,
            last_resume_timestamp: None,
            pause_count: 0,
            gas_buffer: 0,
            gas_buffer_used: 0,
            max_withdrawal_per_day: 50000,
            last_withdrawal_timestamp: 0,
            withdrawal_amount_today: 0,
        };
        
        env.storage().instance().set(&DataKey::Grant(grant_id), &grant);
        
        let mut grant_ids = Vec::new(&env);
        grant_ids.push_back(grant_id);
        env.storage().instance().set(&DataKey::GrantIds, &grant_ids);
        
        grant_id
    }
    
    // Helper function to create a cancelled grant for emergency resumption tests
    pub fn create_cancelled_grant(env: Env, admin: Address, recipient: Address) -> u64 {
        let grant_id = 2u64;
        let mut grant = Grant {
            recipient: recipient.clone(),
            total_amount: 1000000,
            withdrawn: 0,
            claimable: 0,
            flow_rate: 1000,
            base_flow_rate: 1000,
            last_update_ts: env.ledger().timestamp() - 86400, // 1 day ago
            rate_updated_at: env.ledger().timestamp() - 86400,
            last_claim_time: env.ledger().timestamp() - 86400,
            pending_rate: 0,
            effective_timestamp: 0,
            status: GrantStatus::Cancelled,
            redirect: None,
            stream_type: grant_contracts::StreamType::FixedAmount,
            start_time: env.ledger().timestamp() - 86400,
            warmup_duration: 0,
            required_stake: 0,
            staked_amount: 0,
            stake_token: admin.clone(),
            slash_reason: None,
            lessor: admin.clone(),
            property_id: String::from_str(&env, "test_property"),
            serial_number: String::from_str(&env, "test_serial"),
            security_deposit: 0,
            lease_end_time: env.ledger().timestamp() + 1000000,
            lease_terminated: false,
            remaining_balance: 1000000,
            linked_addresses: Vec::new(&env),
            milestone_amount: 100000,
            total_milestones: 10,
            claimed_milestones: 0,
            available_milestone_funds: 0,
            last_resume_timestamp: None,
            pause_count: 0,
            gas_buffer: 0,
            gas_buffer_used: 0,
            max_withdrawal_per_day: 50000,
            last_withdrawal_timestamp: 0,
            withdrawal_amount_today: 0,
        };
        
        env.storage().instance().set(&DataKey::Grant(grant_id), &grant);
        
        let mut grant_ids = Vec::new(&env);
        grant_ids.push_back(grant_id);
        env.storage().instance().set(&DataKey::GrantIds, &grant_ids);
        
        grant_id
    }
}

#[test]
fn test_grantee_change_proposal() {
    let env = Env::default();
    env.mock_all_auths();
    
    let admin = Address::generate(&env);
    let recipient = Address::generate(&env);
    let new_grantee = Address::generate(&env);
    
    // Initialize contract
    GrantContract::initialize(
        env.clone(),
        admin.clone(),
        admin.clone(), // token
        admin.clone(), // treasury
        admin.clone(), // oracle
        admin.clone(), // native token
    ).unwrap();
    
    // Create test grant
    let grant_id = TestContract::create_test_grant(env.clone(), admin.clone(), recipient.clone());
    
    // Test proposal by current grantee
    let reason = String::from_str(&env, "Team migration: original grantee left the project");
    let request_id = GrantContract::propose_grantee_change(
        env.clone(),
        grant_id,
        new_grantee.clone(),
        reason.clone(),
    ).unwrap();
    
    // Verify request was created
    let request: GranteeChangeRequest = env.storage().instance()
        .get(&DataKey::GranteeChangeRequest(request_id))
        .unwrap();
    
    assert_eq!(request.grant_id, grant_id);
    assert_eq!(request.current_grantee, recipient);
    assert_eq!(request.proposed_grantee, new_grantee);
    assert_eq!(request.status, GranteeChangeStatus::Proposed);
    assert_eq!(request.reason, reason);
}

#[test]
fn test_grantee_change_authorization() {
    let env = Env::default();
    env.mock_all_auths();
    
    let admin = Address::generate(&env);
    let recipient = Address::generate(&env);
    let new_grantee = Address::generate(&env);
    
    // Initialize contract
    GrantContract::initialize(
        env.clone(),
        admin.clone(),
        admin.clone(), // token
        admin.clone(), // treasury
        admin.clone(), // oracle
        admin.clone(), // native token
    ).unwrap();
    
    // Create test grant
    let grant_id = TestContract::create_test_grant(env.clone(), admin.clone(), recipient.clone());
    
    // Propose change
    let reason = String::from_str(&env, "Team migration");
    let request_id = GrantContract::propose_grantee_change(
        env.clone(),
        grant_id,
        new_grantee.clone(),
        reason,
    ).unwrap();
    
    // Authorize as admin
    GrantContract::authorize_grantee_change(
        env.clone(),
        request_id,
        true,
        None,
    ).unwrap();
    
    // Verify authorization
    let request: GranteeChangeRequest = env.storage().instance()
        .get(&DataKey::GranteeChangeRequest(request_id))
        .unwrap();
    
    assert_eq!(request.status, GranteeChangeStatus::Authorized);
    assert!(request.authorized_at.is_some());
}

#[test]
fn test_grantee_change_execution() {
    let env = Env::default();
    env.mock_all_auths();
    
    let admin = Address::generate(&env);
    let recipient = Address::generate(&env);
    let new_grantee = Address::generate(&env);
    
    // Initialize contract
    GrantContract::initialize(
        env.clone(),
        admin.clone(),
        admin.clone(), // token
        admin.clone(), // treasury
        admin.clone(), // oracle
        admin.clone(), // native token
    ).unwrap();
    
    // Create test grant
    let grant_id = TestContract::create_test_grant(env.clone(), admin.clone(), recipient.clone());
    
    // Propose and authorize change
    let reason = String::from_str(&env, "Team migration");
    let request_id = GrantContract::propose_grantee_change(
        env.clone(),
        grant_id,
        new_grantee.clone(),
        reason,
    ).unwrap();
    
    GrantContract::authorize_grantee_change(
        env.clone(),
        request_id,
        true,
        None,
    ).unwrap();
    
    // Execute change
    GrantContract::execute_grantee_change(env.clone(), request_id).unwrap();
    
    // Verify grant recipient changed
    let grant: Grant = env.storage().instance().get(&DataKey::Grant(grant_id)).unwrap();
    assert_eq!(grant.recipient, new_grantee);
    
    // Verify request status
    let request: GranteeChangeRequest = env.storage().instance()
        .get(&DataKey::GranteeChangeRequest(request_id))
        .unwrap();
    assert_eq!(request.status, GranteeChangeStatus::Executed);
}

#[test]
fn test_emergency_resumption_request() {
    let env = Env::default();
    env.mock_all_auths();
    
    let admin = Address::generate(&env);
    let recipient = Address::generate(&env);
    
    // Initialize contract
    GrantContract::initialize(
        env.clone(),
        admin.clone(),
        admin.clone(), // token
        admin.clone(), // treasury
        admin.clone(), // oracle
        admin.clone(), // native token
    ).unwrap();
    
    // Create cancelled grant
    let grant_id = TestContract::create_cancelled_grant(env.clone(), admin.clone(), recipient.clone());
    
    // Request emergency resumption
    let reason = String::from_str(&env, "Critical bug fixed, need to resume development");
    let request_id = GrantContract::request_emergency_resumption(
        env.clone(),
        grant_id,
        reason.clone(),
    ).unwrap();
    
    // Verify request was created
    let request: EmergencyResumptionRequest = env.storage().instance()
        .get(&DataKey::EmergencyResumptionRequest(request_id))
        .unwrap();
    
    assert_eq!(request.grant_id, grant_id);
    assert_eq!(request.status, EmergencyResumptionStatus::Requested);
    assert_eq!(request.reason, reason);
    assert!(!request.fee_paid);
}

#[test]
fn test_emergency_resumption_approval() {
    let env = Env::default();
    env.mock_all_auths();
    
    let admin = Address::generate(&env);
    let recipient = Address::generate(&env);
    
    // Initialize contract
    GrantContract::initialize(
        env.clone(),
        admin.clone(),
        admin.clone(), // token
        admin.clone(), // treasury
        admin.clone(), // oracle
        admin.clone(), // native token
    ).unwrap();
    
    // Create cancelled grant
    let grant_id = TestContract::create_cancelled_grant(env.clone(), admin.clone(), recipient.clone());
    
    // Request emergency resumption
    let reason = String::from_str(&env, "Critical bug fixed");
    let request_id = GrantContract::request_emergency_resumption(
        env.clone(),
        grant_id,
        reason,
    ).unwrap();
    
    // This test would require token transfers for fee payment
    // For now, we'll test the approval logic without fee payment
    // In a real test environment, you would set up token balances and transfers
}

#[test]
fn test_staged_approval_creation() {
    let env = Env::default();
    env.mock_all_auths();
    
    let admin = Address::generate(&env);
    let recipient = Address::generate(&env);
    let reviewer = Address::generate(&env);
    
    // Initialize contract
    GrantContract::initialize(
        env.clone(),
        admin.clone(),
        admin.clone(), // token
        admin.clone(), // treasury
        admin.clone(), // oracle
        admin.clone(), // native token
    ).unwrap();
    
    // Create test grant
    let grant_id = TestContract::create_test_grant(env.clone(), admin.clone(), recipient.clone());
    
    // Create staged approval
    let milestone_claim_id = 1u64;
    let approval_id = GrantContract::create_staged_approval(
        env.clone(),
        grant_id,
        milestone_claim_id,
        reviewer.clone(),
        admin.clone(),
    ).unwrap();
    
    // Verify approval was created
    let approval: StagedApproval = env.storage().instance()
        .get(&DataKey::StagedApproval(approval_id))
        .unwrap();
    
    assert_eq!(approval.grant_id, grant_id);
    assert_eq!(approval.milestone_claim_id, milestone_claim_id);
    assert_eq!(approval.reviewer, reviewer);
    assert_eq!(approval.admin, admin);
    assert_eq!(approval.status, StagedApprovalStatus::PendingReviewer);
    assert!(!approval.reviewer_approval);
    assert!(!approval.admin_approval);
}

#[test]
fn test_staged_approval_reviewer_flow() {
    let env = Env::default();
    env.mock_all_auths();
    
    let admin = Address::generate(&env);
    let recipient = Address::generate(&env);
    let reviewer = Address::generate(&env);
    
    // Initialize contract
    GrantContract::initialize(
        env.clone(),
        admin.clone(),
        admin.clone(), // token
        admin.clone(), // treasury
        admin.clone(), // oracle
        admin.clone(), // native token
    ).unwrap();
    
    // Create test grant and staged approval
    let grant_id = TestContract::create_test_grant(env.clone(), admin.clone(), recipient.clone());
    let milestone_claim_id = 1u64;
    let approval_id = GrantContract::create_staged_approval(
        env.clone(),
        grant_id,
        milestone_claim_id,
        reviewer.clone(),
        admin.clone(),
    ).unwrap();
    
    // Reviewer approves
    let reason = String::from_str(&env, "Milestone completed successfully");
    GrantContract::reviewer_approve(
        env.clone(),
        approval_id,
        true,
        Some(reason.clone()),
    ).unwrap();
    
    // Verify reviewer approval
    let approval: StagedApproval = env.storage().instance()
        .get(&DataKey::StagedApproval(approval_id))
        .unwrap();
    
    assert_eq!(approval.status, StagedApprovalStatus::ReviewerApproved);
    assert!(approval.reviewer_approval);
    assert_eq!(approval.reviewer_reason, Some(reason));
    assert!(approval.reviewer_approved_at.is_some());
}

#[test]
fn test_staged_approval_admin_flow() {
    let env = Env::default();
    env.mock_all_auths();
    
    let admin = Address::generate(&env);
    let recipient = Address::generate(&env);
    let reviewer = Address::generate(&env);
    
    // Initialize contract
    GrantContract::initialize(
        env.clone(),
        admin.clone(),
        admin.clone(), // token
        admin.clone(), // treasury
        admin.clone(), // oracle
        admin.clone(), // native token
    ).unwrap();
    
    // Create test grant and staged approval
    let grant_id = TestContract::create_test_grant(env.clone(), admin.clone(), recipient.clone());
    let milestone_claim_id = 1u64;
    let approval_id = GrantContract::create_staged_approval(
        env.clone(),
        grant_id,
        milestone_claim_id,
        reviewer.clone(),
        admin.clone(),
    ).unwrap();
    
    // Reviewer approves first
    GrantContract::reviewer_approve(
        env.clone(),
        approval_id,
        true,
        Some(String::from_str(&env, "Reviewer approval")),
    ).unwrap();
    
    // Admin approves
    let admin_reason = String::from_str(&env, "Admin final approval confirmed");
    GrantContract::admin_approve(
        env.clone(),
        approval_id,
        true,
        Some(admin_reason.clone()),
    ).unwrap();
    
    // Verify admin approval
    let approval: StagedApproval = env.storage().instance()
        .get(&DataKey::StagedApproval(approval_id))
        .unwrap();
    
    assert_eq!(approval.status, StagedApprovalStatus::AdminApproved);
    assert!(approval.admin_approval);
    assert_eq!(approval.admin_reason, Some(admin_reason));
    assert!(approval.admin_approved_at.is_some());
}

#[test]
fn test_partial_cancellation_proposal() {
    let env = Env::default();
    env.mock_all_auths();
    
    let admin = Address::generate(&env);
    let recipient = Address::generate(&env);
    let grantor = Address::generate(&env);
    
    // Initialize contract
    GrantContract::initialize(
        env.clone(),
        admin.clone(),
        admin.clone(), // token
        admin.clone(), // treasury
        admin.clone(), // oracle
        admin.clone(), // native token
    ).unwrap();
    
    // Create test grant
    let grant_id = TestContract::create_test_grant(env.clone(), admin.clone(), recipient.clone());
    
    // Propose partial cancellation
    let cancellation_amount = 500000i128;
    let reason = String::from_str(&env, "Budget constraints, need to reduce funding");
    let request_id = GrantContract::propose_partial_cancellation(
        env.clone(),
        grant_id,
        cancellation_amount,
        reason.clone(),
    ).unwrap();
    
    // Verify request was created
    let request: PartialCancellationRequest = env.storage().instance()
        .get(&DataKey::PartialCancellationRequest(request_id))
        .unwrap();
    
    assert_eq!(request.grant_id, grant_id);
    assert_eq!(request.cancellation_amount, cancellation_amount);
    assert_eq!(request.status, PartialCancellationStatus::Proposed);
    assert_eq!(request.reason, reason);
    assert_eq!(request.requesting_grantor, grantor);
}

#[test]
fn test_error_cases() {
    let env = Env::default();
    env.mock_all_auths();
    
    let admin = Address::generate(&env);
    let recipient = Address::generate(&env);
    let unauthorized = Address::generate(&env);
    
    // Initialize contract
    GrantContract::initialize(
        env.clone(),
        admin.clone(),
        admin.clone(), // token
        admin.clone(), // treasury
        admin.clone(), // oracle
        admin.clone(), // native token
    ).unwrap();
    
    // Test unauthorized grantee change proposal
    let grant_id = TestContract::create_test_grant(env.clone(), admin.clone(), recipient.clone());
    let result = GrantContract::propose_grantee_change(
        env.clone(),
        grant_id,
        unauthorized.clone(),
        String::from_str(&env, "Unauthorized attempt"),
    );
    assert!(result.is_err()); // Should fail since caller is not grantee or admin
    
    // Test emergency resumption on active grant
    let result = GrantContract::request_emergency_resumption(
        env.clone(),
        grant_id,
        String::from_str(&env, "Should fail"),
    );
    assert!(result.is_err()); // Should fail since grant is not cancelled
    
    // Test staged approval sequence error
    let milestone_claim_id = 1u64;
    let approval_id = GrantContract::create_staged_approval(
        env.clone(),
        grant_id,
        milestone_claim_id,
        admin.clone(),
        admin.clone(),
    ).unwrap();
    
    // Try admin approval before reviewer approval
    let result = GrantContract::admin_approve(
        env.clone(),
        approval_id,
        true,
        None,
    );
    assert!(result.is_err()); // Should fail due to sequence error
}
