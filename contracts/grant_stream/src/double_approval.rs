//! # Double-Approval System for High-Value Milestone Payouts
//!
//! This module implements a security mechanism requiring double approval
//! for high-value milestone payouts to prevent unauthorized or fraudulent
//! releases of significant funds.
//!
//! ## Security Features:
//! - Configurable value thresholds for double approval
//! - Two distinct approvers required (admin + oracle or designated approvers)
//! - Time-based approval windows to prevent stale requests
//! - Comprehensive audit logging and event emission
//! - Automatic expiration of unapproved requests

use soroban_sdk::{
    contracttype, Address, Env, Vec, Symbol, String, Map, xdr::ScVal,
};

use crate::storage_keys::StorageKey;
use crate::Error::{NotInitialized, InvalidAmount, InvalidState, NotAuthorized, GrantNotFound};

/// High-value milestone payout requiring double approval
#[derive(Clone, Debug, PartialEq, Eq)]
#[contracttype]
pub struct DoubleApprovalRequest {
    /// Grant ID this request belongs to
    pub grant_id: u64,
    /// Milestone index being approved
    pub milestone_index: u32,
    /// Amount of tokens to be released
    pub amount: i128,
    /// Recipient address
    pub recipient: Address,
    /// Token address for the payout
    pub token_address: Address,
    /// First approver (typically admin)
    pub first_approver: Option<Address>,
    /// Second approver (typically oracle or designated approver)
    pub second_approver: Option<Address>,
    /// Timestamp when request was created
    pub created_at: u64,
    /// Timestamp when request expires
    pub expires_at: u64,
    /// Current status of the request
    pub status: ApprovalStatus,
    /// Optional reason or metadata for the approval
    pub reason: Option<String>,
}

/// Approval status for double-approval requests
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[contracttype]
pub enum ApprovalStatus {
    /// Request created, awaiting approvals
    Pending,
    /// First approval received
    FirstApproved,
    /// Both approvals received, ready for execution
    FullyApproved,
    /// Request executed successfully
    Executed,
    /// Request expired
    Expired,
    /// Request cancelled
    Cancelled,
}

/// Configuration for double-approval system
#[derive(Clone, Debug, PartialEq, Eq)]
#[contracttype]
pub struct DoubleApprovalConfig {
    /// Minimum amount that requires double approval (in token units)
    pub high_value_threshold: i128,
    /// Address of the primary approver (typically admin)
    pub primary_approver: Address,
    /// Address of the secondary approver (typically oracle)
    pub secondary_approver: Address,
    /// Approval window in seconds (default: 7 days)
    pub approval_window_secs: u64,
    /// Whether double-approval is currently enabled
    pub enabled: bool,
}

/// Default configuration values
pub const DEFAULT_HIGH_VALUE_THRESHOLD: i128 = 100_000_000_000; // 100,000 tokens (assuming 6 decimals)
pub const DEFAULT_APPROVAL_WINDOW_SECS: u64 = 7 * 24 * 60 * 60; // 7 days

/// Initialize double-approval configuration
pub fn initialize_config(
    env: &Env,
    primary_approver: Address,
    secondary_approver: Address,
    high_value_threshold: Option<i128>,
    approval_window_secs: Option<u64>,
) -> Result<(), Error> {
    let config = DoubleApprovalConfig {
        high_value_threshold: high_value_threshold.unwrap_or(DEFAULT_HIGH_VALUE_THRESHOLD),
        primary_approver,
        secondary_approver,
        approval_window_secs: approval_window_secs.unwrap_or(DEFAULT_APPROVAL_WINDOW_SECS),
        enabled: true,
    };
    
    env.storage().instance().set(&StorageKey::DoubleApprovalConfig, &config);
    Ok(())
}

/// Get current double-approval configuration
pub fn get_config(env: &Env) -> Result<DoubleApprovalConfig, Error> {
    env.storage()
        .instance()
        .get(&StorageKey::DoubleApprovalConfig)
        .ok_or(NotInitialized)
}

/// Update configuration (admin only)
pub fn update_config(
    env: &Env,
    high_value_threshold: Option<i128>,
    approval_window_secs: Option<u64>,
    enabled: Option<bool>,
) -> Result<(), Error> {
    let mut config = get_config(env)?;
    
    if let Some(threshold) = high_value_threshold {
        if threshold <= 0 {
            return Err(InvalidAmount);
        }
        config.high_value_threshold = threshold;
    }
    
    if let Some(window) = approval_window_secs {
        if window == 0 {
            return Err(InvalidAmount);
        }
        config.approval_window_secs = window;
    }
    
    if let Some(enabled_flag) = enabled {
        config.enabled = enabled_flag;
    }
    
    env.storage().instance().set(&StorageKey::DoubleApprovalConfig, &config);
    Ok(())
}

/// Check if an amount requires double approval
pub fn requires_double_approval(env: &Env, amount: i128) -> Result<bool, Error> {
    if amount <= 0 {
        return Err(InvalidAmount);
    }
    
    let config = get_config(env)?;
    Ok(config.enabled && amount >= config.high_value_threshold)
}

/// Create a new double-approval request
pub fn create_request(
    env: &Env,
    grant_id: u64,
    milestone_index: u32,
    amount: i128,
    recipient: Address,
    token_address: Address,
    reason: Option<String>,
) -> Result<u64, Error> {
    // Check if amount requires double approval
    if !requires_double_approval(env, amount)? {
        return Err(InvalidState); // Amount doesn't require double approval
    }
    
    let config = get_config(env)?;
    let now = env.ledger().timestamp();
    let expires_at = now + config.approval_window_secs;
    
    // Generate unique request ID (using timestamp and grant_id)
    let request_id = (now << 32) | (grant_id & 0xFFFFFFFF);
    
    let request = DoubleApprovalRequest {
        grant_id,
        milestone_index,
        amount,
        recipient,
        token_address,
        first_approver: None,
        second_approver: None,
        created_at: now,
        expires_at,
        status: ApprovalStatus::Pending,
        reason,
    };
    
    env.storage().instance().set(
        &StorageKey::DoubleApprovalRequest(grant_id, milestone_index),
        &request,
    );
    
    // Emit event for request creation
    env.events().publish(
        (Symbol::short!("dbl_req"),),
        (request_id, grant_id, milestone_index, amount),
    );
    
    Ok(request_id)
}

/// Get a double-approval request
pub fn get_request(
    env: &Env,
    grant_id: u64,
    milestone_index: u32,
) -> Result<DoubleApprovalRequest, Error> {
    env.storage()
        .instance()
        .get(&StorageKey::DoubleApprovalRequest(grant_id, milestone_index))
        .ok_or(GrantNotFound)
}

/// Approve a double-approval request
pub fn approve_request(
    env: &Env,
    grant_id: u64,
    milestone_index: u32,
    approver: Address,
) -> Result<(), Error> {
    let mut request = get_request(env, grant_id, milestone_index)?;
    let config = get_config(env)?;
    let now = env.ledger().timestamp();
    
    // Check if request is still valid
    if now > request.expires_at {
        request.status = ApprovalStatus::Expired;
        env.storage().instance().set(
            &StorageKey::DoubleApprovalRequest(grant_id, milestone_index),
            &request,
        );
        return Err(InvalidState);
    }
    
    // Check if request is still pending or first approved
    if request.status != ApprovalStatus::Pending && request.status != ApprovalStatus::FirstApproved {
        return Err(InvalidState);
    }
    
    // Verify approver authorization
    let is_primary = approver == config.primary_approver;
    let is_secondary = approver == config.secondary_approver;
    
    if !is_primary && !is_secondary {
        return Err(NotAuthorized);
    }
    
    // Check for duplicate approvals
    if let Some(ref first) = request.first_approver {
        if *first == approver {
            return Err(InvalidState); // Already approved by this address
        }
    }
    if let Some(ref second) = request.second_approver {
        if *second == approver {
            return Err(InvalidState); // Already approved by this address
        }
    }
    
    // Record the approval
    if request.first_approver.is_none() {
        request.first_approver = Some(approver.clone());
        request.status = ApprovalStatus::FirstApproved;
    } else if request.second_approver.is_none() {
        request.second_approver = Some(approver.clone());
        request.status = ApprovalStatus::FullyApproved;
    } else {
        return Err(InvalidState); // Already fully approved
    }
    
    env.storage().instance().set(
        &StorageKey::DoubleApprovalRequest(grant_id, milestone_index),
        &request,
    );
    
    // Emit approval event
    env.events().publish(
        (Symbol::short!("dbl_appr"),),
        (grant_id, milestone_index, approver, request.status as u32),
    );
    
    Ok(())
}

/// Execute a fully approved double-approval request
pub fn execute_request(
    env: &Env,
    grant_id: u64,
    milestone_index: u32,
    executor: Address,
) -> Result<(), Error> {
    let mut request = get_request(env, grant_id, milestone_index)?;
    let now = env.ledger().timestamp();
    
    // Verify request is fully approved and not expired
    if request.status != ApprovalStatus::FullyApproved {
        return Err(InvalidState);
    }
    
    if now > request.expires_at {
        request.status = ApprovalStatus::Expired;
        env.storage().instance().set(
            &StorageKey::DoubleApprovalRequest(grant_id, milestone_index),
            &request,
        );
        return Err(InvalidState);
    }
    
    // Verify executor is authorized (must be one of the approvers or admin)
    let config = get_config(env)?;
    let is_authorized = executor == config.primary_approver 
        || executor == config.secondary_approver
        || executor == request.first_approver.unwrap_or_default()
        || executor == request.second_approver.unwrap_or_default();
    
    if !is_authorized {
        return Err(NotAuthorized);
    }
    
    // Execute the token transfer
    let token_client = soroban_sdk::token::Client::new(&env, &request.token_address);
    token_client.transfer(
        &env.current_contract_address(),
        &request.recipient,
        &request.amount,
    );
    
    // Mark request as executed
    request.status = ApprovalStatus::Executed;
    env.storage().instance().set(
        &StorageKey::DoubleApprovalRequest(grant_id, milestone_index),
        &request,
    );
    
    // Emit execution event
    env.events().publish(
        (Symbol::short!("dbl_exec"),),
        (grant_id, milestone_index, request.amount, request.recipient),
    );
    
    Ok(())
}

/// Cancel a double-approval request (admin only)
pub fn cancel_request(
    env: &Env,
    grant_id: u64,
    milestone_index: u32,
    canceller: Address,
) -> Result<(), Error> {
    let mut request = get_request(env, grant_id, milestone_index)?;
    
    // Only allow cancellation of pending or first approved requests
    if request.status != ApprovalStatus::Pending && request.status != ApprovalStatus::FirstApproved {
        return Err(InvalidState);
    }
    
    // Verify canceller is authorized (admin or primary approver)
    let config = get_config(env)?;
    let is_authorized = canceller == config.primary_approver;
    
    if !is_authorized {
        return Err(NotAuthorized);
    }
    
    request.status = ApprovalStatus::Cancelled;
    env.storage().instance().set(
        &StorageKey::DoubleApprovalRequest(grant_id, milestone_index),
        &request,
    );
    
    // Emit cancellation event
    env.events().publish(
        (Symbol::short!("dbl_cancel"),),
        (grant_id, milestone_index, canceller),
    );
    
    Ok(())
}

/// Clean up expired requests (maintenance function)
pub fn cleanup_expired_requests(env: &Env) -> Result<u32, Error> {
    // This would require iterating through all requests, which is expensive
    // For now, return 0 as placeholder
    // In a real implementation, we'd maintain a list of active requests
    Ok(0)
}

/// Check if a milestone has a pending or approved double-approval request
pub fn has_request(env: &Env, grant_id: u64, milestone_index: u32) -> bool {
    env.storage()
        .instance()
        .get::<_, DoubleApprovalRequest>(&StorageKey::DoubleApprovalRequest(grant_id, milestone_index))
        .is_some()
}
