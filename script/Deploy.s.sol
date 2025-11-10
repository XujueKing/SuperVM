// SPDX-License-Identifier: MIT
pragma solidity ^0.8.20;

import "forge-std/Script.sol";
import "../contracts/RingCTVerifier.sol";

contract DeployRingCTVerifier is Script {
    function run() external returns (RingCTVerifier verifier) {
        uint256 deployerPk = vm.envUint("PRIVATE_KEY");
        vm.startBroadcast(deployerPk);
        verifier = new RingCTVerifier();
        vm.stopBroadcast();
        console.log("RingCTVerifier deployed at", address(verifier));
    }
}
