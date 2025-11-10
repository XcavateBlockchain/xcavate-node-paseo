# Technical Reference

Deep dive into custody models, precision handling, and security considerations.

**Navigation:**
- [← Back to Index](./README.md)
- [← Transfer Flows](./TRANSFER_FLOWS.md)
- [Asset Registration](./ASSET_REGISTRATION.md)
- [Examples & Troubleshooting →](./EXAMPLES.md)

---

## Table of Contents

1. [Asset Custody Models](#asset-custody-models)
2. [Decimal Precision Handling](#decimal-precision-handling)
3. [Security Considerations](#security-considerations)
4. [Best Practices](#best-practices)

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

## Decimal Precision Handling

### The Decimal Problem

Different blockchains use different decimal precisions:
- **Substrate/Polkadot:** Typically 10-12 decimals
- **Ethereum ERC20:** Can vary (6, 18, etc.)
- **Some tokens:** May have different decimals on origin chain

### Precision Strategy

The Token Gateway maintains **consistent decimal precision** across chains for each asset:

**Example: TGBP**
- Native precision on Ethereum: **6 decimals**
- Precision on Xcavate: **6 decimals** (maintained)
- No conversion needed

**Example: XCAV**
- Native precision on Xcavate: **12 decimals**
- Precision on Ethereum (ERC6160): **18 decimals** (ERC20 standard)
- Conversion required: 12 → 18 when sending, 18 → 12 when receiving

### Precision Mapping

When registering an asset, specify its precision on each chain:

```rust
precision: BTreeMap::from([
    (StateMachine::Evm(1), 6),      // Ethereum: 6 decimals (TGBP)
    (StateMachine::Evm(137), 6),    // Polygon: 6 decimals
    (StateMachine::Kusama(4683), 6), // Xcavate: 6 decimals
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
    } else if remote_decimals < local_decimals {
        // Scale down (loses precision!)
        amount / 10^(local_decimals - remote_decimals)
    } else {
        // No conversion needed
        amount
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
    } else if local_decimals < remote_decimals {
        // Scale down (loses precision!)
        amount / 10^(remote_decimals - local_decimals)
    } else {
        // No conversion needed
        amount.try_into()
    }
}
```

### Example: TGBP (6 decimals on all chains)

**Sending 100 TGBP from Ethereum to Xcavate:**
```
Input:  100_000_000 (100 TGBP, 6 decimals)
Scale:  No conversion (6 decimals = 6 decimals)
Output: 100_000_000 (100 TGBP, 6 decimals)
```

**Receiving 50 TGBP from Ethereum:**
```
Input:  50_000_000 (50 TGBP, 6 decimals)
Scale:  No conversion (6 decimals = 6 decimals)
Output: 50_000_000 (50 TGBP, 6 decimals)
```

### Example: XCAV (12 → 18 decimals)

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

**Example: Hypothetical token with conversion 18 → 6**

```
Input:  1_000_001_500_000_000_000 (1.000001500000 TOKEN, 18 decimals)
Scale:  / 10^(18-6) = / 10^12
Output: 1_000_001 (1.000001 TOKEN, 6 decimals)
⚠️ Lost 0.000000500000 TOKEN due to precision truncation!
```

**Best Practice:** Maintain consistent decimals across chains to avoid conversion and potential precision loss.

---

## Security Considerations

### 1. Asset Origin Verification

Always ensure `native` flag is set correctly:
- **native: true** → Asset MUST originate from your chain
- **native: false** → Asset comes from elsewhere

Incorrect configuration can lead to:
- **Double-spending** (marking foreign asset as native)
- **Inability to redeem** (marking native asset as foreign)

**Verification:**
```rust
// Before registration, verify:
assert!(
    (is_native && asset_originates_here) ||
    (!is_native && asset_from_elsewhere),
    "Native flag mismatch"
);
```

### 2. Precision Configuration

- Always verify precision for each chain
- Test conversions with small amounts first
- Be aware of precision loss when scaling down
- Document precision decisions

**Testing:**
```rust
// Test round-trip conversion
let original = 1_000_000u128;
let converted = convert_to_erc20(original, 18, 12);
let back = convert_to_balance(converted, 18, 12)?;
assert_eq!(original, back, "Round-trip precision loss");
```

### 3. Timeout Periods

Choose timeout values considering:
- Source chain finalization time (~12 seconds for Polkadot, ~15 min for Ethereum)
- Hyperbridge processing time (~2-5 minutes)
- Destination chain confirmation time
- Relayer network latency

**Recommended timeouts:**
- **Parachain → Ethereum:** 3600 seconds (1 hour)
- **Ethereum → Parachain:** 3600 seconds (1 hour)
- **Parachain → Parachain:** 1800 seconds (30 minutes)

### 4. Gateway Address Whitelisting

Only register trusted gateway contracts:

```rust
// Verify contract addresses before registration
let verified_addresses = vec![
    // Ethereum mainnet TokenGateway (verified by Hyperbridge team)
    (StateMachine::Evm(1), hex!("Fd413e3AFe560182C4471F4d143A96d3e259B6dE")),
];

TokenGateway::set_token_gateway_addresses(
    RuntimeOrigin::root(),
    BTreeMap::from(verified_addresses)
)?;
```

**Best practices:**
- Verify contract addresses on block explorers
- Use multisig for `set_token_gateway_addresses`
- Monitor for unauthorized address changes
- Keep list of verified addresses in documentation

### 5. Relayer Fees

Setting `relayer_fee: 0` means:
- The dispatcher (Hyperbridge) will relay the message
- No additional cost but may be slower
- For important transfers, consider paying a fee

**Fee calculation:**
```rust
// Estimate relayer fee based on message size and priority
let message_size = 250; // bytes
let per_byte_fee = 1000; // fee tokens
let priority_multiplier = 2; // 2x for priority

let relayer_fee = message_size * per_byte_fee * priority_multiplier;
```

### 6. Existential Deposits

Ensure transfers exceed existential deposits on both chains:

```rust
// Check minimum balance requirements
let min_balance = Assets::minimum_balance(asset_id);
ensure!(amount >= min_balance, "Below minimum balance");

// Also check destination chain requirements
let dest_min = Precisions::get(asset_id, destination)
    .and_then(|decimals| get_dest_min_balance(decimals));
ensure!(converted_amount >= dest_min?, "Below destination minimum");
```

### 7. Reentrancy Protection

Token Gateway uses FRAME's built-in reentrancy protection, but be aware:

```rust
// Calls are protected by FRAME
#[pallet::call]
impl<T: Config> Pallet<T> {
    #[pallet::weight(...)]
    pub fn teleport(...) -> DispatchResult {
        // Automatically protected from reentrancy
        ...
    }
}
```

### 8. Event Monitoring

Monitor events for anomalies:

```rust
// Watch for unexpected patterns
match event {
    Event::AssetTeleported { amount, .. } if amount > threshold => {
        // Alert: Large transfer detected
    }
    Event::AssetRefunded { .. } => {
        // Alert: Timeout occurred
    }
    Event::AssetReceived { source, .. } if !approved_sources.contains(source) => {
        // Alert: Unexpected source
    }
    _ => {}
}
```

---

## Best Practices

### Development

1. **Test on Testnets First**
   - Use Sepolia for Ethereum testing
   - Use Paseo for parachain testing
   - Test with small amounts
   - Verify all edge cases

2. **Implement Proper Error Handling**
   ```rust
   // Don't panic, return errors
   let asset_id = LocalAssets::get(gateway_id)
       .ok_or(Error::AssetNotRegistered)?;
   ```

3. **Use Descriptive Names**
   ```rust
   // Good
   const TGBP_ASSET_ID: u32 = 1;
   const TGBP_DECIMALS: u8 = 6;

   // Bad
   const A1: u32 = 1;
   const D: u8 = 6;
   ```

4. **Document Precision Decisions**
   ```rust
   /// TGBP precision mapping:
   /// - Ethereum (native): 6 decimals
   /// - Xcavate: 6 decimals (matches native)
   /// - Polygon: 6 decimals (matches native)
   /// No conversion needed between chains
   ```

### Deployment

1. **Use Governance for Production**
   - All production registrations via governance
   - Use multisig for critical operations
   - Implement timelock for safety

2. **Verify Contract Addresses**
   ```javascript
   // Before registering, verify on Etherscan
   const tokenGateway = "0xFd413e...";
   const code = await provider.getCode(tokenGateway);
   assert(code.length > 2, "Contract not deployed");
   ```

3. **Monitor Initially**
   - Watch first few transfers closely
   - Verify amounts match expectations
   - Check event emissions
   - Monitor timeout rates

### Operations

1. **Set Appropriate Timeouts**
   - Account for worst-case finalization
   - Add buffer for network congestion
   - Document timeout reasoning

2. **Maintain Documentation**
   - Keep asset registry up to date
   - Document precision decisions
   - Track all registered gateway addresses
   - Maintain troubleshooting guides

3. **Plan for Upgrades**
   ```rust
   // Support version upgrades
   #[pallet::storage]
   pub type PalletVersion<T> = StorageValue<_, u16, ValueQuery>;

   #[pallet::hooks]
   impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
       fn on_runtime_upgrade() -> Weight {
           migrations::migrate_to_v2::<T>()
       }
   }
   ```

### Monitoring

1. **Track Key Metrics**
   - Transfer success rate
   - Average delivery time
   - Timeout rate
   - Precision conversion accuracy

2. **Set Up Alerts**
   - Large transfers (> threshold)
   - Failed transfers
   - Timeouts
   - Unexpected sources

3. **Regular Audits**
   - Review registered assets monthly
   - Verify precision configurations
   - Check gateway addresses
   - Audit custody balances

---

## Common Pitfalls

### 1. Wrong Native Flag

**Problem:** Marking a foreign asset as native

**Impact:** Custody model used incorrectly, potential loss of funds

**Solution:** Always verify asset origin before registration

### 2. Precision Mismatch

**Problem:** Incorrect decimal configuration

**Impact:** Wrong amounts transferred (10x, 100x off)

**Solution:** Double-check precision for each chain, test with small amounts

### 3. Insufficient Timeout

**Problem:** Timeout too short for chain finalization

**Impact:** Messages timeout unnecessarily, refunds triggered

**Solution:** Use recommended minimums (1 hour for Ethereum)

### 4. Missing Gateway Address

**Problem:** Forgot to register gateway address for a chain

**Impact:** Can't receive messages from that chain

**Solution:** Register addresses before asset registration

### 5. Below Existential Deposit

**Problem:** Transferring amount below minimum balance

**Impact:** Transfer fails or recipient can't receive

**Solution:** Check minimums on both chains before transferring

---

## Next Steps

- **See working examples:** [Examples & Troubleshooting](./EXAMPLES.md)
- **Learn about registration:** [Asset Registration](./ASSET_REGISTRATION.md)
- **Understand transfer flows:** [Transfer Flows](./TRANSFER_FLOWS.md)

[← Back to Index](./README.md)
