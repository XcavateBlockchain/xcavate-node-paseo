use frame_support::{assert_noop, assert_ok, sp_runtime::DispatchError};
use frame_system::RawOrigin;

use crate::{mock::*, Error, MessageInputOf, Messages};

const MESSAGE_ID: u128 = 10;

#[test]
fn write_without_tag() {
    ExtBuilder::default()
        .add_namespace(DEFAULT_NAMESPACE_ID, MetadataMock { unique_plus_1: 10 })
        .with_balances(vec![(ACCOUNT_01, StorageFee::get() + 1)])
        .add_bucket(DEFAULT_NAMESPACE_ID, DEFAULT_BUCKET_ID, BUCKET_EXAMPLE_UNLOCKED)
        .add_contributor(DEFAULT_BUCKET_ID, ACCOUNT_01)
        .build_and_execute_with_sanity_tests(|| {
            let origin = RawOrigin::Signed(ACCOUNT_01);
            let message = MessageInputOf::<Test> {
                reference: create_bounded_vec_example(0),
                tag: None,
                metadata_input: MetadataInputMock { unique: 12 },
            };

            let bucket = Buckets::bucket_with_id(DEFAULT_NAMESPACE_ID, DEFAULT_BUCKET_ID)
                .expect("Bucket should exist");
            let message_id = bucket.get_next_message_id();
            assert!(Buckets::message_with_id(DEFAULT_BUCKET_ID, message_id).is_none());
            assert_ok!(Buckets::write(
                origin.into(),
                DEFAULT_NAMESPACE_ID,
                DEFAULT_BUCKET_ID,
                message
            ));
            assert!(Buckets::message_with_id(DEFAULT_BUCKET_ID, message_id).is_some());

            assert!(events().contains(&crate::Event::NewMessage {
                namespace_id: DEFAULT_NAMESPACE_ID,
                bucket_id: DEFAULT_BUCKET_ID,
                contributor: ACCOUNT_01,
                message_id,
                message: MessageMock {
                    reference: create_bounded_vec_example(0),
                    tag: None,
                    metadata: MetadataMock { unique_plus_1: 13 },
                }
            }));
            assert_eq!(events().len(), 1);
        });
}

#[test]
fn write_insufficient_balance() {
    ExtBuilder::default()
        .add_namespace(DEFAULT_NAMESPACE_ID, MetadataMock { unique_plus_1: 10 })
        .with_balances(vec![(ACCOUNT_01, StorageFee::get())])
        .add_bucket(DEFAULT_NAMESPACE_ID, DEFAULT_BUCKET_ID, BUCKET_EXAMPLE_UNLOCKED)
        .add_contributor(DEFAULT_BUCKET_ID, ACCOUNT_01)
        .build_and_execute_with_sanity_tests(|| {
            let origin = RawOrigin::Signed(ACCOUNT_01);
            let message = MessageInputOf::<Test> {
                reference: create_bounded_vec_example(0),
                tag: None,
                metadata_input: MetadataInputMock { unique: 12 },
            };
            assert_noop!(
                Buckets::write(origin.into(), DEFAULT_NAMESPACE_ID, DEFAULT_BUCKET_ID, message),
                Error::<Test>::UnableToPayFees
            );
            assert_eq!(events().len(), 0);
        });
}

#[test]
fn write_unknown_tag() {
    ExtBuilder::default()
        .add_namespace(DEFAULT_NAMESPACE_ID, MetadataMock { unique_plus_1: 10 })
        .add_bucket(DEFAULT_NAMESPACE_ID, DEFAULT_BUCKET_ID, BUCKET_EXAMPLE_UNLOCKED)
        .add_contributor(DEFAULT_BUCKET_ID, ACCOUNT_01)
        .build_and_execute_with_sanity_tests(|| {
            let origin = RawOrigin::Signed(ACCOUNT_01);
            let message = MessageInputOf::<Test> {
                reference: create_bounded_vec_example(0),
                tag: Some(create_bounded_vec_example(1)),
                metadata_input: MetadataInputMock { unique: 12 },
            };
            assert_noop!(
                Buckets::write(origin.into(), DEFAULT_NAMESPACE_ID, DEFAULT_BUCKET_ID, message),
                Error::<Test>::UnknownTag
            );
            assert_eq!(events().len(), 0);
        });
}

#[test]
fn admin_can_not_write() {
    ExtBuilder::default()
        .add_namespace(DEFAULT_NAMESPACE_ID, MetadataMock { unique_plus_1: 10 })
        .add_bucket(DEFAULT_NAMESPACE_ID, DEFAULT_BUCKET_ID, BUCKET_EXAMPLE_UNLOCKED)
        .add_admin(DEFAULT_BUCKET_ID, ACCOUNT_01)
        .build_and_execute_with_sanity_tests(|| {
            let origin = RawOrigin::Signed(ACCOUNT_01);
            let message = MessageInputOf::<Test> {
                reference: create_bounded_vec_example(0),
                tag: None,
                metadata_input: MetadataInputMock { unique: 12 },
            };
            assert_noop!(
                Buckets::write(origin.into(), DEFAULT_NAMESPACE_ID, DEFAULT_BUCKET_ID, message),
                Error::<Test>::NotContributor
            );
            assert_eq!(events().len(), 0);
        });
}

#[test]
fn manager_can_not_write() {
    ExtBuilder::default()
        .add_namespace(DEFAULT_NAMESPACE_ID, MetadataMock { unique_plus_1: 10 })
        .add_bucket(DEFAULT_NAMESPACE_ID, DEFAULT_BUCKET_ID, BUCKET_EXAMPLE_UNLOCKED)
        .add_manager(DEFAULT_NAMESPACE_ID, ACCOUNT_01)
        .build_and_execute_with_sanity_tests(|| {
            let origin = RawOrigin::Signed(ACCOUNT_01);
            let message = MessageInputOf::<Test> {
                reference: create_bounded_vec_example(0),
                tag: None,
                metadata_input: MetadataInputMock { unique: 12 },
            };
            assert_noop!(
                Buckets::write(origin.into(), DEFAULT_NAMESPACE_ID, DEFAULT_BUCKET_ID, message),
                Error::<Test>::NotContributor
            );
            assert_eq!(events().len(), 0);
        });
}

#[test]
fn bucket_is_locked() {
    ExtBuilder::default()
        .add_namespace(DEFAULT_NAMESPACE_ID, MetadataMock { unique_plus_1: 10 })
        .add_bucket(DEFAULT_NAMESPACE_ID, DEFAULT_BUCKET_ID, BUCKET_EXAMPLE_LOCKED)
        .add_contributor(DEFAULT_BUCKET_ID, ACCOUNT_01)
        .build_and_execute_with_sanity_tests(|| {
            let origin = RawOrigin::Signed(ACCOUNT_01);
            let message = MessageInputOf::<Test> {
                reference: create_bounded_vec_example(0),
                tag: None,
                metadata_input: MetadataInputMock { unique: 12 },
            };
            assert_noop!(
                Buckets::write(origin.into(), DEFAULT_NAMESPACE_ID, DEFAULT_BUCKET_ID, message),
                Error::<Test>::BucketIsLocked
            );
            assert_eq!(events().len(), 0);
        });
}

#[test]
fn force_remove_message() {
    let message = MessageMock {
        reference: create_bounded_vec_example(0),
        tag: None,
        metadata: MetadataMock { unique_plus_1: 9 },
    };
    ExtBuilder::default()
        .add_namespace(DEFAULT_NAMESPACE_ID, MetadataMock { unique_plus_1: 10 })
        .add_bucket(DEFAULT_NAMESPACE_ID, DEFAULT_BUCKET_ID, BUCKET_EXAMPLE_LOCKED)
        .add_contributor(DEFAULT_BUCKET_ID, ACCOUNT_01)
        .add_message(DEFAULT_BUCKET_ID, MESSAGE_ID, message.clone())
        .build_and_execute_with_sanity_tests(|| {
            assert!(Messages::<Test>::get(DEFAULT_BUCKET_ID, MESSAGE_ID).is_some());
            let origin = RawOrigin::Root;
            // `force_remove_message` works even when the bucket is locked.
            assert_ok!(Buckets::force_remove_message(origin.into(), DEFAULT_BUCKET_ID, MESSAGE_ID));
            assert!(Messages::<Test>::get(DEFAULT_BUCKET_ID, MESSAGE_ID).is_none());
            assert!(events().contains(&crate::Event::MessageDeleted {
                bucket_id: DEFAULT_BUCKET_ID,
                message_id: MESSAGE_ID,
                message,
            }));
        });
}

#[test]
fn force_remove_message_unauthorized() {
    let message = MessageMock {
        reference: create_bounded_vec_example(0),
        tag: None,
        metadata: MetadataMock { unique_plus_1: 9 },
    };
    ExtBuilder::default()
        .add_namespace(DEFAULT_NAMESPACE_ID, MetadataMock { unique_plus_1: 10 })
        .add_bucket(DEFAULT_NAMESPACE_ID, DEFAULT_BUCKET_ID, BUCKET_EXAMPLE_LOCKED)
        .add_contributor(DEFAULT_BUCKET_ID, ACCOUNT_01)
        .add_message(DEFAULT_BUCKET_ID, MESSAGE_ID, message)
        .build_and_execute_with_sanity_tests(|| {
            let origin = RawOrigin::Signed(ACCOUNT_01);
            assert_noop!(
                Buckets::force_remove_message(origin.into(), DEFAULT_BUCKET_ID, MESSAGE_ID),
                DispatchError::BadOrigin
            );
            assert_eq!(events().len(), 0);
        });
}
