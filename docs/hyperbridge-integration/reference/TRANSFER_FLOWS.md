# Transfer Flows

Detailed mechanics of cross-chain asset transfers via the Token Gateway.

**Navigation:**
- [← Back to Index](./README.md)
- [← Asset Registration](./ASSET_REGISTRATION.md)
- [Architecture Overview](./ARCHITECTURE.md)
- [Custody & Precision →](./CUSTODY_AND_PRECISION.md)

---

## Table of Contents

1. [Ethereum → Xcavate: Receiving tGBP](#ethereum--xcavate-receiving-tgbp)
2. [Receiving Assets (on_accept Handler)](#receiving-assets-on_accept-handler)
3. [Xcavate → Ethereum: Sending Assets](#xcavate--ethereum-sending-assets)
4. [Sending Assets (teleport Extrinsic)](#sending-assets-teleport-extrinsic)
5. [Timeout Handling](#timeout-handling)
6. [Event Monitoring](#event-monitoring)

---

## Ethereum → Xcavate: Receiving tGBP

This is the primary use case: bridging ERC-20 tokens like tGBP from Ethereum to Xcavate.

### Overview

```
┌──────────────────────────────────────────────────────────────┐
│                    Ethereum Mainnet                          │
│                                                              │
│  User Wallet                                                 │
│      │                                                       │
│      ├─1─► Approve ERC20 (tGBP)                              │
│      │                                                       │
│      └─2─► Call TokenGateway.teleport()                      │
│                    │                                         │
│                    ├─► Lock tGBP in custody                  │
│                    └─► Dispatch ISMP Message                 │
│                                                              │
│  TokenGateway Contract (0xFd413e3AFe560182C4471F4d143A96d3e259B6dE)
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
│      ├─► Lookup local asset ID (tGBP → 1)                    │
│      ├─► Handle precision (18 decimals maintained)           │
│      ├─► Mint tGBP to recipient                              │
│      └─► Emit AssetReceived event                            │
│                                                              │
│  Recipient's tGBP balance updated ✓                          │
└──────────────────────────────────────────────────────────────┘
```

### Timeline

**Total Duration: 20-30 minutes typically**

1. **Ethereum Transaction:** ~15 seconds
   - User submits `teleport()` with tGBP
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
   - tGBP minted to recipient
   - `AssetReceived` event emitted

### Complete Example

See [Main Bridging Guide](../BRIDGING_ERC20.md) for detailed Ethereum interaction code and complete JavaScript examples.

---

## Receiving Assets (on_accept Handler)

When assets are sent TO Xcavate from another chain (like Ethereum), the token gateway's `on_accept` handler is invoked by ISMP after the consensus proof is verified.

### Step-by-Step Flow

**Step 1: ISMP Receives Message**

```rust
// pallet_token_gateway: IsmpModule::on_accept()
// Called by pallet_ismp when message proof is verified

fn on_accept(
    &self,
    PostRequest { body, from, source, dest, nonce, .. }: PostRequest,
) -> Result<(), anyhow::Error> {
    // Verify message is from a registered gateway address
    let expected = TokenGatewayAddresses::<T>::get(source)
        .ok_or_else(|| anyhow!("Not configured to receive assets from {source:?}"))?;

    ensure!(
        from == expected,
        ismp::error::Error::ModuleDispatchError {
            msg: "Token Gateway: Unknown source contract address".to_string(),
            meta: Meta { source, dest, nonce },
        }
    );
```

**Step 2: Decode Message**

```rust
    // Decode ABI-encoded message (skip first byte - enum variant selector)
    let body: RequestBody = if let Ok(body) = Body::abi_decode(&mut &body[1..], true) {
        body.into()
    } else if let Ok(body) = BodyWithCall::abi_decode(&mut &body[1..], true) {
        body.into()
    } else {
        Err(anyhow!("Token Gateway: Failed to decode request body"))?
    };

    // RequestBody contains:
    // - amount: U256 (in source chain's ERC20 precision)
    // - asset_id: 32-byte keccak256(symbol) hash
    // - redeem: bool
    // - from: 32-byte sender address
    // - to: 32-byte recipient address
    // - data: Option<Vec<u8>> (optional calldata)
```

**Step 3: Lookup Local Asset**

```rust
    // Map gateway asset ID (keccak256 hash) to local asset ID
    let local_asset_id = LocalAssets::<T>::get(H256::from(body.asset_id.0))
        .ok_or_else(|| ismp::error::Error::ModuleDispatchError {
            msg: "Token Gateway: Unknown asset".to_string(),
            meta: Meta { source, dest, nonce },
        })?;

    // Example: keccak256("tGBP") maps to local_id = 1
```

**Step 4: Precision Handling**

For assets like tGBP with matching decimals (18 on both chains):

```rust
    // Get local decimals
    let decimals = if local_asset_id == T::NativeAssetId::get() {
        T::Decimals::get() // e.g., 12 for XCAV
    } else {
        <T::Assets as fungibles::metadata::Inspect<T::AccountId>>::decimals(
            local_asset_id.clone(),
        ) // e.g., 18 for tGBP
    };

    // Get source (ERC20) decimals from storage
    let erc_decimals = Precisions::<T>::get(local_asset_id.clone(), source)
        .ok_or_else(|| anyhow!("Asset decimals not configured"))?;
    // e.g., 18 for tGBP on Ethereum

    // Convert using convert_to_balance()
    // This scales DOWN when erc_decimals > local_decimals
    // Formula: amount / 10^(erc_decimals - local_decimals)
    let amount = convert_to_balance(
        U256::from_big_endian(&body.amount.to_be_bytes::<32>()),
        erc_decimals,
        decimals,
    )?;

    // For tGBP (18 = 18): 100_000_000_000_000_000_000 stays the same
    // For XCAV (18 → 12): 100_000_000_000_000_000_000 / 10^6 = 100_000_000_000_000
```

**Step 5: Mint/Unlock Assets**

```rust
    let beneficiary: T::AccountId = body.to.0.into();

    if local_asset_id == T::NativeAssetId::get() {
        // Handling native currency (e.g., XCAV)
        let is_native = NativeAssets::<T>::get(T::NativeAssetId::get());

        if is_native {
            // CUSTODY MODEL: Unlock from pallet account
            <T as Config>::NativeCurrency::transfer(
                &Pallet::<T>::pallet_account(),
                &beneficiary,
                amount.into(),
                ExistenceRequirement::AllowDeath,
            )?;
        } else {
            // MINT MODEL: Increase total supply
            let imbalance = <T as Config>::NativeCurrency::issue(amount.into());
            <T as Config>::NativeCurrency::resolve_creating(&beneficiary, imbalance);
        }
    } else {
        // Handling other assets (like tGBP)
        let is_native = NativeAssets::<T>::get(local_asset_id.clone());

        if is_native {
            // Asset originates from Xcavate
            // CUSTODY MODEL: Unlock from pallet account
            <T as Config>::Assets::transfer(
                local_asset_id,
                &Pallet::<T>::pallet_account(),
                &beneficiary,
                amount.into(),
                Preservation::Expendable,
            )?;
        } else {
            // Asset is from another chain (like tGBP)
            // MINT MODEL: Create new tokens
            <T as Config>::Assets::mint_into(
                local_asset_id,
                &beneficiary,
                amount.into(),
            )?;
        }
    }
```

**Step 6: Execute Optional Call**

```rust
    // pallet_token_gateway: on_accept() continued

    // If calldata was included, execute it
    if let Some(call_data) = body.data {
        let substrate_data = SubstrateCalldata::decode(&mut &call_data.0[..])
            .map_err(|err| anyhow!("Calldata decode error: {err:?}"))?;

        // Determine origin for the call
        let origin = if let Some(signature) = substrate_data.signature {
            // Verify Ed25519, Sr25519, or ECDSA signature
            // ... signature verification logic ...
            beneficiary.clone()
        } else {
            if source.is_evm() {
                // Sender is EVM account - convert to Substrate account
                T::EvmToSubstrate::convert(H160::from_slice(&body.from[12..]))
            } else {
                // Sender is Substrate account
                body.from.0.into()
            }
        };

        // Execute the runtime call
        let runtime_call = T::RuntimeCall::decode(&mut &*substrate_data.runtime_call)
            .map_err(|err| anyhow!("RuntimeCall decode error: {err:?}"))?;
        runtime_call.dispatch(RawOrigin::Signed(origin.clone()).into())?;

        // Increment nonce to prevent replay attacks
        frame_system::Pallet::<T>::inc_account_nonce(origin);
    }
```

**Step 7: Emit Event**

```rust
    // pallet_token_gateway: on_accept() final step

    Self::deposit_event(Event::<T>::AssetReceived {
        beneficiary,
        amount: amount.into(),
        source,
    });

    Ok(())
}
```

---

## Xcavate → Ethereum: Sending Assets

Send assets from Xcavate to Ethereum (or other chains). This includes:
- Sending tGBP back to Ethereum (burns on Xcavate, unlocks on Ethereum)
- Sending XCAV to Ethereum (locks on Xcavate, mints wrapped on Ethereum)

### Overview

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
│ Destination Chain (Ethereum)                                │
│ 1. Token Gateway contract receives message                  │
│ 2. Verifies consensus proof via Hyperbridge                 │
│ 3. Mints/unlocks tokens to recipient                        │
└─────────────────────────────────────────────────────────────┘
```

---

## Sending Assets (teleport Extrinsic)

The `teleport` extrinsic locks or burns assets on Xcavate and sends a message to the destination chain.

### Parameters

```rust
pub struct TeleportParams<AssetId, Balance> {
    /// Local asset ID (0 for XCAV, 1+ for others like tGBP)
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
    /// true = unlock native ERC20 on destination
    /// false = mint wrapped ERC6160 on destination
    pub redeem: bool,
}
```

### Step-by-Step Flow

**Step 1: User Initiates Transfer**

```rust
// Example: Sending tGBP back to Ethereum
let params = TeleportParams {
    asset_id: 1, // tGBP
    destination: StateMachine::Evm(1), // Ethereum
    recepient: H256::from([
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, // padding
        0xd8, 0xda, 0x6b, 0xf2, 0x69, 0x64, // EVM address
        0xaf, 0x9d, 0x7e, 0xed, 0x9e, 0x03,
        0xe5, 0x34, 0x15, 0xd3, 0x7a, 0xa9,
        0x60, 0x45,
    ]),
    amount: 100_000_000_000_000_000_000, // 100 tGBP (18 decimals)
    timeout: 3600, // 1 hour
    token_gateway: ethereum_gateway_address,
    relayer_fee: 0,
    call_data: None,
    redeem: true, // Unlock native tGBP on Ethereum
};

TokenGateway::teleport(
    RuntimeOrigin::signed(alice),
    params
)?;
```

**Step 2: Asset Custody/Burning**

```rust
// pallet_token_gateway: teleport()

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
    // Handling other assets (tGBP, etc.)
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
        // Asset is bridged from another chain (like tGBP)
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

```rust
// pallet_token_gateway: teleport()

// Get local decimals
let local_decimals = if asset_id == NativeAssetId::get() {
    T::Decimals::get() // e.g., 12 for XCAV
} else {
    Assets::decimals(asset_id) // e.g., 18 for tGBP
};

// Get destination (ERC20) decimals from storage
let erc_decimals = Precisions::get(asset_id, destination)
    .ok_or(Error::AssetDecimalsNotFound)?;
// e.g., 18 for tGBP on Ethereum

// Convert amount using convert_to_erc20()
// This scales UP when erc_decimals > local_decimals
// Formula: amount * 10^(erc_decimals - local_decimals)
let converted_amount = convert_to_erc20(amount, erc_decimals, local_decimals);

// Example for tGBP (18 = 18): no scaling needed
// Example for XCAV (12 → 18): 100 * 10^6 = 100_000_000_000_000_000_000
```

**Step 4: Create Cross-Chain Message**

```rust
// pallet_token_gateway: teleport()

// Encode message body with ABI encoding
let body = Body {
    amount: converted_amount,        // U256 in ERC20 precision
    asset_id: gateway_asset_id.0.into(), // 32-byte keccak256(symbol)
    redeem: params.redeem,
    from: sender_account.into(),     // 32-byte source account
    to: params.recepient.into(),     // 32-byte destination account
};

// Prefix with handleIncomingAsset enum variant (0x00)
let mut encoded = vec![0];
encoded.extend_from_slice(&Body::abi_encode(&body));

// Create ISMP Post Request
let dispatch_post = DispatchPost {
    dest: params.destination,
    from: PALLET_TOKEN_GATEWAY_ID.to_vec(),
    to: params.token_gateway,
    timeout: params.timeout,
    body: encoded,  // Prefixed ABI-encoded body
};
```

**Step 5: Dispatch via ISMP**

```rust
// pallet_token_gateway: teleport()

// Send message through the dispatcher (Hyperbridge)
let metadata = FeeMetadata {
    payer: sender.clone(),
    fee: params.relayer_fee.into(),
};

let commitment = dispatcher
    .dispatch_request(DispatchRequest::Post(dispatch_post), metadata)
    .map_err(|_| Error::AssetTeleportError)?;

// Emit event with the commitment hash
Self::deposit_event(Event::AssetTeleported {
    from: sender,
    to: params.recepient,
    dest: params.destination,
    amount: params.amount,  // Original amount (local precision)
    commitment,             // H256 commitment hash
});
```

---

## Timeout Handling

### Overview

If a cross-chain message isn't delivered within the timeout period, the sender can recover their funds.

### On-Timeout Handler

```rust
// pallet_token_gateway: IsmpModule::on_timeout()

fn on_timeout(&self, request: Timeout) -> Result<(), anyhow::Error> {
    match request {
        Timeout::Request(Request::Post(PostRequest { body, source, dest, nonce, .. })) => {
            // Decode original request body
            let body: RequestBody = if let Ok(body) = Body::abi_decode(&mut &body[1..], true) {
                body.into()
            } else if let Ok(body) = BodyWithCall::abi_decode(&mut &body[1..], true) {
                body.into()
            } else {
                Err(anyhow!("Token Gateway: Failed to decode request body"))?
            };

            // Refund goes back to original sender (body.from, not body.to)
            let beneficiary: T::AccountId = body.from.0.into();
            let local_asset_id = LocalAssets::<T>::get(H256::from(body.asset_id.0))?;

            // Get decimals - note: we use 'dest' here (the failed destination)
            let decimals = if local_asset_id == T::NativeAssetId::get() {
                T::Decimals::get()
            } else {
                <T::Assets as fungibles::metadata::Inspect<T::AccountId>>::decimals(
                    local_asset_id.clone(),
                )
            };
            let erc_decimals = Precisions::<T>::get(local_asset_id.clone(), dest)?;

            // Convert amount back to local precision
            let amount = convert_to_balance(
                U256::from_big_endian(&body.amount.to_be_bytes::<32>()),
                erc_decimals,
                decimals,
            )?;

            // Unlock/mint assets back to sender (same logic as on_accept)
            // ... custody model: unlock from pallet account
            // ... mint model: mint new tokens

            Pallet::<T>::deposit_event(Event::<T>::AssetRefunded {
                beneficiary,
                amount: amount.into(),
                source: dest, // Note: 'source' field is the original destination that timed out
            });
        }
        // ... handle other timeout types with errors
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
const balance = await api.query.Assets.Account.getValue(1, aliceAddress) // Asset 1 = tGBP
console.log(`Alice's tGBP balance: ${balance?.balance ?? 0}`)
```

### On Ethereum

```javascript
// Watch for AssetTeleported event
tokenGateway.on('AssetTeleported', (from, to, dest, amount, commitment, assetId, redeem, event) => {
    console.log(`
        Transfer initiated!
        From: ${from}
        To: ${to}
        Amount: ${ethers.formatUnits(amount, 18)} // 18 decimals for tGBP
        Commitment: ${commitment}
        Block: ${event.blockNumber}
    `);
});
```

---

## Next Steps

- **Understand custody models:** [Custody & Precision](./CUSTODY_AND_PRECISION.md)
- **See working examples:** [Examples & Troubleshooting](./EXAMPLES.md)
- **Learn about registration:** [Asset Registration](./ASSET_REGISTRATION.md)

[← Back to Index](./README.md)
