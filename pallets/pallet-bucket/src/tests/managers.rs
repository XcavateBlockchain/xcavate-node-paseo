use frame_support::{assert_noop, assert_ok, sp_runtime::DispatchError};
use frame_system::RawOrigin;

use crate::{mock::*, Error, Managers};

#[test]
fn add_manager() {
    ExtBuilder::default()
        .add_namespace(DEFAULT_NAMESPACE_ID, MetadataMock { unique_plus_1: 10 })
        .add_manager(DEFAULT_NAMESPACE_ID, ACCOUNT_00)
        .build_and_execute_with_sanity_tests(|| {
            let origin = RawOrigin::Signed(ACCOUNT_00);
            assert_ok!(Buckets::add_manager(origin.into(), DEFAULT_NAMESPACE_ID, ACCOUNT_01));
            assert!(Managers::<Test>::get(DEFAULT_NAMESPACE_ID, ACCOUNT_01).is_some());

            assert!(events().contains(&crate::Event::ManagerAdded {
                namespace_id: DEFAULT_NAMESPACE_ID,
                manager: ACCOUNT_01,
                caller: Some(ACCOUNT_00)
            }));
            assert_eq!(events().len(), 1);
        });
}

#[test]
fn add_manager_no_namespace() {
    ExtBuilder::default()
        .add_namespace(DEFAULT_NAMESPACE_ID, MetadataMock { unique_plus_1: 10 })
        .add_manager(DEFAULT_NAMESPACE_ID, ACCOUNT_00)
        .build_and_execute_with_sanity_tests(|| {
            let origin = RawOrigin::Signed(ACCOUNT_00);
            assert_noop!(
                Buckets::add_manager(origin.into(), DEFAULT_NAMESPACE_ID + 1, ACCOUNT_01),
                Error::<Test>::UnknownNamespace
            );
            assert_eq!(events().len(), 0);
        });
}

#[test]
fn unauthorized_add_manager() {
    ExtBuilder::default()
        .add_namespace(DEFAULT_NAMESPACE_ID, MetadataMock { unique_plus_1: 10 })
        .build_and_execute_with_sanity_tests(|| {
            let origin = RawOrigin::Signed(ACCOUNT_00);
            assert_noop!(
                Buckets::add_manager(origin.into(), DEFAULT_NAMESPACE_ID, ACCOUNT_01),
                Error::<Test>::NotManager
            );
            assert_eq!(events().len(), 0);
        });
}

#[test]
fn is_manager() {
    ExtBuilder::default()
        .add_namespace(DEFAULT_NAMESPACE_ID, MetadataMock { unique_plus_1: 10 })
        .add_manager(DEFAULT_NAMESPACE_ID, ACCOUNT_00)
        .build_and_execute_with_sanity_tests(|| {
            assert!(Buckets::is_manager(&DEFAULT_NAMESPACE_ID, &ACCOUNT_00));
            assert!(!Buckets::is_manager(&DEFAULT_NAMESPACE_ID, &ACCOUNT_01));
            assert_eq!(events().len(), 0);
        });
}

#[test]
fn remove_manager() {
    ExtBuilder::default()
        .add_namespace(DEFAULT_NAMESPACE_ID, MetadataMock { unique_plus_1: 10 })
        .add_manager(DEFAULT_NAMESPACE_ID, ACCOUNT_00)
        .add_manager(DEFAULT_NAMESPACE_ID, ACCOUNT_01)
        .build_and_execute_with_sanity_tests(|| {
            let origin = RawOrigin::Signed(ACCOUNT_00);
            assert_ok!(Buckets::remove_manager(origin.into(), DEFAULT_NAMESPACE_ID, ACCOUNT_00));
            assert!(Managers::<Test>::get(DEFAULT_NAMESPACE_ID, ACCOUNT_00).is_none());

            assert!(events().contains(&crate::Event::ManagerRemoved {
                namespace_id: DEFAULT_NAMESPACE_ID,
                manager: ACCOUNT_00,
                caller: Some(ACCOUNT_00),
            }));
            assert_eq!(events().len(), 1);
        });
}

#[test]
fn remove_last_manager() {
    ExtBuilder::default()
        .add_namespace(DEFAULT_NAMESPACE_ID, MetadataMock { unique_plus_1: 10 })
        .add_manager(DEFAULT_NAMESPACE_ID, ACCOUNT_00)
        .build_and_execute_with_sanity_tests(|| {
            let origin = RawOrigin::Signed(ACCOUNT_00);
            assert_noop!(
                Buckets::remove_manager(origin.into(), DEFAULT_NAMESPACE_ID, ACCOUNT_00),
                Error::<Test>::LastManagerRemoval
            );
            assert_eq!(events().len(), 0);
        });
}

#[test]
fn unauthorized_remove_manager() {
    ExtBuilder::default()
        .add_namespace(DEFAULT_NAMESPACE_ID, MetadataMock { unique_plus_1: 10 })
        .add_manager(DEFAULT_NAMESPACE_ID, ACCOUNT_00)
        .build_and_execute_with_sanity_tests(|| {
            let origin = RawOrigin::Signed(ACCOUNT_01);
            assert_noop!(
                Buckets::remove_manager(origin.into(), DEFAULT_NAMESPACE_ID, ACCOUNT_01),
                Error::<Test>::NotManager
            );
            assert_eq!(events().len(), 0);
        });
}

#[test]
fn remove_manager_unknown_namespace() {
    ExtBuilder::default().build_and_execute_with_sanity_tests(|| {
        let origin = RawOrigin::Signed(ACCOUNT_01);
        assert_noop!(
            Buckets::remove_manager(origin.into(), DEFAULT_NAMESPACE_ID, ACCOUNT_01),
            Error::<Test>::UnknownNamespace
        );
        assert_eq!(events().len(), 0);
    });
}

#[test]
fn force_add_manager_bad_origin() {
    ExtBuilder::default()
        .add_namespace(DEFAULT_NAMESPACE_ID, MetadataMock { unique_plus_1: 10 })
        .build_and_execute_with_sanity_tests(|| {
            let origin = RawOrigin::Signed(ACCOUNT_00);
            assert_noop!(
                Buckets::force_add_manager(origin.into(), DEFAULT_NAMESPACE_ID, ACCOUNT_01),
                DispatchError::BadOrigin
            );
            assert_eq!(events().len(), 0);
        });
}

#[test]
fn force_add_manager() {
    ExtBuilder::default()
        .add_namespace(DEFAULT_NAMESPACE_ID, MetadataMock { unique_plus_1: 10 })
        .build_and_execute_with_sanity_tests(|| {
            let origin = RawOrigin::Root;
            assert_ok!(Buckets::force_add_manager(origin.into(), DEFAULT_NAMESPACE_ID, ACCOUNT_01));
            assert!(events().contains(&crate::Event::ManagerAdded {
                namespace_id: DEFAULT_NAMESPACE_ID,
                manager: ACCOUNT_01,
                caller: None
            }));
            assert_eq!(events().len(), 1);
        });
}
