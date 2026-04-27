//! Admin Module - Governance Security Components
//!
//! This module provides security-focused admin functionality including:
//! - Dead Man's Switch: Automated admin recovery after inactivity
//! - Governance Activity Monitor: Circuit breaker for rapid parameter changes
//!
//! These components work together to ensure protocol security and proper
//! governance oversight while maintaining operational flexibility.

pub mod dead_mans_switch;
pub mod governance_activity_monitor;

// Re-export main types for easier integration
pub use dead_mans_switch::DeadMansSwitchContract;
pub use governance_activity_monitor::GovernanceActivityMonitor;
pub use governance_activity_monitor::{ParameterType, ChangeStatus, MonitorError};
