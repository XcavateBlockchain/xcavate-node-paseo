# Transfer Flows

Detailed mechanics of cross-chain asset transfers via the Token Gateway.

**Navigation:**
- [← Back to Index](./README.md)
- [← Asset Registration](./ASSET_REGISTRATION.md)
- [Architecture Overview](./ARCHITECTURE.md)
- [Technical Reference →](./TECHNICAL_REFERENCE.md)

---

## Table of Contents

1. [Sending Assets (Teleport)](#sending-assets-teleport)
2. [Receiving Assets](#receiving-assets)
3. [Ethereum → Xcavate Complete Flow](#ethereum--xcavate-complete-flow)
4. [Timeout Handling](#timeout-handling)
5. [Event Monitoring](#event-monitoring)

---

## Sending Assets (Teleport)

### Overview

The `teleport` extrinsic locks or burns assets on Xcavate and sends a message to the destination chain to mint or unlock them for the recipient.

### Parameters

```rust
pub struct TeleportParams<AssetId, Balance> {
    /// Local asset ID (0 for XCAV, 1+ for others)
    pub asset_id: AssetId,

    /// Destination chain
    pub destination: StateMachine,

    /// Recipient address (32 bytes)
    /// For EVM: left-pad address with 12 zeros
    /// For Substrate: use AccountId directly
    pub recepient: H256,

    /// Amount to send (in local precision)
    pub amount: Balance,

    /// Timeout in seconds
    /// Accounts for finalization time on both chains
    pub timeout: u64,

    /// Token gateway address on destination
    pub token_gateway: Vec<u8>,

    /// Fee to pay relayer (0 = dispatcher relays)
    pub relayer_fee: Balance,

    /// Optional calldata to execute on destination
    pub call_data: Option<Vec<u8>>,

    /// Redeem to native ERC20 (for EVM destinations)
    pub redeem: bool,
}
```

### Step-by-Step Flow

**Step 1: User Initiates Transfer**

```rust
let params = TeleportParams {
    asset_id: 0, // XCAV
    destination: StateMachine::Evm(1), // Ethereum
    recepient: H256::from([
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, // padding
        0xd8, 0xda, 0x6b, 0xf2, 0x69, 0x64, // EVM address
        0xaf, 0x9d, 0x7e, 0xed, 0x9e, 0x03,
        0xe5, 0x34, 0x15, 0xd3, 0x7a, 0xa9,
        0x60, 0x45,
    ]),
    amount: 100_000_000_000_000, // 100 XCAV (12 decimals)
    timeout: 3600, // 1 hour
    token_gateway: ethereum_gateway_address,
    relayer_fee: 0,
    call_data: None,
    redeem: false,
};

TokenGateway::teleport(
    RuntimeOrigin::signed(alice),
    params
)?;
```

**Step 2: Asset Custody/Burning**

```rust
// Check if asset is native to Xcavate
let is_native = NativeAssets::get(asset_id);

if asset_id == NativeAssetId::get() {
    // Handling XCAV (native token)
    if is_native {
        // CUSTODY MODEL: Lock in pallet account
        NativeCurrency::transfer(
            &sender,
            &TokenGateway::pallet_account(),
            amount,
            ExistenceRequirement::AllowDeath,
        )?;
    } else {
        // BURN MODEL: Reduce total supply
        let imbalance = NativeCurrency::burn(amount);
        NativeCurrency::settle(&sender, imbalance, ...)?;
    }
} else {
    // Handling other assets (via pallet_assets)
    if is_native {
        // Asset originates from Xcavate
        // CUSTODY MODEL: Lock in pallet account
        Assets::transfer(
            asset_id,
            &sender,
            &TokenGateway::pallet_account(),
            amount,
            Preservation::Expendable,
        )?;
    } else {
        // Asset is bridged from another chain
        // BURN MODEL: Destroy tokens
        Assets::burn_from(
            asset_id,
            &sender,
            amount,
            Preservation::Expendable,
            Precision::Exact,
            Fortitude::Polite,
        )?;
    }
}
```

**Step 3: Precision Handling**

For assets bridged from chains with matching decimals (like TGBP: 6 decimals on both Ethereum and Xcavate):

```rust
// Get source decimals
let source_decimals = if asset_id == NativeAssetId::get() {
    12 // XCAV decimals
} else {
    Assets::decimals(asset_id) // e.g., 6 for TGBP
};

// Get destination decimals
let dest_decimals = Precisions::get(asset_id, destination)
    .ok_or(Error::AssetDecimalsNotFound)?;
// e.g., 6 for TGBP on Ethereum

// Convert amount (if necessary)
let converted_amount = if source_decimals == dest_decimals {
    // No conversion needed - amounts match
    amount
} else {
    // Convert between precisions
    convert_to_erc20(amount, dest_decimals, source_decimals)
};
```

**Step 4: Create Cross-Chain Message**

```rust
// Encode message body
let body = Body {
    amount: converted_amount,
    asset_id: gateway_asset_id.0.into(), // 32-byte keccak256 hash
    redeem: params.redeem,
    from: sender_account.into(),
    to: params.recepient.into(),
};

// Create ISMP Post Request
let dispatch_post = DispatchPost {
    dest: params.destination,
    from: PALLET_TOKEN_GATEWAY_ID.to_vec(),
    to: params.token_gateway,
    timeout: params.timeout,
    body: Body::abi_encode(&body),
};
```

**Step 5: Dispatch via ISMP**

```rust
// Send message through Hyperbridge
let commitment = Hyperbridge::dispatch_request(
    DispatchRequest::Post(dispatch_post),
    FeeMetadata {
        payer: sender,
        fee: params.relayer_fee,
    }
)?;

// Emit event
TokenGateway::deposit_event(Event::AssetTeleported {
    from: sender,
    to: params.recepient,
    dest: params.destination,
    amount: params.amount,
    commitment,
});
```

**Step 6: Message Relay (Hyperbridge)**

```
┌─────────────────────────────────────────────────────────────┐
│ Xcavate Parachain                                           │
│ - Assets locked/burned                                      │
│ - Request commitment stored on-chain                        │
│ - Full request data available off-chain                     │
└────────────┬────────────────────────────────────────────────┘
             │
             ▼
┌─────────────────────────────────────────────────────────────┐
│ Hyperbridge Coprocessor                                     │
│ 1. Observes request commitment on Xcavate                   │
│ 2. Generates consensus proof of Xcavate state               │
│ 3. Packages request + proof for destination                 │
└────────────┬────────────────────────────────────────────────┘
             │
             ▼
┌─────────────────────────────────────────────────────────────┐
│ Relayer Network                                             │
│ - Monitors Hyperbridge for pending messages                 │
│ - Submits messages to destination chains                    │
│ - Collects relayer fees                                     │
└────────────┬────────────────────────────────────────────────┘
             │
             ▼
┌─────────────────────────────────────────────────────────────┐
│ Destination Chain                                           │
│ 1. Token Gateway contract receives message                  │
│ 2. Verifies consensus proof via Hyperbridge                 │
│ 3. Mints/unlocks tokens to recipient                        │
└─────────────────────────────────────────────────────────────┘
```

---

## Receiving Assets

### Overview

When assets are sent TO Xcavate from another chain, the token gateway's `on_accept` handler is invoked by ISMP.

### Step-by-Step Flow

**Step 1: ISMP Receives Message**

```rust
// Called by pallet_ismp when message is verified
fn on_accept(&self, request: PostRequest) -> Result<(), anyhow::Error> {
    let PostRequest {
        body,
        from,
        source,
        dest,
        nonce,
        ..
    } = request;

    // Verify message is from registered gateway
    let expected = TokenGatewayAddresses::get(source)
        .ok_or("Not configured to receive from source")?;

    ensure!(from == expected, "Unknown source contract");
```

**Step 2: Decode Message**

```rust
    // Decode ABI-encoded message
    let body: RequestBody = Body::abi_decode(&body[1..])?;

    // body contains:
    // - amount: U256 (in source chain precision)
    // - asset_id: 32-byte keccak256 hash
    // - redeem: bool
    // - from: 32-byte sender address
    // - to: 32-byte recipient address
```

**Step 3: Lookup Local Asset**

```rust
    // Map gateway asset ID to local asset ID
    let local_asset_id = LocalAssets::get(H256::from(body.asset_id.0))
        .ok_or("Unknown asset")?;

    // Example: gateway_id for "TGBP" maps to local_id = 1
```

**Step 4: Precision Handling**

For assets like TGBP with matching decimals (6 on both chains):

```rust
    // Get local decimals
    let decimals = if local_asset_id == NativeAssetId::get() {
        12 // XCAV
    } else {
        Assets::decimals(local_asset_id) // 6 for TGBP
    };

    // Get source decimals
    let source_decimals = Precisions::get(local_asset_id, source)
        .ok_or("Asset decimals not configured")?; // 6 for TGBP on Ethereum

    // Convert if necessary
    let amount = if source_decimals == decimals {
        // No conversion needed
        body.amount.try_into()?
    } else {
        // Convert between precisions
        convert_to_balance(body.amount, source_decimals, decimals)?
    };
    // For TGBP: 100_000_000 (6 decimals) stays 100_000_000 (6 decimals)
```

**Step 5: Mint/Unlock Assets**

```rust
    let beneficiary: AccountId = body.to.0.into();

    if local_asset_id == NativeAssetId::get() {
        // Handling XCAV
        let is_native = NativeAssets::get(NativeAssetId::get());

        if is_native {
            // CUSTODY MODEL: Unlock from pallet account
            NativeCurrency::transfer(
                &TokenGateway::pallet_account(),
                &beneficiary,
                amount,
                ExistenceRequirement::AllowDeath,
            )?;
        } else {
            // MINT MODEL: Increase total supply
            let imbalance = NativeCurrency::issue(amount);
            NativeCurrency::resolve_creating(&beneficiary, imbalance);
        }
    } else {
        // Handling other assets (like TGBP)
        let is_native = NativeAssets::get(local_asset_id);

        if is_native {
            // Asset originates from Xcavate
            // CUSTODY MODEL: Unlock from pallet account
            Assets::transfer(
                local_asset_id,
                &TokenGateway::pallet_account(),
                &beneficiary,
                amount,
                Preservation::Expendable,
            )?;
        } else {
            // Asset is from another chain (like TGBP)
            // MINT MODEL: Create new tokens
            Assets::mint_into(
                local_asset_id,
                &beneficiary,
                amount
            )?;
        }
    }
```

**Step 6: Execute Optional Call**

```rust
    // If calldata was included, execute it
    if let Some(call_data) = body.data {
        let substrate_data = SubstrateCalldata::decode(&call_data)?;

        // Verify signature if provided
        if let Some(signature) = substrate_data.signature {
            // Verify Ed25519, Sr25519, or ECDSA signature
            // ...signature verification logic...
        }

        // Execute the call
        let runtime_call = RuntimeCall::decode(&substrate_data.runtime_call)?;
        runtime_call.dispatch(RawOrigin::Signed(origin).into())?;

        // Increment nonce to prevent replay
        frame_system::Pallet::<T>::inc_account_nonce(origin);
    }
```

**Step 7: Emit Event**

```rust
    TokenGateway::deposit_event(Event::AssetReceived {
        beneficiary,
        amount,
        source,
    });

    Ok(())
}
```

---

## Ethereum → Xcavate Complete Flow

### Overview

```
┌──────────────────────────────────────────────────────────────┐
│                    Ethereum Mainnet                          │
│                                                              │
│  User Wallet                                                 │
│      │                                                       │
│      ├─1─► Approve ERC20 (if needed)                         │
│      │                                                       │
│      └─2─► Call TokenGateway.teleport()                      │
│                    │                                         │
│                    ├─► Lock/Burn Assets                      │
│                    └─► Dispatch ISMP Message                 │
│                                                              │
│  TokenGateway Contract (0x...)                               │
│      │                                                       │
│      └─► IsmpHost Contract                                   │
│              │                                               │
│              └─► Store commitment + emit event               │
└──────────────┼───────────────────────────────────────────────┘
               │
               │ Relayer picks up event
               ▼
┌──────────────────────────────────────────────────────────────┐
│              Hyperbridge Coprocessor (4009)                  │
│                                                              │
│  1. Observes Ethereum state commitment                       │
│  2. Verifies finality (~15 minutes on Ethereum)              │
│  3. Generates consensus proof                                │
│  4. Packages message + proof                                 │
│  5. Relayers submit to Xcavate                               │
└──────────────┼───────────────────────────────────────────────┘
               │
               │ Message + Proof
               ▼
┌──────────────────────────────────────────────────────────────┐
│                  Xcavate Parachain (4683)                    │
│                                                              │
│  pallet_ismp                                                 │
│      ├─► Verify consensus proof                              │
│      ├─► Route to pallet_token_gateway                       │
│      │                                                       │
│  pallet_token_gateway::on_accept()                           │
│      ├─► Verify source gateway address                       │
│      ├─► Decode message body                                 │
│      ├─► Lookup local asset ID                               │
│      ├─► Handle precision (6 decimals maintained)            │
│      ├─► Mint/Unlock tokens to recipient                     │
│      └─► Emit AssetReceived event                            │
│                                                              │
│  Recipient's balance updated ✓                               │
└──────────────────────────────────────────────────────────────┘
```

### Complete Example: Sending TGBP from Ethereum

See [Bridging ERC20 Guide](./BRIDGING_ERC20_GUIDE.md) for detailed Ethereum interaction code and complete examples.

### Timeline

**Total Duration: 20-30 minutes typically**

1. **Ethereum Transaction:** ~15 seconds
   - User submits `teleport()`
   - Transaction confirmed

2. **Ethereum Finalization:** ~15 minutes
   - Wait for ~32 epochs
   - Block becomes finalized

3. **Hyperbridge Processing:** ~2-5 minutes
   - Generate consensus proof
   - Validators sign
   - Package message

4. **Relay to Xcavate:** ~1-2 minutes
   - Relayer picks up message
   - Submits to `pallet_ismp`
   - Verification + routing

5. **Xcavate Processing:** ~12 seconds
   - Next block includes message
   - Assets minted/unlocked
   - Event emitted

---

## Timeout Handling

### Overview

If a cross-chain message isn't delivered within the timeout period, the sender can recover their funds.

### On-Timeout Handler

```rust
fn on_timeout(&self, request: Timeout) -> Result<(), anyhow::Error> {
    match request {
        Timeout::Request(Request::Post(post_request)) => {
            // Decode original request
            let body: RequestBody = Body::abi_decode(&post_request.body[1..])?;

            // Refund original sender
            let beneficiary = body.from.0.into();
            let local_asset_id = LocalAssets::get(H256::from(body.asset_id.0))?;

            // Get decimals
            let local_decimals = Assets::decimals(local_asset_id);
            let erc_decimals = Precisions::get(local_asset_id, post_request.dest)?;

            // Convert amount back to local precision
            let amount = if local_decimals == erc_decimals {
                body.amount.try_into()?
            } else {
                convert_to_balance(body.amount, erc_decimals, local_decimals)?
            };

            // Unlock/mint assets back to sender
            // ... same logic as receiving ...

            TokenGateway::deposit_event(Event::AssetRefunded {
                beneficiary,
                amount,
                source: post_request.dest, // Original destination
            });
        }
    }
    Ok(())
}
```

### Requesting Refund (Ethereum Side)

If timeout period expired and assets weren't delivered on Ethereum, you can request a refund:

```javascript
const handler = new ethers.Contract(HANDLER_ADDRESS, HANDLER_ABI, wallet);

// Get timeout proof from Hyperbridge
const timeoutProof = await fetch(
    `https://hyperbridge.network/api/timeout-proof/${commitment}`
).then(r => r.json());

// Submit timeout
const refundTx = await handler.handlePostRequestTimeouts([{
    request: originalRequest,
    proof: timeoutProof
}]);

await refundTx.wait();
console.log('Assets refunded!');
```

TokenGateway will:
- Unlock ERC20 (if custody model)
- Mint ERC6160 (if burn model)
- Return assets to original sender

---

## Event Monitoring

### On Xcavate

```javascript
// Using Polkadot API
import { createClient } from "polkadot-api"
import { getWsProvider } from "polkadot-api/ws-provider/web"
import { xcavate } from "./xcavate-descriptors"

const client = createClient(getWsProvider('wss://xcavate-rpc.example.com'))
const api = client.getTypedApi(xcavate)

// Subscribe to AssetReceived events
const unsubscribe = api.event.TokenGateway.AssetReceived.watch((event) => {
    console.log(`
        Assets received!
        Beneficiary: ${event.beneficiary}
        Amount: ${event.amount}
        Source: ${event.source}
    `)
})

// Query balance
const balance = await api.query.Assets.Account.getValue(1, aliceAddress) // Asset 1 = TGBP
console.log(`Alice's TGBP balance: ${balance?.balance ?? 0}`)
```

### On Ethereum

```javascript
// Watch for AssetTeleported event
tokenGateway.on('AssetTeleported', (from, to, dest, amount, commitment, assetId, redeem, event) => {
    console.log(`
        Transfer initiated!
        From: ${from}
        To: ${to}
        Amount: ${ethers.formatUnits(amount, 6)} // 6 decimals for TGBP
        Commitment: ${commitment}
        Block: ${event.blockNumber}
    `);
});
```

---

## Next Steps

- **Understand custody models:** [Technical Reference](./TECHNICAL_REFERENCE.md)
- **See working examples:** [Examples & Troubleshooting](./EXAMPLES.md)
- **Learn about registration:** [Asset Registration](./ASSET_REGISTRATION.md)

[← Back to Index](./README.md)
