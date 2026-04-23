# Stellar Metadata Monitoring System

## Overview

The Stellar Metadata Monitor ensures that Grant-Stream contracts stay synchronized with Stellar asset metadata changes. When a DAO rebrands or changes its token's metadata (e.g., ticker symbol), this system detects the change and updates the contract state, ensuring accurate user experience throughout long-term (4-year) grant cycles.

## Components

### 1. Smart Contract (`StellarMetadataMonitor.sol`)

**Key Features:**
- Register Stellar assets for monitoring
- Track metadata changes via change requests
- Emit `MetadataUpdate` events for dashboard/backend integration
- Owner-managed approval process for metadata changes

**Main Functions:**
- `registerAsset()` - Register a new Stellar asset
- `reportMetadataChange()` - Report detected metadata change
- `processMetadataChange()` - Process and approve a change request
- `updateMetadataDirect()` - Emergency direct metadata update
- `getAssetMetadata()` - View current asset metadata

**Events:**
- `MetadataUpdate` - Emitted when metadata changes (triggers dashboard update)
- `MetadataChangeRequested` - New change request submitted
- `MetadataChangeProcessed` - Change request approved and processed

### 2. Monitoring Worker (`stellar-metadata-worker.js`)

**Purpose:** Off-chain service that monitors Stellar network for metadata changes

**Features:**
- Polls Stellar Horizon API at configurable intervals
- Compares on-chain metadata with current Stellar state
- Automatically reports changes to smart contract
- Emits detailed logs for debugging

**Configuration:**
```bash
STELLAR_RPC_URL=https://horizon.stellar.org
ETHEREUM_RPC_URL=http://localhost:8545
CONTRACT_ADDRESS=0x...
PRIVATE_KEY=0x...
MONITOR_INTERVAL_MS=300000  # 5 minutes
ASSETS_TO_MONITOR=[{"assetCode":"USD","issuer":"GDUKMGUGDZQK6YHYA5Z6AY2G4XDSZPSZ3SW5UN3ARVMO6QSRDWP5YLEX","name":"US Dollar","domain":"centre.io"}]
```

## Usage

### Deploying the Contract

```bash
# Deploy using Foundry
forge script script/DeployStellarMonitor.s.sol --rpc-url <RPC_URL> --broadcast
```

### Running the Worker

```bash
# Install dependencies
npm install stellar-sdk ethers

# Set environment variables
export STELLAR_RPC_URL=https://horizon.stellar.org
export ETHEREUM_RPC_URL=http://localhost:8545
export CONTRACT_ADDRESS=0xYourContractAddress
export PRIVATE_KEY=0xYourPrivateKey
export MONITOR_INTERVAL_MS=300000

# Run the worker
node scripts/stellar-metadata-worker.js
```

### Example Workflow

#### 1. Register Asset for Monitoring

```solidity
// Transaction: Register Stellar asset
contract.registerAsset(
    "USD",                              // Asset code
    "GDUKMGUGDZQK6YHYA5Z6AY2G4XDSZPSZ3SW5UN3ARVMO6QSRDWP5YLEX",  // Issuer
    "US Dollar",                        // Name
    "centre.io"                         // Domain
);
```

#### 2. Worker Detects Rebrand

When DAO rebrands from "USD" to "USDC":
- Worker polls Stellar Horizon API
- Detects asset code changed from "USD" to "USDC"
- Calls `reportMetadataChange()` on contract
- Creates change request #1

#### 3. Process Change Request

```solidity
// Owner reviews and processes change
contract.processMetadataChange(1);  // Change request ID

// This emits MetadataUpdate event
// Backend listens for this event to update dashboard
```

#### 4. Dashboard Updates

Backend services listen for `MetadataUpdate` events:
```javascript
contract.on('MetadataUpdate', (stellarAssetId, oldCode, newCode, oldName, newName, timestamp) => {
    console.log(`Asset rebranded: ${oldCode} -> ${newCode}`);
    console.log(`Name changed: ${oldName} -> ${newName}`);
    
    // Update database/cache
    updateAssetMetadata(stellarAssetId, {
        assetCode: newCode,
        name: newName
    });
    
    // Refresh UI
    refreshDashboard();
});
```

## Architecture

```
┌─────────────────┐
│  Stellar Network │
│  (Horizon API)   │
└────────┬─────────┘
         │
         │ Poll every 5 min
         ▼
┌─────────────────┐      ┌──────────────────┐
│  Metadata       │─────▶│  Smart Contract  │
│  Worker         │      │  (Ethereum)      │
│  (Node.js)      │◀─────│                  │
└─────────────────┘      └────────┬──────────┘
                                  │
                                  │ Events
                                  ▼
                         ┌──────────────────┐
                         │  Dashboard/UI    │
                         │  Backend Service │
                         └──────────────────┘
```

## Benefits

1. **Professional UX**: Users always see correct token information
2. **Prevents Confusion**: No mismatched tickers during 4-year grants
3. **Automated**: No manual intervention required
4. **Auditable**: All changes tracked on-chain
5. **Flexible**: Owner can approve/reject changes

## Security Considerations

- Only owner can process metadata changes
- Change requests create audit trail
- Emergency direct update function for critical fixes
- Worker requires secure private key management

## Production Deployment

For production use:

1. **Run Multiple Workers**: Deploy workers in multiple regions for redundancy
2. **Use WebSockets**: Subscribe to Stellar ledger events instead of polling
3. **Add Alerts**: Notify team of metadata changes via email/Slack
4. **Rate Limiting**: Implement rate limiting on change requests
5. **Monitoring**: Add health checks and metrics for the worker

## Testing

```bash
# Run tests
npx hardhat test test/StellarMetadataMonitor.test.js

# Test on local fork
anvil --fork-url https://eth-mainnet.g.alchemy.com/v2/demo
node scripts/stellar-metadata-worker.js
```

## Future Enhancements

- Chainlink oracle integration for decentralized metadata verification
- Automatic approval for trusted issuers
- Governance-based voting on metadata changes
- Support for multiple blockchain networks beyond Stellar

## Support

For issues or questions:
- GitHub Issues: https://github.com/lifewithbigdamz/Grant-Stream-Contracts/issues
- Documentation: See main README.md

---

**Built for institutional-grade grant management across multi-year cycles.**
