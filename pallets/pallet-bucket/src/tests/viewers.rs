use frame_support::{assert_noop, assert_ok};
use frame_system::RawOrigin;

use crate::{mock::*, Error, Viewers};

#[test]
fn add_viewer() {
    ExtBuilder::default()
        .add_namespace(DEFAULT_NAMESPACE_ID, MetadataMock { unique_plus_1: 10 })
        .add_bucket(DEFAULT_NAMESPACE_ID, DEFAULT_BUCKET_ID, BUCKET_EXAMPLE_LOCKED)
        .add_admin(DEFAULT_BUCKET_ID, ACCOUNT_00)
        .build_and_execute_with_sanity_tests(|| {
            let origin = RawOrigin::Signed(ACCOUNT_00);
            assert_ok!(Buckets::add_viewer(
                origin.into(),
                DEFAULT_NAMESPACE_ID,
                DEFAULT_BUCKET_ID,
                DEFAULT_VIEWER_KEY
            ));
            assert!(Viewers::<Test>::get(DEFAULT_BUCKET_ID, DEFAULT_VIEWER_KEY).is_some());

            assert!(events().contains(&crate::Event::ViewerAdded {
                namespace_id: DEFAULT_NAMESPACE_ID,
                bucket_id: DEFAULT_BUCKET_ID,
                viewer: DEFAULT_VIEWER_KEY,
                caller: Some(ACCOUNT_00)
            }));
            assert_eq!(events().len(), 1);
        });
}

#[test]
fn add_viewer_not_authorized() {
    ExtBuilder::default()
        .add_namespace(DEFAULT_NAMESPACE_ID, MetadataMock { unique_plus_1: 10 })
        .add_bucket(DEFAULT_NAMESPACE_ID, DEFAULT_BUCKET_ID, BUCKET_EXAMPLE_LOCKED)
        .add_manager(DEFAULT_NAMESPACE_ID, ACCOUNT_00)
        .build_and_execute_with_sanity_tests(|| {
            let origin = RawOrigin::Signed(ACCOUNT_00);
            assert_noop!(
                Buckets::add_viewer(
                    origin.into(),
                    DEFAULT_NAMESPACE_ID,
                    DEFAULT_BUCKET_ID,
                    DEFAULT_VIEWER_KEY
                ),
                Error::<Test>::NotAdmin
            );
            assert_eq!(events().len(), 0);
        });
}

#[test]
fn add_viewer_no_bucket() {
    ExtBuilder::default()
        .add_namespace(DEFAULT_NAMESPACE_ID, MetadataMock { unique_plus_1: 10 })
        .add_bucket(DEFAULT_NAMESPACE_ID, DEFAULT_BUCKET_ID, BUCKET_EXAMPLE_LOCKED)
        .add_admin(DEFAULT_BUCKET_ID, ACCOUNT_00)
        .build_and_execute_with_sanity_tests(|| {
            let origin = RawOrigin::Signed(ACCOUNT_00);
            assert_noop!(
                Buckets::add_viewer(
                    origin.into(),
                    DEFAULT_NAMESPACE_ID,
                    DEFAULT_BUCKET_ID + 1,
                    DEFAULT_VIEWER_KEY
                ),
                Error::<Test>::UnknownBucket
            );
            assert_eq!(events().len(), 0);
        });
}

#[test]
fn remove_viewer() {
    ExtBuilder::default()
        .add_namespace(DEFAULT_NAMESPACE_ID, MetadataMock { unique_plus_1: 10 })
        .add_bucket(DEFAULT_NAMESPACE_ID, DEFAULT_BUCKET_ID, BUCKET_EXAMPLE_LOCKED)
        .add_admin(DEFAULT_BUCKET_ID, ACCOUNT_00)
        .add_viewer(DEFAULT_BUCKET_ID, DEFAULT_VIEWER_KEY)
        .build_and_execute_with_sanity_tests(|| {
            assert!(Viewers::<Test>::get(DEFAULT_BUCKET_ID, DEFAULT_VIEWER_KEY).is_some());
            let origin = RawOrigin::Signed(ACCOUNT_00);
            assert_ok!(Buckets::remove_viewer(
                origin.into(),
                DEFAULT_NAMESPACE_ID,
                DEFAULT_BUCKET_ID,
                DEFAULT_VIEWER_KEY
            ));
            assert!(Viewers::<Test>::get(DEFAULT_BUCKET_ID, DEFAULT_VIEWER_KEY).is_none());

            assert!(events().contains(&crate::Event::ViewerRemoved {
                namespace_id: DEFAULT_NAMESPACE_ID,
                bucket_id: DEFAULT_BUCKET_ID,
                viewer: DEFAULT_VIEWER_KEY,
                caller: Some(ACCOUNT_00)
            }));
            assert_eq!(events().len(), 1);
        });
}

#[test]
fn remove_viewer_no_bucket() {
    ExtBuilder::default()
        .add_namespace(DEFAULT_NAMESPACE_ID, MetadataMock { unique_plus_1: 10 })
        .add_bucket(DEFAULT_NAMESPACE_ID, DEFAULT_BUCKET_ID, BUCKET_EXAMPLE_LOCKED)
        .add_admin(DEFAULT_BUCKET_ID, ACCOUNT_00)
        .add_viewer(DEFAULT_BUCKET_ID, DEFAULT_VIEWER_KEY)
        .build_and_execute_with_sanity_tests(|| {
            assert!(Viewers::<Test>::get(DEFAULT_BUCKET_ID, DEFAULT_VIEWER_KEY).is_some());
            let origin = RawOrigin::Signed(ACCOUNT_00);
            assert_noop!(
                Buckets::remove_viewer(
                    origin.into(),
                    DEFAULT_NAMESPACE_ID,
                    DEFAULT_BUCKET_ID + 1,
                    DEFAULT_VIEWER_KEY
                ),
                Error::<Test>::UnknownBucket
            );
            assert_eq!(events().len(), 0);
        });
}

#[test]
fn remove_viewer_no_namespace() {
    ExtBuilder::default()
        .add_namespace(DEFAULT_NAMESPACE_ID, MetadataMock { unique_plus_1: 10 })
        .add_bucket(DEFAULT_NAMESPACE_ID, DEFAULT_BUCKET_ID, BUCKET_EXAMPLE_LOCKED)
        .add_manager(DEFAULT_NAMESPACE_ID, ACCOUNT_00)
        .add_viewer(DEFAULT_BUCKET_ID, DEFAULT_VIEWER_KEY)
        .build_and_execute_with_sanity_tests(|| {
            assert!(Viewers::<Test>::get(DEFAULT_BUCKET_ID, DEFAULT_VIEWER_KEY).is_some());
            let origin = RawOrigin::Signed(ACCOUNT_00);
            assert_noop!(
                Buckets::remove_viewer(
                    origin.into(),
                    DEFAULT_NAMESPACE_ID + 1,
                    DEFAULT_BUCKET_ID,
                    DEFAULT_VIEWER_KEY
                ),
                Error::<Test>::UnknownBucket
            );
            assert_eq!(events().len(), 0);
        });
}
