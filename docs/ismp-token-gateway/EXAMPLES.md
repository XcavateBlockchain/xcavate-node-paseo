# Examples & Troubleshooting

Practical examples and solutions to common issues.

**Navigation:**
- [← Back to Index](./README.md)
- [← Technical Reference](./TECHNICAL_REFERENCE.md)
- [Asset Registration](./ASSET_REGISTRATION.md)
- [Transfer Flows](./TRANSFER_FLOWS.md)

---

## Table of Contents

1. [Example 1: Registering XCAV for Ethereum Bridge](#example-1-registering-xcav-for-ethereum-bridge)
2. [Example 2: Sending XCAV to Ethereum](#example-2-sending-xcav-to-ethereum)
3. [Example 3: Receiving TGBP from Ethereum](#example-3-receiving-tgbp-from-ethereum)
4. [Example 4: Cross-Chain Contract Call](#example-4-cross-chain-contract-call)
5. [Common Issues & Solutions](#common-issues--solutions)
6. [Debugging Checklist](#debugging-checklist)

---

## Example 1: Registering XCAV for Ethereum Bridge

Complete step-by-step guide to enable XCAV bridging to Ethereum.

### Step 1: Set Up Ethereum Gateway Address

```rust
// Ethereum mainnet TokenGateway contract (deployed by Hyperbridge)
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
let xcav_registration = AssetRegistration {
    local_id: 0, // NativeAssetId for XCAV
    native: true, // XCAV originates from Xcavate

    reg: GatewayAssetRegistration {
        symbol: "XCAV".as_bytes().to_vec(),
        name: "Xcavate".as_bytes().to_vec(),
        chains: vec![StateMachine::Evm(1)], // Ethereum mainnet
        minimum_balance: 1_000_000_000, // 0.001 XCAV (12 decimals)
    },

    precision: BTreeMap::from([
        (StateMachine::Evm(1), 18), // ERC20 standard on Ethereum
    ]),
};

TokenGateway::create_erc6160_asset(
    RuntimeOrigin::root(),
    xcav_registration
)?;
```

### Step 3: Verify Registration

```rust
// Verify storage
let asset_id = keccak256(b"XCAV");
let local_id = LocalAssets::<Runtime>::get(asset_id);
assert_eq!(local_id, Some(0), "XCAV should map to asset ID 0");

// Verify precision
let precision = Precisions::<Runtime>::get(0, StateMachine::Evm(1));
assert_eq!(precision, Some(18), "XCAV should have 18 decimals on Ethereum");

// Verify custody model
let is_native = NativeAssets::<Runtime>::get(0);
assert_eq!(is_native, Some(true), "XCAV should be marked as native");
```

### Result

- ✅ XCAV can now be bridged to Ethereum
- ✅ ERC6160 contract deployed on Ethereum by Hyperbridge
- ✅ Gateway asset ID: `keccak256("XCAV")`
- ✅ Uses custody model (locks XCAV on Xcavate)

---

## Example 2: Sending XCAV to Ethereum

Send 1000 XCAV from Xcavate to an Ethereum address.

### Prepare Parameters

```rust
// Ethereum recipient address (20 bytes, left-padded to 32 bytes)
let eth_recipient = [
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, // 12 zero bytes padding
    0xd8, 0xda, 0x6b, 0xf2, 0x69, 0x64, // Ethereum address
    0xaf, 0x9d, 0x7e, 0xed, 0x9e, 0x03,
    0xe5, 0x34, 0x15, 0xd3, 0x7a, 0xa9,
    0x60, 0x45,
];

let params = TeleportParams {
    asset_id: 0, // XCAV
    destination: StateMachine::Evm(1), // Ethereum mainnet
    recepient: H256::from(eth_recipient),
    amount: 1_000_000_000_000_000, // 1000 XCAV (12 decimals)
    timeout: 7200, // 2 hours
    token_gateway: hex!("Fd413e3AFe560182C4471F4d143A96d3e259B6dE").to_vec(),
    relayer_fee: 0, // Hyperbridge will relay
    call_data: None,
    redeem: false,
};
```

### Execute Transfer

```rust
// Call from Alice's account
TokenGateway::teleport(
    RuntimeOrigin::signed(alice()),
    params
)?;
```

### What Happens

**On Xcavate:**
1. 1000 XCAV locked in `TokenGateway::pallet_account()`
2. Amount converted: `1_000_000_000_000_000 * 10^6 = 1_000_000_000_000_000_000_000` (18 decimals)
3. Request dispatched to Hyperbridge
4. Event emitted: `AssetTeleported`

**Via Hyperbridge:**
1. Observes Xcavate state commitment
2. Generates consensus proof
3. Relayers pick up message

**On Ethereum:**
1. Token Gateway contract receives message
2. Verifies proof via Hyperbridge
3. Mints 1000 XCAV (18 decimals) to recipient
4. Event emitted: `AssetReceived`

---

## Example 3: Receiving TGBP from Ethereum

Receive TGBP tokens sent from Ethereum.

### One-Time Setup

#### Create TGBP Asset on Xcavate

```rust
// Create asset with 6 decimals (same as Ethereum)
Assets::create(
    RuntimeOrigin::root(),
    1, // Asset ID
    treasury_account(),
    1, // Min balance in 6 decimals
)?;

Assets::set_metadata(
    RuntimeOrigin::signed(admin()),
    1,
    b"Tokenised GBP".to_vec(),
    b"TGBP".to_vec(),
    6, // Same decimals as Ethereum
)?;
```

#### Register with Token Gateway

```rust
let tgbp_registration = AssetRegistration {
    local_id: 1,
    native: false, // TGBP originates from Ethereum

    reg: GatewayAssetRegistration {
        symbol: "TGBP".as_bytes().to_vec(),
        name: "Tokenised GBP".as_bytes().to_vec(),
        chains: vec![StateMachine::Evm(1)],
        minimum_balance: 1, // Minimum in 6 decimals
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

### Receiving Transfer

When a user sends TGBP from Ethereum:

**Ethereum Side:**
```javascript
// User calls TokenGateway.teleport() on Ethereum
// Locks 500 TGBP (500_000_000 with 6 decimals)
// Message sent through Hyperbridge
```

**Xcavate Side:**
```rust
// Automatic processing via on_accept handler
// 1. Message decoded: 500_000_000 (6 decimals)
// 2. No conversion needed (6 decimals = 6 decimals)
// 3. 500 TGBP minted to recipient
// 4. Event emitted: AssetReceived
```

### Verify Receipt

```rust
// Check balance
let balance = Assets::balance(1, &alice()); // Asset 1 = TGBP
assert_eq!(balance, 500_000_000); // 500 TGBP in 6 decimals
```

---

## Example 4: Cross-Chain Contract Call

Execute arbitrary logic on destination chain along with asset transfer.

### Scenario

Send 1000 XCAV to another parachain AND automatically transfer 100 XCAV to Bob.

### Encode the Call

```rust
// Encode a pallet call to execute on destination
let call = RuntimeCall::Balances(
    pallet_balances::Call::transfer_keep_alive {
        dest: bob().into(),
        value: 100_000_000_000_000, // 100 XCAV
    }
);

let substrate_calldata = SubstrateCalldata {
    signature: None, // Or include signature for verification
    runtime_call: call.encode(),
};

let params = TeleportParams {
    asset_id: 0,
    destination: StateMachine::Kusama(4683), // Another parachain
    recepient: alice_h256(),
    amount: 1_000_000_000_000_000, // 1000 XCAV
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
let gateway_asset_id = keccak256(b"SYMBOL");
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
    RuntimeOrigin::root(), // ← Must be root
    registration
)?;

// Regular transfers use Signed origin
TokenGateway::teleport(
    RuntimeOrigin::signed(alice()), // ← Use signed origin
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
    1_000_000_000_000_000, // 1000 XCAV
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

- **Understand custody models:** [Technical Reference](./TECHNICAL_REFERENCE.md)
- **Learn about registration:** [Asset Registration](./ASSET_REGISTRATION.md)
- **See transfer mechanics:** [Transfer Flows](./TRANSFER_FLOWS.md)
- **Complete guide for ERC20:** [Bridging ERC20 Guide](./BRIDGING_ERC20_GUIDE.md)

[← Back to Index](./README.md)
