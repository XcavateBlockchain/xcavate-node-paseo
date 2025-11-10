use alloc::{boxed::Box, vec::Vec};

use frame_support::{parameter_types, traits::Get, PalletId};
use frame_system::EnsureRoot;
use ismp::{host::StateMachine, module::IsmpModule, router::IsmpRouter};
use sp_runtime::traits::AccountIdConversion;

use crate::{
    weights, AccountId, Assets, Balance, Balances, Hyperbridge, Ismp, IsmpParachain, Runtime,
    RuntimeEvent, Timestamp, TokenGateway,
};

/// Decimals of the native currency XCAV.
pub const NATIVE_DECIMALS: u8 = 12;

// Temporary definition of treasury `PalletId` until `pallet_treasury` is present in the runtime.
const TREASURY_PALLET_ID: PalletId = PalletId(*b"py/trsry");

parameter_types! {
    // /// Hyperbridge parachain on Polkadot
    // pub const Coprocessor: Option<StateMachine> = Some(StateMachine::Polkadot(3367));
    //
    // /// The host state machine of this pallet on Polkadot
    // pub const HostStateMachine: StateMachine = StateMachine::Polkadot(3413);

    /// Hyperbridge parachain on Paseo.
    pub const Coprocessor: Option<StateMachine> = Some(StateMachine::Kusama(4009));

    /// The host state machine of this pallet on Paseo.
    pub const HostStateMachine: StateMachine = StateMachine::Kusama(4683);
}

impl pallet_ismp::Config for Runtime {
    // Modify the consensus client's permissions
    type AdminOrigin = EnsureRoot<AccountId>;
    type Balance = Balance;
    // A tuple of types implementing the ConsensusClient interface,
    // which defines all consensus algorithms supported by this protocol deployment
    type ConsensusClients = (ismp_parachain::ParachainConsensusClient<Runtime, IsmpParachain>,);
    // Co-processor
    type Coprocessor = Coprocessor;
    // The token used to collect fees, only XCAV is supported
    type Currency = Balances;
    /// Fee handling implementation for ISMP message processing.
    type FeeHandler = pallet_ismp::fee_handler::WeightFeeHandler<()>;
    // The state machine identifier of the chain. Its parachain id.
    type HostStateMachine = HostStateMachine;
    /// Offchain database implementation. Outgoing requests and responses are inserted in this database,
    /// while their commitments are stored onchain.
    type OffchainDB = ();
    // The router provides the implementation for the IsmpModule as the module id.
    type Router = IsmpModuleRouter;
    /// The overarching event type.
    type RuntimeEvent = RuntimeEvent;
    type TimestampProvider = Timestamp;
}
/// Implementation for routing requests & responses to their appropriate modules.
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
    // `pallet-ismp` implements `IsmpHost`.
    type IsmpHost = Ismp;
    /// Origin for privileged actions.
    type RootOrigin = EnsureRoot<AccountId>;
    /// The overarching event type.
    type RuntimeEvent = RuntimeEvent;
    type WeightInfo = weights::ismp_parachain::WeightInfo<Runtime>;
}

impl pallet_hyperbridge::Config for Runtime {
    // `IsmpHost` implementation provided by `pallet_ismp`.
    type IsmpHost = Ismp;
    /// The overarching event type.
    type RuntimeEvent = RuntimeEvent;
}

parameter_types! {
    pub const NativeTokenDecimals: u8 = NATIVE_DECIMALS;
    /// Id for the local asset in `Assets` corresponding to Xcavate's native token.
    // Token Id `0` should be reserved for the native token representation.
    pub const NativeAssetId: u32 = 0;
}

/// Provides an account that would be set as asset admin and also cover fees for asset creation.
pub struct AssetAdmin;
impl Get<AccountId> for AssetAdmin {
    fn get() -> AccountId {
        // TODO: Once `pallet_treasury` is present in the runtime this can be substituted by
        // `Treasury::account_id()`
        TREASURY_PALLET_ID.into_account_truncating()
    }
}
impl pallet_token_gateway::Config for Runtime {
    type AssetAdmin = AssetAdmin;
    type Assets = Assets;
    #[cfg(not(feature = "runtime-benchmarks"))]
    type CreateOrigin = EnsureRoot<AccountId>;
    #[cfg(feature = "runtime-benchmarks")]
    type CreateOrigin = frame_system::EnsureSigned<AccountId>;
    type Decimals = NativeTokenDecimals;
    type Dispatcher = Hyperbridge;
    type EvmToSubstrate = ();
    type NativeAssetId = NativeAssetId;
    type NativeCurrency = Balances;
    /// The overarching event type.
    type RuntimeEvent = RuntimeEvent;
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
        fn ensure_currency_type() {
            assert_eq!(
                TypeId::of::<<Runtime as pallet_ismp::Config>::Currency>(),
                TypeId::of::<Balances>(),
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
            let expected_account = TREASURY_PALLET_ID.into_account_truncating();
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
