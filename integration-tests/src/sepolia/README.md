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

## Test Token: USD.h

For Sepolia testing, we use **USD.h** which has been successfully tested for bridging to Xcavate.

| Token | Address | Decimals | Asset ID |
|-------|---------|----------|----------|
| USD.h | [`0xa801da100bf16d07f668f4a49e1f71fc54d05177`](https://sepolia.etherscan.io/address/0xa801da100bf16d07f668f4a49e1f71fc54d05177) | 18 | `0x829f01563df2ff9752a529f62c33a4b03b805da1e1dfc748127d6d37795d7257` |

The Asset ID is `keccak256("USD.h")` and is used to identify the token on the TokenGateway contract.

**Alternative:** WETH (Wrapped Ether) at `0xfFf9976782d46CC05630D1f6eBAb18b2324d6B14` can also be used for testing if registered.

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
# From the integration-tests directory
cd integration-tests

# Prepare - validates and shows parameters without executing
npm run teleport

# Execute teleport (requires wallet)
PRIVATE_KEY=0x... npm run teleport:execute
```

**Default Configuration (USD.h):**
```javascript
const CONFIG = {
    token: {
        symbol: 'USD.h',
        address: '0xa801da100bf16d07f668f4a49e1f71fc54d05177',
        decimals: 18,
    },
    amount: '10',              // Amount to bridge (10 USD.h)
    recipient: '5Grwva...',    // Xcavate SS58 address
    destination: 'KUSAMA-4683', // Xcavate testnet (para ID 4683)
    timeout: 3600,             // 1 hour
};
```

**Key Points:**
- Recipient address must be in SS58 format (e.g., `5Grwva...`)
- The script converts it to bytes32 automatically
- Tokens arrive on Xcavate in ~20-30 minutes

### verify-registration.js

Verifies that USD.h (or other token) is properly registered on the TokenGateway contract.

**Default Token:** USD.h - 18 decimals

**What it checks:**
1. ERC20 token registration (via `erc20()` mapping)
2. ERC6160 token registration (via `erc6160()` mapping)
3. Overall registration status

A token is considered registered if it appears in either the `erc20()` or `erc6160()` mapping.

**Usage:**
```bash
# From the integration-tests directory
npm run verify-registration
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
# From the integration-tests directory

# Calculate for tGBP
npm run calc-asset-id -- tGBP

# Calculate for any symbol
npm run calc-asset-id -- USDC
npm run calc-asset-id -- WETH
```

## Network Information

| Parameter | Value |
|-----------|-------|
| Network | Sepolia Testnet |
| Chain ID | 11155111 |
| Block Explorer | https://sepolia.etherscan.io |
| Destination | `KUSAMA-4683` |

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
