# Integration Tests

This crate contains integration tests for Xcavate's token gateway functionality, verifying cross-chain asset transfers via Hyperbridge/ISMP.

## Overview

The integration tests are split into two parts:

| Type | Language | Purpose |
|------|----------|---------|
| **Rust Tests** | Rust | Runtime logic validation with mocked ISMP messages |
| **Sepolia Scripts** | JavaScript | Live testnet interaction with Ethereum Sepolia |

## Quick Start

### Rust Tests
```bash
cd integration-tests
cargo test
```

### Sepolia Scripts
```bash
cd integration-tests
npm install
npm run teleport          # Prepare (no transaction)
npm run verify-registration
```

## Test Tokens

| Network | Token | Address | Asset ID |
|---------|-------|---------|----------|
| **Ethereum Mainnet** | tGBP | [`0x27f6c8289550fCE67f6B50BeD1F519966aFE5287`](https://etherscan.io/address/0x27f6c8289550fCE67f6B50BeD1F519966aFE5287) | `0x99bb6e8574d7a5293a476638667ca3492c7e3f9ae2f5a47457f96c3c5c7fc843` |
| **Sepolia Testnet** | USD.h | [`0xa801da100bf16d07f668f4a49e1f71fc54d05177`](https://sepolia.etherscan.io/address/0xa801da100bf16d07f668f4a49e1f71fc54d05177) | `0x829f01563df2ff9752a529f62c33a4b03b805da1e1dfc748127d6d37795d7257` |

Asset IDs are computed as `keccak256(symbol)` and are deterministic across all chains.

---

## Rust Tests

### Test Status: 27 tests passing

**Layer 1: Message Structure Tests** (`tgbp_ismp.rs`) - 8 tests
- Message structure validation (source, destination, addresses)
- ABI encoding format (0x00 prefix for Body)
- Asset ID calculation via keccak256
- Nonce uniqueness and sequencing
- Multiple source chains (Ethereum Mainnet, Sepolia, BSC)
- Recipient account encoding (AccountId32)
- Sepolia testnet message structure (WETH)

**Layer 2: Runtime Integration Tests** (`tgbp_integration_tests.rs`) - 13 tests
- Asset registration in token gateway storage
- Message routing to `pallet-token-gateway`
- ABI body decoding and processing
- Token minting (mint/burn model)
- Balance tracking and accumulation
- Event emission verification
- Error handling (unregistered assets, invalid precision)

**Mock Module Tests** - 6 tests
- Body encoding validation
- Asset ID calculation
- Nonce generation
- Test account uniqueness

### Chain Coverage

The Rust tests cover both mainnet and testnet scenarios:

```rust
// From src/mock/ismp_messages.rs
pub const ETHEREUM_MAINNET: StateMachine = StateMachine::Evm(1);
pub const ETHEREUM_SEPOLIA: StateMachine = StateMachine::Evm(11155111);
pub const XCAVATE_PARACHAIN: StateMachine = StateMachine::Kusama(4683);
```

### Key Design Decision

tGBP maintains **18 decimal precision** on both Ethereum and Xcavate. There is NO precision conversion - amounts are preserved exactly as they appear on the source chain.

---

## Sepolia Scripts

JavaScript scripts for live testnet interaction. See [`src/sepolia/README.md`](src/sepolia/README.md) for detailed documentation.

### Available Scripts

| Script | Command | Description |
|--------|---------|-------------|
| `teleport-erc20.js` | `npm run teleport` | Prepare USD.h bridge from Sepolia to Xcavate |
| `teleport-erc20.js --execute` | `npm run teleport:execute` | Execute actual bridge transaction |
| `verify-registration.js` | `npm run verify-registration` | Check if token is registered on TokenGateway |
| `calculate-asset-id.js` | `npm run calc-asset-id` | Calculate keccak256 asset ID for a symbol |

### Contract Addresses (Sepolia - Gargantua V3)

| Contract | Address |
|----------|---------|
| TokenGateway | [`0xFcDa26cA021d5535C3059547390E6cCd8De7acA6`](https://sepolia.etherscan.io/address/0xFcDa26cA021d5535C3059547390E6cCd8De7acA6) |
| IsmpHost | [`0x2EdB74C269948b60ec1000040E104cef0eABaae8`](https://sepolia.etherscan.io/address/0x2EdB74C269948b60ec1000040E104cef0eABaae8) |

### Example: Prepare Mode
```bash
$ npm run teleport

TELEPORT ERC-20 TOKENS: Ethereum -> Xcavate
================================================================================

Transfer Configuration:
  Token:       USD.h (0xa801da100bf16d07f668f4a49e1f71fc54d05177)
  Amount:      10 USD.h
  Destination: KUSAMA-4683
  Mode:        PREPARE (no transaction)
```

---

## File Structure

```
integration-tests/
├── Cargo.toml                           # Rust crate (NOT in workspace)
├── package.json                         # Node.js scripts
├── README.md                            # This file
└── src/
    ├── lib.rs                           # Test entry point
    ├── mock/
    │   ├── mod.rs                       # Mock module exports
    │   ├── ismp_messages.rs             # ISMP PostRequest builders
    │   └── test_accounts.rs             # Test accounts (Ethereum & Substrate)
    ├── tests/
    │   ├── mod.rs                       # Test module exports
    │   ├── test_externalities.rs        # Runtime externalities setup
    │   ├── tgbp_ismp.rs                 # Message structure tests
    │   └── tgbp_integration_tests.rs    # Runtime integration tests
    └── sepolia/
        ├── README.md                    # Sepolia scripts documentation
        ├── teleport-erc20.js            # Bridge tokens to Xcavate
        ├── verify-registration.js       # Check token registration
        ├── calculate-asset-id.js        # Calculate asset IDs
        ├── tokenGateway.address         # Contract address
        ├── tokenGateway.abi             # Contract ABI
        └── endpoints                    # RPC endpoint list
```

---

## Running Tests

### Rust Tests

Since this crate is intentionally NOT part of the workspace:

```bash
# From integration-tests directory
cd integration-tests
cargo test

# From repository root
cargo test --manifest-path integration-tests/Cargo.toml

# Run specific test
cargo test --manifest-path integration-tests/Cargo.toml sepolia_testnet_messages
```

### JavaScript Scripts

```bash
cd integration-tests
npm install

# Prepare (no transaction)
npm run teleport

# Execute with private key
PRIVATE_KEY=0x... npm run teleport:execute

# Verify token registration
npm run verify-registration

# Calculate asset ID
npm run calc-asset-id WETH
```

### Gas Estimation

Use [Foundry's `cast`](https://book.getfoundry.sh/cast/) to estimate gas costs before executing:

```bash
# Estimate gas for a teleport call
cast estimate 0xFcDa26cA021d5535C3059547390E6cCd8De7acA6 \
  "teleport((uint256,uint256,bytes32,bool,bytes32,bytes,uint64,uint256,bytes))" \
  "(1000000000000000,0,0x0f8a193ff464434486c0daf7db2a895884365d2bc84ba47a68fcf89c1b14b5b8,false,0xd43593c715fdd31c61141abd04a99fd6822c8558854ccde39a5684e7a56da27d,0x504153454f2d34363833,3600,0,0x)" \
  --rpc-url https://ethereum-sepolia-rpc.publicnode.com

# Simulate full transaction (requires private key)
cast send --simulate 0xFcDa26cA021d5535C3059547390E6cCd8De7acA6 \
  "teleport((uint256,uint256,bytes32,bool,bytes32,bytes,uint64,uint256,bytes))" \
  "(1000000000000000,0,0x0f8a193ff464434486c0daf7db2a895884365d2bc84ba47a68fcf89c1b14b5b8,false,0xd43593c715fdd31c61141abd04a99fd6822c8558854ccde39a5684e7a56da27d,0x504153454f2d34363833,3600,0,0x)" \
  --private-key $PRIVATE_KEY \
  --rpc-url https://ethereum-sepolia-rpc.publicnode.com
```

The tuple fields are: `(amount, relayerFee, assetId, redeem, to, dest, timeout, nativeCost, data)`

---

## Test Coverage

### Currently Tested

| Feature | Mainnet (tGBP) | Testnet (WETH) |
|---------|----------------|----------------|
| Asset registration | ✅ Rust tests | ✅ Rust tests |
| ISMP message creation | ✅ Rust tests | ✅ Rust tests |
| Message routing | ✅ Rust tests | ✅ Rust tests |
| Token minting | ✅ Rust tests | ✅ Rust tests |
| Balance accumulation | ✅ Rust tests | ✅ Rust tests |
| Live teleport | - | ✅ JS scripts |
| Registration verification | - | ✅ JS scripts |

### Future Test Scenarios

- Outbound transfers: Xcavate → Ethereum (burn on Xcavate, unlock on Ethereum)
- Timeout handling and message expiry
- Full end-to-end tests with live mainnet

---

## References

- [Token Gateway Documentation](../docs/ismp-token-gateway/)
- [Sepolia Scripts Documentation](src/sepolia/README.md)
- [Hyperbridge SDK Tests](https://github.com/polytope-labs/hyperbridge-sdk/blob/main/packages/sdk/src/tests/tokenGateway.test.ts)
- [Hyperbridge Contracts](https://docs.hyperbridge.network/developers/evm/contracts)
