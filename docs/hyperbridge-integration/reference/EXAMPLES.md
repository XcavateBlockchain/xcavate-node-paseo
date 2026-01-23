# Examples & Troubleshooting

Practical examples and solutions to common issues.

**Navigation:**
- [← Back to Index](./README.md)
- [← Custody & Precision](./CUSTODY_AND_PRECISION.md)
- [Asset Registration](./ASSET_REGISTRATION.md)
- [Transfer Flows](./TRANSFER_FLOWS.md)

---

## Table of Contents

1. [Example 1: Registering tGBP on Xcavate (One-Time Setup)](#example-1-registering-tgbp-on-xcavate-one-time-setup)
2. [Example 2: Sending tGBP from Ethereum to Xcavate](#example-2-sending-tgbp-from-ethereum-to-xcavate)
3. [Example 3: Sending tGBP from Xcavate back to Ethereum](#example-3-sending-tgbp-from-xcavate-back-to-ethereum)
4. [Example 4: Registering XCAV for Ethereum Bridge](#example-4-registering-xcav-for-ethereum-bridge)
5. [Example 5: Cross-Chain Contract Call](#example-5-cross-chain-contract-call)
6. [Common Issues & Solutions](#common-issues--solutions)
7. [Debugging Checklist](#debugging-checklist)

---

## Example 1: Registering tGBP on Xcavate (One-Time Setup)

Before tGBP can be received from Ethereum, it must be registered on Xcavate. This is a one-time governance operation.

### Step 1: Register Ethereum Gateway Address

```rust
// pallet_token_gateway extrinsic (requires Root origin)
let ethereum_gateway = hex!("Fd413e3AFe560182C4471F4d143A96d3e259B6dE");

TokenGateway::set_token_gateway_addresses(
    RuntimeOrigin::root(),
    BTreeMap::from([
        (StateMachine::Evm(1), ethereum_gateway.to_vec()),
    ])
)?;
```

### Step 2: Create tGBP Asset

```rust
// pallet_assets extrinsic
Assets::create(
    RuntimeOrigin::root(),
    1,                    // Asset ID
    treasury_account(),   // Admin account
    1,                    // Minimum balance (1 unit in 18 decimals)
)?;

Assets::set_metadata(
    RuntimeOrigin::signed(admin()),
    1,
    b"tGBP".to_vec(),
    b"tGBP".to_vec(),
    18,  // Same decimals as Ethereum
)?;
```

### Step 3: Register with Token Gateway

```rust
// pallet_token_gateway extrinsic
let tgbp_registration = AssetRegistration {
    local_id: 1,
    native: false,  // tGBP originates from Ethereum, not Xcavate

    reg: GatewayAssetRegistration {
        symbol: "tGBP".as_bytes().to_vec(),
        name: "tGBP".as_bytes().to_vec(),
        chains: vec![StateMachine::Evm(1)],  // Ethereum mainnet
        minimum_balance: Some(1),
    },

    precision: BTreeMap::from([
        (StateMachine::Evm(1), 18),  // 18 decimals on Ethereum
    ]),
};

TokenGateway::create_erc6160_asset(
    RuntimeOrigin::root(),
    tgbp_registration
)?;
```

### Verify Registration

```rust
// Check storage mappings
let asset_id = keccak256(b"tGBP");
let local_id = LocalAssets::<Runtime>::get(asset_id);
assert_eq!(local_id, Some(1));

let precision = Precisions::<Runtime>::get(1, StateMachine::Evm(1));
assert_eq!(precision, Some(18));

let is_native = NativeAssets::<Runtime>::get(1);
assert_eq!(is_native, Some(false));  // false = bridged asset (mint/burn model)
```

---

## Example 2: Sending tGBP from Ethereum to Xcavate

This is the primary use case: bridging tGBP tokens from Ethereum to Xcavate.

> **Prerequisites:** tGBP must be registered on Xcavate first. See [Example 1](#example-1-registering-tgbp-on-xcavate-one-time-setup).

### Step 1: Approve TokenGateway to Spend tGBP

```javascript
const { ethers } = require('ethers');

// Connect to Ethereum
const provider = new ethers.JsonRpcProvider(process.env.ETHEREUM_RPC);
const wallet = new ethers.Wallet(process.env.PRIVATE_KEY, provider);

// tGBP contract on Ethereum Mainnet
const TGBP_ADDRESS = '0x27f6c8289550fCE67f6B50BeD1F519966aFE5287';
const TOKEN_GATEWAY = '0xFd413e3AFe560182C4471F4d143A96d3e259B6dE';

const tgbp = new ethers.Contract(TGBP_ADDRESS, [
    'function approve(address spender, uint256 amount) returns (bool)',
    'function allowance(address owner, address spender) view returns (uint256)'
], wallet);

// Approve 100 tGBP (18 decimals)
const amount = ethers.parseUnits('100', 18);
const approveTx = await tgbp.approve(TOKEN_GATEWAY, amount);
await approveTx.wait();
console.log('Approved TokenGateway to spend tGBP');
```

### Step 2: Call teleport() on TokenGateway

```javascript
const TOKEN_GATEWAY_ABI = [
    'function teleport((uint256 amount, uint256 relayerFee, bytes32 assetId, bool redeem, bytes32 to, bytes dest, uint64 timeout, uint256 nativeCost, bytes data) params) payable'
];

const tokenGateway = new ethers.Contract(TOKEN_GATEWAY, TOKEN_GATEWAY_ABI, wallet);

// Xcavate recipient (32-byte Substrate account)
const recipientAccountId = '0x...'; // Your Xcavate account in hex

// Asset ID = keccak256("tGBP")
const assetId = ethers.keccak256(ethers.toUtf8Bytes('tGBP'));

const teleportParams = {
    amount: ethers.parseUnits('100', 18),  // 100 tGBP
    relayerFee: 0,                          // Hyperbridge relays for free
    assetId: assetId,
    redeem: false,                          // false = mint wrapped tokens on destination
    to: recipientAccountId,                 // 32-byte Xcavate account
    dest: ethers.toUtf8Bytes('POLKADOT-4683'), // Xcavate parachain ID
    timeout: 3600,                          // 1 hour timeout
    nativeCost: 0,
    data: '0x'
};

const tx = await tokenGateway.teleport(teleportParams);
const receipt = await tx.wait();
console.log('Teleport initiated:', receipt.hash);
```

### What Happens

**On Ethereum:**
1. 100 tGBP locked in TokenGateway contract
2. ISMP message dispatched with commitment hash
3. Event emitted: `AssetTeleported`

**Via Hyperbridge (~20-30 minutes):**
1. Waits for Ethereum finalization (~15 minutes)
2. Generates consensus proof
3. Relayers deliver message to Xcavate

**On Xcavate:**
1. `pallet_token_gateway::on_accept()` processes the message
2. 100 tGBP minted to recipient (18 decimals)
3. Event emitted: `AssetReceived`

### Verify Receipt on Xcavate

```javascript
// Using Polkadot API
const balance = await api.query.assets.account(1, recipientAddress);
console.log(`tGBP balance: ${balance.unwrap().balance.toString()}`);
// Expected: 100_000_000_000_000_000_000 (100 tGBP with 18 decimals)
```

---

## Example 3: Sending tGBP from Xcavate back to Ethereum

Send tGBP tokens from Xcavate back to Ethereum (burn on Xcavate, unlock on Ethereum).

### Prepare Parameters

```rust
// Ethereum recipient address (20 bytes, left-padded to 32 bytes)
let eth_recipient = [
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,  // 12 zero bytes padding
    0xd8, 0xda, 0x6b, 0xf2, 0x69, 0x64,  // Ethereum address bytes
    0xaf, 0x9d, 0x7e, 0xed, 0x9e, 0x03,
    0xe5, 0x34, 0x15, 0xd3, 0x7a, 0xa9,
    0x60, 0x45,
];

let params = TeleportParams {
    asset_id: 1,  // tGBP local asset ID
    destination: StateMachine::Evm(1),  // Ethereum mainnet
    recepient: H256::from(eth_recipient),
    amount: 50_000_000_000_000_000_000,  // 50 tGBP (18 decimals)
    timeout: 3600,  // 1 hour
    token_gateway: hex!("Fd413e3AFe560182C4471F4d143A96d3e259B6dE").to_vec(),
    relayer_fee: 0,
    call_data: None,
    redeem: true,  // true = unlock native ERC20 on Ethereum
};
```

### Execute Transfer

```rust
// pallet_token_gateway extrinsic
TokenGateway::teleport(
    RuntimeOrigin::signed(alice()),
    params
)?;
```

### What Happens

**On Xcavate:**
1. 50 tGBP burned from sender's account (mint/burn model for bridged assets)
2. ISMP message dispatched to Ethereum
3. Event emitted: `AssetTeleported`

**On Ethereum:**
1. TokenGateway contract receives verified message
2. 50 tGBP unlocked from custody and transferred to recipient
3. Event emitted: `AssetReceived`

> **Note:** `redeem: true` is used because we want to receive the native ERC20 token on Ethereum, not a wrapped version.

---

## Example 4: Registering XCAV for Ethereum Bridge

Enable XCAV (Xcavate's native token) to be bridged to Ethereum.

### Step 1: Set Up Ethereum Gateway Address

```rust
// pallet_token_gateway extrinsic
let ethereum_gateway = hex!("Fd413e3AFe560182C4471F4d143A96d3e259B6dE");

let addresses = BTreeMap::from([
    (StateMachine::Evm(1), ethereum_gateway.to_vec()),
]);

TokenGateway::set_token_gateway_addresses(
    RuntimeOrigin::root(),
    addresses
)?;
```

### Step 2: Register XCAV

```rust
// pallet_token_gateway extrinsic
let xcav_registration = AssetRegistration {
    local_id: 0,     // NativeAssetId for XCAV
    native: true,    // XCAV originates from Xcavate

    reg: GatewayAssetRegistration {
        symbol: "XCAV".as_bytes().to_vec(),
        name: "Xcavate".as_bytes().to_vec(),
        chains: vec![StateMachine::Evm(1)],
        minimum_balance: Some(1_000_000_000),  // 0.001 XCAV (12 decimals)
    },

    precision: BTreeMap::from([
        (StateMachine::Evm(1), 18),  // ERC20 standard on Ethereum
    ]),
};

TokenGateway::create_erc6160_asset(
    RuntimeOrigin::root(),
    xcav_registration
)?;
```

### Step 3: Verify Registration

```rust
let asset_id = keccak256(b"XCAV");
let local_id = LocalAssets::<Runtime>::get(asset_id);
assert_eq!(local_id, Some(0), "XCAV should map to asset ID 0");

let precision = Precisions::<Runtime>::get(0, StateMachine::Evm(1));
assert_eq!(precision, Some(18), "XCAV should have 18 decimals on Ethereum");

let is_native = NativeAssets::<Runtime>::get(0);
assert_eq!(is_native, Some(true), "XCAV should be marked as native");
```

### Result

- XCAV can now be bridged to Ethereum
- ERC6160 contract deployed on Ethereum by Hyperbridge
- Uses **custody model** (locks XCAV on Xcavate, mints wrapped on Ethereum)
- Precision conversion: 12 decimals (Xcavate) → 18 decimals (Ethereum)

---

## Example 5: Cross-Chain Contract Call

Execute arbitrary logic on destination chain along with asset transfer.

### Scenario

Send 1000 XCAV to another parachain AND automatically transfer 100 XCAV to Bob.

### Encode the Call

```rust
// Encode a pallet call to execute on destination
let call = RuntimeCall::Balances(
    pallet_balances::Call::transfer_keep_alive {
        dest: bob().into(),
        value: 100_000_000_000_000,  // 100 XCAV (12 decimals)
    }
);

let substrate_calldata = SubstrateCalldata {
    signature: None,  // Or include signature for verification
    runtime_call: call.encode(),
};

let params = TeleportParams {
    asset_id: 0,
    destination: StateMachine::Polkadot(1000),  // Another parachain
    recepient: alice_h256(),
    amount: 1_000_000_000_000_000,  // 1000 XCAV (12 decimals)
    timeout: 3600,
    token_gateway: dest_gateway_address,
    relayer_fee: 0,
    call_data: Some(substrate_calldata.encode()),
    redeem: false,
};

TokenGateway::teleport(
    RuntimeOrigin::signed(alice()),
    params
)?;
```

### Result

- 1000 XCAV transferred to Alice on destination
- 100 XCAV automatically transferred from Alice to Bob
- All atomic: either both succeed or both fail

---

## Common Issues & Solutions

### Issue 1: Transfer Not Arriving

**Symptoms:**
- Assets deducted from sender
- No assets received on destination
- No timeout event

**Possible Causes:**

1. **Timeout not expired yet**
   ```rust
   // Check if enough time has passed
   let elapsed = current_timestamp - tx_timestamp;
   if elapsed < timeout {
       // Still within timeout window, wait longer
   }
   ```

2. **Gateway address not registered**
   ```rust
   // Check if gateway is registered
   let gateway = TokenGatewayAddresses::<T>::get(destination);
   assert!(gateway.is_some(), "Gateway not registered");
   ```

3. **Precision not configured**
   ```rust
   // Check precision mapping
   let precision = Precisions::<T>::get(asset_id, destination);
   assert!(precision.is_some(), "Precision not configured");
   ```

**Solutions:**
- Wait for timeout period to complete
- Register gateway address via root
- Configure precision via `update_asset_precision`

---

### Issue 2: Wrong Amount Received

**Symptoms:**
- Transfer succeeds but amount is incorrect
- Amount is 10x, 100x, or 1000x off

**Cause:** Precision misconfiguration

**Debug:**
```rust
// Check precision on both chains
let source_decimals = Precisions::get(asset_id, source)?;
let dest_decimals = Assets::decimals(local_asset_id);

println!("Source: {} decimals", source_decimals);
println!("Dest: {} decimals", dest_decimals);

// Calculate expected amount
let expected = if source_decimals == dest_decimals {
    source_amount
} else if source_decimals > dest_decimals {
    source_amount / 10_u128.pow((source_decimals - dest_decimals) as u32)
} else {
    source_amount * 10_u128.pow((dest_decimals - source_decimals) as u32)
};
```

**Solution:**
Update precision configuration to match actual decimals on each chain.

---

### Issue 3: "Unknown Asset" Error

**Symptoms:**
- Transaction fails with `Unknown asset`
- Asset ID not found

**Cause:** Asset not registered in Token Gateway

**Debug:**
```rust
// Check if asset is registered
let gateway_asset_id = keccak256(b"tGBP");
let local_id = LocalAssets::<Runtime>::get(gateway_asset_id);

match local_id {
    Some(id) => println!("Asset registered with local ID: {}", id),
    None => println!("Asset not registered! Gateway ID: {:?}", gateway_asset_id),
}
```

**Solution:**
Register asset via `create_erc6160_asset` extrinsic.

---

### Issue 4: "Unknown Source Contract" Error

**Symptoms:**
- Incoming message rejected
- Error: `Unknown source contract address`

**Cause:** Source gateway address not whitelisted

**Debug:**
```rust
// Check registered gateway addresses
let registered = TokenGatewayAddresses::<Runtime>::get(StateMachine::Evm(1));
println!("Registered Ethereum gateway: {:?}", registered);

// Compare with message source
println!("Message from: {:?}", message.from);
```

**Solution:**
```rust
// Register the gateway address
TokenGateway::set_token_gateway_addresses(
    RuntimeOrigin::root(),
    BTreeMap::from([(
        StateMachine::Evm(1),
        hex!("Fd413e3AFe560182C4471F4d143A96d3e259B6dE").to_vec()
    )]),
)?;
```

---

### Issue 5: Transaction Fails with "BadOrigin"

**Symptoms:**
- Extrinsic rejected immediately
- Error: `BadOrigin`

**Cause:** Wrong origin for privileged operation

**Solution:**
```rust
// Asset registration requires Root origin
TokenGateway::create_erc6160_asset(
    RuntimeOrigin::root(),  // Must be root
    registration
)?;

// Regular transfers use Signed origin
TokenGateway::teleport(
    RuntimeOrigin::signed(alice()),  // Use signed origin
    params
)?;
```

---

### Issue 6: "Insufficient Balance" on Testnet

**Symptoms:**
- Can't execute transactions
- Account has no funds

**Solution:**
```rust
// Fund account with native tokens
Balances::transfer(
    RuntimeOrigin::signed(treasury()),
    alice().into(),
    1_000_000_000_000_000,  // 1000 XCAV (12 decimals)
)?;

// Or use faucet (testnet only)
// Visit: https://faucet.xcavate.io
```

---

## Debugging Checklist

### Before Sending

- [ ] Asset is registered on both chains
- [ ] Gateway addresses are configured
- [ ] Precision is configured for destination
- [ ] Sender has sufficient balance
- [ ] Timeout is appropriate (≥ 1 hour for Ethereum)
- [ ] Recipient address is correctly formatted

### During Transfer

- [ ] Transaction succeeded on source chain
- [ ] Event `AssetTeleported` was emitted
- [ ] Commitment hash was recorded
- [ ] Assets were locked/burned correctly

### After Sending

- [ ] Check Hyperbridge explorer for relay status
- [ ] Monitor destination chain for `AssetReceived` event
- [ ] Verify recipient balance increased
- [ ] Check for `AssetRefunded` event if timeout

### If Transfer Fails

1. **Check transaction status**
   ```rust
   // Query transaction result
   let result = api.query.system.events(block_hash);
   ```

2. **Check event logs**
   ```rust
   // Look for error events
   for event in events {
       if let Event::System(SystemEvent::ExtrinsicFailed { .. }) = event {
           println!("Transaction failed: {:?}", event);
       }
   }
   ```

3. **Verify configuration**
   ```rust
   // Check all mappings
   let supported = SupportedAssets::get(local_id);
   let local = LocalAssets::get(gateway_id);
   let precision = Precisions::get(local_id, destination);
   let gateway = TokenGatewayAddresses::get(destination);
   ```

4. **Check balances**
   ```rust
   // Verify custody balance
   let pallet_balance = Assets::balance(asset_id, &TokenGateway::pallet_account());
   println!("Pallet custody balance: {}", pallet_balance);
   ```

---

## Useful Commands

### Query Asset Information

```rust
// Get asset metadata
let metadata = Assets::metadata(asset_id);
println!("Name: {}", String::from_utf8_lossy(&metadata.name));
println!("Symbol: {}", String::from_utf8_lossy(&metadata.symbol));
println!("Decimals: {}", metadata.decimals);

// Check if asset exists
let exists = Assets::asset_exists(asset_id);
println!("Asset exists: {}", exists);

// Get total supply
let supply = Assets::total_supply(asset_id);
println!("Total supply: {}", supply);
```

### Query Gateway Configuration

```rust
// Get gateway asset ID
let gateway_id = SupportedAssets::<Runtime>::get(local_id);
println!("Gateway asset ID: {:?}", gateway_id);

// Get precision for a chain
let precision = Precisions::<Runtime>::get(local_id, StateMachine::Evm(1));
println!("Ethereum precision: {:?}", precision);

// Check native flag
let is_native = NativeAssets::<Runtime>::get(local_id);
println!("Is native: {:?}", is_native);

// Get registered gateway address
let gateway = TokenGatewayAddresses::<Runtime>::get(StateMachine::Evm(1));
println!("Ethereum gateway: {:?}", gateway.map(hex::encode));
```

### Monitor Events

```javascript
// Using Polkadot API
api.query.system.events((events) => {
    events.forEach((record) => {
        const { event } = record;

        if (api.events.tokenGateway.AssetTeleported.is(event)) {
            console.log('Asset teleported:', event.data);
        }

        if (api.events.tokenGateway.AssetReceived.is(event)) {
            console.log('Asset received:', event.data);
        }

        if (api.events.tokenGateway.AssetRefunded.is(event)) {
            console.log('Asset refunded:', event.data);
        }
    });
});
```

---

## Next Steps

- **Understand custody models:** [Custody & Precision](./CUSTODY_AND_PRECISION.md)
- **Learn about registration:** [Asset Registration](./ASSET_REGISTRATION.md)
- **See transfer mechanics:** [Transfer Flows](./TRANSFER_FLOWS.md)
- **Complete guide for ERC20:** [Main Bridging Guide](../BRIDGING_ERC20.md)

[← Back to Index](./README.md)
