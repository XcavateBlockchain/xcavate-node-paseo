/**
 * Helper Script: Calculate Asset ID from Token Symbol
 *
 * This script calculates the asset ID (keccak256 hash) for a token symbol.
 * The asset ID is used to identify tokens in the TokenGateway contract.
 *
 * Usage: node calculate-asset-id.js [symbol]
 */

const { ethers } = require("ethers");

// Token symbol - can be overridden by command line argument
const SYMBOL = process.argv[2] || "WETH";

console.log("=".repeat(60));
console.log("Asset ID Calculator");
console.log("=".repeat(60));
console.log();
console.log(`Token Symbol: ${SYMBOL}`);
console.log();

// Calculate asset ID
const assetId = ethers.keccak256(ethers.toUtf8Bytes(SYMBOL));

console.log(`Asset ID: ${assetId}`);
console.log();
console.log("This is the value used in TokenGateway contract calls:");
console.log("  - tokenGateway.erc20(assetId)");
console.log("  - tokenGateway.erc6160(assetId)");
console.log("  - tokenGateway.teleport({ assetId, ... })");
console.log();
console.log("=".repeat(60));
