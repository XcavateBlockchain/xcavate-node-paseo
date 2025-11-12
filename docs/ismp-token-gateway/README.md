# Token Gateway Documentation

Guide to token registration and cross-chain asset transfers using the Hyperbridge/ISMP stack integrated into Xcavate runtime.

## 📚 Documentation Index

### Getting Started

**→ [Bridging Existing ERC20 Tokens](./BRIDGING_ERC20_GUIDE.md)**
Complete step-by-step guide for bridging existing ERC20 tokens from Ethereum to Xcavate. Start here if you want to enable TGBP, DAI, or any other Ethereum token on Xcavate.

**Covers:**
- Prerequisites and setup
- Registration flow (Ethereum ↔ Xcavate)
- Verification steps
- User transfer flows
- Troubleshooting

---

### Core Documentation

#### [Architecture Overview](./ARCHITECTURE.md)
Understanding the components, contracts, and protocols involved.

**Topics:**
- System architecture diagrams
- Xcavate parachain components
- Ethereum smart contracts (TokenGateway, IsmpHost, HandlerV1)
- Fee tokens and payment mechanisms
- ISMP and Hyperbridge concepts

---

#### [Asset Registration](./ASSET_REGISTRATION.md)
How to register assets for cross-chain transfers.

**Topics:**
- Native vs bridged assets
- Registration flow on Xcavate
- Gateway address configuration
- ERC6160 deployment
- Storage mappings

---

#### [Transfer Flows](./TRANSFER_FLOWS.md)
Detailed mechanics of cross-chain asset transfers.

**Topics:**
- Teleport extrinsic (Xcavate → Ethereum)
- Receiving assets (Ethereum → Xcavate)
- Complete Ethereum → Xcavate flow with timeline
- Timeout handling and refunds
- Event monitoring

---

### Reference Documentation

#### [Technical Reference](./TECHNICAL_REFERENCE.md)
Deep dive into custody models, precision handling, and security.

**Topics:**
- Asset custody models (native vs bridged)
- Decimal precision mapping per chain
- Precision handling and consistency
- Security considerations
- Best practices

---

#### [Examples & Troubleshooting](./EXAMPLES.md)
Practical examples and solutions to common issues.

**Topics:**
- Example 1: Registering XCAV for Ethereum bridge
- Example 2: Sending XCAV to Ethereum
- Example 3: Receiving TGBP from Ethereum
- Example 4: Cross-chain contract calls
- Common issues and solutions
- Debugging checklist

---

## Quick Links

### I want to...

**Bridge an existing Ethereum token (TGBP, DAI, etc.) to Xcavate**
→ [Bridging ERC20 Guide](./BRIDGING_ERC20_GUIDE.md)

**Understand the architecture and smart contracts**
→ [Architecture Overview](./ARCHITECTURE.md)

**Register a new asset for bridging**
→ [Asset Registration](./ASSET_REGISTRATION.md)

**Learn how transfers work under the hood**
→ [Transfer Flows](./TRANSFER_FLOWS.md)

**Understand custody models and precision**
→ [Technical Reference](./TECHNICAL_REFERENCE.md)

**See working examples**
→ [Examples & Troubleshooting](./EXAMPLES.md)

---

## Key Concepts

### ISMP (Interoperable State Machine Protocol)
Protocol for trustless cross-chain communication using consensus proofs to verify state across chains without trusted intermediaries.

### Hyperbridge Coprocessor
Provides enhanced security for cross-chain state verification, aggregates and verifies consensus proofs from multiple chains.

### Token Gateway
ISMP module for asset bridging that manages custody, minting, and burning of assets. Each asset maintains its registered decimal precision per chain.

### Asset Custody Models
- **Native assets** (originating from Xcavate): Use custody model (lock/unlock)
- **Bridged assets** (from other chains): Use mint/burn model

---

## Contract Addresses

### Ethereum Mainnet
- **TokenGateway:** [`0xFd413e3AFe560182C4471F4d143A96d3e259B6dE`](https://etherscan.io/address/0xFd413e3AFe560182C4471F4d143A96d3e259B6dE)
- **IsmpHost:** [`0x792A6236AF69787C40cF76b69B4c8c7B28c4cA20`](https://etherscan.io/address/0x792A6236AF69787C40cF76b69B4c8c7B28c4cA20)
- **HandlerV1:** [`0x6C84eDd2A018b1fe2Fc93a56066B5C60dA4E6D64`](https://etherscan.io/address/0x6C84eDd2A018b1fe2Fc93a56066B5C60dA4E6D64)

### Sepolia Testnet
- **TokenGateway:** [`0xFcDa26cA021d5535C3059547390E6cCd8De7acA6`](https://sepolia.etherscan.io/address/0xFcDa26cA021d5535C3059547390E6cCd8De7acA6)
- **IsmpHost:** [`0x2EdB74C269948b60ec1000040E104cef0eABaae8`](https://sepolia.etherscan.io/address/0x2EdB74C269948b60ec1000040E104cef0eABaae8)
- **HandlerV1:** [`0x4638945E120846366cB7Abc08DB9c0766E3a663F`](https://sepolia.etherscan.io/address/0x4638945E120846366cB7Abc08DB9c0766E3a663F)

*Contract addresses provided by Polytope Labs. Find further documentation [here](https://docs.hyperbridge.network/developers/explore/configurations/mainnet).*

