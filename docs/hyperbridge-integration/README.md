# Hyperbridge Integration with Xcavate

This documentation describes how Xcavate integrates with [Hyperbridge](https://docs.hyperbridge.network) to enable trustless cross-chain token transfers between Ethereum and the Xcavate parachain.

## How to Use This Documentation

| Document | Purpose |
|----------|---------|
| **This README** | Overview of the integration and key concepts |
| [Bridging Guide](./BRIDGING_ERC20.md) | Step-by-step instructions for bridging tokens |
| [Technical Reference](./reference/README.md) | Deep-dive into architecture, custody models, and internals |

**Recommended reading order:**
1. Read this README for the big picture
2. Follow the [Bridging Guide](./BRIDGING_ERC20.md) for implementation
3. Consult the [Technical Reference](./reference/README.md) for debugging or advanced topics

---

## Overview

Traditional bridges rely on trusted intermediaries (multisigs, committees) which introduce security risks and single points of failure. Instead, Xcavate uses **Hyperbridge** and the **ISMP protocol** to create a fully trustless bridge where security is guaranteed by cryptographic proofs rather than trust assumptions.

**What this integration enables:**
- Bridge ERC-20 tokens (like tGBP) from Ethereum to Xcavate
- (Optionally) Transfer Xcavate-native assets back to Ethereum
- Maintain provably consistent token balances across chains
- Execute cross-chain transfers without trusted intermediaries

---

## Key Technologies

### ISMP (Interoperable State Machine Protocol)

[ISMP](https://docs.hyperbridge.network/protocol/ismp) is a cross-chain communication protocol that enables trustless message passing between blockchains. Instead of relying on trusted relayers or committees, ISMP uses **consensus proofs** to verify that a message was actually committed on the source chain.

**How it works:**
1. Source chain commits a message hash to its state
2. The message is relayed to the destination chain along with a cryptographic proof
3. Destination chain verifies the proof against the source chain's consensus
4. If valid, the message is executed

### Hyperbridge

[Hyperbridge](https://docs.hyperbridge.network) is a coprocessor parachain that aggregates and verifies consensus proofs from multiple chains. It acts as a cryptographic verification hub that:

- Tracks the consensus state of connected chains (Ethereum, Polkadot parachains, etc.)
- Generates and validates state proofs
- Coordinates the relayer network for message delivery

### Token Gateway

The **Token Gateway** is an ISMP application specifically designed for cross-chain asset transfers. It handles:

- **Asset registration** - Mapping tokens across chains with consistent identifiers
- **Custody management** - Locking/unlocking native assets, minting/burning bridged assets
- **Precision handling** - Managing decimal differences between chains

---

## How It Works

```
ETHEREUM                    HYPERBRIDGE                 XCAVATE
    |                           |                           |
    |  Lock ERC-20 ---------->  |  ---------------------->  |  Mint tokens
    |  in TokenGateway          |  Verify & relay proof     |  to recipient
```

1. **User locks tokens** on Ethereum via the TokenGateway contract
2. **Hyperbridge verifies** Ethereum consensus and generates a cryptographic proof
3. **Xcavate mints** equivalent tokens to the recipient's account

Transfers take approximately **20-30 minutes** due to Ethereum finalization time (~15 minutes) plus proof generation and relay.

> For detailed transfer mechanics, see [Transfer Flows](./reference/TRANSFER_FLOWS.md).

---

## Key Concepts

| Concept | Description | Learn More |
|---------|-------------|------------|
| **Asset ID** | `keccak256(symbol)` - deterministic identifier across all chains | [Asset Registration](./reference/ASSET_REGISTRATION.md) |
| **Native vs Bridged** | `native: false` = bridged asset using mint/burn model | [Custody & Precision](./reference/CUSTODY_AND_PRECISION.md) |
| **Precision** | Use same decimals on both chains (tGBP uses 18) | [Decimal Handling](./reference/CUSTODY_AND_PRECISION.md#decimal-precision-handling) |
| **Teleport** | The operation that transfers tokens across chains | [Bridging Guide](./BRIDGING_ERC20.md) |

---

## Contract Addresses

### Ethereum Mainnet

| Contract | Address |
|----------|---------|
| TokenGateway | [`0xFd413e3AFe560182C4471F4d143A96d3e259B6dE`](https://etherscan.io/address/0xFd413e3AFe560182C4471F4d143A96d3e259B6dE) |
| IsmpHost | [`0x792A6236AF69787C40cF76b69B4c8c7B28c4cA20`](https://etherscan.io/address/0x792A6236AF69787C40cF76b69B4c8c7B28c4cA20) |

### Sepolia Testnet

| Contract | Address |
|----------|---------|
| TokenGateway | [`0xFcDa26cA021d5535C3059547390E6cCd8De7acA6`](https://sepolia.etherscan.io/address/0xFcDa26cA021d5535C3059547390E6cCd8De7acA6) |
| IsmpHost | [`0x2EdB74C269948b60ec1000040E104cef0eABaae8`](https://sepolia.etherscan.io/address/0x2EdB74C269948b60ec1000040E104cef0eABaae8) |

> For all network contract addresses, see [Hyperbridge Contracts Documentation](https://docs.hyperbridge.network/developers/evm/contracts).

---

## Supported Tokens

| Network | Token | Address | Decimals | Asset ID |
|---------|-------|---------|----------|----------|
| Ethereum Mainnet | tGBP | [`0x27f6c8289550fCE67f6B50BeD1F519966aFE5287`](https://etherscan.io/address/0x27f6c8289550fCE67f6B50BeD1F519966aFE5287) | 18 | `0x99bb...c843` |
| Sepolia Testnet | WETH | [`0xfFf9976782d46CC05630D1f6eBAb18b2324d6B14`](https://sepolia.etherscan.io/address/0xfFf9976782d46CC05630D1f6eBAb18b2324d6B14) | 18 | `0x0f8a...b5b8` |

---

## Quick Reference

### For Governance: Register an ERC-20

```rust
// 1. Create asset on Xcavate
Assets::create(RuntimeOrigin::root(), 1, admin, 1)?;
Assets::set_metadata(origin, 1, b"Tokenised GBP".to_vec(), b"tGBP".to_vec(), 18)?;

// 2. Register with Token Gateway
TokenGateway::create_erc6160_asset(
    RuntimeOrigin::root(),
    AssetRegistration {
        local_id: 1,
        native: false,  // Bridged from Ethereum
        reg: GatewayAssetRegistration {
            symbol: b"tGBP".to_vec(),
            name: b"Tokenised GBP".to_vec(),
            chains: vec![StateMachine::Evm(1)],  // Ethereum Mainnet
            minimum_balance: Some(1),
        },
        precision: BTreeMap::from([(StateMachine::Evm(1), 18)]),
    }
)?;
```

> For Sepolia testnet, use `StateMachine::Evm(11155111)` instead of `Evm(1)`.
>
> See [Asset Registration](./reference/ASSET_REGISTRATION.md) for detailed guidance.

### For Users: Bridge Tokens

```javascript
// token = ERC-20 contract (e.g., tGBP at 0x27f6c8289550fCE67f6B50BeD1F519966aFE5287)

// 1. Approve TokenGateway to spend tokens
await token.approve(TOKEN_GATEWAY, amount);

// 2. Teleport to Xcavate
await tokenGateway.teleport({
    amount: amount,
    assetId: ethers.keccak256(ethers.toUtf8Bytes('tGBP')),
    to: recipientAccountId,  // 32-byte Substrate account
    dest: ethers.toUtf8Bytes('PASEO-4683'),
    timeout: 3600,
    relayerFee: 0,
    redeem: false,
    nativeCost: 0,
    data: '0x'
});
```

> See the [Bridging Guide](./BRIDGING_ERC20.md) for complete examples and parameter explanations.

---

## Resources

### Documentation

- [Bridging Guide](./BRIDGING_ERC20.md) - Complete walkthrough for bridging tokens
- [Technical Reference](./reference/README.md) - Architecture, custody models, and implementation details
  - [Architecture Overview](./reference/ARCHITECTURE.md) - System components and contracts
  - [Asset Registration](./reference/ASSET_REGISTRATION.md) - How to register new assets
  - [Transfer Flows](./reference/TRANSFER_FLOWS.md) - Detailed transfer mechanics
  - [Examples & Troubleshooting](./reference/EXAMPLES.md) - Common issues and solutions

### External Resources

- [Hyperbridge Documentation](https://docs.hyperbridge.network)
- [ISMP Protocol](https://docs.hyperbridge.network/protocol/ismp)
- [Hyperbridge Contracts](https://docs.hyperbridge.network/developers/evm/contracts)

### Testing

See `/integration-tests/` for Rust tests and Sepolia scripts that demonstrate the bridging flow.
