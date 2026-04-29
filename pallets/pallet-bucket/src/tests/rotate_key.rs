use frame_support::{assert_noop, assert_ok};
use frame_system::RawOrigin;

use crate::{mock::*, types::Status, Error};

#[test]
fn rotate_key() {
    ExtBuilder::default()
        .add_namespace(DEFAULT_NAMESPACE_ID, MetadataMock { unique_plus_1: 10 })
        .add_bucket(DEFAULT_NAMESPACE_ID, DEFAULT_BUCKET_ID, BUCKET_EXAMPLE_UNLOCKED)
        .add_admin(DEFAULT_BUCKET_ID, ACCOUNT_00)
        .build_and_execute_with_sanity_tests(|| {
            let origin = RawOrigin::Signed(ACCOUNT_00);
            assert_ok!(Buckets::rotate_key(
                origin.clone().into(),
                DEFAULT_NAMESPACE_ID,
                DEFAULT_BUCKET_ID,
                DEFAULT_ENCRYPTION_KEY
            ));
            let bucket = Buckets::bucket_with_id(DEFAULT_NAMESPACE_ID, DEFAULT_BUCKET_ID)
                .expect("Bucket should exist");
            assert!(matches!(bucket.status, Status::Writable(1)));

            assert_eq!(events().len(), 1);
            assert!(events().contains(&crate::Event::BucketWritableWithKey {
                namespace_id: DEFAULT_NAMESPACE_ID,
                bucket_id: DEFAULT_BUCKET_ID,
                new_encryption_key: DEFAULT_ENCRYPTION_KEY,
                bucket: BucketMock {
                    metadata: MetadataMock { unique_plus_1: 10 },
                    status: Status::Writable(DEFAULT_ENCRYPTION_KEY),
                    next_message_id: 2,
                },
                caller: Some(ACCOUNT_00)
            }));
        });
}

#[test]
fn rotate_key_bucket_is_locked() {
    ExtBuilder::default()
        .add_namespace(DEFAULT_NAMESPACE_ID, MetadataMock { unique_plus_1: 10 })
        .add_bucket(DEFAULT_NAMESPACE_ID, DEFAULT_BUCKET_ID, BUCKET_EXAMPLE_LOCKED)
        .add_admin(DEFAULT_BUCKET_ID, ACCOUNT_00)
        .build_and_execute_with_sanity_tests(|| {
            let origin = RawOrigin::Signed(ACCOUNT_00);
            assert_noop!(
                Buckets::rotate_key(
                    origin.clone().into(),
                    DEFAULT_NAMESPACE_ID,
                    DEFAULT_BUCKET_ID,
                    DEFAULT_ENCRYPTION_KEY
                ),
                Error::<Test>::BucketIsLocked
            );
            assert_eq!(events().len(), 0);
        });
}

#[test]
fn rotate_key_not_admin() {
    ExtBuilder::default()
        .add_namespace(DEFAULT_NAMESPACE_ID, MetadataMock { unique_plus_1: 10 })
        .add_bucket(DEFAULT_NAMESPACE_ID, DEFAULT_BUCKET_ID, BUCKET_EXAMPLE_LOCKED)
        .add_manager(DEFAULT_NAMESPACE_ID, ACCOUNT_00)
        .add_contributor(DEFAULT_BUCKET_ID, ACCOUNT_00)
        .build_and_execute_with_sanity_tests(|| {
            let origin = RawOrigin::Signed(ACCOUNT_00);
            assert_noop!(
                Buckets::rotate_key(
                    origin.clone().into(),
                    DEFAULT_NAMESPACE_ID,
                    DEFAULT_BUCKET_ID,
                    DEFAULT_ENCRYPTION_KEY
                ),
                Error::<Test>::NotAdmin
            );
            assert_eq!(events().len(), 0);
        });
}
