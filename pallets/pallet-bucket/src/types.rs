use frame_support::{
    pallet_prelude::RuntimeDebug,
    sp_runtime::{
        traits::{CheckedAdd, One},
        ArithmeticError, BoundedBTreeMap, BoundedVec,
    },
};
use frame_system::pallet_prelude::BlockNumberFor;
use parity_scale_codec::{Decode, DecodeWithMemTracking, Encode, MaxEncodedLen};
use scale_info::TypeInfo;

use crate::{traits::ConstructMetadata, Config};

#[derive(
    Encode,
    Decode,
    MaxEncodedLen,
    TypeInfo,
    PartialEq,
    Eq,
    Clone,
    Copy,
    RuntimeDebug,
    DecodeWithMemTracking,
)]
pub struct BucketPublicKey(pub [u8; 32]);

#[derive(
    Encode,
    Decode,
    MaxEncodedLen,
    TypeInfo,
    PartialEq,
    Eq,
    Clone,
    RuntimeDebug,
    DecodeWithMemTracking,
)]
#[scale_info(skip_type_params(T))]
pub struct NamespaceMetadataInput<T: Config> {
    pub name: BoundedVec<u8, T::MaxNameLen>,
    pub schema_uri: Option<BoundedVec<u8, T::MaxUriLen>>,
    pub properties: BoundedBTreeMap<
        BoundedVec<u8, T::MaxPropertyKeyLen>,
        BoundedVec<u8, T::MaxPropertyValueLen>,
        T::MaxProperties,
    >,
}

#[derive(
    Encode,
    Decode,
    MaxEncodedLen,
    TypeInfo,
    PartialEq,
    Eq,
    Clone,
    RuntimeDebug,
    DecodeWithMemTracking,
)]
#[scale_info(skip_type_params(T))]
pub struct BucketMetadataInput<T: Config> {
    pub name: BoundedVec<u8, T::MaxNameLen>,
    pub category: BoundedVec<u8, T::MaxCategoryLen>,
    pub properties: BoundedBTreeMap<
        BoundedVec<u8, T::MaxPropertyKeyLen>,
        BoundedVec<u8, T::MaxPropertyValueLen>,
        T::MaxProperties,
    >,
}

#[derive(
    Encode,
    Decode,
    MaxEncodedLen,
    TypeInfo,
    PartialEq,
    Eq,
    Clone,
    RuntimeDebug,
    DecodeWithMemTracking,
)]
#[scale_info(skip_type_params(T))]
pub struct MessageMetadataInput<T: Config> {
    pub description: BoundedVec<u8, T::MaxNameLen>,
    pub content_type: BoundedVec<u8, T::MaxCategoryLen>,
    pub content_hash: [u8; 32],
    pub properties: BoundedBTreeMap<
        BoundedVec<u8, T::MaxPropertyKeyLen>,
        BoundedVec<u8, T::MaxPropertyValueLen>,
        T::MaxProperties,
    >,
}

#[derive(
    Encode,
    Decode,
    MaxEncodedLen,
    TypeInfo,
    PartialEq,
    Eq,
    Clone,
    RuntimeDebug,
    DecodeWithMemTracking,
)]
#[scale_info(skip_type_params(T))]
pub struct NamespaceMetadata<T: Config> {
    pub name: BoundedVec<u8, T::MaxNameLen>,
    pub created_at: BlockNumberFor<T>,
    pub schema_uri: Option<BoundedVec<u8, T::MaxUriLen>>,
    pub properties: BoundedBTreeMap<
        BoundedVec<u8, T::MaxPropertyKeyLen>,
        BoundedVec<u8, T::MaxPropertyValueLen>,
        T::MaxProperties,
    >,
}

#[derive(
    Encode,
    Decode,
    MaxEncodedLen,
    TypeInfo,
    PartialEq,
    Eq,
    Clone,
    RuntimeDebug,
    DecodeWithMemTracking,
)]
#[scale_info(skip_type_params(T))]
pub struct BucketMetadata<T: Config> {
    pub name: BoundedVec<u8, T::MaxNameLen>,
    pub created_at: BlockNumberFor<T>,
    pub category: BoundedVec<u8, T::MaxCategoryLen>,
    pub properties: BoundedBTreeMap<
        BoundedVec<u8, T::MaxPropertyKeyLen>,
        BoundedVec<u8, T::MaxPropertyValueLen>,
        T::MaxProperties,
    >,
}

#[derive(
    Encode,
    Decode,
    MaxEncodedLen,
    TypeInfo,
    PartialEq,
    Eq,
    Clone,
    RuntimeDebug,
    DecodeWithMemTracking,
)]
#[scale_info(skip_type_params(T))]
pub struct MessageMetadata<T: Config> {
    pub description: BoundedVec<u8, T::MaxNameLen>,
    pub created_at: BlockNumberFor<T>,
    pub content_type: BoundedVec<u8, T::MaxCategoryLen>,
    pub content_hash: [u8; 32],
    pub properties: BoundedBTreeMap<
        BoundedVec<u8, T::MaxPropertyKeyLen>,
        BoundedVec<u8, T::MaxPropertyValueLen>,
        T::MaxProperties,
    >,
}

impl<T: Config> From<NamespaceMetadataInput<T>> for NamespaceMetadata<T> {
    fn from(input: NamespaceMetadataInput<T>) -> Self {
        Self {
            name: input.name,
            schema_uri: input.schema_uri,
            properties: input.properties,
            created_at: frame_system::Pallet::<T>::block_number(),
        }
    }
}

impl<T: Config> From<BucketMetadataInput<T>> for BucketMetadata<T> {
    fn from(input: BucketMetadataInput<T>) -> Self {
        Self {
            name: input.name,
            category: input.category,
            properties: input.properties,
            created_at: frame_system::Pallet::<T>::block_number(),
        }
    }
}

impl<T: Config> From<MessageMetadataInput<T>> for MessageMetadata<T> {
    fn from(input: MessageMetadataInput<T>) -> Self {
        Self {
            description: input.description,
            content_type: input.content_type,
            content_hash: input.content_hash,
            properties: input.properties,
            created_at: frame_system::Pallet::<T>::block_number(),
        }
    }
}

#[derive(
    Clone,
    Decode,
    DecodeWithMemTracking,
    RuntimeDebug,
    Encode,
    TypeInfo,
    MaxEncodedLen,
    PartialEq,
    Eq,
    Default,
)]
pub enum Status<KeyId> {
    /// Bucket is writable. Admin can lock it.
    Writable(KeyId),
    /// Bucket is locked. Admin can resume writing.
    #[default]
    Locked,
}

#[derive(
    Clone,
    Decode,
    DecodeWithMemTracking,
    RuntimeDebug,
    Encode,
    TypeInfo,
    MaxEncodedLen,
    PartialEq,
    Eq,
)]
pub struct Bucket<Metadata, MessageId, KeyId> {
    /// Metadata of the bucket.
    pub metadata: Metadata,
    /// Status of the bucket.
    pub status: Status<KeyId>,
    /// Next message id.
    pub next_message_id: MessageId,
}

impl<Metadata, MessageId, KeyId> Bucket<Metadata, MessageId, KeyId>
where
    MessageId: CheckedAdd + One + Default + Clone,
{
    pub fn new(metadata: Metadata) -> Self {
        Self { metadata, status: Status::default(), next_message_id: MessageId::default() }
    }

    pub fn lock(&mut self) {
        self.status = Status::Locked;
    }

    pub fn set_writable(&mut self, key: KeyId) {
        self.status = Status::Writable(key);
    }

    pub fn is_locked(&self) -> bool {
        matches!(self.status, Status::Locked)
    }

    pub fn is_writable(&self) -> bool {
        matches!(self.status, Status::Writable(_))
    }

    pub fn get_next_message_id(&self) -> MessageId {
        self.next_message_id.clone()
    }

    pub fn increment_message_id(&mut self) -> Result<(), ArithmeticError> {
        self.next_message_id =
            self.next_message_id.checked_add(&MessageId::one()).ok_or(ArithmeticError::Overflow)?;
        Ok(())
    }
}

#[derive(
    Clone, Decode, DecodeWithMemTracking, RuntimeDebug, Encode, TypeInfo, MaxEncodedLen, PartialEq,
)]
pub struct MessageInput<Tag, Reference, Metadata> {
    /// Unique reference of the message to the storage layer
    pub(crate) reference: Reference,
    /// Tag of the message.
    pub(crate) tag: Option<Tag>,
    /// Metadata of the message.
    pub(crate) metadata_input: Metadata,
}

#[derive(
    Clone,
    Decode,
    DecodeWithMemTracking,
    RuntimeDebug,
    Encode,
    TypeInfo,
    MaxEncodedLen,
    PartialEq,
    Eq,
)]
pub struct Message<Reference, Tag, Metadata> {
    /// Unique reference of the message to the storage layer
    pub reference: Reference,
    /// Tag of the message.
    pub tag: Option<Tag>,
    /// Metadata of the message.
    pub metadata: Metadata,
}

impl<Reference, Tag, Metadata> Message<Reference, Tag, Metadata> {
    pub fn new(reference: Reference, tag: Option<Tag>, metadata: Metadata) -> Self {
        Self { reference, tag, metadata }
    }

    pub fn from_message_input<MetadataInput, CallOrigin>(
        source: MessageInput<Tag, Reference, MetadataInput>,
        origin: &CallOrigin,
    ) -> Result<Self, Metadata::Error>
    where
        Metadata: ConstructMetadata<MetadataInput, CallOrigin>,
    {
        let metadata = Metadata::construct(source.metadata_input, origin)?;
        Ok(Message::new(source.reference, source.tag, metadata))
    }
}
