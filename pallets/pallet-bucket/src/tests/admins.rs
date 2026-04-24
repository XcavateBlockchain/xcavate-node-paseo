use frame_support::{assert_noop, assert_ok};
use frame_system::RawOrigin;

use crate::{mock::*, Admins, Error};

#[test]
fn add_admin_through_manager() {
    ExtBuilder::default()
        .add_namespace(DEFAULT_NAMESPACE_ID, MetadataMock { unique_plus_1: 10 })
        .add_bucket(DEFAULT_NAMESPACE_ID, DEFAULT_BUCKET_ID, BUCKET_EXAMPLE_LOCKED)
        .add_manager(DEFAULT_NAMESPACE_ID, ACCOUNT_00)
        .build_and_execute_with_sanity_tests(|| {
            let origin = RawOrigin::Signed(ACCOUNT_00);
            assert_ok!(Buckets::add_admin(
                origin.into(),
                DEFAULT_NAMESPACE_ID,
                DEFAULT_BUCKET_ID,
                ACCOUNT_01
            ));
            assert!(Admins::<Test>::get(DEFAULT_BUCKET_ID, ACCOUNT_01).is_some());
            assert!(events().contains(&crate::Event::AdminAdded {
                namespace_id: DEFAULT_NAMESPACE_ID,
                bucket_id: DEFAULT_BUCKET_ID,
                admin: ACCOUNT_01,
                caller: Some(ACCOUNT_00)
            }));
            assert_eq!(events().len(), 1);
        });
}

#[test]
fn add_admin_through_admin() {
    ExtBuilder::default()
        .add_namespace(DEFAULT_NAMESPACE_ID, MetadataMock { unique_plus_1: 10 })
        .add_bucket(DEFAULT_NAMESPACE_ID, DEFAULT_BUCKET_ID, BUCKET_EXAMPLE_LOCKED)
        .add_admin(DEFAULT_BUCKET_ID, ACCOUNT_00)
        .build_and_execute_with_sanity_tests(|| {
            let origin = RawOrigin::Signed(ACCOUNT_00);
            assert_noop!(
                Buckets::add_admin(
                    origin.into(),
                    DEFAULT_NAMESPACE_ID,
                    DEFAULT_BUCKET_ID,
                    ACCOUNT_01
                ),
                Error::<Test>::NotManager
            );
            assert_eq!(events().len(), 0);
        });
}

#[test]
fn add_admin_not_authorized() {
    ExtBuilder::default()
        .add_namespace(DEFAULT_NAMESPACE_ID, MetadataMock { unique_plus_1: 10 })
        .add_bucket(DEFAULT_NAMESPACE_ID, DEFAULT_BUCKET_ID, BUCKET_EXAMPLE_LOCKED)
        .add_contributor(DEFAULT_BUCKET_ID, ACCOUNT_00)
        .build_and_execute_with_sanity_tests(|| {
            let origin = RawOrigin::Signed(ACCOUNT_00);
            assert_noop!(
                Buckets::add_admin(
                    origin.into(),
                    DEFAULT_NAMESPACE_ID,
                    DEFAULT_BUCKET_ID,
                    ACCOUNT_01
                ),
                Error::<Test>::NotManager
            );
            assert_eq!(events().len(), 0);
        });
}

#[test]
fn add_admin_invalid_namespace() {
    ExtBuilder::default()
        .add_namespace(DEFAULT_NAMESPACE_ID, MetadataMock { unique_plus_1: 10 })
        .add_bucket(DEFAULT_NAMESPACE_ID, DEFAULT_BUCKET_ID, BUCKET_EXAMPLE_LOCKED)
        .add_manager(DEFAULT_NAMESPACE_ID, ACCOUNT_00)
        .build_and_execute_with_sanity_tests(|| {
            let origin = RawOrigin::Signed(ACCOUNT_00);
            assert_noop!(
                Buckets::add_admin(
                    origin.into(),
                    DEFAULT_NAMESPACE_ID + 1,
                    DEFAULT_BUCKET_ID,
                    ACCOUNT_01
                ),
                Error::<Test>::UnknownBucket
            );
            assert_eq!(events().len(), 0);
        });
}

#[test]
fn add_admin_invalid_bucket() {
    ExtBuilder::default()
        .add_namespace(DEFAULT_NAMESPACE_ID, MetadataMock { unique_plus_1: 10 })
        .add_bucket(DEFAULT_NAMESPACE_ID, DEFAULT_BUCKET_ID, BUCKET_EXAMPLE_LOCKED)
        .add_manager(DEFAULT_NAMESPACE_ID, ACCOUNT_00)
        .build_and_execute_with_sanity_tests(|| {
            let origin = RawOrigin::Signed(ACCOUNT_00);
            assert_noop!(
                Buckets::add_admin(
                    origin.into(),
                    DEFAULT_NAMESPACE_ID,
                    DEFAULT_BUCKET_ID + 1,
                    ACCOUNT_01
                ),
                Error::<Test>::UnknownBucket
            );
            assert_eq!(events().len(), 0);
        });
}

#[test]
fn remove_admin() {
    ExtBuilder::default()
        .add_namespace(DEFAULT_NAMESPACE_ID, MetadataMock { unique_plus_1: 10 })
        .add_bucket(DEFAULT_NAMESPACE_ID, DEFAULT_BUCKET_ID, BUCKET_EXAMPLE_LOCKED)
        .add_manager(DEFAULT_NAMESPACE_ID, ACCOUNT_00)
        .add_admin(DEFAULT_BUCKET_ID, ACCOUNT_01)
        .build_and_execute_with_sanity_tests(|| {
            let origin = RawOrigin::Signed(ACCOUNT_00);
            assert!(Admins::<Test>::get(DEFAULT_BUCKET_ID, ACCOUNT_01).is_some());
            assert_ok!(Buckets::remove_admin(
                origin.into(),
                DEFAULT_NAMESPACE_ID,
                DEFAULT_BUCKET_ID,
                ACCOUNT_01
            ));
            assert!(Admins::<Test>::get(DEFAULT_BUCKET_ID, ACCOUNT_01).is_none());

            assert!(events().contains(&crate::Event::AdminRemoved {
                namespace_id: DEFAULT_NAMESPACE_ID,
                bucket_id: DEFAULT_BUCKET_ID,
                admin: ACCOUNT_01,
                caller: Some(ACCOUNT_00)
            }));
            assert_eq!(events().len(), 1);
        });
}

#[test]
fn remove_admin_unauthorized() {
    ExtBuilder::default()
        .add_namespace(DEFAULT_NAMESPACE_ID, MetadataMock { unique_plus_1: 10 })
        .add_bucket(DEFAULT_NAMESPACE_ID, DEFAULT_BUCKET_ID, BUCKET_EXAMPLE_LOCKED)
        .add_manager(DEFAULT_NAMESPACE_ID, ACCOUNT_00)
        .add_admin(DEFAULT_BUCKET_ID, ACCOUNT_01)
        .build_and_execute_with_sanity_tests(|| {
            let origin = RawOrigin::Signed(ACCOUNT_99);
            assert_noop!(
                Buckets::remove_admin(
                    origin.into(),
                    DEFAULT_NAMESPACE_ID,
                    DEFAULT_BUCKET_ID,
                    ACCOUNT_01
                ),
                Error::<Test>::NotManager
            );

            assert_eq!(events().len(), 0);
        });
}

#[test]
fn remove_admin_invalid_bucket() {
    ExtBuilder::default()
        .add_namespace(DEFAULT_NAMESPACE_ID, MetadataMock { unique_plus_1: 10 })
        .add_bucket(DEFAULT_NAMESPACE_ID, DEFAULT_BUCKET_ID, BUCKET_EXAMPLE_LOCKED)
        .add_manager(DEFAULT_NAMESPACE_ID, ACCOUNT_00)
        .add_admin(DEFAULT_BUCKET_ID, ACCOUNT_01)
        .build_and_execute_with_sanity_tests(|| {
            let origin = RawOrigin::Signed(ACCOUNT_00);
            assert!(Admins::<Test>::get(DEFAULT_BUCKET_ID, ACCOUNT_01).is_some());
            assert_noop!(
                Buckets::remove_admin(
                    origin.into(),
                    DEFAULT_NAMESPACE_ID,
                    DEFAULT_BUCKET_ID + 1,
                    ACCOUNT_01
                ),
                Error::<Test>::UnknownBucket
            );
            assert_eq!(events().len(), 0);
        });
}

#[test]
fn remove_admin_invalid_namespace() {
    ExtBuilder::default()
        .add_namespace(DEFAULT_NAMESPACE_ID, MetadataMock { unique_plus_1: 10 })
        .add_bucket(DEFAULT_NAMESPACE_ID, DEFAULT_BUCKET_ID, BUCKET_EXAMPLE_LOCKED)
        .add_manager(DEFAULT_NAMESPACE_ID, ACCOUNT_00)
        .add_admin(DEFAULT_BUCKET_ID, ACCOUNT_01)
        .build_and_execute_with_sanity_tests(|| {
            let origin = RawOrigin::Signed(ACCOUNT_00);
            assert!(Admins::<Test>::get(DEFAULT_BUCKET_ID, ACCOUNT_01).is_some());
            assert_noop!(
                Buckets::remove_admin(
                    origin.into(),
                    DEFAULT_NAMESPACE_ID + 1,
                    DEFAULT_BUCKET_ID,
                    ACCOUNT_01
                ),
                Error::<Test>::UnknownBucket
            );
            assert_eq!(events().len(), 0);
        });
}
