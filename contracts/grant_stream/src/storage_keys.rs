/// # Unified Storage Key Organization
/// 
/// This module provides a centralized, well-documented enum for all contract storage keys.
/// It prevents key collisions by using proper namespacing and clear categorization.
/// 
/// ## Key Categories:
/// 
/// 1. **Core Contract State** - Essential contract configuration and admin data
/// 2. **Grant Management** - All grant-related storage including metadata and balances
/// 3. **User Data** - User-specific balances, permissions, and grant associations
/// 4. **Treasury & Yield** - Treasury operations and yield farming data
/// 5. **Governance** - Proposal, voting, and governance-related storage
/// 6. **Circuit Breakers** - Safety mechanisms and monitoring data
/// 7. **Audit & Reporting** - Audit logs, tax reporting, and compliance data
/// 8. **Multi-Token Operations** - Multi-token and wrapped asset storage
/// 9. **Emergency & Recovery** - Multi-signature rescue operations
/// 10. **Reentrancy Protection** - Security guards against reentrancy attacks

use soroban_sdk::{contracttype, Address, Bytes};

/// Unified storage key enum with namespaced categories to prevent collisions
#[derive(Clone, Debug, PartialEq, Eq)]
#[contracttype]
pub enum StorageKey {
    // ── Core Contract State ──────────────────────────────────────────────────────
    
    /// Contract administrator address with full permissions
    Admin,
    /// Primary token address used for grants (e.g., USDC)
    GrantToken,
    /// Native token address (e.g., XLM) for fees and bounties
    NativeToken,
    /// Treasury address for holding and managing funds
    Treasury,
    /// Oracle address for price feeds and external data
    Oracle,
    /// Global list of all grant IDs for iteration
    GrantIds,
    /// Contract initialization status and metadata
    ContractInitialized,
    
    // ── Grant Management ────────────────────────────────────────────────────────
    
    /// Individual grant data keyed by grant ID
    Grant(u64),
    /// Grant milestone data keyed by (grant_id, milestone_index)
    Milestone(u64, u32),
    /// Expected monotonic nonce for off-chain milestone proof submission
    MilestoneSubmitNonce(u64),
    /// Grant streaming metadata and configuration
    GrantStreamConfig(u64),
    /// Grant legal compliance data (hashes, signatures)
    GrantLegalData(u64),
    /// Grant validator information and rewards
    GrantValidatorData(u64),
    /// Grant performance metrics and KPIs
    GrantMetrics(u64),
    /// Grant dispute status and resolution data
    GrantDisputeData(u64),
    /// Grant donor information for clawback authorization
    GrantDonor(u64),
    /// Clawback checkpoint data to prevent double-spending
    ClawbackCheckpoint(u64),
    /// Dispute escrow balance for contested clawbacks
    DisputeEscrow(u64),
    
    // ── User Data ───────────────────────────────────────────────────────────────
    
    /// List of grant IDs associated with a specific recipient
    RecipientGrants(Address),
    /// User-specific balance and withdrawal data
    UserBalance(Address),
    /// User permissions and role assignments
    UserPermissions(Address),
    /// User voting power and governance data
    UserVotingPower(Address),
    /// User tax reporting and flow history
    UserTaxData(Address),
    /// User audit trail and compliance records
    UserAuditTrail(Address),
    
    // ── Treasury & Yield Operations ────────────────────────────────────────────
    
    /// Treasury configuration parameters
    TreasuryConfig,
    /// Current yield position and investment data
    YieldPosition,
    /// Yield farming metrics and performance data
    YieldMetrics,
    /// Reserve balance for treasury operations
    ReserveBalance,
    /// Yield token address for farming operations
    YieldToken,
    /// Yield strategy configuration and parameters
    YieldStrategy,
    /// Harvest schedule and automation data
    HarvestSchedule,
    
    // ── Governance ─────────────────────────────────────────────────────────────
    
    /// Governance proposal data keyed by proposal ID
    Proposal(u64),
    /// Individual vote data keyed by (voter_address, proposal_id)
    Vote(Address, u64),
    /// Voting power allocation for users
    VotingPower(Address),
    /// List of all proposal IDs
    ProposalIds,
    /// Governance token address for voting
    GovernanceToken,
    /// Voting threshold configuration
    VotingThreshold,
    /// Quorum requirements for proposals
    QuorumThreshold,
    /// Council membership list (stored as raw bytes for efficiency)
    CouncilMembers,
    /// Stake token for governance participation
    StakeToken,
    /// Required stake amount for proposals
    ProposalStakeAmount,
    /// Optimistic proposal limits
    OptimisticLimit,
    /// Challenge bond requirements
    ChallengeBond,
    /// Conviction calculation parameters (basis points)
    ConvictionAlpha,
    
    // ── Circuit Breakers & Safety ─────────────────────────────────────────────
    
    /// Last confirmed oracle price (scaled by SCALING_FACTOR)
    LastOraclePrice,
    /// Sanity-check oracle address for price verification
    SanityOracle,
    /// Oracle price freeze status
    OracleFrozen,
    /// Total liquidity snapshot for velocity calculations
    TvlSnapshot,
    /// Velocity monitoring window start timestamp
    VelocityWindowStart,
    /// Cumulative withdrawals in current velocity window
    VelocityAccumulator,
    /// Soft pause status due to velocity limit breach
    SoftPaused,
    /// Oracle last heartbeat timestamp
    OracleLastHeartbeat,
    /// Oracle freeze due to missing heartbeat
    OracleFrozenDueToNoHeartbeat,
    /// Manual exchange rate set by governance
    ManualExchangeRate,
    /// Dispute monitoring window start
    DisputeWindowStart,
    /// Dispute count in current window
    DisputeAccumulator,
    /// Active grants count at dispute window start
    ActiveGrantsSnapshot,
    /// Grant initialization halt status
    GrantInitializationHalted,
    /// Rent preservation mode status
    RentPreservationMode,
    /// Rent balance threshold for preservation
    RentBufferThreshold,
    
    // ── Audit & Reporting ───────────────────────────────────────────────────────
    
    /// Rolling transaction counter for audit trails
    AuditTxCounter,
    /// Current Merkle root for audit verification
    AuditMerkleRoot,
    /// Individual audit log entries
    AuditLogEntry(u64),
    /// Tax reporting flow history for users
    TaxFlowHistory(Address),
    /// Compliance monitoring data
    ComplianceData,
    /// Regulatory reporting snapshots
    RegulatoryReport(u64),
    
    // ── Multi-Token Operations ─────────────────────────────────────────────────
    
    /// Wrapped asset data keyed by token address
    WrappedAsset(Address),
    /// Multi-token bridge configuration
    BridgeConfig,
    /// Cross-chain transaction tracking
    CrossChainTx(u64),
    /// Token oracle price feeds
    TokenPriceFeed(Address),
    
    // ── Emergency & Recovery ─────────────────────────────────────────────────
    
    /// Registered emergency signers for multi-sig operations
    EmergencySigners,
    /// Emergency rescue proposals keyed by proposal ID
    RescueProposal(u64),
    /// Emergency execution logs
    EmergencyExecutionLog(u64),
    /// Circuit breaker trigger events
    CircuitBreakerTrigger(u64),
    
    // ── Reentrancy Protection ───────────────────────────────────────────────────
    
    /// Global reentrancy guard lock
    ReentrancyGuard,
    /// Function-specific reentrancy locks
    FunctionReentrancyLock(Bytes),
    /// Operation timeout tracking
    OperationTimeout(Bytes),
    
    // ── Public Dashboard & Monitoring ──────────────────────────────────────────
    
    /// Last heartbeat timestamp for monitoring
    LastHeartbeat,
    /// Last TVL snapshot for dashboard
    LastTvl,
    /// Dashboard configuration parameters
    DashboardConfig,
    /// Health check metrics
    HealthMetrics,
    
    // ── Miscellaneous & Future Extensions ───────────────────────────────────────
    
    /// Contract version information
    ContractVersion,
    /// Feature flags for gradual rollouts
    FeatureFlag(Bytes),
    /// Temporary data (should be cleaned up)
    TemporaryData(Bytes),
    /// Migration status for contract upgrades
    MigrationStatus,
}

impl StorageKey {
    /// Returns the namespace category for this storage key
    /// Useful for debugging and storage analysis
    pub fn namespace(&self) -> &'static str {
        match self {
            // Core Contract State
            StorageKey::Admin
            | StorageKey::GrantToken
            | StorageKey::NativeToken
            | StorageKey::Treasury
            | StorageKey::Oracle
            | StorageKey::GrantIds
            | StorageKey::ContractInitialized => "core",
            
            // Grant Management
            StorageKey::Grant(_)
            | StorageKey::Milestone(_, _)
            | StorageKey::MilestoneSubmitNonce(_)
            | StorageKey::GrantStreamConfig(_)
            | StorageKey::GrantLegalData(_)
            | StorageKey::GrantValidatorData(_)
            | StorageKey::GrantMetrics(_)
            | StorageKey::GrantDisputeData(_)
            | StorageKey::GrantDonor(_)
            | StorageKey::ClawbackCheckpoint(_)
            | StorageKey::DisputeEscrow(_) => "grant",
            
            // User Data
            StorageKey::RecipientGrants(_)
            | StorageKey::UserBalance(_)
            | StorageKey::UserPermissions(_)
            | StorageKey::UserVotingPower(_)
            | StorageKey::UserTaxData(_)
            | StorageKey::UserAuditTrail(_) => "user",
            
            // Treasury & Yield
            StorageKey::TreasuryConfig
            | StorageKey::YieldPosition
            | StorageKey::YieldMetrics
            | StorageKey::ReserveBalance
            | StorageKey::YieldToken
            | StorageKey::YieldStrategy
            | StorageKey::HarvestSchedule => "treasury",
            
            // Governance
            StorageKey::Proposal(_)
            | StorageKey::Vote(_, _)
            | StorageKey::VotingPower(_)
            | StorageKey::ProposalIds
            | StorageKey::GovernanceToken
            | StorageKey::VotingThreshold
            | StorageKey::QuorumThreshold
            | StorageKey::CouncilMembers
            | StorageKey::StakeToken
            | StorageKey::ProposalStakeAmount
            | StorageKey::OptimisticLimit
            | StorageKey::ChallengeBond
            | StorageKey::ConvictionAlpha => "governance",
            
            // Circuit Breakers
            StorageKey::LastOraclePrice
            | StorageKey::SanityOracle
            | StorageKey::OracleFrozen
            | StorageKey::TvlSnapshot
            | StorageKey::VelocityWindowStart
            | StorageKey::VelocityAccumulator
            | StorageKey::SoftPaused
            | StorageKey::OracleLastHeartbeat
            | StorageKey::OracleFrozenDueToNoHeartbeat
            | StorageKey::ManualExchangeRate
            | StorageKey::DisputeWindowStart
            | StorageKey::DisputeAccumulator
            | StorageKey::ActiveGrantsSnapshot
            | StorageKey::GrantInitializationHalted
            | StorageKey::RentPreservationMode
            | StorageKey::RentBufferThreshold => "circuit_breaker",
            
            // Audit & Reporting
            StorageKey::AuditTxCounter
            | StorageKey::AuditMerkleRoot
            | StorageKey::AuditLogEntry(_)
            | StorageKey::TaxFlowHistory(_)
            | StorageKey::ComplianceData
            | StorageKey::RegulatoryReport(_) => "audit",
            
            // Multi-Token
            StorageKey::WrappedAsset(_)
            | StorageKey::BridgeConfig
            | StorageKey::CrossChainTx(_)
            | StorageKey::TokenPriceFeed(_) => "multi_token",
            
            // Emergency & Recovery
            StorageKey::EmergencySigners
            | StorageKey::RescueProposal(_)
            | StorageKey::EmergencyExecutionLog(_)
            | StorageKey::CircuitBreakerTrigger(_) => "emergency",
            
            // Reentrancy Protection
            StorageKey::ReentrancyGuard
            | StorageKey::FunctionReentrancyLock(_)
            | StorageKey::OperationTimeout(_) => "security",
            
            // Dashboard & Monitoring
            StorageKey::LastHeartbeat
            | StorageKey::LastTvl
            | StorageKey::DashboardConfig
            | StorageKey::HealthMetrics => "monitoring",
            
            // Miscellaneous
            StorageKey::ContractVersion
            | StorageKey::FeatureFlag(_)
            | StorageKey::TemporaryData(_)
            | StorageKey::MigrationStatus => "misc",
        }
    }
    
    /// Returns a human-readable description of this storage key
    pub fn description(&self) -> &'static str {
        match self {
            StorageKey::Admin => "Contract administrator address",
            StorageKey::GrantToken => "Primary token for grant operations",
            StorageKey::NativeToken => "Native token for fees and bounties",
            StorageKey::Treasury => "Treasury address for fund management",
            StorageKey::Oracle => "Oracle address for price feeds",
            StorageKey::GrantIds => "Global list of all grant IDs",
            StorageKey::ContractInitialized => "Contract initialization status",
            
            StorageKey::Grant(_) => "Individual grant data and metadata",
            StorageKey::Milestone(_, _) => "Grant milestone information",
            StorageKey::GrantStreamConfig(_) => "Grant streaming configuration",
            StorageKey::GrantLegalData(_) => "Grant legal compliance data",
            StorageKey::GrantValidatorData(_) => "Grant validator rewards data",
            StorageKey::GrantMetrics(_) => "Grant performance metrics",
            StorageKey::GrantDisputeData(_) => "Grant dispute status",
            StorageKey::GrantDonor(_) => "Grant donor information for clawback",
            StorageKey::ClawbackCheckpoint(_) => "Clawback checkpoint to prevent double-spending",
            StorageKey::DisputeEscrow(_) => "Dispute escrow balance for contested clawbacks",
            
            StorageKey::RecipientGrants(_) => "Grants associated with recipient",
            StorageKey::UserBalance(_) => "User balance information",
            StorageKey::UserPermissions(_) => "User permissions and roles",
            StorageKey::UserVotingPower(_) => "User voting power allocation",
            StorageKey::UserTaxData(_) => "User tax reporting data",
            StorageKey::UserAuditTrail(_) => "User audit trail records",
            
            StorageKey::TreasuryConfig => "Treasury configuration parameters",
            StorageKey::YieldPosition => "Current yield farming position",
            StorageKey::YieldMetrics => "Yield farming performance metrics",
            StorageKey::ReserveBalance => "Treasury reserve balance",
            StorageKey::YieldToken => "Token used for yield farming",
            StorageKey::YieldStrategy => "Yield farming strategy config",
            StorageKey::HarvestSchedule => "Automated harvest schedule",
            
            StorageKey::Proposal(_) => "Governance proposal data",
            StorageKey::Vote(_, _) => "Individual vote records",
            StorageKey::VotingPower(_) => "Voting power allocation",
            StorageKey::ProposalIds => "List of all proposal IDs",
            StorageKey::GovernanceToken => "Token used for governance",
            StorageKey::VotingThreshold => "Voting threshold configuration",
            StorageKey::QuorumThreshold => "Quorum requirements",
            StorageKey::CouncilMembers => "Council membership list",
            StorageKey::StakeToken => "Token for governance staking",
            StorageKey::ProposalStakeAmount => "Required stake for proposals",
            StorageKey::OptimisticLimit => "Optimistic proposal limits",
            StorageKey::ChallengeBond => "Challenge bond requirements",
            StorageKey::ConvictionAlpha => "Conviction calculation parameters",
            
            StorageKey::LastOraclePrice => "Last confirmed oracle price",
            StorageKey::SanityOracle => "Sanity-check oracle address",
            StorageKey::OracleFrozen => "Oracle price freeze status",
            StorageKey::TvlSnapshot => "Total liquidity snapshot",
            StorageKey::VelocityWindowStart => "Velocity monitoring start",
            StorageKey::VelocityAccumulator => "Cumulative withdrawals",
            StorageKey::SoftPaused => "Soft pause due to velocity",
            StorageKey::OracleLastHeartbeat => "Oracle last heartbeat",
            StorageKey::OracleFrozenDueToNoHeartbeat => "Oracle freeze (no heartbeat)",
            StorageKey::ManualExchangeRate => "Manual exchange rate",
            StorageKey::DisputeWindowStart => "Dispute monitoring start",
            StorageKey::DisputeAccumulator => "Dispute count in window",
            StorageKey::ActiveGrantsSnapshot => "Active grants count",
            StorageKey::GrantInitializationHalted => "Grant init halt status",
            StorageKey::RentPreservationMode => "Rent preservation mode",
            StorageKey::RentBufferThreshold => "Rent buffer threshold",
            
            StorageKey::AuditTxCounter => "Audit transaction counter",
            StorageKey::AuditMerkleRoot => "Current audit Merkle root",
            StorageKey::AuditLogEntry(_) => "Individual audit log entry",
            StorageKey::TaxFlowHistory(_) => "User tax flow history",
            StorageKey::ComplianceData => "Compliance monitoring data",
            StorageKey::RegulatoryReport(_) => "Regulatory report snapshot",
            
            StorageKey::WrappedAsset(_) => "Wrapped asset data",
            StorageKey::BridgeConfig => "Multi-token bridge config",
            StorageKey::CrossChainTx(_) => "Cross-chain transaction",
            StorageKey::TokenPriceFeed(_) => "Token price feed data",
            
            StorageKey::EmergencySigners => "Emergency signer set",
            StorageKey::RescueProposal(_) => "Emergency rescue proposal",
            StorageKey::EmergencyExecutionLog(_) => "Emergency execution log",
            StorageKey::CircuitBreakerTrigger(_) => "Circuit breaker trigger",
            
            StorageKey::ReentrancyGuard => "Global reentrancy guard",
            StorageKey::FunctionReentrancyLock(_) => "Function-specific lock",
            StorageKey::OperationTimeout(_) => "Operation timeout tracking",
            
            StorageKey::LastHeartbeat => "Last monitoring heartbeat",
            StorageKey::LastTvl => "Last TVL snapshot",
            StorageKey::DashboardConfig => "Dashboard configuration",
            StorageKey::HealthMetrics => "Health check metrics",
            
            StorageKey::ContractVersion => "Contract version info",
            StorageKey::FeatureFlag(_) => "Feature flag configuration",
            StorageKey::TemporaryData(_) => "Temporary storage data",
            StorageKey::MigrationStatus => "Contract migration status",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_storage_key_namespace() {
        assert_eq!(StorageKey::Admin.namespace(), "core");
        assert_eq!(StorageKey::Grant(123).namespace(), "grant");
        assert_eq!(StorageKey::RecipientGrants(Address::random()).namespace(), "user");
        assert_eq!(StorageKey::TreasuryConfig.namespace(), "treasury");
        assert_eq!(StorageKey::Proposal(456).namespace(), "governance");
        assert_eq!(StorageKey::OracleFrozen.namespace(), "circuit_breaker");
        assert_eq!(StorageKey::AuditTxCounter.namespace(), "audit");
        assert_eq!(StorageKey::WrappedAsset(Address::random()).namespace(), "multi_token");
        assert_eq!(StorageKey::EmergencySigners.namespace(), "emergency");
        assert_eq!(StorageKey::ReentrancyGuard.namespace(), "security");
        assert_eq!(StorageKey::LastHeartbeat.namespace(), "monitoring");
        assert_eq!(StorageKey::ContractVersion.namespace(), "misc");
    }

    #[test]
    fn test_storage_key_description() {
        assert!(!StorageKey::Admin.description().is_empty());
        assert!(!StorageKey::Grant(123).description().is_empty());
        assert!(!StorageKey::RecipientGrants(Address::random()).description().is_empty());
    }
}
