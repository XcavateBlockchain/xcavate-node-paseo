use frame_support::{
    derive_impl, parameter_types,
    sp_runtime::{
        traits::{AccountIdLookup, BlakeTwo256, IdentifyAccount, Morph, Verify},
        BuildStorage, MultiSignature,
    },
    traits::MapSuccess,
    BoundedVec,
};
use frame_system::{EnsureRoot, EnsureSigned};
use parity_scale_codec::{Decode, DecodeWithMemTracking, Encode, MaxEncodedLen};
use scale_info::TypeInfo;
use sp_core::{bounded_vec, ConstU128, ConstU32};
use sp_io::TestExternalities;

use super::{Buckets as BucketsStorage, *};
use crate::{
    self as pallet_buckets,
    traits::CallSources,
    types::{Bucket, Message, Status},
};

pub(crate) const ACCOUNT_00: AccountId = AccountId::new([0u8; 32]);
pub(crate) const ACCOUNT_01: AccountId = AccountId::new([1u8; 32]);
pub(crate) const ACCOUNT_99: AccountId = AccountId::new([99u8; 32]);

pub(crate) const DEFAULT_NAMESPACE_ID: u128 = 0;
pub(crate) const DEFAULT_BUCKET_ID: u128 = 4;
pub(crate) const DEFAULT_BALANCE: u128 = 1_000_000_000_000_000;
pub(crate) const DEFAULT_ENCRYPTION_KEY: u128 = 1;

pub(crate) const BUCKET_EXAMPLE_LOCKED: BucketMock = BucketMock {
    metadata: MetadataMock { unique_plus_1: 10 },
    status: Status::Locked,
    next_message_id: 2,
};

pub(crate) const BUCKET_EXAMPLE_UNLOCKED: BucketMock = BucketMock {
    metadata: MetadataMock { unique_plus_1: 10 },
    status: Status::Writable(0),
    next_message_id: 2,
};

pub(crate) fn create_bounded_vec_example(value: u8) -> BoundedVec<u8, MaxStringLength> {
    bounded_vec![value; 32]
}

// Configure a mock runtime to test the pallet.
frame_support::construct_runtime!(
    pub enum Test
    {
        System: frame_system,
        Balances: pallet_balances,
        Buckets: pallet_buckets
    }
);

pub(crate) type Block = frame_system::mocking::MockBlock<Test>;

pub(crate) type BlockNumber = u64;

pub(crate) type AccountId = <<Signature as Verify>::Signer as IdentifyAccount>::AccountId;
pub(crate) type Signature = MultiSignature;

parameter_types! {
    pub const BlockHashCount: BlockNumber = 2400;
}

#[derive_impl(frame_system::config_preludes::ParaChainDefaultConfig as frame_system::DefaultConfig)]
impl frame_system::Config for Test {
    type AccountData = pallet_balances::AccountData<u128>;
    type AccountId = AccountId;
    type BaseCallFilter = frame_support::traits::Everything;
    type Block = Block;
    type BlockHashCount = BlockHashCount;
    type BlockLength = ();
    type BlockWeights = ();
    type DbWeight = ();
    type Hash = sp_core::H256;
    type Hashing = BlakeTwo256;
    type Lookup = AccountIdLookup<AccountId, ()>;
    type MaxConsumers = frame_support::traits::ConstU32<16>;
    type Nonce = u32;
    type OnKilledAccount = ();
    type OnNewAccount = ();
    type OnSetCode = ();
    type PalletInfo = PalletInfo;
    type RuntimeCall = RuntimeCall;
    type RuntimeEvent = RuntimeEvent;
    type RuntimeOrigin = RuntimeOrigin;
    type RuntimeTask = ();
    type SS58Prefix = ();
    type SystemWeightInfo = ();
    type Version = ();
}

impl pallet_balances::Config for Test {
    type AccountStore = System;
    type Balance = u128;
    type DustRemoval = ();
    type ExistentialDeposit = ConstU128<1>;
    type FreezeIdentifier = ();
    type MaxFreezes = ConstU32<0>;
    type MaxLocks = ();
    type MaxReserves = ConstU32<50>;
    type ReserveIdentifier = [u8; 8];
    type RuntimeEvent = RuntimeEvent;
    type RuntimeFreezeReason = RuntimeFreezeReason;
    type RuntimeHoldReason = RuntimeHoldReason;
    type WeightInfo = ();
    type DoneSlashHandler = ();
}

parameter_types! {
    pub const StorageFee: u128 = 100_000;
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

pub struct SuccessOrigin {
    sender: AccountIdOf<Test>,
}
impl SuccessOrigin {
    fn new(sender: AccountIdOf<Test>) -> Self {
        SuccessOrigin { sender }
    }
}

impl CallSources<AccountIdOf<Test>, AccountIdOf<Test>> for SuccessOrigin {
    fn sender(&self) -> AccountIdOf<Test> {
        self.sender.clone()
    }

    fn subject(&self) -> AccountIdOf<Test> {
        self.sender.clone()
    }
}
impl Morph<AccountIdOf<Test>> for SuccessOrigin {
    type Outcome = Self;

    fn morph(a: AccountIdOf<Test>) -> Self::Outcome {
        Self::new(a)
    }
}
type BucketsOrigin = MapSuccess<EnsureSigned<AccountIdOf<Test>>, SuccessOrigin>;

#[derive(
    Clone,
    Default,
    MaxEncodedLen,
    Debug,
    Encode,
    Decode,
    DecodeWithMemTracking,
    TypeInfo,
    PartialEq,
    Eq,
)]
pub struct MetadataInputMock {
    pub unique: u32,
}

#[derive(
    Clone,
    Default,
    MaxEncodedLen,
    Debug,
    Encode,
    Decode,
    DecodeWithMemTracking,
    TypeInfo,
    PartialEq,
    Eq,
)]
pub struct MetadataMock {
    pub unique_plus_1: u32,
}

impl From<MetadataInputMock> for MetadataMock {
    fn from(input: MetadataInputMock) -> Self {
        MetadataMock { unique_plus_1: input.unique + 1 }
    }
}

#[cfg(feature = "runtime-benchmarks")]
pub struct BenchmarkHelper;

#[cfg(feature = "runtime-benchmarks")]
impl crate::benchmarking::BenchmarkHelper<Test> for BenchmarkHelper {
    fn create_force_origin(_seed: u32) -> <Test as frame_system::Config>::RuntimeOrigin {
        frame_system::RawOrigin::Root.into()
    }

    fn create_origin(seed: u32) -> <Test as frame_system::Config>::RuntimeOrigin {
        use frame_support::traits::fungible::Mutate;

        let mut array = [0u8; 32];
        let bytes = seed.to_le_bytes();

        array[..4].copy_from_slice(&bytes);

        let account = AccountId::from(array);
        Balances::set_balance(&account, 100_000_000_000_000_000_000_000);
        let origin = frame_system::RawOrigin::Signed(account);
        origin.into()
    }

    fn get_bucket(
        seed: u32,
    ) -> (
        <Test as crate::Config>::BucketId,
        <Test as crate::Config>::BucketMetadataInput,
        <Test as crate::Config>::BucketMetadata,
    ) {
        (seed.into(), MetadataInputMock { unique: 0 }, MetadataMock { unique_plus_1: 1 })
    }

    fn get_key_id(seed: u32) -> <Test as crate::Config>::KeyId {
        seed.into()
    }

    fn get_message(
        _seed: u32,
    ) -> (
        <Test as crate::Config>::Reference,
        crate::MessageMetadataInputOf<Test>,
        <Test as crate::Config>::MessageMetadata,
    ) {
        let reference =
            frame_support::BoundedVec::<u8, MaxStringLength>::try_from(vec![0; 200]).unwrap();
        (reference, MetadataInputMock { unique: 0 }, MetadataMock { unique_plus_1: 1 })
    }

    fn get_namespace(
        seed: u32,
    ) -> (
        <Test as crate::Config>::NamespaceId,
        <Test as crate::Config>::NamespaceMetadataInput,
        <Test as crate::Config>::NamespaceMetadata,
    ) {
        (seed.into(), MetadataInputMock { unique: 0 }, MetadataMock { unique_plus_1: 1 })
    }
}

pub type BucketMock = Bucket<MetadataMock, u128, u128>;

impl pallet_buckets::Config for Test {
    #[cfg(feature = "runtime-benchmarks")]
    type BenchmarkHelper = BenchmarkHelper;
    type BucketId = u128;
    type BucketMetadata = MetadataMock;
    type BucketMetadataInput = MetadataInputMock;
    type Currency = Balances;
    type FeeBucket = StorageFee;
    type FeeCollector = ();
    type FeeMessage = StorageFee;
    type FeeNamespace = StorageFee;
    type FeeTag = StorageFee;
    type ForceOriginCheck = EnsureRoot<AccountId>;
    type KeyId = u128;
    type MaxStringInputLengthTag = MaxStringLength;
    type MessageId = u128;
    type MessageMetadata = MetadataMock;
    type MessageMetadataInput = MetadataInputMock;
    type NamespaceId = u128;
    type NamespaceMetadata = MetadataMock;
    type NamespaceMetadataInput = MetadataInputMock;
    type OnCallHooks = ();
    type OriginCheck = BucketsOrigin;
    type OriginSuccess = SuccessOrigin;
    type Reference = BoundedVec<u8, MaxStringLength>;
    type RuntimeEvent = RuntimeEvent;
    type SubjectId = AccountIdOf<Test>;
    type WeightInfo = weights::SubstrateWeight<Test>;
    type MaxNameLen = MaxNameLen;
    type MaxUriLen = MaxUriLen;
    type MaxCategoryLen = MaxCategoryLen;
    type MaxProperties = MaxProperties;
    type MaxPropertyKeyLen = MaxPropertyKeyLen;
    type MaxPropertyValueLen = MaxPropertyValueLen;
}

pub(crate) type MessageMock =
    Message<BoundedVec<u8, MaxStringLength>, BoundedVec<u8, MaxStringLength>, MetadataMock>;

pub(crate) fn events() -> Vec<Event<Test>> {
    System::events()
        .into_iter()
        .map(|r| r.event)
        .filter_map(|e| if let RuntimeEvent::Buckets(e) = e { Some(e) } else { None })
        .collect::<Vec<_>>()
}

#[derive(Clone, Default)]
pub struct ExtBuilder {
    balances: Vec<(AccountId, BalanceOf<Test>)>,
    namespace_metadata: Vec<(u128, MetadataMock)>,
    managers: Vec<(u128, AccountId)>,
    buckets: Vec<(u128, u128, BucketMock)>,
    contributors: Vec<(u128, AccountId)>,
    admins: Vec<(u128, AccountId)>,
    tags: Vec<(u128, BoundedVec<u8, MaxStringLength>)>,
    messages: Vec<(u128, u128, MessageMock)>,
}

impl ExtBuilder {
    pub fn with_balances(mut self, balances: Vec<(AccountId, BalanceOf<Test>)>) -> Self {
        self.balances = balances;
        self
    }

    pub fn add_namespace(mut self, id: u128, metadata: MetadataMock) -> Self {
        self.namespace_metadata.push((id, metadata));
        self
    }

    pub fn add_manager(mut self, id: u128, account: AccountId) -> Self {
        self.managers.push((id, account));
        self
    }

    pub fn add_bucket(mut self, namespace_id: u128, bucket_id: u128, bucket: BucketMock) -> Self {
        self.buckets.push((namespace_id, bucket_id, bucket));
        self
    }

    pub fn add_contributor(mut self, bucket_id: u128, account: AccountId) -> Self {
        self.contributors.push((bucket_id, account));
        self
    }

    pub fn add_admin(mut self, bucket_id: u128, account: AccountId) -> Self {
        self.admins.push((bucket_id, account));
        self
    }

    pub fn add_message(mut self, bucket_id: u128, message_id: u128, message: MessageMock) -> Self {
        self.messages.push((bucket_id, message_id, message));
        self
    }

    pub fn build(self) -> TestExternalities {
        let mut storage = frame_system::GenesisConfig::<Test>::default().build_storage().unwrap();
        pallet_balances::GenesisConfig::<Test> {
            balances: self.balances.clone(),
            dev_accounts: None,
        }
        .assimilate_storage(&mut storage)
        .expect("assimilate should not fail");

        let mut externalities = sp_io::TestExternalities::new(storage);
        externalities.execute_with(|| {
            System::set_block_number(1);
            // Initialize the storage with the provided data
            if let Some((_, max_bucket_id, _)) =
                self.buckets.iter().max_by_key(|(_, bucket_id, _)| bucket_id)
            {
                NextBucketId::<Test>::put(*max_bucket_id + 1);
            }

            self.namespace_metadata
                .into_iter()
                .for_each(|metadata| Namespaces::<Test>::insert(metadata.0, metadata.1));

            self.managers.into_iter().for_each(|manager| {
                Managers::<Test>::insert(manager.0, manager.1, ());
            });
            self.buckets.into_iter().for_each(|bucket| {
                let (namespace_id, bucket_id, bucket) = bucket;
                BucketsStorage::<Test>::insert(namespace_id, bucket_id, bucket);
            });
            self.contributors.into_iter().for_each(|contributor| {
                let (bucket_id, account) = contributor;
                Contributors::<Test>::insert(bucket_id, account, ());
            });
            self.admins.into_iter().for_each(|contributor| {
                let (bucket_id, account) = contributor;
                Admins::<Test>::insert(bucket_id, account, ());
            });
            self.tags.into_iter().for_each(|tag| {
                let (bucket_id, tag) = tag;
                Tags::<Test>::insert(bucket_id, tag, ());
            });
            self.messages.into_iter().for_each(|message| {
                let (bucket_id, message_id, message) = message;
                Messages::<Test>::insert(bucket_id, message_id, message);
            });
        });
        externalities
    }

    pub fn build_and_execute_with_sanity_tests(self, test: impl FnOnce()) {
        self.build().execute_with(|| {
            test();
            crate::try_state::do_try_state::<Test>()
                .expect("Sanity test for pallet-bucket failed.");
        })
    }

    #[cfg(feature = "runtime-benchmarks")]
    pub(crate) fn build_with_keystore(self) -> sp_io::TestExternalities {
        use sp_keystore::{testing::MemoryKeystore, KeystoreExt};
        use std::sync::Arc;

        let mut ext = self.build();

        let keystore = MemoryKeystore::new();
        ext.register_extension(KeystoreExt(Arc::new(keystore)));

        ext
    }
}
