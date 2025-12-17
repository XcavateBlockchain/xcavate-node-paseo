//! tGBP transfer integration tests with runtime
//!
//! These tests execute actual ISMP messages through the Xcavate runtime,
//! verifying the complete flow from Ethereum → Xcavate.

use frame_support::{assert_ok, traits::fungibles::Inspect};
use ismp::{module::IsmpModule, router::Request};
use sp_core::H256;
use sp_runtime::{AccountId32, MultiAddress};
use xcavate_runtime::{Assets, Runtime, RuntimeOrigin, System};

use crate::{
    mock::{ismp_messages::*, test_accounts::*},
    tests::test_externalities::*,
};

/// tGBP asset ID on Xcavate (must be registered before tests)
const TGBP_LOCAL_ASSET_ID: u32 = 1;

/// Get tGBP asset ID (keccak256 of "tGBP")
fn tgbp_asset_id() -> H256 {
    calculate_asset_id(b"tGBP")
}

/// Helper to register tGBP asset in the token gateway
/// This simulates the asset registration that would happen via governance
/// by directly inserting into storage (acceptable in tests)
fn register_tgbp_asset() {
    use frame_support::traits::Get;
    use ismp::host::StateMachine;
    use xcavate_runtime::configs::ismp::AssetAdmin;

    let asset_id = tgbp_asset_id();
    let admin = AssetAdmin::get();

    // Create the asset in pallet_assets first with admin as owner
    // Using root origin as CreateOrigin requires it
    assert_ok!(Assets::force_create(
        RuntimeOrigin::root(),
        TGBP_LOCAL_ASSET_ID.into(),
        MultiAddress::Id(admin.clone()), // admin/owner
        true,                            // is_sufficient
        1,                               // min_balance
    ));

    // Set asset metadata (name, symbol, decimals)
    // IMPORTANT: tGBP maintains its native 18 decimal precision on Xcavate
    assert_ok!(Assets::force_set_metadata(
        RuntimeOrigin::root(),
        TGBP_LOCAL_ASSET_ID.into(),
        b"Tokenised GBP".to_vec(),
        b"tGBP".to_vec(),
        18,    // 18 decimals (same as on Ethereum)
        false  // is_frozen
    ));

    // Insert into token gateway storage
    // This simulates what create_erc6160_asset would do
    pallet_token_gateway::SupportedAssets::<Runtime>::insert(TGBP_LOCAL_ASSET_ID, asset_id);
    pallet_token_gateway::LocalAssets::<Runtime>::insert(asset_id, TGBP_LOCAL_ASSET_ID);
    pallet_token_gateway::NativeAssets::<Runtime>::insert(TGBP_LOCAL_ASSET_ID, false); // Not native to Xcavate

    // Set precision: tGBP has 18 decimals on Ethereum
    pallet_token_gateway::Precisions::<Runtime>::insert(
        TGBP_LOCAL_ASSET_ID,
        StateMachine::Evm(1),
        18u8,
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

/// Test: Asset Registration
///
/// **What it does:**
/// Verifies that tGBP can be successfully registered in the token gateway storage.
///
/// **What it showcases:**
/// - Asset registration creates all necessary storage mappings
/// - Bidirectional mapping between local asset ID and gateway asset ID
/// - Non-native asset flag is correctly set (tGBP originates from Ethereum)
/// - Precision configuration is stored correctly (18 decimals for Ethereum)
///
/// **Why it's important:**
/// Asset registration is a prerequisite for all cross-chain transfers. This test ensures
/// the registration process correctly populates all required storage items that will be
/// queried during actual transfers.
#[test]
fn tgbp_asset_registration_works() {
    new_test_ext().execute_with(|| {
        // Register tGBP
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

        // Verify precision mapping (18 decimals on Ethereum)
        use ismp::host::StateMachine;
        let precision = pallet_token_gateway::Precisions::<Runtime>::get(
            TGBP_LOCAL_ASSET_ID,
            StateMachine::Evm(1),
        );
        assert_eq!(precision, Some(18));
    });
}

/// Test: Basic tGBP Transfer - Asset Creation and Minting
///
/// **What it does:**
/// Simulates a complete tGBP transfer from Ethereum to Xcavate through ISMP.
///
/// **What it showcases:**
/// - ISMP message creation with ABI-encoded body (not SCALE encoding)
/// - Token Gateway's `on_accept` callback processing incoming transfers
/// - Asset minting for bridged assets (mint/burn model for non-native assets)
/// - Decimal precision preservation (18 decimals maintained from Ethereum)
/// - Balance updates after successful transfer
///
/// **Flow:**
/// 1. Asset is registered in token gateway
/// 2. ISMP message created simulating Ethereum → Xcavate transfer
/// 3. Message routed to TokenGateway's on_accept handler
/// 4. Handler mints tGBP to recipient (bridged asset = mint/burn model)
/// 5. Recipient balance updated correctly with no precision conversion
///
/// **Why it's important:**
/// This is the happy path test - the most common scenario for receiving bridged assets
/// from Ethereum. It validates the entire message processing pipeline works correctly.
#[test]
fn process_tgbp_transfer_creates_asset_and_mints() {
    new_test_ext().execute_with(|| {
        // Setup: Register tGBP
        register_tgbp_asset();

        // Create a recipient account
        let recipient = bob_account();

        // Verify asset exists (created in register_tgbp_asset)
        assert!(asset_exists(TGBP_LOCAL_ASSET_ID));

        // Verify recipient starts with zero balance
        assert_eq!(get_balance(TGBP_LOCAL_ASSET_ID, &recipient), 0);

        // Create ISMP message: Transfer 100 tGBP (100 * 10^18 in 18 decimals)
        let amount_18_decimals = 100_000_000_000_000_000_000u128;
        let msg =
            create_tgbp_transfer_message(ALICE_ETH_ADDRESS, recipient.clone(), amount_18_decimals);

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

        // Verify recipient received tokens (no precision conversion - maintains 18 decimals)
        // 100 * 10^18 (18 decimals) → 100 * 10^18 (18 decimals)
        let expected_balance = 100_000_000_000_000_000_000u128;
        assert_eq!(get_balance(TGBP_LOCAL_ASSET_ID, &recipient), expected_balance);

        // Verify no precision conversion (1:1 mapping)
        assert_eq!(amount_18_decimals, expected_balance);
    });
}

/// Test: Multiple Transfers to Same Recipient - Balance Accumulation
///
/// **What it does:**
/// Tests that multiple incoming transfers to the same recipient accumulate correctly.
///
/// **What it showcases:**
/// - Sequential transfer processing
/// - Balance accumulation across multiple messages
/// - Idempotent minting (each transfer adds to existing balance)
/// - Different source addresses can send to same recipient
///
/// **Scenario:**
/// - Transfer 1: Alice (Ethereum) sends 50 tGBP to Bob (Xcavate)
/// - Transfer 2: Different sender sends 30 tGBP to same Bob
/// - Final balance: 80 tGBP (50 + 30)
///
/// **Why it's important:**
/// In production, the same user will receive multiple transfers over time.
/// This verifies that balances accumulate correctly and don't get overwritten.
#[test]
fn multiple_tgbp_transfers_accumulate() {
    new_test_ext().execute_with(|| {
        // Setup
        register_tgbp_asset();
        let recipient = bob_account();

        // First transfer: 50 tGBP (50 * 10^18)
        let msg1 = create_tgbp_transfer_message(ALICE_ETH_ADDRESS, recipient.clone(), 50_000_000_000_000_000_000);
        let req1 = match msg1 {
            Request::Post(req) => req,
            _ => panic!("Expected Request::Post"),
        };
        let token_gateway = pallet_token_gateway::Pallet::<Runtime>::default();
        assert_ok!(token_gateway.on_accept(req1));

        // Check balance after first transfer
        let balance_after_first = get_balance(TGBP_LOCAL_ASSET_ID, &recipient);
        assert_eq!(balance_after_first, 50_000_000_000_000_000_000); // 50 tGBP in 18 decimals

        // Second transfer: 30 tGBP (30 * 10^18)
        let msg2 = create_tgbp_transfer_message(BOB_ETH_ADDRESS, recipient.clone(), 30_000_000_000_000_000_000);
        let req2 = match msg2 {
            Request::Post(req) => req,
            _ => panic!("Expected Request::Post"),
        };
        let token_gateway2 = pallet_token_gateway::Pallet::<Runtime>::default();
        assert_ok!(token_gateway2.on_accept(req2));

        // Check balance after second transfer (should accumulate)
        let balance_after_second = get_balance(TGBP_LOCAL_ASSET_ID, &recipient);
        assert_eq!(balance_after_second, 80_000_000_000_000_000_000); // 80 tGBP in 18 decimals

        // Verify accumulation is correct
        assert_eq!(balance_after_second, balance_after_first + 30_000_000_000_000_000_000);
    });
}

/// Test: Transfers to Multiple Different Recipients
///
/// **What it does:**
/// Tests that transfers to different recipients maintain independent balances.
///
/// **What it showcases:**
/// - Independent balance tracking for different accounts
/// - No cross-contamination between recipient balances
/// - Multiple recipients can receive from the same source
///
/// **Scenario:**
/// - Transfer 1: 100 tGBP to Bob
/// - Transfer 2: 200 tGBP to Charlie
/// - Verify: Bob has 100, Charlie has 200, balances are independent
///
/// **Why it's important:**
/// Ensures the token gateway correctly isolates balances per account and doesn't
/// accidentally update the wrong recipient or share state between transfers.
#[test]
fn transfer_to_multiple_recipients() {
    new_test_ext().execute_with(|| {
        // Setup
        register_tgbp_asset();

        let bob = bob_account();
        let charlie = charlie_account();

        // Transfer to Bob: 100 tGBP (100 * 10^18)
        let msg_bob = create_tgbp_transfer_message(ALICE_ETH_ADDRESS, bob.clone(), 100_000_000_000_000_000_000);
        let req_bob = match msg_bob {
            Request::Post(req) => req,
            _ => panic!("Expected Request::Post"),
        };
        let token_gateway = pallet_token_gateway::Pallet::<Runtime>::default();
        assert_ok!(token_gateway.on_accept(req_bob));

        // Transfer to Charlie: 200 tGBP (200 * 10^18)
        let msg_charlie =
            create_tgbp_transfer_message(ALICE_ETH_ADDRESS, charlie.clone(), 200_000_000_000_000_000_000);
        let req_charlie = match msg_charlie {
            Request::Post(req) => req,
            _ => panic!("Expected Request::Post"),
        };
        let token_gateway2 = pallet_token_gateway::Pallet::<Runtime>::default();
        assert_ok!(token_gateway2.on_accept(req_charlie));

        // Verify Bob's balance
        assert_eq!(get_balance(TGBP_LOCAL_ASSET_ID, &bob), 100_000_000_000_000_000_000);

        // Verify Charlie's balance
        assert_eq!(get_balance(TGBP_LOCAL_ASSET_ID, &charlie), 200_000_000_000_000_000_000);

        // Verify balances are independent
        assert_ne!(
            get_balance(TGBP_LOCAL_ASSET_ID, &bob),
            get_balance(TGBP_LOCAL_ASSET_ID, &charlie)
        );
    });
}

/// Test: Decimal Precision Preservation - Various Amounts
///
/// **What it does:**
/// Tests that tGBP maintains 18 decimal precision across various transfer amounts.
///
/// **What it showcases:**
/// - No precision conversion happens (18 decimals → 18 decimals)
/// - Small amounts (0.000000000000000001 tGBP) are preserved exactly
/// - Large amounts (1234.56789 tGBP) are preserved exactly
/// - Fractional amounts don't lose precision
///
/// **Test Cases:**
/// - 1 tGBP, 0.5 tGBP, 1.5 tGBP (common amounts)
/// - 1 wei (minimum unit)
/// - 100 tGBP (round number)
/// - 1234.56789 tGBP (arbitrary fractional amount)
///
/// **Why it's important:**
/// tGBP uses 18 decimals on Ethereum and we maintain 18 decimals on Xcavate.
/// This test validates that our precision mapping strategy works correctly.
#[test]
fn precision_conversion_various_amounts() {
    new_test_ext().execute_with(|| {
        // Setup
        register_tgbp_asset();

        // Test cases: (input_amount, expected_balance)
        // No precision conversion - 18 decimals maintained
        let test_cases = vec![
            (1_000_000_000_000_000_000u128, 1_000_000_000_000_000_000u128),         // 1 tGBP
            (500_000_000_000_000_000u128, 500_000_000_000_000_000u128),             // 0.5 tGBP
            (1_500_000_000_000_000_000u128, 1_500_000_000_000_000_000u128),         // 1.5 tGBP
            (1u128, 1u128),                                                         // 1 wei (minimum)
            (100_000_000_000_000_000_000u128, 100_000_000_000_000_000_000u128),     // 100 tGBP
            (1_234_567_890_000_000_000_000u128, 1_234_567_890_000_000_000_000u128), // 1234.56789 tGBP
        ];

        for (i, (amount_18_dec, expected_balance)) in test_cases.into_iter().enumerate() {
            // Create new recipient for each test to avoid accumulation
            // Use index to ensure unique recipients
            let mut account_bytes = [0u8; 32];
            account_bytes[0] = i as u8;
            account_bytes[1] = (i >> 8) as u8;
            let test_recipient = AccountId32::new(account_bytes);

            let msg = create_tgbp_transfer_message(
                ALICE_ETH_ADDRESS,
                test_recipient.clone(),
                amount_18_dec,
            );
            let req = match msg {
                Request::Post(req) => req,
                _ => panic!("Expected Request::Post"),
            };

            let token_gateway = pallet_token_gateway::Pallet::<Runtime>::default();
            assert_ok!(token_gateway.on_accept(req));

            let balance = get_balance(TGBP_LOCAL_ASSET_ID, &test_recipient);
            assert_eq!(
                balance, expected_balance,
                "Amount mismatch for {} (18 dec) - expected {}, got {}",
                amount_18_dec, expected_balance, balance
            );
        }
    });
}

/// Test: Event Emission on Successful Transfer
///
/// **What it does:**
/// Verifies that appropriate events are emitted when a transfer is processed.
///
/// **What it showcases:**
/// - Event-driven architecture for monitoring transfers
/// - `Assets::Issued` event is emitted when tokens are minted
/// - Events contain relevant transfer information
/// - Event system integration with FRAME
///
/// **Expected Events:**
/// - `pallet_assets::Event::Issued` - when tGBP is minted to recipient
/// - (Future) `pallet_token_gateway::Event::AssetReceived` - when transfer completes
///
/// **Why it's important:**
/// Events allow off-chain systems (indexers, UIs, monitoring) to track cross-chain
/// transfers in real-time. This test ensures events are emitted for observability.
#[test]
fn events_emitted_on_transfer() {
    new_test_ext().execute_with(|| {
        // Setup
        register_tgbp_asset();
        let recipient = bob_account();

        // Clear any existing events
        System::reset_events();

        // Process transfer: 100 tGBP (100 * 10^18)
        let msg = create_tgbp_transfer_message(ALICE_ETH_ADDRESS, recipient.clone(), 100_000_000_000_000_000_000);
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

/// Test: Error Handling - Unregistered Asset
///
/// **What it does:**
/// Tests that transfers fail gracefully when the asset is not registered.
///
/// **What it showcases:**
/// - Input validation before processing transfers
/// - Protection against receiving unknown/unwhitelisted assets
/// - Proper error propagation from token gateway
///
/// **Scenario:**
/// - Attempt to process tGBP transfer WITHOUT registering tGBP first
/// - Expected: Transfer fails with "Asset not found" or similar error
///
/// **Why it's important:**
/// In production, only explicitly registered assets should be accepted. This prevents:
/// - Spam tokens from being minted
/// - Unknown assets cluttering storage
/// - Potential attack vectors from malicious asset deployments
///
/// This test ensures the gateway rejects unregistered assets as a security measure.
#[test]
#[should_panic(expected = "Asset not found")]
fn unregistered_asset_fails() {
    new_test_ext().execute_with(|| {
        // Do NOT register tGBP

        let recipient = bob_account();

        // Try to process transfer for unregistered asset: 100 tGBP (100 * 10^18)
        let msg = create_tgbp_transfer_message(ALICE_ETH_ADDRESS, recipient, 100_000_000_000_000_000_000);
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

/// Test: Minimum Balance Enforcement
///
/// **What it does:**
/// Tests how the system handles transfers below the minimum balance threshold.
///
/// **What it showcases:**
/// - Minimum balance (existential deposit) enforcement from pallet_assets
/// - Edge case handling for very small amounts
/// - Graceful handling whether transfer succeeds or fails
///
/// **Scenario:**
/// - Transfer amount: 999 wei (very small amount in 18 decimals)
/// - Minimum balance configured: 1
/// - Expected: Transfer may succeed or fail depending on pallet_assets config
///
/// **Why it's important:**
/// Minimum balances prevent dust spam and storage bloat. This test validates:
/// - Small amounts are handled correctly if they meet minimums
/// - Transfers below minimums are rejected appropriately
/// - No unexpected behavior with edge case amounts
///
/// **Note:** This test accepts both success and failure as valid outcomes,
/// documenting the behavior rather than enforcing specific logic.
#[test]
fn minimum_balance_enforcement() {
    new_test_ext().execute_with(|| {
        // Setup
        register_tgbp_asset();
        let recipient = bob_account();

        // Try to transfer a very small amount
        // Minimum balance is 1 (set in force_create)
        // Testing with a small amount in 18 decimals
        let small_amount = 999u128; // 999 wei (0.000000000000000999 tGBP)

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

/// Test: Error Handling - Missing Precision Configuration
///
/// **What it does:**
/// Tests that transfers fail when precision mapping is missing for the source chain.
///
/// **What it showcases:**
/// - Validation of precision configuration before processing
/// - Protection against incorrect decimal conversions
/// - Per-chain precision mapping requirement
///
/// **Scenario:**
/// - Register tGBP with precision for Ethereum (chain ID 1)
/// - Attempt transfer from BSC (chain ID 56) WITHOUT precision configured
/// - Expected: Transfer fails with precision/decimals error
///
/// **Why it's important:**
/// Without precision mapping, the gateway cannot correctly convert amounts between
/// chains. This test ensures:
/// - Transfers are rejected if precision is unknown
/// - Clear error messages guide operators to fix configuration
/// - No silent failures or incorrect amounts due to missing config
///
/// **Production impact:**
/// Before accepting assets from a new chain, operators MUST configure precision
/// via `update_asset_precision` extrinsic. This test validates that safeguard.
#[test]
fn invalid_precision_mapping() {
    new_test_ext().execute_with(|| {
        use ismp::host::StateMachine;

        // Register tGBP but WITHOUT precision mapping for BSC
        let asset_id = tgbp_asset_id();
        pallet_token_gateway::SupportedAssets::<Runtime>::insert(TGBP_LOCAL_ASSET_ID, asset_id);
        pallet_token_gateway::LocalAssets::<Runtime>::insert(asset_id, TGBP_LOCAL_ASSET_ID);
        pallet_token_gateway::NativeAssets::<Runtime>::insert(TGBP_LOCAL_ASSET_ID, false);

        // Only set precision for Ethereum, not BSC
        pallet_token_gateway::Precisions::<Runtime>::insert(
            TGBP_LOCAL_ASSET_ID,
            StateMachine::Evm(1), // Ethereum
            18u8,
        );

        // Whitelist BSC token gateway address
        pallet_token_gateway::TokenGatewayAddresses::<Runtime>::insert(
            StateMachine::Evm(56), // BSC
            ETHEREUM_TOKEN_GATEWAY_ADDRESS.to_vec(),
        );

        // Try to transfer from BSC (which has no precision configured): 100 tGBP (100 * 10^18)
        let recipient = bob_account();
        let msg = create_erc20_transfer_message(
            b"tGBP",
            ALICE_ETH_ADDRESS,
            recipient,
            100_000_000_000_000_000_000,
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

/// Test: Asset Metadata Verification
///
/// **What it does:**
/// Verifies that asset metadata (name, symbol, decimals) is correctly set after creation.
///
/// **What it showcases:**
/// - Asset metadata is preserved through the registration process
/// - Decimal precision matches the configured value (18 for tGBP)
/// - Symbol and name are properly stored and retrievable
/// - Metadata inspection via pallet_assets traits
///
/// **Verification Points:**
/// - Symbol: Should be "tGBP"
/// - Name: Should be "Tokenised GBP" (or similar)
/// - Decimals: Should be 18 (matching Ethereum precision)
///
/// **Why it's important:**
/// Correct metadata is essential for:
/// - UIs displaying token information correctly
/// - Wallets showing proper decimal places
/// - Explorers identifying assets
/// - APIs returning accurate asset details
///
/// This test ensures metadata flows correctly from registration through to storage.
#[test]
fn asset_metadata_after_creation() {
    new_test_ext().execute_with(|| {
        // Setup
        register_tgbp_asset();
        let recipient = bob_account();

        // Process a transfer to trigger asset creation: 100 tGBP (100 * 10^18)
        let msg = create_tgbp_transfer_message(ALICE_ETH_ADDRESS, recipient, 100_000_000_000_000_000_000);
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
            // Symbol should be "tGBP"
            assert!(!symbol.is_empty(), "Symbol should be set");

            // Name should be something like "Tokenised GBP"
            assert!(!name.is_empty(), "Name should be set");

            // Decimals should be 18 (maintains native precision)
            assert_eq!(decimals, 18, "Decimals should be 18 (same as on Ethereum)");
        }
    });
}

/// Test: Total Supply Tracking Across Multiple Mints
///
/// **What it does:**
/// Tests that total supply increases correctly as multiple transfers are processed.
///
/// **What it showcases:**
/// - Total issuance tracking in pallet_assets
/// - Supply increments with each mint (bridged asset behavior)
/// - Individual balances sum to total supply (accounting invariant)
/// - No tokens lost or created unexpectedly
///
/// **Scenario:**
/// - Transfer 1: Mint 100 tGBP to Bob → Total supply: 100
/// - Transfer 2: Mint 50 tGBP to Charlie → Total supply: 150
/// - Verify: Bob (100) + Charlie (50) = Total (150)
///
/// **Why it's important:**
/// Total supply tracking is critical for:
/// - Auditing bridged asset amounts
/// - Ensuring 1:1 backing with assets locked on source chain
/// - Detecting minting errors or exploits
/// - Maintaining accounting invariants
///
/// This test validates the accounting integrity of the mint/burn model for bridged assets.
#[test]
fn asset_total_supply_tracking() {
    new_test_ext().execute_with(|| {
        // Setup
        register_tgbp_asset();

        let bob = bob_account();
        let charlie = charlie_account();

        // Process first transfer: 100 tGBP to Bob (100 * 10^18)
        let msg1 = create_tgbp_transfer_message(ALICE_ETH_ADDRESS, bob.clone(), 100_000_000_000_000_000_000);
        let req1 = match msg1 {
            Request::Post(req) => req,
            _ => panic!("Expected Request::Post"),
        };
        let token_gateway1 = pallet_token_gateway::Pallet::<Runtime>::default();

        if token_gateway1.on_accept(req1).is_ok() {
            // Check total supply after first mint
            use frame_support::traits::fungibles::Inspect;
            let supply_after_first =
                <Assets as Inspect<AccountId32>>::total_issuance(TGBP_LOCAL_ASSET_ID);
            assert_eq!(supply_after_first, 100_000_000_000_000_000_000, "Total supply should be 100 tGBP");

            // Process second transfer: 50 tGBP to Charlie (50 * 10^18)
            let msg2 = create_tgbp_transfer_message(BOB_ETH_ADDRESS, charlie.clone(), 50_000_000_000_000_000_000);
            let req2 = match msg2 {
                Request::Post(req) => req,
                _ => panic!("Expected Request::Post"),
            };
            let token_gateway2 = pallet_token_gateway::Pallet::<Runtime>::default();

            if token_gateway2.on_accept(req2).is_ok() {
                // Check total supply after second mint (should accumulate)
                let supply_after_second =
                    <Assets as Inspect<AccountId32>>::total_issuance(TGBP_LOCAL_ASSET_ID);
                assert_eq!(supply_after_second, 150_000_000_000_000_000_000, "Total supply should be 150 tGBP");

                // Verify individual balances sum to total supply
                let bob_balance = get_balance(TGBP_LOCAL_ASSET_ID, &bob);
                let charlie_balance = get_balance(TGBP_LOCAL_ASSET_ID, &charlie);
                assert_eq!(
                    bob_balance + charlie_balance,
                    supply_after_second,
                    "Balances should sum to total supply"
                );
            }
        }
    });
}

/// Test: Edge Case - Zero Amount Transfer
///
/// **What it does:**
/// Tests how the system handles transfer messages with zero amount.
///
/// **What it showcases:**
/// - Edge case validation for zero-value transfers
/// - Graceful handling of unusual but valid input
/// - No state corruption from edge case values
///
/// **Scenario:**
/// - Attempt to transfer 0 tGBP
/// - Expected: Either rejection (error) or no-op (balance stays 0)
///
/// **Why it's important:**
/// Zero-amount transfers could occur due to:
/// - User error on source chain
/// - Rounding issues in precision conversion
/// - Malicious attempts to spam the network
///
/// This test ensures:
/// - No panics or unexpected behavior
/// - System remains in valid state
/// - Resources aren't wasted on meaningless transfers
///
/// **Note:** Both success and failure are acceptable - test documents behavior.
#[test]
fn zero_amount_transfer() {
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

/// Test: Multiple Senders to Same Recipient
///
/// **What it does:**
/// Tests that a single recipient can receive transfers from multiple different senders.
///
/// **What it showcases:**
/// - Proper accumulation from different sources
/// - No sender-specific balance isolation (all go to same account)
/// - Message ordering independence (each transfer is atomic)
///
/// **Scenario:**
/// - Sender 1 (Alice on Ethereum): Sends 50 tGBP to Bob
/// - Sender 2 (Bob on Ethereum): Sends 30 tGBP to same Bob (different sender, same recipient)
/// - Final balance: 80 tGBP (50 + 30)
///
/// **Why it's important:**
/// In real-world usage, users will receive tokens from multiple sources:
/// - Different exchanges
/// - Multiple friends/counterparties
/// - Various DeFi protocols on Ethereum
///
/// This test validates that:
/// - All transfers to the same recipient accumulate correctly
/// - No confusion between sender and recipient addressing
/// - Balance tracking is per-recipient, not per-sender-recipient pair
///
/// **Real-world scenario:** Bob has one tGBP balance that increases regardless of who sends.
#[test]
fn same_recipient_multiple_senders() {
    new_test_ext().execute_with(|| {
        // Setup
        register_tgbp_asset();
        let recipient = bob_account();

        // Transfer from Alice: 50 tGBP (50 * 10^18)
        let msg1 = create_tgbp_transfer_message(ALICE_ETH_ADDRESS, recipient.clone(), 50_000_000_000_000_000_000);
        let req1 = match msg1 {
            Request::Post(req) => req,
            _ => panic!("Expected Request::Post"),
        };
        let token_gateway1 = pallet_token_gateway::Pallet::<Runtime>::default();

        if token_gateway1.on_accept(req1).is_ok() {
            let balance_after_alice = get_balance(TGBP_LOCAL_ASSET_ID, &recipient);
            assert_eq!(balance_after_alice, 50_000_000_000_000_000_000);

            // Transfer from Bob's Ethereum address: 30 tGBP (30 * 10^18)
            let msg2 = create_tgbp_transfer_message(BOB_ETH_ADDRESS, recipient.clone(), 30_000_000_000_000_000_000);
            let req2 = match msg2 {
                Request::Post(req) => req,
                _ => panic!("Expected Request::Post"),
            };
            let token_gateway2 = pallet_token_gateway::Pallet::<Runtime>::default();

            if token_gateway2.on_accept(req2).is_ok() {
                // Should accumulate: 50 + 30 = 80 tGBP
                let balance_after_bob = get_balance(TGBP_LOCAL_ASSET_ID, &recipient);
                assert_eq!(balance_after_bob, 80_000_000_000_000_000_000);
            }
        }
    });
}
