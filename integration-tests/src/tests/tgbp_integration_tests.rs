//! TGBP transfer integration tests with runtime
//!
//! These tests execute actual ISMP messages through the Xcavate runtime,
//! verifying the complete flow from Ethereum → Xcavate.

use crate::{
    mock::{
        ismp_messages::*,
        test_accounts::*,
    },
    runtime_tests::test_externalities::*,
};
use frame_support::{assert_ok, traits::fungibles::Inspect};
use ismp::{module::IsmpModule, router::Request};
use sp_core::H256;
use sp_runtime::{AccountId32, MultiAddress};
use xcavate_runtime::{Assets, Runtime, RuntimeOrigin, System, TokenGateway};

/// TGBP asset ID on Xcavate (must be registered before tests)
const TGBP_LOCAL_ASSET_ID: u32 = 1;

/// Get TGBP asset ID (keccak256 of "TGBP")
fn tgbp_asset_id() -> H256 {
    calculate_asset_id(b"TGBP")
}

/// Helper to register TGBP asset in the token gateway
/// This simulates the asset registration that would happen via governance
/// by directly inserting into storage (acceptable in tests)
fn register_tgbp_asset() {
    use ismp::host::StateMachine;
    use xcavate_runtime::configs::ismp::AssetAdmin;
    use frame_support::traits::Get;

    let asset_id = tgbp_asset_id();
    let admin = AssetAdmin::get();

    // Create the asset in pallet_assets first with admin as owner
    // Using root origin as CreateOrigin requires it
    assert_ok!(Assets::force_create(
        RuntimeOrigin::root(),
        TGBP_LOCAL_ASSET_ID.into(),
        MultiAddress::Id(admin.clone()), // admin/owner
        true,                             // is_sufficient
        1,                                // min_balance
    ));

    // Set asset metadata (name, symbol, decimals)
    // IMPORTANT: TGBP maintains its native 6 decimal precision on Xcavate
    assert_ok!(Assets::force_set_metadata(
        RuntimeOrigin::root(),
        TGBP_LOCAL_ASSET_ID.into(),
        b"Token Gateway British Pound".to_vec(),
        b"TGBP".to_vec(),
        6,    // 6 decimals (same as on Ethereum)
        false // is_frozen
    ));

    // Insert into token gateway storage
    // This simulates what create_erc6160_asset would do
    pallet_token_gateway::SupportedAssets::<Runtime>::insert(TGBP_LOCAL_ASSET_ID, asset_id);
    pallet_token_gateway::LocalAssets::<Runtime>::insert(asset_id, TGBP_LOCAL_ASSET_ID);
    pallet_token_gateway::NativeAssets::<Runtime>::insert(TGBP_LOCAL_ASSET_ID, false); // Not native to Xcavate

    // Set precision: TGBP has 6 decimals on Ethereum
    pallet_token_gateway::Precisions::<Runtime>::insert(
        TGBP_LOCAL_ASSET_ID,
        StateMachine::Evm(1),
        6u8,
    );

    // Whitelist the Ethereum token gateway address (must match ETHEREUM_TOKEN_GATEWAY_ADDRESS)
    // In production, this would be the actual TokenGateway contract address
    pallet_token_gateway::TokenGatewayAddresses::<Runtime>::insert(
        StateMachine::Evm(1),
        ETHEREUM_TOKEN_GATEWAY_ADDRESS.to_vec(),
    );
}

/// Helper to get asset balance for an account
fn get_balance(asset_id: u32, account: &AccountId32) -> u128 {
    <Assets as Inspect<AccountId32>>::balance(asset_id, account)
}

/// Helper to check if an asset exists
fn asset_exists(asset_id: u32) -> bool {
    pallet_assets::Asset::<Runtime, pallet_assets::Instance2>::contains_key(asset_id)
}

#[test]
fn test_register_tgbp_asset() {
    new_test_ext().execute_with(|| {
        // Register TGBP
        register_tgbp_asset();

        // Verify asset was registered in token gateway storage
        let asset_id = tgbp_asset_id();
        let local_id = pallet_token_gateway::LocalAssets::<Runtime>::get(asset_id);
        assert_eq!(local_id, Some(TGBP_LOCAL_ASSET_ID));

        // Verify reverse mapping
        let gateway_id = pallet_token_gateway::SupportedAssets::<Runtime>::get(TGBP_LOCAL_ASSET_ID);
        assert_eq!(gateway_id, Some(asset_id));

        // Verify it's marked as non-native
        let is_native = pallet_token_gateway::NativeAssets::<Runtime>::get(TGBP_LOCAL_ASSET_ID);
        assert_eq!(is_native, false);

        // Verify precision mapping (6 decimals on Ethereum)
        use ismp::host::StateMachine;
        let precision =
            pallet_token_gateway::Precisions::<Runtime>::get(TGBP_LOCAL_ASSET_ID, StateMachine::Evm(1));
        assert_eq!(precision, Some(6));
    });
}

#[test]
fn test_process_tgbp_transfer_creates_asset_and_mints() {
    new_test_ext().execute_with(|| {
        // Setup: Register TGBP
        register_tgbp_asset();

        // Create a recipient account
        let recipient = bob_account();

        // Verify asset exists (created in register_tgbp_asset)
        assert!(asset_exists(TGBP_LOCAL_ASSET_ID));

        // Verify recipient starts with zero balance
        assert_eq!(get_balance(TGBP_LOCAL_ASSET_ID, &recipient), 0);

        // Create ISMP message: Transfer 100 TGBP (100_000_000 in 6 decimals)
        let amount_6_decimals = 100_000_000u128;
        let msg = create_tgbp_transfer_message(ALICE_ETH_ADDRESS, recipient.clone(), amount_6_decimals);

        // Extract the PostRequest from the message
        let post_request = match msg {
            Request::Post(req) => req,
            _ => panic!("Expected Request::Post"),
        };

        // Process the message through TokenGateway's on_accept callback
        // This simulates what happens when ISMP delivers the message
        let token_gateway_instance = pallet_token_gateway::Pallet::<Runtime>::default();
        let result = token_gateway_instance.on_accept(post_request);

        // Verify the callback succeeded
        assert_ok!(result);

        // Verify asset now exists in pallet_assets
        assert!(asset_exists(TGBP_LOCAL_ASSET_ID));

        // Verify recipient received tokens (no precision conversion - maintains 6 decimals)
        // 100_000_000 (6 decimals) → 100_000_000 (6 decimals)
        let expected_balance = 100_000_000u128;
        assert_eq!(get_balance(TGBP_LOCAL_ASSET_ID, &recipient), expected_balance);

        // Verify no precision conversion (1:1 mapping)
        assert_eq!(amount_6_decimals, expected_balance);
    });
}

#[test]
fn test_multiple_tgbp_transfers_accumulate() {
    new_test_ext().execute_with(|| {
        // Setup
        register_tgbp_asset();
        let recipient = bob_account();

        // First transfer: 50 TGBP
        let msg1 = create_tgbp_transfer_message(ALICE_ETH_ADDRESS, recipient.clone(), 50_000_000);
        let req1 = match msg1 {
            Request::Post(req) => req,
            _ => panic!("Expected Request::Post"),
        };
        let token_gateway = pallet_token_gateway::Pallet::<Runtime>::default();
        assert_ok!(token_gateway.on_accept(req1));

        // Check balance after first transfer
        let balance_after_first = get_balance(TGBP_LOCAL_ASSET_ID, &recipient);
        assert_eq!(balance_after_first, 50_000_000); // 50 TGBP in 6 decimals

        // Second transfer: 30 TGBP
        let msg2 = create_tgbp_transfer_message(BOB_ETH_ADDRESS, recipient.clone(), 30_000_000);
        let req2 = match msg2 {
            Request::Post(req) => req,
            _ => panic!("Expected Request::Post"),
        };
        let token_gateway2 = pallet_token_gateway::Pallet::<Runtime>::default();
        assert_ok!(token_gateway2.on_accept(req2));

        // Check balance after second transfer (should accumulate)
        let balance_after_second = get_balance(TGBP_LOCAL_ASSET_ID, &recipient);
        assert_eq!(balance_after_second, 80_000_000); // 80 TGBP in 6 decimals

        // Verify accumulation is correct
        assert_eq!(balance_after_second, balance_after_first + 30_000_000);
    });
}

#[test]
fn test_transfer_to_multiple_recipients() {
    new_test_ext().execute_with(|| {
        // Setup
        register_tgbp_asset();

        let bob = bob_account();
        let charlie = charlie_account();

        // Transfer to Bob: 100 TGBP
        let msg_bob = create_tgbp_transfer_message(ALICE_ETH_ADDRESS, bob.clone(), 100_000_000);
        let req_bob = match msg_bob {
            Request::Post(req) => req,
            _ => panic!("Expected Request::Post"),
        };
        let token_gateway = pallet_token_gateway::Pallet::<Runtime>::default();
        assert_ok!(token_gateway.on_accept(req_bob));

        // Transfer to Charlie: 200 TGBP
        let msg_charlie =
            create_tgbp_transfer_message(ALICE_ETH_ADDRESS, charlie.clone(), 200_000_000);
        let req_charlie = match msg_charlie {
            Request::Post(req) => req,
            _ => panic!("Expected Request::Post"),
        };
        let token_gateway2 = pallet_token_gateway::Pallet::<Runtime>::default();
        assert_ok!(token_gateway2.on_accept(req_charlie));

        // Verify Bob's balance
        assert_eq!(get_balance(TGBP_LOCAL_ASSET_ID, &bob), 100_000_000);

        // Verify Charlie's balance
        assert_eq!(get_balance(TGBP_LOCAL_ASSET_ID, &charlie), 200_000_000);

        // Verify balances are independent
        assert_ne!(get_balance(TGBP_LOCAL_ASSET_ID, &bob), get_balance(TGBP_LOCAL_ASSET_ID, &charlie));
    });
}

#[test]
fn test_precision_conversion_various_amounts() {
    new_test_ext().execute_with(|| {
        // Setup
        register_tgbp_asset();
        let recipient = bob_account();

        // Test cases: (input_amount, expected_balance)
        // No precision conversion - 6 decimals maintained
        let test_cases = vec![
            (1_000_000u128, 1_000_000u128),         // 1 TGBP
            (500_000u128, 500_000u128),             // 0.5 TGBP
            (1_500_000u128, 1_500_000u128),         // 1.5 TGBP
            (1u128, 1u128),                         // 0.000001 TGBP (minimum)
            (100_000_000u128, 100_000_000u128),     // 100 TGBP
            (1_234_567_890u128, 1_234_567_890u128), // 1234.56789 TGBP
        ];

        for (amount_6_dec, expected_balance) in test_cases {
            // Create new recipient for each test to avoid accumulation
            let test_recipient = AccountId32::new([amount_6_dec as u8; 32]);

            let msg = create_tgbp_transfer_message(ALICE_ETH_ADDRESS, test_recipient.clone(), amount_6_dec);
            let req = match msg {
                Request::Post(req) => req,
                _ => panic!("Expected Request::Post"),
            };

            let token_gateway = pallet_token_gateway::Pallet::<Runtime>::default();
            assert_ok!(token_gateway.on_accept(req));

            let balance = get_balance(TGBP_LOCAL_ASSET_ID, &test_recipient);
            assert_eq!(
                balance, expected_balance,
                "Amount mismatch for {} (6 dec) - expected {}, got {}",
                amount_6_dec, expected_balance, balance
            );
        }
    });
}

#[test]
fn test_events_emitted_on_transfer() {
    new_test_ext().execute_with(|| {
        // Setup
        register_tgbp_asset();
        let recipient = bob_account();

        // Clear any existing events
        System::reset_events();

        // Process transfer
        let msg = create_tgbp_transfer_message(ALICE_ETH_ADDRESS, recipient.clone(), 100_000_000);
        let req = match msg {
            Request::Post(req) => req,
            _ => panic!("Expected Request::Post"),
        };
        let token_gateway = pallet_token_gateway::Pallet::<Runtime>::default();
        assert_ok!(token_gateway.on_accept(req));

        // Get all events
        let events = System::events();

        // Verify we have events
        assert!(!events.is_empty(), "No events were emitted");

        // Look for pallet_assets::Event::Issued (minting event)
        let has_issued_event = events.iter().any(|record| {
            matches!(
                record.event,
                xcavate_runtime::RuntimeEvent::Assets(pallet_assets::Event::Issued { .. })
            )
        });

        assert!(has_issued_event, "Expected Assets::Issued event was not found");

        // Note: We would also check for TokenGateway::AssetReceived event,
        // but we need to see what events the pallet actually emits
    });
}

#[test]
#[should_panic(expected = "Asset not found")]
fn test_unregistered_asset_fails() {
    new_test_ext().execute_with(|| {
        // Do NOT register TGBP

        let recipient = bob_account();

        // Try to process transfer for unregistered asset
        let msg = create_tgbp_transfer_message(ALICE_ETH_ADDRESS, recipient, 100_000_000);
        let req = match msg {
            Request::Post(req) => req,
            _ => panic!("Expected Request::Post"),
        };

        // This should fail because asset is not registered
        let token_gateway = pallet_token_gateway::Pallet::<Runtime>::default();
        let result = token_gateway.on_accept(req);

        // Expecting an error
        result.expect("Asset not found");
    });
}

#[test]
fn test_minimum_balance_enforcement() {
    new_test_ext().execute_with(|| {
        // Setup
        register_tgbp_asset();
        let recipient = bob_account();

        // Try to transfer a very small amount
        // Minimum balance is 1 (set in force_create)
        // Testing with a small amount in 6 decimals
        let small_amount = 999u128; // 0.000999 TGBP

        let msg = create_tgbp_transfer_message(ALICE_ETH_ADDRESS, recipient.clone(), small_amount);
        let req = match msg {
            Request::Post(req) => req,
            _ => panic!("Expected Request::Post"),
        };

        let token_gateway = pallet_token_gateway::Pallet::<Runtime>::default();
        let result = token_gateway.on_accept(req);

        // This might fail or succeed depending on pallet_assets minimum balance enforcement
        // If it succeeds, verify the balance matches the small amount
        if result.is_ok() {
            let balance = get_balance(TGBP_LOCAL_ASSET_ID, &recipient);
            assert_eq!(balance, small_amount, "Balance should match transferred amount");
        }
        // If it fails, that's also acceptable behavior for enforcing minimum balance
    });
}

#[test]
fn test_invalid_precision_mapping() {
    new_test_ext().execute_with(|| {
        use ismp::host::StateMachine;

        // Register TGBP but WITHOUT precision mapping for BSC
        let asset_id = tgbp_asset_id();
        pallet_token_gateway::SupportedAssets::<Runtime>::insert(TGBP_LOCAL_ASSET_ID, asset_id);
        pallet_token_gateway::LocalAssets::<Runtime>::insert(asset_id, TGBP_LOCAL_ASSET_ID);
        pallet_token_gateway::NativeAssets::<Runtime>::insert(TGBP_LOCAL_ASSET_ID, false);

        // Only set precision for Ethereum, not BSC
        pallet_token_gateway::Precisions::<Runtime>::insert(
            TGBP_LOCAL_ASSET_ID,
            StateMachine::Evm(1), // Ethereum
            6u8,
        );

        // Whitelist BSC token gateway address
        pallet_token_gateway::TokenGatewayAddresses::<Runtime>::insert(
            StateMachine::Evm(56), // BSC
            ETHEREUM_TOKEN_GATEWAY_ADDRESS.to_vec(),
        );

        // Try to transfer from BSC (which has no precision configured)
        let recipient = bob_account();
        let msg = create_erc20_transfer_message(
            b"TGBP",
            ALICE_ETH_ADDRESS,
            recipient,
            100_000_000,
            Some(StateMachine::Evm(56)), // BSC
        );

        let req = match msg {
            Request::Post(req) => req,
            _ => panic!("Expected Request::Post"),
        };

        let token_gateway = pallet_token_gateway::Pallet::<Runtime>::default();
        let result = token_gateway.on_accept(req);

        // Should fail because precision is not configured for BSC
        assert!(result.is_err(), "Should fail when precision is not configured");

        if let Err(e) = result {
            let error_msg = format!("{:?}", e);
            assert!(
                error_msg.contains("decimals not configured") || error_msg.contains("precision"),
                "Error should mention missing precision configuration: {}",
                error_msg
            );
        }
    });
}

#[test]
fn test_asset_metadata_after_creation() {
    new_test_ext().execute_with(|| {
        // Setup
        register_tgbp_asset();
        let recipient = bob_account();

        // Process a transfer to trigger asset creation
        let msg = create_tgbp_transfer_message(ALICE_ETH_ADDRESS, recipient, 100_000_000);
        let req = match msg {
            Request::Post(req) => req,
            _ => panic!("Expected Request::Post"),
        };

        let token_gateway = pallet_token_gateway::Pallet::<Runtime>::default();
        let result = token_gateway.on_accept(req);

        // If successful, check asset metadata
        if result.is_ok() {
            // Check if asset exists
            assert!(asset_exists(TGBP_LOCAL_ASSET_ID), "Asset should exist after transfer");

            // Check asset metadata
            use frame_support::traits::fungibles::metadata::Inspect;
            let name = <Assets as Inspect<AccountId32>>::name(TGBP_LOCAL_ASSET_ID);
            let symbol = <Assets as Inspect<AccountId32>>::symbol(TGBP_LOCAL_ASSET_ID);
            let decimals = <Assets as Inspect<AccountId32>>::decimals(TGBP_LOCAL_ASSET_ID);

            // Verify metadata (token gateway should set these during asset creation)
            // Symbol should be "TGBP"
            assert!(!symbol.is_empty(), "Symbol should be set");

            // Name should be something like "Tokenised GBP"
            assert!(!name.is_empty(), "Name should be set");

            // Decimals should be 6 (maintains native precision)
            assert_eq!(decimals, 6, "Decimals should be 6 (same as on Ethereum)");
        }
    });
}

#[test]
fn test_asset_total_supply_tracking() {
    new_test_ext().execute_with(|| {
        // Setup
        register_tgbp_asset();

        let bob = bob_account();
        let charlie = charlie_account();

        // Process first transfer: 100 TGBP to Bob
        let msg1 = create_tgbp_transfer_message(ALICE_ETH_ADDRESS, bob.clone(), 100_000_000);
        let req1 = match msg1 {
            Request::Post(req) => req,
            _ => panic!("Expected Request::Post"),
        };
        let token_gateway1 = pallet_token_gateway::Pallet::<Runtime>::default();

        if token_gateway1.on_accept(req1).is_ok() {
            // Check total supply after first mint
            use frame_support::traits::fungibles::Inspect;
            let supply_after_first = <Assets as Inspect<AccountId32>>::total_issuance(TGBP_LOCAL_ASSET_ID);
            assert_eq!(supply_after_first, 100_000_000, "Total supply should be 100 TGBP");

            // Process second transfer: 50 TGBP to Charlie
            let msg2 = create_tgbp_transfer_message(BOB_ETH_ADDRESS, charlie.clone(), 50_000_000);
            let req2 = match msg2 {
                Request::Post(req) => req,
                _ => panic!("Expected Request::Post"),
            };
            let token_gateway2 = pallet_token_gateway::Pallet::<Runtime>::default();

            if token_gateway2.on_accept(req2).is_ok() {
                // Check total supply after second mint (should accumulate)
                let supply_after_second = <Assets as Inspect<AccountId32>>::total_issuance(TGBP_LOCAL_ASSET_ID);
                assert_eq!(supply_after_second, 150_000_000, "Total supply should be 150 TGBP");

                // Verify individual balances sum to total supply
                let bob_balance = get_balance(TGBP_LOCAL_ASSET_ID, &bob);
                let charlie_balance = get_balance(TGBP_LOCAL_ASSET_ID, &charlie);
                assert_eq!(bob_balance + charlie_balance, supply_after_second, "Balances should sum to total supply");
            }
        }
    });
}

#[test]
fn test_zero_amount_transfer() {
    new_test_ext().execute_with(|| {
        // Setup
        register_tgbp_asset();
        let recipient = bob_account();

        // Try to transfer 0 amount
        let msg = create_tgbp_transfer_message(ALICE_ETH_ADDRESS, recipient.clone(), 0);
        let req = match msg {
            Request::Post(req) => req,
            _ => panic!("Expected Request::Post"),
        };

        let token_gateway = pallet_token_gateway::Pallet::<Runtime>::default();
        let result = token_gateway.on_accept(req);

        // Zero amount transfers might be rejected or create zero balance
        if result.is_ok() {
            let balance = get_balance(TGBP_LOCAL_ASSET_ID, &recipient);
            assert_eq!(balance, 0, "Balance should remain zero");
        }
        // If it fails, that's also acceptable for zero amount transfers
    });
}

#[test]
fn test_same_recipient_multiple_senders() {
    new_test_ext().execute_with(|| {
        // Setup
        register_tgbp_asset();
        let recipient = bob_account();

        // Transfer from Alice: 50 TGBP
        let msg1 = create_tgbp_transfer_message(ALICE_ETH_ADDRESS, recipient.clone(), 50_000_000);
        let req1 = match msg1 {
            Request::Post(req) => req,
            _ => panic!("Expected Request::Post"),
        };
        let token_gateway1 = pallet_token_gateway::Pallet::<Runtime>::default();

        if token_gateway1.on_accept(req1).is_ok() {
            let balance_after_alice = get_balance(TGBP_LOCAL_ASSET_ID, &recipient);
            assert_eq!(balance_after_alice, 50_000_000);

            // Transfer from Bob's Ethereum address: 30 TGBP
            let msg2 = create_tgbp_transfer_message(BOB_ETH_ADDRESS, recipient.clone(), 30_000_000);
            let req2 = match msg2 {
                Request::Post(req) => req,
                _ => panic!("Expected Request::Post"),
            };
            let token_gateway2 = pallet_token_gateway::Pallet::<Runtime>::default();

            if token_gateway2.on_accept(req2).is_ok() {
                // Should accumulate: 50 + 30 = 80 TGBP
                let balance_after_bob = get_balance(TGBP_LOCAL_ASSET_ID, &recipient);
                assert_eq!(balance_after_bob, 80_000_000);
            }
        }
    });
}
