use crate::{mock::*, traits::Inspect};

#[test]
fn is_manager() {
    ExtBuilder::default()
        .add_namespace(DEFAULT_NAMESPACE_ID, MetadataMock { unique_plus_1: 10 })
        .add_manager(DEFAULT_NAMESPACE_ID, ACCOUNT_00)
        .build_and_execute_with_sanity_tests(|| {
            assert!(<Buckets as Inspect<Test>>::is_manager(&DEFAULT_NAMESPACE_ID, &ACCOUNT_00));
            assert!(!<Buckets as Inspect<Test>>::is_manager(&DEFAULT_NAMESPACE_ID, &ACCOUNT_01));
        });
}

#[test]
fn is_admin() {
    ExtBuilder::default()
        .add_namespace(DEFAULT_NAMESPACE_ID, MetadataMock { unique_plus_1: 10 })
        .add_bucket(DEFAULT_NAMESPACE_ID, DEFAULT_BUCKET_ID, BUCKET_EXAMPLE_LOCKED)
        .add_admin(DEFAULT_BUCKET_ID, ACCOUNT_00)
        .build_and_execute_with_sanity_tests(|| {
            assert!(<Buckets as Inspect<Test>>::is_admin(&DEFAULT_BUCKET_ID, &ACCOUNT_00));
            assert!(!<Buckets as Inspect<Test>>::is_admin(&DEFAULT_BUCKET_ID, &ACCOUNT_01));
        });
}

#[test]
fn is_contributor() {
    ExtBuilder::default()
        .add_namespace(DEFAULT_NAMESPACE_ID, MetadataMock { unique_plus_1: 10 })
        .add_bucket(DEFAULT_NAMESPACE_ID, DEFAULT_BUCKET_ID, BUCKET_EXAMPLE_LOCKED)
        .add_contributor(DEFAULT_BUCKET_ID, ACCOUNT_00)
        .build_and_execute_with_sanity_tests(|| {
            assert!(<Buckets as Inspect<Test>>::is_contributor(&DEFAULT_BUCKET_ID, &ACCOUNT_00));
            assert!(!<Buckets as Inspect<Test>>::is_contributor(&DEFAULT_BUCKET_ID, &ACCOUNT_01));
        });
}

#[test]
fn bucket_details() {
    ExtBuilder::default()
        .add_namespace(DEFAULT_NAMESPACE_ID, MetadataMock { unique_plus_1: 10 })
        .add_bucket(DEFAULT_NAMESPACE_ID, DEFAULT_BUCKET_ID, BUCKET_EXAMPLE_LOCKED)
        .build_and_execute_with_sanity_tests(|| {
            assert_eq!(
                <Buckets as Inspect<Test>>::bucket_details(
                    &DEFAULT_NAMESPACE_ID,
                    &DEFAULT_BUCKET_ID
                )
                .unwrap()
                .metadata,
                BUCKET_EXAMPLE_LOCKED.metadata
            );
            assert!(<Buckets as Inspect<Test>>::bucket_details(
                &DEFAULT_NAMESPACE_ID,
                &(DEFAULT_BUCKET_ID + 1)
            )
            .is_none(),);
        });
}
