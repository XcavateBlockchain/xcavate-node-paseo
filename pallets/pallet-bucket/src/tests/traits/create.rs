use frame_support::assert_ok;

use crate::{mock::*, traits::Create};

#[test]
fn create_namespace() {
    ExtBuilder::default().build_and_execute_with_sanity_tests(|| {
        assert_ok!(<Buckets as Create<Test>>::namespace(MetadataMock { unique_plus_1: 10 }, None));
        let namespace = Buckets::namespace_with_id(DEFAULT_NAMESPACE_ID);
        assert!(namespace.is_some());
    })
}

#[test]
fn create_namespace_with_manager() {
    ExtBuilder::default().build_and_execute_with_sanity_tests(|| {
        assert_ok!(<Buckets as Create<Test>>::namespace(
            MetadataMock { unique_plus_1: 10 },
            Some(ACCOUNT_00)
        ));
        let namespace = Buckets::namespace_with_id(DEFAULT_NAMESPACE_ID);
        assert!(namespace.is_some());
        let manager = Buckets::manager_with_id(DEFAULT_NAMESPACE_ID, ACCOUNT_00);
        assert!(manager.is_some());
    })
}

#[test]
fn create_bucket() {
    ExtBuilder::default()
        .add_namespace(DEFAULT_NAMESPACE_ID, MetadataMock { unique_plus_1: 10 })
        .add_manager(DEFAULT_NAMESPACE_ID, ACCOUNT_00)
        .build_and_execute_with_sanity_tests(|| {
            let bucket_id = Buckets::next_bucket_id();
            assert_eq!(bucket_id, 0);
            assert_ok!(<Buckets as Create<Test>>::bucket(
                DEFAULT_NAMESPACE_ID,
                MetadataMock { unique_plus_1: 20 }
            ));
            let bucket = Buckets::bucket_with_id(DEFAULT_NAMESPACE_ID, bucket_id)
                .expect("Bucket should exist");
            assert!(bucket.is_locked());
            assert_eq!(bucket.get_next_message_id(), 0);
            assert_eq!(bucket.metadata.unique_plus_1, 20);
            assert_eq!(Buckets::next_bucket_id(), bucket_id + 1)
        });
}

#[test]
fn create_tag() {
    ExtBuilder::default()
        .with_balances(vec![(ACCOUNT_01, StorageFee::get() + 1)])
        .add_namespace(DEFAULT_NAMESPACE_ID, MetadataMock { unique_plus_1: 10 })
        .add_bucket(DEFAULT_NAMESPACE_ID, DEFAULT_BUCKET_ID, BUCKET_EXAMPLE_UNLOCKED)
        .add_admin(DEFAULT_BUCKET_ID, ACCOUNT_01)
        .build_and_execute_with_sanity_tests(|| {
            let tag = create_bounded_vec_example(0);
            assert_ok!(<Buckets as Create<Test>>::tag(DEFAULT_BUCKET_ID, tag.clone()));
            assert!(Buckets::tag_with_id(DEFAULT_BUCKET_ID, tag.clone()).is_some());
        });
}

// TODO: why there's no create message?
