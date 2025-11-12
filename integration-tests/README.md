# Integration Tests

This crate contains integration tests for Xcavate's token gateway functionality, verifying cross-chain asset transfers via Hyperbridge/ISMP.

## What These Tests Do

These tests validate that Xcavate can correctly receive and process bridged assets from Ethereum through ISMP messages:

1. **Message Creation** - Mock ISMP messages are created with ABI-encoded bodies simulating Ethereum TokenGateway transfers
2. **Message Processing** - Messages are routed through the runtime's `pallet-token-gateway`
3. **Token Minting** - Bridged assets (like TGBP) are minted to recipient accounts using a mint/burn model
4. **Balance Tracking** - Balances accumulate correctly across multiple transfers
5. **Error Handling** - Invalid messages are rejected (unregistered assets, missing precision configs, etc.)

**Current Test Asset:** TGBP (Token Gateway British Pound) - 6 decimals on both Ethereum and Xcavate

## Design Philosophy

This crate is **intentionally NOT part of the workspace** to avoid introducing build delays for the node and runtime during development.

## Test Architecture

The tests are organized in two layers:

### Layer 1: Message Structure Tests (`tgbp_ismp.rs`)
Low-level tests that validate mock ISMP message creation without involving the runtime:
- Message structure correctness (source, destination, addresses)
- ABI encoding format validation
- Asset ID calculation (keccak256 hashing)
- Nonce uniqueness and sequencing
- Support for multiple source chains (Ethereum, BSC, etc.)
- Recipient account encoding (Substrate AccountId32 format)

### Layer 2: Runtime Integration Tests (`tgbp_integration_tests.rs`)
Full-stack tests that execute ISMP messages through the Xcavate runtime:
- Asset registration in token gateway storage
- Message routing to `pallet-token-gateway`
- ABI body decoding and processing
- Token minting (mint/burn model for bridged assets)
- Balance tracking and accumulation
- Event emission verification
- Error handling and validation
- Edge case coverage

## Test Results

**Test Status:** ✅ All 26 tests passing

### What's Working ✅

**Layer 1: Message Structure Tests (7 tests)**
- ✅ **Message structure validation** - Verifies ISMP PostRequest format (source, destination, addresses, body)
- ✅ **ABI encoding** - Validates body starts with 0x00 prefix and contains ABI-encoded data
- ✅ **Precision preservation** - Confirms TGBP maintains 6 decimals (no conversion needed)
- ✅ **Asset ID consistency** - keccak256 hashing produces consistent asset IDs
- ✅ **Unique nonces** - Sequential, monotonically increasing nonce generation
- ✅ **Multiple source chains** - Creates messages from different EVM chains (Ethereum, BSC)
- ✅ **Recipient encoding** - Correctly encodes Substrate AccountId32 as 32-byte format

**Layer 2: Runtime Integration Tests (19 tests passing)**
- ✅ **Asset registration** - Creates assets in pallet_assets and registers in token gateway storage
- ✅ **Storage mappings** - Bidirectional mapping between local asset ID and gateway asset ID
- ✅ **Basic transfer processing** - Full ISMP message flow: routing → decoding → minting
- ✅ **Balance accumulation** - Multiple transfers to same recipient add up correctly
- ✅ **Multiple recipients** - Independent balance tracking for different accounts
- ✅ **Decimal precision** - TGBP maintains 6 decimals throughout (1 TGBP = 1,000,000 units)
- ✅ **Event emission** - `Assets::Issued` events emitted on successful mints
- ✅ **Asset metadata** - Name, symbol, and decimals correctly stored and retrievable
- ✅ **Total supply tracking** - Issuance increases correctly with each mint
- ✅ **Multiple senders** - Same recipient can receive from different Ethereum addresses
- ✅ **Error: Unregistered assets** - Rejects transfers for assets not in token gateway
- ✅ **Error: Missing precision** - Fails gracefully when source chain precision not configured
- ✅ **Edge case: Minimum balance** - Handles small amounts correctly
- ✅ **Edge case: Zero amount** - Handles zero-value transfers without panicking

### Edge Cases Covered 🧪

1. **Missing precision configuration** - BSC transfer fails gracefully when precision not configured for that chain
2. **Unregistered assets** - Rejects transfers for assets not registered in token gateway
3. **Below minimum balance** - Handles dust amounts (e.g., 0.000999 TGBP) according to pallet_assets rules
4. **Zero amount transfers** - Validates zero-value transfers don't cause panics or state corruption
5. **Maximum amount** - Tests extreme values (u128::MAX) for message creation
6. **Multiple senders to same recipient** - Ensures no cross-contamination between sender contexts
7. **Minimum unit precision** - Tests smallest possible amount (1 in 6 decimals = 0.000001 TGBP)

## Testing Approach

### Phase 1: Mocked ISMP Messages (Current)

We test the Xcavate side with **mocked ISMP messages** that simulate what would be received from Ethereum via Hyperbridge. This is implemented in two complementary layers:

**Layer 1: Message Structure Tests** (`tgbp_ismp.rs`)
- Fast, lightweight tests that validate message creation utilities
- Verify ABI encoding, nonce generation, and asset ID calculation
- No runtime involvement - pure message validation
- Ideal for catching encoding bugs early

**Layer 2: Runtime Integration Tests** (`tgbp_integration_tests.rs`)
- Execute actual ISMP messages through the Xcavate runtime
- Test complete flow: message routing → body decoding → token minting
- Verify storage updates, balance changes, and event emission
- Validate error handling and edge cases with real runtime behavior

**Benefits of this approach:**
- Test token gateway logic without requiring live chain connections
- Validate precision preservation (TGBP maintains 6 decimals throughout)
- Rapid iteration on edge cases and error conditions
- Deterministic, reproducible test environment

### Phase 2: Full Integration (Future)

Later, we can add end-to-end tests similar to the [Hyperbridge SDK tests](https://github.com/polytope-labs/hyperbridge-sdk/blob/main/packages/sdk/src/tests/tokenGateway.test.ts):

- Connect to live testnets (Ethereum Sepolia, Xcavate testnet)
- Send actual transactions from Ethereum and wait for cross-chain delivery
- Use the Hyperbridge indexer to track message status and finality
- Verify round-trip flows: Ethereum → Xcavate → Ethereum

## File Structure

```
integration-tests/
├── src/
│   ├── lib.rs                           # Test setup and runtime configuration
│   ├── mock/
│   │   ├── mod.rs                       # Mock module exports
│   │   ├── ismp_messages.rs             # ISMP PostRequest message builders
│   │   └── test_accounts.rs             # Test Ethereum addresses & Substrate accounts
│   └── tests/
│       ├── mod.rs                       # Test module exports
│       ├── test_externalities.rs        # Runtime externalities setup
│       ├── tgbp_ismp.rs                 # Layer 1: Message structure tests
│       └── tgbp_integration_tests.rs    # Layer 2: Runtime integration tests
└── Cargo.toml                           # Not in workspace (intentional)
```

## Running Tests

Since this crate is not in the workspace:

```bash
cd integration-tests
cargo test
```

Or from the repository root:

```bash
cargo test --manifest-path integration-tests/Cargo.toml
```

## Test Coverage

### Current Focus: TGBP (Ethereum → Xcavate)

**What's Tested:**
- ✅ Asset registration with precision mapping (6 decimals on both Ethereum and Xcavate)
- ✅ Creating mock ISMP PostRequest messages with ABI encoding
- ✅ Processing messages through token gateway's `on_accept` callback
- ✅ Decimal precision preservation: 1,000,000 (6 dec) → 1,000,000 (6 dec) - no conversion
- ✅ Token minting to recipient accounts (mint/burn model for bridged assets)
- ✅ Balance accumulation across multiple transfers
- ✅ Event emission verification (`Assets::Issued`)
- ✅ Minimum balance enforcement via pallet_assets
- ✅ Invalid source chain precision rejection
- ✅ Unregistered asset rejection
- ✅ Total supply tracking for auditing

**Key Design Decision:**
TGBP maintains its native 6 decimal precision on both Ethereum and Xcavate. There is NO precision conversion - amounts are preserved exactly as they appear on the source chain. This simplifies accounting and reduces rounding errors.

### Future Test Scenarios

- 🔜 Outbound transfers: Xcavate → Ethereum (burn on Xcavate, unlock on Ethereum)
- 🔜 Timeout handling and message expiry
- 🔜 Native asset teleportation (lock/unlock model)
- 🔜 Full end-to-end tests with live testnet connections (Phase 2)

## References

- [Token Gateway Documentation](../docs/token-gateway/)
- [Integration Test Plan](../docs/token-gateway/INTEGRATION_TESTS_PLAN.md)
- [Hyperbridge SDK Tests](https://github.com/polytope-labs/hyperbridge-sdk/blob/main/packages/sdk/src/tests/tokenGateway.test.ts)
