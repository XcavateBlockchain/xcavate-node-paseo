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
    /// Id for an asset in `Assets` corresponding to Xcavate's native token.
    // Reserve ID `0` for the native token representation
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
