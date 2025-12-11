//! Mock ISMP message creation utilities
//!
//! This module provides helpers to create mock PostRequest messages
//! simulating what Xcavate would receive from Ethereum via Hyperbridge.

use alloy_sol_types::SolValue;
use hex_literal::hex;
use ismp::{
    host::StateMachine,
    router::{PostRequest, Request},
};
use sp_core::{crypto::AccountId32, H256};

/// Token Gateway pallet module ID
/// This is where ISMP messages for token transfers are routed
pub const PALLET_TOKEN_GATEWAY_ID: [u8; 20] = hex!("a09b1c60e8650245f92518c8a17314878c4043ed");

/// Ethereum TokenGateway contract address (mock for testing)
/// In real scenarios, this would be the deployed TokenGateway contract on Ethereum
pub const ETHEREUM_TOKEN_GATEWAY_ADDRESS: [u8; 20] =
    hex!("0000000000000000000000000000000000000000");

/// Ethereum Mainnet state machine identifier
pub const ETHEREUM_MAINNET: StateMachine = StateMachine::Evm(1);

/// Ethereum Sepolia testnet state machine identifier
pub const ETHEREUM_SEPOLIA: StateMachine = StateMachine::Evm(11155111);

/// Xcavate parachain state machine identifier (Kusama parachain 4683)
pub const XCAVATE_PARACHAIN: StateMachine = StateMachine::Kusama(4683);

/// Counter for generating unique nonces
static mut NONCE_COUNTER: u64 = 0;

/// Generate a unique nonce for testing
pub fn next_nonce() -> u64 {
    unsafe {
        NONCE_COUNTER += 1;
        NONCE_COUNTER
    }
}

/// Calculate the asset ID for a token symbol
/// Asset IDs are computed as keccak256(symbol)
pub fn calculate_asset_id(symbol: &[u8]) -> H256 {
    H256::from(sp_core::hashing::keccak_256(symbol))
}

// Define the ABI struct for token gateway body using alloy
alloy_sol_types::sol! {
    #[derive(Debug, PartialEq, Eq)]
    struct Body {
        uint256 amount;
        bytes32 asset_id;
        bool redeem;
        bytes32 from;
        bytes32 to;
    }
}

/// Helper to encode the token transfer body
/// Returns ABI-encoded bytes with a 0x00 prefix
fn encode_token_transfer_body(
    amount: u128,
    asset_id: [u8; 32],
    from: [u8; 32],
    to: [u8; 32],
    redeem: bool,
) -> Vec<u8> {
    let body = Body {
        amount: alloy_primitives::U256::from(amount),
        asset_id: alloy_primitives::FixedBytes::from(asset_id),
        redeem,
        from: alloy_primitives::FixedBytes::from(from),
        to: alloy_primitives::FixedBytes::from(to),
    };

    // ABI encode the body
    let encoded = body.abi_encode();

    // Prepend with 0x00 byte (indicates Body, not BodyWithCall)
    let mut result = vec![0x00];
    result.extend_from_slice(&encoded);
    result
}

/// Create a mock PostRequest for a tGBP transfer from Ethereum to Xcavate
///
/// # Parameters
/// - `from`: User's Ethereum address (used in body, not request.from)
/// - `to`: Xcavate account (32 bytes)
/// - `amount`: Amount in tGBP's native precision (18 decimals)
///
/// # Note
/// The PostRequest.from field contains the Ethereum TokenGateway contract address,
/// not the user's address. The user's address would be encoded in the body if needed.
///
/// # Example
/// ```ignore
/// // Transfer 100 tGBP (100 * 10^18 in 18 decimal precision)
/// let msg = create_tgbp_transfer_message(
///     ALICE_ETH_ADDRESS,
///     bob_account(),
///     100_000_000_000_000_000_000,  // 100 tGBP
/// );
/// ```
pub fn create_tgbp_transfer_message(
    user_address: [u8; 20], // User's Ethereum address (goes in body.from)
    to: AccountId32,
    amount: u128,
) -> Request {
    let asset_id = calculate_asset_id(b"tGBP");

    // Convert addresses to 32 bytes
    let mut from_32 = [0u8; 32];
    from_32[12..].copy_from_slice(&user_address); // EVM address in last 20 bytes

    let to_32: [u8; 32] = *to.as_ref();

    // Encode the body using ABI encoding
    let body = encode_token_transfer_body(
        amount,
        asset_id.0,
        from_32,
        to_32,
        false, // redeem = false (we want ERC6160 wrapped asset)
    );

    let request = PostRequest {
        source: ETHEREUM_MAINNET,
        dest: XCAVATE_PARACHAIN,
        nonce: next_nonce(),
        // from is the TokenGateway contract address
        from: ETHEREUM_TOKEN_GATEWAY_ADDRESS.to_vec(),
        to: PALLET_TOKEN_GATEWAY_ID.to_vec(),
        timeout_timestamp: u64::MAX, // No timeout for tests
        body,
    };

    Request::Post(request)
}

/// Create a generic mock PostRequest for any ERC20 token
///
/// # Parameters
/// - `token_symbol`: Token symbol (e.g., "USDT", "tGBP")
/// - `from`: User's Ethereum address (not used in request.from)
/// - `to`: Xcavate account (32 bytes)
/// - `amount`: Amount in source chain decimals
/// - `source_chain`: Source state machine (default: Ethereum Mainnet)
pub fn create_erc20_transfer_message(
    token_symbol: &[u8],
    user_address: [u8; 20],
    to: AccountId32,
    amount: u128,
    source_chain: Option<StateMachine>,
) -> Request {
    let asset_id = calculate_asset_id(token_symbol);

    // Convert addresses to 32 bytes
    let mut from_32 = [0u8; 32];
    from_32[12..].copy_from_slice(&user_address);

    let to_32: [u8; 32] = *to.as_ref();

    // Encode the body using ABI encoding
    let body = encode_token_transfer_body(amount, asset_id.0, from_32, to_32, false);

    let request = PostRequest {
        source: source_chain.unwrap_or(ETHEREUM_MAINNET),
        dest: XCAVATE_PARACHAIN,
        nonce: next_nonce(),
        // from is the TokenGateway contract address
        from: ETHEREUM_TOKEN_GATEWAY_ADDRESS.to_vec(),
        to: PALLET_TOKEN_GATEWAY_ID.to_vec(),
        timeout_timestamp: u64::MAX,
        body,
    };

    Request::Post(request)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mock::test_accounts::*;

    #[test]
    fn test_calculate_asset_id() {
        let tgbp_asset_id = calculate_asset_id(b"tGBP");
        let usdt_asset_id = calculate_asset_id(b"USDT");

        // Asset IDs should be different for different symbols
        assert_ne!(tgbp_asset_id, usdt_asset_id);

        // Same symbol should produce same asset ID
        let tgbp_asset_id_2 = calculate_asset_id(b"tGBP");
        assert_eq!(tgbp_asset_id, tgbp_asset_id_2);
    }

    #[test]
    fn test_create_tgbp_transfer_message() {
        let msg = create_tgbp_transfer_message(
            ALICE_ETH_ADDRESS,
            bob_account(),
            100_000_000_000_000_000_000, // 100 tGBP in 18 decimals
        );

        match msg {
            Request::Post(req) => {
                assert_eq!(req.source, ETHEREUM_MAINNET);
                assert_eq!(req.dest, XCAVATE_PARACHAIN);
                // from should be the TokenGateway contract, not user address
                assert_eq!(req.from, ETHEREUM_TOKEN_GATEWAY_ADDRESS.to_vec());
                assert_eq!(req.to, PALLET_TOKEN_GATEWAY_ID.to_vec());
                assert!(!req.body.is_empty());
            }
            _ => panic!("Expected Request::Post"),
        }
    }

    #[test]
    fn test_nonce_uniqueness() {
        let nonce1 = next_nonce();
        let nonce2 = next_nonce();
        let nonce3 = next_nonce();

        assert_ne!(nonce1, nonce2);
        assert_ne!(nonce2, nonce3);
        assert!(nonce2 > nonce1);
        assert!(nonce3 > nonce2);
    }

    #[test]
    fn test_body_encoding() {
        let asset_id = calculate_asset_id(b"tGBP");
        let recipient = AccountId32::new([42u8; 32]);
        let sender = [1u8; 20];

        let mut from_32 = [0u8; 32];
        from_32[12..].copy_from_slice(&sender);

        let encoded = encode_token_transfer_body(
            1_000_000_000_000_000_000, // 1 tGBP in 18 decimals
            asset_id.0,
            from_32,
            recipient.into(),
            false,
        );

        // Should start with 0x00 prefix
        assert_eq!(encoded[0], 0x00);
        assert!(encoded.len() > 1);
    }
}
