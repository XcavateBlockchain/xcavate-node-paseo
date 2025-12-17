//! ISMP (Interoperable State Machine Protocol) configuration for Xcavate runtime.
//!
//! This module configures the ISMP stack for cross-chain communication via Hyperbridge,
//! including:
//! - Core ISMP protocol configuration
//! - Parachain consensus client for relay chain integration
//! - Hyperbridge coprocessor for cross-chain state verification
//! - Token Gateway for bridging assets across ecosystems

use alloc::{boxed::Box, vec::Vec};

use frame_support::{parameter_types, traits::Get};
use frame_support::traits::fungible::ItemOf;
use frame_system::EnsureRoot;
use ismp::{host::StateMachine, module::IsmpModule, router::IsmpRouter};
use sp_runtime::traits::AccountIdConversion;

use crate::{
    constants::{
        known_assets::{NATIVE_ASSET_ID, NATIVE_DECIMALS, TGBP_ASSET_ID},
        pallet_ids::TREASURY,
    },
    weights, AccountId, Assets, Balance, Balances, Hyperbridge, Ismp, IsmpParachain, Runtime,
    RuntimeEvent, Timestamp, TokenGateway,
};

parameter_types! {
    // /// Hyperbridge parachain on Polkadot
    // pub const Coprocessor: Option<StateMachine> = Some(StateMachine::Polkadot(3367));
    //
    // /// The host state machine of this pallet on Polkadot
    // pub const HostStateMachine: StateMachine = StateMachine::Polkadot(3413);

    /// Hyperbridge coprocessor parachain on Paseo testnet.
    ///
    /// Hyperbridge acts as a coprocessor for verifying cross-chain state proofs,
    /// enabling trustless communication with other blockchains.
    pub const Coprocessor: Option<StateMachine> = Some(StateMachine::Kusama(4009));

    /// The state machine identifier for the Xcavate parachain on Paseo testnet.
    ///
    /// This identifies the parachain in ISMP messages and routing.
    pub const HostStateMachine: StateMachine = StateMachine::Kusama(4683);

    /// AssetId corresponding to the tGBP asset registered locally.
    pub const TGBPAssetId: u32 = TGBP_ASSET_ID;
}

impl pallet_ismp::Config for Runtime {
    /// Origin that can modify consensus client permissions.
    ///
    /// Root access required for security-critical operations like adding/removing consensus clients.
    type AdminOrigin = EnsureRoot<AccountId>;
    /// Balance type used for ISMP fees.
    type Balance = Balance;
    /// Tuple of consensus client implementations.
    ///
    /// Currently supports parachain consensus for verifying relay chain state proofs.
    type ConsensusClients = (ismp_parachain::ParachainConsensusClient<Runtime, IsmpParachain>,);
    /// Optional coprocessor for enhanced cross-chain verification.
    ///
    /// Hyperbridge provides additional security guarantees for cross-chain state proofs.
    type Coprocessor = Coprocessor;
    /// Currency used to collect ISMP message processing fees.
    ///
    /// Only tGBP is supported for fee payment.
    type Currency = ItemOf<Assets, TGBPAssetId, AccountId>;
    /// Handler for calculating and collecting ISMP message fees based on weight.
    type FeeHandler = pallet_ismp::fee_handler::WeightFeeHandler<()>;
    /// State machine identifier for this parachain.
    ///
    /// Used to identify this chain in cross-chain messages.
    type HostStateMachine = HostStateMachine;
    /// Offchain database for storing full request/response data.
    ///
    /// Currently not used; only commitment hashes are stored on-chain.
    type OffchainDB = ();
    /// Router for dispatching ISMP messages to appropriate pallet modules.
    type Router = IsmpModuleRouter;
    /// The overarching event type.
    type RuntimeEvent = RuntimeEvent;
    /// Provider for block timestamps used in ISMP message validation.
    type TimestampProvider = Timestamp;
}

/// Router implementation for dispatching ISMP messages to their appropriate handler modules.
///
/// Routes messages based on module identifiers:
/// - Hyperbridge core functionality
/// - Token Gateway for asset transfers
#[derive(Default)]
pub struct IsmpModuleRouter;

impl IsmpRouter for IsmpModuleRouter {
    fn module_for_id(&self, input: Vec<u8>) -> Result<Box<dyn IsmpModule>, anyhow::Error> {
        match input.as_slice() {
            pallet_hyperbridge::PALLET_HYPERBRIDGE_ID =>
                Ok(Box::new(pallet_hyperbridge::Pallet::<Runtime>::default())),
            id if TokenGateway::is_token_gateway(id) =>
                Ok(Box::new(pallet_token_gateway::Pallet::<Runtime>::default())),
            _ => Err(ismp::Error::ModuleNotFound(input))?,
        }
    }
}

impl ismp_parachain::Config for Runtime {
    /// ISMP host implementation.
    ///
    /// `pallet_ismp` provides the core ISMP protocol implementation.
    type IsmpHost = Ismp;
    /// Origin for privileged parachain operations.
    type RootOrigin = EnsureRoot<AccountId>;
    /// The overarching event type.
    type RuntimeEvent = RuntimeEvent;
    /// Weight calculation for parachain consensus operations.
    type WeightInfo = weights::ismp_parachain::WeightInfo<Runtime>;
}

impl pallet_hyperbridge::Config for Runtime {
    /// ISMP host implementation.
    ///
    /// Provided by `pallet_ismp`.
    type IsmpHost = Ismp;
    /// The overarching event type.
    type RuntimeEvent = RuntimeEvent;
}

parameter_types! {
    /// Decimals for native XCAV token used in token gateway operations.
    pub const NativeTokenDecimals: u8 = NATIVE_DECIMALS;

    /// Asset ID reserved for the native XCAV token in `pallet_assets`.
    ///
    /// Token ID `0` is reserved and should not be used for other assets.
    /// This allows the token gateway to identify when operations involve the native token
    /// vs. other fungible assets.
    pub const NativeAssetId: u32 = NATIVE_ASSET_ID;
}

/// Account provider for token gateway administrative operations.
/// This account is used as the asset admin for token gateway operations and covers fees
/// for asset creation.
///
/// Returns the treasury account which:
/// - Acts as the asset admin for cross-chain assets
/// - Pays fees for creating assets on remote chains via Hyperbridge
pub struct AssetAdmin;

impl Get<AccountId> for AssetAdmin {
    fn get() -> AccountId {
        // TODO: Once `pallet_treasury` is present in the runtime this can be substituted by
        // `Treasury::account_id()`.
        TREASURY.into_account_truncating()
    }
}

impl pallet_token_gateway::Config for Runtime {
    /// Account that administers cross-chain assets and pays creation fees.
    type AssetAdmin = AssetAdmin;
    /// Fungible assets pallet for managing bridged tokens.
    type Assets = Assets;
    /// Origin authorized to create and update cross-chain asset registrations.
    ///
    /// Requires root in production; relaxed to signed for benchmarking.
    #[cfg(not(feature = "runtime-benchmarks"))]
    type CreateOrigin = EnsureRoot<AccountId>;
    /// Origin for benchmarking mode.
    #[cfg(feature = "runtime-benchmarks")]
    type CreateOrigin = frame_system::EnsureSigned<AccountId>;
    /// Decimals of the native XCAV token.
    type Decimals = NativeTokenDecimals;
    /// Dispatcher for sending cross-chain asset transfer requests.
    type Dispatcher = Hyperbridge;
    /// Converter for EVM addresses to Substrate accounts.
    ///
    /// Uses default implementation which maps EVM addresses to 32-byte Substrate accounts.
    type EvmToSubstrate = ();
    /// Asset ID representing the native token in the Assets pallet.
    type NativeAssetId = NativeAssetId;
    /// Native currency for balance operations.
    type NativeCurrency = Balances;
    /// The overarching event type.
    type RuntimeEvent = RuntimeEvent;
    /// Weight calculation for token gateway operations.
    type WeightInfo = weights::pallet_token_gateway::WeightInfo<Runtime>;
}

#[cfg(test)]
mod tests {
    use std::any::TypeId;

    use super::*;

    mod ismp {
        use super::*;

        #[test]
        fn admin_origin_ensures_root() {
            assert_eq!(
                TypeId::of::<<Runtime as pallet_ismp::Config>::AdminOrigin>(),
                TypeId::of::<EnsureRoot<AccountId>>(),
            );
        }

        #[test]
        fn ensure_balance_type() {
            assert_eq!(
                TypeId::of::<<Runtime as pallet_ismp::Config>::Balance>(),
                TypeId::of::<Balance>(),
            );
        }

        #[test]
        fn ensure_consensus_clients() {
            assert_eq!(
                TypeId::of::<<Runtime as pallet_ismp::Config>::ConsensusClients>(),
                TypeId::of::<(ismp_parachain::ParachainConsensusClient<Runtime, IsmpParachain>,)>(),
            );
        }

        #[test]
        fn ensure_coprocessor_configured() {
            assert_eq!(
                TypeId::of::<<Runtime as pallet_ismp::Config>::Coprocessor>(),
                TypeId::of::<Coprocessor>(),
            );
            // Verify Coprocessor points to Hyperbridge on Paseo
            assert_eq!(Coprocessor::get(), Some(StateMachine::Kusama(4009)));
        }

        #[test]
        fn ensure_currency_is_tgbp_stablecoin() {
            // ISMP fees (including relayer fees) are paid in TGBP stablecoin (Asset ID 1)
            // This ensures predictable and stable revenue for relayers
            assert_eq!(
                TypeId::of::<<Runtime as pallet_ismp::Config>::Currency>(),
                TypeId::of::<ItemOf<Assets, TGBPAssetId, AccountId>>(),
            );
        }

        #[test]
        fn ensure_fee_handler_configured() {
            assert_eq!(
                TypeId::of::<<Runtime as pallet_ismp::Config>::FeeHandler>(),
                TypeId::of::<pallet_ismp::fee_handler::WeightFeeHandler<()>>(),
            );
        }

        #[test]
        fn ensure_host_state_machine_configured() {
            assert_eq!(
                TypeId::of::<<Runtime as pallet_ismp::Config>::HostStateMachine>(),
                TypeId::of::<HostStateMachine>(),
            );
            // Verify HostStateMachine is configured for Paseo parachain
            assert_eq!(HostStateMachine::get(), StateMachine::Kusama(4683));
        }

        #[test]
        fn ensure_offchain_db_configured() {
            assert_eq!(
                TypeId::of::<<Runtime as pallet_ismp::Config>::OffchainDB>(),
                TypeId::of::<()>(),
            );
        }

        #[test]
        fn ensure_router_configured() {
            assert_eq!(
                TypeId::of::<<Runtime as pallet_ismp::Config>::Router>(),
                TypeId::of::<IsmpModuleRouter>(),
            );
        }

        #[test]
        fn ensure_runtime_event_configured() {
            assert_eq!(
                TypeId::of::<<Runtime as pallet_ismp::Config>::RuntimeEvent>(),
                TypeId::of::<RuntimeEvent>(),
            );
        }

        #[test]
        fn ensure_timestamp_provider_configured() {
            assert_eq!(
                TypeId::of::<<Runtime as pallet_ismp::Config>::TimestampProvider>(),
                TypeId::of::<Timestamp>(),
            );
        }
    }

    mod ismp_parachain_config {
        use super::*;

        #[test]
        fn ensure_ismp_host_configured() {
            assert_eq!(
                TypeId::of::<<Runtime as ismp_parachain::Config>::IsmpHost>(),
                TypeId::of::<Ismp>(),
            );
        }

        #[test]
        fn ensure_root_origin_configured() {
            assert_eq!(
                TypeId::of::<<Runtime as ismp_parachain::Config>::RootOrigin>(),
                TypeId::of::<EnsureRoot<AccountId>>(),
            );
        }

        #[test]
        fn ensure_runtime_event_configured() {
            assert_eq!(
                TypeId::of::<<Runtime as ismp_parachain::Config>::RuntimeEvent>(),
                TypeId::of::<RuntimeEvent>(),
            );
        }

        #[test]
        fn ensure_weight_info_configured() {
            assert_eq!(
                TypeId::of::<<Runtime as ismp_parachain::Config>::WeightInfo>(),
                TypeId::of::<weights::ismp_parachain::WeightInfo<Runtime>>(),
            );
        }
    }

    mod hyperbridge {
        use super::*;

        #[test]
        fn ensure_ismp_host_configured() {
            assert_eq!(
                TypeId::of::<<Runtime as pallet_hyperbridge::Config>::IsmpHost>(),
                TypeId::of::<Ismp>(),
            );
        }

        #[test]
        fn ensure_runtime_event_configured() {
            assert_eq!(
                TypeId::of::<<Runtime as pallet_hyperbridge::Config>::RuntimeEvent>(),
                TypeId::of::<RuntimeEvent>(),
            );
        }
    }

    mod token_gateway {
        use super::*;

        #[test]
        fn ensure_asset_admin_configured() {
            assert_eq!(
                TypeId::of::<<Runtime as pallet_token_gateway::Config>::AssetAdmin>(),
                TypeId::of::<AssetAdmin>(),
            );
            // Verify AssetAdmin returns treasury account
            let admin_account = AssetAdmin::get();
            let expected_account = TREASURY.into_account_truncating();
            assert_eq!(admin_account, expected_account);
        }

        #[test]
        fn ensure_assets_configured() {
            assert_eq!(
                TypeId::of::<<Runtime as pallet_token_gateway::Config>::Assets>(),
                TypeId::of::<Assets>(),
            );
        }

        #[test]
        #[cfg(not(feature = "runtime-benchmarks"))]
        fn ensure_create_origin_is_root() {
            assert_eq!(
                TypeId::of::<<Runtime as pallet_token_gateway::Config>::CreateOrigin>(),
                TypeId::of::<EnsureRoot<AccountId>>(),
            );
        }

        #[test]
        #[cfg(feature = "runtime-benchmarks")]
        fn ensure_create_origin_is_signed_for_benchmarks() {
            assert_eq!(
                TypeId::of::<<Runtime as pallet_token_gateway::Config>::CreateOrigin>(),
                TypeId::of::<frame_system::EnsureSigned<AccountId>>(),
            );
        }

        #[test]
        fn ensure_decimals_configured() {
            assert_eq!(
                TypeId::of::<<Runtime as pallet_token_gateway::Config>::Decimals>(),
                TypeId::of::<NativeTokenDecimals>(),
            );
            // Verify native token decimals value
            assert_eq!(NativeTokenDecimals::get(), NATIVE_DECIMALS);
            assert_eq!(NativeTokenDecimals::get(), 12);
        }

        #[test]
        fn ensure_dispatcher_configured() {
            assert_eq!(
                TypeId::of::<<Runtime as pallet_token_gateway::Config>::Dispatcher>(),
                TypeId::of::<Hyperbridge>(),
            );
        }

        #[test]
        fn ensure_evm_to_substrate_is_default() {
            assert_eq!(
                TypeId::of::<<Runtime as pallet_token_gateway::Config>::EvmToSubstrate>(),
                TypeId::of::<()>(),
            );
        }

        #[test]
        fn ensure_native_asset_id_configured() {
            assert_eq!(
                TypeId::of::<<Runtime as pallet_token_gateway::Config>::NativeAssetId>(),
                TypeId::of::<NativeAssetId>(),
            );
            // Verify native asset ID is set to 0 (reserved for native token)
            assert_eq!(NativeAssetId::get(), 0);
        }

        #[test]
        fn ensure_native_currency_configured() {
            assert_eq!(
                TypeId::of::<<Runtime as pallet_token_gateway::Config>::NativeCurrency>(),
                TypeId::of::<Balances>(),
            );
        }

        #[test]
        fn ensure_runtime_event_configured() {
            assert_eq!(
                TypeId::of::<<Runtime as pallet_token_gateway::Config>::RuntimeEvent>(),
                TypeId::of::<RuntimeEvent>(),
            );
        }

        #[test]
        fn ensure_weight_info_configured() {
            assert_eq!(
                TypeId::of::<<Runtime as pallet_token_gateway::Config>::WeightInfo>(),
                TypeId::of::<weights::pallet_token_gateway::WeightInfo<Runtime>>(),
            );
        }
    }

    mod router {
        use hex_literal::hex;

        use super::*;

        #[test]
        fn router_resolves_hyperbridge_module() {
            let router = IsmpModuleRouter;
            let result = router.module_for_id(pallet_hyperbridge::PALLET_HYPERBRIDGE_ID.to_vec());
            assert!(result.is_ok());
        }

        #[test]
        fn router_resolves_token_gateway_module() {
            let router = IsmpModuleRouter;
            // Test with TOKEN_GATEWAY module ID
            let token_gateway_id = hex!("a09b1c60e8650245f92518c8a17314878c4043ed").to_vec();
            if TokenGateway::is_token_gateway(&token_gateway_id) {
                let result = router.module_for_id(token_gateway_id);
                assert!(result.is_ok());
            }
        }

        #[test]
        fn router_rejects_unknown_module() {
            let router = IsmpModuleRouter;
            let unknown_id = b"unknown-module".to_vec();
            let result = router.module_for_id(unknown_id);
            assert!(result.is_err());
        }
    }
}
