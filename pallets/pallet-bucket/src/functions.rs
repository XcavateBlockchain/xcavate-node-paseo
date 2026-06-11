use frame_support::{
    pallet_prelude::*,
    sp_runtime::{
        traits::{CheckedAdd, One},
        ArithmeticError, Saturating,
    },
    traits::{
        fungible::Balanced,
        tokens::{Fortitude, Precision, Preservation},
        OnUnbalanced,
    },
    StorageDoubleMap,
};

use crate::{
    types::{Bucket, Message},
    AccountIdOf, Admins, BalanceOf, BucketDetailsOf, BucketMetadataOf, Buckets, Config,
    Contributors, Error, Event, Managers, MessageMetadataOf, Messages, NamespaceMetadataOf,
    Namespaces, NextBucketId, NextNamespaceId, Pallet, ReferenceOf, SubjectIdOf, TagMessages,
    TagOf, Tags,
};

impl<T: Config> Pallet<T> {
    /// Check if the subject is a manager for the namespace.
    pub fn is_manager(namespace_id: &T::NamespaceId, subject: &SubjectIdOf<T>) -> bool {
        Managers::<T>::contains_key(namespace_id, subject)
    }

    /// Check if the subject is an admin for the bucket.
    pub fn is_admin(bucket_id: &T::BucketId, subject: &SubjectIdOf<T>) -> bool {
        Admins::<T>::contains_key(bucket_id, subject)
    }

    /// Check if the subject is a contributor for the bucket.
    pub fn is_contributor(bucket_id: &T::BucketId, subject: &SubjectIdOf<T>) -> bool {
        Contributors::<T>::contains_key(bucket_id, subject)
    }

    /// Ensures that the subject is a manager for the specified namespace.
    /// If the subject is not a manager, this function returns the `NotManager` error.
    pub(super) fn ensure_is_manager(
        namespace_id: &T::NamespaceId,
        subject: &SubjectIdOf<T>,
    ) -> Result<(), Error<T>> {
        ensure!(Self::is_manager(namespace_id, subject), Error::<T>::NotManager);

        Ok(())
    }

    /// Ensures that the subject is an admin for the specified bucket.
    /// If the subject is not an admin, this function returns the `NotAdmin` error.
    pub(super) fn ensure_is_admin(
        bucket_id: &T::BucketId,
        subject: &SubjectIdOf<T>,
    ) -> Result<(), Error<T>> {
        ensure!(Self::is_admin(bucket_id, subject), Error::<T>::NotAdmin);

        Ok(())
    }

    /// Ensures that the subject is a contributor for the specified bucket.
    /// If the subject is not a contributor, this function returns the `NotContributor` error.
    pub(super) fn ensure_is_contributor(
        bucket_id: &T::BucketId,
        subject: &SubjectIdOf<T>,
    ) -> Result<(), Error<T>> {
        ensure!(Self::is_contributor(bucket_id, subject), Error::<T>::NotContributor);

        Ok(())
    }

    /// Creates a new namespace with the given metadata, assigns an optional manager, and deducts fees if a payer is provided.
    ///
    /// This function ensures that the namespace does not already exist. If a payer is specified, the required fees are deducted from their account.
    /// The namespace metadata is then stored, and if a caller is provided, they are assigned as the manager of the namespace.
    /// A `NamespaceCreated` event is emitted upon successful creation.
    pub(super) fn do_create_namespace(
        metadata: NamespaceMetadataOf<T>,
        caller: Option<SubjectIdOf<T>>,
        payer: Option<AccountIdOf<T>>,
    ) -> DispatchResult {
        let namespace_id = NextNamespaceId::<T>::get();
        // check if namespace is already created
        ensure!(!Namespaces::<T>::contains_key(&namespace_id), Error::<T>::NamespaceAlreadyExists);

        // take fees from the payer
        if let Some(p) = payer {
            let amount = T::FeeNamespace::get();
            Self::take_fees(&p, amount)?;
        }

        // insert namespace in storage
        Namespaces::<T>::insert(&namespace_id, metadata.clone());

        if let Some(ref manager) = caller {
            // insert manager in storage

            Self::do_add_manager(namespace_id.clone(), manager.clone(), None)?;
        }
        let next_namespace_id = namespace_id
            .checked_add(&T::NamespaceId::one())
            .ok_or(Error::<T>::ArithmeticOverflow)?;
        NextNamespaceId::<T>::put(next_namespace_id);
        Self::deposit_event(Event::NamespaceCreated { namespace_id, metadata, creator: caller });

        Ok(())
    }

    /// Deletes a namespace if it has no associated buckets.
    ///
    /// This function ensures that there are no dangling buckets associated with the namespace before deletion.
    /// If buckets are found, it returns the `DanglingBuckets` error. The namespace is then removed from storage,
    /// and a `NamespaceDeleted` event is emitted.
    pub(super) fn do_delete_namespace(namespace_id: T::NamespaceId) -> DispatchResult {
        // check for dangling buckets
        ensure!(!Buckets::<T>::contains_prefix(&namespace_id), Error::<T>::DanglingBuckets);
        ensure!(!Managers::<T>::contains_prefix(&namespace_id), Error::<T>::DanglingManagers);

        // find and remove namespace
        let metadata = Namespaces::<T>::take(&namespace_id).ok_or(Error::<T>::UnknownNamespace)?;

        Self::deposit_event(Event::NamespaceDeleted { namespace_id, metadata });

        Ok(())
    }

    /// Creates a new bucket within the specified namespace.
    ///
    /// This function ensures that the namespace exists before creating the bucket. If a caller is provided,
    /// it verifies that the caller is a manager of the namespace. A unique bucket ID is generated, and if a payer
    /// is specified, the required fees are deducted from their account. The bucket is then stored, and the next
    /// bucket ID is incremented. A `BucketCreated` event is emitted upon successful creation.
    pub(super) fn do_create_bucket(
        namespace_id: T::NamespaceId,
        metadata: BucketMetadataOf<T>,
        caller: Option<SubjectIdOf<T>>,
        payer: Option<AccountIdOf<T>>,
    ) -> Result<T::BucketId, DispatchError> {
        // ensure namespace exists
        ensure!(Namespaces::<T>::contains_key(&namespace_id), Error::<T>::UnknownNamespace);

        if let Some(ref manager) = caller {
            // check if origin is a manager for the namespace
            Self::ensure_is_manager(&namespace_id, manager)?;
        }

        // get bucket id
        let bucket_id = NextBucketId::<T>::get();

        // take fees from the payer
        if let Some(p) = payer {
            let amount = T::FeeBucket::get();
            Self::take_fees(&p, amount)?;
        }

        // get new bucket instance
        let bucket: BucketDetailsOf<T> = Bucket::new(metadata);

        // insert bucket in storage
        Buckets::<T>::insert(&namespace_id, &bucket_id, bucket.clone());

        // increment next bucket id
        let next_bucket_id = bucket_id.checked_add(&One::one()).ok_or(ArithmeticError::Overflow)?;

        NextBucketId::<T>::put(next_bucket_id);

        Self::deposit_event(Event::BucketCreated {
            namespace_id,
            bucket_id: bucket_id.clone(),
            bucket,
            creator: caller,
        });

        Ok(bucket_id)
    }

    /// Deletes a bucket within the specified namespace.
    ///
    /// This function ensures that there are no dangling messages associated with the bucket before deletion.
    /// If messages are found, it returns the `DanglingMessages` error. The bucket is then removed from storage,
    /// and a `BucketDeleted` event is emitted.
    pub(super) fn do_delete_bucket(
        namespace_id: T::NamespaceId,
        bucket_id: T::BucketId,
    ) -> DispatchResult {
        // check for dangling resources
        ensure!(!Messages::<T>::contains_prefix(&bucket_id), Error::<T>::DanglingMessages);
        ensure!(!Admins::<T>::contains_prefix(&bucket_id), Error::<T>::DanglingAdmins);
        ensure!(!Contributors::<T>::contains_prefix(&bucket_id), Error::<T>::DanglingContributors);
        ensure!(!Tags::<T>::contains_prefix(&bucket_id), Error::<T>::DanglingTags);

        // find and remove bucket
        let bucket =
            Buckets::<T>::take(&namespace_id, &bucket_id).ok_or(Error::<T>::UnknownBucket)?;

        Self::deposit_event(Event::BucketDeleted { namespace_id, bucket_id, bucket });

        Ok(())
    }

    /// Deletes a tag within the specified bucket.
    ///
    /// This function ensures that there are no dangling messages associated with the tag before deletion.
    /// If messages are found, it returns the `DanglingMessages` error. The tag is then removed from storage,
    /// and a `TagDeleted` event is emitted.
    pub(super) fn do_delete_tag(bucket_id: T::BucketId, tag: TagOf<T>) -> DispatchResult {
        // check for dangling resources
        ensure!(TagMessages::<T>::get(&bucket_id, &tag).is_zero(), Error::<T>::DanglingMessages);

        // find and remove tag
        Tags::<T>::take(&bucket_id, &tag).ok_or(Error::<T>::UnknownTag)?;

        Self::deposit_event(Event::TagDeleted { bucket_id, tag });

        Ok(())
    }

    /// Adds a new manager to the specified namespace.
    ///
    /// This function ensures that the namespace exists before adding the manager. If a caller is provided,
    /// it verifies that the caller is a manager of the namespace. The new manager is then added to storage.
    /// Depending on whether the operation is performed by a caller or as a force action, the appropriate event
    /// (`ManagerAdded` or `SudoNewManager`) is emitted.
    pub(super) fn do_add_manager(
        namespace_id: T::NamespaceId,
        new_manager: SubjectIdOf<T>,
        caller: Option<SubjectIdOf<T>>,
    ) -> DispatchResult {
        // check if namespace exist
        ensure!(Namespaces::<T>::contains_key(&namespace_id), Error::<T>::UnknownNamespace);

        if let Some(ref manager) = caller {
            // check if origin is a manager for the namespace
            Self::ensure_is_manager(&namespace_id, manager)?;
        }

        // insert manager in storage
        Managers::<T>::insert(&namespace_id, &new_manager, ());

        Self::deposit_event(Event::ManagerAdded { namespace_id, manager: new_manager, caller });

        Ok(())
    }

    /// Removes a manager from the specified namespace.
    ///
    /// This function ensures that the namespace exists before attempting to remove the manager.
    /// If a caller is provided, it verifies that the caller is a manager of the namespace.
    /// The specified manager is then removed from the set of managers for the namespace,
    /// and a `ManagerRemoved` event is emitted.
    pub(super) fn do_remove_manager(
        namespace_id: T::NamespaceId,
        manager: SubjectIdOf<T>,
        caller: Option<SubjectIdOf<T>>,
    ) -> DispatchResult {
        // check if namespace exist
        ensure!(Namespaces::<T>::contains_key(&namespace_id), Error::<T>::UnknownNamespace);

        if let Some(ref manager) = caller {
            // check if origin is a manager for the namespace
            Self::ensure_is_manager(&namespace_id, manager)?;
        }
        let mut manager_count: u32 = 0;
        for _ in Managers::<T>::iter_prefix(&namespace_id) {
            manager_count.saturating_accrue(1);
            if manager_count > 1 {
                break;
            }
        }
        ensure!(manager_count > 1, Error::<T>::LastManagerRemoval);

        // drop manager from set
        Managers::<T>::remove(&namespace_id, &manager);

        Self::deposit_event(Event::ManagerRemoved { namespace_id, manager, caller });

        Ok(())
    }

    /// Adds a new admin to the specified bucket within a namespace.
    ///
    /// This function ensures that the bucket exists before adding the admin. If a caller is provided,
    /// it verifies that the caller is a manager of the namespace. The new admin is then added to the
    /// storage for the specified bucket, and an `AdminAdded` event is emitted.
    pub(super) fn do_add_admin(
        namespace_id: T::NamespaceId,
        bucket_id: T::BucketId,
        new_admin: SubjectIdOf<T>,
        caller: Option<SubjectIdOf<T>>,
    ) -> DispatchResult {
        // check if bucket exist
        ensure!(Buckets::<T>::contains_key(&namespace_id, &bucket_id), Error::<T>::UnknownBucket);

        if let Some(ref manager) = caller {
            // check if origin is a manager for the namespace
            Self::ensure_is_manager(&namespace_id, manager)?;
        }

        // insert admin in storage
        Admins::<T>::insert(&bucket_id, &new_admin, ());

        Self::deposit_event(Event::AdminAdded {
            namespace_id,
            bucket_id,
            admin: new_admin,
            caller,
        });

        Ok(())
    }

    /// Removes an admin from the specified bucket within a namespace.
    ///
    /// This function ensures that the bucket exists before attempting to remove the admin.
    /// If a caller is provided, it verifies that the caller is a manager of the namespace.
    /// The specified admin is then removed from the storage for the bucket, and an `AdminRemoved`
    /// event is emitted.
    pub(super) fn do_remove_admin(
        namespace_id: T::NamespaceId,
        bucket_id: T::BucketId,
        admin: SubjectIdOf<T>,
        caller: Option<SubjectIdOf<T>>,
    ) -> DispatchResult {
        // check if bucket exist
        ensure!(Buckets::<T>::contains_key(&namespace_id, &bucket_id), Error::<T>::UnknownBucket);

        if let Some(ref manager) = caller {
            // check if origin is a manager for the namespace
            Self::ensure_is_manager(&namespace_id, manager)?;
        }

        // remove admin from storage
        Admins::<T>::remove(&bucket_id, &admin);

        Self::deposit_event(Event::AdminRemoved { namespace_id, bucket_id, admin, caller });

        Ok(())
    }

    /// Adds a new contributor to the specified bucket within a namespace.
    ///
    /// This function ensures that the bucket exists before adding the contributor.
    /// If a caller is provided, it verifies that the caller is a manager of the namespace.
    /// The new contributor is then added to the storage for the specified bucket,
    /// and a `ContributorAdded` event is emitted.
    pub(super) fn do_add_contributor(
        namespace_id: T::NamespaceId,
        bucket_id: T::BucketId,
        contributor: SubjectIdOf<T>,
        caller: Option<SubjectIdOf<T>>,
    ) -> DispatchResult {
        // check if bucket exist
        ensure!(Buckets::<T>::contains_key(&namespace_id, &bucket_id), Error::<T>::UnknownBucket);

        if let Some(ref admin) = caller {
            // check if origin is a manager for the namespace
            Self::ensure_is_admin(&bucket_id, admin)?;
        }

        // insert contributor in storage
        Contributors::<T>::insert(&bucket_id, &contributor, ());

        Self::deposit_event(Event::ContributorAdded {
            namespace_id,
            bucket_id,
            contributor,
            caller,
        });

        Ok(())
    }

    /// Removes a contributor from the specified bucket within a namespace.
    ///
    /// This function ensures that the bucket exists before attempting to remove the contributor.
    /// If a caller is provided, it verifies that the caller is a manager of the namespace.
    /// The specified contributor is then removed from the storage for the bucket,
    /// and a `ContributorRemoved` event is emitted.
    pub(super) fn do_remove_contributor(
        namespace_id: T::NamespaceId,
        bucket_id: T::BucketId,
        contributor: SubjectIdOf<T>,
        caller: Option<SubjectIdOf<T>>,
    ) -> DispatchResult {
        ensure!(Buckets::<T>::contains_key(&namespace_id, &bucket_id), Error::<T>::UnknownBucket);

        if let Some(ref admin) = caller {
            Self::ensure_is_admin(&bucket_id, admin)?;
        }

        // drop contributor
        Contributors::<T>::remove(&bucket_id, &contributor);

        Self::deposit_event(Event::ContributorRemoved {
            namespace_id,
            bucket_id,
            contributor,
            caller,
        });

        Ok(())
    }

    /// Locks a bucket within the specified namespace.
    ///
    /// This function ensures that the caller, if provided, is an admin for the bucket.
    /// It then locks the bucket by mutating its state in storage. If the bucket does not exist,
    /// the function returns an `UnknownBucket` error. A `PausedBucket` event is emitted upon successful locking.
    pub(super) fn do_lock_bucket(
        namespace_id: T::NamespaceId,
        bucket_id: T::BucketId,
        caller: Option<SubjectIdOf<T>>,
    ) -> DispatchResult {
        if let Some(ref admin) = caller {
            // check if origin is an admin for the bucket
            Self::ensure_is_admin(&bucket_id, admin)?;
        };

        let bucket = Buckets::<T>::try_mutate(
            &namespace_id,
            &bucket_id,
            |bucket| -> Result<BucketDetailsOf<T>, DispatchError> {
                let Some(bucket) = bucket.as_mut() else {
                    return Err(Error::<T>::UnknownBucket.into());
                };

                bucket.lock();

                Ok(bucket.clone())
            },
        )?;

        Self::deposit_event(Event::PausedBucket { namespace_id, bucket_id, bucket, caller });

        Ok(())
    }

    /// Sets a new encryption key for the specified bucket within a namespace.
    ///
    /// This function ensures that the caller, if provided, is an admin for the bucket.
    /// It then updates the encryption key for the bucket, provided the bucket is writable
    /// or the `allow_locked` flag is set to `true`. If the bucket does not exist, the function
    /// returns an `UnknownBucket` error. A `BucketWritableWithKey` event is emitted upon successfully
    /// updating the encryption key.
    pub(super) fn do_set_key(
        namespace_id: T::NamespaceId,
        bucket_id: T::BucketId,
        new_encryption_key: T::KeyId,
        allow_locked: bool,
        caller: Option<SubjectIdOf<T>>,
    ) -> DispatchResult {
        if let Some(ref admin) = caller {
            // check if origin is an admin for the bucket
            Self::ensure_is_admin(&bucket_id, admin)?;
        };

        let bucket = Buckets::<T>::try_mutate(
            &namespace_id,
            &bucket_id,
            |bucket| -> Result<BucketDetailsOf<T>, DispatchError> {
                let bucket = bucket.as_mut().ok_or(Error::<T>::UnknownBucket)?;

                ensure!(allow_locked || bucket.is_writable(), Error::<T>::BucketIsLocked);

                bucket.set_writable(new_encryption_key.clone());

                Ok(bucket.clone())
            },
        )?;

        Self::deposit_event(Event::BucketWritableWithKey {
            namespace_id,
            bucket_id,
            new_encryption_key,
            bucket,
            caller,
        });

        Ok(())
    }

    /// Creates a new tag for the specified bucket.
    ///
    /// This function ensures that the caller, if provided, is an admin for the bucket.
    /// If a payer is specified, the required fees are deducted from their account.
    /// The tag is then added to the storage for the specified bucket, and a `NewTag`
    /// event is emitted upon successful creation.
    pub(super) fn do_create_tag(
        bucket_id: T::BucketId,
        tag: TagOf<T>,
        caller: Option<SubjectIdOf<T>>,
        payer: Option<AccountIdOf<T>>,
    ) -> DispatchResult {
        if let Some(ref admin) = caller {
            // check if origin is an admin for the bucket
            Self::ensure_is_admin(&bucket_id, admin)?;
        };

        // take fees from the payer
        if let Some(p) = payer {
            let amount = T::FeeTag::get();
            Self::take_fees(&p, amount)?;
        }

        // insert tag in storage
        Tags::<T>::insert(&bucket_id, &tag, ());

        Self::deposit_event(Event::NewTag { bucket_id, tag, creator: caller });

        Ok(())
    }

    /// Creates a new message within the specified bucket of a namespace.
    ///
    /// This function ensures that the bucket exists and is writable before creating the message.
    /// It verifies that the contributor is authorized to add messages to the bucket and checks
    /// if the provided tag (if any) exists in the bucket. If a payer is specified, the required
    /// fees are deducted from their account. The message is then added to storage, and the bucket's
    /// message ID is incremented. A `NewMessage` event is emitted upon successful creation.
    pub(super) fn do_create_message(
        namespace_id: T::NamespaceId,
        bucket_id: T::BucketId,
        message_details: Message<ReferenceOf<T>, TagOf<T>, MessageMetadataOf<T>>,
        contributor: SubjectIdOf<T>,
        payer: Option<AccountIdOf<T>>,
    ) -> Result<T::MessageId, DispatchError> {
        // fetch bucket
        let mut bucket =
            Buckets::<T>::get(&namespace_id, &bucket_id).ok_or(Error::<T>::UnknownBucket)?;

        // check if bucket is writable
        ensure!(bucket.is_writable(), Error::<T>::BucketIsLocked);

        // check if subject is contributor
        Self::ensure_is_contributor(&bucket_id, &contributor)?;

        if let Some(tag) = &message_details.tag {
            ensure!(Tags::<T>::contains_key(&bucket_id, tag), Error::<T>::UnknownTag);
            // increment message counter
            TagMessages::<T>::try_mutate(&bucket_id, tag, |count| -> DispatchResult {
                *count = count.checked_add(1).ok_or(Error::<T>::ArithmeticOverflow)?;
                Ok(())
            })?;
        }

        // take fees from the payer
        if let Some(p) = payer {
            let amount = T::FeeMessage::get();
            Self::take_fees(&p, amount)?;
        }

        // create message
        let message_id = bucket.get_next_message_id();

        // insert message in storage
        Messages::<T>::insert(&bucket_id, &message_id, &message_details);
        // increment message id
        bucket.increment_message_id()?;
        Buckets::<T>::insert(&namespace_id, &bucket_id, bucket);

        Self::deposit_event(Event::NewMessage {
            namespace_id,
            bucket_id,
            message_id: message_id.clone(),
            message: message_details,
            contributor,
        });

        Ok(message_id)
    }

    /// Removes a message from the specified bucket.
    ///
    /// This function ensures that the message exists before attempting to remove it.
    /// If the message does not exist, the function returns an `UnknownMessage` error.
    /// Upon successful removal, a `MessageDeleted` event is emitted.
    pub(super) fn do_remove_message(
        bucket_id: T::BucketId,
        message_id: T::MessageId,
    ) -> DispatchResult {
        // check if message exist & remove it
        let message_details =
            Messages::<T>::take(&bucket_id, &message_id).ok_or(Error::<T>::UnknownMessage)?;

        if let Some(tag) = &message_details.tag {
            // decrement message counter
            TagMessages::<T>::try_mutate(&bucket_id, tag, |count| -> DispatchResult {
                *count = count.checked_sub(1).ok_or(Error::<T>::ArithmeticUnderflow)?;
                Ok(())
            })?;
        }

        Self::deposit_event(Event::MessageDeleted {
            bucket_id,
            message_id,
            message: message_details,
        });

        Ok(())
    }

    /// Take fees from the payer and deposit them to the fee collector.
    fn take_fees(payer: &AccountIdOf<T>, value: BalanceOf<T>) -> DispatchResult {
        // Collect the fees.
        let imbalance = T::Currency::withdraw(
            payer,
            value,
            Precision::Exact,
            Preservation::Protect,
            Fortitude::Polite,
        )
        .map_err(|_| Error::<T>::UnableToPayFees)?;

        // Deposit the fees to the fee collector.
        T::FeeCollector::on_unbalanced(imbalance);
        Ok(())
    }
}
