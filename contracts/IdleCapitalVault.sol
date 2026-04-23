// SPDX-License-Identifier: MIT
pragma solidity ^0.8.20;

import "@openzeppelin/contracts/access/Ownable.sol";
import "@openzeppelin/contracts/utils/ReentrancyGuard.sol";
import "./interfaces/IYieldRouter.sol";

contract IdleCapitalVault is Ownable, ReentrancyGuard {
    // === Constants ===
    uint256 public constant INSTANT_ACCESS_BPS = 2000;  // 20% in basis points
    uint256 public constant MAX_BPS = 10000;

    // === State ===
    IYieldRouter public yieldRouter;
    address public treasury;

    uint256 public instantAccessBalance;   // always-liquid funds
    uint256 public deployedShares;         // shares held in yield position

    // === Events ===
    event FundsDeposited(uint256 total, uint256 toInstant, uint256 toYield);
    event FundsWithdrawn(address indexed grantee, uint256 amount);
    event YieldHarvested(uint256 yieldAmount);
    event YieldRouterUpdated(address newRouter);

    constructor(address _yieldRouter, address _treasury) Ownable(msg.sender) {
        yieldRouter = IYieldRouter(_yieldRouter);
        treasury = _treasury;
    }

    // === Core: Route incoming funds ===
    function depositAndRoute() external payable nonReentrant {
        uint256 incoming = msg.value;
        require(incoming > 0, "No funds sent");

        // 20% stays instant-access
        uint256 keepInstant = (incoming * INSTANT_ACCESS_BPS) / MAX_BPS;
        // 80% goes to yield
        uint256 toYield = incoming - keepInstant;

        instantAccessBalance += keepInstant;

        // Deploy to yield router
        if (toYield > 0 && address(yieldRouter) != address(0)) {
            uint256 sharesReceived = yieldRouter.deposit{value: toYield}(toYield);
            deployedShares += sharesReceived;
        }

        emit FundsDeposited(incoming, keepInstant, toYield);
    }

    // === Grantee Withdrawal ===
    function withdraw(uint256 amount) external nonReentrant {
        // Try instant access first
        if (instantAccessBalance >= amount) {
            instantAccessBalance -= amount;
            _sendFunds(msg.sender, amount);
        } else {
            // Pull from yield position to cover the shortfall
            uint256 shortfall = amount - instantAccessBalance;
            _recallFromYield(shortfall);
            instantAccessBalance -= amount;
            _sendFunds(msg.sender, amount);
        }

        emit FundsWithdrawn(msg.sender, amount);
    }

    // === Harvest yield back to DAO treasury ===
    function harvestYield() external onlyOwner nonReentrant {
        uint256 currentValue = yieldRouter.totalAssets();
        uint256 originalDeployed = deployedShares; // simplistic; real impl tracks cost basis

        if (currentValue > originalDeployed) {
            uint256 yieldAmount = currentValue - originalDeployed;
            // Withdraw just the yield
            yieldRouter.withdraw(yieldAmount);
            _sendFunds(treasury, yieldAmount);
            emit YieldHarvested(yieldAmount);
        }
    }

    // === Enforce liquidity threshold ===
    function rebalance() external onlyOwner nonReentrant {
        uint256 totalFunds = totalManagedFunds();
        uint256 requiredInstant = (totalFunds * INSTANT_ACCESS_BPS) / MAX_BPS;

        if (instantAccessBalance < requiredInstant) {
            // Recall from yield to restore 20% floor
            uint256 needed = requiredInstant - instantAccessBalance;
            _recallFromYield(needed);
        }
    }

    // === View ===
    function totalManagedFunds() public view returns (uint256) {
        uint256 inYield = address(yieldRouter) != address(0)
            ? yieldRouter.totalAssets()
            : 0;
        return instantAccessBalance + inYield;
    }

    function liquidityRatio() external view returns (uint256 bps) {
        uint256 total = totalManagedFunds();
        if (total == 0) return MAX_BPS;
        return (instantAccessBalance * MAX_BPS) / total;
    }

    // === Admin ===
    function setYieldRouter(address _newRouter) external onlyOwner {
        yieldRouter = IYieldRouter(_newRouter);
        emit YieldRouterUpdated(_newRouter);
    }

    // === Internal ===
    function _recallFromYield(uint256 amount) internal {
        uint256 recalled = yieldRouter.withdraw(amount);
        instantAccessBalance += recalled;
        deployedShares = deployedShares > recalled ? deployedShares - recalled : 0;
    }

    function _sendFunds(address to, uint256 amount) internal {
        (bool ok, ) = payable(to).call{value: amount}("");
        require(ok, "Transfer failed");
    }

    receive() external payable {}
}