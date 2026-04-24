use frame_support::{assert_noop, assert_ok};
use frame_system::RawOrigin;

use crate::{mock::*, types::Status, Error};

#[test]
fn pause_writing() {
    ExtBuilder::default()
        .add_namespace(DEFAULT_NAMESPACE_ID, MetadataMock { unique_plus_1: 10 })
        .add_bucket(DEFAULT_NAMESPACE_ID, DEFAULT_BUCKET_ID, BUCKET_EXAMPLE_UNLOCKED)
        .add_admin(DEFAULT_BUCKET_ID, ACCOUNT_00)
        .build_and_execute_with_sanity_tests(|| {
            let origin = RawOrigin::Signed(ACCOUNT_00);
            assert_ok!(Buckets::pause_writing(
                origin.clone().into(),
                DEFAULT_NAMESPACE_ID,
                DEFAULT_BUCKET_ID
            ));
            let bucket = Buckets::bucket_with_id(DEFAULT_NAMESPACE_ID, DEFAULT_BUCKET_ID)
                .expect("Bucket should exist");
            assert!(matches!(bucket.status, Status::Locked));

            assert!(events().contains(&crate::Event::PausedBucket {
                namespace_id: DEFAULT_NAMESPACE_ID,
                bucket_id: DEFAULT_BUCKET_ID,
                bucket: BucketMock {
                    metadata: MetadataMock { unique_plus_1: 10 },
                    status: Status::Locked,
                    next_message_id: 2,
                },
                caller: Some(ACCOUNT_00)
            }));
            assert_eq!(events().len(), 1);
        });
}

#[test]
fn pause_writing_not_admin() {
    ExtBuilder::default()
        .add_namespace(DEFAULT_NAMESPACE_ID, MetadataMock { unique_plus_1: 10 })
        .add_bucket(DEFAULT_NAMESPACE_ID, DEFAULT_BUCKET_ID, BUCKET_EXAMPLE_UNLOCKED)
        .add_contributor(DEFAULT_BUCKET_ID, ACCOUNT_00)
        .add_manager(DEFAULT_NAMESPACE_ID, ACCOUNT_00)
        .build_and_execute_with_sanity_tests(|| {
            let origin = RawOrigin::Signed(ACCOUNT_00);
            assert_noop!(
                Buckets::pause_writing(
                    origin.clone().into(),
                    DEFAULT_NAMESPACE_ID,
                    DEFAULT_BUCKET_ID
                ),
                Error::<Test>::NotAdmin
            );
            assert_eq!(events().len(), 0);
        });
}
