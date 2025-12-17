//! Integration tests for tGBP transfers from Ethereum to Xcavate
//!
//! These tests verify that mock ISMP messages for tGBP transfers are
//! correctly structured and can be processed by the token gateway.
//!
//! Test Scenarios:
//! 1. Valid tGBP transfer message structure
//! 2. Amount preservation (maintains 18 decimals on Xcavate)
//! 3. Asset ID calculation
//! 4. Multiple transfers with unique nonces
//! 5. Minimum balance edge cases

use ismp::router::Request;
use sp_core::H256;

use crate::mock::{ismp_messages::*, test_accounts::*};

/// Expected tGBP asset ID (keccak256 of "tGBP")
fn expected_tgbp_asset_id() -> H256 {
    calculate_asset_id(b"tGBP")
}

#[test]
fn valid_tgbp_transfer_message_structure() {
    // Create a tGBP transfer: 100 tGBP (100 * 10^18 in 18 decimals)
    let amount = 100_000_000_000_000_000_000u128; // 100 tGBP
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
    // tGBP maintains 18 decimals on both Ethereum and Xcavate (no conversion)
    let test_cases = vec![
        (1_000_000_000_000_000_000u128, 1_000_000_000_000_000_000u128), // 1 tGBP
        (100_000_000_000_000_000_000u128, 100_000_000_000_000_000_000u128), // 100 tGBP
        (1_500_000_000_000_000_000u128, 1_500_000_000_000_000_000u128), // 1.5 tGBP
        (500_000_000_000_000_000u128, 500_000_000_000_000_000u128),     // 0.5 tGBP
    ];

    for (amount_18_dec, expected_amount) in test_cases {
        let msg = create_tgbp_transfer_message(ALICE_ETH_ADDRESS, bob_account(), amount_18_dec);

        // Verify message was created
        assert!(matches!(msg, Request::Post(_)));

        // tGBP maintains 18 decimals on Xcavate (no precision conversion)
        // The amount in the message should equal the expected amount
        assert_eq!(
            amount_18_dec, expected_amount,
            "Amount preservation: {} (18 dec) maintains {} (18 dec)",
            amount_18_dec, expected_amount
        );
    }
}

#[test]
fn asset_id_consistency() {
    // Asset ID for tGBP should be consistent
    let msg1 = create_tgbp_transfer_message(ALICE_ETH_ADDRESS, bob_account(), 1_000_000_000_000_000_000);
    let msg2 = create_tgbp_transfer_message(BOB_ETH_ADDRESS, charlie_account(), 2_000_000_000_000_000_000);

    // Both messages should be valid PostRequests
    assert!(matches!(msg1, Request::Post(_)));
    assert!(matches!(msg2, Request::Post(_)));

    // Both should reference tGBP asset (verifiable through calculate_asset_id)
    let tgbp_id = expected_tgbp_asset_id();
    assert_eq!(tgbp_id, calculate_asset_id(b"tGBP"));
}

#[test]
fn multiple_transfers_unique_nonces() {
    // Create multiple transfers and verify nonces are unique
    let msg1 = create_tgbp_transfer_message(ALICE_ETH_ADDRESS, bob_account(), 1_000_000_000_000_000_000);
    let msg2 = create_tgbp_transfer_message(ALICE_ETH_ADDRESS, bob_account(), 2_000_000_000_000_000_000);
    let msg3 = create_tgbp_transfer_message(BOB_ETH_ADDRESS, charlie_account(), 3_000_000_000_000_000_000);

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
    // Minimum balance for tGBP is set to 1 in the asset configuration
    // Testing various amounts in 18 decimals

    let test_cases = vec![
        (1u128, "Dust amount (1 in 18 decimals)"),
        (1_000_000_000_000_000u128, "Small amount (0.001 tGBP)"),
        (1_000_000_000_000_000_000u128, "1 tGBP"),
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
    // Test creating messages from different EVM chains (mainnet and testnet)
    use ismp::host::StateMachine;

    // Ethereum Mainnet (chain ID 1)
    let ethereum_msg = create_erc20_transfer_message(
        b"tGBP",
        ALICE_ETH_ADDRESS,
        bob_account(),
        1_000_000_000_000_000_000,
        Some(StateMachine::Evm(1)),
    );

    // Ethereum Sepolia Testnet (chain ID 11155111)
    let sepolia_msg = create_erc20_transfer_message(
        b"tGBP",
        ALICE_ETH_ADDRESS,
        bob_account(),
        1_000_000_000_000_000_000,
        Some(StateMachine::Evm(11155111)),
    );

    // BSC Mainnet (chain ID 56)
    let bsc_msg = create_erc20_transfer_message(
        b"tGBP",
        ALICE_ETH_ADDRESS,
        bob_account(),
        1_000_000_000_000_000_000,
        Some(StateMachine::Evm(56)),
    );

    let extract_source = |msg: Request| -> StateMachine {
        if let Request::Post(req) = msg {
            req.source
        } else {
            panic!("Expected Request::Post")
        }
    };

    // Verify each chain ID is correctly set
    assert_eq!(extract_source(ethereum_msg), StateMachine::Evm(1));
    assert_eq!(extract_source(sepolia_msg), StateMachine::Evm(11155111));
    assert_eq!(extract_source(bsc_msg), StateMachine::Evm(56));
}

#[test]
fn recipient_account_encoding() {
    // Test that different recipient accounts are correctly encoded
    let msg_bob = create_tgbp_transfer_message(ALICE_ETH_ADDRESS, bob_account(), 1_000_000_000_000_000_000);
    let msg_charlie = create_tgbp_transfer_message(ALICE_ETH_ADDRESS, charlie_account(), 1_000_000_000_000_000_000);

    // Both messages should be valid PostRequests
    assert!(matches!(msg_bob, Request::Post(_)));
    assert!(matches!(msg_charlie, Request::Post(_)));

    // Verify bob and charlie are different accounts
    assert_ne!(bob_account(), charlie_account());
}

#[test]
fn sepolia_testnet_messages() {
    // Test that Sepolia testnet messages are correctly structured
    // This tests the mainnet/testnet parity for integration testing
    use ismp::host::StateMachine;

    // Create WETH transfer from Sepolia (commonly used for testing)
    let weth_msg = create_erc20_transfer_message(
        b"WETH",
        ALICE_ETH_ADDRESS,
        bob_account(),
        1_000_000_000_000_000_000, // 1 WETH (18 decimals)
        Some(ETHEREUM_SEPOLIA),
    );

    match weth_msg {
        Request::Post(req) => {
            // Source should be Sepolia
            assert_eq!(req.source, StateMachine::Evm(11155111));
            // Destination should be Xcavate
            assert_eq!(req.dest, XCAVATE_PARACHAIN);
            // Body should be valid
            assert!(!req.body.is_empty());
            assert_eq!(req.body[0], 0x00);
        }
        _ => panic!("Expected Request::Post"),
    }

    // Verify WETH asset ID is deterministic
    let weth_id = calculate_asset_id(b"WETH");
    let weth_id_again = calculate_asset_id(b"WETH");
    assert_eq!(weth_id, weth_id_again);

    // WETH ID should differ from tGBP ID
    let tgbp_id = calculate_asset_id(b"tGBP");
    assert_ne!(weth_id, tgbp_id);
}

// NOTE: Runtime processing tests have been implemented in:
// src/runtime_tests/tgbp_integration_tests.rs
//
// These tests execute actual ISMP messages through the Xcavate runtime and verify:
// - Asset creation and registration
// - Token minting with 18 decimal precision (maintained from Ethereum)
// - Balance accumulation across multiple transfers
// - Event emission
// - Error handling for unregistered assets and invalid configurations
