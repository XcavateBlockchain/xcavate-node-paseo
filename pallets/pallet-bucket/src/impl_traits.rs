use frame_support::sp_runtime::DispatchResult;

use crate::{
    traits::{BucketManager, Create, Delete, Inspect, UserManagement},
    BucketMetadataOf, Buckets, Config, KeyIdOf, NamespaceMetadataOf, Pallet, TagOf,
};

impl<T: Config> Create<T> for Pallet<T> {
    fn namespace(
        metadata: NamespaceMetadataOf<T>,
        add_manager: Option<T::SubjectId>,
    ) -> DispatchResult {
        Self::do_create_namespace(metadata, add_manager, None)
    }

    fn bucket(namespace_id: T::NamespaceId, metadata: BucketMetadataOf<T>) -> DispatchResult {
        Self::do_create_bucket(namespace_id, metadata, None, None)?;

        Ok(())
    }

    fn tag(bucket_id: T::BucketId, tag: TagOf<T>) -> DispatchResult {
        Self::do_create_tag(bucket_id, tag, None, None)
    }
}

impl<T: Config> Delete<T> for Pallet<T> {
    fn namespace(id: T::NamespaceId) -> DispatchResult {
        Self::do_delete_namespace(id)
    }

    fn bucket(namespace_id: T::NamespaceId, bucket_id: T::BucketId) -> DispatchResult {
        Self::do_delete_bucket(namespace_id, bucket_id)
    }

    fn message(bucket_id: T::BucketId, message_id: T::MessageId) -> DispatchResult {
        Self::do_remove_message(bucket_id, message_id)
    }
}

impl<T: Config> UserManagement<T> for Pallet<T> {
    fn add_manager(namespace_id: T::NamespaceId, new_manager: T::SubjectId) -> DispatchResult {
        Self::do_add_manager(namespace_id, new_manager, None)
    }

    fn remove_manager(namespace_id: T::NamespaceId, manager: T::SubjectId) -> DispatchResult {
        Self::do_remove_manager(namespace_id, manager, None)
    }

    fn add_admin(
        namespace_id: T::NamespaceId,
        bucket_id: T::BucketId,
        new_admin: T::SubjectId,
    ) -> DispatchResult {
        Self::do_add_admin(namespace_id, bucket_id, new_admin, None)
    }

    fn remove_admin(
        namespace_id: T::NamespaceId,
        bucket_id: T::BucketId,
        admin: T::SubjectId,
    ) -> DispatchResult {
        Self::do_remove_admin(namespace_id, bucket_id, admin, None)
    }

    fn add_contributor(
        namespace_id: T::NamespaceId,
        bucket_id: T::BucketId,
        contributor: T::SubjectId,
    ) -> DispatchResult {
        Self::do_add_contributor(namespace_id, bucket_id, contributor, None)
    }

    fn remove_contributor(
        namespace_id: T::NamespaceId,
        bucket_id: T::BucketId,
        contributor: T::SubjectId,
    ) -> DispatchResult {
        Self::do_remove_contributor(namespace_id, bucket_id, contributor, None)
    }
}

impl<T: Config> BucketManager<T> for Pallet<T> {
    fn set_locked(namespace_id: T::NamespaceId, bucket_id: T::BucketId) -> DispatchResult {
        Self::do_lock_bucket(namespace_id, bucket_id, None)
    }

    fn set_writable(
        namespace_id: T::NamespaceId,
        bucket_id: T::BucketId,
        new_encryption_key: KeyIdOf<T>,
    ) -> DispatchResult {
        Self::do_set_key(namespace_id, bucket_id, new_encryption_key, true, None)
    }
}

impl<T: Config> Inspect<T> for Pallet<T> {
    fn is_manager(namespace_id: &T::NamespaceId, subject: &T::SubjectId) -> bool {
        Self::is_manager(namespace_id, subject)
    }

    fn is_admin(bucket_id: &T::BucketId, subject: &T::SubjectId) -> bool {
        Self::is_admin(bucket_id, subject)
    }

    fn is_contributor(bucket_id: &T::BucketId, subject: &T::SubjectId) -> bool {
        Self::is_contributor(bucket_id, subject)
    }

    fn bucket_details(
        namespace_id: &T::NamespaceId,
        bucket_id: &T::BucketId,
    ) -> Option<crate::BucketDetailsOf<T>> {
        Buckets::<T>::get(namespace_id, bucket_id)
    }
}
