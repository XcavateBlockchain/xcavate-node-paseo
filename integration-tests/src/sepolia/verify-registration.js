/**
 * Verify Token Registration on Ethereum TokenGateway (Sepolia)
 *
 * This script checks if WETH (or other token) is properly registered
 * on the TokenGateway contract on Sepolia.
 *
 * Usage: node verify-registration.js
 */

const { ethers } = require('ethers');
const fs = require('fs');
const path = require('path');

// ============================================================================
// CONSTANTS - Token Information (WETH for Sepolia testing)
// ============================================================================
const TOKEN_INFO = {
    name: 'Wrapped Ether',
    symbol: 'WETH',
    decimals: 18
};

// ============================================================================
// CONSTANTS - Load from files
// ============================================================================
const TOKEN_GATEWAY_ADDRESS = fs.readFileSync(
    path.join(__dirname, 'tokenGateway.address'),
    'utf8'
).trim();

const TOKEN_GATEWAY_ABI = JSON.parse(
    fs.readFileSync(path.join(__dirname, 'tokenGateway.abi'), 'utf8')
);

const RPC_ENDPOINTS = fs.readFileSync(
    path.join(__dirname, 'endpoints'),
    'utf8'
).trim().split('\n').filter(line => line.trim());

// ============================================================================
// Main Verification Function
// ============================================================================
async function verifyRegistration() {
    console.log('='.repeat(80));
    console.log('STEP 4 VERIFICATION: Token Registration on Ethereum TokenGateway');
    console.log('='.repeat(80));
    console.log();

    console.log('Token Information:');
    console.log(`  Name:     ${TOKEN_INFO.name}`);
    console.log(`  Symbol:   ${TOKEN_INFO.symbol}`);
    console.log(`  Decimals: ${TOKEN_INFO.decimals}`);
    console.log();

    console.log('Network: Sepolia Testnet');
    console.log(`TokenGateway Address: ${TOKEN_GATEWAY_ADDRESS}`);
    console.log();

    // Try connecting to RPC endpoints
    let provider;
    let connectedEndpoint;

    console.log('Connecting to Sepolia RPC...');
    for (const endpoint of RPC_ENDPOINTS) {
        try {
            console.log(`  Trying: ${endpoint}`);
            provider = new ethers.JsonRpcProvider(endpoint);
            await provider.getNetwork();
            connectedEndpoint = endpoint;
            console.log(`  ✓ Connected successfully!`);
            break;
        } catch (error) {
            console.log(`  ✗ Failed: ${error.message}`);
        }
    }

    if (!provider) {
        console.error('\n❌ Failed to connect to any RPC endpoint');
        process.exit(1);
    }

    console.log();
    console.log('-'.repeat(80));
    console.log('Checking Token Registration Status');
    console.log('-'.repeat(80));
    console.log();

    // Create contract instance
    const tokenGateway = new ethers.Contract(
        TOKEN_GATEWAY_ADDRESS,
        TOKEN_GATEWAY_ABI,
        provider
    );

    // Calculate asset ID (keccak256 of symbol)
    const assetId = ethers.keccak256(ethers.toUtf8Bytes(TOKEN_INFO.symbol));
    console.log(`Calculated Asset ID: ${assetId}`);
    console.log(`  (This is keccak256("${TOKEN_INFO.symbol}"))`);
    console.log();

    try {
        // Check ERC20 address
        console.log('1. Checking Native ERC20 Registration...');
        const erc20Address = await tokenGateway.erc20(assetId);
        console.log(`   ERC20 Address: ${erc20Address}`);

        const isERC20Registered = erc20Address !== ethers.ZeroAddress;
        if (isERC20Registered) {
            console.log(`   ✓ ERC20 is registered!`);
        } else {
            console.log(`   ✗ ERC20 NOT registered (returns zero address)`);
        }
        console.log();

        // Check ERC6160 wrapper
        console.log('2. Checking ERC6160 Wrapper Deployment...');
        const erc6160Address = await tokenGateway.erc6160(assetId);
        console.log(`   ERC6160 Address: ${erc6160Address}`);

        const isERC6160Deployed = erc6160Address !== ethers.ZeroAddress;
        if (isERC6160Deployed) {
            console.log(`   ✓ ERC6160 wrapper is deployed!`);
        } else {
            console.log(`   ✗ ERC6160 wrapper NOT deployed (returns zero address)`);
        }
        console.log();

        // Summary
        console.log('='.repeat(80));
        console.log('VERIFICATION SUMMARY');
        console.log('='.repeat(80));
        console.log();

        const fullyRegistered = isERC20Registered && isERC6160Deployed;

        if (fullyRegistered) {
            console.log('✓ SUCCESS: Token is fully registered and ready for bridging!');
            console.log();
            console.log('Details:');
            console.log(`  - Native ERC20: ${erc20Address}`);
            console.log(`  - ERC6160 Wrapper: ${erc6160Address}`);
            console.log();
            console.log('Next Steps:');
            console.log('  - Users can now approve and teleport tokens from Ethereum to Xcavate');
            console.log('  - Verify the ERC20 contract has correct decimals (should be 18)');
            console.log('  - Test with a small amount first');
        } else {
            console.log('✗ INCOMPLETE: Token registration is not complete');
            console.log();
            console.log('Status:');
            console.log(`  - ERC20 Registered: ${isERC20Registered ? 'YES' : 'NO'}`);
            console.log(`  - ERC6160 Wrapper: ${isERC6160Deployed ? 'YES' : 'NO'}`);
            console.log();
            console.log('Possible Reasons:');
            console.log('  1. Cross-chain message from Xcavate is still pending (~15-20 minutes)');
            console.log('  2. create_erc6160_asset was not called on Xcavate yet');
            console.log('  3. The registration failed - check Hyperbridge explorer');
            console.log();
            console.log('What to do:');
            console.log('  - Wait 15-20 minutes after calling create_erc6160_asset on Xcavate');
            console.log('  - Run this script again to check status');
            console.log('  - Check Hyperbridge explorer for message status');
        }

        console.log();
        console.log('='.repeat(80));

    } catch (error) {
        console.error('\n❌ Error querying TokenGateway:');
        console.error(`   ${error.message}`);
        console.error();
        console.error('Possible causes:');
        console.error('  - RPC endpoint connection issues');
        console.error('  - TokenGateway contract not deployed at this address');
        console.error('  - Network mismatch');
        process.exit(1);
    }
}

// ============================================================================
// Run the script
// ============================================================================
if (require.main === module) {
    verifyRegistration()
        .then(() => process.exit(0))
        .catch(error => {
            console.error('Unexpected error:', error);
            process.exit(1);
        });
}

module.exports = { verifyRegistration, TOKEN_INFO, TOKEN_GATEWAY_ADDRESS };
