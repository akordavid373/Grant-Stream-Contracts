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

#[derive(Clone, Debug, PartialEq, Eq)]
#[contracttype]
pub struct MilestoneKey(u64, u32);

#[derive(Clone, Debug, PartialEq, Eq)]
#[contracttype]
pub struct VoteKey(Address, u64);

/// Unified storage key enum with namespaced categories to prevent collisions
/// NOTE: All variant names must be <= 9 characters for Soroban symbol compatibility
#[contracttype]
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Key {
    // ── Core Contract State ──────────────────────────────────────────────────────
    
    /// Contract administrator address with full permissions
    Admin,
    /// Primary token address used for grants (e.g., USDC)
    GrantTok,
    /// Native token address (e.g., XLM) for fees and bounties
    NativeTok,
    /// Treasury address for holding and managing funds
    Treasury,
    /// Oracle address for price feeds and external data
    Oracle,
    /// Global list of all grant IDs for iteration
    GrantIds,
    /// Initialization marker flag
    Init,
    
    // ── Grant Management ────────────────────────────────────────────────────────
    
    /// Individual grant data keyed by grant ID
    Grant(u64),
    /// Grant milestone data keyed by (grant_id, milestone_index)
    Milestone(MilestoneKey),
    /// Clawback protection record keyed by grant ID
    Clawback(u64),
    /// Grant configuration metadata keyed by grant ID
    GrantCfg(u64),
    /// Grant legal metadata keyed by grant ID
    GrantLeg(u64),
    /// Grant validator metadata keyed by grant ID
    GrantVal(u64),
    /// Grant metrics data keyed by grant ID
    GrantMet(u64),
    /// Grant dispute metadata keyed by grant ID
    GrantDis(u64),

    // ── User Data ───────────────────────────────────────────────────────────────
    
    /// List of grant IDs associated with a recipient
    RecipGnt(Address),
    /// Stored user balance data
    UserBal(Address),
    /// User permission metadata
    UserPerm(Address),
    /// User voting metadata
    UserVote(Address),
    /// User tax metadata
    UserTax(Address),
    /// User audit trail metadata
    UserAud(Address),

    // ── Treasury & Yield ─────────────────────────────────────────────────────────

    /// Yield contract configuration
    Config,
    /// Current yield position data
    YieldPos,
    /// Yield metrics and performance data
    Metrics,
    /// Current reserve balance for yield operations
    ResBal,
    /// Yield token address
    YieldTok,
    /// Selected yield strategy metadata
    YieldStr,
    /// Harvest schedule or record metadata
    Harvest,

    // ── Governance ───────────────────────────────────────────────────────────────

    /// Proposal storage keyed by proposal ID
    Proposal(u64),
    /// Vote storage keyed by (voter, proposal_id)
    Vote(VoteKey),
    /// Voter conviction and power data keyed by address
    VotePow(Address),
    /// Global list of proposal IDs
    PropIds,
    /// Governance token address
    GovTok,
    /// Voting threshold configuration
    VoteThres,
    /// Quorum threshold configuration
    Quorum,
    /// Council member list
    Council,
    /// Stake token address
    StakeTok,
    /// Proposal stake amount configuration
    PropStake,
    /// Optimistic execution limit
    OptLimit,
    /// Challenge bond configuration
    ChalBond,
    /// Conviction alpha configuration
    Convict,

    // ── Circuit Breakers ─────────────────────────────────────────────────────────
    
    /// Last oracle price recorded
    LastPric,
    /// Sanity-check oracle address
    SanityOra,
    /// Oracle freeze flag due to price deviation
    OraFrozen,
    /// TVL snapshot for velocity checks
    TvlSnap,
    /// Velocity window start timestamp
    VelWinSt,
    /// Velocity accumulator over the window
    VelAccum,
    /// Soft pause flag for velocity breaches
    SoftPa,
    /// Oracle heartbeat timestamp
    OraHeart,
    /// Oracle freeze flag due to heartbeat failure
    OraFrzHb,
    /// Manual exchange rate override
    ManRate,
    /// Dispute window start timestamp
    DispWin,
    /// Dispute count accumulator
    DispAcc,
    /// Active grants snapshot for dispute ratio
    ActGntSn,
    /// Grant initialization halt flag
    GntHalt,
    /// Rent preservation mode flag
    RentMode,
    /// Rent buffer threshold configuration
    RentThres,

    // ── Audit & Reporting ────────────────────────────────────────────────────────

    /// Audit transaction counter
    AudTxCnt,
    /// Audit merkle root for log verification
    AudRoot,
    /// Individual audit log entry keyed by index
    AudLog(u64),
    /// Tax flow history keyed by recipient address
    TaxHist(Address),
    /// Compliance metadata
    ComplDat,
    /// Regulatory report keyed by report ID
    RegRep(u64),

    // ── Multi-Token Operations ────────────────────────────────────────────────────

    /// Wrapped asset configuration keyed by token address
    WrapAst(Address),
    /// Multi-token bridge configuration
    BridgeCfg,
    /// Cross-chain transaction record keyed by ID
    CrossTx(u64),
    /// Token price feed configuration keyed by token address
    TokPrice(Address),

    // ── Emergency & Recovery ─────────────────────────────────────────────────────

    /// Emergency signer list
    EmergSig,
    /// Rescue proposal keyed by proposal ID
    RescProp(u64),
    /// Emergency execution log entry keyed by index
    EmergLog(u64),
    /// Circuit breaker trigger record keyed by index
    CircTrig(u64),

    // ── Reentrancy Protection ─────────────────────────────────────────────────────

    /// Reentrancy guard marker
    ReentGd,
    /// Function-specific reentrancy lock keyed by bytes
    FuncLock(Bytes),
    /// Operation timeout keyed by bytes
    OpTime(Bytes),

    // ── Monitoring & Diagnostics ─────────────────────────────────────────────────

    /// Last contract heartbeat timestamp
    LastHb,
    /// Last reported TVL timestamp
    LastTvl,
    /// Dashboard configuration storage
    DashCfg,
    /// Health metrics and diagnostics storage
    Health,

    // ── Miscellaneous ────────────────────────────────────────────────────────────

    /// Contract version metadata
    Version,
    /// Feature flag keyed by arbitrary bytes
    Feature(Bytes),
    /// Temporary arbitrary data storage keyed by bytes
    TempData(Bytes),
    /// Migration status marker
    MigStat,
}

pub type StorageKey = Key;

impl Key {
    /// Returns the namespace category for this storage key
    /// Useful for debugging and storage analysis
    pub fn namespace(&self) -> &'static str {
        match self {
            Key::Admin
            | Key::GrantTok
            | Key::NativeTok
            | Key::Treasury
            | Key::Oracle
            | Key::GrantIds
            | Key::Init => "core",

            Key::Grant(_)
            | Key::Milestone(_)
            | Key::Clawback(_)
            | Key::GrantCfg(_)
            | Key::GrantLeg(_)
            | Key::GrantVal(_)
            | Key::GrantMet(_)
            | Key::GrantDis(_) => "grant",

            Key::RecipGnt(_)
            | Key::UserBal(_)
            | Key::UserPerm(_)
            | Key::UserVote(_)
            | Key::UserTax(_)
            | Key::UserAud(_) => "user",

            Key::Config
            | Key::YieldPos
            | Key::Metrics
            | Key::ResBal
            | Key::YieldTok
            | Key::YieldStr
            | Key::Harvest => "treasury",

            Key::Proposal(_)
            | Key::Vote(_)
            | Key::VotePow(_)
            | Key::PropIds
            | Key::GovTok
            | Key::VoteThres
            | Key::Quorum
            | Key::Council
            | Key::StakeTok
            | Key::PropStake
            | Key::OptLimit
            | Key::ChalBond
            | Key::Convict => "governance",

            Key::LastPric
            | Key::SanityOra
            | Key::OraFrozen
            | Key::TvlSnap
            | Key::VelWinSt
            | Key::VelAccum
            | Key::SoftPa
            | Key::OraHeart
            | Key::OraFrzHb
            | Key::ManRate
            | Key::DispWin
            | Key::DispAcc
            | Key::ActGntSn
            | Key::GntHalt
            | Key::RentMode
            | Key::RentThres => "circuit_breaker",

            Key::AudTxCnt
            | Key::AudRoot
            | Key::AudLog(_)
            | Key::TaxHist(_)
            | Key::ComplDat
            | Key::RegRep(_) => "audit",

            Key::WrapAst(_)
            | Key::BridgeCfg
            | Key::CrossTx(_)
            | Key::TokPrice(_) => "multi_token",

            Key::EmergSig
            | Key::RescProp(_)
            | Key::EmergLog(_)
            | Key::CircTrig(_) => "emergency",

            Key::ReentGd
            | Key::FuncLock(_)
            | Key::OpTime(_) => "security",

            Key::LastHb
            | Key::LastTvl
            | Key::DashCfg
            | Key::Health => "monitoring",

            Key::Version
            | Key::Feature(_)
            | Key::TempData(_)
            | Key::MigStat => "misc",
        }
    }

    /// Returns a human-readable description of the storage key
    /// Useful for debugging and documentation
    pub fn description(&self) -> &'static str {
        match self {
            Key::Admin => "Contract administrator address",
            Key::GrantTok => "Primary grant token address",
            Key::NativeTok => "Native token address",
            Key::Treasury => "Treasury address",
            Key::Oracle => "Oracle address",
            Key::GrantIds => "Global list of grant IDs",
            Key::Init => "Initialization marker flag",
            Key::Grant(_) => "Individual grant data",
            Key::Milestone(_) => "Grant milestone data",
            Key::Clawback(_) => "Clawback protection record",
            Key::GrantCfg(_) => "Grant configuration metadata",
            Key::GrantLeg(_) => "Grant legal metadata",
            Key::GrantVal(_) => "Grant validator metadata",
            Key::GrantMet(_) => "Grant metrics data",
            Key::GrantDis(_) => "Grant dispute metadata",
            Key::RecipGnt(_) => "Recipient grant associations",
            Key::UserBal(_) => "User balance data",
            Key::UserPerm(_) => "User permission metadata",
            Key::UserVote(_) => "User voting metadata",
            Key::UserTax(_) => "User tax metadata",
            Key::UserAud(_) => "User audit metadata",
            Key::Config => "Yield treasury configuration",
            Key::YieldPos => "Current yield position",
            Key::Metrics => "Yield performance metrics",
            Key::ResBal => "Yield reserve balance",
            Key::YieldTok => "Yield token address",
            Key::YieldStr => "Yield strategy metadata",
            Key::Harvest => "Yield harvest record",
            Key::Proposal(_) => "Governance proposal metadata",
            Key::Vote(_) => "Governance vote record",
            Key::VotePow(_) => "Voter power metadata",
            Key::PropIds => "Global proposal ID list",
            Key::GovTok => "Governance token address",
            Key::VoteThres => "Voting threshold configuration",
            Key::Quorum => "Quorum threshold configuration",
            Key::Council => "Council member list",
            Key::StakeTok => "Stake token address",
            Key::PropStake => "Proposal stake amount",
            Key::OptLimit => "Optimistic execution limit",
            Key::ChalBond => "Challenge bond configuration",
            Key::Convict => "Conviction alpha setting",
            Key::LastPric => "Last recorded oracle price",
            Key::SanityOra => "Sanity oracle address",
            Key::OraFrozen => "Oracle price freeze flag",
            Key::TvlSnap => "TVL snapshot for velocity checks",
            Key::VelWinSt => "Velocity window start timestamp",
            Key::VelAccum => "Velocity accumulator",
            Key::SoftPa => "Soft pause status flag",
            Key::OraHeart => "Oracle heartbeat timestamp",
            Key::OraFrzHb => "Oracle heartbeat freeze flag",
            Key::ManRate => "Manual exchange rate override",
            Key::DispWin => "Dispute window start timestamp",
            Key::DispAcc => "Dispute accumulator",
            Key::ActGntSn => "Active grants snapshot",
            Key::GntHalt => "Grant initialization halt flag",
            Key::RentMode => "Rent preservation mode flag",
            Key::RentThres => "Rent buffer threshold",
            Key::AudTxCnt => "Audit transaction counter",
            Key::AudRoot => "Audit merkle root",
            Key::AudLog(_) => "Audit log entry",
            Key::TaxHist(_) => "Tax flow history",
            Key::ComplDat => "Compliance metadata",
            Key::RegRep(_) => "Regulatory report record",
            Key::WrapAst(_) => "Wrapped asset configuration",
            Key::BridgeCfg => "Bridge configuration",
            Key::CrossTx(_) => "Cross-chain transaction record",
            Key::TokPrice(_) => "Token price feed configuration",
            Key::EmergSig => "Emergency signer list",
            Key::RescProp(_) => "Rescue proposal data",
            Key::EmergLog(_) => "Emergency execution log entry",
            Key::CircTrig(_) => "Circuit breaker trigger record",
            Key::ReentGd => "Reentrancy guard marker",
            Key::FuncLock(_) => "Function reentrancy lock",
            Key::OpTime(_) => "Operation timeout record",
            Key::LastHb => "Last heartbeat timestamp",
            Key::LastTvl => "Last TVL measurement",
            Key::DashCfg => "Dashboard configuration data",
            Key::Health => "Health metrics and diagnostics",
            Key::Version => "Contract version metadata",
            Key::Feature(_) => "Feature flag storage",
            Key::TempData(_) => "Temporary data storage",
            Key::MigStat => "Migration status marker",
        }
    }
}
