# Pallet Bucket Messages

This pallet is a core component of the [DIDComm Vault protocol][didcomm-vault-document].

The pallet bucket message module enables the creation of logical message collections. Its purpose is to provide a decentralized method for collecting and organizing information about entities (such as NFTs or real estate properties) in a logical, transparent, and auditable manner.

Users or the runtime can create namespaces for entities. Within these namespaces, buckets can be created, and messages can be added to those buckets. Messages are references to a storage layer where the content is encrypted.

For more details, please refer to the official [DIDComm Vault specification][didcomm-vault-document].


## Overview

This pallet provides functionality to:

- Create a new namespace for an entity with associated buckets.
- Implement a role-based system where only specific users can update buckets and write messages.
- Provide references to messages associated with an entity.
- Enable a tagging system for messages.
- Support configurable DID origins.
- Execute call hooks for custom validation at the runtime level.


### Key Concepts

#### Namespaces and Buckets

Each entity has its own namespace.  
The manager decides the types of buckets required for the entity. 
Different buckets can be used to address various use cases, such as restricting readers to specific sets of messages. 
The actual documents associated with the entity are stored in the bucket.  


#### Role System

The system defines four roles:

- **Managers**:  
  Managers can create new buckets in a namespace and assign admins to buckets.
  
- **Admins**:  
  Admins can create new tags for a bucket, assign contributors, pause a bucket, and unlock it with a new encryption key.
  
- **Contributors**:  
  Contributors are the only entities allowed to write messages into a bucket.
  
- **Readers**:  
  Readers are not represented on-chain. They have permissions to read the content written by contributors.

#### Metadata Construction

Namespaces, buckets, and messages have metadata attached to them.  
Depending on the use case, metadata may be optional or require a specific format. To maintain flexibility, runtime developers must provide the metadata structures in the configuration.


## Storage Items

- **Namespaces**: Stores details of each entity, including metadata.
- **Buckets**: Stores details of each bucket, including metadata.
- **Messages**: Stores details of each message, including metadata.
- **Managers**: Tracks the manager of each namespace.
- **Admins**: Tracks the admins of each bucket.
- **Contributors**: Tracks the contributors of each bucket.


## Events

- **NamespaceCreated**: A new namespace has been created.
- **ContributorAdded**: A contributor has been assigned to a bucket.
- **ContributorRemoved**: A contributor has been removed from a bucket.
- **AdminAdded**: A new admin has been assigned to a bucket.
- **AdminRemoved**: An admin has been removed from a bucket.
- **ManagerAdded**: A new manager has been assigned to a namespace.
- **ManagerRemoved**: A manager has been removed from a namespace.
- **BucketCreated**: A new bucket has been created.
- **PausedBucket**: A bucket has been paused for writing.
- **BucketWritableWithKey**: A bucket is writable with an encryption key.
- **NewTag**: A new tag has been created.
- **NewMessage**: A new message has been written.
- **NamespaceDeleted**: A namespace has been deleted.
- **BucketDeleted**: A bucket has been deleted.
- **MessageDeleted**: A message has been deleted.
- **SudoNewManager**: A new manager has been assigned via Sudo.


## Errors

- **NamespaceAlreadyExists**: The requested namespace already exists.
- **UnknownNamespace**: The requested namespace does not exist.
- **UnknownBucket**: The bucket does not exist.
- **BucketIsLocked**: The bucket is locked.
- **UnknownMessage**: The requested message does not exist.
- **DanglingBuckets**: There are unlinked buckets for the namespace.
- **DanglingMessages**: There are unlinked messages for the bucket.
- **NotManager**: The origin is not authorized to perform manager actions for the namespace.
- **NotContributor**: The contributor does not exist for the requested bucket.
- **NotAdmin**: The origin is not authorized to perform admin actions for the bucket.
- **UnknownTag**: The specified tag does not exist.
- **UnableToPayFees**: The account cannot pay the required fees.


## Config

- **BucketId**: Identifier type for a bucket.
- **Currency**: Currency type used for fees (e.g., creating namespaces, buckets, and messages).
- **SubjectId**: Identifier type for the subject, typically a DID.
- **FeeCollector**: Handler for unbalanced fees.
- **FeeNamespace**: Fee for creating a namespace.
- **FeeBucket**: Fee for creating a bucket.
- **FeeMessage**: Fee for creating a message.
- **FeeTag**: Fee for creating a tag.
- **NamespaceId**: Identifier type for a namespace.
- **KeyId**: Encryption key type for a bucket.
- **NamespaceMetadataInput**: Input type for namespace metadata.
- **BucketMetadataInput**: Input type for bucket metadata.
- **MessageMetadataInput**: Input type for message metadata.
- **NamespaceMetadata**: Metadata type for a namespace.
- **BucketMetadata**: Metadata type for a bucket.
- **MessageMetadata**: Metadata type for a message.
- **MessageId**: Identifier type for a message.
- **MaxStringInputLengthTag**: Maximum length for string inputs (e.g., tags).
- **ForceOriginCheck**: Origin check for forced actions.
- **OriginCheck**: DID origin check.
- **OriginSuccess**: Type representing successful origin checks.
- **Reference**: Reference to the storage layer where the message is located.
- **RuntimeEvent**: Type of runtime events emitted by the pallet.
- **OnCallHooks**: Hooks executed during each call dispatch for additional verification or actions.
- **WeightInfo**: Type defining weight information for extrinsics.
- **BenchmarkHelper**: Helper type for runtime benchmarks (enabled with the `runtime-benchmarks` feature).


## Trait Implementation

This pallet is designed to be loosely coupled and can be integrated into other pallets.  
It implements traits for creating or deleting namespaces, messages, and buckets, as well as for user management.

[didcomm-vault-document]: https://docs.google.com/document/d/1RJO2OaPJ-MXigxIeNO0CiBGicgmC_srMvSOlZKaNaag/edit?tab=t.0#heading=h.8ve9bbp9t3jq
