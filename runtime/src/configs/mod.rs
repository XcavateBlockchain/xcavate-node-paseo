pub mod ismp;
pub mod xcm_config;

use cumulus_pallet_parachain_system::RelayNumberMonotonicallyIncreases;
use cumulus_primitives_core::{AggregateMessageOrigin, ParaId};
use frame_support::{
    derive_impl,
    dispatch::DispatchClass,
    instances::{Instance1, Instance2},
    parameter_types,
    traits::{
        AsEnsureOriginWithArg, ConstU32, ConstU64, Contains, EnsureOriginWithArg, InstanceFilter,
        MapSuccess, OriginTrait, TransformOrigin, WithdrawReasons,
    },
    weights::{ConstantMultiplier, Weight},
    BoundedVec, PalletId,
};
use frame_system::{
    limits::{BlockLength, BlockWeights},
    EnsureRoot, EnsureRootWithSuccess, EnsureSigned,
};
use pallet_nfts::PalletFeatures;
use parachains_common::message_queue::{NarrowOriginToSibling, ParaIdToSibling};
use parity_scale_codec::{Decode, DecodeWithMemTracking, Encode, MaxEncodedLen};
use polkadot_runtime_common::{BlockHashCount, SlowAdjustingFeeUpdate};
use scale_info::TypeInfo;
use sp_consensus_aura::sr25519::AuthorityId as AuraId;
use sp_core::ConstBool;
use sp_runtime::{
    traits::{AccountIdLookup, BlakeTwo256, ConvertInto, Morph, Verify},
    MultiSignature, Perbill, Percent, Permill, RuntimeDebug,
};
use sp_version::RuntimeVersion;
use xcm::latest::prelude::{AssetId, BodyId};
#[cfg(not(feature = "runtime-benchmarks"))]
use xcm_builder::ProcessXcmMessage;
use xcm_config::{RelayLocation, XcmOriginToTransactDispatchOrigin};

use crate::{
    constants::{
        currency::{deposit, EXISTENTIAL_DEPOSIT, MICROXCAV, MILLIXCAV, XCAV},
        AVERAGE_ON_INITIALIZE_RATIO, DAYS, HOURS, MAXIMUM_BLOCK_WEIGHT, MAX_BLOCK_LENGTH,
        NORMAL_DISPATCH_RATIO, SLOT_DURATION, VERSION,
    },
    types::{
        AccountId, Balance, Block, BlockNumber, CollatorSelectionUpdateOrigin, ConsensusHook, Hash,
        Nonce, PriceForSiblingParachainDelivery,
    },
    weights::{self, BlockExecutionWeight, ExtrinsicBaseWeight, ParityDbWeight},
    Assets, AssetsFreezer, AssetsHolder, Aura, Balances, CollatorSelection, EducationAssets,
    EducationNfts, EducationRegions, KiltMigration, MessageQueue, OriginCaller, PalletInfo,
    ParachainSystem, PropertyManagement, RealEstateAssets, RealEstateNfts, RealWorldAsset, Regions,
    Runtime, RuntimeCall, RuntimeEvent, RuntimeFreezeReason, RuntimeHoldReason, RuntimeOrigin,
    RuntimeTask, Session, SessionKeys, System, WeightToFee, XcavateWhitelist, XcmpQueue,
};

use primitives::{AssetMetadataProvider, MarketplaceFreezeReason, MarketplaceHoldReason};

pub type Signature = MultiSignature;

parameter_types! {
    pub const Version: RuntimeVersion = VERSION;

    // This part is copied from Substrate's `bin/node/runtime/src/lib.rs`.
    //  The `RuntimeBlockLength` and `RuntimeBlockWeights` exist here because the
    // `DeletionWeightLimit` and `DeletionQueueDepth` depend on those to parameterize
    // the lazy contract deletion.
    pub RuntimeBlockLength: BlockLength =
        BlockLength::max_with_normal_ratio(MAX_BLOCK_LENGTH, NORMAL_DISPATCH_RATIO);
    pub RuntimeBlockWeights: BlockWeights = BlockWeights::builder()
        .base_block(BlockExecutionWeight::get())
        .for_class(DispatchClass::all(), |weights| {
            weights.base_extrinsic = ExtrinsicBaseWeight::get();
        })
        .for_class(DispatchClass::Normal, |weights| {
            weights.max_total = Some(NORMAL_DISPATCH_RATIO * MAXIMUM_BLOCK_WEIGHT);
        })
        .for_class(DispatchClass::Operational, |weights| {
            weights.max_total = Some(MAXIMUM_BLOCK_WEIGHT);
            // Operational transactions have some extra reserved space, so that they
            // are included even if block reached `MAXIMUM_BLOCK_WEIGHT`.
            weights.reserved = Some(
                MAXIMUM_BLOCK_WEIGHT - NORMAL_DISPATCH_RATIO * MAXIMUM_BLOCK_WEIGHT
            );
        })
        .avg_block_initialization(AVERAGE_ON_INITIALIZE_RATIO)
        .build_or_panic();
    // generic substrate prefix. For more info, see: [Polkadot Accounts In-Depth](https://wiki.polkadot.network/docs/learn-account-advanced#:~:text=The%20address%20format%20used%20in,belonging%20to%20a%20specific%20network)
    pub const SS58Prefix: u16 = 0;
}

pub struct NormalFilter;
impl Contains<RuntimeCall> for NormalFilter {
    fn contains(c: &RuntimeCall) -> bool {
        match c {
            // We filter anonymous proxy as they make "reserve" inconsistent
            // See: https://github.com/paritytech/polkadot-sdk/blob/v1.9.0-rc2/substrate/frame/proxy/src/lib.rs#L260
            RuntimeCall::Proxy(method) => !matches!(
                method,
                pallet_proxy::Call::create_pure { .. }
                    | pallet_proxy::Call::kill_pure { .. }
                    | pallet_proxy::Call::remove_proxies { .. }
            ),
            _ => true,
        }
    }
}

/// The default types are being injected by [`derive_impl`](`frame_support::derive_impl`) from
/// [`ParaChainDefaultConfig`](`struct@frame_system::config_preludes::ParaChainDefaultConfig`),
/// but overridden as needed.
#[derive_impl(frame_system::config_preludes::ParaChainDefaultConfig as frame_system::DefaultConfig)]
impl frame_system::Config for Runtime {
    /// The data to be stored in an account.
    type AccountData = pallet_balances::AccountData<Balance>;
    /// The identifier used to distinguish between accounts.
    type AccountId = AccountId;
    /// The basic call filter to use in dispatchable.
    type BaseCallFilter = NormalFilter;
    /// The block type.
    type Block = Block;
    /// Maximum number of block number to block hash mappings to keep (oldest pruned first).
    type BlockHashCount = BlockHashCount;
    /// The maximum length of a block (in bytes).
    type BlockLength = RuntimeBlockLength;
    /// Block & extrinsics weights: base values and limits.
    type BlockWeights = RuntimeBlockWeights;
    /// The weight of database operations that the runtime can invoke.
    type DbWeight = ParityDbWeight;
    /// The type for hashing blocks and tries.
    type Hash = Hash;
    /// The lookup mechanism to get account ID from whatever is passed in
    /// dispatchers.
    type Lookup = AccountIdLookup<AccountId, ()>;
    /// The maximum number of consumers allowed on a single account.
    type MaxConsumers = ConstU32<16>;
    /// The index type for storing how many extrinsics an account has signed.
    type Nonce = Nonce;
    /// The action to take on a Runtime Upgrade
    type OnSetCode = cumulus_pallet_parachain_system::ParachainSetCode<Self>;
    /// Converts a module to an index of this module in the runtime.
    type PalletInfo = PalletInfo;
    /// The aggregated dispatch type that is available for extrinsics.
    type RuntimeCall = RuntimeCall;
    /// The ubiquitous event type.
    type RuntimeEvent = RuntimeEvent;
    /// The ubiquitous origin type.
    type RuntimeOrigin = RuntimeOrigin;
    /// This is used as an identifier of the chain. 42 is the generic substrate prefix.
    type SS58Prefix = SS58Prefix;
    /// Runtime version.
    type Version = Version;
}

impl pallet_timestamp::Config for Runtime {
    type MinimumPeriod = ConstU64<0>;
    /// A timestamp: milliseconds since the unix epoch.
    type Moment = u64;
    type OnTimestampSet = Aura;
    /// Rerun benchmarks if you are making changes to runtime configuration.
    type WeightInfo = weights::pallet_timestamp::WeightInfo<Runtime>;
}

impl pallet_authorship::Config for Runtime {
    type EventHandler = (CollatorSelection,);
    type FindAuthor = pallet_session::FindAccountFromAuthorIndex<Self, Aura>;
}

parameter_types! {
    pub const MaxProxies: u32 = 32;
    pub const MaxPending: u32 = 32;
    pub const ProxyDepositBase: Balance = deposit(1, 40);
    pub const AnnouncementDepositBase: Balance = deposit(1, 48);
    pub const ProxyDepositFactor: Balance = deposit(0, 33);
    pub const AnnouncementDepositFactor: Balance = deposit(0, 66);
}

/// The type used to represent the kinds of proxying allowed.
/// If you are adding new pallets, consider adding new ProxyType variant
#[derive(
    Copy,
    Clone,
    Decode,
    DecodeWithMemTracking,
    Default,
    Encode,
    Eq,
    MaxEncodedLen,
    Ord,
    PartialEq,
    PartialOrd,
    RuntimeDebug,
    TypeInfo,
)]
pub enum ProxyType {
    /// Allows to proxy all calls
    #[default]
    Any,
    /// Allows all non-transfer calls
    NonTransfer,
    /// Allows to finish the proxy
    CancelProxy,
    /// Allows to operate with collators list (invulnerables, candidates, etc.)
    Collator,
}

impl InstanceFilter<RuntimeCall> for ProxyType {
    fn filter(&self, c: &RuntimeCall) -> bool {
        match self {
            ProxyType::Any => true,
            ProxyType::NonTransfer => !matches!(c, RuntimeCall::Balances { .. }),
            ProxyType::CancelProxy => matches!(
                c,
                RuntimeCall::Proxy(pallet_proxy::Call::reject_announcement { .. })
                    | RuntimeCall::Multisig { .. }
            ),
            ProxyType::Collator => {
                matches!(c, RuntimeCall::CollatorSelection { .. } | RuntimeCall::Multisig { .. })
            }
        }
    }
}

impl pallet_proxy::Config for Runtime {
    type AnnouncementDepositBase = AnnouncementDepositBase;
    type AnnouncementDepositFactor = AnnouncementDepositFactor;
    type BlockNumberProvider = System;
    type CallHasher = BlakeTwo256;
    type Currency = Balances;
    type MaxPending = MaxPending;
    type MaxProxies = MaxProxies;
    type ProxyDepositBase = ProxyDepositBase;
    type ProxyDepositFactor = ProxyDepositFactor;
    type ProxyType = ProxyType;
    type RuntimeCall = RuntimeCall;
    type RuntimeEvent = RuntimeEvent;
    /// Rerun benchmarks if you are making changes to runtime configuration.
    type WeightInfo = weights::pallet_proxy::WeightInfo<Runtime>;
}

parameter_types! {
    pub const ExistentialDeposit: Balance = EXISTENTIAL_DEPOSIT;
    pub const MaxFreezes: u32 = 0;
    pub const MaxLocks: u32 = 50;
    pub const MaxReserves: u32 = 50;
}

impl pallet_balances::Config for Runtime {
    type AccountStore = System;
    /// The type for recording an account's balance.
    type Balance = Balance;
    type DoneSlashHandler = ();
    type DustRemoval = ();
    type ExistentialDeposit = ExistentialDeposit;
    type FreezeIdentifier = ();
    type MaxFreezes = MaxFreezes;
    type MaxLocks = MaxLocks;
    type MaxReserves = MaxReserves;
    type ReserveIdentifier = [u8; 8];
    /// The ubiquitous event type.
    type RuntimeEvent = RuntimeEvent;
    type RuntimeFreezeReason = RuntimeFreezeReason;
    type RuntimeHoldReason = RuntimeHoldReason;
    /// Rerun benchmarks if you are making changes to runtime configuration.
    type WeightInfo = weights::pallet_balances::WeightInfo<Runtime>;
}

parameter_types! {
    pub const AssetDeposit: Balance = 10 * XCAV;
    pub const AssetAccountDeposit: Balance = deposit(1, 16);
    pub const ApprovalDeposit: Balance = EXISTENTIAL_DEPOSIT;
    pub const StringLimit: u32 = 5000;
    pub const MetadataDepositBase: Balance = deposit(1, 68);
    pub const MetadataDepositPerByte: Balance = deposit(0, 1);
    pub const RemoveItemsLimit: u32 = 1000;
    pub const ZeroDeposit: Balance = 0;
    pub RootAccountId: AccountId = AccountId::from([0xffu8; 32]);
}

impl pallet_assets::Config<pallet_assets::Instance1> for Runtime {
    type ApprovalDeposit = ApprovalDeposit;
    type AssetAccountDeposit = AssetAccountDeposit;
    type AssetDeposit = ZeroDeposit;
    type AssetId = u32;
    type AssetIdParameter = parity_scale_codec::Compact<u32>;
    type Balance = Balance;
    #[cfg(feature = "runtime-benchmarks")]
    type BenchmarkHelper = ();
    type CallbackHandle = ();
    type CreateOrigin = AsEnsureOriginWithArg<EnsureRootWithSuccess<AccountId, RootAccountId>>;
    type Currency = Balances;
    type Extra = ();
    type ForceOrigin = EnsureRoot<AccountId>;
    type Freezer = AssetsFreezer;
    type Holder = ();
    type MetadataDepositBase = ZeroDeposit;
    type MetadataDepositPerByte = ZeroDeposit;
    type RemoveItemsLimit = RemoveItemsLimit;
    type RuntimeEvent = RuntimeEvent;
    type StringLimit = StringLimit;
    /// Rerun benchmarks if you are making changes to runtime configuration.
    type WeightInfo = weights::pallet_assets::WeightInfo<Runtime>;
}

impl pallet_assets::Config<pallet_assets::Instance2> for Runtime {
    type ApprovalDeposit = ApprovalDeposit;
    type AssetAccountDeposit = AssetAccountDeposit;
    type AssetDeposit = AssetDeposit;
    type AssetId = u32;
    type AssetIdParameter = parity_scale_codec::Compact<u32>;
    type Balance = Balance;
    #[cfg(feature = "runtime-benchmarks")]
    type BenchmarkHelper = ();
    type CallbackHandle = ();
    type CreateOrigin = AsEnsureOriginWithArg<EnsureRootWithSuccess<AccountId, RootAccountId>>;
    type Currency = Balances;
    type Extra = ();
    type ForceOrigin = EnsureRoot<AccountId>;
    type Freezer = ();
    type Holder = AssetsHolder;
    type MetadataDepositBase = MetadataDepositBase;
    type MetadataDepositPerByte = MetadataDepositPerByte;
    type RemoveItemsLimit = RemoveItemsLimit;
    type RuntimeEvent = RuntimeEvent;
    type StringLimit = StringLimit;
    /// Rerun benchmarks if you are making changes to runtime configuration.
    type WeightInfo = weights::pallet_assets::WeightInfo<Runtime>;
}

impl pallet_assets::Config<pallet_assets::Instance3> for Runtime {
    type ApprovalDeposit = ApprovalDeposit;
    type AssetAccountDeposit = AssetAccountDeposit;
    type AssetDeposit = AssetDeposit;
    type AssetId = u32;
    type AssetIdParameter = parity_scale_codec::Compact<u32>;
    type Balance = Balance;
    #[cfg(feature = "runtime-benchmarks")]
    type BenchmarkHelper = ();
    type CallbackHandle = ();
    type CreateOrigin = AsEnsureOriginWithArg<EnsureRootWithSuccess<AccountId, RootAccountId>>;
    type Currency = Balances;
    type Extra = ();
    type ForceOrigin = EnsureRoot<AccountId>;
    type Freezer = ();
    type Holder = ();
    type MetadataDepositBase = MetadataDepositBase;
    type MetadataDepositPerByte = MetadataDepositPerByte;
    type RemoveItemsLimit = RemoveItemsLimit;
    type RuntimeEvent = RuntimeEvent;
    type StringLimit = StringLimit;
    /// Rerun benchmarks if you are making changes to runtime configuration.
    type WeightInfo = weights::pallet_assets::WeightInfo<Runtime>;
}

parameter_types! {
    /// Relay Chain `TransactionByteFee` / 10
    pub const TransactionByteFee: Balance = 10 * MICROXCAV;
    pub const OperationalFeeMultiplier: u8 = 5;
}

impl pallet_transaction_payment::Config for Runtime {
    /// There are two possible mechanisms available: slow and fast adjusting.
    /// With slow adjusting fees stay almost constant in short periods of time, changing only in long term.
    /// It may lead to long inclusion times during spikes, therefore tipping is enabled.
    /// With fast adjusting fees change rapidly, but fixed for all users at each block (no tipping)
    type FeeMultiplierUpdate = SlowAdjustingFeeUpdate<Self>;
    type LengthToFee = ConstantMultiplier<Balance, TransactionByteFee>;
    type OnChargeTransaction = pallet_transaction_payment::FungibleAdapter<Balances, ()>;
    type OperationalFeeMultiplier = OperationalFeeMultiplier;
    type RuntimeEvent = RuntimeEvent;
    type WeightInfo = weights::pallet_transaction_payment::WeightInfo<Runtime>;
    type WeightToFee = WeightToFee;
}

impl pallet_sudo::Config for Runtime {
    type RuntimeCall = RuntimeCall;
    type RuntimeEvent = RuntimeEvent;
    /// Rerun benchmarks if you are making changes to runtime configuration.
    type WeightInfo = weights::pallet_sudo::WeightInfo<Runtime>;
}

parameter_types! {
    pub const ReservedXcmpWeight: Weight = MAXIMUM_BLOCK_WEIGHT.saturating_div(4);
    pub const ReservedDmpWeight: Weight = MAXIMUM_BLOCK_WEIGHT.saturating_div(4);
    pub const RelayOrigin: AggregateMessageOrigin = AggregateMessageOrigin::Parent;
}

impl cumulus_pallet_parachain_system::Config for Runtime {
    type CheckAssociatedRelayNumber = RelayNumberMonotonicallyIncreases;
    type ConsensusHook = ConsensusHook;
    type DmpQueue = frame_support::traits::EnqueueWithOrigin<MessageQueue, RelayOrigin>;
    type OnSystemEvent = ();
    type OutboundXcmpMessageSource = XcmpQueue;
    type ReservedDmpWeight = ReservedDmpWeight;
    type ReservedXcmpWeight = ReservedXcmpWeight;
    type RuntimeEvent = RuntimeEvent;
    type SelectCore = cumulus_pallet_parachain_system::DefaultCoreSelector<Runtime>;
    type SelfParaId = parachain_info::Pallet<Runtime>;
    /// Rerun benchmarks if you are making changes to runtime configuration.
    type WeightInfo = weights::cumulus_pallet_parachain_system::WeightInfo<Runtime>;
    type XcmpMessageHandler = XcmpQueue;
}

impl parachain_info::Config for Runtime {}

parameter_types! {
    pub MessageQueueServiceWeight: Weight = Perbill::from_percent(35) * RuntimeBlockWeights::get().max_block;
    pub const HeapSize: u32 = 64 * 1024;
    pub const MaxStale: u32 = 8;
}

impl pallet_message_queue::Config for Runtime {
    type HeapSize = HeapSize;
    type IdleMaxServiceWeight = MessageQueueServiceWeight;
    type MaxStale = MaxStale;
    #[cfg(feature = "runtime-benchmarks")]
    type MessageProcessor = pallet_message_queue::mock_helpers::NoopMessageProcessor<
        cumulus_primitives_core::AggregateMessageOrigin,
    >;
    #[cfg(not(feature = "runtime-benchmarks"))]
    type MessageProcessor = ProcessXcmMessage<
        AggregateMessageOrigin,
        xcm_executor::XcmExecutor<xcm_config::XcmConfig>,
        RuntimeCall,
    >;
    // The XCMP queue pallet is only ever able to handle the `Sibling(ParaId)` origin:
    type QueueChangeHandler = NarrowOriginToSibling<XcmpQueue>;
    type QueuePausedQuery = NarrowOriginToSibling<XcmpQueue>;
    type RuntimeEvent = RuntimeEvent;
    type ServiceWeight = MessageQueueServiceWeight;
    type Size = u32;
    /// Rerun benchmarks if you are making changes to runtime configuration.
    type WeightInfo = weights::pallet_message_queue::WeightInfo<Runtime>;
}

impl cumulus_pallet_aura_ext::Config for Runtime {}

parameter_types! {
    pub const MaxInboundSuspended: u32 = 1000;
    /// The asset ID for the asset that we use to pay for message delivery fees.
    pub FeeAssetId: AssetId = AssetId(RelayLocation::get());
    /// The base fee for the message delivery fees. Kusama is based for the reference.
    pub const ToSiblingBaseDeliveryFee: u128 = XCAV.saturating_mul(3);
}

impl cumulus_pallet_xcmp_queue::Config for Runtime {
    type ChannelInfo = ParachainSystem;
    type ControllerOrigin = EnsureRoot<AccountId>;
    type ControllerOriginConverter = XcmOriginToTransactDispatchOrigin;
    type MaxActiveOutboundChannels = ConstU32<128>;
    type MaxInboundSuspended = MaxInboundSuspended;
    type MaxPageSize = ConstU32<{ 103 * 1024 }>;
    /// Ensure that this value is not set to null (or NoPriceForMessageDelivery) to prevent spamming
    type PriceForSiblingDelivery = PriceForSiblingParachainDelivery;
    type RuntimeEvent = RuntimeEvent;
    type VersionWrapper = ();
    /// Rerun benchmarks if you are making changes to runtime configuration.
    type WeightInfo = weights::cumulus_pallet_xcmp_queue::WeightInfo<Runtime>;
    // Enqueue XCMP messages from siblings for later processing.
    type XcmpQueue = TransformOrigin<MessageQueue, AggregateMessageOrigin, ParaId, ParaIdToSibling>;
}

parameter_types! {
    // One storage item; key size is 32; value is size 4+4+16+32 bytes = 56 bytes.
    pub const DepositBase: Balance = deposit(1, 88);
    // Additional storage item size of 32 bytes.
    pub const DepositFactor: Balance = deposit(0, 32);
    pub const MaxSignatories: u16 = 20;
}

impl pallet_multisig::Config for Runtime {
    type BlockNumberProvider = System;
    type Currency = Balances;
    type DepositBase = DepositBase;
    type DepositFactor = DepositFactor;
    type MaxSignatories = MaxSignatories;
    type RuntimeCall = RuntimeCall;
    type RuntimeEvent = RuntimeEvent;
    /// Rerun benchmarks if you are making changes to runtime configuration.
    type WeightInfo = weights::pallet_multisig::WeightInfo<Runtime>;
}

parameter_types! {
    // pallet_session ends the session after a fixed period of blocks.
    // The first session will have length of Offset,
    // and the following sessions will have length of Period.
    // This may prove nonsensical if Offset >= Period.
    pub const Period: u32 = 6 * HOURS;
    pub const Offset: u32 = 0;
}

impl pallet_session::Config for Runtime {
    type DisablingStrategy = ();
    type Keys = SessionKeys;
    type NextSessionRotation = pallet_session::PeriodicSessions<Period, Offset>;
    type RuntimeEvent = RuntimeEvent;
    // Essentially just Aura, but let's be pedantic.
    type SessionHandler = <SessionKeys as sp_runtime::traits::OpaqueKeys>::KeyTypeIdProviders;
    type SessionManager = CollatorSelection;
    type ShouldEndSession = pallet_session::PeriodicSessions<Period, Offset>;
    type ValidatorId = <Self as frame_system::Config>::AccountId;
    // we don't have stash and controller, thus we don't need the convert as well.
    type ValidatorIdOf = pallet_collator_selection::IdentityCollator;
    /// Rerun benchmarks if you are making changes to runtime configuration.
    type WeightInfo = weights::pallet_session::WeightInfo<Runtime>;
}

parameter_types! {
    pub const AllowMultipleBlocksPerSlot: bool = true;
    pub const MaxAuthorities: u32 = 100_000;
}

impl pallet_aura::Config for Runtime {
    type AllowMultipleBlocksPerSlot = AllowMultipleBlocksPerSlot;
    type AuthorityId = AuraId;
    type DisabledValidators = ();
    type MaxAuthorities = MaxAuthorities;
    type SlotDuration = ConstU64<SLOT_DURATION>;
}

parameter_types! {
    pub const PotId: PalletId = PalletId(*b"PotStake");
    pub const SessionLength: BlockNumber = 6 * HOURS;
    // StakingAdmin pluralistic body.
    pub const StakingAdminBodyId: BodyId = BodyId::Defense;
    pub const MaxCandidates: u32 = 100;
    pub const MaxInvulnerables: u32 = 20;
    pub const MinEligibleCollators: u32 = 1;
}

impl pallet_collator_selection::Config for Runtime {
    type Currency = Balances;
    // should be a multiple of session or things will get inconsistent
    type KickThreshold = Period;
    type MaxCandidates = MaxCandidates;
    type MaxInvulnerables = MaxInvulnerables;
    type MinEligibleCollators = MinEligibleCollators;
    type PotId = PotId;
    type RuntimeEvent = RuntimeEvent;
    type UpdateOrigin = CollatorSelectionUpdateOrigin;
    type ValidatorId = <Self as frame_system::Config>::AccountId;
    type ValidatorIdOf = pallet_collator_selection::IdentityCollator;
    type ValidatorRegistration = Session;
    /// Rerun benchmarks if you are making changes to runtime configuration.
    type WeightInfo = weights::pallet_collator_selection::WeightInfo<Runtime>;
}

impl pallet_utility::Config for Runtime {
    type PalletsOrigin = OriginCaller;
    type RuntimeCall = RuntimeCall;
    type RuntimeEvent = RuntimeEvent;
    /// Rerun benchmarks if you are making changes to runtime configuration.
    type WeightInfo = weights::pallet_utility::WeightInfo<Runtime>;
}

/// Configure the pallet weight reclaim tx.
impl cumulus_pallet_weight_reclaim::Config for Runtime {
    type WeightInfo = weights::cumulus_pallet_weight_reclaim::WeightInfo<Runtime>;
}

parameter_types! {
    pub const MinVestedTransfer: Balance = EXISTENTIAL_DEPOSIT;
    pub UnvestedFundsAllowedWithdrawReasons: WithdrawReasons =
        WithdrawReasons::except(WithdrawReasons::TRANSFER | WithdrawReasons::RESERVE);
}

impl pallet_vesting::Config for Runtime {
    type BlockNumberProvider = System;
    type BlockNumberToBalance = ConvertInto;
    type Currency = Balances;
    type MinVestedTransfer = MinVestedTransfer;
    type RuntimeEvent = RuntimeEvent;
    type UnvestedFundsAllowedWithdrawReasons = UnvestedFundsAllowedWithdrawReasons;
    type WeightInfo = weights::pallet_vesting::WeightInfo<Runtime>;

    const MAX_VESTING_SCHEDULES: u32 = 30;
}

parameter_types! {
    pub Features: PalletFeatures = PalletFeatures::all_enabled();
    pub const MaxAttributesPerCall: u32 = 10;
    pub const CollectionDeposit: Balance = 0;
    pub const ItemDeposit: Balance = 0;
    pub const KeyLimit: u32 = 32;
    pub const ValueLimit: u32 = 256;
    pub const ApprovalsLimit: u32 = 20;
    pub const ItemAttributesApprovalsLimit: u32 = 20;
    pub const MaxTips: u32 = 10;
    pub const MaxDeadlineDuration: BlockNumber = 12 * 30 * DAYS;
}

impl pallet_nfts::Config<pallet_nfts::Instance1> for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type CollectionId = u32;
    type ItemId = u32;
    type Currency = Balances;
    type ForceOrigin = frame_system::EnsureRoot<AccountId>;
    type CollectionDeposit = CollectionDeposit;
    type ItemDeposit = ItemDeposit;
    type MetadataDepositBase = MetadataDepositBase;
    type AttributeDepositBase = MetadataDepositBase;
    type DepositPerByte = MetadataDepositPerByte;
    type StringLimit = StringLimit;
    type KeyLimit = KeyLimit;
    type ValueLimit = ValueLimit;
    type ApprovalsLimit = ApprovalsLimit;
    type ItemAttributesApprovalsLimit = ItemAttributesApprovalsLimit;
    type MaxTips = MaxTips;
    type MaxDeadlineDuration = MaxDeadlineDuration;
    type MaxAttributesPerCall = MaxAttributesPerCall;
    type Features = Features;
    type OffchainSignature = Signature;
    type OffchainPublic = <Signature as Verify>::Signer;
    type WeightInfo = ();
    #[cfg(feature = "runtime-benchmarks")]
    type Helper = ();
    type CreateOrigin = AsEnsureOriginWithArg<EnsureRootWithSuccess<AccountId, RootAccountId>>;
    type Locker = ();
    type BlockNumberProvider = frame_system::Pallet<Runtime>;
}

impl pallet_nfts::Config<pallet_nfts::Instance2> for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type CollectionId = u32;
    type ItemId = u32;
    type Currency = Balances;
    type ForceOrigin = frame_system::EnsureRoot<AccountId>;
    type CollectionDeposit = CollectionDeposit;
    type ItemDeposit = ItemDeposit;
    type MetadataDepositBase = ZeroDeposit;
    type AttributeDepositBase = ZeroDeposit;
    type DepositPerByte = ZeroDeposit;
    type StringLimit = StringLimit;
    type KeyLimit = KeyLimit;
    type ValueLimit = ValueLimit;
    type ApprovalsLimit = ApprovalsLimit;
    type ItemAttributesApprovalsLimit = ItemAttributesApprovalsLimit;
    type MaxTips = MaxTips;
    type MaxDeadlineDuration = MaxDeadlineDuration;
    type MaxAttributesPerCall = MaxAttributesPerCall;
    type Features = Features;
    type OffchainSignature = Signature;
    type OffchainPublic = <Signature as Verify>::Signer;
    type WeightInfo = ();
    #[cfg(feature = "runtime-benchmarks")]
    type Helper = ();
    type CreateOrigin = AsEnsureOriginWithArg<EnsureRootWithSuccess<AccountId, RootAccountId>>;
    type Locker = ();
    type BlockNumberProvider = frame_system::Pallet<Runtime>;
}

parameter_types! {
    pub const NftFractionalizationPalletId: PalletId = PalletId(*b"fraction");
    pub NewAssetSymbol: BoundedVec<u8, StringLimit> = (*b"BRIX").to_vec().try_into().unwrap();
    pub NewAssetName: BoundedVec<u8, StringLimit> = (*b"Brix").to_vec().try_into().unwrap();
    pub const Deposit: Balance = XCAV;
}

impl pallet_nft_fractionalization::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type Deposit = Deposit;
    type Currency = Balances;
    type NewAssetSymbol = NewAssetSymbol;
    type NewAssetName = NewAssetName;
    type NftCollectionId = <Self as pallet_nfts::Config<Instance1>>::CollectionId;
    type NftId = <Self as pallet_nfts::Config<Instance1>>::ItemId;
    type AssetBalance = <Self as pallet_balances::Config>::Balance;
    type AssetId = <Self as pallet_assets::Config<Instance1>>::AssetId;
    type Assets = RealEstateAssets;
    type Nfts = RealEstateNfts;
    type PalletId = NftFractionalizationPalletId;
    type WeightInfo = ();
    type StringLimit = StringLimit;
    #[cfg(feature = "runtime-benchmarks")]
    type BenchmarkHelper = ();
    type RuntimeHoldReason = RuntimeHoldReason;
}

impl pallet_assets_holder::Config<pallet_assets::Instance2> for Runtime {
    type RuntimeHoldReason = MarketplaceHoldReason;
    type RuntimeEvent = RuntimeEvent;
}

impl pallet_assets_freezer::Config<pallet_assets::Instance1> for Runtime {
    type RuntimeFreezeReason = MarketplaceFreezeReason;
    type RuntimeEvent = RuntimeEvent;
}

parameter_types! {
    pub const WhitelistAirdropNativeAmount: Balance = 10 * XCAV; // 10 XCAV
    pub const WhitelistAirdropAssetId: u32 = 10; // tGBP
    pub const WhitelistAirdropAssetAmount: Balance = 10_000_000_000_000_000_000_000; // 10,000 tGBP (18 decimals)
}

#[cfg(feature = "runtime-benchmarks")]
pub struct WhitelistBenchmarkHelper;
#[cfg(feature = "runtime-benchmarks")]
impl pallet_xcavate_whitelist::BenchmarkHelper<Runtime> for WhitelistBenchmarkHelper {
    fn setup_airdrop_asset() {
        use frame_support::traits::fungibles::Create;
        let admin = AccountId::from([0xffu8; 32]);
        let _ = <Assets as Create<AccountId>>::create(WhitelistAirdropAssetId::get(), admin, true, 1);
    }
}

/// Configure the pallet-xcavate-whitelist in pallets/xcavate-whitelist.
impl pallet_xcavate_whitelist::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type WeightInfo = weights::pallet_xcavate_whitelist::WeightInfo<Runtime>;
    type WhitelistOrigin = EnsureRoot<Self::AccountId>;
    type Balance = Balance;
    type NativeCurrency = Balances;
    type ForeignCurrency = Assets;
    type AirdropNativeAmount = WhitelistAirdropNativeAmount;
    type AirdropAssetId = WhitelistAirdropAssetId;
    type AirdropAssetAmount = WhitelistAirdropAssetAmount;
    #[cfg(feature = "runtime-benchmarks")]
    type BenchmarkHelper = WhitelistBenchmarkHelper;
}

use pallet_xcavate_whitelist::{self as whitelist, RolePermission};

pub struct EnsureHasRole<T>(core::marker::PhantomData<T>);

impl<T: whitelist::Config> EnsureOriginWithArg<T::RuntimeOrigin, whitelist::Role>
    for EnsureHasRole<T>
{
    type Success = T::AccountId;

    fn try_origin(
        origin: T::RuntimeOrigin,
        role: &whitelist::Role,
    ) -> Result<Self::Success, T::RuntimeOrigin> {
        let Some(who) = origin.clone().into_signer() else {
            return Err(origin);
        };
        if whitelist::Pallet::<T>::has_role(&who, role.clone()) {
            Ok(who)
        } else {
            Err(origin)
        }
    }

    #[cfg(feature = "runtime-benchmarks")]
    fn try_successful_origin(_role: &whitelist::Role) -> Result<T::RuntimeOrigin, ()> {
        let account = frame_benchmarking::whitelisted_caller();
        Ok(frame_system::RawOrigin::Signed(account).into())
    }
}

pub struct EnsureCompliant<T>(core::marker::PhantomData<T>);

impl<T: whitelist::Config> EnsureOriginWithArg<T::RuntimeOrigin, whitelist::Role>
    for EnsureCompliant<T>
{
    type Success = T::AccountId;

    fn try_origin(
        origin: T::RuntimeOrigin,
        role: &whitelist::Role,
    ) -> Result<Self::Success, T::RuntimeOrigin> {
        let Some(who) = origin.clone().into_signer() else {
            return Err(origin);
        };
        if whitelist::Pallet::<T>::is_compliant(&who, role.clone()) {
            Ok(who)
        } else {
            Err(origin)
        }
    }

    #[cfg(feature = "runtime-benchmarks")]
    fn try_successful_origin(_role: &whitelist::Role) -> Result<T::RuntimeOrigin, ()> {
        let account = frame_benchmarking::whitelisted_caller();
        Ok(frame_system::RawOrigin::Signed(account).into())
    }
}

parameter_types! {
    pub const Postcode: u32 = 10;
    pub const LocationDepositAmount: Balance = 10_000 * XCAV;
    pub const MaximumListingDuration: BlockNumber = 30 * DAYS;
    pub const RegionVotingTime: BlockNumber = 30;
    pub const RegionAuctionTime: BlockNumber = 30;
    pub const RegionOperatorVotingTime: BlockNumber = 20;
    pub const RegionThreshold: Percent = Percent::from_percent(75);
    pub const MaxProposalForBlock: u32 = 100;
    pub const RegionSlashingAmount: Balance = 10 * XCAV;
    pub const RegionOwnerChangeTime: BlockNumber = 400;
    pub const RegionOwnerNoticeTime: BlockNumber = 50;
    pub const RegionOwnerDisputeDepositAmount: Balance = 1_000 * XCAV;
    pub const MinimumRegionDepositAmount: Balance = 100_000 * XCAV;
    pub const RegionProposalDepositAmount: Balance = 5_000 * XCAV;
    pub const MinimumVotingPower: Balance = 100 * XCAV;
    pub const LawyerDepositAmount: Balance = 10_000 * XCAV;
    pub const MaximumTaxPercent: Permill = Permill::from_percent(10);
    pub const MaxAllowedStrikes: u8 = 3;
    pub const RegionVotingQuorum: Permill = Permill::from_percent(1);
}

/// Configure the pallet-property-governance in pallets/property-governance.
impl pallet_regions::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type WeightInfo = weights::pallet_regions::WeightInfo<Runtime>;
    type Balance = Balance;
    type NativeCurrency = Balances;
    type RuntimeHoldReason = RuntimeHoldReason;
    type Nfts = RealEstateNfts;
    type NftCollectionId = <Self as pallet_nfts::Config<Instance1>>::CollectionId;
    type NftId = <Self as pallet_nfts::Config<Instance1>>::ItemId;
    type MarketplacePalletId = MarketplacePalletId;
    type MaxListingDuration = MaximumListingDuration;
    type PostcodeLimit = Postcode;
    type LocationDeposit = LocationDepositAmount;
    type RegionVotingTime = RegionVotingTime;
    type RegionAuctionTime = RegionAuctionTime;
    type RegionThreshold = RegionThreshold;
    type RegionOperatorVotingTime = RegionOperatorVotingTime;
    type MaxProposalsForBlock = MaxProposalForBlock;
    type RegionSlashingAmount = RegionSlashingAmount;
    type TreasuryId = TreasuryPalletId;
    type RegionOwnerChangePeriod = RegionOwnerChangeTime;
    type Slash = ();
    type RegionOwnerNoticePeriod = RegionOwnerNoticeTime;
    type RegionOwnerDisputeDeposit = RegionOwnerDisputeDepositAmount;
    type MinimumRegionDeposit = MinimumRegionDepositAmount;
    type RegionProposalDeposit = RegionProposalDepositAmount;
    type MinimumVotingAmount = MinimumVotingPower;
    type PermissionOrigin = EnsureHasRole<Self>;
    type LawyerDeposit = LawyerDepositAmount;
    type BlockNumberProvider = System;
    type MaxTaxPercent = MaximumTaxPercent;
    type AllowedStrikes = MaxAllowedStrikes;
    type MinVotingQuorum = RegionVotingQuorum;
}

/// Configure the pallet-property-governance in pallets/property-governance.
impl pallet_education_regions::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type WeightInfo = weights::pallet_education_regions::WeightInfo<Runtime>;
    type Balance = Balance;
    type NativeCurrency = Balances;
    type RuntimeHoldReason = RuntimeHoldReason;
    type RegionVotingTime = RegionVotingTime;
    type RegionAuctionTime = RegionAuctionTime;
    type RegionThreshold = RegionThreshold;
    type RegionOperatorVotingTime = RegionOperatorVotingTime;
    type MaxProposalsForBlock = MaxProposalForBlock;
    type RegionSlashingAmount = RegionSlashingAmount;
    type TreasuryId = TreasuryPalletId;
    type RegionOwnerChangePeriod = RegionOwnerChangeTime;
    type Slash = ();
    type RegionOwnerNoticePeriod = RegionOwnerNoticeTime;
    type RegionOwnerDisputeDeposit = RegionOwnerDisputeDepositAmount;
    type MinimumRegionDeposit = MinimumRegionDepositAmount;
    type RegionProposalDeposit = RegionProposalDepositAmount;
    type MinimumVotingAmount = MinimumVotingPower;
    type PermissionOrigin = EnsureHasRole<Self>;
    type BlockNumberProvider = System;
    type AllowedStrikes = MaxAllowedStrikes;
    type MinVotingQuorum = RegionVotingQuorum;
}

parameter_types! {
    pub const XEducationPalletId: PalletId = PalletId(*b"py/xeduc");
    pub const MaximumModuleToken: u32 = 1000;
    pub const ModulePriceLimit: Balance = 100;
    pub const ContentCreatorPercentage: Perbill = Perbill::from_parts(83_000_000);
    pub const RegionalOperatorPercentage: Perbill = Perbill::from_parts(83_000_000);
    pub const ProtocolPercentage: Perbill = Perbill::from_parts(50_000_000);
    pub const DBSPercentage: Perbill = Perbill::from_parts(34_000_000);
    pub const BookingDepositAmount: Balance = 10 * XCAV;
    pub const ModuleDepositAmount: Balance = 100 * XCAV;
    pub const MaxCancellationAmount: u32 = 5;
    pub const CancellationWindow: BlockNumber = 100;
    pub const SponsorshipWindow: BlockNumber = 200;
    pub const ModuleDelivererDepositAmount: Balance = 500 * XCAV;
    pub const MaxAllowedStrikesAmount: u8 = 3;
    pub const StrikeSlashPercentage: Perbill = Perbill::from_parts(100_000_000);
    pub const MaxCleanupPerCallAmount: u32 = 50;
    pub const MinimumImpactScore: Permill = Permill::from_percent(50);
    pub const SuccessfulDeliveriesForStrikeReduction: u32 = 5;
    pub const AcceptedPaymentAssets: [u32; 3] = [10, 1337, 1984];
}

/// Configure the pallet-real-x-education in pallets/real-x-education.
impl pallet_real_x_education::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type WeightInfo = weights::pallet_real_x_education::WeightInfo<Runtime>;
    type Balance = Balance;
    type NativeCurrency = Balances;
    type RuntimeHoldReason = RuntimeHoldReason;
    type Nfts = EducationNfts;
    type NftCollectionId = <Self as pallet_nfts::Config<Instance2>>::CollectionId;
    type NftId = <Self as pallet_nfts::Config<Instance2>>::ItemId;
    type MaxModuleToken = MaximumModuleToken;
    type LocalCurrency = EducationAssets;
    type ForeignCurrency = Assets;
    type ForeignAssetsHolder = AssetsHolder;
    type StringLimit = StringLimit;
    type ModulePrice = ModulePriceLimit;
    type BlockNumberProvider = System;
    type ContentCreatorPercentage = ContentCreatorPercentage;
    type RegionalOperatorPercentage = RegionalOperatorPercentage;
    type ProtocolPercentage = ProtocolPercentage;
    type DBSPercentage = DBSPercentage;
    type PalletId = XEducationPalletId;
    type TreasuryId = TreasuryPalletId;
    type PermissionOrigin = EnsureHasRole<Self>;
    type AcceptedAssets = AcceptedPaymentAssets;
    type BookingDeposit = BookingDepositAmount;
    type ModuleDeposit = ModuleDepositAmount;
    type RegionProvider = EducationRegions;
    type NewAssetSymbol = NewAssetSymbol;
    type NewAssetName = NewAssetName;
    type Slash = ();
    type MaxCancellations = MaxCancellationAmount;
    type CancellationWindow = CancellationWindow;
    type SponsorshipWindow = SponsorshipWindow;
    type ModuleDelivererDeposit = ModuleDelivererDepositAmount;
    type MaxAllowedStrikes = MaxAllowedStrikesAmount;
    type StrikeSlashPercentage = StrikeSlashPercentage;
    type MaxCleanupPerCall = MaxCleanupPerCallAmount;
    type MinImpactScore = MinimumImpactScore;
    type SuccessfulDeliveriesForStrikeReduction = SuccessfulDeliveriesForStrikeReduction;
    type RoleProvider = XcavateWhitelist;
    type AssetMetadata = AssetsMetadataWrapper;
}

pub struct AssetsMetadataWrapper;

impl AssetMetadataProvider for AssetsMetadataWrapper {
    type AssetId = u32;

    fn get_decimals(asset_id: Self::AssetId) -> Option<u8> {
        Some(pallet_assets::Metadata::<Runtime, Instance2>::get(asset_id).decimals)
    }
}

parameter_types! {
    pub const MarketplacePalletId: PalletId = PalletId(*b"py/nftxc");
    pub const TreasuryPalletId: PalletId = PalletId(*b"py/trsry");
    pub const PropertyFundingAmount: Balance = 10 * XCAV;
    pub const MaxPropertyShares: u32 = 250;
}

pub struct BucketNamespaceManager;

impl pallet_real_world_asset::NamespaceManager<AccountId> for BucketNamespaceManager {
    fn create_namespace_for_property(
        manager: &AccountId,
        real_world_asset_id: u32,
    ) -> Result<u128, frame_support::pallet_prelude::DispatchError> {
        let mut properties =
            frame_support::storage::bounded_btree_map::BoundedBTreeMap::default();
        properties
            .try_insert(
                BoundedVec::truncate_from(b"propertyId".to_vec()),
                BoundedVec::truncate_from(real_world_asset_id.to_le_bytes().to_vec()),
            )
            .map_err(|_| {
                frame_support::pallet_prelude::DispatchError::Other(
                    "Namespace metadata properties full",
                )
            })?;

        let namespace_id = pallet_bucket::NextNamespaceId::<Runtime>::get();
        let metadata_input = pallet_bucket::types::NamespaceMetadataInput::<Runtime> {
            name: BoundedVec::truncate_from(b"Property namespace".to_vec()),
            schema_uri: None,
            properties,
        };

        <pallet_bucket::Pallet<Runtime> as pallet_bucket::traits::Create<Runtime>>::namespace(
            metadata_input.into(),
            Some(manager.clone()),
        )?;

        Ok(namespace_id)
    }
}

/// Configure the pallet-property-governance in pallets/property-governance.
impl pallet_real_world_asset::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type Balance = Balance;
    type NativeCurrency = Balances;
    type NftCollectionId = <Self as pallet_nfts::Config<Instance1>>::CollectionId;
    type NftId = <Self as pallet_nfts::Config<Instance1>>::ItemId;
    type Nfts = RealEstateNfts;
    type MarketplacePalletId = MarketplacePalletId;
    type LocalCurrency = RealEstateAssets;
    type FractionalizeCollectionId = <Self as pallet_nfts::Config<Instance1>>::CollectionId;
    type FractionalizeItemId = <Self as pallet_nfts::Config<Instance1>>::ItemId;
    type AssetId = <Self as pallet_assets::Config<Instance1>>::AssetId;
    type PropertyAccountFundingAmount = PropertyFundingAmount;
    type MaxPropertyShares = MaxPropertyShares;
    type StringLimit = StringLimit;
    type RegionProvider = Regions;
    type PostcodeLimit = Postcode;
    type NamespaceManager = BucketNamespaceManager;
}

parameter_types! {
    pub const MinPropertyShares: u32 = 100;
    pub const ListingDepositAmount: Balance = 10 * MICROXCAV;
    pub const MarketplaceFeePercent: Perbill = Perbill::from_percent(1);
    pub const MaximumAcceptedAssets: u32 = 2;
    pub const LawyerVotingDuration: BlockNumber = 30;
    pub const LegalProcessDuration: BlockNumber = 80;
    pub const MinimumVotingQuorum: Percent = Percent::from_percent(50);
    pub const ClaimWindowTime: BlockNumber = 100;
    pub const MaximumRelistAttempts: u8 = 1;
    pub const MaxOwnershipPercentage: Perbill = Perbill::from_percent(50);
    pub const AcceptedMarketplacePaymentAssets: [u32; 2] = [10, 1];
}

/// Configure the pallet-marketplace in pallets/marketplace.
impl pallet_marketplace::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type WeightInfo = weights::pallet_marketplace::WeightInfo<Runtime>;
    type Balance = Balance;
    type NativeCurrency = Balances;
    type RuntimeHoldReason = RuntimeHoldReason;
    type LocalCurrency = RealEstateAssets;
    type ForeignCurrency = Assets;
    type ForeignAssetsHolder = AssetsHolder;
    type AssetsFreezer = AssetsFreezer;
    type NftCollectionId = <Self as pallet_nfts::Config<Instance1>>::CollectionId;
    type NftId = <Self as pallet_nfts::Config<Instance1>>::ItemId;
    type PalletId = MarketplacePalletId;
    type MinPropertyShares = MinPropertyShares;
    type MaxPropertyShares = MaxPropertyShares;
    type TreasuryId = TreasuryPalletId;
    type AssetId = <Self as pallet_assets::Config<Instance1>>::AssetId;
    type ListingDeposit = ListingDepositAmount;
    type MarketplaceFeePercentage = MarketplaceFeePercent;
    type AcceptedAssets = AcceptedMarketplacePaymentAssets;
    type MaxAcceptedAssets = MaximumAcceptedAssets;
    type PropertyShares = RealWorldAsset;
    type LawyerVotingTime = LawyerVotingDuration;
    type LegalProcessTime = LegalProcessDuration;
    type Whitelist = XcavateWhitelist;
    type PermissionOrigin = EnsureHasRole<Self>;
    type CompliantOrigin = EnsureCompliant<Self>;
    type MinVotingQuorum = MinimumVotingQuorum;
    type ClaimWindow = ClaimWindowTime;
    type MaxRelistAttempts = MaximumRelistAttempts;
    type BlockNumberProvider = System;
    type IncomeSettlement = PropertyManagement;
    type RegionProvider = Regions;
    type StringLimit = StringLimit;
    type PostcodeLimit = Postcode;
    type MaxOwnershipPercentage = MaxOwnershipPercentage;
}

parameter_types! {
    pub const MinimumStakingAmount: Balance = 1000 * XCAV;
    pub const MaxProperty: u32 = 1000;
    pub const MaxLocation: u32 = 50;
    pub const LettingAgentVotingDuration: BlockNumber = 20;
    pub const LettingAgentNoticeTime: BlockNumber = 30;
    pub const MaximumNoticesPerBlock: u32 = 10;
}

/// Configure the pallet-property-management in pallets/property-management.
impl pallet_property_management::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type WeightInfo = weights::pallet_property_management::WeightInfo<Runtime>;
    type Balance = Balance;
    type RuntimeHoldReason = RuntimeHoldReason;
    type NativeCurrency = Balances;
    type ForeignCurrency = Assets;
    type AssetsFreezer = AssetsFreezer;
    type NftCollectionId = <Self as pallet_nfts::Config<Instance1>>::CollectionId;
    type NftId = <Self as pallet_nfts::Config<Instance1>>::ItemId;
    type MarketplacePalletId = MarketplacePalletId;
    type LettingAgentDeposit = MinimumStakingAmount;
    type MaxProperties = MaxProperty;
    type MaxLocations = MaxLocation;
    type AcceptedAssets = AcceptedMarketplacePaymentAssets;
    type PropertyShares = RealWorldAsset;
    type LettingAgentVotingTime = LettingAgentVotingDuration;
    type PermissionOrigin = EnsureHasRole<Self>;
    type MinVotingQuorum = MinimumVotingQuorum;
    type LettingAgentNoticePeriod = LettingAgentNoticeTime;
    type MaxNoticesPerBlock = MaximumNoticesPerBlock;
    type BlockNumberProvider = System;
    type RegionProvider = Regions;
    type PostcodeLimit = Postcode;
}

parameter_types! {
    pub const PropertyVotingTime: BlockNumber = 20;
    pub const MaxVoteForBlock: u32 = 100;
    pub const MinimumSlashingAmount: Balance = 10 * XCAV;
    pub const VotingThreshold: Percent = Percent::from_percent(51);
    pub const HighVotingThreshold: Percent = Percent::from_percent(67);
    pub const LowProposal: Balance = 500 * XCAV;
    pub const HighProposal: Balance = 10_000 * XCAV;
    pub const ChallengeDepositAmount: Balance = 500 * XCAV;
    pub const AutoExecutionCooldown: BlockNumber = 28;
}

/// Configure the pallet-property-governance in pallets/property-governance.
impl pallet_property_governance::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type WeightInfo = weights::pallet_property_governance::WeightInfo<Runtime>;
    type Balance = Balance;
    type NativeCurrency = Balances;
    type AssetsFreezer = AssetsFreezer;
    type NftCollectionId = <Self as pallet_nfts::Config<Instance1>>::CollectionId;
    type NftId = <Self as pallet_nfts::Config<Instance1>>::ItemId;
    type VotingTime = PropertyVotingTime;
    type MaxVotesForBlock = MaxVoteForBlock;
    type MinSlashingAmount = MinimumSlashingAmount;
    type HighThreshold = HighVotingThreshold;
    type LowProposal = LowProposal;
    type HighProposal = HighProposal;
    type MarketplacePalletId = MarketplacePalletId;
    type Slash = ();
    type PropertyShares = RealWorldAsset;
    type PermissionOrigin = EnsureHasRole<Self>;
    type MinVotingQuorum = MinimumVotingQuorum;
    type BlockNumberProvider = System;
    type ChallengeDeposit = ChallengeDepositAmount;
    type StringLimit = StringLimit;
    type PostcodeLimit = Postcode;
    type AutoExecutionCooldown = AutoExecutionCooldown;
}

use pallet_bucket::{traits::CallSources, AccountIdOf};

use pallet_bucket::types::{
    BucketMetadata, BucketMetadataInput, BucketPublicKey, MessageMetadata, MessageMetadataInput,
    NamespaceMetadata, NamespaceMetadataInput,
};

pub struct SuccessOrigin {
    sender: AccountIdOf<Runtime>,
}
impl SuccessOrigin {
    fn new(sender: AccountIdOf<Runtime>) -> Self {
        SuccessOrigin { sender }
    }
}

impl CallSources<AccountIdOf<Runtime>, AccountIdOf<Runtime>> for SuccessOrigin {
    fn sender(&self) -> AccountIdOf<Runtime> {
        self.sender.clone()
    }

    fn subject(&self) -> AccountIdOf<Runtime> {
        self.sender.clone()
    }
}
impl Morph<AccountIdOf<Runtime>> for SuccessOrigin {
    type Outcome = Self;

    fn morph(a: AccountIdOf<Runtime>) -> Self::Outcome {
        Self::new(a)
    }
}

type BucketsOrigin = MapSuccess<EnsureSigned<AccountIdOf<Runtime>>, SuccessOrigin>;

parameter_types! {
    pub const NamespaceStorageFee: Balance = deposit(1, 2856);
    pub const BucketStorageFee: Balance = deposit(1, 2650);
    pub const MessageStorageFee: Balance = deposit(1, 2682);
    pub const StorageFee: Balance = MILLIXCAV;
    pub const MaxStringLength: u32 = 200;
    #[derive(PartialEq, Eq, Clone, sp_core::RuntimeDebug)]
    pub const MaxNameLen: u32 = 100;
    #[derive(PartialEq, Eq, Clone, sp_core::RuntimeDebug)]
    pub const MaxUriLen: u32 = 256;
    #[derive(PartialEq, Eq, Clone, sp_core::RuntimeDebug)]
    pub const MaxCategoryLen: u32 = 50;
    #[derive(PartialEq, Eq, Clone, sp_core::RuntimeDebug)]
    pub const MaxProperties: u32 = 10;
    #[derive(PartialEq, Eq, Clone, sp_core::RuntimeDebug)]
    pub const MaxPropertyKeyLen: u32 = 50;
    #[derive(PartialEq, Eq, Clone, sp_core::RuntimeDebug)]
    pub const MaxPropertyValueLen: u32 = 200;
}

#[cfg(feature = "runtime-benchmarks")]
pub struct BucketHelper;

#[cfg(feature = "runtime-benchmarks")]
impl pallet_bucket::benchmarking::BenchmarkHelper<Runtime> for BucketHelper {
    fn create_force_origin(_seed: u32) -> RuntimeOrigin {
        frame_system::RawOrigin::Root.into()
    }

    fn create_origin(seed: u32) -> RuntimeOrigin {
        use frame_support::traits::fungible::Mutate;

        let mut array = [0u8; 32];
        let bytes = seed.to_le_bytes();
        array[..4].copy_from_slice(&bytes);

        let account = AccountId::from(array);
        Balances::set_balance(&account, 1_000_000_000_000_000_000_000_000);
        frame_system::RawOrigin::Signed(account).into()
    }

    fn get_bucket(
        _seed: u32,
    ) -> (
        <Runtime as pallet_bucket::Config>::BucketId,
        <Runtime as pallet_bucket::Config>::BucketMetadataInput,
        <Runtime as pallet_bucket::Config>::BucketMetadata,
    ) {
        use alloc::{collections::BTreeMap, vec};
        use frame_support::storage::bounded_btree_map::BoundedBTreeMap;
        let properties = BoundedBTreeMap::<
            BoundedVec<u8, MaxPropertyKeyLen>,
            BoundedVec<u8, MaxPropertyValueLen>,
            MaxProperties,
        >::try_from(
            (0..MaxProperties::get() as usize)
                .map(|i| {
                    (
                        BoundedVec::truncate_from(vec![
                            b'a' + (i as u8 % 26);
                            MaxPropertyKeyLen::get() as usize
                        ]),
                        BoundedVec::truncate_from(vec![
                            b'a' + (i as u8 % 26);
                            MaxPropertyValueLen::get() as usize
                        ]),
                    )
                })
                .collect::<BTreeMap<_, _>>(),
        )
        .expect("Failed to create max-length properties");
        let input = BucketMetadataInput {
            name: BoundedVec::truncate_from(vec![1; MaxNameLen::get() as usize]),
            category: BoundedVec::truncate_from(vec![1; MaxCategoryLen::get() as usize]),
            properties: properties.clone(),
        };
        (
            u128::MAX - 1,
            input.clone(),
            BucketMetadata {
                name: BoundedVec::truncate_from(vec![1; MaxNameLen::get() as usize]),
                created_at: 1,
                category: BoundedVec::truncate_from(vec![1; MaxCategoryLen::get() as usize]),
                properties,
            },
        )
    }

    fn get_key_id(seed: u32) -> <Runtime as pallet_bucket::Config>::KeyId {
        let account_id = frame_benchmarking::account::<AccountId>("key_id", seed, 0);
        BucketPublicKey(account_id.into())
    }

    fn get_message(
        _seed: u32,
    ) -> (
        <Runtime as pallet_bucket::Config>::Reference,
        pallet_bucket::MessageMetadataInputOf<Runtime>,
        <Runtime as pallet_bucket::Config>::MessageMetadata,
    ) {
        use alloc::{collections::BTreeMap, vec};
        use frame_support::storage::bounded_btree_map::BoundedBTreeMap;
        let properties = BoundedBTreeMap::<
            BoundedVec<u8, MaxPropertyKeyLen>,
            BoundedVec<u8, MaxPropertyValueLen>,
            MaxProperties,
        >::try_from(
            (0..MaxProperties::get() as usize)
                .map(|i| {
                    (
                        BoundedVec::truncate_from(vec![
                            b'a' + (i as u8 % 26);
                            MaxPropertyKeyLen::get() as usize
                        ]),
                        BoundedVec::truncate_from(vec![
                            b'a' + (i as u8 % 26);
                            MaxPropertyValueLen::get() as usize
                        ]),
                    )
                })
                .collect::<BTreeMap<_, _>>(),
        )
        .expect("Failed to create max-length properties");
        let reference =
            frame_support::BoundedVec::<u8, MaxStringLength>::try_from(vec![0; 200]).unwrap();
        let input = MessageMetadataInput {
            description: BoundedVec::truncate_from(vec![1; MaxNameLen::get() as usize]),
            content_type: BoundedVec::truncate_from(vec![1; MaxCategoryLen::get() as usize]),
            content_hash: Default::default(),
            properties: properties.clone(),
        };
        (
            reference,
            input,
            MessageMetadata {
                description: BoundedVec::truncate_from(vec![1; MaxNameLen::get() as usize]),
                created_at: 1,
                content_type: BoundedVec::truncate_from(vec![1; MaxCategoryLen::get() as usize]),
                content_hash: Default::default(),
                properties,
            },
        )
    }

    fn get_namespace(
        _seed: u32,
    ) -> (
        <Runtime as pallet_bucket::Config>::NamespaceId,
        <Runtime as pallet_bucket::Config>::NamespaceMetadataInput,
        <Runtime as pallet_bucket::Config>::NamespaceMetadata,
    ) {
        use alloc::{collections::BTreeMap, vec};
        use frame_support::storage::bounded_btree_map::BoundedBTreeMap;
        let properties = BoundedBTreeMap::<
            BoundedVec<u8, MaxPropertyKeyLen>,
            BoundedVec<u8, MaxPropertyValueLen>,
            MaxProperties,
        >::try_from(
            (0..MaxProperties::get() as usize)
                .map(|i| {
                    (
                        BoundedVec::truncate_from(vec![
                            b'a' + (i as u8 % 26);
                            MaxPropertyKeyLen::get() as usize
                        ]),
                        BoundedVec::truncate_from(vec![
                            b'a' + (i as u8 % 26);
                            MaxPropertyValueLen::get() as usize
                        ]),
                    )
                })
                .collect::<BTreeMap<_, _>>(),
        )
        .expect("Failed to create max-length properties");
        let input = NamespaceMetadataInput {
            name: BoundedVec::truncate_from(vec![1; MaxNameLen::get() as usize]),
            schema_uri: Some(BoundedVec::truncate_from(vec![1; MaxUriLen::get() as usize])),
            properties: properties.clone(),
        };
        (
            0,
            input.clone(),
            NamespaceMetadata {
                name: BoundedVec::truncate_from(vec![1; MaxNameLen::get() as usize]),
                created_at: 1,
                schema_uri: Some(BoundedVec::truncate_from(vec![1; MaxUriLen::get() as usize])),
                properties,
            },
        )
    }
}

impl pallet_bucket::Config for Runtime {
    #[cfg(feature = "runtime-benchmarks")]
    type BenchmarkHelper = BucketHelper;
    type BucketId = u128;
    type Currency = Balances;
    type FeeBucket = BucketStorageFee;
    type FeeCollector = ();
    type FeeMessage = MessageStorageFee;
    type FeeNamespace = NamespaceStorageFee;
    type FeeTag = StorageFee;
    type ForceOriginCheck = EnsureRoot<AccountId>;
    type KeyId = BucketPublicKey;
    type MaxStringInputLengthTag = MaxStringLength;
    type MessageId = u128;
    type NamespaceId = u128;
    type NamespaceMetadataInput = NamespaceMetadataInput<Self>;
    type BucketMetadataInput = BucketMetadataInput<Self>;
    type MessageMetadataInput = MessageMetadataInput<Self>;
    type NamespaceMetadata = NamespaceMetadata<Self>;
    type BucketMetadata = BucketMetadata<Self>;
    type MessageMetadata = MessageMetadata<Self>;
    type OnCallHooks = ();
    type OriginCheck = BucketsOrigin;
    type OriginSuccess = SuccessOrigin;
    type Reference = BoundedVec<u8, MaxStringLength>;
    type RuntimeEvent = RuntimeEvent;
    type SubjectId = AccountIdOf<Runtime>;
    type WeightInfo = weights::pallet_bucket::WeightInfo<Runtime>;
    type MaxNameLen = MaxNameLen;
    type MaxUriLen = MaxUriLen;
    type MaxCategoryLen = MaxCategoryLen;
    type MaxProperties = MaxProperties;
    type MaxPropertyKeyLen = MaxPropertyKeyLen;
    type MaxPropertyValueLen = MaxPropertyValueLen;
}

parameter_types! {
    pub const FaucetDripAssetId: u32 = 10;
    pub const FaucetDripAmount: Balance = 1_000_000_000_000_000_000_000; // 1000 tGBP (18 decimals)
    pub const FaucetMinXcavBalance: Balance = XCAV;
    pub const FaucetCooldownPeriod: BlockNumber = 7 * DAYS;
}

impl pallet_faucet::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type WeightInfo = weights::pallet_faucet::WeightInfo<Runtime>;
    type Balance = Balance;
    type NativeCurrency = Balances;
    type ForeignCurrency = Assets;
    type DripAssetId = FaucetDripAssetId;
    type DripAmount = FaucetDripAmount;
    type MinXcavBalance = FaucetMinXcavBalance;
    type CooldownPeriod = FaucetCooldownPeriod;
    type BlockNumberProvider = System;
}

use crate::{
    assets::AssetDid,
    authorization::{AuthorizationId, PalletAuthorize},
};
use delegation::DelegationAc;
use did::{
    DeriveDidCallAuthorizationVerificationKeyRelationship, DeriveDidCallKeyRelationshipResult,
    DidRawOrigin, DidVerificationKeyRelationship, EnsureDidOrigin, RelationshipDeriveError,
};

/// A Kilt DID subject identifier.
pub type DidIdentifier = AccountId;

impl DeriveDidCallAuthorizationVerificationKeyRelationship for RuntimeCall {
    fn derive_verification_key_relationship(&self) -> DeriveDidCallKeyRelationshipResult {
        /// ensure that all calls have the same VerificationKeyRelationship
        fn single_key_relationship(calls: &[RuntimeCall]) -> DeriveDidCallKeyRelationshipResult {
            let init = calls
                .first()
                .ok_or(RelationshipDeriveError::InvalidCallParameter)?
                .derive_verification_key_relationship()?;
            calls.iter().skip(1).map(RuntimeCall::derive_verification_key_relationship).try_fold(
                init,
                |acc, next| {
                    if next.is_err() {
                        next
                    } else if Ok(acc) == next {
                        Ok(acc)
                    } else {
                        Err(RelationshipDeriveError::InvalidCallParameter)
                    }
                },
            )
        }
        match self {
            RuntimeCall::Attestation { .. } => Ok(DidVerificationKeyRelationship::AssertionMethod),
            RuntimeCall::Ctype { .. } => Ok(DidVerificationKeyRelationship::AssertionMethod),
            RuntimeCall::Delegation { .. } => {
                Ok(DidVerificationKeyRelationship::CapabilityDelegation)
            }
            // DID creation is not allowed through the DID proxy.
            RuntimeCall::Did(did::Call::create { .. }) => {
                Err(RelationshipDeriveError::NotCallableByDid)
            }
            RuntimeCall::Did { .. } => Ok(DidVerificationKeyRelationship::Authentication),
            RuntimeCall::PublicCredentials { .. } => {
                Ok(DidVerificationKeyRelationship::AssertionMethod)
            }
            RuntimeCall::Utility(pallet_utility::Call::batch { calls }) => {
                single_key_relationship(&calls[..])
            }
            RuntimeCall::Utility(pallet_utility::Call::batch_all { calls }) => {
                single_key_relationship(&calls[..])
            }
            RuntimeCall::Utility(pallet_utility::Call::force_batch { calls }) => {
                single_key_relationship(&calls[..])
            }
            #[cfg(not(feature = "runtime-benchmarks"))]
            _ => Err(RelationshipDeriveError::NotCallableByDid),
            // By default, returns the authentication key
            #[cfg(feature = "runtime-benchmarks")]
            _ => Ok(DidVerificationKeyRelationship::Authentication),
        }
    }

    // Always return a System::remark() extrinsic call
    #[cfg(feature = "runtime-benchmarks")]
    fn get_call_for_did_call_benchmark() -> Self {
        RuntimeCall::System(frame_system::Call::remark { remark: alloc::vec![] })
    }
}

parameter_types! {
    pub const CtypeFee: Balance = 0;
}

impl ctype::Config for Runtime {
    type CtypeCreatorId = AccountId;
    type Currency = Balances;
    type Fee = CtypeFee;
    type FeeCollector = ();

    type EnsureOrigin = EnsureDidOrigin<DidIdentifier, AccountId>;
    type OriginSuccess = DidRawOrigin<AccountId, DidIdentifier>;
    type OverarchingOrigin = EnsureRoot<AccountId>;

    type RuntimeEvent = RuntimeEvent;
    type WeightInfo = weights::ctype::WeightInfo<Runtime>;
}

parameter_types! {
    pub const MaxDelegatedAttestations: u32 = 1000;
    pub const AttestationDeposit: Balance = EXISTENTIAL_DEPOSIT;
}

impl attestation::Config for Runtime {
    type EnsureOrigin = EnsureDidOrigin<DidIdentifier, AccountId>;
    type OriginSuccess = DidRawOrigin<AccountId, DidIdentifier>;
    type RuntimeHoldReason = RuntimeHoldReason;
    type RuntimeEvent = RuntimeEvent;
    type WeightInfo = weights::attestation::WeightInfo<Runtime>;

    type Currency = Balances;
    type Deposit = AttestationDeposit;
    type MaxDelegatedAttestations = MaxDelegatedAttestations;
    type AttesterId = DidIdentifier;
    type AuthorizationId = AuthorizationId<<Runtime as delegation::Config>::DelegationNodeId>;
    type AccessControl = PalletAuthorize<DelegationAc<Runtime>>;
    type BalanceMigrationManager = KiltMigration;
}

parameter_types! {
pub const MaxSignatureByteLength: u16 = 64;
pub const MaxParentChecks: u32 = 5;
pub const MaxRevocations: u32 = 5;
pub const MaxRemovals: u32 = 5;
#[derive(Clone, TypeInfo)]
pub const MaxChildren: u32 = 1000;
pub const DelegationDeposit: Balance = EXISTENTIAL_DEPOSIT;
}

#[cfg(feature = "runtime-benchmarks")]
#[derive(
    Clone, Copy, Default, Debug, Encode, Decode, DecodeWithMemTracking, PartialEq, Eq, TypeInfo,
)]
pub struct DummySignature;

#[cfg(feature = "runtime-benchmarks")]
impl<A> From<(A, alloc::vec::Vec<u8>)> for DummySignature {
    fn from(_: (A, alloc::vec::Vec<u8>)) -> Self {
        DummySignature
    }
}

impl delegation::Config for Runtime {
    type DelegationEntityId = DidIdentifier;
    type DelegationNodeId = Hash;

    type EnsureOrigin = EnsureDidOrigin<DidIdentifier, AccountId>;
    type OriginSuccess = DidRawOrigin<AccountId, DidIdentifier>;

    #[cfg(not(feature = "runtime-benchmarks"))]
    type DelegationSignatureVerification = did::DidSignatureVerify<Runtime>;
    #[cfg(not(feature = "runtime-benchmarks"))]
    type Signature = did::DidSignature;

    #[cfg(feature = "runtime-benchmarks")]
    type Signature = DummySignature;
    #[cfg(feature = "runtime-benchmarks")]
    type DelegationSignatureVerification =
        kilt_support::signature::AlwaysVerify<AccountId, alloc::vec::Vec<u8>, Self::Signature>;

    type RuntimeEvent = RuntimeEvent;
    type RuntimeHoldReason = RuntimeHoldReason;
    type MaxSignatureByteLength = MaxSignatureByteLength;
    type MaxParentChecks = MaxParentChecks;
    type MaxRevocations = MaxRevocations;
    type MaxRemovals = MaxRemovals;
    type MaxChildren = MaxChildren;
    type WeightInfo = weights::delegation::WeightInfo<Runtime>;
    type Currency = Balances;
    type Deposit = DelegationDeposit;
    type BalanceMigrationManager = KiltMigration;
}

parameter_types! {
#[derive(Debug, Clone, Eq, PartialEq, TypeInfo, Decode, Encode, DecodeWithMemTracking)]
pub const MaxNewKeyAgreementKeys: u32 = 10;
#[derive(Clone)]
pub const MaxPublicKeysPerDid: u32 = 20;
#[derive(Debug, Clone, Eq, PartialEq)]
pub const MaxTotalKeyAgreementKeys: u32 = 20;
// Standalone block time is half the duration of a parachain block.
pub const MaxBlocksTxValidity: BlockNumber = HOURS;
pub const DidBaseDeposit: Balance = 0;
pub const KeyDeposit: Balance = 0;
pub const ServiceEndpointDeposit: Balance = 0;
pub const DidFee: Balance = 0;
pub const MaxNumberOfServicesPerDid: u32 = 25;
pub const MaxServiceIdLength: u32 = 50;
pub const MaxServiceTypeLength: u32 = 50;
pub const MaxServiceUrlLength: u32 = 2_000;
pub const MaxNumberOfTypesPerService: u32 = 1;
pub const MaxNumberOfUrlsPerService: u32 = 2;
}

impl did::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type RuntimeCall = RuntimeCall;
    type RuntimeHoldReason = RuntimeHoldReason;
    type RuntimeOrigin = RuntimeOrigin;
    type Currency = Balances;
    type DidIdentifier = DidIdentifier;
    type KeyDeposit = KeyDeposit;
    type ServiceEndpointDeposit = ServiceEndpointDeposit;
    type BaseDeposit = DidBaseDeposit;
    type Fee = DidFee;
    type FeeCollector = ();

    #[cfg(not(feature = "runtime-benchmarks"))]
    type EnsureOrigin = EnsureDidOrigin<DidIdentifier, AccountId>;
    #[cfg(not(feature = "runtime-benchmarks"))]
    type OriginSuccess = DidRawOrigin<AccountId, DidIdentifier>;

    #[cfg(feature = "runtime-benchmarks")]
    type EnsureOrigin = frame_system::EnsureSigned<DidIdentifier>;
    #[cfg(feature = "runtime-benchmarks")]
    type OriginSuccess = DidIdentifier;

    type MaxNewKeyAgreementKeys = MaxNewKeyAgreementKeys;
    type MaxTotalKeyAgreementKeys = MaxTotalKeyAgreementKeys;
    type MaxPublicKeysPerDid = MaxPublicKeysPerDid;
    type MaxBlocksTxValidity = MaxBlocksTxValidity;
    type MaxNumberOfServicesPerDid = MaxNumberOfServicesPerDid;
    type MaxServiceIdLength = MaxServiceIdLength;
    type MaxServiceTypeLength = MaxServiceTypeLength;
    type MaxServiceUrlLength = MaxServiceUrlLength;
    type MaxNumberOfTypesPerService = MaxNumberOfTypesPerService;
    type MaxNumberOfUrlsPerService = MaxNumberOfUrlsPerService;
    type WeightInfo = weights::did::WeightInfo<Runtime>;
    type BalanceMigrationManager = KiltMigration;
    type DidLifecycleHooks = ();
}

parameter_types! {
    pub const DidAssetPrefix: &'static [u8] = b"did:asset:";  // = 10
    pub const MaximumChainIdLength: u32 = 8 + 1 + 32;           // = 41
    pub const MaximumAssetIdLength: u32 = 8 + 1 + 128 + 1 + 78; // = 216
    pub const PublicCredentialsDeposit: Balance = EXISTENTIAL_DEPOSIT;
    pub const MaxEncodedClaimsLength: u32 = 100_000;	// 100 Kb
    pub const MaxSubjectIdLength: u32 = 41 + 1 + 216 + 10;
}

impl public_credentials::Config for Runtime {
    type RuntimeHoldReason = RuntimeHoldReason;
    type AccessControl = PalletAuthorize<DelegationAc<Runtime>>;
    type AttesterId = DidIdentifier;
    type AuthorizationId = AuthorizationId<<Runtime as delegation::Config>::DelegationNodeId>;
    type CredentialId = Hash;
    type CredentialHash = BlakeTwo256;
    type Currency = Balances;
    type Deposit = PublicCredentialsDeposit;
    type EnsureOrigin = EnsureDidOrigin<DidIdentifier, AccountId>;
    type MaxEncodedClaimsLength = MaxEncodedClaimsLength;
    type MaxSubjectIdLength = MaxSubjectIdLength;
    type OriginSuccess = DidRawOrigin<AccountId, DidIdentifier>;
    type RuntimeEvent = RuntimeEvent;
    type SubjectId = AssetDid;
    type WeightInfo = weights::public_credentials::WeightInfo<Runtime>;
    type BalanceMigrationManager = KiltMigration;
}

parameter_types! {
    pub const  MaxMigrationsPerPallet: u32 = 100;
}

impl pallet_kilt_migration::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type Currency = Balances;
    type MaxMigrationsPerPallet = MaxMigrationsPerPallet;
    type WeightInfo = ();
}

parameter_types! {
    pub const DidLookupDeposit: Balance = 10;
}

impl pallet_did_lookup::Config for Runtime {
    type RuntimeHoldReason = RuntimeHoldReason;
    type RuntimeEvent = RuntimeEvent;

    type DidIdentifier = DidIdentifier;

    type Currency = Balances;
    type Deposit = DidLookupDeposit;

    type EnsureOrigin = EnsureDidOrigin<DidIdentifier, AccountId>;
    type AssociateOrigin = Self::EnsureOrigin;
    type OriginSuccess = DidRawOrigin<AccountId, DidIdentifier>;

    type WeightInfo = weights::pallet_did_lookup::WeightInfo<Runtime>;
    type BalanceMigrationManager = KiltMigration;
    // Do not change the below flag to `true` without also deploying a runtime
    // migration which removes any links that point to the same DID!
    type UniqueLinkingEnabled = ConstBool<false>;
}
