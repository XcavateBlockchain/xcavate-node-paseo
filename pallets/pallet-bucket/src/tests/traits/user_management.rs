use frame_support::assert_ok;

use crate::{mock::*, traits::UserManagement, Admins, Contributors, Managers};

#[test]
fn add_admin() {
    ExtBuilder::default()
        .add_namespace(DEFAULT_NAMESPACE_ID, MetadataMock { unique_plus_1: 10 })
        .add_bucket(DEFAULT_NAMESPACE_ID, DEFAULT_BUCKET_ID, BUCKET_EXAMPLE_LOCKED)
        .add_manager(DEFAULT_NAMESPACE_ID, ACCOUNT_00)
        .build_and_execute_with_sanity_tests(|| {
            assert!(Admins::<Test>::get(DEFAULT_BUCKET_ID, ACCOUNT_01).is_none());
            assert_ok!(<Buckets as UserManagement<Test>>::add_admin(
                DEFAULT_NAMESPACE_ID,
                DEFAULT_BUCKET_ID,
                ACCOUNT_01
            ));
            assert!(Admins::<Test>::get(DEFAULT_BUCKET_ID, ACCOUNT_01).is_some());
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
            assert!(Admins::<Test>::get(DEFAULT_BUCKET_ID, ACCOUNT_01).is_some());
            assert_ok!(<Buckets as UserManagement<Test>>::remove_admin(
                DEFAULT_NAMESPACE_ID,
                DEFAULT_BUCKET_ID,
                ACCOUNT_01
            ));
            assert!(Admins::<Test>::get(DEFAULT_BUCKET_ID, ACCOUNT_01).is_none());
        });
}

#[test]
fn add_contributor() {
    ExtBuilder::default()
        .add_namespace(DEFAULT_NAMESPACE_ID, MetadataMock { unique_plus_1: 10 })
        .add_bucket(DEFAULT_NAMESPACE_ID, DEFAULT_BUCKET_ID, BUCKET_EXAMPLE_LOCKED)
        .add_admin(DEFAULT_BUCKET_ID, ACCOUNT_00)
        .build_and_execute_with_sanity_tests(|| {
            assert!(Contributors::<Test>::get(DEFAULT_BUCKET_ID, ACCOUNT_01).is_none());
            assert_ok!(<Buckets as UserManagement<Test>>::add_contributor(
                DEFAULT_NAMESPACE_ID,
                DEFAULT_BUCKET_ID,
                ACCOUNT_01
            ));
            assert!(Contributors::<Test>::get(DEFAULT_BUCKET_ID, ACCOUNT_01).is_some());
        });
}

#[test]
fn remove_contributor() {
    ExtBuilder::default()
        .add_namespace(DEFAULT_NAMESPACE_ID, MetadataMock { unique_plus_1: 10 })
        .add_bucket(DEFAULT_NAMESPACE_ID, DEFAULT_BUCKET_ID, BUCKET_EXAMPLE_LOCKED)
        .add_manager(DEFAULT_NAMESPACE_ID, ACCOUNT_00)
        .add_contributor(DEFAULT_BUCKET_ID, ACCOUNT_01)
        .build_and_execute_with_sanity_tests(|| {
            assert!(Contributors::<Test>::get(DEFAULT_BUCKET_ID, ACCOUNT_01).is_some());
            assert_ok!(<Buckets as UserManagement<Test>>::remove_contributor(
                DEFAULT_NAMESPACE_ID,
                DEFAULT_BUCKET_ID,
                ACCOUNT_01
            ));
            assert!(Contributors::<Test>::get(DEFAULT_BUCKET_ID, ACCOUNT_01).is_none());
        });
}

#[test]
fn add_manager() {
    ExtBuilder::default()
        .add_namespace(DEFAULT_NAMESPACE_ID, MetadataMock { unique_plus_1: 10 })
        .add_manager(DEFAULT_NAMESPACE_ID, ACCOUNT_00)
        .build_and_execute_with_sanity_tests(|| {
            assert_ok!(<Buckets as UserManagement<Test>>::add_manager(
                DEFAULT_NAMESPACE_ID,
                ACCOUNT_01
            ));
            assert!(Managers::<Test>::get(DEFAULT_NAMESPACE_ID, ACCOUNT_01).is_some());
        });
}

#[test]
fn remove_manager() {
    ExtBuilder::default()
        .add_namespace(DEFAULT_NAMESPACE_ID, MetadataMock { unique_plus_1: 10 })
        .add_manager(DEFAULT_NAMESPACE_ID, ACCOUNT_00)
        .add_manager(DEFAULT_NAMESPACE_ID, ACCOUNT_01)
        .build_and_execute_with_sanity_tests(|| {
            assert!(Managers::<Test>::get(DEFAULT_NAMESPACE_ID, ACCOUNT_00).is_some());
            assert_ok!(<Buckets as UserManagement<Test>>::remove_manager(
                DEFAULT_NAMESPACE_ID,
                ACCOUNT_00
            ));
            assert!(Managers::<Test>::get(DEFAULT_NAMESPACE_ID, ACCOUNT_00).is_none());
        });
}
