use frame_support::{assert_err, assert_noop, assert_ok, sp_runtime::DispatchError};
use frame_system::RawOrigin;

use crate::{mock::*, Error};

#[test]
fn create_namespace() {
    ExtBuilder::default()
        .with_balances(vec![(ACCOUNT_00, DEFAULT_BALANCE)])
        .build_and_execute_with_sanity_tests(|| {
            let origin = RawOrigin::Signed(ACCOUNT_00);
            assert_ok!(Buckets::create_namespace(
                origin.clone().into(),
                MetadataInputMock { unique: 10 }
            ));
            let namespace = Buckets::namespace_with_id(DEFAULT_NAMESPACE_ID);
            assert!(namespace.is_some());
            let manager = Buckets::manager_with_id(DEFAULT_NAMESPACE_ID, ACCOUNT_00);
            assert!(manager.is_some());
            let balance = Balances::free_balance(ACCOUNT_00);
            assert_eq!(DEFAULT_BALANCE - StorageFee::get(), balance);

            let events = events();
            assert!(events.contains(&crate::Event::NamespaceCreated {
                namespace_id: DEFAULT_NAMESPACE_ID,
                metadata: MetadataMock { unique_plus_1: 11 },
                creator: Some(ACCOUNT_00),
            }));
            assert!(events.contains(&crate::Event::ManagerAdded {
                namespace_id: DEFAULT_NAMESPACE_ID,
                manager: ACCOUNT_00,
                caller: None,
            }));
            assert_eq!(events.len(), 2);
            assert_ok!(Buckets::create_namespace(origin.into(), MetadataInputMock { unique: 10 }));
            let namespace = Buckets::namespace_with_id(1);
            assert!(namespace.is_some());
        })
}

#[test]
fn insufficient_balance() {
    ExtBuilder::default()
        .with_balances(vec![(ACCOUNT_00, StorageFee::get())])
        .build_and_execute_with_sanity_tests(|| {
            let origin = RawOrigin::Signed(ACCOUNT_00);
            assert_noop!(
                Buckets::create_namespace(origin.into(), MetadataInputMock { unique: 10 }),
                Error::<Test>::UnableToPayFees
            );
            assert_eq!(events().len(), 0);
        })
}

#[test]
fn exact_sufficient_balance() {
    ExtBuilder::default()
        .with_balances(vec![(ACCOUNT_00, StorageFee::get() + 1)])
        .build_and_execute_with_sanity_tests(|| {
            let origin = RawOrigin::Signed(ACCOUNT_00);
            assert_ok!(Buckets::create_namespace(origin.into(), MetadataInputMock { unique: 10 }));
        })
}

#[test]
fn duplicate_namespace() {
    ExtBuilder::default()
        .with_balances(vec![(ACCOUNT_00, DEFAULT_BALANCE)])
        .add_namespace(DEFAULT_NAMESPACE_ID, MetadataMock { unique_plus_1: 10 })
        .build_and_execute_with_sanity_tests(|| {
            let origin = RawOrigin::Signed(ACCOUNT_00);
            assert_noop!(
                Buckets::create_namespace(origin.into(), MetadataInputMock { unique: 10 }),
                Error::<Test>::NamespaceAlreadyExists
            );
            assert_eq!(events().len(), 0);
        })
}

#[test]
fn remove_namespace_dangling_bucket() {
    ExtBuilder::default()
        .add_namespace(DEFAULT_NAMESPACE_ID, MetadataMock { unique_plus_1: 10 })
        .add_bucket(DEFAULT_NAMESPACE_ID, DEFAULT_BUCKET_ID, BUCKET_EXAMPLE_LOCKED)
        .build_and_execute_with_sanity_tests(|| {
            let origin = RawOrigin::Root;
            assert_noop!(
                Buckets::force_remove_namespace(origin.into(), DEFAULT_NAMESPACE_ID),
                Error::<Test>::DanglingBuckets
            );
            assert_eq!(events().len(), 0);
        })
}

#[test]
fn force_remove_namespace_dangling_managers() {
    ExtBuilder::default()
        .add_namespace(DEFAULT_NAMESPACE_ID, MetadataMock { unique_plus_1: 10 })
        .add_manager(DEFAULT_NAMESPACE_ID, ACCOUNT_00)
        .add_manager(DEFAULT_NAMESPACE_ID, ACCOUNT_01)
        .build_and_execute_with_sanity_tests(|| {
            let origin = RawOrigin::Root;
            assert!(Buckets::namespace_with_id(DEFAULT_NAMESPACE_ID).is_some());
            assert_err!(
                Buckets::force_remove_namespace(origin.into(), DEFAULT_NAMESPACE_ID),
                Error::<Test>::DanglingManagers
            );
            assert_eq!(events().len(), 0);
        })
}

#[test]
fn only_force_origin_can_remove_namespace() {
    ExtBuilder::default()
        .add_namespace(DEFAULT_NAMESPACE_ID, MetadataMock { unique_plus_1: 10 })
        .add_manager(DEFAULT_NAMESPACE_ID, ACCOUNT_00)
        .build_and_execute_with_sanity_tests(|| {
            let origin = RawOrigin::Signed(ACCOUNT_00);
            assert_noop!(
                Buckets::force_remove_namespace(origin.into(), DEFAULT_NAMESPACE_ID),
                DispatchError::BadOrigin
            );
            assert_eq!(events().len(), 0);
        })
}

#[test]
fn force_remove_namespace() {
    let mut ext = ExtBuilder::default()
        .add_namespace(DEFAULT_NAMESPACE_ID, MetadataMock { unique_plus_1: 10 })
        .build();
    ext.execute_with(|| {
        let origin = RawOrigin::Root;
        assert!(Buckets::namespace_with_id(DEFAULT_NAMESPACE_ID).is_some());
        assert_ok!(Buckets::force_remove_namespace(origin.into(), DEFAULT_NAMESPACE_ID),);
        assert!(Buckets::namespace_with_id(DEFAULT_NAMESPACE_ID).is_none());
        assert!(events().contains(&crate::Event::NamespaceDeleted {
            namespace_id: DEFAULT_NAMESPACE_ID,
            metadata: MetadataMock { unique_plus_1: 10 }
        }));
        assert_eq!(events().len(), 1);
    })
}
