// SPDX-License-Identifier: MIT
pragma solidity ^0.8.24;

import "forge-std/Script.sol";
import "../contracts/SustainabilityFund.sol";
import "../contracts/ArbitrationEscrow.sol";
import "../contracts/GrantStreamWithArbitration.sol";
import "../contracts/Web3Courtroom.sol";

contract DeployArbitrationSystem is Script {
    function run() external {
        uint256 deployerPrivateKey = vm.envUint("PRIVATE_KEY");
        address deployer = vm.addr(deployerPrivateKey);
        address treasury = vm.envAddress("TREASURY_ADDRESS");
        address arbitrator1 = vm.envAddress("ARBITRATOR_1_ADDRESS");
        address arbitrator2 = vm.envAddress("ARBITRATOR_2_ADDRESS");
        
        vm.startBroadcast(deployerPrivateKey);
        
        // 1. Deploy SustainabilityFund
        SustainabilityFund sustainabilityFund = new SustainabilityFund(treasury);
        console.log("SustainabilityFund deployed to:", address(sustainabilityFund));
        
        // 2. Deploy ArbitrationEscrow
        ArbitrationEscrow arbitrationEscrow = new ArbitrationEscrow();
        console.log("ArbitrationEscrow deployed to:", address(arbitrationEscrow));
        
        // 3. Deploy GrantStreamWithArbitration
        GrantStreamWithArbitration grantStream = new GrantStreamWithArbitration(
            address(sustainabilityFund),
            address(arbitrationEscrow)
        );
        console.log("GrantStreamWithArbitration deployed to:", address(grantStream));
        
        // 4. Deploy Web3Courtroom
        Web3Courtroom courtroom = new Web3Courtroom(address(arbitrationEscrow));
        console.log("Web3Courtroom deployed to:", address(courtroom));
        
        // 5. Set GrantStream contract in ArbitrationEscrow
        arbitrationEscrow.setGrantStreamContract(address(grantStream));
        console.log("Set GrantStream contract in ArbitrationEscrow");
        
        // 6. Register arbitrators
        uint256 arbitrator1Id = arbitrationEscrow.registerArbitrator(
            arbitrator1,
            "Kleros - Decentralized Justice",
            "International Arbitration"
        );
        arbitrationEscrow.setArbitratorApproval(arbitrator1Id, true);
        console.log("Registered arbitrator 1 with ID:", arbitrator1Id);
        
        uint256 arbitrator2Id = arbitrationEscrow.registerArbitrator(
            arbitrator2,
            "Aragon Court",
            "Digital Dispute Resolution"
        );
        arbitrationEscrow.setArbitratorApproval(arbitrator2Id, true);
        console.log("Registered arbitrator 2 with ID:", arbitrator2Id);
        
        // 7. Register arbitrator profiles in courtroom
        courtroom.registerArbitratorProfile(
            arbitrator1,
            "Kleros",
            "International",
            "Smart Contract Disputes",
            "https://kleros.io",
            "contact@kleros.io"
        );
        
        courtroom.registerArbitratorProfile(
            arbitrator2,
            "Aragon Court",
            "Digital",
            "DAO Governance",
            "https://aragon.org/court",
            "court@aragon.org"
        );
        
        // 8. Transfer ownership to treasury (optional)
        arbitrationEscrow.transferOwnership(treasury);
        grantStream.transferOwnership(treasury);
        courtroom.transferOwnership(treasury);
        
        vm.stopBroadcast();
        
        console.log("\n=== Deployment Summary ===");
        console.log("SustainabilityFund:", address(sustainabilityFund));
        console.log("ArbitrationEscrow:", address(arbitrationEscrow));
        console.log("GrantStreamWithArbitration:", address(grantStream));
        console.log("Web3Courtroom:", address(courtroom));
        console.log("Arbitrator 1 ID:", arbitrator1Id);
        console.log("Arbitrator 2 ID:", arbitrator2Id);
        console.log("Owner transferred to:", treasury);
    }
}
