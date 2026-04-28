# Grant-Stream `is_active_grantee` Adapter Documentation

## Overview

The `is_active_grantee(address)` function is a standardized, zero-gas read-only contract adapter that allows partner protocols (e.g., Soroswap, Lending Pools) to verify if a user is an "Approved Grantee" with active, uncompleted grants on the Grant-Stream protocol.

## Purpose

This adapter serves as a "Reputation-as-a-Service" gateway, enabling:
- **Builder Discounts**: Special pricing for active grant recipients
- **Premium Access**: Exclusive features for grantee ecosystem members
- **Cross-Protocol Integration**: Seamless verification across Stellar ecosystem

## Function Signature

```rust
pub fn is_active_grantee(env: Env, address: Address) -> bool
```

### Parameters
- `env`: Soroban environment context
- `address`: The Stellar address to check for active grantee status

### Returns
- `bool`: `true` if the user has at least one active, uncompleted grant, `false` otherwise

## Active Grant Criteria

A user is considered an "active grantee" if they have at least one grant that meets **ALL** of the following criteria:

1. **Status**: `Active` or `Paused`
   - ✅ `Active`: Grant is currently streaming
   - ✅ `Paused`: Grant is temporarily suspended (still considered active)
   - ❌ `Completed`: Grant has finished streaming
   - ❌ `Cancelled`: Grant was cancelled
   - ❌ `RageQuitted`: Grantee rage quit

2. **Funding**: Grant has remaining funds to be streamed
   - Total withdrawn < total amount, OR
   - There are claimable funds available

3. **Data Freshness**: Grant data exists and is not archived/purged

## Security Considerations

### ✅ What's Exposed
- Simple boolean response (active/inactive status only)
- No sensitive grant amounts or IDs
- No personal grant details

### ❌ What's NOT Exposed
- Grant amounts or values
- Grant IDs or timestamps
- Grant terms or conditions
- Withdrawal history
- Validator information

## Performance Requirements

The function is optimized for high-frequency cross-contract queries:

- **CPU Limit**: < 5,000 Soroban CPU instructions per call
- **Gas Cost**: Zero-gas for external contracts (read-only)
- **Storage Access**: Minimal, targeted lookups
- **Execution Time**: Sub-millisecond on typical hardware

## Usage Examples

### 1. Soroswap Integration

```rust
// In Soroswap contract
use soroban_sdk::{Address, Env};

fn apply_builder_discount(env: &Env, user: Address, base_fee: i128) -> i128 {
    let grant_stream_contract = Address::from_string(&env, "GRANT_STREAM_CONTRACT_ID");
    
    // Check if user is active grantee
    let is_grantee = env.invoke_contract::<bool>(
        &grant_stream_contract,
        &Symbol::new(&env, "is_active_grantee"),
        (user,)
    );
    
    if is_grantee {
        // Apply 25% builder discount
        base_fee * 75 / 100
    } else {
        base_fee
    }
}
```

### 2. Lending Protocol Integration

```rust
// In lending protocol contract
fn calculate_borrowing_rate(env: &Env, user: Address, base_rate: i128) -> i128 {
    let grant_stream_contract = Address::from_string(&env, "GRANT_STREAM_CONTRACT_ID");
    
    let is_grantee = env.invoke_contract::<bool>(
        &grant_stream_contract,
        &Symbol::new(&env, "is_active_grantee"),
        (user,)
    );
    
    if is_grantee {
        // Reduced borrowing rate for active grantees
        base_rate * 80 / 100  // 20% discount
    } else {
        base_rate
    }
}
```

### 3. Access Control Integration

```rust
// In DeFi protocol with premium features
fn access_premium_features(env: &Env, user: Address) -> Result<(), Error> {
    let grant_stream_contract = Address::from_string(&env, "GRANT_STREAM_CONTRACT_ID");
    
    let is_grantee = env.invoke_contract::<bool>(
        &grant_stream_contract,
        &Symbol::new(&env, "is_active_grantee"),
        (user,)
    );
    
    if is_grantee {
        // Grant access to premium features
        Ok(())
    } else {
        Err(Error::AccessDenied)
    }
}
```

## Edge Cases

### 1. Archived/Stale Records
- Grants that have been purged from storage return `false`
- Completed/cancelled grants return `false`
- This ensures only currently active grantees are verified

### 2. Multiple Grants
- If a user has multiple grants, returns `true` if ANY grant is active
- No performance impact from grant count

### 3. Zero-Amount Grants
- Grants with zero total amount are not considered active
- Prevents false positives from test/placeholder grants

## Implementation Details

### Storage Access Pattern
1. Lookup `RecipientGrants(address)` to get user's grant IDs
2. Iterate through grants (early exit on first active grant found)
3. For each grant:
   - Check status (`Active` or `Paused`)
   - Verify remaining funds exist
   - Return `true` immediately if active grant found

### Optimization Techniques
- **Early Exit**: Stop searching after finding first active grant
- **Minimal Storage**: Only access necessary grant data
- **Efficient Comparison**: Direct status enum comparison
- **No Events**: Zero event emission to reduce gas cost

## Testing

The implementation includes comprehensive tests covering:

- ✅ Basic functionality (active/inactive users)
- ✅ Different grant statuses
- ✅ Edge cases (archived data, zero amounts)
- ✅ Performance benchmarks (< 5,000 CPU instructions)
- ✅ Multiple grants per user
- ✅ Security (no data leakage)

## Deployment

### Contract Address
The deployed contract address will be provided in the Grant-Stream documentation and can be accessed via:

```rust
let grant_stream_contract = Address::from_string(&env, "DEPLOYED_CONTRACT_ID");
```

### Version Compatibility
- Compatible with Soroban SDK v20+
- No breaking changes planned
- Backward compatible with existing integrations

## Support

For integration support or questions:
- GitHub Issues: [Grant-Stream Contracts](https://github.com/frankosakwe/Grant-Stream-Contracts)
- Documentation: [Grant-Stream Docs](https://docs.grant-stream.io)
- Community: [Stellar Ecosystem Discord](https://discord.gg/stellar)

## Acceptance Criteria

✅ **Acceptance 1**: Grant-Stream successfully acts as an "Identity/Status Provider" for the builder ecosystem
✅ **Acceptance 2**: Grant recipients can leverage their status to access premium features on external platforms  
✅ **Acceptance 3**: The adapter is optimized for high-frequency cross-contract queries without gas overhead

---

*This adapter is designed to be a foundational building block for the Stellar ecosystem, enabling seamless integration between Grant-Stream and partner protocols while maintaining security and performance standards.*
