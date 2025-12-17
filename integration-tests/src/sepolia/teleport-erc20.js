/**
 * Teleport ERC-20 Tokens from Ethereum to Xcavate
 *
 * This script demonstrates how a user bridges ERC-20 tokens (e.g., WETH, USDC)
 * from Ethereum to Xcavate via the Hyperbridge TokenGateway.
 *
 * Prerequisites:
 * 1. Token must be registered on both chains (via create_erc6160_asset on Xcavate)
 * 2. User must have the ERC-20 tokens to bridge
 * 3. User must have ETH for gas fees
 *
 * Usage:
 *   # Prepare (validates and shows parameters, no transaction)
 *   npm run teleport
 *
 *   # Execute teleport (requires PRIVATE_KEY env var)
 *   PRIVATE_KEY=0x... npm run teleport:execute
 */

const { ethers } = require('ethers');
const { decodeAddress } = require('@polkadot/util-crypto');
const fs = require('fs');
const path = require('path');

// ============================================================================
// CONFIGURATION - Modify these values for your transfer
// ============================================================================

const CONFIG = {
    // Token to bridge - WETH (Wrapped Ether) on Sepolia
    // WETH is readily available on Sepolia for testing
    token: {
        symbol: 'WETH',
        address: '0xfFf9976782d46CC05630D1f6eBAb18b2324d6B14', // WETH on Sepolia
        decimals: 18,
    },

    // Amount to bridge (in human-readable format)
    amount: '0.001', // 0.001 WETH (small amount for testing)

    // Recipient on Xcavate (SS58 address)
    recipient: '5GrwvaEF5zXb26Fz9rcQpDWS57CtERHpNehXCPcNoHGKutQY', // Alice

    // Destination chain identifier for Xcavate on Paseo
    destination: 'PASEO-4683',

    // Timeout in seconds (1 hour default)
    timeout: 3600,

    // Relayer fee (in token's smallest unit, 0 = use default)
    relayerFee: 0,

    // Redeem native asset on destination (false for bridged ERC-20s)
    redeem: false,
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

// Standard ERC20 ABI for approve and balanceOf
const ERC20_ABI = [
    'function approve(address spender, uint256 amount) returns (bool)',
    'function allowance(address owner, address spender) view returns (uint256)',
    'function balanceOf(address account) view returns (uint256)',
    'function decimals() view returns (uint8)',
    'function symbol() view returns (string)',
];

// ============================================================================
// UTILITY FUNCTIONS
// ============================================================================

/**
 * Convert SS58 address to bytes32 hex format
 */
function ss58ToBytes32(ss58Address) {
    const publicKey = decodeAddress(ss58Address);
    return '0x' + Buffer.from(publicKey).toString('hex');
}

/**
 * Calculate asset ID from symbol (keccak256 hash)
 */
function calculateAssetId(symbol) {
    return ethers.keccak256(ethers.toUtf8Bytes(symbol));
}

/**
 * Format amount with decimals for display
 */
function formatAmount(amount, decimals) {
    return ethers.formatUnits(amount, decimals);
}

/**
 * Parse amount string to BigInt with decimals
 */
function parseAmount(amountStr, decimals) {
    return ethers.parseUnits(amountStr, decimals);
}

// ============================================================================
// MAIN TELEPORT FUNCTION
// ============================================================================

async function teleportERC20() {
    const executeMode = process.argv.includes('--execute');

    console.log('='.repeat(80));
    console.log('TELEPORT ERC-20 TOKENS: Ethereum -> Xcavate');
    console.log('='.repeat(80));
    console.log();

    // Display configuration
    console.log('Transfer Configuration:');
    console.log(`  Token:       ${CONFIG.token.symbol} (${CONFIG.token.address})`);
    console.log(`  Amount:      ${CONFIG.amount} ${CONFIG.token.symbol}`);
    console.log(`  Recipient:   ${CONFIG.recipient}`);
    console.log(`  Destination: ${CONFIG.destination}`);
    console.log(`  Timeout:     ${CONFIG.timeout} seconds`);
    console.log(`  Mode:        ${executeMode ? 'EXECUTE' : 'PREPARE (no transaction)'}`);
    console.log();

    // Connect to RPC
    console.log('Connecting to Sepolia...');
    let provider;
    for (const endpoint of RPC_ENDPOINTS) {
        try {
            provider = new ethers.JsonRpcProvider(endpoint);
            await provider.getNetwork();
            console.log(`  Connected to: ${endpoint}`);
            break;
        } catch (error) {
            console.log(`  Failed: ${endpoint}`);
        }
    }

    if (!provider) {
        console.error('Failed to connect to any RPC endpoint');
        process.exit(1);
    }
    console.log();

    // Setup wallet (if executing)
    let wallet;
    if (executeMode) {
        const privateKey = process.env.PRIVATE_KEY;
        if (!privateKey) {
            console.error('ERROR: PRIVATE_KEY environment variable required for --execute mode');
            console.error('Usage: PRIVATE_KEY=0x... node teleport-erc20.js --execute');
            process.exit(1);
        }
        wallet = new ethers.Wallet(privateKey, provider);
        console.log(`Wallet Address: ${wallet.address}`);
        console.log();
    }

    // Create contract instances
    const tokenGateway = new ethers.Contract(
        TOKEN_GATEWAY_ADDRESS,
        TOKEN_GATEWAY_ABI,
        wallet || provider
    );

    const erc20 = new ethers.Contract(
        CONFIG.token.address,
        ERC20_ABI,
        wallet || provider
    );

    // Calculate values
    const assetId = calculateAssetId(CONFIG.token.symbol);
    const recipientBytes32 = ss58ToBytes32(CONFIG.recipient);
    const amountWei = parseAmount(CONFIG.amount, CONFIG.token.decimals);
    const destBytes = ethers.toUtf8Bytes(CONFIG.destination);

    console.log('-'.repeat(80));
    console.log('Calculated Values:');
    console.log('-'.repeat(80));
    console.log(`  Asset ID:     ${assetId}`);
    console.log(`  Recipient:    ${recipientBytes32}`);
    console.log(`  Amount (wei): ${amountWei.toString()}`);
    console.log(`  Destination:  ${CONFIG.destination} (${ethers.hexlify(destBytes)})`);
    console.log();

    // Check token registration
    console.log('-'.repeat(80));
    console.log('Step 1: Verify Token Registration');
    console.log('-'.repeat(80));

    const erc20Address = await tokenGateway.erc20(assetId);
    const erc6160Address = await tokenGateway.erc6160(assetId);

    console.log(`  ERC20 Address:  ${erc20Address}`);
    console.log(`  ERC6160 Address: ${erc6160Address}`);

    if (erc20Address === ethers.ZeroAddress) {
        console.error('\nERROR: Token is not registered on TokenGateway!');
        console.error('The token must be registered via create_erc6160_asset on Xcavate first.');
        console.error('See: docs/ismp-token-gateway/BRIDGING_ERC20.md');
        process.exit(1);
    }
    console.log('  Status: Token is registered');
    console.log();

    // Check balances (if we have a wallet)
    if (wallet) {
        console.log('-'.repeat(80));
        console.log('Step 2: Check Balances');
        console.log('-'.repeat(80));

        const ethBalance = await provider.getBalance(wallet.address);
        const tokenBalance = await erc20.balanceOf(wallet.address);

        console.log(`  ETH Balance:   ${formatAmount(ethBalance, 18)} ETH`);
        console.log(`  Token Balance: ${formatAmount(tokenBalance, CONFIG.token.decimals)} ${CONFIG.token.symbol}`);

        if (tokenBalance < amountWei) {
            console.error(`\nERROR: Insufficient ${CONFIG.token.symbol} balance!`);
            console.error(`  Required: ${CONFIG.amount} ${CONFIG.token.symbol}`);
            console.error(`  Available: ${formatAmount(tokenBalance, CONFIG.token.decimals)} ${CONFIG.token.symbol}`);
            process.exit(1);
        }
        console.log('  Status: Sufficient balance');
        console.log();

        // Check allowance
        console.log('-'.repeat(80));
        console.log('Step 3: Check/Set Allowance');
        console.log('-'.repeat(80));

        const currentAllowance = await erc20.allowance(wallet.address, TOKEN_GATEWAY_ADDRESS);
        console.log(`  Current Allowance: ${formatAmount(currentAllowance, CONFIG.token.decimals)} ${CONFIG.token.symbol}`);

        if (currentAllowance < amountWei) {
            console.log('  Need to approve TokenGateway to spend tokens...');

            if (executeMode) {
                console.log('  Sending approve transaction...');
                const approveTx = await erc20.approve(TOKEN_GATEWAY_ADDRESS, amountWei);
                console.log(`  Transaction: ${approveTx.hash}`);
                console.log('  Waiting for confirmation...');
                await approveTx.wait();
                console.log('  Approval confirmed!');
            } else {
                console.log('  [DRY RUN] Would send approve transaction');
            }
        } else {
            console.log('  Status: Sufficient allowance already set');
        }
        console.log();
    }

    // Build teleport parameters
    console.log('-'.repeat(80));
    console.log('Step 4: Teleport Parameters');
    console.log('-'.repeat(80));

    const teleportParams = {
        amount: amountWei,
        relayerFee: CONFIG.relayerFee,
        assetId: assetId,
        redeem: CONFIG.redeem,
        to: recipientBytes32,
        dest: destBytes,
        timeout: CONFIG.timeout,
        nativeCost: 0,
        data: '0x',
    };

    console.log('  TeleportParams struct:');
    console.log(`    amount:     ${teleportParams.amount.toString()}`);
    console.log(`    relayerFee: ${teleportParams.relayerFee}`);
    console.log(`    assetId:    ${teleportParams.assetId}`);
    console.log(`    redeem:     ${teleportParams.redeem}`);
    console.log(`    to:         ${teleportParams.to}`);
    console.log(`    dest:       ${ethers.hexlify(destBytes)}`);
    console.log(`    timeout:    ${teleportParams.timeout}`);
    console.log(`    nativeCost: ${teleportParams.nativeCost}`);
    console.log(`    data:       ${teleportParams.data}`);
    console.log();

    // Execute teleport
    if (executeMode && wallet) {
        console.log('-'.repeat(80));
        console.log('Step 5: Execute Teleport');
        console.log('-'.repeat(80));

        try {
            console.log('  Sending teleport transaction...');
            const teleportTx = await tokenGateway.teleport(teleportParams);
            console.log(`  Transaction: ${teleportTx.hash}`);
            console.log('  Waiting for confirmation...');

            const receipt = await teleportTx.wait();
            console.log(`  Block: ${receipt.blockNumber}`);
            console.log(`  Gas Used: ${receipt.gasUsed.toString()}`);

            // Parse events
            const assetTeleportedEvent = receipt.logs.find(log => {
                try {
                    const parsed = tokenGateway.interface.parseLog(log);
                    return parsed.name === 'AssetTeleported';
                } catch {
                    return false;
                }
            });

            if (assetTeleportedEvent) {
                const parsed = tokenGateway.interface.parseLog(assetTeleportedEvent);
                console.log();
                console.log('  AssetTeleported Event:');
                console.log(`    commitment: ${parsed.args.commitment}`);
                console.log(`    to:         ${parsed.args.to}`);
                console.log(`    dest:       ${parsed.args.dest}`);
                console.log(`    amount:     ${parsed.args.amount.toString()}`);
            }

            console.log();
            console.log('='.repeat(80));
            console.log('SUCCESS! Tokens are being bridged to Xcavate');
            console.log('='.repeat(80));
            console.log();
            console.log('What happens next:');
            console.log('  1. Hyperbridge relayers will pick up the message (~2-5 minutes)');
            console.log('  2. Consensus proof is submitted to Hyperbridge coprocessor');
            console.log('  3. State proof is relayed to Xcavate (~15-20 minutes total)');
            console.log('  4. Tokens are minted to recipient on Xcavate');
            console.log();
            console.log('Track progress:');
            console.log('  - Etherscan: https://sepolia.etherscan.io/tx/' + teleportTx.hash);
            console.log('  - Hyperbridge Explorer: https://explorer.hyperbridge.network');
            console.log();

        } catch (error) {
            console.error('\nERROR: Teleport transaction failed!');
            console.error(`  ${error.message}`);

            if (error.data) {
                console.error(`  Data: ${error.data}`);
            }
            process.exit(1);
        }
    } else {
        console.log('-'.repeat(80));
        console.log('To execute the teleport:');
        console.log('  PRIVATE_KEY=0x... npm run teleport:execute');
        console.log('-'.repeat(80));
    }
}

// ============================================================================
// RUN
// ============================================================================

if (require.main === module) {
    teleportERC20()
        .then(() => process.exit(0))
        .catch(error => {
            console.error('Unexpected error:', error);
            process.exit(1);
        });
}

module.exports = { teleportERC20, CONFIG, ss58ToBytes32, calculateAssetId };
