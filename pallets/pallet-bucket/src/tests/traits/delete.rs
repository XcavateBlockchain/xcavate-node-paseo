use frame_support::assert_ok;

use crate::{mock::*, traits::Delete, Messages};

const MESSAGE_ID: u128 = 10;

#[test]
fn delete_namespace() {
    ExtBuilder::default()
        .add_namespace(DEFAULT_NAMESPACE_ID, MetadataMock { unique_plus_1: 10 })
        .build_and_execute_with_sanity_tests(|| {
            assert!(Buckets::namespace_with_id(DEFAULT_NAMESPACE_ID).is_some());
            assert_ok!(<Buckets as Delete<Test>>::namespace(DEFAULT_NAMESPACE_ID));
            assert!(Buckets::namespace_with_id(DEFAULT_NAMESPACE_ID).is_none());
        });
}

#[test]
fn remove_bucket() {
    ExtBuilder::default()
        .add_namespace(DEFAULT_NAMESPACE_ID, MetadataMock { unique_plus_1: 10 })
        .add_manager(DEFAULT_NAMESPACE_ID, ACCOUNT_00)
        .add_bucket(DEFAULT_NAMESPACE_ID, DEFAULT_BUCKET_ID, BUCKET_EXAMPLE_LOCKED)
        .build_and_execute_with_sanity_tests(|| {
            assert!(Buckets::bucket_with_id(DEFAULT_NAMESPACE_ID, DEFAULT_BUCKET_ID).is_some());
            assert_ok!(<Buckets as Delete<Test>>::bucket(DEFAULT_NAMESPACE_ID, DEFAULT_BUCKET_ID));
            assert!(Buckets::bucket_with_id(DEFAULT_NAMESPACE_ID, DEFAULT_BUCKET_ID).is_none());
        });
}

#[test]
fn remove_message() {
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
            assert!(Messages::<Test>::get(DEFAULT_BUCKET_ID, MESSAGE_ID).is_some());
            assert_ok!(<Buckets as Delete<Test>>::message(DEFAULT_BUCKET_ID, MESSAGE_ID));
            assert!(Messages::<Test>::get(DEFAULT_BUCKET_ID, MESSAGE_ID).is_none());
        });
}
