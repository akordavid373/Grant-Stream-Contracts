#!/usr/bin/env node

/**
 * Stellar Metadata Monitoring Worker
 * ===================================
 * This worker monitors Stellar network for asset metadata changes.
 * When a change is detected, it reports to the smart contract.
 * 
 * Usage: node scripts/stellar-metadata-worker.js
 * 
 * Note: This is a conceptual implementation. Production deployment requires:
 * - Stellar RPC endpoint (e.g., from Stellar.org or QuickNode)
 * - Web3 provider for Ethereum network
 * - Proper error handling and retry logic
 */

const { Horizon, Networks, Asset } = require('stellar-sdk');
const { ethers } = require('ethers');

// Configuration
const STELLAR_RPC_URL = process.env.STELLAR_RPC_URL || 'https://horizon.stellar.org';
const ETHEREUM_RPC_URL = process.env.ETHEREUM_RPC_URL || 'http://localhost:8545';
const CONTRACT_ADDRESS = process.env.CONTRACT_ADDRESS;
const MONITOR_INTERVAL_MS = parseInt(process.env.MONITOR_INTERVAL_MS || '300000', 10); // 5 minutes
const ASSETS_TO_MONITOR = JSON.parse(process.env.ASSETS_TO_MONITOR || '[]');

// Contract ABI (minimal for metadata monitoring)
const CONTRACT_ABI = [
    "function reportMetadataChange(bytes32 _stellarAssetId, string memory _newAssetCode, string memory _newName) external returns (uint256)",
    "function getAssetMetadata(bytes32 _stellarAssetId) external view returns (tuple(string assetCode, string issuer, string name, string domain, uint256 lastUpdateTime, bool exists))",
    "event MetadataUpdate(bytes32 indexed stellarAssetId, string oldAssetCode, string newAssetCode, string oldName, string newName, uint256 updateTimestamp)"
];

class StellarMetadataMonitor {
    constructor() {
        this.horizon = new Horizon.Server(STELLAR_RPC_URL);
        this.provider = new ethers.JsonRpcProvider(ETHEREUM_RPC_URL);
        this.contract = null;
        this.wallet = null;
        this.trackedAssets = new Map();
    }

    /**
     * Initialize the worker
     */
    async initialize() {
        console.log('🚀 Initializing Stellar Metadata Monitor...');
        
        // Setup wallet and contract
        if (process.env.PRIVATE_KEY && CONTRACT_ADDRESS) {
            this.wallet = new ethers.Wallet(process.env.PRIVATE_KEY, this.provider);
            this.contract = new ethers.Contract(CONTRACT_ADDRESS, CONTRACT_ABI, this.wallet);
            console.log('✓ Contract connected:', CONTRACT_ADDRESS);
        } else {
            console.warn('⚠️  No PRIVATE_KEY or CONTRACT_ADDRESS set. Running in read-only mode.');
        }

        // Load assets to monitor
        await this.loadAssetsToMonitor();
        
        console.log('✓ Initialization complete');
        console.log(`📊 Monitoring ${this.trackedAssets.size} assets`);
    }

    /**
     * Load assets from environment or contract
     */
    async loadAssetsToMonitor() {
        if (ASSETS_TO_MONITOR.length > 0) {
            ASSETS_TO_MONITOR.forEach(asset => {
                this.trackedAssets.set(asset.assetCode, {
                    assetCode: asset.assetCode,
                    issuer: asset.issuer,
                    name: asset.name,
                    domain: asset.domain,
                    stellarAssetId: this.generateStellarAssetId(asset.assetCode, asset.issuer)
                });
            });
        } else if (this.contract) {
            // Load from contract if available
            try {
                const trackedAssets = await this.contract.getAllTrackedAssets();
                for (const assetId of trackedAssets) {
                    const metadata = await this.contract.getAssetMetadata(assetId);
                    if (metadata.exists) {
                        this.trackedAssets.set(metadata.assetCode, {
                            ...metadata,
                            stellarAssetId: assetId
                        });
                    }
                }
            } catch (error) {
                console.error('Error loading assets from contract:', error.message);
            }
        }
    }

    /**
     * Generate Stellar asset ID (matches contract's hashing method)
     */
    generateStellarAssetId(assetCode, issuer) {
        const ethers = require('ethers');
        return ethers.keccak256(
            ethers.solidityPacked(['string', 'string'], [assetCode, issuer])
        );
    }

    /**
     * Fetch current metadata from Stellar network
     */
    async fetchStellarMetadata(assetCode, issuer) {
        try {
            // Fetch asset info from Stellar
            const response = await this.horizon.assets()
                .forCode(assetCode)
                .call();

            const assetRecord = response.records.find(r => r.issuer === issuer);
            
            if (!assetRecord) {
                console.warn(`Asset ${assetCode}:${issuer} not found on Stellar`);
                return null;
            }

            // Extract metadata
            return {
                assetCode: assetRecord.asset_code,
                issuer: assetRecord.issuer,
                name: assetRecord.name || assetRecord.asset_code,
                domain: assetRecord.anchor_asset_domain || '',
                lastModified: assetRecord.last_modified_ledger
            };
        } catch (error) {
            console.error(`Error fetching metadata for ${assetCode}:${issuer}:`, error.message);
            return null;
        }
    }

    /**
     * Check for metadata changes
     */
    async checkForChanges() {
        console.log('\n🔍 Checking for metadata changes...');
        
        for (const [assetCode, storedAsset] of this.trackedAssets) {
            try {
                const currentMetadata = await this.fetchStellarMetadata(
                    storedAsset.assetCode,
                    storedAsset.issuer
                );

                if (!currentMetadata) {
                    continue;
                }

                // Check if metadata changed
                const codeChanged = currentMetadata.assetCode !== storedAsset.assetCode;
                const nameChanged = currentMetadata.name !== storedAsset.name;

                if (codeChanged || nameChanged) {
                    console.log(`⚠️  Metadata change detected for ${assetCode}`);
                    console.log(`   Old: ${storedAsset.assetCode} / ${storedAsset.name}`);
                    console.log(`   New: ${currentMetadata.assetCode} / ${currentMetadata.name}`);

                    // Report change to contract
                    await this.reportChange(storedAsset.stellarAssetId, currentMetadata);
                    
                    // Update local cache
                    this.trackedAssets.set(assetCode, {
                        ...storedAsset,
                        assetCode: currentMetadata.assetCode,
                        name: currentMetadata.name
                    });
                } else {
                    console.log(`✓ No change for ${assetCode}`);
                }
            } catch (error) {
                console.error(`Error checking ${assetCode}:`, error.message);
            }
        }
    }

    /**
     * Report metadata change to smart contract
     */
    async reportChange(stellarAssetId, newMetadata) {
        if (!this.contract) {
            console.log('📝 Would report change (no contract connection):', {
                stellarAssetId,
                newAssetCode: newMetadata.assetCode,
                newName: newMetadata.name
            });
            return;
        }

        try {
            console.log('📝 Reporting metadata change to contract...');
            
            const tx = await this.contract.reportMetadataChange(
                stellarAssetId,
                newMetadata.assetCode,
                newMetadata.name
            );

            console.log('⏳ Waiting for transaction confirmation...');
            const receipt = await tx.wait();
            
            console.log('✅ Change reported successfully!');
            console.log('   TX Hash:', receipt.hash);
            
            // Parse events
            const metadataUpdateEvent = receipt.logs.find(log => {
                try {
                    const parsed = this.contract.interface.parseLog(log);
                    return parsed && parsed.name === 'MetadataUpdate';
                } catch {
                    return false;
                }
            });

            if (metadataUpdateEvent) {
                const parsed = this.contract.interface.parseLog(metadataUpdateEvent);
                console.log('   📢 MetadataUpdate event emitted');
                console.log('      Old Code:', parsed.args.oldAssetCode);
                console.log('      New Code:', parsed.args.newAssetCode);
            }
        } catch (error) {
            console.error('❌ Error reporting change:', error.message);
            if (error.reason) {
                console.error('   Reason:', error.reason);
            }
        }
    }

    /**
     * Start monitoring loop
     */
    startMonitoring() {
        console.log(`\n🔄 Starting monitoring loop (interval: ${MONITOR_INTERVAL_MS / 1000}s)`);
        
        setInterval(async () => {
            await this.checkForChanges();
        }, MONITOR_INTERVAL_MS);

        // Initial check
        this.checkForChanges();
    }

    /**
     * Run the worker
     */
    async run() {
        try {
            await this.initialize();
            this.startMonitoring();
            
            console.log('\n✅ Stellar Metadata Monitor is running...');
            console.log('Press Ctrl+C to stop\n');
        } catch (error) {
            console.error('❌ Failed to start worker:', error.message);
            process.exit(1);
        }
    }
}

// Main execution
if (require.main === module) {
    const monitor = new StellarMetadataMonitor();
    monitor.run();
}

module.exports = StellarMetadataMonitor;
