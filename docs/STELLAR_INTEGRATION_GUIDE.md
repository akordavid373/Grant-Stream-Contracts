# SEP-12 & SEP-10 Integration Technical Guide

## Overview

This guide explains how Grant-Stream integrates with Stellar's native identity and authentication standards, enabling seamless wallet integration and ecosystem partnership opportunities.

## Stellar Ecosystem Standards

### SEP-10: Stellar Authentication
SEP-10 provides a standardized way for Stellar accounts to authenticate with web services using cryptographic signatures.

### SEP-12: Stellar Account Verification
SEP-12 defines a standard for KYC/AML verification processes and status sharing across the Stellar ecosystem.

## Grant-Stream Integration Architecture

### Authentication Flow (SEP-10)

#### 1. Challenge Transaction Generation
```rust
// Grant-Stream generates a SEP-10 challenge
pub fn generate_challenge(env: &Env, client_domain: &str, account: &Address) -> Result<Transaction, Error> {
    let challenge = env
        .stellar()
        .build_challenge_transaction(
            &env.current_contract_address(),
            account,
            client_domain,
            300, // 5 minute validity
        )?;
    
    Ok(challenge)
}
```

#### 2. Challenge Signing by Wallet
Wallets should:
- Parse the challenge transaction
- Verify the server's signature
- Sign with the user's private key
- Return the signed transaction

#### 3. Challenge Verification
```rust
pub fn verify_challenge(env: &Env, signed_challenge: &Transaction) -> Result<Address, Error> {
    let signer = env
        .stellar()
        .verify_challenge_transaction(
            signed_challenge,
            &env.current_contract_address(),
        )?;
    
    Ok(signer)
}
```

### Identity Verification (SEP-12)

#### 1. KYC Status Storage
```rust
#[contracttype]
pub struct KYCStatus {
    pub account: Address,
    pub status: VerificationStatus,
    pub verified_at: u64,
    pub expires_at: u64,
    pub provider: Address, // KYC provider address
}

#[contracttype]
pub enum VerificationStatus {
    Pending,
    Verified,
    Rejected,
    Expired,
}
```

#### 2. Status Query Interface
```rust
pub fn get_kyc_status(env: Env, account: Address) -> Result<KYCStatus, Error> {
    let kyc_key = DataKey::KYCStatus(account);
    env.storage()
        .instance()
        .get(&kyc_key)
        .ok_or(Error::KYCNotFound)
}
```

## Wallet Integration Guide

### Required SEP-10 Implementation

#### 1. Challenge Request
```javascript
// Frontend: Request challenge from Grant-Stream
async function requestChallenge(userPublicKey, clientDomain) {
    const response = await fetch('https://grant-stream-api.example.com/challenge', {
        method: 'POST',
        headers: {
            'Content-Type': 'application/json',
        },
        body: JSON.stringify({
            account: userPublicKey,
            client_domain: clientDomain,
        }),
    });
    
    return response.json(); // Contains challenge transaction
}
```

#### 2. Challenge Signing
```javascript
// Wallet: Sign the challenge transaction
async function signChallenge(challengeTransaction, userKeypair) {
    const transaction = new Transaction(challengeTransaction, Networks.PUBLIC);
    transaction.sign(userKeypair);
    
    return transaction.toXDR(); // Return signed XDR
}
```

#### 3. Authentication Token
```javascript
// Frontend: Submit signed challenge for authentication
async function authenticate(signedChallengeXDR) {
    const response = await fetch('https://grant-stream-api.example.com/auth', {
        method: 'POST',
        headers: {
            'Content-Type': 'application/json',
        },
        body: JSON.stringify({
            signed_challenge: signedChallengeXDR,
        }),
    });
    
    return response.json(); // Contains JWT token
}
```

### SEP-12 Integration for KYC Providers

#### 1. KYC Status Submission
```javascript
// KYC Provider: Submit verification status
async function submitKYCStatus(accountAddress, status, providerSignature) {
    const response = await fetch('https://grant-stream-api.example.com/kyc/submit', {
        method: 'POST',
        headers: {
            'Content-Type': 'application/json',
            'Authorization': `Bearer ${providerToken}`,
        },
        body: JSON.stringify({
            account: accountAddress,
            status: status, // 'VERIFIED', 'REJECTED', etc.
            provider_signature: providerSignature,
            expires_at: new Date(Date.now() + 365 * 24 * 60 * 60 * 1000).toISOString(), // 1 year
        }),
    });
    
    return response.json();
}
```

#### 2. Status Verification
```javascript
// Grant-Stream: Verify KYC status before grant operations
async function verifyKYCStatus(accountAddress) {
    const response = await fetch(`https://grant-stream-api.example.com/kyc/${accountAddress}`);
    const kycStatus = await response.json();
    
    // Check if status is valid and not expired
    if (kycStatus.status === 'VERIFIED' && 
        new Date(kycStatus.expires_at) > new Date()) {
        return true;
    }
    
    return false;
}
```

## Smart Contract Integration

### Grant Creation with KYC Requirements
```rust
pub fn create_grant_with_kyc(
    env: Env,
    grant_id: u64,
    recipient: Address,
    total_amount: i128,
    flow_rate: i128,
    warmup_duration: u64,
    validator: Option<Address>,
    require_kyc: bool, // New parameter for SEP-12 integration
) -> Result<(), Error> {
    require_admin_auth(&env)?;
    
    // Verify KYC status if required
    if require_kyc {
        let kyc_status = get_kyc_status(&env, recipient.clone())?;
        match kyc_status.status {
            VerificationStatus::Verified => {
                if kyc_status.expires_at < env.ledger().timestamp() {
                    return Err(Error::KYCExpired);
                }
            }
            _ => return Err(Error::KYCRequired),
        }
    }
    
    // ... rest of grant creation logic
}
```

### Withdrawal with KYC Validation
```rust
pub fn withdraw_with_kyc(env: Env, grant_id: u64, amount: i128) -> Result<(), Error> {
    let mut grant = read_grant(&env, grant_id)?;
    
    // SEP-12: Verify KYC status before withdrawal
    let kyc_status = get_kyc_status(&env, grant.recipient.clone())?;
    match kyc_status.status {
        VerificationStatus::Verified => {
            if kyc_status.expires_at < env.ledger().timestamp() {
                return Err(Error::KYCExpired);
            }
        }
        _ => return Err(Error::KYCRequired),
    }
    
    // ... rest of withdrawal logic
}
```

## Ecosystem Partner Integration

### DAO Integration Example

#### 1. DAO Treasury Integration
```javascript
// DAO: Grant-Stream integration for treasury management
class GrantStreamDAOIntegration {
    constructor(daoContractAddress, grantStreamContract) {
        this.daoContract = daoContractAddress;
        this.grantStream = grantStreamContract;
    }
    
    async createGrantFromDAO(proposal, grantDetails) {
        // Authenticate using SEP-10
        const auth = await this.authenticateDAOTreasury();
        
        // Create grant through Grant-Stream
        const grant = await this.grantStream.create_grant({
            recipient: grantDetails.recipient,
            amount: grantDetails.amount,
            flow_rate: grantDetails.flow_rate,
            require_kyc: true, // DAO requires KYC verification
            auth_token: auth.token,
        });
        
        // Record in DAO ledger
        await this.daoContract.recordGrantCreation(proposal.id, grant.id);
        
        return grant;
    }
    
    async authenticateDAOTreasury() {
        // Multi-sig authentication for DAO treasury
        const challenge = await this.requestChallenge(this.daoTreasuryAddress);
        const signatures = await this.collectMultiSigSignatures(challenge);
        
        return this.submitMultiSigAuth(challenge, signatures);
    }
}
```

#### 2. Wallet Provider Integration
```javascript
// Wallet Provider: Native Grant-Stream support
class WalletGrantStreamSupport {
    constructor(walletInstance) {
        this.wallet = walletInstance;
    }
    
    async connectToGrantStream() {
        // SEP-10 authentication
        const publicKey = this.wallet.getPublicKey();
        const challenge = await this.requestChallenge(publicKey);
        const signedChallenge = await this.wallet.signTransaction(challenge);
        const authToken = await this.authenticate(signedChallenge);
        
        // Check KYC status
        const kycStatus = await this.getKYCStatus(publicKey);
        
        return {
            authenticated: true,
            token: authToken,
            kyc_verified: kycStatus.status === 'VERIFIED',
        };
    }
    
    async createGrantWithWallet(grantDetails) {
        const connection = await this.connectToGrantStream();
        
        if (!connection.kyc_verified) {
            throw new Error('KYC verification required for grant creation');
        }
        
        return this.wallet.submitTransaction({
            contract: 'grant-stream',
            method: 'create_grant_with_kyc',
            params: {
                ...grantDetails,
                require_kyc: true,
            },
            auth: connection.token,
        });
    }
}
```

### Exchange Integration

#### 1. Exchange Grant Distribution
```javascript
// Exchange: Automated grant distribution via Grant-Stream
class ExchangeGrantDistribution {
    constructor(exchangeAPI, grantStreamClient) {
        this.exchange = exchangeAPI;
        this.grantStream = grantStreamClient;
    }
    
    async distributeGrantsToUsers(grantList) {
        const results = [];
        
        for (const grant of grantList) {
            try {
                // Verify user KYC status
                const kycStatus = await this.exchange.getUserKYC(grant.userId);
                if (kycStatus.status !== 'VERIFIED') {
                    results.push({ userId: grant.userId, status: 'KYC_REQUIRED' });
                    continue;
                }
                
                // Create grant through Grant-Stream
                const grantResult = await this.grantStream.create_grant({
                    recipient: grant.stellarAddress,
                    amount: grant.amount,
                    flow_rate: grant.flowRate,
                    require_kyc: true,
                });
                
                results.push({ 
                    userId: grant.userId, 
                    status: 'SUCCESS', 
                    grantId: grantResult.id 
                });
                
            } catch (error) {
                results.push({ 
                    userId: grant.userId, 
                    status: 'ERROR', 
                    error: error.message 
                });
            }
        }
        
        return results;
    }
}
```

## Security Considerations

### SEP-10 Security
1. **Challenge Uniqueness**: Each challenge must be unique and non-reusable
2. **Time Validity**: Challenges expire after 5 minutes
3. **Domain Verification**: Server domain must be verified
4. **Signature Validation**: All signatures must be cryptographically valid

### SEP-12 Security
1. **Provider Authentication**: KYC providers must be authenticated
2. **Status Integrity**: KYC status must be tamper-proof
3. **Expiration Management**: Automatic expiration of old KYC data
4. **Privacy Protection**: Sensitive KYC data should be encrypted

### Best Practices
1. **Multi-Factor Authentication**: Combine SEP-10 with additional factors
2. **Rate Limiting**: Prevent brute force attacks on authentication
3. **Audit Logging**: Log all authentication and KYC operations
4. **Regular Rotation**: Rotate authentication keys regularly

## API Endpoints

### Authentication (SEP-10)
```
POST /api/v1/challenge
POST /api/v1/auth
GET  /api/v1/auth/status
```

### KYC Operations (SEP-12)
```
POST /api/v1/kyc/submit
GET  /api/v1/kyc/{account}
PUT  /api/v1/kyc/{account}/status
GET  /api/v1/kyc/providers
```

### Grant Operations
```
POST /api/v1/grants/create
GET  /api/v1/grants/{id}
POST /api/v1/grants/{id}/withdraw
GET  /api/v1/grants/{id}/status
```

## Testing Integration

### SEP-10 Test Suite
```javascript
describe('SEP-10 Integration', () => {
    test('should generate valid challenge', async () => {
        const challenge = await requestChallenge(testPublicKey, testDomain);
        expect(challenge).toBeDefined();
        expect(challenge.transaction).toBeDefined();
    });
    
    test('should verify signed challenge', async () => {
        const signedChallenge = await signChallenge(challenge, testKeypair);
        const authResult = await authenticate(signedChallenge);
        expect(authResult.token).toBeDefined();
    });
    
    test('should reject expired challenges', async () => {
        const expiredChallenge = generateExpiredChallenge();
        const signedChallenge = await signChallenge(expiredChallenge, testKeypair);
        
        await expect(authenticate(signedChallenge))
            .rejects.toThrow('Challenge expired');
    });
});
```

### SEP-12 Test Suite
```javascript
describe('SEP-12 Integration', () => {
    test('should submit KYC status', async () => {
        const result = await submitKYCStatus(testAccount, 'VERIFIED', providerSignature);
        expect(result.success).toBe(true);
    });
    
    test('should retrieve KYC status', async () => {
        const status = await getKYCStatus(testAccount);
        expect(status.status).toBe('VERIFIED');
        expect(status.expires_at).toBeDefined();
    });
    
    test('should reject invalid KYC provider', async () => {
        await expect(submitKYCStatus(testAccount, 'VERIFIED', invalidSignature))
            .rejects.toThrow('Invalid provider');
    });
});
```

## Migration Guide

### Existing Wallet Integration
1. **Update Authentication**: Implement SEP-10 challenge-response flow
2. **Add KYC Support**: Integrate with SEP-12 providers
3. **Update APIs**: Use new authentication endpoints
4. **Test Compatibility**: Ensure backward compatibility

### New Partner Onboarding
1. **Register KYC Provider**: Get approved as SEP-12 provider
2. **Implement Auth**: Add SEP-10 authentication
3. **Test Integration**: Run full test suite
4. **Go Live**: Deploy to production

## Support and Resources

### Documentation
- [Stellar SEP-10 Specification](https://github.com/stellar/stellar-protocol/blob/master/ecosystem/sep-0010.md)
- [Stellar SEP-12 Specification](https://github.com/stellar/stellar-protocol/blob/master/ecosystem/sep-0012.md)
- [Grant-Stream API Reference](https://docs.grant-stream.dev/api)

### SDKs and Libraries
- JavaScript SDK: `@grant-stream/js-sdk`
- Python SDK: `grant-stream-python`
- Rust SDK: `grant-stream-rust`

### Community Support
- Discord: [Grant-Stream Community](https://discord.gg/grant-stream)
- GitHub: [Grant-Stream Issues](https://github.com/Grant-Stream/contracts/issues)
- Email: support@grant-stream.dev

---

**Document Version:** 1.0  
**Last Updated:** 2026-04-28  
**Next Review:** 2026-07-28
