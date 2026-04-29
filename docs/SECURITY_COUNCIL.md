# Security Council Implementation

## Overview

The Security Council is a critical security layer designed to protect the Grant Stream protocol from governance attacks and social engineering. It implements a 3-of-5 multi-signature veto authority over DAO-triggered sensitive operations.

## Problem Statement

Governance tokens can be hijacked through various attack vectors:
- **51% attacks**: Malicious actors accumulate voting power
- **Flash loan governance attacks**: Temporary token borrowing to pass malicious proposals
- **Social engineering**: Manipulation of token holders to approve harmful actions
- **Compromised keys**: Admin or DAO keys falling into wrong hands

Without additional safeguards, a compromised DAO could:
- Claw back all active grants
- Drain the treasury
- Change critical protocol parameters
- Lock out legitimate users

## Solution: Security Council

### Architecture

The Security Council provides a **human-in-the-loop** security layer with the following characteristics:

1. **3-of-5 Multi-Sig Authority**: Requires 3 out of 5 council members to veto an action
2. **48-Hour Timelock**: All governance-sensitive operations have a mandatory 48-hour delay
3. **Veto Power Only**: Council cannot initiate actions, only block them
4. **Annual Key Rotation**: Council keys must be rotated yearly via 7-day DAO-approved timelock
5. **Checks and Balances**: Neither DAO nor Council has absolute power

### Protected Operations

The following operations are subject to Security Council oversight:

- **Clawback**: Canceling grants and returning funds to treasury
- **Emergency Pause**: Protocol-wide emergency stops
- **Rate Changes**: Significant flow rate modifications
- **Treasury Withdrawals**: Large fund movements
- **Admin Changes**: Changing protocol administrators

## How It Works

### 1. Action Creation (Timelock Starts)

When the DAO or admin wants to perform a sensitive operation:

```rust
// Create a timelocked clawback action
let action_id = contract.create_timelocked_action(
    env,
    ActionType::Clawback,
    Some(grant_id),
    dao_address,
    parameters
);
```

- Action enters **Pending** state
- 48-hour timelock begins
- `ActionPending` event emitted
- Community is alerted

### 2. Security Council Review (0-48 Hours)

During the timelock window, Security Council members review the action:

```rust
// Council member 1 signs veto
contract.council_sign_veto(env, action_id, council_member_1);

// Council member 2 signs veto
contract.council_sign_veto(env, action_id, council_member_2);

// Council member 3 signs veto (threshold reached)
contract.council_sign_veto(env, action_id, council_member_3);
```

- Each signature is recorded
- When 3 signatures reached, action is **Vetoed**
- `ActionVetoed` event emitted
- Action is permanently blocked

### 3. Execution or Veto (After 48 Hours)

**If NOT vetoed:**
```rust
// After 48 hours, action can be executed
contract.execute_timelocked_action(env, action_id);
```

**If vetoed:**
```rust
// Execution fails - action is blocked
// Error: ActionAlreadyVetoed
```

## Example: Rogue DAO Attack

### Attack Scenario

1. Malicious actor accumulates 51% of governance tokens
2. Proposes to claw back all active grants (stealing donor funds)
3. Votes pass through compromised DAO
4. Clawback action is initiated

### Defense Mechanism

```rust
// Step 1: Malicious clawback initiated
let action_id = contract.protected_clawback(
    env,
    grant_id,
    rogue_dao_address
);
// 48-hour timelock starts

// Step 2: Community alerts Security Council
// Council members review the suspicious action

// Step 3: Council vetoes within 48 hours
contract.council_sign_veto(env, action_id, council_member_1);
contract.council_sign_veto(env, action_id, council_member_2);
contract.council_sign_veto(env, action_id, council_member_3);
// Action is VETOED - funds are safe

// Step 4: Execution attempt fails
let result = contract.execute_protected_clawback(env, action_id, grant_id);
// Error: ActionAlreadyVetoed
```

**Result**: Treasury protected, grants continue streaming, attack thwarted.

## Council Key Rotation

To prevent council key compromise, annual rotation is mandatory:

### Rotation Process

```rust
// Step 1: DAO proposes new council members (7-day timelock)
contract.propose_council_rotation(
    env,
    new_council_members,
    dao_admin
);

// Step 2: Wait 7 days for community review

// Step 3: Execute rotation
contract.execute_council_rotation(env);
```

### Rotation Requirements

- Must occur at least once per year
- Requires DAO approval
- 7-day timelock for community review
- All 5 members must be replaced simultaneously
- Old council loses veto power immediately after rotation

### Checking Rotation Status

```rust
// Check if rotation is due
let is_due = contract.is_council_rotation_due(env);

// Get current council members
let members = contract.get_council_members(env);
```

## Security Properties

### 1. No Absolute Power

- **DAO**: Can propose actions but cannot execute immediately
- **Council**: Can block actions but cannot initiate them
- **Admin**: Subject to same timelock and veto rules

### 2. Time-Bounded Defense

- 48-hour window provides time for:
  - Community review
  - Council deliberation
  - Emergency response coordination
  - Public discourse

### 3. Transparent Operations

All actions emit events:
- `ActionPending`: New timelocked action created
- `VetoSigned`: Council member signs veto
- `ActionVetoed`: Action blocked by council
- `ActionExecuted`: Action completed successfully
- `CouncilRotationProposed`: New members proposed
- `CouncilRotationExecuted`: Rotation completed

### 4. Fail-Safe Design

- If council doesn't act, legitimate actions proceed
- If council acts maliciously, DAO can rotate them (7-day delay)
- If DAO is compromised, council can block attacks (48-hour window)

## API Reference

### Initialization

```rust
fn initialize_security_council(env: Env, members: Vec<Address>) -> Result<(), Error>
```

Initialize the Security Council with 5 members. Admin only.

### Action Management

```rust
fn create_timelocked_action(
    env: Env,
    action_type: ActionType,
    target_grant_id: Option<u64>,
    initiator: Address,
    parameters: Vec<i128>
) -> Result<u64, Error>
```

Create a pending governance action with 48-hour timelock.

```rust
fn council_sign_veto(env: Env, action_id: u64, signer: Address) -> Result<(), Error>
```

Security Council member signs to veto an action.

```rust
fn execute_timelocked_action(env: Env, action_id: u64) -> Result<(), Error>
```

Execute action after timelock if not vetoed.

### Protected Operations

```rust
fn protected_clawback(env: Env, grant_id: u64, initiator: Address) -> Result<u64, Error>
```

Initiate a clawback with Security Council oversight.

```rust
fn execute_protected_clawback(env: Env, action_id: u64, grant_id: u64) -> Result<(), Error>
```

Execute clawback after timelock expires (if not vetoed).

### Council Management

```rust
fn propose_council_rotation(
    env: Env,
    new_members: Vec<Address>,
    dao_admin: Address
) -> Result<(), Error>
```

Propose new council members with 7-day timelock.

```rust
fn execute_council_rotation(env: Env) -> Result<(), Error>
```

Execute council rotation after timelock.

```rust
fn is_council_rotation_due(env: Env) -> bool
```

Check if annual rotation is due.

### Query Functions

```rust
fn get_council_members(env: Env) -> Result<Vec<Address>, Error>
fn get_pending_action(env: Env, action_id: u64) -> Result<PendingAction, Error>
fn get_veto_signature_count(env: Env, action_id: u64) -> u32
fn get_all_pending_actions(env: Env) -> Vec<u64>
fn can_execute_timelocked_action(env: Env, action_id: u64) -> Result<bool, Error>
```

## Testing

Comprehensive test suite covers:

1. **Initialization**: Council setup with correct member count
2. **Veto Threshold**: 3-of-5 signature requirement
3. **Timelock Enforcement**: Cannot execute before 48 hours
4. **Veto Blocking**: Vetoed actions cannot execute
5. **Rogue DAO Scenarios**: Malicious clawback attempts
6. **Multiple Attacks**: Simultaneous malicious actions
7. **Council Rotation**: Annual key rotation process
8. **Legitimate Actions**: Non-vetoed actions proceed normally
9. **Access Control**: Only council members can veto
10. **Double-Signing Prevention**: Members cannot sign twice

Run tests:
```bash
cargo test test_security_council
```

## Acceptance Criteria

✅ **Acceptance 1**: Protocol treasury is protected from governance attacks and social engineering

✅ **Acceptance 2**: Checks and balances ensure neither token holders nor council have absolute power

✅ **Acceptance 3**: High-value donor capital is structurally shielded from sudden, malicious state changes

## Operational Guidelines

### For DAO/Admin

1. Always use `protected_clawback` instead of direct `cancel_grant` for sensitive operations
2. Communicate planned actions to community before initiating
3. Provide clear justification for all timelocked actions
4. Monitor council rotation schedule

### For Security Council

1. Review all pending actions within 48-hour window
2. Investigate suspicious patterns or unusual actions
3. Coordinate with other council members on veto decisions
4. Participate in annual key rotation
5. Maintain operational security of council keys

### For Community

1. Monitor `ActionPending` events for new timelocked actions
2. Alert Security Council of suspicious activity
3. Participate in governance discussions during timelock periods
4. Review council rotation proposals during 7-day windows

## Threat Model

### Threats Mitigated

✅ Governance token hijacking  
✅ Flash loan governance attacks  
✅ Social engineering of token holders  
✅ Compromised admin keys  
✅ Malicious treasury draining  
✅ Unauthorized grant clawbacks  

### Residual Risks

⚠️ **Council Collusion**: If 3+ council members collude, they could block legitimate actions
- **Mitigation**: DAO can rotate council with 7-day timelock

⚠️ **Council Unavailability**: If council doesn't respond, malicious actions could execute
- **Mitigation**: 48-hour window provides time for at least 3 members to respond

⚠️ **Slow Response**: 48-hour delay could slow emergency responses
- **Mitigation**: Legitimate emergencies can be communicated to council for fast-track approval

## Conclusion

The Security Council provides a critical defense layer against governance attacks while maintaining decentralization through checks and balances. The 3-of-5 multi-sig with 48-hour timelock creates a practical security boundary that protects high-value donor capital without centralizing control.

**Labels**: security, governance, critical
