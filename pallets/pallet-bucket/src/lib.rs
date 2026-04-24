#![cfg_attr(not(feature = "std"), no_std)]

pub use pallet::*;

#[cfg(test)]
mod mock;

#[cfg(feature = "runtime-benchmarks")]
pub mod benchmarking;
pub mod weights;
pub use weights::*;

#[cfg(test)]
mod tests;
#[cfg(any(feature = "try-runtime", test))]
mod try_state;

mod functions;
mod impl_traits;
pub mod traits;
pub mod types;

#[frame_support::pallet]
pub mod pallet {

    use frame_support::{
        pallet_prelude::{Member, StorageDoubleMap, *},
        sp_runtime::traits::{CheckedAdd, One},
        traits::{
            fungible::{Balanced, Credit, Inspect},
            OnUnbalanced,
        },
        Blake2_128Concat, Parameter,
    };
    use frame_system::pallet_prelude::*;
    use parity_scale_codec::MaxEncodedLen;

    use crate::{
        traits::{CallSources, ConstructMetadata, OnCallHooks},
        types::{Bucket, Message, MessageInput},
        weights::WeightInfo,
    };

    pub type AccountIdOf<T> = <T as frame_system::Config>::AccountId;

    pub(crate) type SubjectIdOf<T> = <T as Config>::SubjectId;

    pub(crate) type TagOf<T> = BoundedVec<u8, <T as Config>::MaxStringInputLengthTag>;

    pub(crate) type NamespaceMetadataOf<T> = <T as Config>::NamespaceMetadata;

    pub(crate) type BucketMetadataOf<T> = <T as Config>::BucketMetadata;

    pub(crate) type MessageMetadataOf<T> = <T as Config>::MessageMetadata;

    pub(crate) type NamespaceMetadataInputOf<T> = <T as Config>::NamespaceMetadataInput;

    pub(crate) type BucketMetadataInputOf<T> = <T as Config>::BucketMetadataInput;

    pub type MessageMetadataInputOf<T> = <T as Config>::MessageMetadataInput;

    pub(crate) type ReferenceOf<T> = <T as Config>::Reference;

    pub(crate) type CurrencyOf<T> = <T as Config>::Currency;

    pub(crate) type KeyIdOf<T> = <T as Config>::KeyId;

    pub(crate) type BalanceOf<T> = <CurrencyOf<T> as Inspect<AccountIdOf<T>>>::Balance;

    pub(crate) type MessageIdOf<T> = <T as Config>::MessageId;

    pub(crate) type BucketDetailsOf<T> = Bucket<BucketMetadataOf<T>, MessageIdOf<T>, KeyIdOf<T>>;

    pub(crate) type MessageDetailsOf<T> = Message<ReferenceOf<T>, TagOf<T>, MessageMetadataOf<T>>;

    pub(crate) type MessageInputOf<T> =
        MessageInput<TagOf<T>, ReferenceOf<T>, MessageMetadataInputOf<T>>;

    type CreditOf<T> = Credit<<T as frame_system::Config>::AccountId, <T as Config>::Currency>;

    #[pallet::config]
    pub trait Config: frame_system::Config {
        /// The type of the identifier for a bucket.
        type BucketId: Member + Parameter + MaxEncodedLen + CheckedAdd + One + Default + PartialOrd;

        /// The currency that is used to take fees for creating Namespaces, Buckets, and Messages.
        type Currency: Balanced<AccountIdOf<Self>>;

        /// The type of the identifier for the subject most likely the DID.
        type SubjectId: Member + Parameter + MaxEncodedLen;

        type FeeCollector: OnUnbalanced<CreditOf<Self>>;

        /// The fee of the namespace.
        #[pallet::constant]
        type FeeNamespace: Get<BalanceOf<Self>>;

        /// The fee of the Bucket.
        #[pallet::constant]
        type FeeBucket: Get<BalanceOf<Self>>;

        /// The fee of the Message.
        #[pallet::constant]
        type FeeMessage: Get<BalanceOf<Self>>;

        /// The fee of the Tag.
        #[pallet::constant]
        type FeeTag: Get<BalanceOf<Self>>;

        /// The type of the identifier for an namespace.
        type NamespaceId: Member + Parameter + MaxEncodedLen + CheckedAdd + One + Default;

        /// The type of encryption key for the bucket.
        type KeyId: Member + Parameter + MaxEncodedLen;

        /// The input type of the namespace metadata.
        type NamespaceMetadataInput: Member + Parameter + MaxEncodedLen;

        /// The input type of the bucket metadata.
        type BucketMetadataInput: Member + Parameter + MaxEncodedLen;

        /// The input type of the message metadata.
        type MessageMetadataInput: Member + Parameter + MaxEncodedLen;

        /// The type of the namespace metadata.
        type NamespaceMetadata: Member
            + Parameter
            + MaxEncodedLen
            + ConstructMetadata<NamespaceMetadataInputOf<Self>, Self::OriginSuccess>;

        /// The type of the bucket metadata.
        type BucketMetadata: Member
            + Parameter
            + MaxEncodedLen
            + ConstructMetadata<BucketMetadataInputOf<Self>, Self::OriginSuccess>;

        /// The type of the message metadata.
        type MessageMetadata: Member
            + Parameter
            + MaxEncodedLen
            + ConstructMetadata<MessageMetadataInputOf<Self>, Self::OriginSuccess>;

        /// The type of the identifier for a message.
        type MessageId: Member + Parameter + MaxEncodedLen + Default + CheckedAdd + One;

        /// The max length of the string input. Used for Tags.
        #[pallet::constant]
        type MaxStringInputLengthTag: Get<u32>;

        /// The origin check for the force origin.
        type ForceOriginCheck: EnsureOrigin<<Self as frame_system::Config>::RuntimeOrigin>;

        /// The Did origin check.
        type OriginCheck: EnsureOrigin<
            <Self as frame_system::Config>::RuntimeOrigin,
            Success = Self::OriginSuccess,
        >;

        type OriginSuccess: CallSources<Self::SubjectId, AccountIdOf<Self>>;

        /// The reference to the storage layer, where the message is located.
        type Reference: Member + Parameter + MaxEncodedLen;

        type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;

        /// Call hooks executed during each call dispatch.
        /// These hooks can be used to perform additional verification
        /// or execute extra actions before or after the call is executed.
        type OnCallHooks: OnCallHooks<Self>;

        type WeightInfo: WeightInfo;

        #[cfg(feature = "runtime-benchmarks")]
        type BenchmarkHelper: crate::benchmarking::BenchmarkHelper<Self>;

        /// The maximum length for a human-readable name or description string.
        #[pallet::constant]
        type MaxNameLen: Get<u32>;

        /// The maximum length for a URI string (e.g., for a schema link).
        #[pallet::constant]
        type MaxUriLen: Get<u32>;

        /// The maximum length for a category or content-type string.
        #[pallet::constant]
        type MaxCategoryLen: Get<u32>;

        /// The maximum number of key-value pairs in the properties map.
        #[pallet::constant]
        type MaxProperties: Get<u32>;

        /// The maximum length for a key in the properties map.
        #[pallet::constant]
        type MaxPropertyKeyLen: Get<u32>;

        /// The maximum length for a value in the properties map.
        #[pallet::constant]
        type MaxPropertyValueLen: Get<u32>;
    }

    /// The next available namespace identifier.
    #[pallet::storage]
    #[pallet::getter(fn next_namespace_id)]
    pub type NextNamespaceId<T: Config> = StorageValue<_, T::NamespaceId, ValueQuery>;

    /// Namespaces stored on chain
    ///
    /// It maps from any entity id (called namespace id) to the metadata of the namespace.
    #[pallet::storage]
    #[pallet::getter(fn namespace_with_id)]
    pub type Namespaces<T: Config> =
        StorageMap<_, Blake2_128Concat, T::NamespaceId, T::NamespaceMetadata, OptionQuery>;

    /// Buckets stored on chain.
    ///
    /// Double storage map from namespace id to bucket id to the metadata of the bucket.
    #[pallet::storage]
    #[pallet::getter(fn bucket_with_id)]
    pub type Buckets<T: Config> = StorageDoubleMap<
        _,
        Blake2_128Concat,
        T::NamespaceId,
        Blake2_128Concat,
        T::BucketId,
        BucketDetailsOf<T>,
        OptionQuery,
    >;

    /// Messages stored on chain.
    ///
    /// Double storage map from bucket id to message id to the metadata of the message.
    #[pallet::storage]
    #[pallet::getter(fn message_with_id)]
    pub type Messages<T: Config> = StorageDoubleMap<
        _,
        Blake2_128Concat,
        T::BucketId,
        Blake2_128Concat,
        T::MessageId,
        MessageDetailsOf<T>,
        OptionQuery,
    >;

    /// Contributors stored on chain.
    ///
    /// Double storage map from bucket id to contributor id to an empty tuple.
    /// By using a double storage map, we can have multiple contributors for the same bucket.
    /// The double storage map allows efficient look ups. By using a simple storage map, we would
    /// have to iterate over all contributors for the bucket.
    #[pallet::storage]
    #[pallet::getter(fn contributor_with_id)]
    pub type Contributors<T: Config> = StorageDoubleMap<
        _,
        Blake2_128Concat,
        T::BucketId,
        Blake2_128Concat,
        SubjectIdOf<T>,
        (),
        OptionQuery,
    >;

    /// Admins stored on chain.
    ///
    /// Double storage map from bucket id to admin id to an empty tuple.
    /// By using a double storage map, we can have multiple admins for the same bucket.
    /// The double storage map allows efficient look ups. By using a simple storage map, we would
    /// have to iterate over all admins for the bucket.
    #[pallet::storage]
    #[pallet::getter(fn admin_with_id)]
    pub type Admins<T: Config> = StorageDoubleMap<
        _,
        Blake2_128Concat,
        T::BucketId,
        Blake2_128Concat,
        SubjectIdOf<T>,
        (),
        OptionQuery,
    >;

    /// Managers stored on chain.
    ///
    /// Double storage map from namespace id to manager id to an empty tuple.
    /// By using a double storage map, we can have multiple managers for the same namespace.
    /// The double storage map allows efficient look ups. By using a simple storage map, we would
    /// have to iterate over all managers for the namespace.
    #[pallet::storage]
    #[pallet::getter(fn manager_with_id)]
    pub type Managers<T: Config> = StorageDoubleMap<
        _,
        Blake2_128Concat,
        T::NamespaceId,
        Blake2_128Concat,
        SubjectIdOf<T>,
        (),
        OptionQuery,
    >;

    /// Tags stored on chain.
    ///
    /// Tags are only available for a specific bucket.
    #[pallet::storage]
    #[pallet::getter(fn tag_with_id)]
    pub type Tags<T: Config> = StorageDoubleMap<
        _,
        Blake2_128Concat,
        T::BucketId,
        Blake2_128Concat,
        TagOf<T>,
        (),
        OptionQuery,
    >;

    /// Tracks how many messages reference a given tag within a specific bucket.
    #[pallet::storage]
    pub type TagMessages<T: Config> = StorageDoubleMap<
        _,
        Blake2_128Concat,
        T::BucketId,
        Blake2_128Concat,
        TagOf<T>,
        u32,
        ValueQuery,
    >;

    /// The next namespace id to be used.
    #[pallet::storage]
    #[pallet::getter(fn next_bucket_id)]
    pub type NextBucketId<T: Config> = StorageValue<_, T::BucketId, ValueQuery>;

    #[pallet::pallet]
    pub struct Pallet<T>(_);

    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {
        /// A new namespace has been created.
        NamespaceCreated {
            namespace_id: T::NamespaceId,
            metadata: T::NamespaceMetadata,
            creator: Option<SubjectIdOf<T>>,
        },

        /// A contributor is assigned to a bucket.
        ContributorAdded {
            namespace_id: T::NamespaceId,
            bucket_id: T::BucketId,
            contributor: SubjectIdOf<T>,
            caller: Option<SubjectIdOf<T>>,
        },

        /// A contributor is removed from a bucket.
        ContributorRemoved {
            namespace_id: T::NamespaceId,
            bucket_id: T::BucketId,
            contributor: SubjectIdOf<T>,
            caller: Option<SubjectIdOf<T>>,
        },

        /// A new admin is assigned to a bucket.
        AdminAdded {
            namespace_id: T::NamespaceId,
            bucket_id: T::BucketId,
            admin: SubjectIdOf<T>,
            caller: Option<SubjectIdOf<T>>,
        },

        /// An admin is removed from a bucket.
        AdminRemoved {
            namespace_id: T::NamespaceId,
            bucket_id: T::BucketId,
            admin: SubjectIdOf<T>,
            caller: Option<SubjectIdOf<T>>,
        },

        /// A new manager is assigned to an namespace.
        ManagerAdded {
            namespace_id: T::NamespaceId,
            manager: SubjectIdOf<T>,
            caller: Option<SubjectIdOf<T>>,
        },

        /// A manager is removed from an namespace.
        ManagerRemoved {
            namespace_id: T::NamespaceId,
            manager: SubjectIdOf<T>,
            caller: Option<SubjectIdOf<T>>,
        },

        /// A new bucket has been created.
        BucketCreated {
            namespace_id: T::NamespaceId,
            bucket_id: T::BucketId,
            bucket: BucketDetailsOf<T>,
            creator: Option<SubjectIdOf<T>>,
        },

        /// A bucket has been paused for writing.
        PausedBucket {
            namespace_id: T::NamespaceId,
            bucket_id: T::BucketId,
            bucket: BucketDetailsOf<T>,
            caller: Option<SubjectIdOf<T>>,
        },

        /// A bucket is writable with a specific key.
        /// This event is independent of the bucket being previously paused or not.
        BucketWritableWithKey {
            namespace_id: T::NamespaceId,
            bucket_id: T::BucketId,
            new_encryption_key: KeyIdOf<T>,
            bucket: BucketDetailsOf<T>,
            caller: Option<SubjectIdOf<T>>,
        },

        /// A new tag has been created.
        NewTag { bucket_id: T::BucketId, tag: TagOf<T>, creator: Option<SubjectIdOf<T>> },

        /// A new message has been written.
        NewMessage {
            namespace_id: T::NamespaceId,
            bucket_id: T::BucketId,
            message_id: T::MessageId,
            message: MessageDetailsOf<T>,
            contributor: SubjectIdOf<T>,
        },

        /// An namespace has been deleted.
        NamespaceDeleted { namespace_id: T::NamespaceId, metadata: T::NamespaceMetadata },

        /// A bucket has been deleted.
        BucketDeleted {
            namespace_id: T::NamespaceId,
            bucket_id: T::BucketId,
            bucket: BucketDetailsOf<T>,
        },

        /// A tag has been deleted.
        TagDeleted { bucket_id: T::BucketId, tag: TagOf<T> },

        /// A message has been deleted.
        MessageDeleted {
            bucket_id: T::BucketId,
            message_id: T::MessageId,
            message: MessageDetailsOf<T>,
        },
    }

    #[pallet::error]
    pub enum Error<T> {
        /// The requested namespace already exists.
        NamespaceAlreadyExists,
        /// The requested namespace does not exist.
        UnknownNamespace,
        /// The bucket does not exist.
        UnknownBucket,
        /// The bucket is locked.
        BucketIsLocked,
        /// The requested message is unknown.
        UnknownMessage,
        /// There are dangling buckets for the namespace.
        DanglingBuckets,
        /// There are dangling messages for the bucket.
        DanglingMessages,
        /// The origin is not authorized to perform the manager action for the namespace.
        NotManager,
        /// The contributor does not exist for the requested bucket.
        NotContributor,
        /// The origin is not authorized to perform the manager action for the bucket.
        NotAdmin,
        /// The given tag does not exist.
        UnknownTag,
        /// The account is unable to pay the fees.
        UnableToPayFees,
        /// There are dangling contributors
        DanglingContributors,
        /// There are dangling admins
        DanglingAdmins,
        /// There are dangling managers
        DanglingManagers,
        /// Overflow in arithmetic operations.
        ArithmeticOverflow,
        /// Underflow in arithmetic operations.
        ArithmeticUnderflow,
        /// Cannot remove the last manager of a namespace.
        LastManagerRemoval,
        /// There are dangling Tags.
        DanglingTags,
    }

    #[pallet::hooks]
    impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
        #[cfg(feature = "try-runtime")]
        fn try_state(
            _n: BlockNumberFor<T>,
        ) -> Result<(), frame_support::sp_runtime::TryRuntimeError> {
            crate::try_state::do_try_state::<T>()
        }
    }

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        /// Create a new namespace.
        ///
        /// The namespace is created with the given metadata.
        /// The metadata is used to store additional information about the namespace.
        /// If successful, the events `NamespaceCreated` and `ManagerAdded` will be emitted.
        ///
        /// # Parameters
        /// - `metadata_input`: The metadata of the namespace to be created.
        #[pallet::call_index(0)]
        #[pallet::weight(T::WeightInfo::create_namespace())]
        pub fn create_namespace(
            origin: OriginFor<T>,
            metadata_input: NamespaceMetadataInputOf<T>,
        ) -> DispatchResult {
            let success_origin = T::OriginCheck::ensure_origin(origin)?;

            let call = Call::<T>::new_call_variant_create_namespace(metadata_input.clone());
            T::OnCallHooks::pre_call_dispatch(&success_origin, call.clone())?;

            let manager = success_origin.subject();
            let sender = success_origin.sender();
            let metadata = T::NamespaceMetadata::construct(metadata_input, &success_origin)
                .map_err(|e| e.into())?;

            Self::do_create_namespace(metadata, Some(manager), Some(sender))?;

            T::OnCallHooks::post_call_dispatch(&success_origin, call)
        }

        /// Add a contributor to a bucket.
        ///
        /// The contributor is added to the bucket with the given id.
        /// The contributor is allowed to write messages to the bucket.
        /// If successful, a `ContributorAdded` event will be emitted.
        ///
        /// # Parameters
        /// - `namespace_id`: The id of the namespace to which the bucket belongs.
        /// - `bucket_id`: The id of the bucket to which the contributor is added.
        /// - `contributor`: The id of the contributor to be added.
        #[pallet::call_index(1)]
        #[pallet::weight(T::WeightInfo::add_contributor())]
        pub fn add_contributor(
            origin: OriginFor<T>,
            namespace_id: T::NamespaceId,
            bucket_id: T::BucketId,
            contributor: SubjectIdOf<T>,
        ) -> DispatchResult {
            let success_origin = T::OriginCheck::ensure_origin(origin)?;

            let call = Call::<T>::new_call_variant_add_contributor(
                namespace_id.clone(),
                bucket_id.clone(),
                contributor.clone(),
            );
            T::OnCallHooks::pre_call_dispatch(&success_origin, call.clone())?;

            let admin = success_origin.subject();

            Self::do_add_contributor(namespace_id, bucket_id, contributor, Some(admin))?;

            T::OnCallHooks::post_call_dispatch(&success_origin, call)
        }

        /// Remove a contributor from a bucket.
        ///
        /// The contributor is removed from the bucket with the given id.
        /// The contributor is no longer allowed to write messages to the bucket.
        /// If successful, a `ContributorRemoved` event will be emitted.
        ///
        /// # Parameters
        /// - `namespace_id`: The id of the namespace to which the bucket belongs.
        /// - `bucket_id`: The id of the bucket from which the contributor is removed.
        /// - `contributor`: The id of the contributor to be removed.
        #[pallet::call_index(2)]
        #[pallet::weight(T::WeightInfo::remove_contributor())]
        pub fn remove_contributor(
            origin: OriginFor<T>,
            namespace_id: T::NamespaceId,
            bucket_id: T::BucketId,
            contributor: SubjectIdOf<T>,
        ) -> DispatchResult {
            let success_origin = T::OriginCheck::ensure_origin(origin)?;

            let call = Call::<T>::new_call_variant_remove_contributor(
                namespace_id.clone(),
                bucket_id.clone(),
                contributor.clone(),
            );
            T::OnCallHooks::pre_call_dispatch(&success_origin, call.clone())?;

            let admin = success_origin.subject();

            Self::do_remove_contributor(namespace_id, bucket_id, contributor, Some(admin))?;

            T::OnCallHooks::post_call_dispatch(&success_origin, call)
        }

        /// Add a new admin to a bucket.
        ///
        /// The admin is added to the bucket with the given id.
        /// The admin is allowed to manage the bucket.
        /// If successful, a `AdminAdded` event will be emitted.
        ///
        /// # Parameters
        /// - `namespace_id`: The id of the namespace to which the bucket belongs.
        /// - `bucket_id`: The id of the bucket to which the admin is added.
        /// - `admin`: The id of the admin to be added.
        #[pallet::call_index(3)]
        #[pallet::weight(T::WeightInfo::add_admin())]
        pub fn add_admin(
            origin: OriginFor<T>,
            namespace_id: T::NamespaceId,
            bucket_id: T::BucketId,
            admin: SubjectIdOf<T>,
        ) -> DispatchResult {
            let success_origin = T::OriginCheck::ensure_origin(origin)?;

            let call = Call::<T>::new_call_variant_add_admin(
                namespace_id.clone(),
                bucket_id.clone(),
                admin.clone(),
            );
            T::OnCallHooks::pre_call_dispatch(&success_origin, call.clone())?;

            let manager = success_origin.subject();

            Self::do_add_admin(namespace_id, bucket_id, admin, Some(manager))?;

            T::OnCallHooks::post_call_dispatch(&success_origin, call)
        }

        /// Remove an admin from a bucket.
        ///
        /// The admin is removed from the bucket with the given id.
        /// The admin is no longer allowed to manage the bucket.
        /// If successful, a `AdminRemoved` event will be emitted.
        ///
        /// # Parameters
        /// - `namespace_id`: The id of the namespace to which the bucket belongs.
        /// - `bucket_id`: The id of the bucket from which the admin is removed.
        /// - `admin`: The id of the admin to be removed.
        #[pallet::call_index(4)]
        #[pallet::weight(T::WeightInfo::remove_admin())]
        pub fn remove_admin(
            origin: OriginFor<T>,
            namespace_id: T::NamespaceId,
            bucket_id: T::BucketId,
            admin: SubjectIdOf<T>,
        ) -> DispatchResult {
            let success_origin = T::OriginCheck::ensure_origin(origin)?;

            let call = Call::<T>::new_call_variant_remove_admin(
                namespace_id.clone(),
                bucket_id.clone(),
                admin.clone(),
            );
            T::OnCallHooks::pre_call_dispatch(&success_origin, call.clone())?;

            let manager = success_origin.subject();

            Self::do_remove_admin(namespace_id, bucket_id, admin, Some(manager))?;

            T::OnCallHooks::post_call_dispatch(&success_origin, call)
        }

        /// Add a new manager to a namespace.
        ///
        /// The manager is added to the namespace with the given id.
        /// The manager is allowed to manage the namespace.
        /// If successful, a `ManagerAdded` event will be emitted.
        ///
        /// # Parameters
        /// - `namespace_id`: The id of the namespace to which the manager is added.
        /// - `new_manager`: The id of the manager to be added.
        #[pallet::call_index(5)]
        #[pallet::weight(T::WeightInfo::add_manager())]
        pub fn add_manager(
            origin: OriginFor<T>,
            namespace_id: T::NamespaceId,
            new_manager: SubjectIdOf<T>,
        ) -> DispatchResult {
            let success_origin = T::OriginCheck::ensure_origin(origin)?;

            let call =
                Call::<T>::new_call_variant_add_manager(namespace_id.clone(), new_manager.clone());
            T::OnCallHooks::pre_call_dispatch(&success_origin, call.clone())?;

            let manager = success_origin.subject();

            // add manager to storage
            Self::do_add_manager(namespace_id, new_manager, Some(manager))?;

            T::OnCallHooks::post_call_dispatch(&success_origin, call)
        }

        /// Remove a manager from a namespace.
        ///
        /// The manager is removed from the namespace with the given id.
        /// The manager is no longer allowed to manage the namespace.
        /// If successful, a `ManagerRemoved` event will be emitted.
        ///
        /// # Parameters
        /// - `namespace_id`: The id of the namespace from which the manager is removed.
        /// - `old_manager`: The id of the manager to be removed.
        #[pallet::call_index(6)]
        #[pallet::weight(T::WeightInfo::remove_manager())]
        pub fn remove_manager(
            origin: OriginFor<T>,
            namespace_id: T::NamespaceId,
            old_manager: SubjectIdOf<T>,
        ) -> DispatchResult {
            let success_origin = T::OriginCheck::ensure_origin(origin)?;

            let call = Call::<T>::new_call_variant_remove_manager(
                namespace_id.clone(),
                old_manager.clone(),
            );
            T::OnCallHooks::pre_call_dispatch(&success_origin, call.clone())?;

            let manager = success_origin.subject();

            Self::do_remove_manager(namespace_id, old_manager, Some(manager))?;

            T::OnCallHooks::post_call_dispatch(&success_origin, call)
        }

        /// Create a new bucket.
        ///
        /// The bucket is created with the given metadata.
        /// The metadata is used to store additional information about the bucket.
        /// If successful the event `BucketCreated` will be emitted.
        ///
        /// # Parameters
        /// - `namespace_id`: The id of the namespace to which the bucket belongs.
        /// - `metadata_input`: The metadata of the bucket to be created.
        #[pallet::call_index(7)]
        #[pallet::weight(T::WeightInfo::create_bucket())]
        pub fn create_bucket(
            origin: OriginFor<T>,
            namespace_id: T::NamespaceId,
            metadata_input: BucketMetadataInputOf<T>,
        ) -> DispatchResult {
            let success_origin = T::OriginCheck::ensure_origin(origin)?;

            let call = Call::<T>::new_call_variant_create_bucket(
                namespace_id.clone(),
                metadata_input.clone(),
            );
            T::OnCallHooks::pre_call_dispatch(&success_origin, call.clone())?;

            let manager = success_origin.subject();
            let sender = success_origin.sender();
            let metadata = T::BucketMetadata::construct(metadata_input, &success_origin)
                .map_err(|e| e.into())?;

            Self::do_create_bucket(namespace_id, metadata, Some(manager), Some(sender))?;

            T::OnCallHooks::post_call_dispatch(&success_origin, call)
        }

        /// Pause writing to a bucket.
        ///
        /// The bucket is paused for writing.
        /// The bucket is locked and no new messages can not be written to it.
        /// Only admin can pause the bucket.
        /// If successful, a `PausedBucket` event will be emitted.
        ///
        /// # Parameters
        /// - `namespace_id`: The id of the namespace to which the bucket belongs.
        /// - `bucket_id`: The id of the bucket to be paused.
        #[pallet::call_index(8)]
        #[pallet::weight(T::WeightInfo::pause_writing())]
        pub fn pause_writing(
            origin: OriginFor<T>,
            namespace_id: T::NamespaceId,
            bucket_id: T::BucketId,
        ) -> DispatchResult {
            let success_origin = T::OriginCheck::ensure_origin(origin)?;

            let call =
                Call::<T>::new_call_variant_pause_writing(namespace_id.clone(), bucket_id.clone());
            T::OnCallHooks::pre_call_dispatch(&success_origin, call.clone())?;

            Self::do_lock_bucket(namespace_id, bucket_id, Some(success_origin.subject()))?;

            T::OnCallHooks::post_call_dispatch(&success_origin, call)
        }

        /// Resume writing to a bucket.
        ///
        /// The bucket is resumed for writing.
        /// The bucket is unlocked and new messages can be written to it.
        /// Only admin can resume the bucket.
        /// If successful, an `BucketWritableWithKey` event will be emitted.
        ///
        /// # Parameters
        /// - `namespace_id`: The id of the namespace to which the bucket belongs.
        /// - `bucket_id`: The id of the bucket to be resumed.
        /// - `new_encryption_key`: The new encryption key to be used for the bucket.
        #[pallet::call_index(9)]
        #[pallet::weight(T::WeightInfo::resume_writing())]
        pub fn resume_writing(
            origin: OriginFor<T>,
            namespace_id: T::NamespaceId,
            bucket_id: T::BucketId,
            new_encryption_key: KeyIdOf<T>,
        ) -> DispatchResult {
            let success_origin = T::OriginCheck::ensure_origin(origin)?;

            let call = Call::<T>::new_call_variant_resume_writing(
                namespace_id.clone(),
                bucket_id.clone(),
                new_encryption_key.clone(),
            );
            T::OnCallHooks::pre_call_dispatch(&success_origin, call.clone())?;

            let admin = success_origin.subject();

            Self::do_set_key(namespace_id, bucket_id, new_encryption_key, true, Some(admin))?;

            T::OnCallHooks::post_call_dispatch(&success_origin, call)
        }

        /// Create a new tag.
        ///
        /// The tag is created with the content.
        /// Only Admin can create a tag.
        /// If successful, a `NewTag` event will be emitted.
        ///
        /// # Parameters
        /// - `bucket_id`: The id of the bucket to which the tag belongs.
        /// - `new_tag`: The tag to be created.
        #[pallet::call_index(10)]
        #[pallet::weight(T::WeightInfo::create_tag())]
        pub fn create_tag(
            origin: OriginFor<T>,
            bucket_id: T::BucketId,
            new_tag: TagOf<T>,
        ) -> DispatchResult {
            let success_origin = T::OriginCheck::ensure_origin(origin)?;

            let call = Call::<T>::new_call_variant_create_tag(bucket_id.clone(), new_tag.clone());
            T::OnCallHooks::pre_call_dispatch(&success_origin, call.clone())?;

            let manager = success_origin.subject();
            let sender = success_origin.sender();

            Self::do_create_tag(bucket_id, new_tag, Some(manager), Some(sender))?;

            T::OnCallHooks::post_call_dispatch(&success_origin, call)
        }

        /// Rotate the encryption key of a bucket.
        ///
        /// The encryption key is rotated for the bucket with the given id.
        /// The bucket is unlocked and new messages can be written to it.
        /// Only admin can rotate the bucket key.
        /// If successful, an `BucketWritableWithKey` event will be emitted.
        ///
        /// # Parameters
        /// - `namespace_id`: The id of the namespace to which the bucket belongs.
        /// - `bucket_id`: The id of the bucket to be rotated.
        /// - `new_encryption_key`: The new encryption key to be used for the bucket.
        #[pallet::call_index(11)]
        #[pallet::weight(T::WeightInfo::rotate_key())]
        pub fn rotate_key(
            origin: OriginFor<T>,
            namespace_id: T::NamespaceId,
            bucket_id: T::BucketId,
            new_encryption_key: KeyIdOf<T>,
        ) -> DispatchResult {
            let success_origin = T::OriginCheck::ensure_origin(origin)?;

            let call = Call::<T>::new_call_variant_rotate_key(
                namespace_id.clone(),
                bucket_id.clone(),
                new_encryption_key.clone(),
            );
            T::OnCallHooks::pre_call_dispatch(&success_origin, call.clone())?;

            Self::do_set_key(
                namespace_id,
                bucket_id,
                new_encryption_key,
                false,
                Some(success_origin.subject()),
            )?;

            T::OnCallHooks::post_call_dispatch(&success_origin, call)
        }

        /// Write a new message to a bucket.
        ///
        /// The message is created with the content.
        /// Only contributors can write a message to the bucket.
        /// If successful, a `NewMessage` event will be emitted.
        ///
        /// # Parameters
        /// - `namespace_id`: The id of the namespace to which the bucket belongs.
        /// - `bucket_id`: The id of the bucket to which the message belongs.
        /// - `message_input`: The message to be created.
        #[pallet::call_index(12)]
        #[pallet::weight(T::WeightInfo::write())]
        pub fn write(
            origin: OriginFor<T>,
            namespace_id: T::NamespaceId,
            bucket_id: T::BucketId,
            message_input: MessageInputOf<T>,
        ) -> DispatchResult {
            let success_origin = T::OriginCheck::ensure_origin(origin)?;

            let call = Call::<T>::new_call_variant_write(
                namespace_id.clone(),
                bucket_id.clone(),
                message_input.clone(),
            );
            T::OnCallHooks::pre_call_dispatch(&success_origin, call.clone())?;

            let contributor = success_origin.subject();
            let sender = success_origin.sender();

            let message_details =
                MessageDetailsOf::<T>::from_message_input(message_input, &success_origin)
                    .map_err(|err| err.into())?;

            Self::do_create_message(
                namespace_id,
                bucket_id,
                message_details,
                contributor,
                Some(sender),
            )?;

            T::OnCallHooks::post_call_dispatch(&success_origin, call)
        }

        /// Forcefully removes a namespace.
        ///
        /// This function is only available for the force origin.
        /// It will remove the specified namespace. To avoid dangling buckets, all buckets
        /// within the namespace must be removed beforehand.
        /// If successful, a `NamespaceDeleted` event will be emitted.
        ///
        /// # Parameters
        /// - `namespace_id`: The ID of the namespace to be removed.
        #[pallet::call_index(13)]
        #[pallet::weight(T::WeightInfo::force_remove_namespace())]
        pub fn force_remove_namespace(
            origin: OriginFor<T>,
            namespace_id: T::NamespaceId,
        ) -> DispatchResult {
            T::ForceOriginCheck::ensure_origin(origin)?;
            Self::do_delete_namespace(namespace_id)
        }

        /// Forcefully removes a bucket.
        ///
        /// This function is only available for the force origin.
        /// It will remove the specified bucket. To avoid dangling messages, all messages
        /// within the bucket must be removed beforehand.
        /// If successful, a `BucketDeleted` event will be emitted.
        ///
        /// # Parameters
        /// - `namespace_id`: The ID of the namespace to which the bucket belongs.
        /// - `bucket_id`: The ID of the bucket to be removed.
        #[pallet::call_index(14)]
        #[pallet::weight(T::WeightInfo::force_remove_bucket())]
        pub fn force_remove_bucket(
            origin: OriginFor<T>,
            namespace_id: T::NamespaceId,
            bucket_id: T::BucketId,
        ) -> DispatchResult {
            T::ForceOriginCheck::ensure_origin(origin)?;
            Self::do_delete_bucket(namespace_id, bucket_id)
        }

        /// Forcefully removes a tag.
        ///
        /// This function is only available for the force origin.
        /// It will remove the specified tag. To avoid dangling messages, all messages
        /// that use the tag must be removed beforehand.
        /// If successful, a `TagDeleted` event will be emitted.
        ///
        /// # Parameters
        /// - `bucket_id`: The id of the bucket to which the tag belongs.
        /// - `tag`: The tag to be deleted.
        #[pallet::call_index(15)]
        #[pallet::weight(T::WeightInfo::force_remove_tag())]
        pub fn force_remove_tag(
            origin: OriginFor<T>,
            bucket_id: T::BucketId,
            tag: TagOf<T>,
        ) -> DispatchResult {
            T::ForceOriginCheck::ensure_origin(origin)?;
            Self::do_delete_tag(bucket_id, tag)
        }

        /// Forcefully removes a message.
        ///
        /// This function is only available for the force origin.
        /// It will remove the specified message.
        /// If successful, a `MessageDeleted` event will be emitted.
        ///
        /// # Parameters
        /// - `bucket_id`: The ID of the bucket to which the message belongs.
        /// - `message_id`: The ID of the message to be removed.
        #[pallet::call_index(16)]
        #[pallet::weight(T::WeightInfo::force_remove_message())]
        pub fn force_remove_message(
            origin: OriginFor<T>,
            bucket_id: T::BucketId,
            message_id: T::MessageId,
        ) -> DispatchResult {
            T::ForceOriginCheck::ensure_origin(origin)?;
            Self::do_remove_message(bucket_id, message_id)
        }

        /// Forcefully adds a manager to a namespace.
        ///
        /// This function is only available for the force origin.
        /// It will add the specified manager to the namespace.
        /// If successful, a `ManagerAdded` event will be emitted.
        ///
        /// # Parameters
        /// - `namespace_id`: The ID of the namespace to which the manager belongs.
        /// - `manager`: The ID of the manager to be added.
        #[pallet::call_index(17)]
        #[pallet::weight(T::WeightInfo::force_add_manager())]
        pub fn force_add_manager(
            origin: OriginFor<T>,
            namespace_id: T::NamespaceId,
            manager: SubjectIdOf<T>,
        ) -> DispatchResult {
            T::ForceOriginCheck::ensure_origin(origin)?;
            Self::do_add_manager(namespace_id, manager, None)
        }
    }
}
