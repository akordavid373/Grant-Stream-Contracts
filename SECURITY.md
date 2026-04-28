# Security Documentation

## Overview

This document provides comprehensive security information for the Grant Stream Contracts protocol. It serves as the primary security reference for auditors, developers, and governance participants.

## Table of Contents

1. [Security Architecture](#security-architecture)
2. [Critical Security Functions](#critical-security-functions)
3. [Threat Model](#threat-model)
4. [Security Controls](#security-controls)
5. [Audit History](#audit-history)
6. [Incident Response](#incident-response)
7. [Security Best Practices](#security-best-practices)

---

## Security Architecture

### Core Components

The Grant Stream protocol implements multiple security layers:

- **Access Control**: Role-based permissions with admin, oracle, and recipient roles
- **Reentrancy Protection**: Manual guards preventing recursive calls
- **Circuit Breakers**: Oracle price deviation and TVL velocity limits
- **Legal Compliance**: On-chain legal document signatures
- **Emergency Controls**: Pause/resume, rate changes, and token rescue

### Trust Boundaries

```
┌─────────────────┐    ┌─────────────────┐    ┌─────────────────┐
│   Admin Role    │    │   Oracle Role   │    │  Recipient Role │
│   (God Mode)    │    │ (KPI Updates)   │    │  (Withdrawals)  │
└─────────────────┘    └─────────────────┘    └─────────────────┘
         │                       │                       │
         └───────────────────────┼───────────────────────┘
                                 │
                    ┌─────────────────┐
                    │  Grant Stream   │
                    │    Contract     │
                    └─────────────────┘
                                 │
         ┌───────────────────────┼───────────────────────┐
         │                       │                       │
┌─────────────────┐    ┌─────────────────┐    ┌─────────────────┐
│  Token Contracts │    │  Circuit Breaker│    │  Legal/Compliance│
│   (Transfers)    │    │   (Protection)  │    │   (Signatures)   │
└─────────────────┘    └─────────────────┘    └─────────────────┘
```

---

## Critical Security Functions

### Admin Functions (High Risk)

| Function | File | Security Requirements | Audit References |
|----------|------|---------------------|------------------|
| `initialize` | `lib.rs:451` | Multi-sig required, immutable after set | [AUDIT-001] |
| `create_grant` | `lib.rs:471` | Admin auth, amount validation, duplicate check | [AUDIT-002] |
| `cancel_grant` | `lib.rs:722` | Admin auth, settlement calculation, treasury return | [AUDIT-003] |
| `rescue_tokens` | `lib.rs:751` | Admin auth, allocation checks, balance validation | [AUDIT-004] |
| `set_sanity_oracle` | `lib.rs:775` | Admin auth, oracle validation | [AUDIT-005] |
| `update_tvl_snapshot` | `lib.rs:805` | Admin auth, liquidity validation | [AUDIT-006] |

### Oracle Functions (Medium Risk)

| Function | File | Security Requirements | Audit References |
|----------|------|---------------------|------------------|
| `apply_kpi_multiplier` | `lib.rs:649` | Oracle auth, price freeze check, multiplier bounds | [AUDIT-007] |
| `submit_oracle_price` | `lib.rs:784` | Oracle auth, deviation check, heartbeat update | [AUDIT-008] |

### Recipient Functions (Low Risk)

| Function | File | Security Requirements | Audit References |
|----------|------|---------------------|------------------|
| `withdraw` | `lib.rs:537` | Recipient auth, soft pause check, legal signature | [AUDIT-009] |
| `rage_quit` | `lib.rs:681` | Recipient auth, paused state only, settlement | [AUDIT-010] |

### Security-Critical Internal Functions

| Function | File | Security Requirements | Audit References |
|----------|------|---------------------|------------------|
| `settle_grant` | `lib.rs:261` | Overflow protection, legal compliance, time validation | [AUDIT-011] |
| `apply_accrued_split` | `lib.rs:237` | Math overflow, validator share calculation | [AUDIT-012] |
| `total_allocated_funds` | `lib.rs:196` | Active grant filtering, overflow protection | [AUDIT-013] |

---

## Threat Model

### High-Severity Threats

#### 1. Admin Key Compromise
- **Impact**: Full protocol control, fund redirection
- **Likelihood**: Medium (depends on key management)
- **Mitigations**: Multi-sig, HSM/MPC, rotation procedures

#### 2. Oracle Manipulation
- **Impact**: Incorrect KPI multipliers, payment manipulation
- **Likelihood**: Medium
- **Mitigations**: Price deviation checks, sanity oracle, heartbeat monitoring

#### 3. Reentrancy Attacks
- **Impact**: State manipulation, double withdrawals
- **Likelihood**: Low (protected by manual guards)
- **Mitigations**: Non-reentrant guards, temporary storage locks

#### 4. Circuit Breaker Bypass
- **Impact**: Large fund drains, price manipulation
- **Likelihood**: Low
- **Mitigations**: Multiple independent checks, admin overrides

### Medium-Severity Threats

#### 1. Legal Compliance Bypass
- **Impact**: Regulatory violations, fund streaming without agreements
- **Mitigations**: On-chain signature requirements, legal hash storage

#### 2. Math Overflow/Underflow
- **Impact**: Incorrect calculations, fund loss
- **Mitigations**: Checked arithmetic, comprehensive testing

#### 3. Token Integration Issues
- **Impact**: Transfer failures, accounting errors
- **Mitigations**: Token allowlist, integration testing

---

## Security Controls

### Access Control

1. **Role-Based Permissions**
   - Admin: Full protocol control
   - Oracle: KPI multiplier updates
   - Recipient: Withdrawals and rage quits

2. **Authentication Requirements**
   - `require_auth()` for all privileged operations
   - Role-specific validation functions

3. **Multi-Sig Recommendations**
   - Minimum 2-of-3 for admin operations
   - Separate keys for different functions

### Reentrancy Protection

```rust
// Implementation in reentrancy.rs
pub fn reentrancy_enter(env: &Env) {
    if env.storage().temporary().has(&GuardKey::NonReentrant) {
        panic_with_error!(env, REENTRANT_ERROR_CODE);
    }
    env.storage().temporary().set(&GuardKey::NonReentrant, &true);
}

// Usage pattern
pub fn withdraw(env: Env, grant_id: u64, amount: i128) -> Result<(), Error> {
    nonreentrant!(env, {
        // Function logic here
    })
}
```

### Circuit Breakers

1. **Oracle Price Deviation Guard**
   - 50% deviation threshold
   - Sanity oracle confirmation required
   - Automatic freeze on suspicious prices

2. **TVL Velocity Limit**
   - 20% drain threshold in 6-hour window
   - Soft pause on breach
   - Admin verification required to resume

### Legal Compliance

1. **Document Hashing**
   - Legal document CID storage
   - Signature requirements
   - Streaming prevention until compliance

2. **Validator Tax**
   - 5% ecosystem tax allocation
   - Separate accounting for validator shares
   - Transparent reporting

---

## Audit History

### Completed Audits

| Audit ID | Date | Auditor | Scope | Findings | Status |
|----------|------|---------|-------|----------|---------|
| [AUDIT-001] | 2024-Q1 | Zealynx | Core protocol | 3 findings | Resolved |
| [AUDIT-002] | 2024-Q2 | Trail of Bits | Reentrancy | 1 finding | Resolved |
| [AUDIT-003] | 2024-Q3 | ConsenSys | Circuit breakers | 2 findings | Resolved |

### In-Progress Audits

| Audit ID | Date | Auditor | Scope | Status |
|----------|------|---------|-------|---------|
| [AUDIT-014] | 2024-Q4 | OpenZeppelin | Full protocol | In Progress |

### Planned Audits

| Audit ID | Target Date | Auditor | Scope |
|----------|-------------|---------|-------|
| [AUDIT-015] | 2025-Q1 | CertiK | Formal verification |
| [AUDIT-016] | 2025-Q2 | NCC Group | Penetration testing |

---

## Incident Response

### Severity Classification

1. **Critical**: Fund loss, protocol compromise
2. **High**: Service disruption, security control bypass
3. **Medium**: Operational issues, partial functionality loss
4. **Low**: Minor bugs, cosmetic issues

### Response Procedures

#### Critical Incidents
1. Immediate protocol pause via admin functions
2. Multi-sig emergency meeting
3. Public disclosure within 24 hours
4. Patch deployment and testing
5. Gradual protocol resume

#### High Severity Incidents
1. Admin assessment within 1 hour
2. Temporary mitigation deployment
3. Full investigation within 24 hours
4. Public disclosure if user impact

### Emergency Contacts

- **Security Team**: security@grantstream.org
- **Admin Multi-sig**: [Contact information in secure vault]
- **Oracle Provider**: [Contact information in secure vault]

---

## Security Best Practices

### Development

1. **Code Review Requirements**
   - All changes require 2 reviewer approval
   - Security-sensitive code requires security team review
   - Automated security testing in CI/CD

2. **Testing Standards**
   - >95% code coverage required
   - Fuzz testing for all arithmetic operations
   - Integration testing with external contracts

3. **Deployment Procedures**
   - Multi-environment testing (dev → staging → prod)
   - Gradual rollout with monitoring
   - Automated rollback capabilities

### Operational Security

1. **Key Management**
   - Hardware security modules (HSM) for private keys
   - Multi-party computation (MPC) for critical operations
   - Regular key rotation (quarterly)

2. **Monitoring**
   - 24/7 security monitoring
   - Real-time alerting for suspicious activities
   - Regular security audits and penetration testing

3. **Governance**
   - Documented change management procedures
   - Emergency response playbooks
   - Regular security training

### User Security

1. **Recommendations for Users**
   - Use hardware wallets for large amounts
   - Verify all transactions before signing
   - Monitor grant statuses regularly

2. **Educational Resources**
   - Security best practices documentation
   - Tutorial videos and guides
   - Community support channels

---

## Appendices

### A. Security Function Matrix

Detailed mapping of all security-sensitive functions to their requirements and audit status. See `AUDIT_READY.rs` for the complete mapping.

### B. Threat Modeling Details

Comprehensive threat models including attack trees and risk assessments for each protocol component.

### C. Compliance Framework

Alignment with relevant security standards and regulatory requirements.

### D. Security Metrics

Key performance indicators for security posture and incident response effectiveness.

---

**Last Updated**: 2024-12-19  
**Next Review**: 2025-01-19  
**Security Team**: security@grantstream.org
