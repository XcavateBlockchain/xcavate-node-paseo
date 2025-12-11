# Technical Reference Documentation

This folder contains detailed technical documentation for the Hyperbridge integration. For getting started, see the [main guide](../BRIDGING_ERC20.md).

## Primary Use Case

Receive ERC-20 tokens (like tGBP) from Ethereum on Xcavate using the ISMP Token Gateway.

## Recommended Reading Order

| # | Document | Description |
|---|----------|-------------|
| 1 | [Architecture Overview](./ARCHITECTURE.md) | System components, smart contracts, and how they interact |
| 2 | [Asset Registration](./ASSET_REGISTRATION.md) | Step-by-step guide to registering assets for cross-chain transfers |
| 3 | [Custody & Precision](./CUSTODY_AND_PRECISION.md) | Custody models, decimal precision handling, security considerations |
| 4 | [Transfer Flows](./TRANSFER_FLOWS.md) | Detailed mechanics of receiving and sending assets |
| 5 | [Redeem Flag Analysis](./REDEEM.md) | Technical analysis of the `redeem` parameter |
| 6 | [Examples & Troubleshooting](./EXAMPLES.md) | Practical examples and solutions to common issues |

## When to Use These Docs

**Use the main guide** ([BRIDGING_ERC20.md](../BRIDGING_ERC20.md)) for:
- Quick start instructions
- Basic bridging workflow
- Contract addresses

**Use the reference docs** for:
- Understanding the underlying architecture
- Debugging complex issues
- Implementing custom integrations
- Security audits and reviews

## Key Information

### Contract Addresses

See the [main README](../README.md) for current contract addresses on Ethereum Mainnet and Sepolia.

### Token Information

| Network | Token | Decimals | Asset ID |
|---------|-------|----------|----------|
| Ethereum Mainnet | tGBP | 18 | `0x99bb6e8574d7a5293a476638667ca3492c7e3f9ae2f5a47457f96c3c5c7fc843` |
| Sepolia Testnet | WETH | 18 | `0x0f8a193ff464434486c0daf7db2a895884365d2bc84ba47a68fcf89c1b14b5b8` |

Asset IDs are computed as `keccak256(symbol)` and are deterministic across all chains.

---

[← Back to Main Documentation](../README.md)
