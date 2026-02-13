use frame_support::pallet_prelude::*;

use crate::{BucketDetailsOf, BucketMetadataOf, Call, Config, KeyIdOf, NamespaceMetadataOf, TagOf};

/// trait for getting the subject of a call.
pub trait CallSources<Subject, AccountId> {
    fn subject(&self) -> Subject;

    fn sender(&self) -> AccountId;
}

/// On call hooks, which are called before and after the transaction execution.
pub trait OnCallHooks<T: Config> {
    /// Called before in the beginning of the transaction.
    fn pre_call_dispatch(origin: &T::OriginSuccess, call: Call<T>) -> DispatchResult;

    /// Called after the transaction execution.
    fn post_call_dispatch(origin: &T::OriginSuccess, call: Call<T>) -> DispatchResult;
}

impl<T> OnCallHooks<T> for ()
where
    T: Config,
{
    fn pre_call_dispatch(_origin: &<T as Config>::OriginSuccess, _call: Call<T>) -> DispatchResult {
        Ok(())
    }

    fn post_call_dispatch(
        _origin: &<T as Config>::OriginSuccess,
        _call: Call<T>,
    ) -> DispatchResult {
        Ok(())
    }
}

pub trait Create<T: Config> {
    // Creates a new namespace in the pallet.
    fn namespace(metadata: NamespaceMetadataOf<T>, manager: Option<T::SubjectId>)
        -> DispatchResult;

    // Creates a new bucket within a namespace
    fn bucket(namespace_id: T::NamespaceId, metadata: BucketMetadataOf<T>) -> DispatchResult;

    // Creates a new tag for a bucket
    fn tag(bucket_id: T::BucketId, tag: TagOf<T>) -> DispatchResult;
}

pub trait Delete<T: Config> {
    // Removes namespace from pallet
    fn namespace(id: T::NamespaceId) -> DispatchResult;

    // Removes bucket from pallet
    fn bucket(namespace_id: T::NamespaceId, bucket_id: T::BucketId) -> DispatchResult;

    // Removes a message from pallet
    fn message(bucket_id: T::BucketId, message_id: T::MessageId) -> DispatchResult;
}

pub trait UserManagement<T: Config> {
    // Adds a new account to the list of admins.
    fn add_admin(
        namespace_id: T::NamespaceId,
        bucket_id: T::BucketId,
        account: T::SubjectId,
    ) -> DispatchResult;

    // Removes the account from the admins list
    fn remove_admin(
        namespace_id: T::NamespaceId,
        bucket_id: T::BucketId,
        account: T::SubjectId,
    ) -> DispatchResult;

    // Adds a new account to the list of contributors.
    fn add_contributor(
        namespace_id: T::NamespaceId,
        bucket_id: T::BucketId,
        account: T::SubjectId,
    ) -> DispatchResult;

    // Removes the account from the contributors list
    fn remove_contributor(
        namespace_id: T::NamespaceId,
        bucket_id: T::BucketId,
        account: T::SubjectId,
    ) -> DispatchResult;

    // Adds a new account to the list of managers.
    fn add_manager(namespace_id: T::NamespaceId, account: T::SubjectId) -> DispatchResult;

    // Removes the account from the contributors list
    fn remove_manager(namespace_id: T::NamespaceId, account: T::SubjectId) -> DispatchResult;
}

pub trait BucketManager<T: Config> {
    // Pauses the write operation of the bucket
    fn set_locked(namespace_id: T::NamespaceId, bucket_id: T::BucketId) -> DispatchResult;

    // Resume the write operation of the bucket
    fn set_writable(
        namespace_id: T::NamespaceId,
        bucket_id: T::BucketId,
        encryption_key: KeyIdOf<T>,
    ) -> DispatchResult;
}

/// trait for constructing metadata from a source.
pub trait ConstructMetadata<Source, CallOrigin>: Sized {
    type Error: Into<DispatchError>;

    /// Constructs metadata from a source.
    fn construct(source: Source, origin: &CallOrigin) -> Result<Self, Self::Error>;
}

impl<T, MetadataInput, CallOrigin> ConstructMetadata<MetadataInput, CallOrigin> for T
where
    T: From<MetadataInput>,
{
    type Error = DispatchError;

    fn construct(source: MetadataInput, _origin: &CallOrigin) -> Result<Self, Self::Error> {
        Ok(Self::from(source))
    }
}

pub trait Inspect<T: Config> {
    fn is_manager(namespace_id: &T::NamespaceId, subject_id: &T::SubjectId) -> bool;

    fn is_admin(bucket_id: &T::BucketId, subject_id: &T::SubjectId) -> bool;

    fn is_contributor(bucket_id: &T::BucketId, subject_id: &T::SubjectId) -> bool;

    fn bucket_details(
        namespace_id: &T::NamespaceId,
        bucket_id: &T::BucketId,
    ) -> Option<BucketDetailsOf<T>>;
}
