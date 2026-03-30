// SPDX-License-Identifier: MIT
pragma solidity ^0.8.24;

import "@openzeppelin/contracts/access/Ownable.sol";
import "@openzeppelin/contracts/utils/ReentrancyGuard.sol";

/**
 * @title StellarMetadataMonitor
 * @notice Monitors Stellar asset metadata changes and emits events for DAO token rebrands.
 * @dev If a DAO changes its token's metadata (e.g., ticker), this contract detects
 *      the change on Stellar and emits a MetadataUpdate event for backend/dashboard updates.
 * 
 * This ensures the User Experience remains accurate and professional during long-term
 * (4-year) grant cycles, preventing confusion if a DAO rebrands.
 */
contract StellarMetadataMonitor is Ownable, ReentrancyGuard {
    
    // ─── Structs ──────────────────────────────────────────────────────────────
    
    struct AssetMetadata {
        string assetCode;      // e.g., "USD", "BTC", custom ticker
        string issuer;         // Stellar issuer address
        string name;           // Full asset name
        string domain;         // Home domain
        uint256 lastUpdateTime;
        bool exists;
    }
    
    struct MetadataChangeRequest {
        uint256 id;
        bytes32 stellarAssetId;
        string oldAssetCode;
        string newAssetCode;
        string oldName;
        string newName;
        address requester;
        uint256 timestamp;
        bool processed;
        bool exists;
    }
    
    // ─── State ────────────────────────────────────────────────────────────────
    
    uint256 public nextChangeRequestId;
    
    // Mapping from Stellar asset ID (hash) to metadata
    mapping(bytes32 => AssetMetadata) public assetMetadata;
    
    // Mapping from change request ID to metadata change
    mapping(uint256 => MetadataChangeRequest) public metadataChanges;
    
    // Mapping from asset code to Stellar asset ID
    mapping(string => bytes32) public assetCodeToId;
    
    // Array of tracked asset IDs
    bytes32[] public trackedAssets;
    
    // ─── Events ───────────────────────────────────────────────────────────────
    
    event AssetRegistered(
        bytes32 indexed stellarAssetId,
        string assetCode,
        string issuer,
        string name
    );
    
    event MetadataUpdate(
        bytes32 indexed stellarAssetId,
        string oldAssetCode,
        string newAssetCode,
        string oldName,
        string newName,
        uint256 updateTimestamp
    );
    
    event MetadataChangeRequested(
        uint256 indexed changeRequestId,
        bytes32 indexed stellarAssetId,
        string oldAssetCode,
        string newAssetCode,
        address indexed requester
    );
    
    event MetadataChangeProcessed(
        uint256 indexed changeRequestId,
        bytes32 indexed stellarAssetId
    );
    
    event AssetUntracked(bytes32 indexed stellarAssetId);
    
    // ─── Modifiers ────────────────────────────────────────────────────────────
    
    modifier assetExists(bytes32 _stellarAssetId) {
        require(assetMetadata[_stellarAssetId].exists, "StellarMetadataMonitor: Asset does not exist");
        _;
    }
    
    modifier changeRequestExists(uint256 _changeRequestId) {
        require(metadataChanges[_changeRequestId].exists, "StellarMetadataMonitor: Change request does not exist");
        _;
    }
    
    // ─── Constructor ──────────────────────────────────────────────────────────
    
    constructor() Ownable(msg.sender) {
        nextChangeRequestId = 1;
    }
    
    // ─── External Functions ───────────────────────────────────────────────────
    
    /**
     * @notice Register a new Stellar asset for metadata monitoring.
     * @param _assetCode The asset code/ticker (e.g., "USD", "DAO").
     * @param _issuer The Stellar issuer address.
     * @param _name The full asset name.
     * @param _domain The home domain of the asset.
     * @return stellarAssetId The unique ID for this Stellar asset.
     */
    function registerAsset(
        string memory _assetCode,
        string memory _issuer,
        string memory _name,
        string memory _domain
    ) external onlyOwner returns (bytes32 stellarAssetId) {
        require(bytes(_assetCode).length > 0, "StellarMetadataMonitor: Empty asset code");
        require(bytes(_issuer).length > 0, "StellarMetadataMonitor: Empty issuer");
        
        // Generate unique asset ID from asset code and issuer
        stellarAssetId = keccak256(abi.encodePacked(_assetCode, _issuer));
        
        require(!assetMetadata[stellarAssetId].exists, "StellarMetadataMonitor: Asset already registered");
        
        assetMetadata[stellarAssetId] = AssetMetadata({
            assetCode: _assetCode,
            issuer: _issuer,
            name: _name,
            domain: _domain,
            lastUpdateTime: block.timestamp,
            exists: true
        });
        
        assetCodeToId[_assetCode] = stellarAssetId;
        trackedAssets.push(stellarAssetId);
        
        emit AssetRegistered(stellarAssetId, _assetCode, _issuer, _name);
    }
    
    /**
     * @notice Report a metadata change detected on Stellar network.
     * @dev Called by off-chain workers that monitor Stellar for metadata updates.
     * @param _stellarAssetId The Stellar asset ID.
     * @param _newAssetCode The new asset code/ticker.
     * @param _newName The new asset name.
     * @return changeRequestId ID of the created change request.
     */
    function reportMetadataChange(
        bytes32 _stellarAssetId,
        string memory _newAssetCode,
        string memory _newName
    ) external assetExists(_stellarAssetId) returns (uint256 changeRequestId) {
        AssetMetadata storage asset = assetMetadata[_stellarAssetId];
        
        // Only create change request if metadata actually changed
        require(
            keccak256(abi.encodePacked(asset.assetCode)) != keccak256(abi.encodePacked(_newAssetCode)) ||
            keccak256(abi.encodePacked(asset.name)) != keccak256(abi.encodePacked(_newName)),
            "StellarMetadataMonitor: No metadata change detected"
        );
        
        changeRequestId = nextChangeRequestId++;
        
        metadataChanges[changeRequestId] = MetadataChangeRequest({
            id: changeRequestId,
            stellarAssetId: _stellarAssetId,
            oldAssetCode: asset.assetCode,
            newAssetCode: _newAssetCode,
            oldName: asset.name,
            newName: _newName,
            requester: msg.sender,
            timestamp: block.timestamp,
            processed: false,
            exists: true
        });
        
        emit MetadataChangeRequested(
            changeRequestId,
            _stellarAssetId,
            asset.assetCode,
            _newAssetCode,
            msg.sender
        );
    }
    
    /**
     * @notice Process and approve a metadata change request.
     * @dev Updates the stored metadata and emits MetadataUpdate event for dashboard.
     * @param _changeRequestId ID of the change request to process.
     */
    function processMetadataChange(uint256 _changeRequestId) 
        external 
        onlyOwner 
        changeRequestExists(_changeRequestId)
        nonReentrant 
    {
        MetadataChangeRequest storage change = metadataChanges[_changeRequestId];
        require(!change.processed, "StellarMetadataMonitor: Change already processed");
        
        AssetMetadata storage asset = assetMetadata[change.stellarAssetId];
        
        // Store old values for event
        string memory oldAssetCode = asset.assetCode;
        string memory oldName = asset.name;
        
        // Update asset code mapping if changed
        if (keccak256(abi.encodePacked(oldAssetCode)) != keccak256(abi.encodePacked(change.newAssetCode))) {
            delete assetCodeToId[oldAssetCode];
            assetCodeToId[change.newAssetCode] = change.stellarAssetId;
        }
        
        // Update metadata
        asset.assetCode = change.newAssetCode;
        asset.name = change.newName;
        asset.lastUpdateTime = block.timestamp;
        
        // Mark change as processed
        change.processed = true;
        
        emit MetadataUpdate(
            change.stellarAssetId,
            oldAssetCode,
            change.newAssetCode,
            oldName,
            change.newName,
            block.timestamp
        );
        
        emit MetadataChangeProcessed(_changeRequestId, change.stellarAssetId);
    }
    
    /**
     * @notice Directly update metadata without going through change request.
     * @dev Emergency function for immediate updates when needed.
     * @param _stellarAssetId The Stellar asset ID.
     * @param _newAssetCode New asset code/ticker.
     * @param _newName New asset name.
     * @param _newDomain New home domain.
     */
    function updateMetadataDirect(
        bytes32 _stellarAssetId,
        string memory _newAssetCode,
        string memory _newName,
        string memory _newDomain
    ) external onlyOwner assetExists(_stellarAssetId) {
        AssetMetadata storage asset = assetMetadata[_stellarAssetId];
        
        string memory oldAssetCode = asset.assetCode;
        string memory oldName = asset.name;
        
        // Update asset code mapping if changed
        if (keccak256(abi.encodePacked(oldAssetCode)) != keccak256(abi.encodePacked(_newAssetCode))) {
            delete assetCodeToId[oldAssetCode];
            assetCodeToId[_newAssetCode] = _stellarAssetId;
        }
        
        asset.assetCode = _newAssetCode;
        asset.name = _newName;
        asset.domain = _newDomain;
        asset.lastUpdateTime = block.timestamp;
        
        emit MetadataUpdate(
            _stellarAssetId,
            oldAssetCode,
            _newAssetCode,
            oldName,
            _newName,
            block.timestamp
        );
    }
    
    /**
     * @notice Get metadata for a Stellar asset.
     * @param _stellarAssetId The Stellar asset ID.
     * @return Complete asset metadata.
     */
    function getAssetMetadata(bytes32 _stellarAssetId) 
        external 
        view 
        assetExists(_stellarAssetId) 
        returns (AssetMetadata memory) 
    {
        return assetMetadata[_stellarAssetId];
    }
    
    /**
     * @notice Get metadata change request details.
     * @param _changeRequestId ID of the change request.
     * @return Complete change request information.
     */
    function getMetadataChange(uint256 _changeRequestId) 
        external 
        view 
        changeRequestExists(_changeRequestId) 
        returns (MetadataChangeRequest memory) 
    {
        return metadataChanges[_changeRequestId];
    }
    
    /**
     * @notice Get all tracked Stellar assets.
     * @return Array of all tracked asset IDs.
     */
    function getAllTrackedAssets() external view returns (bytes32[] memory) {
        return trackedAssets;
    }
    
    /**
     * @notice Get pending (unprocessed) metadata change requests.
     * @return Array of pending change request IDs.
     */
    function getPendingChangeRequests() external view returns (uint256[] memory) {
        uint256 count = 0;
        for (uint256 i = 1; i < nextChangeRequestId; i++) {
            if (metadataChanges[i].exists && !metadataChanges[i].processed) {
                count++;
            }
        }
        
        uint256[] memory result = new uint256[](count);
        uint256 index = 0;
        for (uint256 i = 1; i < nextChangeRequestId; i++) {
            if (metadataChanges[i].exists && !metadataChanges[i].processed) {
                result[index] = i;
                index++;
            }
        }
        
        return result;
    }
    
    /**
     * @notice Check if an asset code is being tracked.
     * @param _assetCode The asset code to check.
     * @return True if the asset is tracked.
     */
    function isAssetTracked(string memory _assetCode) external view returns (bool) {
        bytes32 assetId = assetCodeToId[_assetCode];
        return assetMetadata[assetId].exists;
    }
    
    /**
     * @notice Stop tracking an asset (owner only).
     * @param _stellarAssetId The Stellar asset ID to stop tracking.
     */
    function untrackAsset(bytes32 _stellarAssetId) 
        external 
        onlyOwner 
        assetExists(_stellarAssetId) 
    {
        AssetMetadata storage asset = assetMetadata[_stellarAssetId];
        delete assetCodeToId[asset.assetCode];
        asset.exists = false;
        
        emit AssetUntracked(_stellarAssetId);
    }
    
    /**
     * @notice Get total number of tracked assets.
     * @return Number of currently tracked assets.
     */
    function getTrackedAssetCount() external view returns (uint256) {
        uint256 count = 0;
        for (uint256 i = 0; i < trackedAssets.length; i++) {
            if (assetMetadata[trackedAssets[i]].exists) {
                count++;
            }
        }
        return count;
    }
    
    /**
     * @notice Get total number of metadata change requests.
     * @return Total change request count.
     */
    function getChangeRequestCount() external view returns (uint256) {
        return nextChangeRequestId - 1;
    }
}
