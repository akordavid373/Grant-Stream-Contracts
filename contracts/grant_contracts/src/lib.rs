#![allow(unexpected_cfgs)]
#![no_std]

use core::cmp::min;
use soroban_sdk::{
    contract, contracterror, contractimpl, contracttype, panic_with_error, symbol_short, 
    token, Address, Env, Map, String, Symbol, Vec, vec, IntoVal,
};

// Import custom modules
use crate::wasm_hash_verification::{WasmHashVerification, VerificationError};
use crate::cross_chain_metadata::{CrossChainMetadata, MetadataError};

// --- Constants ---
pub const SCALING_FACTOR: i128 = 10_000_000; 
const RATE_INCREASE_TIMELOCK_SECS: u64 = 48 * 60 * 60;
const INACTIVITY_THRESHOLD_SECS: u64 = 90 * 24 * 60 * 60;
const CLAWBACK_WINDOW_SECS: u64 = 4 * 60 * 60;
const SNAPSHOT_EXPIRY: u64 = 86400;
const FLASH_LOAN_FEE_BPS: u32 = 50;
const MAX_SLASHING_REASON_LENGTH: u32 = 500;
const PAUSE_COOLDOWN_PERIOD: u64 = 14 * 24 * 60 * 60;
const SUPER_MAJORITY_THRESHOLD: u32 = 7500;

// --- Enums ---

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[contracttype]
pub enum GrantStatus {
    Active,
    Paused,
    Completed,
    RageQuitted,
    Cancelled,
    Slashed,
    MilestoneClaimed,
    MilestoneApproved,
    MilestoneChallenged,
    MilestoneRejected,
    DisputeRaised,
    ArbitrationPending,
    ArbitrationResolved,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[contracttype]
pub enum StreamType {
    FixedAmount,
    FixedEndDate,
    TimeLockedLease,
}

#[contracttype]
pub enum DataKey {
    Admin,
    GrantToken,
    GrantIds,
    Treasury,
    Oracle,
    NativeToken,
    Grant(u64),
    RecipientGrants(Address),
    LeaseAgreement(u64),
    FinancialSnapshot(u64, u64),
    SnapshotNonce(u64),
    SlashingProposal(u64),
    SlashingProposalIds,
    VotingPower(Address),
    NextProposalId,
    TotalVotingPower,
    SubDaoAuthorityContract,
    GasBuffer(u64),
    WithdrawalBuffer(u64, Address),
    ClawbackWindow(u64),
    MatchingPool(Address),
    DexPriceBuffer,
    RegulatedAssetInfo(Address),
    NextTaxReceiptId,
    IdentityOracleContract,
    FlashLoanProvider,
    ActiveFlashLoans,
}

#[contracterror]
#[derive(Clone, Copy, Eq, PartialEq, Debug)]
#[repr(u32)]
pub enum Error {
    NotInitialized = 1,
    AlreadyInitialized = 2,
    NotAuthorized = 3,
    GrantNotFound = 4,
    InvalidAmount = 7,
    InvalidState = 8,
    MathOverflow = 9,
    InsufficientReserve = 10,
    WithdrawalLimitExceeded = 200,
    ClawbackWindowActive = 65,
    PriceVolatilityExceeded = 75,
    NoStakeToSlash = 34,
    PauseCooldownActive = 63,
    InsufficientSuperMajority = 64,
}

// --- Structs ---

#[derive(Clone)]
#[contracttype]
pub struct Grant {
    pub recipient: Address,
    pub total_amount: i128,
    pub withdrawn: i128,
    pub claimable: i128,
    pub flow_rate: i128,
    pub base_flow_rate: i128,
    pub last_update_ts: u64,
    pub status: GrantStatus,
    pub token_address: Address,
    pub stream_type: StreamType,
    pub start_time: u64,
    pub priority_level: u32,
    
    // Unified from main & feat/Grant
    pub gas_buffer: i128,
    pub gas_buffer_used: i128,
    pub max_withdrawal_per_day: i128,
    pub last_withdrawal_timestamp: u64,
    pub withdrawal_amount_today: i128,
    
    // Arbitration Escrow fields
    pub escrow_amount: i128,
    pub arbitrator: Option<Address>,
    pub dispute_reason: Option<String>,
    
    // Lease fields
    pub lessor: Address,
    pub lease_terminated: bool,
    pub last_resume_timestamp: Option<u64>,
}

#[derive(Clone)]
#[contracttype]
pub struct GranteeConfig {
    pub recipient: Address,
    pub total_amount: i128,
    pub flow_rate: i128,
    pub asset: Address,
    pub warmup_duration: u64,
    pub validator: Option<Address>,
    pub gas_buffer: i128,
}

// --- Implementation ---

#[contract]
pub struct GrantContract;

#[contractimpl]
impl GrantContract {
    pub fn initialize(
        env: Env,
        admin: Address,
        grant_token: Address,
        treasury: Address,
        oracle: Address,
        native_token: Address,
    ) -> Result<(), Error> {
        if env.storage().instance().has(&DataKey::Admin) {
            return Err(Error::AlreadyInitialized);
        }
        env.storage().instance().set(&DataKey::Admin, &admin);
        env.storage().instance().set(&DataKey::GrantToken, &grant_token);
        env.storage().instance().set(&DataKey::Treasury, &treasury);
        env.storage().instance().set(&DataKey::Oracle, &oracle);
        env.storage().instance().set(&DataKey::NativeToken, &native_token);
        env.storage().instance().set(&DataKey::GrantIds, &Vec::<u64>::new(&env));
        Ok(())
    }

    pub fn withdraw(env: Env, grant_id: u64, amount: i128) -> Result<(), Error> {
        let mut grant = read_grant(&env, grant_id)?;
        grant.recipient.require_auth();

        // Check 24-hour limit
        let now = env.ledger().timestamp();
        if now >= grant.last_withdrawal_timestamp + 86400 {
            grant.withdrawal_amount_today = 0;
            grant.last_withdrawal_timestamp = now;
        }

        if grant.withdrawal_amount_today + amount > grant.max_withdrawal_per_day {
            return Err(Error::WithdrawalLimitExceeded);
        }

        // Logic for transfer...
        grant.claimable -= amount;
        grant.withdrawn += amount;
        grant.withdrawal_amount_today += amount;

        let token_client = token::Client::new(&env, &grant.token_address);
        token_client.transfer(&env.current_contract_address(), &grant.recipient, &amount);

        write_grant(&env, grant_id, &grant);
        Ok(())
    }
}

// --- Helpers ---

fn read_grant(env: &Env, grant_id: u64) -> Result<Grant, Error> {
    env.storage().instance().get(&DataKey::Grant(grant_id)).ok_or(Error::GrantNotFound)
}

fn write_grant(env: &Env, grant_id: u64, grant: &Grant) {
    env.storage().instance().set(&DataKey::Grant(grant_id), grant);
}

fn require_admin_auth(env: &Env) -> Result<Address, Error> {
    let admin: Address = env.storage().instance().get(&DataKey::Admin).ok_or(Error::NotInitialized)?;
    admin.require_auth();
    Ok(admin)
}