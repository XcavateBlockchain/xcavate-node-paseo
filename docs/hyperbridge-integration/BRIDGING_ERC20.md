# Bridging ERC-20 Tokens to Xcavate via Hyperbridge

Complete guide for bridging existing ERC-20 tokens from Ethereum to the Xcavate parachain using Hyperbridge and the ISMP protocol.

---

## Table of Contents

1. [Overview](#overview)
2. [Architecture](#architecture)
   - [System Diagram](#system-diagram)
   - [Xcavate Pallets](#xcavate-pallets)
   - [Ethereum Contracts](#ethereum-contracts)
   - [Hyperbridge Role](#hyperbridge-role)
3. [Asset Registration](#asset-registration)
   - [Prerequisites](#prerequisites)
   - [Step 1: Configure Gateway Addresses](#step-1-configure-gateway-addresses)
   - [Step 2: Create Local Asset](#step-2-create-local-asset)
   - [Step 3: Register with Token Gateway](#step-3-register-with-token-gateway)
   - [What Happens During Registration](#what-happens-during-registration)
   - [Verifying Registration](#verifying-registration)
4. [Transfer Flows](#transfer-flows)
   - [Ethereum → Xcavate](#ethereum--xcavate)
   - [Xcavate → Ethereum](#xcavate--ethereum)
   - [Timeline](#timeline)
5. [Working Example: tGBP](#working-example-tgbp)
6. [Working Example: USD.h (Sepolia Testnet)](#working-example-usdh-sepolia-testnet)
7. [Troubleshooting](#troubleshooting)
8. [Reference](#reference)

---

## Overview

**Use Case:** Bridge an existing ERC-20 token (e.g., tGBP) from Ethereum to Xcavate.

**How it works:**
1. **One-time setup:** Governance registers the asset on both chains
2. **User transfer:** User locks tokens on Ethereum → Hyperbridge verifies → Xcavate mints equivalent tokens
3. **Return transfer:** User burns on Xcavate → Hyperbridge verifies → Ethereum unlocks original tokens

**Transfer time:** ~20-30 minutes (primarily Ethereum finalization)

---

## Architecture

### System Diagram

```
┌─────────────────────────────────────────────────────────────────────────┐
│                          XCAVATE PARACHAIN                              │
│                                                                         │
│  ┌──────────────────────────────────────────────────────────────────┐   │
│  │                    pallet_token_gateway                          │   │
│  │  • Asset registration (create_erc6160_asset)                     │   │
│  │  • Outgoing transfers (teleport)                                 │   │
│  │  • Incoming transfers (on_accept callback from ISMP)             │   │
│  └────────────────────────────┬─────────────────────────────────────┘   │
│                               │                                         │
│  ┌────────────────────────────▼─────────────────────────────────────┐   │
│  │                       pallet_ismp                                │   │
│  │  • ISMP protocol implementation                                  │   │
│  │  • Consensus proof verification                                  │   │
│  │  • Message routing to modules                                    │   │
│  └────────────────────────────┬─────────────────────────────────────┘   │
│                               │                                         │
│  ┌────────────────────────────▼─────────────────────────────────────┐   │
│  │                      pallet_assets                               │   │
│  │  • Stores bridged token balances                                 │   │
│  │  • Mint/burn operations from token_gateway                       │   │
│  └──────────────────────────────────────────────────────────────────┘   │
└───────────────────────────────┬─────────────────────────────────────────┘
                                │
                    ┌───────────▼───────────┐
                    │      HYPERBRIDGE      │
                    │     (Para ID: 4009)   │
                    │                       │
                    │  • Consensus proofs   │
                    │  • State verification │
                    │  • Token Governor     │
                    │  • Relayer network    │
                    └───────────┬───────────┘
                                │
┌───────────────────────────────▼──────────────────────────────────────────┐
│                          ETHEREUM MAINNET                                │
│                                                                          │
│  ┌───────────────────────────────────────────────────────────────────┐   │
│  │                  TokenGateway Contract                            │   │
│  │  • teleport() - Lock ERC-20, dispatch cross-chain message         │   │
│  │  • onAccept() - Receive messages, unlock/mint tokens              │   │
│  │  • Custody of locked ERC-20 tokens                                │   │
│  └────────────────────────────┬──────────────────────────────────────┘   │
│                               │                                          │
│  ┌────────────────────────────▼──────────────────────────────────────┐   │
│  │                    IsmpHost Contract                              │   │
│  │  • dispatch() - Send ISMP messages                                │   │
│  │  • Store message commitments                                      │   │
│  └───────────────────────────────────────────────────────────────────┘   │
└──────────────────────────────────────────────────────────────────────────┘
```

### Xcavate Pallets

#### pallet_token_gateway

The core module for cross-chain asset bridging. Implements the `IsmpModule` trait to receive messages from ISMP.

**Key Extrinsics:**

| Extrinsic | Origin | Purpose |
|-----------|--------|---------|
| `set_token_gateway_addresses` | Root | Whitelist trusted gateway addresses per chain |
| `create_erc6160_asset` | Root | Register an asset for cross-chain bridging |
| `teleport` | Signed | Send assets to another chain |

**Storage Maps:**

| Storage | Key → Value | Purpose |
|---------|-------------|---------|
| `SupportedAssets` | LocalId → H256 | Maps local asset ID to gateway asset ID |
| `LocalAssets` | H256 → LocalId | Reverse mapping (gateway → local) |
| `NativeAssets` | LocalId → bool | Is asset native to this chain? |
| `Precisions` | (LocalId, Chain) → u8 | Decimals on each chain |
| `TokenGatewayAddresses` | Chain → Vec<u8> | Trusted gateway contract addresses |

#### pallet_ismp

The ISMP (Interoperable State Machine Protocol) implementation.

**Responsibilities:**
- Verify consensus proofs from source chains
- Route messages to destination modules (e.g., token_gateway)
- Handle timeouts and refunds

**Integration:** When ISMP receives a valid message for token_gateway, it calls:
- `on_accept()` - Process incoming asset transfer
- `on_timeout()` - Handle failed transfer, trigger refund

#### pallet_assets

Standard Substrate fungibles pallet. Token gateway has mint/burn privileges for registered bridged assets.

### Ethereum Contracts

#### TokenGateway

**Mainnet:** [`0xFd413e3AFe560182C4471F4d143A96d3e259B6dE`](https://etherscan.io/address/0xFd413e3AFe560182C4471F4d143A96d3e259B6dE)
**Sepolia:** [`0xFcDa26cA021d5535C3059547390E6cCd8De7acA6`](https://sepolia.etherscan.io/address/0xFcDa26cA021d5535C3059547390E6cCd8De7acA6)

**Key functions:**
- `teleport(TeleportParams)` - Lock ERC-20 and send cross-chain
- `onAccept(PostRequest)` - Process incoming messages
- `erc20(bytes32 assetId)` - Get registered ERC-20 address
- `erc6160(bytes32 assetId)` - Get ERC6160 wrapper address

**Storage:**
- `_erc20s[assetId]` - Original ERC-20 token address
- `_erc6160s[assetId]` - ERC6160 wrapper (deployed during registration)

#### IsmpHost

**Mainnet:** [`0x792A6236AF69787C40cF76b69B4c8c7B28c4cA20`](https://etherscan.io/address/0x792A6236AF69787C40cF76b69B4c8c7B28c4cA20)
**Sepolia:** [`0x2EdB74C269948b60ec1000040E104cef0eABaae8`](https://sepolia.etherscan.io/address/0x2EdB74C269948b60ec1000040E104cef0eABaae8)

Handles ISMP message dispatch and stores commitments.

### Hyperbridge Role

Hyperbridge is a Polkadot parachain that acts as a cross-chain verification hub:

1. **Consensus verification** - Verifies finality proofs from Ethereum
2. **State proofs** - Generates cryptographic proofs of finalized states
3. **Token Governor** - Coordinates asset registration across chains
4. **Relayer network** - Permissionless relayers deliver messages

Users don't interact directly with Hyperbridge - it operates transparently.

---

## Asset Registration

Before users can bridge tokens, governance must register the asset (one-time setup).

### Prerequisites

- Root/governance access on Xcavate
- ERC-20 token deployed on Ethereum with known: address, symbol, name, decimals

### Step 1: Configure Gateway Addresses

Register the Ethereum TokenGateway address:

```rust
// Extrinsic: tokenGateway.setTokenGatewayAddresses
TokenGateway::set_token_gateway_addresses(
    RuntimeOrigin::root(),
    BTreeMap::from([
        (
            StateMachine::Evm(1),  // Ethereum Mainnet
            hex!("Fd413e3AFe560182C4471F4d143A96d3e259B6dE").to_vec()
        ),
    ])
)?;
```

> **Testnet:** For Sepolia, use `StateMachine::Evm(11155111)` with gateway address `0xFcDa26cA021d5535C3059547390E6cCd8De7acA6`.

This whitelists the source - only messages from this address will be accepted.

### Step 2: Create Local Asset

Create the asset in `pallet_assets`:

```rust
// Extrinsic: assets.create
Assets::create(
    RuntimeOrigin::root(),
    1,                    // Asset ID (0 often reserved for native)
    treasury_account(),   // Admin
    1,                    // Minimum balance
)?;

// Extrinsic: assets.setMetadata
Assets::set_metadata(
    RuntimeOrigin::signed(admin),
    1,
    b"Tokenised GBP".to_vec(),
    b"tGBP".to_vec(),
    18,  // USE SAME DECIMALS AS ETHEREUM
)?;
```

**Important:** Use the **same decimals** as the source ERC-20 to avoid precision conversion issues.

### Step 3: Register with Token Gateway

```rust
// Extrinsic: tokenGateway.createErc6160Asset
TokenGateway::create_erc6160_asset(
    RuntimeOrigin::root(),
    AssetRegistration {
        local_id: 1,

        // CRITICAL: false because token originates from Ethereum
        // false = mint/burn model (for bridged assets)
        // true = custody model (for assets native to Xcavate)
        native: false,

        reg: GatewayAssetRegistration {
            symbol: b"tGBP".to_vec(),  // Must match ERC-20 exactly
            name: b"Tokenised GBP".to_vec(),
            chains: vec![StateMachine::Evm(1)],  // Ethereum Mainnet
            minimum_balance: None,
        },

        // ERC-20 decimals on each source chain
        precision: BTreeMap::from([
            (StateMachine::Evm(1), 18),  // tGBP has 18 decimals
        ]),
    }
)?;
```

> **Testnet:** For Sepolia, use `StateMachine::Evm(11155111)` instead of `Evm(1)`.

### What Happens During Registration

```
XCAVATE                           HYPERBRIDGE                      ETHEREUM
    │                                  │                               │
    │ create_erc6160_asset()           │                               │
    │ ├─ Store: SupportedAssets[1]     │                               │
    │ ├─ Store: LocalAssets[hash]      │                               │
    │ ├─ Store: NativeAssets[1]=false  │                               │
    │ ├─ Store: Precisions[1,Evm]=18   │                               │
    │ └─ Dispatch to Token Governor ──►│                               │
    │                                  │                               │
    │                                  │ Token Governor                │
    │                                  │ └─ Broadcast to Ethereum ────►│
    │                                  │                               │
    │                                  │                  handleCreateAsset()
    │                                  │                  ├─ Register ERC-20 addr
    │                                  │                  └─ Deploy ERC6160 wrapper
```

**Note on ERC6160:** The system deploys an ERC6160 wrapper on Ethereum (symbol "tGBP.h"), but for our use case (bridging existing ERC-20 TO Xcavate), users interact with the original ERC-20. The wrapper is used for reverse flows.

### Verifying Registration

**On Xcavate (Polkadot.js → Chain State):**
```
tokenGateway.supportedAssets(1) → Some(0x...)  // Gateway asset ID
tokenGateway.nativeAssets(1) → false           // Bridged, not native
tokenGateway.precisions(1, Evm(1)) → Some(18)  // 18 decimals on Mainnet
```

> **Testnet:** Query `tokenGateway.precisions(1, Evm(11155111))` for Sepolia.

**On Ethereum:**
```javascript
const assetId = ethers.keccak256(ethers.toUtf8Bytes('tGBP'));
const erc20 = await tokenGateway.erc20(assetId);    // ERC-20 address (if registered)
const erc6160 = await tokenGateway.erc6160(assetId); // ERC6160 address (if registered)
```

A token is considered registered on the TokenGateway if it has an address in either the `erc20()` or `erc6160()` mapping. The specific mapping used depends on the token type and how it was registered.

---

## Transfer Flows

### Ethereum → Xcavate

**User steps:**

1. **Approve TokenGateway:**
```javascript
await tgbp.approve(TOKEN_GATEWAY, amount);
```

2. **Call teleport:**
```javascript
await tokenGateway.teleport({
    amount: ethers.parseUnits('100', 18),
    relayerFee: 0,
    assetId: ethers.keccak256(ethers.toUtf8Bytes('tGBP')),
    redeem: false,
    to: recipientAccountId,  // 32-byte Substrate account
    dest: ethers.toUtf8Bytes('KUSAMA-4683'),
    timeout: 3600,
    nativeCost: 0,
    data: '0x'
});
```

**What happens:**
1. **Ethereum:** 100 tGBP locked in TokenGateway contract
2. **Hyperbridge:** Waits for finality (~15 min), generates proof
3. **Xcavate:** pallet_ismp verifies proof, token_gateway mints 100 tGBP to recipient

### Xcavate → Ethereum

```rust
// Extrinsic: tokenGateway.teleport
TokenGateway::teleport(
    RuntimeOrigin::signed(alice),
    TeleportParams {
        asset_id: 1,
        destination: StateMachine::Evm(1),  // Ethereum Mainnet
        recepient: H256::from_slice(&[/* Eth address padded to 32 bytes */]),
        amount: 100_000_000_000_000_000_000,  // 100 tGBP (18 decimals)
        timeout: 3600,
        token_gateway: hex!("Fd413e3AFe560182C4471F4d143A96d3e259B6dE").to_vec(),  // Mainnet
        relayer_fee: 0,
        call_data: None,
        redeem: true,  // Get original ERC-20 back
    }
)?;
```

> **Testnet:** For Sepolia, use `StateMachine::Evm(11155111)` with `token_gateway: hex!("FcDa26cA021d5535C3059547390E6cCd8De7acA6")`.

**What happens:**
1. **Xcavate:** Burns 100 tGBP from sender
2. **Hyperbridge:** Generates proof of Xcavate state
3. **Ethereum:** Unlocks 100 tGBP from custody to recipient

### Timeline

| Phase | Duration | What Happens |
|-------|----------|--------------|
| TX Confirmation | ~15 sec | Ethereum TX mined |
| Finalization | ~15 min | Ethereum reaches finality (~64 slots) |
| Proof Generation | ~2-5 min | Hyperbridge creates consensus proof |
| Relay | ~1-2 min | Relayer delivers to Xcavate |
| Processing | ~12 sec | Xcavate mints tokens |
| **Total** | **~20-30 min** | |

---

## Working Example: USD.h (Sepolia Testnet)

This example demonstrates a successful bridge transfer of 10 USD.h from Ethereum Sepolia to Xcavate testnet.

### Token Info

| Property | Sepolia Testnet |
|----------|-----------------|
| Contract | [`0xa801da100bf16d07f668f4a49e1f71fc54d05177`](https://sepolia.etherscan.io/address/0xa801da100bf16d07f668f4a49e1f71fc54d05177) |
| Symbol | USD.h |
| Decimals | 18 |
| Asset ID | `0x829f01563df2ff9752a529f62c33a4b03b805da1e1dfc748127d6d37795d7257` |

The Asset ID is computed as `keccak256("USD.h")`.

### Reference Transaction

A successful teleport transaction can be found here:
[`0x68b6f0e6850550fcc5100b50f01b83aa93d898609e749e08a2d4635c12752134`](https://sepolia.etherscan.io/tx/0x68b6f0e6850550fcc5100b50f01b83aa93d898609e749e08a2d4635c12752134)

### Teleport Parameters Used

| Parameter | Value | Description |
|-----------|-------|-------------|
| `amount` | `10000000000000000000` | 10 USD.h (18 decimals) |
| `relayerFee` | `0` | No relayer fee |
| `assetId` | `0x829f01563df2ff9752a529f62c33a4b03b805da1e1dfc748127d6d37795d7257` | keccak256("USD.h") |
| `redeem` | `false` | Mint on destination |
| `to` | `0x36185119e347676ff4eb6041ae90d638f6213cd471c53ced8a52ccb8fa84bc32` | Recipient (bytes32) |
| `dest` | `0x4b5553414d412d34363833` | "KUSAMA-4683" encoded |
| `timeout` | `3600` | 1 hour |
| `nativeCost` | `0` | No native cost |
| `data` | `0x` | Empty |

### Reproducing This Transfer

The transfer involves two operations:

1. **Approve** - Allow the TokenGateway contract to spend your USD.h tokens
2. **Teleport** - Call the TokenGateway to initiate the cross-chain transfer

```javascript
// 1. Approve TokenGateway to spend USD.h
const usdh = new ethers.Contract('0xa801da100bf16d07f668f4a49e1f71fc54d05177', ERC20_ABI, wallet);
await usdh.approve('0xFcDa26cA021d5535C3059547390E6cCd8De7acA6', ethers.parseUnits('10', 18));

// 2. Teleport tokens to Xcavate
const gateway = new ethers.Contract('0xFcDa26cA021d5535C3059547390E6cCd8De7acA6', GATEWAY_ABI, wallet);
await gateway.teleport({
    amount: ethers.parseUnits('10', 18),
    relayerFee: 0n,
    assetId: '0x829f01563df2ff9752a529f62c33a4b03b805da1e1dfc748127d6d37795d7257',
    redeem: false,
    to: recipientBytes32,
    dest: ethers.toUtf8Bytes('KUSAMA-4683'),
    timeout: 3600n,
    nativeCost: 0n,
    data: '0x'
});
```

> **Note:** The integration test script handles both operations automatically. To use it:
> ```bash
> cd integration-tests
> npm install
> npm run teleport              # Preview (no transaction)
> PRIVATE_KEY=0x... npm run teleport:execute  # Execute
> ```
> Edit the `CONFIG` object in `src/sepolia/teleport-erc20.js` to customize the amount or recipient.

---

## Working Example: tGBP

### Token Info

| Property | Ethereum Mainnet |
|----------|------------------|
| Contract | [`0x27f6c8289550fCE67f6B50BeD1F519966aFE5287`](https://etherscan.io/address/0x27f6c8289550fCE67f6B50BeD1F519966aFE5287) |
| Symbol | tGBP |
| Decimals | 18 |
| Asset ID | `0x99bb6e8574d7a5293a476638667ca3492c7e3f9ae2f5a47457f96c3c5c7fc843` |

The Asset ID is computed as `keccak256("tGBP")` and is used to identify the token across all chains.

> **Sepolia testnet:** Use WETH at [`0xfFf9976782d46CC05630D1f6eBAb18b2324d6B14`](https://sepolia.etherscan.io/address/0xfFf9976782d46CC05630D1f6eBAb18b2324d6B14) (18 decimals, Asset ID: `0x0f8a193ff464434486c0daf7db2a895884365d2bc84ba47a68fcf89c1b14b5b8`) for testing.

### Registration (Governance)
```rust
// 1. Whitelist gateway (Ethereum Mainnet)
TokenGateway::set_token_gateway_addresses(
    RuntimeOrigin::root(),
    BTreeMap::from([(StateMachine::Evm(1), hex!("Fd413e3AFe560182C4471F4d143A96d3e259B6dE").to_vec())])
)?;

// 2. Create asset
Assets::create(RuntimeOrigin::root(), 1, treasury(), 1)?;
Assets::set_metadata(RuntimeOrigin::signed(admin()), 1,
    b"Tokenised GBP".to_vec(), b"tGBP".to_vec(), 18)?;

// 3. Register
TokenGateway::create_erc6160_asset(RuntimeOrigin::root(), AssetRegistration {
    local_id: 1,
    native: false,
    reg: GatewayAssetRegistration {
        symbol: b"tGBP".to_vec(),
        name: b"Tokenised GBP".to_vec(),
        chains: vec![StateMachine::Evm(1)],  // Ethereum Mainnet
        minimum_balance: Some(1),
    },
    precision: BTreeMap::from([(StateMachine::Evm(1), 18)]),
})?;
```

> **Testnet:** For Sepolia, replace `Evm(1)` with `Evm(11155111)` and gateway address with `0xFcDa26cA021d5535C3059547390E6cCd8De7acA6`.

### User Bridge Script
```javascript
import { ethers } from 'ethers';

// Ethereum Mainnet configuration
const TGBP = '0x27f6c8289550fCE67f6B50BeD1F519966aFE5287';  // tGBP on mainnet
const GATEWAY = '0xFd413e3AFe560182C4471F4d143A96d3e259B6dE';  // Mainnet TokenGateway

// For Sepolia testnet (use WETH instead):
// const WETH = '0xfFf9976782d46CC05630D1f6eBAb18b2324d6B14';
// const GATEWAY = '0xFcDa26cA021d5535C3059547390E6cCd8De7acA6';

async function bridge(wallet, amount, recipient) {
    const tgbp = new ethers.Contract(TGBP, ['function approve(address,uint256)'], wallet);
    const gateway = new ethers.Contract(GATEWAY, [
        'function teleport((uint256,uint256,bytes32,bool,bytes32,bytes,uint64,uint256,bytes))'
    ], wallet);

    const wei = ethers.parseUnits(amount.toString(), 18);

    await (await tgbp.approve(GATEWAY, wei)).wait();

    const tx = await gateway.teleport({
        amount: wei,
        relayerFee: 0n,
        assetId: ethers.keccak256(ethers.toUtf8Bytes('tGBP')),
        redeem: false,
        to: recipient,  // 32-byte account ID
        dest: ethers.toUtf8Bytes('KUSAMA-4683'),
        timeout: 3600n,
        nativeCost: 0n,
        data: '0x'
    });

    console.log('TX:', (await tx.wait()).hash);
    console.log('Tokens arrive in ~20-30 min');
}
```

---

## Troubleshooting

| Error | Cause | Fix |
|-------|-------|-----|
| `Not configured to receive from source` | Gateway address not registered | `setTokenGatewayAddresses` |
| `Unknown source contract address` | Message from untrusted address | Verify gateway address |
| `Unknown asset` | Asset ID not in LocalAssets | `createErc6160Asset` |
| `Asset decimals not configured` | Missing precision mapping | Include in `precision` map |

**Transfer not arriving?**
1. Check Ethereum TX confirmed
2. Wait ~30 min for finality + relay
3. Verify registration: `tokenGateway.supportedAssets(id)`

**Wrong amount?** Decimal mismatch - ensure `Precisions` matches actual ERC-20 decimals.

---

## Reference

### Native vs Bridged Assets

| `native` | Model | Use For |
|----------|-------|---------|
| `true` | Custody (lock/unlock) | XCAV, Xcavate-native tokens |
| `false` | Mint/burn | Bridged ERC-20s |

**For ERC-20 bridging:** Always `native: false`

### Asset ID
```
Asset ID = keccak256(symbol)
Example: keccak256("tGBP") = 0x...
```
Same ID across all chains.

### Precision
When decimals match, amounts transfer 1:1. Recommended: use same decimals on both chains.

### Contract Addresses
| Network | TokenGateway | IsmpHost |
|---------|--------------|----------|
| Ethereum | `0xFd413e3AFe560182C4471F4d143A96d3e259B6dE` | `0x792A6236AF69787C40cF76b69B4c8c7B28c4cA20` |
| Sepolia | `0xFcDa26cA021d5535C3059547390E6cCd8De7acA6` | `0x2EdB74C269948b60ec1000040E104cef0eABaae8` |

### Resources
- [Hyperbridge Docs](https://docs.hyperbridge.network)
- [Token Gateway Guide](https://docs.hyperbridge.network/developers/evm/token-gateway)
- [ISMP Protocol](https://docs.hyperbridge.network/protocol/ismp)
