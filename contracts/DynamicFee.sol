// SPDX-License-Identifier: MIT
pragma solidity ^0.8.20;

contract DynamicFee {
    // Fee bounds in basis points (1 BPS = 0.01%)
    uint256 public constant MIN_FEE_BPS = 10;   // 0.10% — floor when TVL is high
    uint256 public constant MAX_FEE_BPS = 500;  // 5.00% — ceiling when TVL is low or volatile
    uint256 public constant BASE_FEE_BPS = 100; // 1.00% — baseline at normal load

    // Volatility multiplier bounds (in BPS, applied on top of TVL-scaled fee)
    uint256 public constant VOLATILITY_SCALE_MAX = 200; // max +2% during high volatility

    address public admin;
    uint256 public currentTVL;          // total value locked, in wei
    uint256 public transactionLoad;     // number of txns in current window
    uint256 public volatilityIndex;     // 0–100, set by oracle/admin

    event TVLUpdated(uint256 newTVL);
    event LoadUpdated(uint256 newLoad);
    event VolatilityUpdated(uint256 newIndex);

    constructor() {
        admin = msg.sender;
    }

    modifier onlyAdmin() {
        require(msg.sender == admin, "Not admin");
        _;
    }

    // --- Admin setters (in production these would be oracle-fed) ---

    function setTVL(uint256 _tvl) external onlyAdmin {
        currentTVL = _tvl;
        emit TVLUpdated(_tvl);
    }

    function setTransactionLoad(uint256 _load) external onlyAdmin {
        transactionLoad = _load;
        emit LoadUpdated(_load);
    }

    function setVolatilityIndex(uint256 _index) external onlyAdmin {
        require(_index <= 100, "Index must be 0-100");
        volatilityIndex = _index;
        emit VolatilityUpdated(_index);
    }

    // --- Core: Logarithmic fee curve based on TVL ---
    // As TVL grows → fee shrinks toward MIN_FEE_BPS
    // As TVL shrinks → fee rises toward BASE_FEE_BPS
    // Formula: fee = BASE_FEE - (BASE_FEE - MIN_FEE) * log2(1 + TVL/1e18) / log2(1 + MAX_TVL/1e18)
    // We approximate log2 with integer bit-length (floor(log2(x)) = most significant bit position)

    function computeTVLFee() public view returns (uint256) {
        if (currentTVL == 0) return BASE_FEE_BPS;

        // Normalize TVL to units of 1 ETH (1e18 wei)
        uint256 tvlUnits = currentTVL / 1e18;
        if (tvlUnits == 0) return BASE_FEE_BPS;

        // Integer log2 approximation: count bits
        uint256 logTVL = _log2(tvlUnits + 1);

        // Scale: at logTVL=0 → BASE_FEE, grows toward MIN_FEE
        // Cap logTVL at 20 (represents ~1M ETH TVL as ceiling)
        uint256 maxLog = 20;
        if (logTVL > maxLog) logTVL = maxLog;

        uint256 feeRange = BASE_FEE_BPS - MIN_FEE_BPS;
        uint256 reduction = (feeRange * logTVL) / maxLog;

        return BASE_FEE_BPS - reduction;
    }

    // --- Volatility surcharge: scales up fee when market is stressed ---
    // volatilityIndex 0–100 → adds 0 to VOLATILITY_SCALE_MAX BPS on top

    function computeVolatilitySurcharge() public view returns (uint256) {
        return (VOLATILITY_SCALE_MAX * volatilityIndex) / 100;
    }

    // --- Load surcharge: small bump during high transaction throughput ---
    // Every 1000 txns adds 10 BPS, capped at 100 BPS

    function computeLoadSurcharge() public view returns (uint256) {
        uint256 surcharge = (transactionLoad / 1000) * 10;
        return surcharge > 100 ? 100 : surcharge;
    }

    // --- Final fee: TVL curve + volatility + load, clamped to [MIN, MAX] ---

    function getCurrentFee() public view returns (uint256 feeBPS) {
        uint256 tvlFee       = computeTVLFee();
        uint256 volatilityUp = computeVolatilitySurcharge();
        uint256 loadUp       = computeLoadSurcharge();

        feeBPS = tvlFee + volatilityUp + loadUp;

        // Clamp to bounds
        if (feeBPS < MIN_FEE_BPS) feeBPS = MIN_FEE_BPS;
        if (feeBPS > MAX_FEE_BPS) feeBPS = MAX_FEE_BPS;
    }

    // --- Apply fee to an amount ---

    function applyFee(uint256 amount) external view returns (uint256 afterFee, uint256 feeCharged) {
        uint256 feeBPS = getCurrentFee();
        feeCharged = (amount * feeBPS) / 10000;
        afterFee   = amount - feeCharged;
    }

    // --- Internal: integer floor(log2(x)) ---

    function _log2(uint256 x) internal pure returns (uint256 result) {
        result = 0;
        while (x > 1) {
            x >>= 1;
            result++;
        }
    }
}