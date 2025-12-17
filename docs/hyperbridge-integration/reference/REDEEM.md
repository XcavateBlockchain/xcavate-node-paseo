# Redeem Flag Analysis

When performing a teleport, one required parameter is whether the funds should be _redeemed_.
Below is a breakdown of what `redeem` means and how it affects the process.

The redeem flag controls what happens on BOTH the source and destination chains.

> **Source:** All code references below are from the [TokenGateway.sol](https://github.com/polytope-labs/hyperbridge/blob/main/evm/src/modules/TokenGateway.sol) contract.

---

## Part 1: Source Chain (Sending) - `teleport()` function

```solidity
// TokenGateway.sol - teleport()
Line 375: if (_erc20 != address(0) && !teleportParams.redeem)
```

**When `redeem = false`:**
- Condition met: If asset has an ERC20 address
- Action: Custody the ERC20 tokens (lock them in the TokenGateway contract)
- Meaning: You're sending the native ERC20 from its origin chain

**When `redeem = true`:**
- Condition NOT met: Skips the ERC20 custody branch
- Falls through to line [386-387](https://github.com/polytope-labs/hyperbridge/blob/e0c7e5d0e19f8a6ac97c766221907b3cf1db0409/evm/src/modules/TokenGateway.sol#L386-L387): Burns ERC6160 wrapper instead
- Meaning: You're burning wrapped tokens to redeem the native asset elsewhere

---

## Part 2: Destination Chain (Receiving) - `handleIncomingAsset()` function

```solidity
// TokenGateway.sol - handleIncomingAsset()
Line 553: if (_erc20 != address(0) && body.redeem)
```

**When `redeem = true`:**
- Condition met: If asset has an ERC20 address on destination chain
- Action: Transfer custodied ERC20 from the TokenGateway to the recipient
- Meaning: Recipient receives the native ERC20 token (not a wrapper)

**When `redeem = false`:**
- Condition NOT met: Skips the ERC20 transfer branch
- Falls through to line [556-557](https://github.com/polytope-labs/hyperbridge/blob/e0c7e5d0e19f8a6ac97c766221907b3cf1db0409/evm/src/modules/TokenGateway.sol#L556-L557): Mints ERC6160 wrapper instead
- Meaning: Recipient receives a wrapped version (ERC6160)

---

## Part 3: Timeout/Refund - `onPostRequestTimeout()` function

```solidity
// TokenGateway.sol - onPostRequestTimeout()
Line 492: if (_erc20 != address(0) && !body.redeem)
```

**When `redeem = false`:**
- Condition met: Original sender custodied ERC20
- Action: Return the custodied ERC20 back to sender
- Meaning: Refund the native token that was locked

**When `redeem = true`:**
- Condition NOT met: Original sender burned ERC6160
- Falls through to line [494-495](https://github.com/polytope-labs/hyperbridge/blob/e0c7e5d0e19f8a6ac97c766221907b3cf1db0409/evm/src/modules/TokenGateway.sol#L494-L495): Re-mint ERC6160 wrapper
- Meaning: Refund by minting back the wrapped tokens

---

## Complete Flow Examples

### Example 1: Teleporting tGBP from Ethereum → Xcavate (standard case)

User sets `redeem = false`

| Step       | Chain    | Action                                    | Code Reference                              |
|------------|----------|-------------------------------------------|---------------------------------------------|
| 1. Send    | Ethereum | Custody tGBP in TokenGateway              | TokenGateway.sol `teleport()` [Line 384](https://github.com/polytope-labs/hyperbridge/blob/e0c7e5d0e19f8a6ac97c766221907b3cf1db0409/evm/src/modules/TokenGateway.sol#L384)      |
| 2. Receive | Xcavate  | Mint tGBP to recipient                    | pallet-token-gateway `on_accept()`          |
| 3. Timeout | Ethereum | Return custodied tGBP to sender           | TokenGateway.sol `onPostRequestTimeout()` [Line 493](https://github.com/polytope-labs/hyperbridge/blob/e0c7e5d0e19f8a6ac97c766221907b3cf1db0409/evm/src/modules/TokenGateway.sol#L493) |

**Result:** Recipient gets tGBP on Xcavate

---

### Example 2: Redeeming Native ERC20 (advanced case)

User sets `redeem = true`

**Prerequisites:**
- The same ERC20 must be deployed on BOTH chains
- TokenGateway on destination must have custody of that ERC20 (from previous transfers)

| Step       | Chain    | Action                                       | Code Reference                              |
|------------|----------|----------------------------------------------|---------------------------------------------|
| 1. Send    | Xcavate  | Burn wrapped tokens                          | pallet-token-gateway `teleport()`           |
| 2. Receive | Ethereum | Transfer custodied native ERC20 to recipient | TokenGateway.sol `handleIncomingAsset()` [Line 555](https://github.com/polytope-labs/hyperbridge/blob/e0c7e5d0e19f8a6ac97c766221907b3cf1db0409/evm/src/modules/TokenGateway.sol#L555) |
| 3. Timeout | Xcavate  | Re-mint wrapped tokens to sender             | pallet-token-gateway `on_timeout()`         |

**Result:** Recipient gets native ERC20 on Ethereum (not a wrapper)

> **Note:** This flow is typically used when sending tokens back from Xcavate to Ethereum.

---

## Why This Design?

The redeem flag answers: **"What does the recipient want?"**

```
redeem = false → "I want the wrapped version (ERC6160)"
  ├─ Source: Lock native ERC20
  └─ Destination: Mint wrapper

redeem = true → "I want the native ERC20 on destination"
  ├─ Source: Burn my wrapper
  └─ Destination: Release custodied native ERC20
```

---

## Key Insight: The Inversion

Notice the opposite logic between source and destination:

| Location                           | `redeem = false`        | `redeem = true`         |
|------------------------------------|-------------------------|-------------------------|
| Source (`teleport()`)     | `!redeem` → Custody ERC20 | Burn ERC6160          |
| Destination (`handleIncomingAsset()`) | Mint ERC6160 | `redeem` → Transfer ERC20 |
| Timeout (`onPostRequestTimeout()`)    | `!redeem` → Return ERC20 | Re-mint ERC6160      |

This is because:

**Source logic: What are you giving up?**
- `redeem=false`: Giving up native token → custody it
- `redeem=true`: Giving up wrapper → burn it

**Destination logic: What do you want to receive?**
- `redeem=false`: Want wrapper → mint it
- `redeem=true`: Want native → release custodied tokens

---

## For Xcavate's Use Case

If you're teleporting an existing ERC20 from Ethereum to Xcavate:

### Set `redeem = false` if:

- You want recipients to receive a wrapped version (ERC6160)
- The ERC20 doesn't exist on the destination chain
- **This is the standard case for bridging tGBP to Xcavate**

**Flow:** Custody on Ethereum → Mint wrapper on destination

### Set `redeem = true` if:

- The exact same ERC20 is deployed on destination chain
- TokenGateway on destination already has custody of that ERC20
- You want to receive the native token instead of wrapper

**Flow:** Burn wrapper on Ethereum → Release custodied native on destination

---

[← Back to Reference](./README.md)
