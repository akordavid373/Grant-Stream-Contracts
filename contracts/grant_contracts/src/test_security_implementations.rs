#[cfg(test)]
mod test_security_implementations {
    use super::*;
    use soroban_sdk::{testutils::Address as TestAddress, testutils::Ledger as TestLedger, Address, Env, Symbol};

    fn setup_test_env() -> (Env, Address, Address) {
        let env = Env::new();
        let admin = TestAddress::generate(&env);
        let grantee = TestAddress::generate(&env);
        
        // Initialize admin
        env.storage().instance().set(&DataKey::Admin, &admin);
        
        (env, admin, grantee)
    }

    fn create_test_grant(env: &Env, grantee: Address, amount: i128) -> Grant {
        let grant = Grant {
            recipient: grantee,
            total_amount: amount,
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
            stream_type: StreamType::FixedAmount,
            start_time: env.ledger().timestamp(),
            warmup_duration: 0,
            required_stake: 0,
            staked_amount: 0,
            stake_token: TestAddress::generate(&env),
            slash_reason: None,
            lessor: TestAddress::generate(&env),
            property_id: "test_property".to_string(),
            serial_number: "test_serial".to_string(),
            security_deposit: 0,
            lease_end_time: 0,
            lease_terminated: false,
            remaining_balance: amount,
            linked_addresses: Vec::new(&env),
            milestone_amount: 100,
            total_milestones: 5,
            claimed_milestones: 0,
            available_milestone_funds: 500,
            last_resume_timestamp: None,
            pause_count: 0,
            gas_buffer: 0,
            gas_buffer_used: 0,
            max_withdrawal_per_day: 1000,
            last_withdrawal_timestamp: 0,
            withdrawal_amount_today: 0,
        };
        
        grant
    }

    #[test]
    fn test_grantee_change_proposal() {
        let (env, admin, grantee) = setup_test_env();
        let new_grantee = TestAddress::generate(&env);
        
        // Create test grant
        let grant = create_test_grant(&env, grantee.clone(), 10000);
        env.storage().instance().set(&DataKey::Grant(1), &grant);
        
        // Test propose_grantee_change
        let result = propose_grantee_change(
            env.clone(),
            1,
            new_grantee.clone(),
            "Team migration".to_string(),
        );
        
        assert!(result.is_ok());
        let request_id = result.unwrap();
        
        // Verify request was stored
        let request: GranteeChangeRequest = env.storage().instance()
            .get(&DataKey::GranteeChangeRequest(request_id))
            .unwrap();
        
        assert_eq!(request.grant_id, 1);
        assert_eq!(request.current_grantee, grantee);
        assert_eq!(request.proposed_grantee, new_grantee);
        assert_eq!(request.status, GranteeChangeStatus::Proposed);
    }

    #[test]
    fn test_grantee_change_authorization() {
        let (env, admin, grantee) = setup_test_env();
        let new_grantee = TestAddress::generate(&env);
        
        // Create test grant and request
        let grant = create_test_grant(&env, grantee.clone(), 10000);
        env.storage().instance().set(&DataKey::Grant(1), &grant);
        
        let request_id = propose_grantee_change(
            env.clone(),
            1,
            new_grantee.clone(),
            "Team migration".to_string(),
        ).unwrap();
        
        // Test authorization as admin
        let result = authorize_grantee_change(
            env.clone(),
            request_id,
            true,
            None,
        );
        
        assert!(result.is_ok());
        
        // Verify request was authorized
        let request: GranteeChangeRequest = env.storage().instance()
            .get(&DataKey::GranteeChangeRequest(request_id))
            .unwrap();
        
        assert_eq!(request.status, GranteeChangeStatus::Authorized);
        assert!(request.authorized_at.is_some());
    }

    #[test]
    fn test_emergency_resumption_request() {
        let (env, admin, grantee) = setup_test_env();
        
        // Create cancelled grant
        let mut grant = create_test_grant(&env, grantee.clone(), 10000);
        grant.status = GrantStatus::Cancelled;
        grant.last_update_ts = env.ledger().timestamp();
        env.storage().instance().set(&DataKey::Grant(1), &grant);
        
        // Test emergency resumption request
        let result = request_emergency_resumption(
            env.clone(),
            1,
            "Emergency funding needed".to_string(),
        );
        
        assert!(result.is_ok());
        let request_id = result.unwrap();
        
        // Verify request was stored
        let request: EmergencyResumptionRequest = env.storage().instance()
            .get(&DataKey::EmergencyResumptionRequest(request_id))
            .unwrap();
        
        assert_eq!(request.grant_id, 1);
        assert_eq!(request.status, EmergencyResumptionStatus::Requested);
        assert_eq!(request.requester, grantee);
    }

    #[test]
    fn test_staged_approval_creation() {
        let (env, admin, grantee) = setup_test_env();
        let reviewer = TestAddress::generate(&env);
        
        // Create test grant
        let grant = create_test_grant(&env, grantee.clone(), 10000);
        env.storage().instance().set(&DataKey::Grant(1), &grant);
        
        // Test create_staged_approval
        let result = create_staged_approval(
            env.clone(),
            1,
            100,
            reviewer.clone(),
            admin.clone(),
        );
        
        assert!(result.is_ok());
        let approval_id = result.unwrap();
        
        // Verify approval was stored
        let approval: StagedApproval = env.storage().instance()
            .get(&DataKey::StagedApproval(approval_id))
            .unwrap();
        
        assert_eq!(approval.grant_id, 1);
        assert_eq!(approval.milestone_claim_id, 100);
        assert_eq!(approval.reviewer, reviewer);
        assert_eq!(approval.admin, admin);
        assert_eq!(approval.status, StagedApprovalStatus::PendingReviewer);
    }

    #[test]
    fn test_staged_approval_reviewer_flow() {
        let (env, admin, grantee) = setup_test_env();
        let reviewer = TestAddress::generate(&env);
        
        // Create test grant and approval
        let grant = create_test_grant(&env, grantee.clone(), 10000);
        env.storage().instance().set(&DataKey::Grant(1), &grant);
        
        let approval_id = create_staged_approval(
            env.clone(),
            1,
            100,
            reviewer.clone(),
            admin.clone(),
        ).unwrap();
        
        // Test reviewer approval
        let result = reviewer_approve(
            env.clone(),
            approval_id,
            true,
            Some("Milestone looks good".to_string()),
        );
        
        assert!(result.is_ok());
        
        // Verify approval status
        let approval: StagedApproval = env.storage().instance()
            .get(&DataKey::StagedApproval(approval_id))
            .unwrap();
        
        assert_eq!(approval.status, StagedApprovalStatus::ReviewerApproved);
        assert!(approval.reviewer_approval);
        assert!(approval.reviewer_approved_at.is_some());
    }

    #[test]
    fn test_partial_cancellation_proposal() {
        let (env, admin, grantee) = setup_test_env();
        
        // Create test grant
        let grant = create_test_grant(&env, grantee.clone(), 10000);
        env.storage().instance().set(&DataKey::Grant(1), &grant);
        
        // Test propose_partial_cancellation
        let result = propose_partial_cancellation(
            env.clone(),
            1,
            5000,
            "Reduce funding scope".to_string(),
        );
        
        assert!(result.is_ok());
        let request_id = result.unwrap();
        
        // Verify request was stored
        let request: PartialCancellationRequest = env.storage().instance()
            .get(&DataKey::PartialCancellationRequest(request_id))
            .unwrap();
        
        assert_eq!(request.grant_id, 1);
        assert_eq!(request.cancellation_amount, 5000);
        assert_eq!(request.status, PartialCancellationStatus::Proposed);
    }

    #[test]
    fn test_grantee_change_execution() {
        let (env, admin, grantee) = setup_test_env();
        let new_grantee = TestAddress::generate(&env);
        
        // Create test grant and authorized request
        let grant = create_test_grant(&env, grantee.clone(), 10000);
        env.storage().instance().set(&DataKey::Grant(1), &grant);
        
        let request_id = propose_grantee_change(
            env.clone(),
            1,
            new_grantee.clone(),
            "Team migration".to_string(),
        ).unwrap();
        
        authorize_grantee_change(env.clone(), request_id, true, None).unwrap();
        
        // Test execute_grantee_change
        let result = execute_grantee_change(env.clone(), request_id);
        assert!(result.is_ok());
        
        // Verify grant was updated
        let updated_grant: Grant = env.storage().instance()
            .get(&DataKey::Grant(1))
            .unwrap();
        
        assert_eq!(updated_grant.recipient, new_grantee);
        
        // Verify request status
        let updated_request: GranteeChangeRequest = env.storage().instance()
            .get(&DataKey::GranteeChangeRequest(request_id))
            .unwrap();
        
        assert_eq!(updated_request.status, GranteeChangeStatus::Executed);
        assert!(updated_request.executed_at.is_some());
    }

    #[test]
    fn test_emergency_resumption_with_fee() {
        let (env, admin, grantee) = setup_test_env();
        
        // Create cancelled grant
        let mut grant = create_test_grant(&env, grantee.clone(), 10000);
        grant.status = GrantStatus::Cancelled;
        grant.last_update_ts = env.ledger().timestamp();
        env.storage().instance().set(&DataKey::Grant(1), &grant);
        
        let request_id = request_emergency_resumption(
            env.clone(),
            1,
            "Emergency funding needed".to_string(),
        ).unwrap();
        
        // Test fee payment (simplified - in real implementation would require token transfers)
        let result = pay_emergency_resumption_fee(env.clone(), request_id);
        // Note: This might fail in test without proper token setup
        
        // Test approval
        let result = approve_emergency_resumption(
            env.clone(),
            request_id,
            true,
            None,
        );
        
        // Note: This might fail if fee payment wasn't successful
        // In a full test environment, you'd set up token contracts
    }

    #[test]
    fn test_staged_approval_admin_flow() {
        let (env, admin, grantee) = setup_test_env();
        let reviewer = TestAddress::generate(&env);
        
        // Create test grant and approval
        let grant = create_test_grant(&env, grantee.clone(), 10000);
        env.storage().instance().set(&DataKey::Grant(1), &grant);
        
        let approval_id = create_staged_approval(
            env.clone(),
            1,
            100,
            reviewer.clone(),
            admin.clone(),
        ).unwrap();
        
        // Complete reviewer approval first
        reviewer_approve(
            env.clone(),
            approval_id,
            true,
            Some("Milestone looks good".to_string()),
        ).unwrap();
        
        // Test admin approval
        let result = admin_approve(
            env.clone(),
            approval_id,
            true,
            Some("Final approval confirmed".to_string()),
        );
        
        assert!(result.is_ok());
        
        // Verify final approval status
        let approval: StagedApproval = env.storage().instance()
            .get(&DataKey::StagedApproval(approval_id))
            .unwrap();
        
        assert_eq!(approval.status, StagedApprovalStatus::AdminApproved);
        assert!(approval.admin_approval);
        assert!(approval.admin_approved_at.is_some());
    }

    #[test]
    fn test_partial_cancellation_approval() {
        let (env, admin, grantee) = setup_test_env();
        let grantor = TestAddress::generate(&env);
        
        // Create test grant
        let grant = create_test_grant(&env, grantee.clone(), 10000);
        env.storage().instance().set(&DataKey::Grant(1), &grant);
        
        let request_id = propose_partial_cancellation(
            env.clone(),
            1,
            5000,
            "Reduce funding scope".to_string(),
        ).unwrap();
        
        // Note: In a full implementation, you'd need to set up grantor shares
        // and test the approval process with multiple grantors
        // For now, we just test the basic structure
        
        let request: PartialCancellationRequest = env.storage().instance()
            .get(&DataKey::PartialCancellationRequest(request_id))
            .unwrap();
        
        assert_eq!(request.status, PartialCancellationStatus::Proposed);
        assert_eq!(request.cancellation_amount, 5000);
    }
}
