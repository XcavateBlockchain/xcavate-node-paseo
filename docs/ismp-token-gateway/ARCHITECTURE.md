# Architecture Overview

This document explains the architecture, components, and smart contracts involved in the Hyperbridge/ISMP token gateway integration with Xcavate.

**Related Documentation:**
- [← Back to Index](./README.md)
- [Asset Registration →](./ASSET_REGISTRATION.md)
- [Transfer Flows →](./TRANSFER_FLOWS.md)
- [Bridging ERC20 Guide →](./BRIDGING_ERC20_GUIDE.md)

---

## Components

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

---

## Key Concepts

### ISMP (Interoperable State Machine Protocol)

- Protocol for trustless cross-chain communication
- Uses consensus proofs to verify state across chains
- No trusted intermediaries or oracles required

**How it works:**
1. Source chain commits message hash to storage
2. Relayers observe commitment and wait for finalization
3. Consensus proof generated for the finalized state
4. Destination chain verifies proof and executes message

### Hyperbridge Coprocessor

- Provides enhanced security for cross-chain state verification
- Aggregates and verifies consensus proofs from multiple chains
- Acts as a hub for cross-chain communication

**Role in transfers:**
- Observes state commitments on source chains
- Generates cryptographic proofs of finalized states
- Coordinates relayer network for message delivery

### Token Gateway

- ISMP module for asset bridging
- Manages custody, minting, and burning of assets
- Maintains asset precision as registered on each chain

**Core functionality:**
- Asset registration with ERC6160 deployment
- Cross-chain teleport operations
- Decimal precision mapping per chain
- Custody management for native assets

See [Transfer Flows](./TRANSFER_FLOWS.md) for detailed transfer mechanics.

---

## Ethereum Smart Contracts

When interacting with Ethereum, you'll work with three main contracts deployed by the Polytope Labs team:

### 1. TokenGateway Contract

The main entry point for cross-chain asset transfers from Ethereum.

**Mainnet Address:** [`0xFd413e3AFe560182C4471F4d143A96d3e259B6dE`](https://etherscan.io/address/0xFd413e3AFe560182C4471F4d143A96d3e259B6dE)

**Sepolia Testnet:** [`0xFcDa26cA021d5535C3059547390E6cCd8De7acA6`](https://sepolia.etherscan.io/address/0xFcDa26cA021d5535C3059547390E6cCd8De7acA6)

#### Key Functions

**`teleport(TeleportParams memory params)`**
Send assets from Ethereum to other chains.

**`erc20(bytes32 assetId) → address`**
Get the native ERC20 address for an asset ID. Returns the original token address for bridged tokens.

**`erc6160(bytes32 assetId) → address`**
Get the ERC6160 wrapper address for an asset. Returns the wrapped token contract for bridged assets.

#### Storage

- Maintains custody of native ERC20 tokens being bridged
- Manages ERC6160 token deployments for bridged assets
- Stores asset registrations and mappings

#### Asset ID Generation

Asset IDs are deterministic:
```javascript
const assetId = ethers.keccak256(ethers.toUtf8Bytes('TGBP'));
// Result: 0x... (32-byte hash)
```

---

### 2. IsmpHost Contract

The ISMP protocol handler that manages message dispatch and delivery.

**Mainnet Address:** [`0x792A6236AF69787C40cF76b69B4c8c7B28c4cA20`](https://etherscan.io/address/0x792A6236AF69787C40cF76b69B4c8c7B28c4cA20)

**Sepolia Testnet:** [`0x2EdB74C269948b60ec1000040E104cef0eABaae8`](https://sepolia.etherscan.io/address/0x2EdB74C269948b60ec1000040E104cef0eABaae8)

#### Key Functions

**`dispatch(DispatchPost memory request) → bytes32`**
Send cross-chain messages. Returns the message commitment hash.

**`feeToken() → address`**
Returns the address of the ERC20 token used for paying message fees.

**`perByteFee(bytes memory destination) → uint256`**
Get the per-byte fee for sending messages to a specific destination chain.

#### Responsibilities

- Store message commitments on-chain
- Emit `PostRequestEvent` for relayers to observe
- Verify incoming consensus proofs
- Route messages to appropriate handlers (like TokenGateway)

---

### 3. HandlerV1 Contract

Processes incoming messages from other chains and handles timeouts.

**Mainnet Address:** [`0x6C84eDd2A018b1fe2Fc93a56066B5C60dA4E6D64`](https://etherscan.io/address/0x6C84eDd2A018b1fe2Fc93a56066B5C60dA4E6D64)

**Sepolia Testnet:** [`0x4638945E120846366cB7Abc08DB9c0766E3a663F`](https://sepolia.etherscan.io/address/0x4638945E120846366cB7Abc08DB9c0766E3a663F)

#### Key Functions

**`handlePostRequests(PostRequestMessage[] memory messages)`**
Process incoming messages from other chains. Verifies consensus proofs and executes token transfers.

**`handlePostRequestTimeouts(PostTimeout[] memory timeouts)`**
Handle message timeouts and issue refunds to original senders.

#### Responsibilities

- Verify consensus proofs from Hyperbridge
- Execute incoming token transfers
- Process timeout refunds for failed transfers
- Emit events for successful operations

---

## Fee Token & Payment

**Fee Token:** Hyperbridge uses a designated ERC20 token for paying cross-chain message fees.

### Querying the Fee Token

```javascript
const ismpHost = new ethers.Contract(ISMP_HOST_ADDRESS, ISMP_HOST_ABI, provider);
const feeTokenAddress = await ismpHost.feeToken();
console.log(`Fee token: ${feeTokenAddress}`);
```

### Fee Calculation

```javascript
// Get per-byte fee for Xcavate
const xcavateChainId = ethers.encodeBytes32String('PARA-4683');
const perByteFee = await ismpHost.perByteFee(xcavateChainId);

// Estimate message size (typically 200-300 bytes for token transfers)
const estimatedSize = 250;
const totalFee = perByteFee * BigInt(estimatedSize);

console.log(`Estimated fee: ${ethers.formatEther(totalFee)} tokens`);
```

### Payment Options

#### Option 1: Using Fee Token (Recommended)

1. Approve the TokenGateway to spend fee tokens
2. Call `teleport()` with appropriate parameters
3. Fee tokens are automatically deducted

```javascript
const feeToken = new ethers.Contract(feeTokenAddress, ERC20_ABI, wallet);
await feeToken.approve(TOKEN_GATEWAY_ADDRESS, totalFee);
```

#### Option 2: Using Native ETH

1. Pass `value` parameter with ETH amount when calling `teleport()`
2. Specify `nativeCost` in TeleportParams
3. Excess ETH is refunded

```javascript
await tokenGateway.teleport(teleportParams, {
    value: ethers.parseEther('0.01') // Pay with ETH
});
```

---

## Required Ethereum Contract Calls

Complete sequence for sending assets from Ethereum to Xcavate:

### Step 1: Approve ERC20 Token

If sending native ERC20 tokens like TGBP:

```javascript
const token = new ethers.Contract(TOKEN_ADDRESS, ERC20_ABI, wallet);
await token.approve(TOKEN_GATEWAY_ADDRESS, amount);
```

### Step 2: Approve Fee Token

```javascript
const feeToken = new ethers.Contract(feeTokenAddress, ERC20_ABI, wallet);
await feeToken.approve(TOKEN_GATEWAY_ADDRESS, estimatedFee);
```

### Step 3: Call Teleport

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

**See [Bridging ERC20 Guide](./BRIDGING_ERC20_GUIDE.md) for a complete example.**

---

## Xcavate Parachain Components

### pallet_token_gateway

The core pallet handling token bridging on Xcavate.

**Key Extrinsics:**
- `set_token_gateway_addresses` - Configure gateway addresses for each chain
- `create_erc6160_asset` - Register new assets for bridging
- `teleport` - Send assets to other chains
- `update_asset_precision` - Update decimal mappings

**Storage:**
- `SupportedAssets` - Maps local asset IDs to gateway asset IDs
- `LocalAssets` - Reverse mapping (gateway ID → local ID)
- `NativeAssets` - Flags whether asset originates from Xcavate
- `Precisions` - Decimal precision per chain
- `TokenGatewayAddresses` - Whitelisted gateway addresses

**See [Asset Registration](./ASSET_REGISTRATION.md) for registration details.**

### pallet_ismp

The ISMP protocol implementation for Substrate.

**Responsibilities:**
- Route messages to appropriate modules
- Verify consensus proofs from other chains
- Handle fee payment and accounting
- Manage message timeouts

**Integration:**
- `pallet_token_gateway` implements `IsmpModule` trait
- ISMP calls `on_accept()` when messages arrive
- ISMP calls `on_timeout()` for expired messages

---

## Next Steps

- **Register an asset:** See [Asset Registration](./ASSET_REGISTRATION.md)
- **Bridge an ERC20:** See [Bridging ERC20 Guide](./BRIDGING_ERC20_GUIDE.md)
- **Understand transfers:** See [Transfer Flows](./TRANSFER_FLOWS.md)
- **Technical details:** See [Technical Reference](./TECHNICAL_REFERENCE.md)

[← Back to Index](./README.md)
