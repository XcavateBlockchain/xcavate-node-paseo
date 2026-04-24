use frame_support::{assert_noop, assert_ok};
use frame_system::RawOrigin;

use crate::{mock::*, Contributors, Error};

#[test]
fn add_contributor() {
    ExtBuilder::default()
        .add_namespace(DEFAULT_NAMESPACE_ID, MetadataMock { unique_plus_1: 10 })
        .add_bucket(DEFAULT_NAMESPACE_ID, DEFAULT_BUCKET_ID, BUCKET_EXAMPLE_LOCKED)
        .add_admin(DEFAULT_BUCKET_ID, ACCOUNT_00)
        .build_and_execute_with_sanity_tests(|| {
            let origin = RawOrigin::Signed(ACCOUNT_00);
            assert_ok!(Buckets::add_contributor(
                origin.into(),
                DEFAULT_NAMESPACE_ID,
                DEFAULT_BUCKET_ID,
                ACCOUNT_01
            ));
            assert!(Contributors::<Test>::get(DEFAULT_BUCKET_ID, ACCOUNT_01).is_some());

            assert!(events().contains(&crate::Event::ContributorAdded {
                namespace_id: DEFAULT_NAMESPACE_ID,
                bucket_id: DEFAULT_BUCKET_ID,
                contributor: ACCOUNT_01,
                caller: Some(ACCOUNT_00)
            }));
            assert_eq!(events().len(), 1);
        });
}

#[test]
fn add_contributor_not_authorized() {
    ExtBuilder::default()
        .add_namespace(DEFAULT_NAMESPACE_ID, MetadataMock { unique_plus_1: 10 })
        .add_bucket(DEFAULT_NAMESPACE_ID, DEFAULT_BUCKET_ID, BUCKET_EXAMPLE_LOCKED)
        .add_manager(DEFAULT_NAMESPACE_ID, ACCOUNT_00)
        .build_and_execute_with_sanity_tests(|| {
            let origin = RawOrigin::Signed(ACCOUNT_00);
            assert_noop!(
                Buckets::add_contributor(
                    origin.into(),
                    DEFAULT_NAMESPACE_ID,
                    DEFAULT_BUCKET_ID,
                    ACCOUNT_01
                ),
                Error::<Test>::NotAdmin
            );
            assert_eq!(events().len(), 0);
        });
}

#[test]
fn add_contributor_no_bucket() {
    ExtBuilder::default()
        .add_namespace(DEFAULT_NAMESPACE_ID, MetadataMock { unique_plus_1: 10 })
        .add_bucket(DEFAULT_NAMESPACE_ID, DEFAULT_BUCKET_ID, BUCKET_EXAMPLE_LOCKED)
        .add_admin(DEFAULT_BUCKET_ID, ACCOUNT_00)
        .build_and_execute_with_sanity_tests(|| {
            let origin = RawOrigin::Signed(ACCOUNT_00);
            assert_noop!(
                Buckets::add_contributor(
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
fn remove_contributor() {
    ExtBuilder::default()
        .add_namespace(DEFAULT_NAMESPACE_ID, MetadataMock { unique_plus_1: 10 })
        .add_bucket(DEFAULT_NAMESPACE_ID, DEFAULT_BUCKET_ID, BUCKET_EXAMPLE_LOCKED)
        .add_admin(DEFAULT_BUCKET_ID, ACCOUNT_00)
        .add_contributor(DEFAULT_BUCKET_ID, ACCOUNT_01)
        .build_and_execute_with_sanity_tests(|| {
            assert!(Contributors::<Test>::get(DEFAULT_BUCKET_ID, ACCOUNT_01).is_some());
            let origin = RawOrigin::Signed(ACCOUNT_00);
            assert_ok!(Buckets::remove_contributor(
                origin.into(),
                DEFAULT_NAMESPACE_ID,
                DEFAULT_BUCKET_ID,
                ACCOUNT_01
            ));
            assert!(Contributors::<Test>::get(DEFAULT_BUCKET_ID, ACCOUNT_01).is_none());

            assert!(events().contains(&crate::Event::ContributorRemoved {
                namespace_id: DEFAULT_NAMESPACE_ID,
                bucket_id: DEFAULT_BUCKET_ID,
                contributor: ACCOUNT_01,
                caller: Some(ACCOUNT_00)
            }));
            assert_eq!(events().len(), 1);
        });
}

#[test]
fn remove_contributor_no_bucket() {
    ExtBuilder::default()
        .add_namespace(DEFAULT_NAMESPACE_ID, MetadataMock { unique_plus_1: 10 })
        .add_bucket(DEFAULT_NAMESPACE_ID, DEFAULT_BUCKET_ID, BUCKET_EXAMPLE_LOCKED)
        .add_admin(DEFAULT_BUCKET_ID, ACCOUNT_00)
        .add_contributor(DEFAULT_BUCKET_ID, ACCOUNT_01)
        .build_and_execute_with_sanity_tests(|| {
            assert!(Contributors::<Test>::get(DEFAULT_BUCKET_ID, ACCOUNT_01).is_some());
            let origin = RawOrigin::Signed(ACCOUNT_00);
            assert_noop!(
                Buckets::remove_contributor(
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
fn remove_contributor_no_namespace() {
    ExtBuilder::default()
        .add_namespace(DEFAULT_NAMESPACE_ID, MetadataMock { unique_plus_1: 10 })
        .add_bucket(DEFAULT_NAMESPACE_ID, DEFAULT_BUCKET_ID, BUCKET_EXAMPLE_LOCKED)
        .add_manager(DEFAULT_NAMESPACE_ID, ACCOUNT_00)
        .add_contributor(DEFAULT_BUCKET_ID, ACCOUNT_01)
        .build_and_execute_with_sanity_tests(|| {
            assert!(Contributors::<Test>::get(DEFAULT_BUCKET_ID, ACCOUNT_01).is_some());
            let origin = RawOrigin::Signed(ACCOUNT_00);
            assert_noop!(
                Buckets::remove_contributor(
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
