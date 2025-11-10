# Token Gateway: Registration and Transfer Flows

This document explains how token registration and cross-chain asset transfers work in the Hyperbridge/ISMP stack integrated into Xcavate runtime.

## Table of Contents
1. [Architecture Overview](#architecture-overview)
2. [Token Registration Flow](#token-registration-flow)
3. [Cross-Chain Transfer Flow](#cross-chain-transfer-flow)
4. [Ethereum → Xcavate: Complete Flow](#ethereum--xcavate-complete-flow)
5. [Asset Custody Models](#asset-custody-models)
6. [Precision Handling](#precision-handling)
7. [Practical Examples](#practical-examples)

---

## Architecture Overview

### Components

```
┌─────────────────────────────────────────────────────────────┐
│                     Xcavate Parachain                       │
│  ┌────────────────────────────────────────────────────────┐ │
│  │           pallet_token_gateway                         │ │
│  │  - Asset registration                                  │ │
│  │  - Teleport (send assets)                              │ │
│  │  - Receive assets (via ISMP)                           │ │
│  └────────────────┬───────────────────────────────────────┘ │
│                   │                                         │
│  ┌────────────────▼───────────────────────────────────────┐ │
│  │              pallet_ismp                               │ │
│  │  - Message routing                                     │ │
│  │  - Consensus verification                              │ │
│  │  - Fee handling                                        │ │
│  └────────────────┬───────────────────────────────────────┘ │
└───────────────────┼─────────────────────────────────────────┘
                    │
                    ▼
         ┌──────────────────────┐
         │   Hyperbridge (4009) │
         │   Coprocessor        │
         │  - State verification│
         │  - Proof aggregation │
         └──────────┬───────────┘
                    │
         ┌──────────▼───────────┐
         │  Other Chains        │
         │  - Polygon           │
         │  - BSC               │
         │  - Other parachains  │
         └──────────────────────┘

┌─────────────────────────────────────────────────────────────┐
│                    Ethereum Mainnet                         │
│                                                             │
│  User Wallet (EOA)                                          │
│      │                                                      │
│      └─► ERC20 Token Contracts                              │
│            │ (TGBP, DAI, etc.)                              │
│            │                                                │
│            ▼                                                │
│  ┌────────────────────────────────────────────────────┐     │
│  │         TokenGateway Contract                      │     │
│  │  - teleport(TeleportParams)                        │     │
│  │  - Asset custody/burning                           │     │
│  │  - ERC6160 token management                        │     │
│  └────────────────┬───────────────────────────────────┘     │
│                   │                                         │
│  ┌────────────────▼───────────────────────────────────┐     │
│  │         IsmpHost Contract                          │     │
│  │  - dispatch(DispatchRequest)                       │     │
│  │  - Store message commitments                       │     │
│  │  - Emit PostRequestEvent                           │     │
│  └────────────────┬───────────────────────────────────┘     │
│                   │                                         │
│  ┌────────────────▼───────────────────────────────────┐     │
│  │         HandlerV1 Contract                         │     │
│  │  - handlePostRequests() (receive from Xcavate)     │     │
│  │  - handlePostRequestTimeouts() (refunds)           │     │
│  │  - Verify consensus proofs                         │     │
│  └────────────────────────────────────────────────────┘     │
└─────────────────────────────────────────────────────────────┘
```

### Key Concepts

**ISMP (Interoperable State Machine Protocol):**
- Protocol for trustless cross-chain communication
- Uses consensus proofs to verify state across chains
- No trusted intermediaries or oracles required

**Hyperbridge Coprocessor:**
- Provides enhanced security for cross-chain state verification
- Aggregates and verifies consensus proofs from multiple chains
- Acts as a hub for cross-chain communication

**Token Gateway:**
- ISMP module for asset bridging
- Manages custody, minting, and burning of assets
- Handles precision conversion between chains

### Ethereum Smart Contracts

When interacting with Ethereum, you'll work with three main contracts deployed by the Hyperbridge team:

#### 1. **TokenGateway Contract**

The main entry point for cross-chain asset transfers from Ethereum.

**Mainnet Address:** [`0xFd413e3AFe560182C4471F4d143A96d3e259B6dE`](https://etherscan.io/address/0xFd413e3AFe560182C4471F4d143A96d3e259B6dE) *(Provided by Polytope)*
**Sepolia Testnet:** [`0xFcDa26cA021d5535C3059547390E6cCd8De7acA6`](https://sepolia.etherscan.io/address/0xFcDa26cA021d5535C3059547390E6cCd8De7acA6) *(Provided by Polytope)*

**Key Functions:**
- `teleport(TeleportParams memory params)` - Send assets to other chains
- `erc20(bytes32 assetId)` - Get native ERC20 address for an asset
- `erc6160(bytes32 assetId)` - Get ERC6160 wrapper address for an asset

**Storage:**
- Maintains custody of native ERC20 tokens being bridged
- Manages ERC6160 token deployments for bridged assets
- Stores asset registrations and mappings

#### 2. **IsmpHost Contract**

The ISMP protocol handler that manages message dispatch and delivery.

**Mainnet Address:** [`0x792A6236AF69787C40cF76b69B4c8c7B28c4cA20`](https://etherscan.io/address/0x792A6236AF69787C40cF76b69B4c8c7B28c4cA20) *(Provided by Polytope)*
**Sepolia Testnet:** [`0x2EdB74C269948b60ec1000040E104cef0eABaae8`](https://sepolia.etherscan.io/address/0x2EdB74C269948b60ec1000040E104cef0eABaae8) *(Provided by Polytope)*

**Key Functions:**
- `dispatch(DispatchPost memory request)` - Send cross-chain messages
- `feeToken()` - Returns the address of the token used for fees
- `perByteFee(bytes memory destination)` - Get per-byte fee for a destination chain

**Responsibilities:**
- Store message commitments on-chain
- Emit events for relayers to observe
- Verify incoming consensus proofs
- Route messages to appropriate handlers

#### 3. **HandlerV1 Contract**

Processes incoming messages from other chains and handles timeouts.

**Mainnet Address:** [`0x6C84eDd2A018b1fe2Fc93a56066B5C60dA4E6D64`](https://etherscan.io/address/0x6C84eDd2A018b1fe2Fc93a56066B5C60dA4E6D64) *(Provided by Polytope)*
**Sepolia Testnet:** [`0x4638945E120846366cB7Abc08DB9c0766E3a663F`](https://sepolia.etherscan.io/address/0x4638945E120846366cB7Abc08DB9c0766E3a663F) *(Provided by Polytope)*

**Key Functions:**
- `handlePostRequests(PostRequestMessage[] memory messages)` - Process incoming messages from other chains
- `handlePostRequestTimeouts(PostTimeout[] memory timeouts)` - Handle message timeouts and issue refunds

**Responsibilities:**
- Verify consensus proofs from Hyperbridge
- Execute incoming token transfers
- Process timeout refunds for failed transfers

### Fee Token & Payment

**Fee Token:** Hyperbridge uses a designated ERC20 token for paying cross-chain message fees.

To find the current fee token:
```javascript
const ismpHost = new ethers.Contract(ISMP_HOST_ADDRESS, ISMP_HOST_ABI, provider);
const feeTokenAddress = await ismpHost.feeToken();
console.log(`Fee token: ${feeTokenAddress}`);
```

**Fee Calculation:**
```javascript
// Get per-byte fee for Xcavate
const xcavateChainId = ethers.encodeBytes32String('PARA-4683');
const perByteFee = await ismpHost.perByteFee(xcavateChainId);

// Estimate message size (typically 200-300 bytes for token transfers)
const estimatedSize = 250;
const totalFee = perByteFee * BigInt(estimatedSize);

console.log(`Estimated fee: ${ethers.formatEther(totalFee)} tokens`);
```

**Payment Options:**

1. **Using Fee Token (Recommended):**
   - Approve the IsmpHost contract to spend fee tokens
   - Call `teleport()` with appropriate parameters
   - Fee tokens are automatically deducted

2. **Using Native ETH:**
   - Pass `value` parameter with ETH amount when calling `teleport()`
   - Specify `nativeCost` in TeleportParams
   - Excess ETH is refunded

**Important:** Always approve sufficient fee tokens before calling `teleport()`:
```javascript
const feeToken = new ethers.Contract(feeTokenAddress, ERC20_ABI, wallet);
await feeToken.approve(TOKEN_GATEWAY_ADDRESS, totalFee);
```

### Required Ethereum Contract Calls

Here's the complete sequence of calls needed to send assets from Ethereum to Xcavate:

#### Step 1: Approve ERC20 Token (if sending native tokens like TGBP)
```javascript
const token = new ethers.Contract(TOKEN_ADDRESS, ERC20_ABI, wallet);
await token.approve(TOKEN_GATEWAY_ADDRESS, amount);
```

#### Step 2: Approve Fee Token
```javascript
const feeToken = new ethers.Contract(feeTokenAddress, ERC20_ABI, wallet);
await feeToken.approve(TOKEN_GATEWAY_ADDRESS, estimatedFee);
```

#### Step 3: Call Teleport
```javascript
const tokenGateway = new ethers.Contract(TOKEN_GATEWAY_ADDRESS, TOKEN_GATEWAY_ABI, wallet);

const teleportParams = {
    amount: amount,              // Amount to send (in token decimals)
    relayerFee: 0,              // Optional relayer tip
    assetId: assetId,           // keccak256 of token symbol
    redeem: false,              // false for Substrate chains
    to: recipientBytes32,       // 32-byte recipient address
    dest: destinationChain,     // Encoded chain identifier
    timeout: 3600,              // Timeout in seconds
    nativeCost: 0,              // Or ETH amount if paying with ETH
    data: '0x'                  // Optional calldata
};

const tx = await tokenGateway.teleport(teleportParams);
await tx.wait();
```

---

## Token Registration Flow

### Overview

Before assets can be transferred cross-chain, they must be registered with the token gateway. This process creates the necessary mappings and deploys contracts on destination chains.

### Step-by-Step Registration

#### 1. **Register EVM Gateway Addresses**

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

#### 2. **Create Local Asset (if needed)**

For non-native assets, create them in `pallet_assets` first:

```rust
// Using pallet_assets
Assets::create(
    RuntimeOrigin::root(),
    asset_id: 1, // NOT 0 (reserved for XCAV)
    admin: treasury_account,
    min_balance: 1_000_000, // 0.000001 with 12 decimals
)?;

Assets::set_metadata(
    RuntimeOrigin::signed(admin),
    asset_id: 1,
    name: b"Generic Asset".to_vec(),
    symbol: b"ASSET".to_vec(),
    decimals: 12,
)?;
```

**Note:** Native XCAV uses `asset_id: 0` (configured as `NativeAssetId`)

#### 3. **Register Asset with Token Gateway**

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
        minimum_balance: 1_000_000,
    },

    // Precision per chain (CRITICAL for conversion)
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

#### 4. **What Happens During Registration**

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

## Cross-Chain Transfer Flow

### Sending Assets (Teleport)

#### Overview

The `teleport` extrinsic locks or burns assets on Xcavate and sends a message to the destination chain to mint or unlock them for the recipient.

#### Parameters

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

#### Step-by-Step Flow

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
        // This shouldn't happen for XCAV, but if it did:
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

**Step 3: Precision Conversion**

```rust
// Get source decimals
let source_decimals = if asset_id == NativeAssetId::get() {
    12 // XCAV decimals
} else {
    Assets::decimals(asset_id) // e.g., 12 for ASSET
};

// Get destination decimals
let dest_decimals = Precisions::get(asset_id, destination)
    .ok_or(Error::AssetDecimalsNotFound)?;
// e.g., 18 for ERC20

// Convert amount
let converted_amount = convert_to_erc20(
    amount,        // 100_000_000_000_000 (100 XCAV with 12 decimals)
    dest_decimals, // 18
    source_decimals, // 12
);
// Result: 100_000_000_000_000_000_000 (100 XCAV with 18 decimals)
```

**Conversion Formula:**
```
converted_amount = amount * 10^(dest_decimals - source_decimals)

Example:
  100 XCAV = 100_000_000_000_000 (12 decimals)
  Ethereum needs 18 decimals
  100_000_000_000_000 * 10^(18-12) = 100_000_000_000_000 * 10^6
  = 100_000_000_000_000_000_000 (18 decimals)
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
│ Ethereum (Destination)                                      │
│ 1. Token Gateway contract receives message                  │
│ 2. Verifies consensus proof via Hyperbridge                 │
│ 3. Mints/unlocks tokens to recipient                        │
└─────────────────────────────────────────────────────────────┘
```

### Receiving Assets

#### Overview

When assets are sent TO Xcavate from another chain, the token gateway's `on_accept` handler is invoked by ISMP.

#### Step-by-Step Flow

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

    // Example: gateway_id maps to local_id = 0 (XCAV)
```

**Step 4: Precision Conversion**

```rust
    // Get local decimals
    let decimals = if local_asset_id == NativeAssetId::get() {
        12 // XCAV
    } else {
        Assets::decimals(local_asset_id)
    };

    // Get source decimals
    let source_decimals = Precisions::get(local_asset_id, source)
        .ok_or("Asset decimals not configured")?;

    // Convert from source precision to local precision
    let amount = convert_to_balance(
        body.amount,      // 100_000_000_000_000_000_000 (18 decimals)
        source_decimals,  // 18
        decimals,         // 12
    )?;
    // Result: 100_000_000_000_000 (12 decimals)
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
        // Handling other assets
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
            // Asset is from another chain
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

### Timeout Handling

If a cross-chain message isn't delivered within the timeout period, the sender can recover their funds:

```rust
fn on_timeout(&self, request: Timeout) -> Result<(), anyhow::Error> {
    match request {
        Timeout::Request(Request::Post(post_request)) => {
            // Decode original request
            let body: RequestBody = Body::abi_decode(&post_request.body[1..])?;

            // Refund original sender
            let beneficiary = body.from.0.into();
            let local_asset_id = LocalAssets::get(H256::from(body.asset_id.0))?;

            // Convert amount back to local precision
            let amount = convert_to_balance(
                body.amount,
                erc_decimals,
                local_decimals,
            )?;

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

---

## Ethereum → Xcavate: Complete Flow

This section focuses specifically on receiving tokens FROM Ethereum (or other EVM chains) TO Xcavate.

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
│      ├─► Convert precision (18 decimals → 12 decimals)       │
│      ├─► Mint/Unlock tokens to recipient                     │
│      └─► Emit AssetReceived event                            │
│                                                              │
│  Recipient's balance updated ✓                               │
└──────────────────────────────────────────────────────────────┘
```

### Step-by-Step: User Sending from Ethereum

#### Prerequisites

1. **TokenGateway deployed on Ethereum** (by Hyperbridge team)
2. **Asset registered on both chains** (TGBP, XCAV, etc.)
3. **Xcavate gateway address registered** on Ethereum contract
4. **Sufficient ETH** for gas fees
5. **Sufficient fee tokens** (or ETH for native payment)

#### Step 1: Prepare the Transaction (Web3/Ethers.js)

```javascript
// Using ethers.js v6
import { ethers } from 'ethers';

// Connect to Ethereum
const provider = new ethers.JsonRpcProvider('https://eth-mainnet.g.alchemy.com/v2/YOUR_KEY');
const wallet = new ethers.Wallet(PRIVATE_KEY, provider);

// Token Gateway contract
const TOKEN_GATEWAY_ADDRESS = '0x...'; // Deployed by Hyperbridge
const tokenGateway = new ethers.Contract(
    TOKEN_GATEWAY_ADDRESS,
    TOKEN_GATEWAY_ABI,
    wallet
);

// Asset being sent (e.g., TGBP)
const TGBP_ADDRESS = '0x...'; // TGBP token address on Ethereum
const tgbp = new ethers.Contract(TGBP_ADDRESS, ERC20_ABI, wallet);

// Xcavate parachain identifier (Paseo)
const XCAVATE_CHAIN = ethers.encodeBytes32String('PARA-4683');

// Recipient address on Xcavate (32 bytes)
// If recipient is Alice: 0xd43593c715fdd31c61141abd04a99fd6822c8558854ccde39a5684e7a56da27d
const recipient = '0xd43593c715fdd31c61141abd04a99fd6822c8558854ccde39a5684e7a56da27d';

// Asset ID (keccak256 of symbol)
const assetId = ethers.keccak256(ethers.toUtf8Bytes('TGBP'));
// Result: 0x...

// Amount to send: 1000 TGBP (TGBP has 18 decimals on Ethereum)
const amount = ethers.parseUnits('1000', 18); // 18 decimals for ERC6160
```

#### Step 2: Approve Token Gateway (if sending ERC20)

```javascript
// Only needed if sending ERC20 (not needed for ERC6160)
// Check current allowance
const currentAllowance = await tgbp.allowance(wallet.address, TOKEN_GATEWAY_ADDRESS);

if (currentAllowance < amount) {
    console.log('Approving TGBP...');
    const approveTx = await tgbp.approve(TOKEN_GATEWAY_ADDRESS, amount);
    await approveTx.wait();
    console.log('Approved!');
}
```

#### Step 3: Prepare Teleport Parameters

```javascript
const teleportParams = {
    // Amount to send (18 decimals for ERC6160)
    amount: amount,

    // Relayer fee (in fee token, can be 0 if Hyperbridge relays)
    relayerFee: 0,

    // Asset identifier (keccak256 of symbol)
    assetId: assetId,

    // Redeem to native ERC20 on destination? (false for substrate)
    redeem: false,

    // Recipient address (32 bytes, left-padded if needed)
    to: recipient,

    // Destination chain (Xcavate on Paseo)
    dest: XCAVATE_CHAIN,

    // Timeout in seconds (1 hour = 3600)
    // Must account for:
    // - Ethereum finality (~15 min)
    // - Hyperbridge processing (~5 min)
    // - Xcavate block time (~12 sec)
    timeout: 3600,

    // Native token cost for dispatch (if paying with ETH)
    // Calculate based on message size
    nativeCost: ethers.parseEther('0.01'), // Example: 0.01 ETH

    // Optional: Calldata to execute on destination
    data: '0x' // Empty for simple transfer
};
```

#### Step 4: Calculate Fees

```javascript
// Get per-byte fee for destination
const ismpHost = await tokenGateway.params();
const perByteFee = await ethers.Contract(
    ismpHost.host,
    ISMP_HOST_ABI,
    provider
).perByteFee(XCAVATE_CHAIN);

// Estimate message size (body + headers)
const messageSize = 200; // Approximate bytes

// Total fee in fee token
const totalFee = (perByteFee * BigInt(messageSize)) + teleportParams.relayerFee;

console.log(`Fee required: ${ethers.formatEther(totalFee)} tokens`);
```

#### Step 5: Execute Teleport

```javascript
try {
    console.log('Sending teleport transaction...');

    // Call teleport with native token payment
    const tx = await tokenGateway.teleport(
        teleportParams,
        {
            value: teleportParams.nativeCost, // Pay dispatch fee with ETH
            gasLimit: 500000 // Set appropriate gas limit
        }
    );

    console.log(`Transaction sent: ${tx.hash}`);

    // Wait for confirmation
    const receipt = await tx.wait();
    console.log(`Confirmed in block ${receipt.blockNumber}`);

    // Extract commitment from event
    const event = receipt.logs.find(
        log => log.topics[0] === ethers.id('AssetTeleported(...)')
    );

    if (event) {
        const decoded = tokenGateway.interface.parseLog(event);
        console.log(`Commitment: ${decoded.args.commitment}`);
        console.log(`Amount sent: ${ethers.formatUnits(decoded.args.amount, 18)} TGBP`);
        console.log(`Destination: ${decoded.args.dest}`);
    }

} catch (error) {
    console.error('Transaction failed:', error);
}
```

### What Happens on Ethereum

#### Inside `TokenGateway.teleport()`

```solidity
function teleport(TeleportParams calldata teleportParams) public payable {
    // 1. Validate inputs
    require(teleportParams.to != bytes32(0), "Zero address");
    require(teleportParams.amount > 0, "Invalid amount");

    address _erc20 = _erc20s[teleportParams.assetId];
    address _erc6160 = _erc6160s[teleportParams.assetId];

    // 2. Custody or burn assets
    if (_erc20 != address(0) && !teleportParams.redeem) {
        // CUSTODY MODEL: Lock ERC20 in contract
        SafeERC20.safeTransferFrom(
            IERC20(_erc20),
            msg.sender,
            address(this),
            teleportParams.amount
        );
    } else if (_erc6160 != address(0)) {
        // BURN MODEL: Burn ERC6160 tokens
        IERC6160Ext20(_erc6160).burn(msg.sender, teleportParams.amount);
    }

    // 3. Encode message body
    bytes memory data = abi.encode(
        Body({
            from: addressToBytes32(msg.sender),
            to: teleportParams.to,
            amount: teleportParams.amount,
            assetId: teleportParams.assetId,
            redeem: teleportParams.redeem
        })
    );
    data = bytes.concat(hex"00", data); // Add enum variant

    // 4. Create ISMP dispatch request
    DispatchPost memory request = DispatchPost({
        dest: teleportParams.dest,
        to: abi.encodePacked(instance(teleportParams.dest)),
        body: data,
        timeout: teleportParams.timeout,
        fee: teleportParams.relayerFee,
        payer: msg.sender
    });

    // 5. Dispatch through IsmpHost
    bytes32 commitment = IIsmpHost(_params.host).dispatch{
        value: teleportParams.nativeCost
    }(request);

    // 6. Emit event for relayers
    emit AssetTeleported({
        from: msg.sender,
        to: teleportParams.to,
        dest: string(teleportParams.dest),
        assetId: teleportParams.assetId,
        amount: teleportParams.amount,
        redeem: teleportParams.redeem,
        commitment: commitment
    });
}
```

### Hyperbridge Processing

#### 1. **Observation Phase** (~5-15 minutes on Ethereum)

```
┌─────────────────────────────────────────────────────────────┐
│ Hyperbridge Relayer Network                                 │
│                                                             │
│ 1. Listen for AssetTeleported events on Ethereum            │
│ 2. Wait for block finalization (~15 min on Ethereum)        │
│ 3. Query Ethereum state commitment                          │
│ 4. Fetch full request data from IsmpHost                    │
└─────────────────────────────────────────────────────────────┘
```

**Why wait for finalization?**
- Ethereum requires ~32 epochs (2 epochs = ~13 minutes)
- Ensures the block won't be reorged
- Prevents double-spend attacks

#### 2. **Proof Generation Phase**

```
┌─────────────────────────────────────────────────────────────┐
│ Hyperbridge Coprocessor                                     │
│                                                             │
│ 1. Receive finalized Ethereum state root                    │
│ 2. Generate Merkle proof of:                                │
│    - Transaction inclusion                                  │
│    - State transition                                       │
│    - Request commitment                                     │
│ 3. Sign proof with Hyperbridge validators                   │
│ 4. Package: message + proof + metadata                      │
└─────────────────────────────────────────────────────────────┘
```

**Proof Contents:**
- Ethereum block header
- State root
- Merkle proof path
- Request commitment
- Validator signatures

#### 3. **Relay Phase** (~1-2 minutes)

```
┌─────────────────────────────────────────────────────────────┐
│ Relayer submits to Xcavate                                  │
│                                                             │
│ pallet_ismp::submit_messages()                              │
│     │                                                       │
│     ├─► Verify Hyperbridge proof                            │
│     ├─► Check consensus (BLS signatures)                    │
│     ├─► Extract message body                                │
│     └─► Route to pallet_token_gateway                       │
└─────────────────────────────────────────────────────────────┘
```

### Reception on Xcavate

#### Message Verification

```rust
// Called automatically by pallet_ismp
impl IsmpModule for Pallet<T> {
    fn on_accept(&self, request: PostRequest) -> Result<(), anyhow::Error> {
        // 1. Verify source
        let expected_gateway = TokenGatewayAddresses::<T>::get(request.source)
            .ok_or("Not configured to receive from source")?;

        ensure!(
            request.from == expected_gateway,
            "Unknown source contract"
        );

        // Source: Ethereum (StateMachine::Evm(1))
        // From: 0x... (Ethereum TokenGateway address)
        // ✓ Verified!
```

#### Message Decoding

```rust
        // 2. Decode ABI-encoded body
        let body: RequestBody = Body::abi_decode(&request.body[1..])?;

        // body now contains:
        // - amount: 1000000000000000000000 (1000 TGBP with 18 decimals)
        // - assetId: 0x... (keccak256("TGBP"))
        // - redeem: false
        // - from: 0x... (Ethereum sender, left-padded to 32 bytes)
        // - to: 0xd43593... (Alice's AccountId on Xcavate)
```

#### Asset Lookup

```rust
        // 3. Map gateway asset ID to local asset ID
        let local_asset_id = LocalAssets::<T>::get(H256::from(body.asset_id.0))
            .ok_or("Unknown asset")?;

        // Example: gateway_id for "TGBP" → local_id = 1
        // (Asset 0 is reserved for XCAV)
```

#### Precision Conversion

```rust
        // 4. Get decimals
        let local_decimals = Assets::decimals(local_asset_id); // 12 for TGBP on Xcavate
        let source_decimals = Precisions::<T>::get(local_asset_id, request.source)
            .ok_or("Asset decimals not configured")?; // 18

        // 5. Convert precision
        let amount = convert_to_balance(
            body.amount,      // 1_000_000_000_000_000_000_000 (18 decimals)
            source_decimals,  // 18
            local_decimals,   // 12
        )?;
        // Result: 1_000_000_000_000 (12 decimals = 1000 TGBP)
```

**Conversion:**
```
1_000_000_000_000_000_000_000 / 10^(18-12)
= 1_000_000_000_000_000_000_000 / 10^6
= 1_000_000_000_000 (1000.000000000000 TGBP)
```

#### Asset Disbursement

```rust
        // 6. Mint or unlock assets
        let beneficiary: AccountId = body.to.0.into(); // Alice

        let is_native = NativeAssets::<T>::get(local_asset_id);
        // For TGBP from Ethereum: is_native = false

        if !is_native {
            // MINT MODEL: Create new tokens
            Assets::mint_into(
                local_asset_id,  // Asset 1 (TGBP)
                &beneficiary,    // Alice
                amount,          // 1_000_000_000_000 (1000 TGBP)
            )?;
        }

        // 7. Emit event
        Self::deposit_event(Event::AssetReceived {
            beneficiary,
            amount,
            source: request.source, // Ethereum
        });

        Ok(())
    }
}
```

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

### Monitoring the Transfer

#### On Ethereum

```javascript
// Watch for AssetTeleported event
tokenGateway.on('AssetTeleported', (from, to, dest, amount, commitment, assetId, redeem, event) => {
    console.log(`
        Transfer initiated!
        From: ${from}
        To: ${to}
        Amount: ${ethers.formatUnits(amount, 18)}
        Commitment: ${commitment}
        Block: ${event.blockNumber}
    `);
});
```

#### On Hyperbridge

Check Hyperbridge explorer at `https://explorer.hyperbridge.network/`:
- Search by commitment hash
- View proof generation status
- See relay status

#### On Xcavate

```javascript
// Using Polkadot API
import { createClient } from "polkadot-api"
import { getWsProvider } from "polkadot-api/ws-provider/web"
import { xcavate } from "./xcavate-descriptors" // Generated chain descriptors

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

// Or query balance
const balance = await api.query.Assets.Account.getValue(1, aliceAddress) // Asset 1 = TGBP
console.log(`Alice's TGBP balance: ${balance?.balance ?? 0}`)

// Don't forget to unsubscribe and cleanup
// unsubscribe()
// client.destroy()
```

### Error Handling & Recovery

#### If Transfer Doesn't Arrive

**1. Check Ethereum Transaction Status**
```javascript
const receipt = await provider.getTransactionReceipt(txHash);
console.log('Status:', receipt.status === 1 ? 'Success' : 'Failed');
```

**2. Check Event Emission**
```javascript
const logs = receipt.logs.filter(
    log => log.address === TOKEN_GATEWAY_ADDRESS
);
console.log('Events emitted:', logs.length);
```

**3. Check Finalization**
```javascript
const blockNumber = receipt.blockNumber;
const currentBlock = await provider.getBlockNumber();
const confirmations = currentBlock - blockNumber;

console.log(`Confirmations: ${confirmations}`);
```

**4. Check Hyperbridge Status**
- Visit Hyperbridge explorer
- Search by commitment or transaction hash
- View processing status

**5. Check Timeout**
```rust
// On Xcavate, check if request timed out
// If timeout period passed without delivery, can request refund on Ethereum

// On Ethereum: Call HandlerV1.handlePostRequestTimeouts()
// This will refund your assets
```

#### Requesting Refund (After Timeout)

```javascript
// If timeout period expired and assets weren't delivered
// You can permissionlessly request refund

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

// TokenGateway will:
// - Unlock ERC20 (if custody model)
// - Mint ERC6160 (if burn model)
// - Return assets to original sender
```

### Security Considerations

#### 1. **Asset Verification**
Always verify the asset address before sending:
```javascript
const registeredAsset = await tokenGateway.erc6160(assetId);
console.log('ERC6160 address:', registeredAsset);

// Ensure it matches expected asset
if (registeredAsset === ethers.ZeroAddress) {
    throw new Error('Asset not registered!');
}
```

#### 2. **Recipient Address Format**
Ensure recipient address is correctly formatted:
```javascript
// Substrate address → 32 bytes
// Use polkadot-api's substrate bindings
import { AccountId } from "@polkadot-api/substrate-bindings"

const substrateAddress = 'xxx...'; // SS58 format
const bytes = AccountId().dec(substrateAddress)
const recipient = '0x' + Buffer.from(bytes).toString('hex');

// Verify length
if (recipient.length !== 66) { // 0x + 64 hex chars
    throw new Error('Invalid recipient address');
}
```

#### 3. **Sufficient Timeout**
Set appropriate timeout considering:
- Ethereum finality: ~15 minutes
- Network congestion: +5-10 minutes buffer
- **Recommended minimum: 3600 seconds (1 hour)**

#### 4. **Fee Payment**
Ensure sufficient fee token balance:
```javascript
const feeToken = await ismpHost.feeToken();
const balance = await ethers.Contract(feeToken, ERC20_ABI, provider)
    .balanceOf(wallet.address);

if (balance < totalFee) {
    console.error('Insufficient fee token balance');
    // Either:
    // 1. Buy more fee tokens
    // 2. Use nativeCost parameter to pay with ETH
}
```

### Common Issues & Solutions

| Issue | Cause | Solution |
|-------|-------|----------|
| Transaction reverts with "UnknownAsset" | Asset not registered on Ethereum | Register asset via governance |
| Transaction reverts with "Insufficient allowance" | ERC20 not approved | Call `approve()` first |
| Assets not arriving | Insufficient timeout | Wait longer or increase timeout |
| "Asset decimals not configured" on Xcavate | Missing precision mapping | Update precision via `update_asset_precision` |
| Wrong amount received | Precision conversion error | Verify precision config on both chains |
| Refund not working | Request not timed out yet | Wait for timeout period to pass |

---

## Asset Custody Models

### Native Assets (Custody Model)

**Definition:** Assets that originate from Xcavate

**Examples:**
- XCAV (native token)
- Real estate tokens created on Xcavate
- Any asset where Xcavate is the canonical source

**Behavior:**

**Sending (Teleport):**
```rust
// Lock assets in pallet account
transfer(sender → pallet_account, amount)
```

**Receiving:**
```rust
// Unlock assets from pallet account
transfer(pallet_account → recipient, amount)
```

**Key Characteristic:** Total supply never changes on Xcavate

**Why?**
- Xcavate is the source of truth for these assets
- Bridged representations on other chains are derivative
- Maintaining custody ensures assets can always be redeemed 1:1

### Bridged Assets (Mint/Burn Model)

**Definition:** Assets that originate from other chains

**Examples:**
- TGBP from Ethereum
- DAI from Ethereum
- Tokens from Polygon, BSC, etc.

**Behavior:**

**Receiving (from origin chain):**
```rust
// Mint new tokens
Assets::mint_into(asset_id, recipient, amount)
```

**Sending (back to origin):**
```rust
// Burn tokens
Assets::burn_from(asset_id, sender, amount)
```

**Key Characteristic:** Supply on Xcavate fluctuates based on bridged amount

**Why?**
- Xcavate doesn't have custody of the original assets
- The bridged tokens are representations backed by assets locked elsewhere
- Minting/burning ensures total cross-chain supply remains constant

### Configuration

The custody model is determined during asset registration:

```rust
AssetRegistration {
    local_id: 1,
    native: true,  // ← This determines the model
    // native: true → Custody Model
    // native: false → Mint/Burn Model
    ...
}
```

**Critical:** This flag must be set correctly based on where the asset truly originates!

---

## Precision Handling

### The Decimal Problem

Different blockchains use different decimal precisions:
- **Substrate/Polkadot:** Typically 10-12 decimals
- **Ethereum ERC20:** Always 18 decimals
- **Some tokens:** May have different decimals on origin chain

### Precision Mapping

When registering an asset, you must specify its precision on each chain:

```rust
precision: BTreeMap::from([
    (StateMachine::Evm(1), 18),     // Ethereum: 18 decimals
    (StateMachine::Evm(137), 18),   // Polygon: 18 decimals
    (StateMachine::Kusama(4683), 12), // Xcavate: 12 decimals
]),
```

### Conversion Functions

**Sending (Local → Remote):**
```rust
fn convert_to_erc20(
    amount: u128,           // Local amount
    remote_decimals: u8,    // Destination precision
    local_decimals: u8      // Source precision
) -> U256 {
    if remote_decimals > local_decimals {
        // Scale up
        amount * 10^(remote_decimals - local_decimals)
    } else {
        // Scale down (loses precision!)
        amount / 10^(local_decimals - remote_decimals)
    }
}
```

**Receiving (Remote → Local):**
```rust
fn convert_to_balance(
    amount: U256,           // Remote amount
    remote_decimals: u8,    // Source precision
    local_decimals: u8      // Destination precision
) -> Result<u128> {
    if local_decimals > remote_decimals {
        // Scale up
        amount * 10^(local_decimals - remote_decimals)
    } else {
        // Scale down
        amount / 10^(remote_decimals - local_decimals)
    }
}
```

### Example: XCAV (12 decimals) ↔ Ethereum (18 decimals)

**Sending 100 XCAV to Ethereum:**
```
Input:  100_000_000_000_000 (100 XCAV, 12 decimals)
Scale:  * 10^(18-12) = * 10^6
Output: 100_000_000_000_000_000_000 (100 XCAV, 18 decimals)
```

**Receiving 50 XCAV from Ethereum:**
```
Input:  50_000_000_000_000_000_000 (50 XCAV, 18 decimals)
Scale:  / 10^(18-12) = / 10^6
Output: 50_000_000_000_000 (50 XCAV, 12 decimals)
```

### Precision Loss Warning

⚠️ **Critical:** When converting from higher to lower precision, fractional amounts are lost!

**Example: Sending ASSET (6 decimals) to Ethereum (18 decimals)**

```
Input:  1_000_001 (1.000001 ASSET, 6 decimals)
Scale:  * 10^(18-6) = * 10^12
Output: 1_000_001_000_000_000_000 (1.000001 ASSET, 18 decimals)
✓ No precision loss
```

**Example: Receiving from Ethereum back to ASSET**

```
Input:  1_000_001_500_000_000_000 (1.0000015 ASSET, 18 decimals)
Scale:  / 10^(18-6) = / 10^12
Output: 1_000_001 (1.000001 ASSET, 6 decimals)
⚠️ Lost 0.0000005 ASSET due to precision truncation!
```

**Best Practice:** Always use equal or higher precision on remote chains than the origin chain.

---

## Practical Examples

### Example 1: Registering XCAV for Ethereum Bridge

```rust
// Step 1: Set up Ethereum gateway address
let addresses = BTreeMap::from([
    (StateMachine::Evm(1), hex!("1234...").to_vec()),
]);
TokenGateway::set_token_gateway_addresses(origin, addresses)?;

// Step 2: Register XCAV
let xcav_registration = AssetRegistration {
    local_id: 0, // NativeAssetId
    native: true, // XCAV originates from Xcavate
    reg: GatewayAssetRegistration {
        symbol: "XCAV".as_bytes().to_vec(),
        name: "Xcavate".as_bytes().to_vec(),
        chains: vec![StateMachine::Evm(1)],
        minimum_balance: 1_000_000_000, // 0.001 XCAV
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
- Gateway asset ID: `keccak256("XCAV")`
- Uses custody model (locks XCAV on Xcavate)

### Example 2: Sending 1000 XCAV to Ethereum

```rust
let params = TeleportParams {
    asset_id: 0, // XCAV
    destination: StateMachine::Evm(1),
    recepient: H256::from([
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, // 12 zero bytes
        0xAB, 0xCD, 0xEF, 0x12, 0x34, 0x56, // Ethereum address
        0x78, 0x90, 0xAB, 0xCD, 0xEF, 0x12,
        0x34, 0x56, 0x78, 0x90, 0xAB, 0xCD,
        0xEF, 0x12,
    ]),
    amount: 1_000_000_000_000_000, // 1000 XCAV (12 decimals)
    timeout: 7200, // 2 hours
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

**What Happens:**

1. **Xcavate:**
   - 1000 XCAV locked in `TokenGateway::pallet_account()`
   - Amount converted: `1_000_000_000_000_000 * 10^6 = 1_000_000_000_000_000_000_000`
   - Request dispatched to Hyperbridge

2. **Hyperbridge:**
   - Observes Xcavate state commitment
   - Generates consensus proof
   - Relayers pick up message

3. **Ethereum:**
   - Token Gateway contract receives message
   - Verifies proof via Hyperbridge
   - Mints 1000 XCAV (18 decimals) to recipient
   - Event emitted

### Example 3: Receiving TGBP from Ethereum

**Setup (one-time):**

```rust
// 1. Create TGBP asset on Xcavate
Assets::create(
    RuntimeOrigin::root(),
    asset_id: 1,
    admin: treasury,
    min_balance: 1_000_000_000, // 0.001 TGBP (12 decimals)
)?;

Assets::set_metadata(
    RuntimeOrigin::signed(admin),
    asset_id: 1,
    name: b"Tether Gold".to_vec(),
    symbol: b"TGBP".to_vec(),
    decimals: 12, // TGBP uses 12 decimals on Xcavate
)?;

// 2. Register with Token Gateway
let tgbp_registration = AssetRegistration {
    local_id: 1,
    native: false, // TGBP originates from Ethereum
    reg: GatewayAssetRegistration {
        symbol: "TGBP".as_bytes().to_vec(),
        name: "Tether Gold".as_bytes().to_vec(),
        chains: vec![StateMachine::Evm(1)],
        minimum_balance: 1_000_000_000,
    },
    precision: BTreeMap::from([
        (StateMachine::Evm(1), 18), // Bridged as ERC20 with 18 decimals
    ]),
};

TokenGateway::create_erc6160_asset(
    RuntimeOrigin::root(),
    tgbp_registration
)?;
```

**Receiving Transfer:**

1. **User sends on Ethereum:**
   - Calls Ethereum Token Gateway contract
   - Locks 500 TGBP (actually 500_000_000_000_000_000_000 with 18 decimals)
   - Message sent through Hyperbridge to Xcavate

2. **Xcavate receives:**
   - `on_accept` handler invoked by ISMP
   - Message decoded: 500_000_000_000_000_000_000 (18 decimals)
   - Converted to local precision: `500_000_000_000_000_000_000 / 10^6 = 500_000_000_000_000` (12 decimals)
   - 500 TGBP minted to recipient
   - Event emitted: `AssetReceived`

### Example 4: Cross-Chain Contract Call

You can execute arbitrary logic on the destination chain along with the asset transfer:

```rust
// Encode a pallet call
let call = RuntimeCall::Balances(
    pallet_balances::Call::transfer_keep_alive {
        dest: bob.into(),
        value: 100_000_000_000, // 100 XCAV
    }
);

let substrate_calldata = SubstrateCalldata {
    signature: None, // Or include signature for verification
    runtime_call: call.encode(),
};

let params = TeleportParams {
    asset_id: 0,
    destination: StateMachine::Kusama(4683), // Another parachain
    recepient: alice_h256,
    amount: 1000_000_000_000_000, // 1000 XCAV
    timeout: 3600,
    token_gateway: dest_gateway,
    relayer_fee: 0,
    call_data: Some(substrate_calldata.encode()),
    redeem: false,
};

TokenGateway::teleport(origin, params)?;
```

**Result:**
- 1000 XCAV transferred to Alice on destination
- 100 XCAV automatically transferred from Alice to Bob
- All atomic: either both succeed or both fail

---

## Security Considerations

### 1. Asset Origin Verification

Always ensure `native` flag is set correctly:
- **native: true** → Asset MUST originate from your chain
- **native: false** → Asset comes from elsewhere

Incorrect configuration can lead to:
- Double-spending (marking foreign asset as native)
- Inability to redeem (marking native asset as foreign)

### 2. Precision Configuration

- Always verify precision for each chain
- ERC20 tokens should use 18 decimals
- Test with small amounts first
- Be aware of precision loss when scaling down

### 3. Timeout Periods

Choose timeout values considering:
- Source chain finalization time (~12 seconds for Polkadot)
- Hyperbridge processing time
- Destination chain confirmation time
- Relayer network latency

**Recommended:** 3600 seconds (1 hour) for production

### 4. Gateway Address Whitelisting

Only register trusted gateway contracts:
- Verify contract addresses before registration
- Use multisig for `set_token_gateway_addresses`
- Monitor for unauthorized address changes

### 5. Relayer Fees

Setting `relayer_fee: 0` means:
- The dispatcher (Hyperbridge) will relay the message
- No additional cost but may be slower
- For important transfers, consider paying a fee

---

## Troubleshooting

### Transfer Not Arriving

1. **Check timeout:** Has it expired?
   ```rust
   // Check timeout status in pallet_ismp
   ```

2. **Verify gateway address:** Is destination registered?
   ```rust
   let gateway = TokenGatewayAddresses::<T>::get(destination);
   ```

3. **Check precision:** Is precision configured?
   ```rust
   let precision = Precisions::<T>::get(asset_id, destination);
   ```

4. **Monitor events:** Check for `AssetRefunded` events

### Asset Not Registered

1. **Check asset exists locally:**
   ```rust
   Assets::asset_exists(asset_id)
   ```

2. **Verify gateway asset ID:**
   ```rust
   let gateway_id = SupportedAssets::<T>::get(local_id);
   ```

3. **Check reverse mapping:**
   ```rust
   let local_id = LocalAssets::<T>::get(gateway_id);
   ```

### Insufficient Balance After Transfer

1. **Check precision conversion:**
   - Verify source and destination decimals
   - Calculate expected amount manually

2. **Check existential deposit:**
   - Transfers below minimum balance may be rejected
   - Ensure `amount > existential_deposit + fees`

---

## Summary

The Token Gateway provides a complete solution for cross-chain asset transfers:

1. **Registration:** One-time setup to enable assets for bridging
2. **Teleport:** Lock/burn assets and send cross-chain message
3. **Reception:** Verify message and mint/unlock assets
4. **Timeout:** Recover funds if delivery fails

Key concepts:
- **Custody Model:** For native assets (lock/unlock)
- **Mint/Burn Model:** For foreign assets (mint/burn)
- **Precision Handling:** Critical for correct amounts
- **ISMP Security:** Trustless verification via consensus proofs

With proper configuration and understanding of these flows, Xcavate can seamlessly bridge assets with any blockchain in the Hyperbridge network.
