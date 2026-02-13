use frame_support::assert_ok;

use crate::{mock::*, traits::BucketManager, types::Status};

#[test]
fn set_writable() {
    ExtBuilder::default()
        .add_namespace(DEFAULT_NAMESPACE_ID, MetadataMock { unique_plus_1: 10 })
        .add_bucket(DEFAULT_NAMESPACE_ID, DEFAULT_BUCKET_ID, BUCKET_EXAMPLE_LOCKED)
        .add_admin(DEFAULT_BUCKET_ID, ACCOUNT_00)
        .build_and_execute_with_sanity_tests(|| {
            assert_ok!(<Buckets as BucketManager<Test>>::set_writable(
                DEFAULT_NAMESPACE_ID,
                DEFAULT_BUCKET_ID,
                DEFAULT_ENCRYPTION_KEY
            ));
            let bucket = Buckets::bucket_with_id(DEFAULT_NAMESPACE_ID, DEFAULT_BUCKET_ID)
                .expect("Bucket should exist");
            assert!(matches!(bucket.status, Status::Writable(1)));
        });
}

#[test]
fn set_locked() {
    ExtBuilder::default()
        .add_namespace(DEFAULT_NAMESPACE_ID, MetadataMock { unique_plus_1: 10 })
        .add_bucket(DEFAULT_NAMESPACE_ID, DEFAULT_BUCKET_ID, BUCKET_EXAMPLE_UNLOCKED)
        .add_admin(DEFAULT_BUCKET_ID, ACCOUNT_00)
        .build_and_execute_with_sanity_tests(|| {
            assert_ok!(<Buckets as BucketManager<Test>>::set_locked(
                DEFAULT_NAMESPACE_ID,
                DEFAULT_BUCKET_ID
            ));
            let bucket = Buckets::bucket_with_id(DEFAULT_NAMESPACE_ID, DEFAULT_BUCKET_ID)
                .expect("Bucket should exist");
            assert!(matches!(bucket.status, Status::Locked));
        });
}
