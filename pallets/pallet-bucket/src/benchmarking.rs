use frame_benchmarking::v2::*;
use frame_support::traits::EnsureOrigin;
use frame_system::RawOrigin;

use crate::{
    types::Status, Buckets, Call, Config, Managers, MessageMetadataInputOf, Messages, Namespaces,
    NextBucketId, SubjectIdOf, TagOf, Tags,
};

/// Benchmark helper trait for generating worst-case scenarios for benchmarking.
pub trait BenchmarkHelper<T: Config> {
    /// Create a new origin for benchmarking.
    fn create_origin(seed: u32) -> T::RuntimeOrigin;
    /// Create a new force origin for benchmarking.
    fn create_force_origin(seed: u32) -> T::RuntimeOrigin;
    /// Generate a new namespace_id, metadata input and the resulting metadata.
    fn get_namespace(
        seed: u32,
    ) -> (T::NamespaceId, T::NamespaceMetadataInput, T::NamespaceMetadata);
    /// Generate a new bucket_id, metadata input and the resulting metadata.
    fn get_bucket(seed: u32) -> (T::BucketId, T::BucketMetadataInput, T::BucketMetadata);
    /// Generate a new message reference, metadata input and the resulting metadata.
    fn get_message(seed: u32) -> (T::Reference, MessageMetadataInputOf<T>, T::MessageMetadata);
    /// Generate a new key_id.
    fn get_key_id(seed: u32) -> T::KeyId;
}

impl<T: Config> BenchmarkHelper<T> for ()
where
    T::NamespaceId: Default,
    T::NamespaceMetadataInput: Default,
    T::NamespaceMetadata: Default,
    T::BucketId: Default,
    T::BucketMetadataInput: Default,
    T::BucketMetadata: Default,
    T::Reference: Default,
    T::MessageMetadataInput: Default,
    T::MessageMetadata: Default,
    T::KeyId: Default,
{
    fn create_origin(_seed: u32) -> T::RuntimeOrigin {
        RawOrigin::None.into()
    }

    fn create_force_origin(_seed: u32) -> T::RuntimeOrigin {
        RawOrigin::Root.into()
    }

    fn get_namespace(
        _seed: u32,
    ) -> (T::NamespaceId, T::NamespaceMetadataInput, T::NamespaceMetadata) {
        (Default::default(), Default::default(), Default::default())
    }

    fn get_bucket(_seed: u32) -> (T::BucketId, T::BucketMetadataInput, T::BucketMetadata) {
        (Default::default(), Default::default(), Default::default())
    }

    fn get_message(_seed: u32) -> (T::Reference, MessageMetadataInputOf<T>, T::MessageMetadata) {
        (Default::default(), Default::default(), Default::default())
    }

    fn get_key_id(_seed: u32) -> T::KeyId {
        Default::default()
    }
}

#[benchmarks]
mod benchmarks {
    use frame_support::BoundedVec;
    use parity_scale_codec::MaxEncodedLen;
    use scale_info::prelude::vec;

    use super::*;
    use crate::{pallet::Pallet, traits::CallSources, types::MessageInput};

    fn get_success_origin<T: Config>(origin: T::RuntimeOrigin) -> T::OriginSuccess {
        T::OriginCheck::ensure_origin(origin).expect("Expected non-force origin")
    }

    fn has_max_length<T: MaxEncodedLen>(input: &T) -> bool {
        T::max_encoded_len() == input.encode().len()
    }

    fn get_key_id<T: Config>(seed: u32) -> T::KeyId {
        let key_id = T::BenchmarkHelper::get_key_id(seed);

        assert!(
            has_max_length(&key_id),
            "BenchmarkHelper::get_key_id() must produce worst-case (maximum length) ids."
        );

        key_id
    }

    fn get_bucket_data<T: Config>(
        seed: u32,
    ) -> (T::BucketId, T::BucketMetadataInput, T::BucketMetadata) {
        let (bucket_id, metadata_input, metadata) = T::BenchmarkHelper::get_bucket(seed);

        assert!(
            has_max_length(&bucket_id),
            "BenchmarkHelper::get_bucket() must produce worst-case (maximum length) ids."
        );
        assert!(
            has_max_length(&metadata_input),
            "BenchmarkHelper::get_bucket() must produce worst-case (maximum length) metadata."
        );

        (bucket_id, metadata_input, metadata)
    }

    fn get_namespace_data<T: Config>(
        seed: u32,
    ) -> (T::NamespaceId, T::NamespaceMetadataInput, T::NamespaceMetadata) {
        let (namespace_id, metadata_input, metadata) = T::BenchmarkHelper::get_namespace(seed);

        assert!(
            has_max_length(&namespace_id),
            "BenchmarkHelper::get_namespace() must produce worst-case (maximum length) ids."
        );
        assert!(
            has_max_length(&metadata_input),
            "BenchmarkHelper::get_namespace() must produce worst-case (maximum length) metadata."
        );

        (namespace_id, metadata_input, metadata)
    }

    fn setup_namespace<T: Config>(origin: T::RuntimeOrigin, seed: u32) -> T::NamespaceId {
        let (namespace_id, metadata_input, _) = get_namespace_data::<T>(seed);
        Pallet::<T>::create_namespace(origin, metadata_input).expect("Failed to create namespace");

        namespace_id
    }

    fn setup_bucket<T: Config>(
        origin: T::RuntimeOrigin,
        namespace: T::NamespaceId,
        seed: u32,
        with_admin: Option<T::SubjectId>,
        status: Status<T::KeyId>,
    ) -> T::BucketId {
        let (bucket_id, metadata_input, _) = get_bucket_data::<T>(seed);

        NextBucketId::<T>::set(bucket_id.clone());
        Pallet::<T>::create_bucket(origin.clone(), namespace.clone(), metadata_input)
            .expect("Failed to create bucket");

        Buckets::<T>::mutate(&namespace, &bucket_id, |details| {
            details.as_mut().unwrap().status = status.clone()
        });

        if let Some(admin) = with_admin {
            Pallet::<T>::add_admin(origin, namespace, bucket_id.clone(), admin)
                .expect("failed to add admin")
        };

        bucket_id
    }

    #[benchmark]
    fn create_namespace() {
        let caller: T::RuntimeOrigin = T::BenchmarkHelper::create_origin(0);
        let (namespace_id, metadata_input, metadata) = get_namespace_data::<T>(0);

        #[extrinsic_call]
        create_namespace(caller as T::RuntimeOrigin, metadata_input);

        assert_eq!(Namespaces::<T>::get(&namespace_id), Some(metadata));
    }

    #[benchmark]
    fn add_manager() {
        let caller: T::RuntimeOrigin = T::BenchmarkHelper::create_origin(0);
        let namespace_id = setup_namespace::<T>(caller.clone(), 0);
        let new_manager: SubjectIdOf<T> =
            get_success_origin::<T>(T::BenchmarkHelper::create_origin(1)).subject();

        assert_eq!(Managers::<T>::get(&namespace_id, &new_manager), None);

        let namespace_id_param = namespace_id.clone();
        let new_manager_param = new_manager.clone();

        #[extrinsic_call]
        add_manager(caller as T::RuntimeOrigin, namespace_id_param, new_manager_param);

        assert_eq!(Managers::<T>::get(&namespace_id, &new_manager), Some(()));
    }

    #[benchmark]
    fn force_add_manager() {
        let creator: T::RuntimeOrigin = T::BenchmarkHelper::create_origin(0);
        let force_origin: T::RuntimeOrigin = T::BenchmarkHelper::create_force_origin(1);
        let new_manager: SubjectIdOf<T> =
            get_success_origin::<T>(T::BenchmarkHelper::create_origin(2)).subject();
        let namespace_id = setup_namespace::<T>(creator.clone(), 0);

        assert_eq!(Managers::<T>::get(&namespace_id, &new_manager), None);

        let namespace_id_param = namespace_id.clone();
        let new_manager_param = new_manager.clone();

        #[extrinsic_call]
        force_add_manager(force_origin as T::RuntimeOrigin, namespace_id_param, new_manager_param);

        assert_eq!(Managers::<T>::get(&namespace_id, &new_manager), Some(()));
    }

    #[benchmark]
    fn force_remove_namespace() {
        let caller = T::BenchmarkHelper::create_origin(0);
        let force_origin: T::RuntimeOrigin = T::BenchmarkHelper::create_force_origin(0);
        let namespace_id = setup_namespace::<T>(caller.clone(), 0);

        // remove managers if any
        assert_eq!(Managers::<T>::clear_prefix(&namespace_id, u32::MAX, None).maybe_cursor, None);

        let namespace_id_param = namespace_id.clone();
        #[extrinsic_call]
        force_remove_namespace(force_origin as T::RuntimeOrigin, namespace_id_param);

        assert_eq!(Namespaces::<T>::get(&namespace_id), None);
    }

    #[benchmark]
    fn remove_manager() {
        let caller: T::RuntimeOrigin = T::BenchmarkHelper::create_origin(0);
        let namespace_id = setup_namespace::<T>(caller.clone(), 0);
        let manager: SubjectIdOf<T> = get_success_origin::<T>(caller.clone()).subject();

        assert_eq!(Managers::<T>::get(&namespace_id, &manager), Some(()));

        let namespace_id_param = namespace_id.clone();
        let manager_param = manager.clone();

        let new_manager: SubjectIdOf<T> =
            get_success_origin::<T>(T::BenchmarkHelper::create_origin(1)).subject();

        assert_eq!(Managers::<T>::get(&namespace_id, &new_manager), None);
        let new_manager_param = new_manager.clone();
        Pallet::<T>::add_manager(
            caller.clone() as T::RuntimeOrigin,
            namespace_id_param.clone(),
            new_manager_param,
        )
        .expect("Failed to add manager");

        #[extrinsic_call]
        remove_manager(caller as T::RuntimeOrigin, namespace_id_param, manager_param);

        assert_eq!(Managers::<T>::get(&namespace_id, &manager), None);
    }

    #[benchmark]
    fn create_bucket() {
        let caller: T::RuntimeOrigin = T::BenchmarkHelper::create_origin(0);
        let namespace_id = setup_namespace::<T>(caller.clone(), 0);
        let (bucket_id, bucket_metadata_input, metadata) = get_bucket_data::<T>(0);

        NextBucketId::<T>::set(bucket_id.clone());

        let namespace_id_param = namespace_id.clone();
        #[extrinsic_call]
        create_bucket(caller as T::RuntimeOrigin, namespace_id_param, bucket_metadata_input);

        assert_eq!(
            Buckets::<T>::get(&namespace_id, &bucket_id).map(|bucket| bucket.metadata),
            Some(metadata)
        );
    }

    #[benchmark]
    fn force_remove_bucket() {
        let caller = T::BenchmarkHelper::create_origin(0);
        let force_origin: T::RuntimeOrigin = T::BenchmarkHelper::create_force_origin(0);
        let namespace_id = setup_namespace::<T>(caller.clone(), 0);
        let bucket_id =
            setup_bucket::<T>(caller.clone(), namespace_id.clone(), 0, None, Status::Locked);

        let namespace_id_param = namespace_id.clone();
        let bucket_id_param = bucket_id.clone();
        #[extrinsic_call]
        force_remove_bucket(force_origin as T::RuntimeOrigin, namespace_id_param, bucket_id_param);

        assert_eq!(Buckets::<T>::contains_key(&namespace_id, &bucket_id), false);
    }

    #[benchmark]
    fn force_remove_tag() {
        let caller = T::BenchmarkHelper::create_origin(0);
        let force_origin: T::RuntimeOrigin = T::BenchmarkHelper::create_force_origin(0);
        let namespace_id = setup_namespace::<T>(caller.clone(), 0);
        let admin: SubjectIdOf<T> = get_success_origin::<T>(caller.clone()).subject();
        let bucket_id =
            setup_bucket::<T>(caller.clone(), namespace_id.clone(), 0, Some(admin), Status::Locked);

        let bucket_id_param = bucket_id.clone();
        let tag: TagOf<T> = BoundedVec::truncate_from(vec![1; TagOf::<T>::bound()]);
        Pallet::<T>::create_tag(caller.clone(), bucket_id_param.clone(), tag.clone()).unwrap();

        #[extrinsic_call]
        force_remove_tag(force_origin as T::RuntimeOrigin, bucket_id_param.clone(), tag.clone());

        assert!(!Tags::<T>::contains_key(&bucket_id_param, &tag));
    }

    #[benchmark]
    fn add_admin() {
        let caller: T::RuntimeOrigin = T::BenchmarkHelper::create_origin(0);
        let subject: SubjectIdOf<T> = get_success_origin::<T>(caller.clone()).subject();
        let namespace_id = setup_namespace::<T>(caller.clone(), 0);
        let bucket_id =
            setup_bucket::<T>(caller.clone(), namespace_id.clone(), 0, None, Status::Locked);

        assert_eq!(Pallet::<T>::is_admin(&bucket_id, &subject), false);

        let bucket_id_param = bucket_id.clone();
        let subject_param = subject.clone();
        #[extrinsic_call]
        add_admin(caller as T::RuntimeOrigin, namespace_id, bucket_id_param, subject_param);

        assert_eq!(Pallet::<T>::is_admin(&bucket_id, &subject), true);
    }

    #[benchmark]
    fn remove_admin() {
        let caller: T::RuntimeOrigin = T::BenchmarkHelper::create_origin(0);
        let subject: SubjectIdOf<T> = get_success_origin::<T>(caller.clone()).subject();
        let namespace_id = setup_namespace::<T>(caller.clone(), 0);
        let bucket_id = setup_bucket::<T>(
            caller.clone(),
            namespace_id.clone(),
            0,
            Some(subject.clone()),
            Status::Locked,
        );

        assert_eq!(Pallet::<T>::is_admin(&bucket_id, &subject), true);

        let bucket_id_param = bucket_id.clone();
        let subject_param = subject.clone();
        #[extrinsic_call]
        remove_admin(caller as T::RuntimeOrigin, namespace_id, bucket_id_param, subject_param);

        assert_eq!(Pallet::<T>::is_admin(&bucket_id, &subject), false);
    }

    #[benchmark]
    fn add_contributor() {
        let caller: T::RuntimeOrigin = T::BenchmarkHelper::create_origin(0);
        let contributor: SubjectIdOf<T> = get_success_origin::<T>(caller.clone()).subject();
        let namespace_id = setup_namespace::<T>(caller.clone(), 0);
        let bucket_id = setup_bucket::<T>(
            caller.clone(),
            namespace_id.clone(),
            0,
            Some(contributor.clone()),
            Status::Locked,
        );

        assert_eq!(Pallet::<T>::is_contributor(&bucket_id, &contributor), false);

        let bucket_id_param = bucket_id.clone();
        let contributor_param = contributor.clone();
        #[extrinsic_call]
        add_contributor(
            caller as T::RuntimeOrigin,
            namespace_id,
            bucket_id_param,
            contributor_param,
        );

        assert_eq!(Pallet::<T>::is_contributor(&bucket_id, &contributor), true);
    }

    #[benchmark]
    fn remove_contributor() {
        let caller: T::RuntimeOrigin = T::BenchmarkHelper::create_origin(0);
        let contributor: SubjectIdOf<T> = get_success_origin::<T>(caller.clone()).subject();
        let namespace_id = setup_namespace::<T>(caller.clone(), 0);
        let bucket_id = setup_bucket::<T>(
            caller.clone(),
            namespace_id.clone(),
            0,
            Some(contributor.clone()),
            Status::Locked,
        );

        Pallet::<T>::add_contributor(
            caller.clone(),
            namespace_id.clone(),
            bucket_id.clone(),
            contributor.clone(),
        )
        .unwrap();

        assert_eq!(Pallet::<T>::is_contributor(&bucket_id, &contributor), true);

        let bucket_id_param = bucket_id.clone();
        let contributor_param = contributor.clone();
        #[extrinsic_call]
        remove_contributor(
            caller as T::RuntimeOrigin,
            namespace_id,
            bucket_id_param,
            contributor_param,
        );

        assert_eq!(Pallet::<T>::is_contributor(&bucket_id, &contributor), false);
    }

    #[benchmark]
    fn resume_writing() {
        let caller: T::RuntimeOrigin = T::BenchmarkHelper::create_origin(0);
        let admin: SubjectIdOf<T> = get_success_origin::<T>(caller.clone()).subject();
        let namespace_id = setup_namespace::<T>(caller.clone(), 0);
        let bucket_id =
            setup_bucket::<T>(caller.clone(), namespace_id.clone(), 0, Some(admin), Status::Locked);

        assert!(matches!(
            Pallet::<T>::bucket_with_id(&namespace_id, &bucket_id).map(|bucket| bucket.status),
            Some(Status::Locked)
        ));

        let key_id = get_key_id::<T>(0);

        let bucket_id_param = bucket_id.clone();
        let namespace_id_param = namespace_id.clone();
        #[extrinsic_call]
        resume_writing(caller as T::RuntimeOrigin, namespace_id_param, bucket_id_param, key_id);

        assert!(matches!(
            Pallet::<T>::bucket_with_id(&namespace_id, &bucket_id).map(|bucket| bucket.status),
            Some(Status::Writable(_))
        ));
    }

    #[benchmark]
    fn pause_writing() {
        let caller: T::RuntimeOrigin = T::BenchmarkHelper::create_origin(0);
        let admin: SubjectIdOf<T> = get_success_origin::<T>(caller.clone()).subject();
        let namespace_id = setup_namespace::<T>(caller.clone(), 0);

        let key_id = get_key_id::<T>(0);
        let bucket_id = setup_bucket::<T>(
            caller.clone(),
            namespace_id.clone(),
            0,
            Some(admin),
            Status::Writable(key_id.clone()),
        );

        let bucket_id_param = bucket_id.clone();
        let namespace_id_param = namespace_id.clone();
        #[extrinsic_call]
        pause_writing(caller as T::RuntimeOrigin, namespace_id_param, bucket_id_param);

        assert!(matches!(
            Pallet::<T>::bucket_with_id(&namespace_id, &bucket_id).map(|bucket| bucket.status),
            Some(Status::Locked)
        ));
    }

    #[benchmark]
    fn rotate_key() {
        let caller: T::RuntimeOrigin = T::BenchmarkHelper::create_origin(0);
        let admin: SubjectIdOf<T> = get_success_origin::<T>(caller.clone()).subject();
        let namespace_id = setup_namespace::<T>(caller.clone(), 0);
        let key_id = get_key_id::<T>(0);
        let bucket_id = setup_bucket::<T>(
            caller.clone(),
            namespace_id.clone(),
            0,
            Some(admin),
            Status::Writable(key_id.clone()),
        );

        let new_key_id = get_key_id::<T>(1);

        let bucket_id_param = bucket_id.clone();
        let namespace_id_param = namespace_id.clone();
        #[extrinsic_call]
        rotate_key(caller as T::RuntimeOrigin, namespace_id_param, bucket_id_param, new_key_id);

        assert!(matches!(
            Pallet::<T>::bucket_with_id(&namespace_id, &bucket_id).map(|bucket| bucket.status),
            Some(Status::Writable(_))
        ));
    }

    #[benchmark]
    fn create_tag() {
        let caller: T::RuntimeOrigin = T::BenchmarkHelper::create_origin(0);
        let admin: SubjectIdOf<T> = get_success_origin::<T>(caller.clone()).subject();
        let namespace_id = setup_namespace::<T>(caller.clone(), 0);
        let bucket_id =
            setup_bucket::<T>(caller.clone(), namespace_id.clone(), 0, Some(admin), Status::Locked);

        let tag: TagOf<T> = BoundedVec::truncate_from(vec![1; TagOf::<T>::bound()]);

        assert!(!Tags::<T>::contains_key(&bucket_id, &tag));

        let bucket_id_param = bucket_id.clone();
        let tag_param = tag.clone();
        #[extrinsic_call]
        create_tag(caller as T::RuntimeOrigin, bucket_id_param, tag_param);

        assert!(Tags::<T>::contains_key(&bucket_id, &tag));
    }

    #[benchmark]
    fn write() {
        let caller: T::RuntimeOrigin = T::BenchmarkHelper::create_origin(0);
        let contributor: SubjectIdOf<T> = get_success_origin::<T>(caller.clone()).subject();
        let namespace_id = setup_namespace::<T>(caller.clone(), 0);
        let key_id = get_key_id::<T>(0);
        let bucket_id = setup_bucket::<T>(
            caller.clone(),
            namespace_id.clone(),
            0,
            Some(contributor.clone()),
            Status::Writable(key_id),
        );

        Pallet::<T>::add_contributor(
            caller.clone(),
            namespace_id.clone(),
            bucket_id.clone(),
            contributor.clone(),
        )
        .unwrap();

        let tag: TagOf<T> = BoundedVec::truncate_from(vec![1; TagOf::<T>::bound()]);

        Pallet::<T>::create_tag(caller.clone(), bucket_id.clone(), tag.clone()).unwrap();

        let (reference, metadata_input, _) = T::BenchmarkHelper::get_message(0);

        let message_data = MessageInput { reference, tag: Some(tag), metadata_input };

        assert!(
            has_max_length(&message_data),
            "BenchmarkHelper::get_message() must produce worst-case (maximum length) reference \
             and metadata"
        );

        let message_id: T::MessageId = Default::default();

        assert!(!Messages::<T>::contains_key(&bucket_id, &message_id));

        let bucket_id_param = bucket_id.clone();
        #[extrinsic_call]
        write(caller as T::RuntimeOrigin, namespace_id, bucket_id_param, message_data);

        assert!(Messages::<T>::contains_key(&bucket_id, &message_id));
    }

    #[benchmark]
    fn force_remove_message() {
        let caller = T::BenchmarkHelper::create_origin(0);
        let force_origin: T::RuntimeOrigin = T::BenchmarkHelper::create_force_origin(0);
        let contributor: SubjectIdOf<T> = get_success_origin::<T>(caller.clone()).subject();
        let namespace_id = setup_namespace::<T>(caller.clone(), 0);
        let key_id = get_key_id::<T>(0);
        let bucket_id = setup_bucket::<T>(
            caller.clone(),
            namespace_id.clone(),
            0,
            Some(contributor.clone()),
            Status::Writable(key_id),
        );

        Pallet::<T>::add_contributor(
            caller.clone(),
            namespace_id.clone(),
            bucket_id.clone(),
            contributor.clone(),
        )
        .unwrap();

        let tag: TagOf<T> = BoundedVec::truncate_from(vec![1; TagOf::<T>::bound()]);

        Pallet::<T>::create_tag(caller.clone(), bucket_id.clone(), tag.clone()).unwrap();

        let (reference, metadata_input, _) = T::BenchmarkHelper::get_message(0);

        let message_data = MessageInput { reference, tag: Some(tag), metadata_input };

        Pallet::<T>::write(caller.clone(), namespace_id.clone(), bucket_id.clone(), message_data)
            .unwrap();

        let message_id: T::MessageId = Default::default();
        assert!(Messages::<T>::contains_key(&bucket_id, &message_id));

        let bucket_id_param = bucket_id.clone();
        let message_id_param = message_id.clone();
        #[extrinsic_call]
        force_remove_message(force_origin as T::RuntimeOrigin, bucket_id_param, message_id_param);

        assert!(!Messages::<T>::contains_key(&bucket_id, &message_id));
    }

    impl_benchmark_test_suite!(
        Pallet,
        crate::mock::ExtBuilder::default().build_with_keystore(),
        crate::mock::Test
    );
}
