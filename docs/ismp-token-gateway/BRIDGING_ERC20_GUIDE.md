# Bridging Existing ERC20 Tokens from Ethereum

Complete step-by-step guide for bridging existing ERC20 tokens (like TGBP, DAI, etc.) from Ethereum to Xcavate.

**Related Documentation:**
- [← Back to Index](./README.md)
- [Architecture Overview](./ARCHITECTURE.md)
- [Asset Registration](./ASSET_REGISTRATION.md)
- [Transfer Flows](./TRANSFER_FLOWS.md)
- [Technical Reference](./TECHNICAL_REFERENCE.md)

---

## Scenario

You have an ERC20 token already deployed on Ethereum (e.g., TGBP at [`0x27f6c8289550fce67f6b50bed1f519966afe5287`](https://etherscan.io/token/0x27f6c8289550fce67f6b50bed1f519966afe5287#code)) and want to enable users to:
1. Send TGBP from Ethereum to Xcavate
2. Use TGBP on Xcavate
3. Send TGBP back from Xcavate to Ethereum

---

## Prerequisites

- ✓ The ERC20 token must already be deployed on Ethereum
- ✓ You need governance/root access on Xcavate to register assets
- ✓ The Hyperbridge TokenGateway must be deployed on Ethereum (see [Architecture](./ARCHITECTURE.md#ethereum-smart-contracts))

---

## Complete Registration Flow

### Step 1: Register on Ethereum TokenGateway

#### Option A: Automatic via Xcavate Registration (Recommended)

When you call `create_erc6160_asset` on Xcavate, the registration message is automatically dispatched through Hyperbridge to Ethereum, which:

1. Deploys an ERC6160 wrapper contract for the asset
2. Links the existing ERC20 token to the TokenGateway
3. Stores the asset mapping in the TokenGateway contract

#### Option B: Manual Registration (If needed)

If automatic registration isn't available or you need to register on Ethereum first, contact the Hyperbridge/Polytope team to:

1. Register the ERC20 address in the TokenGateway contract
2. Deploy the ERC6160 wrapper
3. Configure custody settings

---

### Step 2: Create Asset on Xcavate

Create the asset in `pallet_assets` to represent the bridged token:

```rust
// Create TGBP asset on Xcavate
Assets::create(
    RuntimeOrigin::root(),
    asset_id: 1,
    admin: treasury_account,
    min_balance: 1, // Minimum balance in 6 decimals
)?;

Assets::set_metadata(
    RuntimeOrigin::signed(admin),
    asset_id: 1,
    name: b"Tokenised GBP".to_vec(),
    symbol: b"TGBP".to_vec(),
    decimals: 6, // Match Ethereum's decimals (no conversion)
)?;
```

**Important:**
- Use `native: false` since TGBP originates from Ethereum
- Use the same decimals as the source chain (6 for TGBP on Ethereum)
- The symbol must match exactly what you'll register in the gateway

---

### Step 3: Register with Token Gateway on Xcavate

```rust
use token_gateway_primitives::GatewayAssetRegistration;

let tgbp_registration = AssetRegistration {
    local_id: 1,

    // CRITICAL: false because TGBP originates from Ethereum
    native: false,

    reg: GatewayAssetRegistration {
        // Must match the ERC20 symbol
        symbol: "TGBP".as_bytes().to_vec(),
        name: "Tokenised GBP".as_bytes().to_vec(),

        // Chains where this asset exists
        chains: vec![
            StateMachine::Evm(1), // Ethereum mainnet
        ],

        minimum_balance: 1, // Minimum balance in asset's native decimals (6 for TGBP)
    },

    // Map decimals for each chain
    precision: BTreeMap::from([
        // Ethereum: Check the actual ERC20 decimals
        // TGBP uses 6 decimals on Ethereum
        (StateMachine::Evm(1), 6),
    ]),
};

TokenGateway::create_erc6160_asset(
    RuntimeOrigin::root(),
    tgbp_registration
)?;
```

**What happens:**
1. Asset registered in Xcavate's TokenGateway pallet
2. Message dispatched to Hyperbridge TOKEN_GOVERNOR
3. Hyperbridge dispatches to Ethereum TokenGateway
4. ERC6160 wrapper deployed (if not already done)
5. Asset becomes available for bridging

---

### Step 4: Verify Registration on Ethereum

After the cross-chain message is processed (~15-20 minutes), verify the registration on Ethereum:

```javascript
const tokenGateway = new ethers.Contract(
    TOKEN_GATEWAY_ADDRESS,
    TOKEN_GATEWAY_ABI,
    provider
);

// Get the asset ID (keccak256 of symbol)
const assetId = ethers.keccak256(ethers.toUtf8Bytes('TGBP'));

// Check if ERC20 is registered
const erc20Address = await tokenGateway.erc20(assetId);
console.log(`Native ERC20: ${erc20Address}`);

// Check if ERC6160 wrapper is deployed
const erc6160Address = await tokenGateway.erc6160(assetId);
console.log(`ERC6160 Wrapper: ${erc6160Address}`);

// Both should return valid addresses (not 0x0000...)
```

---

### Step 5: Link Existing ERC20 (If Needed)

If the automatic registration didn't link your existing ERC20, the TokenGateway admin needs to call:

```solidity
// This is typically done by Hyperbridge/Polytope team
tokenGateway.setERC20(
    assetId,        // keccak256("TGBP")
    TGBP_ADDRESS    // 0x... (existing ERC20 address)
);
```

**Note:** This step is usually handled automatically or by the Hyperbridge team during registration.

---

## How Bridging Works After Registration

### From Ethereum to Xcavate

#### 1. User on Ethereum:

```javascript
// Approve TGBP tokens
await tgbp.approve(TOKEN_GATEWAY_ADDRESS, amount);

// Approve fee tokens
await feeToken.approve(TOKEN_GATEWAY_ADDRESS, feeAmount);

// Teleport to Xcavate
await tokenGateway.teleport({
    amount: ethers.parseUnits('100', 6), // 100 TGBP (6 decimals on Ethereum)
    assetId: ethers.keccak256(ethers.toUtf8Bytes('TGBP')),
    to: recipientOnXcavate,
    dest: ethers.encodeBytes32String('PARA-4683'),
    timeout: 3600,
    relayerFee: 0,
    nativeCost: 0,
    data: '0x',
    redeem: false
});
```

#### 2. TokenGateway on Ethereum:
- Locks TGBP tokens in custody (since it's a native Ethereum asset)
- Dispatches message through Hyperbridge

#### 3. Xcavate receives:
- **Mints** TGBP tokens to recipient (since `native: false`)
- Emits `AssetReceived` event

**See [Transfer Flows](./TRANSFER_FLOWS.md#ethereum--xcavate-complete-flow) for detailed timeline and monitoring.**

---

### From Xcavate to Ethereum

#### 1. User on Xcavate:

```rust
TokenGateway::teleport(
    RuntimeOrigin::signed(alice),
    TeleportParams {
        asset_id: 1, // TGBP
        destination: StateMachine::Evm(1),
        recepient: ethereum_recipient,
        amount: 100_000_000, // 100 TGBP (6 decimals)
        timeout: 3600,
        token_gateway: ethereum_gateway_address,
        relayer_fee: 0,
        call_data: None,
        redeem: false,
    }
)?;
```

#### 2. TokenGateway on Xcavate:
- **Burns** TGBP tokens (since `native: false`)
- Dispatches message through Hyperbridge

#### 3. Ethereum receives:
- Unlocks TGBP from custody
- Transfers to recipient

---

## Key Points for Existing ERC20s

### ✅ DO:

- Set `native: false` on Xcavate (since token originates from Ethereum)
- Use correct decimals for the source ERC20
- Verify ERC20 is properly linked in TokenGateway
- Test with small amounts first

### ❌ DON'T:

- Set `native: true` for Ethereum-originated tokens (will break custody model)
- Assume decimals - always verify the actual ERC20 contract
- Skip the verification step on Ethereum

**See [Technical Reference](./TECHNICAL_REFERENCE.md#asset-custody-models) for custody model details.**

---

## Troubleshooting

### Issue: Tokens not arriving on Xcavate after teleporting from Ethereum

**Checklist:**
1. ✓ Is the asset registered on Xcavate? Check `SupportedAssets[1]`
2. ✓ Is the ERC20 linked in Ethereum TokenGateway? Call `erc20(assetId)`
3. ✓ Are decimals configured correctly? Check `Precisions[1][Ethereum]`
4. ✓ Did the user approve tokens? Check `token.allowance(user, gateway)`
5. ✓ Did the message timeout? Check Hyperbridge explorer

### Issue: Can't send tokens back to Ethereum from Xcavate

**Possible causes:**
1. Incorrect `native` flag (should be `false`)
2. No tokens in custody on Ethereum (they were never locked)
3. Asset not registered in both directions

**More troubleshooting:** See [Examples & Troubleshooting](./EXAMPLES.md#troubleshooting)

---

## Example: Complete TGBP Registration

```rust
// 1. Create on Xcavate
Assets::create(RuntimeOrigin::root(), 1, treasury, 1_000_000_000)?;
Assets::set_metadata(
    RuntimeOrigin::signed(admin),
    1,
    b"Tokenised GBP".to_vec(),
    b"TGBP".to_vec(),
    12
)?;

// 2. Register with Token Gateway
TokenGateway::create_erc6160_asset(
    RuntimeOrigin::root(),
    AssetRegistration {
        local_id: 1,
        native: false, // From Ethereum
        reg: GatewayAssetRegistration {
            symbol: "TGBP".as_bytes().to_vec(),
            name: "Tokenised GBP".as_bytes().to_vec(),
            chains: vec![StateMachine::Evm(1)],
            minimum_balance: 1_000_000_000,
        },
        precision: BTreeMap::from([
            (StateMachine::Evm(1), 6), // TGBP has 6 decimals on Ethereum
        ]),
    }
)?;

// 3. Wait for cross-chain message to process (~15-20 minutes)

// 4. Verify on Ethereum (using ethers.js)
// const assetId = ethers.keccak256(ethers.toUtf8Bytes('TGBP'));
// const erc20 = await tokenGateway.erc20(assetId);
// Should return the actual TGBP token address

// 5. Now users can bridge TGBP between Ethereum and Xcavate!
```

---

## Next Steps

- **Understand the architecture:** [Architecture Overview](./ARCHITECTURE.md)
- **Learn about asset registration:** [Asset Registration](./ASSET_REGISTRATION.md)
- **See detailed transfer flows:** [Transfer Flows](./TRANSFER_FLOWS.md)
- **Understand custody models:** [Technical Reference](./TECHNICAL_REFERENCE.md)

[← Back to Index](./README.md)
