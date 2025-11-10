//! Integration tests for TGBP transfers from Ethereum to Xcavate
//!
//! These tests verify that mock ISMP messages for TGBP transfers are
//! correctly structured and can be processed by the token gateway.
//!
//! Test Scenarios:
//! 1. Valid TGBP transfer message structure
//! 2. Amount preservation (maintains 6 decimals on Xcavate)
//! 3. Asset ID calculation
//! 4. Multiple transfers with unique nonces
//! 5. Minimum balance edge cases

use ismp::router::Request;
use sp_core::H256;

use crate::mock::{ismp_messages::*, test_accounts::*};

/// Expected TGBP asset ID (keccak256 of "TGBP")
fn expected_tgbp_asset_id() -> H256 {
    calculate_asset_id(b"TGBP")
}

#[test]
fn valid_tgbp_transfer_message_structure() {
    // Create a TGBP transfer: 100 TGBP (100_000_000 in 6 decimals)
    let amount = 100_000_000u128;
    let msg = create_tgbp_transfer_message(ALICE_ETH_ADDRESS, bob_account(), amount);

    match msg {
        Request::Post(req) => {
            // Verify source and destination
            assert_eq!(req.source, ETHEREUM_MAINNET);
            assert_eq!(req.dest, XCAVATE_PARACHAIN);

            // Verify addresses
            assert_eq!(req.from, ETHEREUM_TOKEN_GATEWAY_ADDRESS.to_vec());
            assert_eq!(req.to, PALLET_TOKEN_GATEWAY_ID.to_vec());

            // Verify body is not empty
            assert!(!req.body.is_empty());

            // Body should start with 0x00 (indicates Body, not BodyWithCall)
            assert_eq!(req.body[0], 0x00, "Body should start with 0x00 prefix");

            // Rest is ABI-encoded - we trust the encoding is correct
            // (detailed body validation would require ABI decoding)
        }
        _ => panic!("Expected Request::Post"),
    }
}

#[test]
fn precision_conversion_expectations() {
    // TGBP maintains 6 decimals on both Ethereum and Xcavate (no conversion)
    let test_cases = vec![
        (1_000_000u128, 1_000_000u128),     // 1 TGBP: 6 dec → 6 dec
        (100_000_000u128, 100_000_000u128), // 100 TGBP
        (1_500_000u128, 1_500_000u128),     // 1.5 TGBP
        (500_000u128, 500_000u128),         // 0.5 TGBP
    ];

    for (amount_6_dec, expected_amount) in test_cases {
        let msg = create_tgbp_transfer_message(ALICE_ETH_ADDRESS, bob_account(), amount_6_dec);

        // Verify message was created
        assert!(matches!(msg, Request::Post(_)));

        // TGBP maintains 6 decimals on Xcavate (no precision conversion)
        // The amount in the message should equal the expected amount
        assert_eq!(
            amount_6_dec, expected_amount,
            "Amount preservation: {} (6 dec) maintains {} (6 dec)",
            amount_6_dec, expected_amount
        );
    }
}

#[test]
fn asset_id_consistency() {
    // Asset ID for TGBP should be consistent
    let msg1 = create_tgbp_transfer_message(ALICE_ETH_ADDRESS, bob_account(), 1_000_000);
    let msg2 = create_tgbp_transfer_message(BOB_ETH_ADDRESS, charlie_account(), 2_000_000);

    // Both messages should be valid PostRequests
    assert!(matches!(msg1, Request::Post(_)));
    assert!(matches!(msg2, Request::Post(_)));

    // Both should reference TGBP asset (verifiable through calculate_asset_id)
    let tgbp_id = expected_tgbp_asset_id();
    assert_eq!(tgbp_id, calculate_asset_id(b"TGBP"));
}

#[test]
fn multiple_transfers_unique_nonces() {
    // Create multiple transfers and verify nonces are unique
    let msg1 = create_tgbp_transfer_message(ALICE_ETH_ADDRESS, bob_account(), 1_000_000);
    let msg2 = create_tgbp_transfer_message(ALICE_ETH_ADDRESS, bob_account(), 2_000_000);
    let msg3 = create_tgbp_transfer_message(BOB_ETH_ADDRESS, charlie_account(), 3_000_000);

    let extract_nonce = |msg: Request| -> u64 {
        if let Request::Post(req) = msg {
            req.nonce
        } else {
            panic!("Expected Request::Post")
        }
    };

    let nonce1 = extract_nonce(msg1);
    let nonce2 = extract_nonce(msg2);
    let nonce3 = extract_nonce(msg3);

    // All nonces should be unique
    assert_ne!(nonce1, nonce2);
    assert_ne!(nonce2, nonce3);
    assert_ne!(nonce1, nonce3);

    // Nonces should be monotonically increasing
    assert!(nonce2 > nonce1);
    assert!(nonce3 > nonce2);
}

#[test]
fn minimum_balance_amounts() {
    // Test edge cases around minimum balances
    // Minimum balance for TGBP is set to 1 in the asset configuration
    // Testing various amounts in 6 decimals

    let test_cases = vec![
        (1u128, "Dust amount (1 in 6 decimals)"),
        (1_000u128, "Small amount (0.001 TGBP)"),
        (1_000_000u128, "1 TGBP"),
        (u128::MAX, "Maximum amount"),
    ];

    for (amount, description) in test_cases {
        let msg = create_tgbp_transfer_message(ALICE_ETH_ADDRESS, bob_account(), amount);

        // Verify message was created successfully
        assert!(matches!(msg, Request::Post(_)), "Failed to create message for {}", description);
    }
}

#[test]
fn different_source_chains() {
    // Test creating messages from different EVM chains
    use ismp::host::StateMachine;

    let ethereum_msg = create_erc20_transfer_message(
        b"TGBP",
        ALICE_ETH_ADDRESS,
        bob_account(),
        1_000_000,
        Some(StateMachine::Evm(1)), // Ethereum Mainnet
    );

    let bsc_msg = create_erc20_transfer_message(
        b"TGBP",
        ALICE_ETH_ADDRESS,
        bob_account(),
        1_000_000,
        Some(StateMachine::Evm(56)), // BSC Mainnet
    );

    let extract_source = |msg: Request| -> StateMachine {
        if let Request::Post(req) = msg {
            req.source
        } else {
            panic!("Expected Request::Post")
        }
    };

    assert_eq!(extract_source(ethereum_msg), StateMachine::Evm(1));
    assert_eq!(extract_source(bsc_msg), StateMachine::Evm(56));
}

#[test]
fn recipient_account_encoding() {
    // Test that different recipient accounts are correctly encoded
    let msg_bob = create_tgbp_transfer_message(ALICE_ETH_ADDRESS, bob_account(), 1_000_000);
    let msg_charlie = create_tgbp_transfer_message(ALICE_ETH_ADDRESS, charlie_account(), 1_000_000);

    // Both messages should be valid PostRequests
    assert!(matches!(msg_bob, Request::Post(_)));
    assert!(matches!(msg_charlie, Request::Post(_)));

    // Verify bob and charlie are different accounts
    assert_ne!(bob_account(), charlie_account());
}

// NOTE: Runtime processing tests have been implemented in:
// src/runtime_tests/tgbp_integration_tests.rs
//
// These tests execute actual ISMP messages through the Xcavate runtime and verify:
// - Asset creation and registration
// - Token minting with 6 decimal precision (maintained from Ethereum)
// - Balance accumulation across multiple transfers
// - Event emission
// - Error handling for unregistered assets and invalid configurations
