use frame_support::{assert_noop, assert_ok};
use frame_system::RawOrigin;

use crate::{mock::*, Error, MessageInputOf, TagMessages};

#[test]
fn create_tag() {
    ExtBuilder::default()
        .with_balances(vec![(ACCOUNT_01, StorageFee::get() + 1)])
        .add_namespace(DEFAULT_NAMESPACE_ID, MetadataMock { unique_plus_1: 10 })
        .add_bucket(DEFAULT_NAMESPACE_ID, DEFAULT_BUCKET_ID, BUCKET_EXAMPLE_UNLOCKED)
        .add_admin(DEFAULT_BUCKET_ID, ACCOUNT_01)
        .build_and_execute_with_sanity_tests(|| {
            let origin = RawOrigin::Signed(ACCOUNT_01);
            let tag = create_bounded_vec_example(0);
            assert_ok!(Buckets::create_tag(origin.into(), DEFAULT_BUCKET_ID, tag.clone()));
            assert!(Buckets::tag_with_id(DEFAULT_BUCKET_ID, tag.clone()).is_some());

            assert!(events().contains(&crate::Event::NewTag {
                bucket_id: DEFAULT_BUCKET_ID,
                tag,
                creator: Some(ACCOUNT_01)
            }));
            assert_eq!(events().len(), 1);
        });
}

#[test]
fn create_tag_insufficient_balance() {
    ExtBuilder::default()
        .add_namespace(DEFAULT_NAMESPACE_ID, MetadataMock { unique_plus_1: 10 })
        .with_balances(vec![(ACCOUNT_01, StorageFee::get())])
        .add_bucket(DEFAULT_NAMESPACE_ID, DEFAULT_BUCKET_ID, BUCKET_EXAMPLE_UNLOCKED)
        .add_admin(DEFAULT_BUCKET_ID, ACCOUNT_01)
        .build_and_execute_with_sanity_tests(|| {
            let origin = RawOrigin::Signed(ACCOUNT_01);
            let tag = create_bounded_vec_example(0);
            assert_noop!(
                Buckets::create_tag(origin.into(), DEFAULT_BUCKET_ID, tag.clone()),
                Error::<Test>::UnableToPayFees
            );
            assert_eq!(events().len(), 0);
        });
}

#[test]
fn create_tag_not_admin() {
    ExtBuilder::default()
        .with_balances(vec![(ACCOUNT_01, DEFAULT_BALANCE)])
        .add_namespace(DEFAULT_NAMESPACE_ID, MetadataMock { unique_plus_1: 10 })
        .add_bucket(DEFAULT_NAMESPACE_ID, DEFAULT_BUCKET_ID, BUCKET_EXAMPLE_UNLOCKED)
        .add_contributor(DEFAULT_BUCKET_ID, ACCOUNT_01)
        .build_and_execute_with_sanity_tests(|| {
            let origin = RawOrigin::Signed(ACCOUNT_01);
            let tag = create_bounded_vec_example(0);
            assert_noop!(
                Buckets::create_tag(origin.into(), DEFAULT_BUCKET_ID, tag),
                Error::<Test>::NotAdmin
            );
            assert_eq!(events().len(), 0);
        });
}

#[test]
fn create_tag_no_bucket() {
    ExtBuilder::default()
        .with_balances(vec![(ACCOUNT_01, DEFAULT_BALANCE)])
        .add_namespace(DEFAULT_NAMESPACE_ID, MetadataMock { unique_plus_1: 10 })
        .add_admin(DEFAULT_BUCKET_ID, ACCOUNT_01)
        .build()
        .execute_with(|| {
            let origin = RawOrigin::Signed(ACCOUNT_01);
            let tag = create_bounded_vec_example(0);
            assert_ok!(Buckets::create_tag(origin.into(), DEFAULT_BUCKET_ID, tag.clone()));
        });
}

#[test]
fn remove_tag() {
    ExtBuilder::default()
        .with_balances(vec![(ACCOUNT_01, StorageFee::get() * 2 + 1)])
        .add_namespace(DEFAULT_NAMESPACE_ID, MetadataMock { unique_plus_1: 10 })
        .add_bucket(DEFAULT_NAMESPACE_ID, DEFAULT_BUCKET_ID, BUCKET_EXAMPLE_UNLOCKED)
        .add_admin(DEFAULT_BUCKET_ID, ACCOUNT_01)
        .add_contributor(DEFAULT_BUCKET_ID, ACCOUNT_01)
        .build_and_execute_with_sanity_tests(|| {
            let user_origin = RawOrigin::Signed(ACCOUNT_01);
            let root_origin = RawOrigin::Root;
            let tag = create_bounded_vec_example(0);
            assert_ok!(Buckets::create_tag(
                user_origin.clone().into(),
                DEFAULT_BUCKET_ID,
                tag.clone()
            ));
            assert!(Buckets::tag_with_id(DEFAULT_BUCKET_ID, tag.clone()).is_some());

            let message = MessageInputOf::<Test> {
                reference: create_bounded_vec_example(0),
                tag: Some(tag.clone()),
                metadata_input: MetadataInputMock { unique: 12 },
            };
            let bucket = Buckets::bucket_with_id(DEFAULT_NAMESPACE_ID, DEFAULT_BUCKET_ID)
                .expect("Bucket should exist");
            let message_id = bucket.get_next_message_id();
            assert_ok!(Buckets::write(
                user_origin.into(),
                DEFAULT_NAMESPACE_ID,
                DEFAULT_BUCKET_ID,
                message
            ));
            assert_eq!(TagMessages::<Test>::get(DEFAULT_BUCKET_ID, &tag), 1);

            assert_ok!(Buckets::force_remove_message(
                root_origin.clone().into(),
                DEFAULT_BUCKET_ID,
                message_id
            ));
            assert_eq!(TagMessages::<Test>::get(DEFAULT_BUCKET_ID, &tag), 0);

            assert_ok!(Buckets::force_remove_tag(
                root_origin.into(),
                DEFAULT_BUCKET_ID,
                tag.clone()
            ));
            assert!(Buckets::tag_with_id(DEFAULT_BUCKET_ID, tag.clone()).is_none());

            assert!(
                events().contains(&crate::Event::TagDeleted { bucket_id: DEFAULT_BUCKET_ID, tag })
            );
        });
}

#[test]
fn remove_tag_dangling_messages() {
    ExtBuilder::default()
        .with_balances(vec![(ACCOUNT_01, StorageFee::get() * 2 + 1)])
        .add_namespace(DEFAULT_NAMESPACE_ID, MetadataMock { unique_plus_1: 10 })
        .add_bucket(DEFAULT_NAMESPACE_ID, DEFAULT_BUCKET_ID, BUCKET_EXAMPLE_UNLOCKED)
        .add_admin(DEFAULT_BUCKET_ID, ACCOUNT_01)
        .add_contributor(DEFAULT_BUCKET_ID, ACCOUNT_01)
        .build_and_execute_with_sanity_tests(|| {
            let user_origin = RawOrigin::Signed(ACCOUNT_01);
            let root_origin = RawOrigin::Root;
            let tag = create_bounded_vec_example(0);
            assert_ok!(Buckets::create_tag(
                user_origin.clone().into(),
                DEFAULT_BUCKET_ID,
                tag.clone()
            ));
            assert!(Buckets::tag_with_id(DEFAULT_BUCKET_ID, tag.clone()).is_some());

            let message = MessageInputOf::<Test> {
                reference: create_bounded_vec_example(0),
                tag: Some(tag.clone()),
                metadata_input: MetadataInputMock { unique: 12 },
            };
            assert_ok!(Buckets::write(
                user_origin.into(),
                DEFAULT_NAMESPACE_ID,
                DEFAULT_BUCKET_ID,
                message
            ));
            assert_eq!(TagMessages::<Test>::get(DEFAULT_BUCKET_ID, &tag), 1);

            assert_noop!(
                Buckets::force_remove_tag(root_origin.into(), DEFAULT_BUCKET_ID, tag.clone()),
                Error::<Test>::DanglingMessages
            );
        });
}
