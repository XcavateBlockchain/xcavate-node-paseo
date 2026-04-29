use frame_support::{assert_err, assert_noop, assert_ok, sp_runtime::DispatchError};
use frame_system::RawOrigin;

use crate::types::Status;
use crate::{mock::*, Error};

#[test]
fn create_bucket() {
    ExtBuilder::default()
        .with_balances(vec![(ACCOUNT_00, DEFAULT_BALANCE)])
        .add_namespace(DEFAULT_NAMESPACE_ID, MetadataMock { unique_plus_1: 10 })
        .add_manager(DEFAULT_NAMESPACE_ID, ACCOUNT_00)
        .build_and_execute_with_sanity_tests(|| {
            let origin = RawOrigin::Signed(ACCOUNT_00);

            let bucket_id = Buckets::next_bucket_id();
            assert_eq!(bucket_id, 0);
            assert_ok!(Buckets::create_bucket(
                origin.into(),
                DEFAULT_NAMESPACE_ID,
                MetadataInputMock { unique: 20 }
            ));
            let bucket = Buckets::bucket_with_id(DEFAULT_NAMESPACE_ID, bucket_id)
                .expect("Bucket should exist");
            assert!(bucket.is_locked());
            assert_eq!(bucket.get_next_message_id(), 0);
            assert_eq!(bucket.metadata.unique_plus_1, 21);
            let balance = Balances::free_balance(ACCOUNT_00);
            assert_eq!(DEFAULT_BALANCE - StorageFee::get(), balance);
            assert_eq!(Buckets::next_bucket_id(), bucket_id + 1);

            assert!(events().contains(&crate::Event::BucketCreated {
                namespace_id: DEFAULT_NAMESPACE_ID,
                bucket_id: 0,
                bucket: BucketMock {
                    metadata: MetadataMock { unique_plus_1: 21 },
                    status: Status::Locked,
                    next_message_id: 0,
                },
                creator: Some(ACCOUNT_00)
            }));
            assert_eq!(events().len(), 1);
        });
}

#[test]
fn create_bucket_insufficient_balance() {
    ExtBuilder::default()
        .with_balances(vec![(ACCOUNT_00, StorageFee::get())])
        .add_namespace(DEFAULT_NAMESPACE_ID, MetadataMock { unique_plus_1: 10 })
        .add_manager(DEFAULT_NAMESPACE_ID, ACCOUNT_00)
        .build_and_execute_with_sanity_tests(|| {
            let origin = RawOrigin::Signed(ACCOUNT_00);

            assert_noop!(
                Buckets::create_bucket(
                    origin.into(),
                    DEFAULT_NAMESPACE_ID,
                    MetadataInputMock { unique: 20 }
                ),
                Error::<Test>::UnableToPayFees
            );
            assert_eq!(events().len(), 0);
        });
}

#[test]
fn create_bucket_not_manager() {
    ExtBuilder::default()
        .add_namespace(DEFAULT_NAMESPACE_ID, MetadataMock { unique_plus_1: 10 })
        .build_and_execute_with_sanity_tests(|| {
            let origin = RawOrigin::Signed(ACCOUNT_00);
            assert_noop!(
                Buckets::create_bucket(
                    origin.into(),
                    DEFAULT_NAMESPACE_ID,
                    MetadataInputMock { unique: 20 }
                ),
                Error::<Test>::NotManager
            );
            assert_eq!(events().len(), 0);
        });
}

#[test]
fn remove_bucket_bad_origin() {
    ExtBuilder::default()
        .add_namespace(DEFAULT_NAMESPACE_ID, MetadataMock { unique_plus_1: 10 })
        .add_manager(DEFAULT_NAMESPACE_ID, ACCOUNT_00)
        .add_bucket(DEFAULT_NAMESPACE_ID, DEFAULT_BUCKET_ID, BUCKET_EXAMPLE_LOCKED)
        .build_and_execute_with_sanity_tests(|| {
            let origin = RawOrigin::Signed(ACCOUNT_00);
            assert_noop!(
                Buckets::force_remove_bucket(
                    origin.into(),
                    DEFAULT_NAMESPACE_ID,
                    DEFAULT_BUCKET_ID
                ),
                DispatchError::BadOrigin
            );
            assert_eq!(events().len(), 0);
        });
}

#[test]
fn remove_bucket() {
    ExtBuilder::default()
        .add_namespace(DEFAULT_NAMESPACE_ID, MetadataMock { unique_plus_1: 10 })
        .add_manager(DEFAULT_NAMESPACE_ID, ACCOUNT_00)
        .add_bucket(DEFAULT_NAMESPACE_ID, DEFAULT_BUCKET_ID, BUCKET_EXAMPLE_LOCKED)
        .build_and_execute_with_sanity_tests(|| {
            let origin = RawOrigin::Root;
            assert_ok!(Buckets::force_remove_bucket(
                origin.into(),
                DEFAULT_NAMESPACE_ID,
                DEFAULT_BUCKET_ID
            ));
            assert!(Buckets::bucket_with_id(DEFAULT_NAMESPACE_ID, DEFAULT_BUCKET_ID).is_none());

            assert!(events().contains(&crate::Event::BucketDeleted {
                namespace_id: DEFAULT_NAMESPACE_ID,
                bucket_id: DEFAULT_BUCKET_ID,
                bucket: BUCKET_EXAMPLE_LOCKED,
            }));
            assert_eq!(events().len(), 1);
        });
}

#[test]
fn remove_bucket_dangling_contributor() {
    ExtBuilder::default()
        .add_namespace(DEFAULT_NAMESPACE_ID, MetadataMock { unique_plus_1: 10 })
        .add_manager(DEFAULT_NAMESPACE_ID, ACCOUNT_00)
        .add_bucket(DEFAULT_NAMESPACE_ID, DEFAULT_BUCKET_ID, BUCKET_EXAMPLE_LOCKED)
        .add_contributor(DEFAULT_BUCKET_ID, ACCOUNT_01)
        .build_and_execute_with_sanity_tests(|| {
            let origin = RawOrigin::Root;
            assert_err!(
                Buckets::force_remove_bucket(
                    origin.into(),
                    DEFAULT_NAMESPACE_ID,
                    DEFAULT_BUCKET_ID
                ),
                Error::<Test>::DanglingContributors
            );

            assert_eq!(events().len(), 0);
        });
}

#[test]
fn remove_bucket_dangling_admin() {
    ExtBuilder::default()
        .add_namespace(DEFAULT_NAMESPACE_ID, MetadataMock { unique_plus_1: 10 })
        .add_manager(DEFAULT_NAMESPACE_ID, ACCOUNT_00)
        .add_bucket(DEFAULT_NAMESPACE_ID, DEFAULT_BUCKET_ID, BUCKET_EXAMPLE_LOCKED)
        .add_admin(DEFAULT_BUCKET_ID, ACCOUNT_01)
        .build_and_execute_with_sanity_tests(|| {
            let origin = RawOrigin::Root;
            assert_err!(
                Buckets::force_remove_bucket(
                    origin.into(),
                    DEFAULT_NAMESPACE_ID,
                    DEFAULT_BUCKET_ID
                ),
                Error::<Test>::DanglingAdmins
            );
            assert_eq!(events().len(), 0);
        });
}

#[test]
fn remove_bucket_dangling_tags() {
    ExtBuilder::default()
        .with_balances(vec![(ACCOUNT_01, DEFAULT_BALANCE)])
        .add_namespace(DEFAULT_NAMESPACE_ID, MetadataMock { unique_plus_1: 10 })
        .add_manager(DEFAULT_NAMESPACE_ID, ACCOUNT_00)
        .add_bucket(DEFAULT_NAMESPACE_ID, DEFAULT_BUCKET_ID, BUCKET_EXAMPLE_LOCKED)
        .add_admin(DEFAULT_BUCKET_ID, ACCOUNT_01)
        .build_and_execute_with_sanity_tests(|| {
            let origin = RawOrigin::Root;
            let manager_origin = RawOrigin::Signed(ACCOUNT_00);
            let admin_origin = RawOrigin::Signed(ACCOUNT_01);
            let tag = create_bounded_vec_example(0);
            assert_ok!(Buckets::create_tag(admin_origin.into(), DEFAULT_BUCKET_ID, tag.clone()));
            assert_ok!(Buckets::remove_admin(
                manager_origin.into(),
                DEFAULT_NAMESPACE_ID,
                DEFAULT_BUCKET_ID,
                ACCOUNT_01
            ));
            assert_err!(
                Buckets::force_remove_bucket(
                    origin.into(),
                    DEFAULT_NAMESPACE_ID,
                    DEFAULT_BUCKET_ID
                ),
                Error::<Test>::DanglingTags
            );
        });
}
