use frame_support::{ensure, sp_runtime::TryRuntimeError};

use crate::{
    Admins, Buckets, Config, Contributors, Managers, Messages, Namespaces, NextBucketId, Tags,
};

pub(crate) fn do_try_state<T: Config>() -> Result<(), TryRuntimeError> {
    let mut known_bucket_ids = Vec::new();
    let next_bucket_id = NextBucketId::<T>::get();

    // Each bucket should have a valid namespace.
    Buckets::<T>::iter().try_for_each(|(namespace_id, bucket_id, details)| {
        ensure!(
            Namespaces::<T>::contains_key(&namespace_id),
            TryRuntimeError::Other("Namespace not found for bucket")
        );

        // next bucket id should be greater than existing bucket ids
        ensure!(next_bucket_id >= bucket_id, TryRuntimeError::Other("Bucket Id should not exist"));

        // next message id should not exist
        let next_message_id = details.next_message_id;
        ensure!(
            !Messages::<T>::contains_key(&bucket_id, &next_message_id),
            TryRuntimeError::Other("Next message id in bucket should not exist")
        );

        known_bucket_ids.push(bucket_id);

        Ok::<(), TryRuntimeError>(())
    })?;

    // each manager should have a valid namespace
    Managers::<T>::iter().try_for_each(|(namespace_id, _, _)| {
        ensure!(
            Namespaces::<T>::contains_key(&namespace_id),
            TryRuntimeError::Other("Manager of unknown namespace")
        );
        Ok::<(), TryRuntimeError>(())
    })?;

    // each admin should have a valid bucket
    Admins::<T>::iter().try_for_each(|(bucket_id, _, _)| {
        ensure!(
            known_bucket_ids.contains(&bucket_id),
            TryRuntimeError::Other("Admin of unknown bucket")
        );
        Ok::<(), TryRuntimeError>(())
    })?;

    // each contributor should have a valid bucket
    Contributors::<T>::iter().try_for_each(|(bucket_id, _, _)| {
        ensure!(
            known_bucket_ids.contains(&bucket_id),
            TryRuntimeError::Other("Contributor of unknown bucket")
        );
        Ok::<(), TryRuntimeError>(())
    })?;

    // each tag should have a valid bucket
    Tags::<T>::iter().try_for_each(|(bucket_id, _, _)| {
        ensure!(
            known_bucket_ids.contains(&bucket_id),
            TryRuntimeError::Other("Tag of unknown bucket")
        );
        Ok::<(), TryRuntimeError>(())
    })?;

    // each message should have a valid bucket
    Messages::<T>::iter().try_for_each(|(bucket_id, _, _)| {
        ensure!(
            known_bucket_ids.contains(&bucket_id),
            TryRuntimeError::Other("Message of unknown bucket")
        );
        Ok::<(), TryRuntimeError>(())
    })
}
