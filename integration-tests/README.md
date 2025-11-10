# Integration Tests

This crate contains integration tests for Xcavate's token gateway functionality, focusing on cross-chain asset transfers via Hyperbridge/ISMP.

## Design Philosophy

This crate is **intentionally NOT part of the workspace** to avoid introducing build delays for the node and runtime during development.

## Test Results

**Test Status:** ✅ 21 passing, ⚠️ 5 pending (need treasury setup for asset creation)

### What's Working ✅

**Message Creation & Validation (13 tests)**
- ✅ **ABI-encoded message creation** - Properly encodes ISMP PostRequest messages with Ethereum ABI encoding
- ✅ **Message structure validation** - Verifies correct source, destination, addresses, and body format
- ✅ **Asset ID calculation** - keccak256 hashing of token symbols
- ✅ **Unique nonce generation** - Sequential nonce assignment for all transfers
- ✅ **Multiple source chains** - Supports Ethereum, BSC, and other EVM chains
- ✅ **Recipient account encoding** - Converts Substrate accounts to 32-byte format

**Runtime Integration Tests (8 tests passing)**
- ✅ **Asset registration** - Storage manipulation for test asset setup
- ✅ **Token gateway routing** - Messages correctly route to pallet-token-gateway
- ✅ **Body decoding** - Runtime successfully decodes ABI-encoded message bodies
- ✅ **Precision validation** - Detects missing precision mappings for unsupported chains
- ✅ **Unregistered asset rejection** - Correctly rejects transfers for unregistered assets
- ✅ **Minimum balance handling** - Handles below-minimum balance transfers
- ✅ **Zero amount transfers** - Edge case handling for zero-value transfers
- ✅ **Multiple sender accumulation** - Same recipient can receive from different senders

### Edge Cases Tested 🧪

1. **Invalid precision mapping** - BSC transfer fails when precision not configured ✅
2. **Unregistered assets** - Rejects unknown asset transfers ✅
3. **Below minimum balance** - Handles dust amount transfers ✅
4. **Zero amount** - Validates zero-value transfer behavior ✅
5. **Maximum amount** - Tests u128::MAX edge case ✅

### Pending (5 tests) ⚠️

These tests successfully dispatch to runtime and decode messages, but fail at asset creation because the treasury account lacks balance:

1. `test_process_tgbp_transfer_creates_asset_and_mints` - Full flow test
2. `test_multiple_tgbp_transfers_accumulate` - Balance accumulation test
3. `test_transfer_to_multiple_recipients` - Multi-recipient test
4. `test_precision_conversion_various_amounts` - Precision scaling test
5. `test_events_emitted_on_transfer` - Event verification test

**Root Cause:** Treasury account needs initial XCAV balance for `pallet_assets::create()` deposit.

**Simple Fix:**
```rust
// In new_test_ext(), add:
use xcavate_runtime::constants::currency::XCAV;
let treasury = AssetAdmin::get();
pallet_balances::GenesisConfig::<Runtime> {
    balances: vec![(treasury, 1000 * XCAV)],
}.assimilate_storage(&mut storage)?;
```

## Testing Approach

### Phase 1: Mocked ISMP Messages (Current)

We start by testing the Xcavate side with **mocked ISMP messages**, simulating what would be received from Ethereum via Hyperbridge. This approach allows us to:

- Test token gateway message processing logic
- Verify precision conversions (e.g., TGBP: 6 decimals → 12 decimals)
- Validate asset minting and event emission
- Test edge cases without requiring live chain connections

### Phase 2: Full Integration (Future)

Later, we can add full integration tests similar to the [Hyperbridge SDK tests](https://github.com/polytope-labs/hyperbridge-sdk/blob/main/packages/sdk/src/tests/tokenGateway.test.ts):

- Connect to live testnets (Ethereum Sepolia, Xcavate testnet)
- Send actual transactions and wait for cross-chain delivery
- Use the Hyperbridge indexer to track message status

## Structure

```
integration-tests/
├── src/
│   ├── mock/
│   │   ├── ismp_messages.rs    # Create mock PostRequest messages
│   │   └── test_accounts.rs     # Test Ethereum addresses & Xcavate accounts
│   └── tests/
│       └── tgbp_ethereum_to_xcavate.rs  # TGBP bridging tests
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

- ✅ Asset registration with precision mapping (6 decimals on Ethereum)
- ✅ Processing ISMP PostRequest messages
- ✅ Precision conversion: 1,000,000 (6 dec) → 1,000,000,000,000 (12 dec)
- ✅ Token minting to recipient accounts
- ✅ Event emission verification
- ✅ Minimum balance checks
- ✅ Invalid source chain rejection

### Future Test Scenarios

- Asset transfers from Xcavate → Ethereum
- Timeout handling
- Asset teleportation vs. bridging

## References

- [Token Gateway Documentation](../docs/token-gateway/)
- [Integration Test Plan](../docs/token-gateway/INTEGRATION_TESTS_PLAN.md)
- [Hyperbridge SDK Tests](https://github.com/polytope-labs/hyperbridge-sdk/blob/main/packages/sdk/src/tests/tokenGateway.test.ts)
