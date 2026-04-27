#![allow(unexpected_cfgs)]
#![no_std]

use core::cmp::min;

use soroban_sdk::{
    contract, contracterror, contractimpl, contracttype, symbol_short, token, Address, Env, Vec,
    Symbol, vec, IntoVal, Map,
};

// --- Constants ---
pub const SCALING_FACTOR: i128 = 10_000_000; // 1e7

// --- Submodules ---
pub mod oracle_integration;
pub mod clawback_resilient;
pub mod compliance_screening;
pub mod quadratic_voting;
