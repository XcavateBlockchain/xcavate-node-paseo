# Sepolia Integration Tests

Scripts for verifying and executing token bridging between Ethereum Sepolia and Xcavate.

## Prerequisites

```bash
# Install dependencies
npm install ethers @polkadot/util-crypto
```

## Configuration Files

- `tokenGateway.address` - TokenGateway contract address on Sepolia
- `tokenGateway.abi` - TokenGateway contract ABI
- `endpoints` - List of Sepolia RPC endpoints

## Test Token: WETH

For Sepolia testing, we use **WETH (Wrapped Ether)** which is readily available on the testnet.

| Token | Address | Decimals | Asset ID |
|-------|---------|----------|----------|
| WETH | [`0xfFf9976782d46CC05630D1f6eBAb18b2324d6B14`](https://sepolia.etherscan.io/address/0xfFf9976782d46CC05630D1f6eBAb18b2324d6B14) | 18 | `0x0f8a193ff464434486c0daf7db2a895884365d2bc84ba47a68fcf89c1b14b5b8` |

The Asset ID is `keccak256("WETH")` and is used to identify the token on the TokenGateway contract.

**Note:** On Ethereum Mainnet, tGBP (Tokenised GBP) is the primary bridged asset with Asset ID `0x99bb6e8574d7a5293a476638667ca3492c7e3f9ae2f5a47457f96c3c5c7fc843`. See the main documentation at `/docs/ismp-token-gateway/` for mainnet examples.

## Contract Addresses (Sepolia - Gargantua V3)

| Contract | Address |
|----------|---------|
| TokenGateway | [`0xFcDa26cA021d5535C3059547390E6cCd8De7acA6`](https://sepolia.etherscan.io/address/0xFcDa26cA021d5535C3059547390E6cCd8De7acA6) |
| IsmpHost | [`0x2EdB74C269948b60ec1000040E104cef0eABaae8`](https://sepolia.etherscan.io/address/0x2EdB74C269948b60ec1000040E104cef0eABaae8) |

## Scripts

### teleport-erc20.js (User Flow)

**This is the main script demonstrating the complete user flow for bridging ERC-20 tokens from Ethereum to Xcavate.**

**What it does:**
1. Verifies token is registered on TokenGateway
2. Checks user balances (ETH for gas, tokens to bridge)
3. Approves TokenGateway to spend tokens (if needed)
4. Calls `teleport()` to initiate the bridge

**Usage:**
```bash
# Dry run - shows what would happen without executing
node teleport-erc20.js

# Execute actual teleport (requires wallet)
PRIVATE_KEY=0x... node teleport-erc20.js --execute
```

**Default Configuration (WETH):**
```javascript
const CONFIG = {
    token: {
        symbol: 'WETH',
        address: '0xfFf9976782d46CC05630D1f6eBAb18b2324d6B14',
        decimals: 18,
    },
    amount: '0.001',           // Amount to bridge (0.001 WETH)
    recipient: '5Grwva...',    // Xcavate SS58 address
    destination: 'PASEO-4683', // Xcavate on Paseo
    timeout: 3600,             // 1 hour
};
```

**Key Points:**
- Recipient address must be in SS58 format (e.g., `5Grwva...`)
- The script converts it to bytes32 automatically
- Tokens arrive on Xcavate in ~20-30 minutes

### verify-registration.js

Verifies that WETH (or other token) is properly registered on the TokenGateway contract.

**Default Token:** WETH (Wrapped Ether) - 18 decimals

**What it checks:**
1. ERC20 token registration
2. ERC6160 wrapper deployment
3. Overall registration status

**Usage:**
```bash
node verify-registration.js
```

**Expected Output:**
- If registered: Shows ERC20 and ERC6160 addresses
- If not registered: Explains possible reasons and next steps

**Timeline:**
Registration takes approximately 15-20 minutes after calling `create_erc6160_asset` on Xcavate.

### calculate-asset-id.js

Helper script to calculate the asset ID (keccak256 hash) from a token symbol.

**Usage:**
```bash
# Calculate for tGBP
node calculate-asset-id.js tGBP

# Calculate for any symbol
node calculate-asset-id.js USDC
node calculate-asset-id.js WETH
```

## Network Information

| Parameter | Value |
|-----------|-------|
| Network | Sepolia Testnet |
| Chain ID | 11155111 |
| Block Explorer | https://sepolia.etherscan.io |
| Destination | `PASEO-4683` |

## Troubleshooting

### RPC Connection Issues
The script tries multiple endpoints automatically. If all fail:
1. Check your internet connection
2. Try adding more endpoints to the `endpoints` file
3. Use a custom RPC provider (Alchemy, Infura, etc.)

### Registration Not Found
If the verification shows the token is not registered:
1. Wait 15-20 minutes after calling `create_erc6160_asset`
2. Check Hyperbridge explorer for message status
3. Verify the correct symbol was used in the registration
4. Run the script again

### Wrong Asset ID
Make sure the symbol matches exactly what was registered:
- Case-sensitive: "USD.h" ≠ "usd.h"
- Special characters matter: "USD.h" ≠ "USDh"

## Related Documentation

- Main bridging guide: `/docs/ismp-token-gateway/BRIDGING_ERC20.md`
- Hyperbridge testnet contracts: [Gargantua V3 (Paseo)](https://docs.hyperbridge.network/developers/evm/contracts)
