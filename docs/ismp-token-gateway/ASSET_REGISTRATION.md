# Asset Registration

Complete guide to registering assets for cross-chain transfers via the Token Gateway.

**Navigation:**
- [← Back to Index](./README.md)
- [Architecture Overview](./ARCHITECTURE.md)
- [Transfer Flows →](./TRANSFER_FLOWS.md)
- [Technical Reference](./TECHNICAL_REFERENCE.md)

---

## Overview

Before assets can be transferred cross-chain, they must be registered with the token gateway. This process creates the necessary mappings and deploys contracts on destination chains.

## Step-by-Step Registration

### 1. Register EVM Gateway Addresses

First, configure which EVM chains can send assets to Xcavate:

```rust
// Extrinsic: set_token_gateway_addresses
// Origin: Root only
// Purpose: Whitelist source chains and their gateway addresses

let addresses = BTreeMap::from([
    (StateMachine::Evm(1), vec![0x...]), // Ethereum mainnet
    (StateMachine::Evm(137), vec![0x...]), // Polygon
    (StateMachine::Evm(56), vec![0x...]), // BSC
]);

TokenGateway::set_token_gateway_addresses(
    RuntimeOrigin::root(),
    addresses
)?;
```

**What happens:**
- Stores gateway contract addresses for each chain
- Used to validate incoming cross-chain messages
- Only messages from registered addresses are accepted

---

### 2. Create Local Asset (if needed)

For non-native assets, create them in `pallet_assets` first:

```rust
// Using pallet_assets
Assets::create(
    RuntimeOrigin::root(),
    asset_id: 1, // NOT 0 (reserved for XCAV)
    admin: treasury_account,
    min_balance: 1, // Minimum balance in asset's native decimals
)?;

Assets::set_metadata(
    RuntimeOrigin::signed(admin),
    asset_id: 1,
    name: b"Generic Asset".to_vec(),
    symbol: b"ASSET".to_vec(),
    decimals: 12, // Asset's decimal precision
)?;
```

**Note:** Native XCAV uses `asset_id: 0` (configured as `NativeAssetId`)

---

### 3. Register Asset with Token Gateway

Create multi-chain asset registration:

```rust
use token_gateway_primitives::GatewayAssetRegistration;

let asset_registration = AssetRegistration {
    // Local asset ID in pallet_assets
    local_id: 1, // or 0 for XCAV

    // Is this asset native to Xcavate?
    native: true, // true for XCAV or native assets, false for bridged assets

    // ERC6160 registration details
    reg: GatewayAssetRegistration {
        // Symbol (max 20 chars) - used to generate asset ID
        symbol: "ASSET".as_bytes().to_vec(),

        // Name (max 20 chars)
        name: "Generic Asset".as_bytes().to_vec(),

        // Chains where asset should be deployed
        chains: vec![
            StateMachine::Evm(1),   // Ethereum
            StateMachine::Evm(137), // Polygon
        ],

        // Minimum balance (for Substrate chains)
        minimum_balance: 1,
    },

    // Precision per chain (CRITICAL for handling)
    precision: BTreeMap::from([
        (StateMachine::Evm(1), 18),   // ERC20 always 18 decimals
        (StateMachine::Evm(137), 18), // ERC20 always 18 decimals
    ]),
};

TokenGateway::create_erc6160_asset(
    RuntimeOrigin::root(),
    asset_registration
)?;
```

---

### 4. What Happens During Registration

```
┌─────────────────────────────────────────────────────────────┐
│ 1. Generate Asset ID                                        │
│    asset_id = keccak256("ASSET") // 32-byte hash            │
└────────────┬────────────────────────────────────────────────┘
             │
             ▼
┌─────────────────────────────────────────────────────────────┐
│ 2. Store Mappings                                           │
│    SupportedAssets[1] = asset_id                            │
│    LocalAssets[asset_id] = 1                                │
│    NativeAssets[1] = false                                  │
│    Precisions[1][Ethereum] = 18                             │
│    Precisions[1][Polygon] = 18                              │
└────────────┬────────────────────────────────────────────────┘
             │
             ▼
┌─────────────────────────────────────────────────────────────┐
│ 3. Dispatch to Hyperbridge                                  │
│    - Sends request to TOKEN_GOVERNOR on Hyperbridge         │
│    - Hyperbridge dispatches to each target chain            │
│    - ERC6160 contracts deployed on Ethereum & Polygon       │
└─────────────────────────────────────────────────────────────┘
```

**Storage Updates:**
- `SupportedAssets`: Maps local ID → gateway asset ID
- `LocalAssets`: Reverse mapping (gateway ID → local ID)
- `NativeAssets`: Flags whether asset originates from Xcavate
- `Precisions`: Stores decimal precision per chain

---

## Native vs Bridged Assets

### Native Assets

**Definition:** Assets that originate from Xcavate

**Examples:**
- XCAV (native token)
- Real estate tokens created on Xcavate
- Any asset where Xcavate is the canonical source

**Registration:**
```rust
AssetRegistration {
    local_id: 0, // XCAV
    native: true, // ← Marks as native
    ...
}
```

**Behavior:**
- Uses **custody model** (lock/unlock)
- Total supply on Xcavate never changes
- When sent cross-chain, tokens are locked on Xcavate
- When received back, tokens are unlocked from custody

### Bridged Assets

**Definition:** Assets that originate from other chains

**Examples:**
- TGBP from Ethereum
- DAI from Ethereum
- Tokens from Polygon, BSC, etc.

**Registration:**
```rust
AssetRegistration {
    local_id: 1,
    native: false, // ← Marks as bridged
    ...
}
```

**Behavior:**
- Uses **mint/burn model**
- Supply on Xcavate fluctuates based on bridged amount
- When received, new tokens are minted
- When sent back, tokens are burned

---

## Configuration Parameters

### Symbol and Name

```rust
symbol: "TGBP".as_bytes().to_vec(),  // Max 20 chars
name: "Token Gateway British Pound".as_bytes().to_vec(),  // Max 20 chars
```

- Symbol is used to generate the asset ID: `keccak256(symbol)`
- Must match exactly across all chains
- Case-sensitive

### Supported Chains

```rust
chains: vec![
    StateMachine::Evm(1),   // Ethereum Mainnet
    StateMachine::Evm(11155111), // Sepolia Testnet
    StateMachine::Evm(137), // Polygon
    StateMachine::Evm(56),  // BSC
],
```

Specify all chains where this asset should be deployable.

### Precision Mapping

```rust
precision: BTreeMap::from([
    (StateMachine::Evm(1), 6),  // Ethereum: 6 decimals (TGBP native)
    (StateMachine::Evm(137), 6), // Polygon: 6 decimals
]),
```

**Critical:** This tells the Token Gateway what decimal precision the asset has on each chain. For TGBP:
- Native precision on Ethereum: 6 decimals
- Xcavate maintains: 6 decimals (no conversion)
- All chains use: 6 decimals

---

## Example: Registering TGBP (Bridged Asset)

Complete registration flow for TGBP from Ethereum:

### Step 1: Create Asset on Xcavate

```rust
// Create TGBP asset
Assets::create(
    RuntimeOrigin::root(),
    1, // Asset ID 1
    treasury_account(),
    1, // Minimum balance in 6 decimals
)?;

// Set metadata - use same decimals as source chain
Assets::set_metadata(
    RuntimeOrigin::signed(admin),
    1,
    b"Token Gateway British Pound".to_vec(),
    b"TGBP".to_vec(),
    6, // Same as Ethereum (no conversion)
)?;
```

### Step 2: Register with Token Gateway

```rust
let tgbp_registration = AssetRegistration {
    local_id: 1,
    native: false, // TGBP originates from Ethereum

    reg: GatewayAssetRegistration {
        symbol: "TGBP".as_bytes().to_vec(),
        name: "Token Gateway British Pound".as_bytes().to_vec(),
        chains: vec![StateMachine::Evm(1)], // Ethereum mainnet
        minimum_balance: 1,
    },

    precision: BTreeMap::from([
        (StateMachine::Evm(1), 6), // 6 decimals on Ethereum
    ]),
};

TokenGateway::create_erc6160_asset(
    RuntimeOrigin::root(),
    tgbp_registration
)?;
```

### Step 3: Verify Registration

```rust
// Check storage
let asset_id = keccak256(b"TGBP");
let local_id = LocalAssets::<Runtime>::get(asset_id);
assert_eq!(local_id, Some(1));

// Check precision
let precision = Precisions::<Runtime>::get(1, StateMachine::Evm(1));
assert_eq!(precision, Some(6));

// Check native flag
let is_native = NativeAssets::<Runtime>::get(1);
assert_eq!(is_native, Some(false));
```

---

## Example: Registering XCAV (Native Asset)

Complete registration flow for XCAV to Ethereum:

### Step 1: Register Gateway Address

```rust
let ethereum_gateway = hex!("Fd413e3AFe560182C4471F4d143A96d3e259B6dE");

TokenGateway::set_token_gateway_addresses(
    RuntimeOrigin::root(),
    BTreeMap::from([(StateMachine::Evm(1), ethereum_gateway.to_vec())]),
)?;
```

### Step 2: Register XCAV

```rust
let xcav_registration = AssetRegistration {
    local_id: 0, // NativeAssetId
    native: true, // XCAV originates from Xcavate

    reg: GatewayAssetRegistration {
        symbol: "XCAV".as_bytes().to_vec(),
        name: "Xcavate".as_bytes().to_vec(),
        chains: vec![StateMachine::Evm(1)],
        minimum_balance: 1_000_000_000, // 0.001 XCAV (12 decimals)
    },

    precision: BTreeMap::from([
        (StateMachine::Evm(1), 18), // ERC20 standard
    ]),
};

TokenGateway::create_erc6160_asset(
    RuntimeOrigin::root(),
    xcav_registration
)?;
```

**Result:**
- XCAV can now be bridged to Ethereum
- ERC6160 contract deployed on Ethereum
- Uses custody model (locks XCAV on Xcavate)
- Precision conversion: 12 decimals → 18 decimals

---

## Troubleshooting

### Asset Not Found

**Error:** `Unknown asset`

**Cause:** Asset ID not registered in `LocalAssets` storage

**Solution:**
```rust
// Verify asset was registered
let asset_id = keccak256(b"SYMBOL");
let local_id = LocalAssets::<Runtime>::get(asset_id);
assert!(local_id.is_some(), "Asset not registered");
```

### Precision Not Configured

**Error:** `Asset decimals not configured`

**Cause:** Missing precision mapping for source chain

**Solution:**
```rust
// Update precision for a chain
TokenGateway::update_asset_precision(
    RuntimeOrigin::root(),
    local_id: 1,
    StateMachine::Evm(1),
    6, // decimals
)?;
```

### Gateway Address Not Registered

**Error:** `Not configured to receive from source`

**Cause:** Source chain not in `TokenGatewayAddresses`

**Solution:**
```rust
// Add gateway address for chain
let mut addresses = TokenGatewayAddresses::<Runtime>::get(StateMachine::Evm(1))
    .unwrap_or_default();

TokenGateway::set_token_gateway_addresses(
    RuntimeOrigin::root(),
    BTreeMap::from([(StateMachine::Evm(1), gateway_address)]),
)?;
```

---

## Best Practices

### 1. Verify Asset Origins

Always ensure the `native` flag is set correctly:
- **native: true** → Asset MUST originate from your chain
- **native: false** → Asset comes from elsewhere

Incorrect configuration can lead to:
- Double-spending (marking foreign asset as native)
- Inability to redeem (marking native asset as foreign)

### 2. Test on Testnet First

- Register on Sepolia/testnet before mainnet
- Test small amounts first
- Verify precision conversions work correctly
- Test timeout scenarios

### 3. Document Precision Decisions

- Document why specific decimal values were chosen
- Maintain consistency across all chains
- Consider UX implications of precision choices

### 4. Use Governance for Production

- All production registrations should go through governance
- Use multisig for critical operations
- Implement timelock for safety

---

## Next Steps

- **Understand transfer flows:** [Transfer Flows](./TRANSFER_FLOWS.md)
- **Learn about custody models:** [Technical Reference](./TECHNICAL_REFERENCE.md)
- **See working examples:** [Examples & Troubleshooting](./EXAMPLES.md)

[← Back to Index](./README.md)
