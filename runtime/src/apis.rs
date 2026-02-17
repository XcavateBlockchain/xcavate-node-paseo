#[cfg(feature = "runtime-benchmarks")]
use alloc::boxed::Box;
use alloc::vec::Vec;

use cumulus_pallet_parachain_system::RelayChainState;
use frame_support::{
    genesis_builder_helper::{build_state, get_preset},
    weights::Weight,
};
use ismp::{
    consensus::{ConsensusClientId, StateMachineHeight, StateMachineId},
    host::StateMachine,
};
use kilt_runtime_api_did::RawDidLinkedInfo;
use kilt_support::traits::ItemFilter;
use pallet_did_lookup::linkable_account::LinkableAccountId;
use parity_scale_codec::{Decode, Encode};
use public_credentials::CredentialEntry;
use scale_info::TypeInfo;
use sp_api::impl_runtime_apis;
use sp_consensus_aura::sr25519::AuthorityId as AuraId;
use sp_core::{crypto::KeyTypeId, OpaqueMetadata, H256};
use sp_runtime::{
    traits::Block as BlockT,
    transaction_validity::{TransactionSource, TransactionValidity},
    ApplyExtrinsicResult,
};
use sp_version::RuntimeVersion;

use crate::{
    assets::{AssetDid, PublicCredentialsFilter},
    authorization::AuthorizationId,
    configs::DidIdentifier,
    constants::{SLOT_DURATION, VERSION},
    types::{AccountId, Balance, Block, BlockNumber, ConsensusHook, Executive, Hash, Nonce},
    InherentDataExt, IsmpParachain, ParachainSystem, Runtime, RuntimeCall, RuntimeGenesisConfig,
    SessionKeys, System, TransactionPayment,
};

#[derive(Encode, Decode, TypeInfo)]
pub enum PublicCredentialsApiError {
    InvalidSubjectId,
}

impl_runtime_apis! {
    impl sp_consensus_aura::AuraApi<Block, AuraId> for Runtime {
        fn slot_duration() -> sp_consensus_aura::SlotDuration {
            sp_consensus_aura::SlotDuration::from_millis(SLOT_DURATION)
        }

        fn authorities() -> Vec<AuraId> {
            pallet_aura::Authorities::<Runtime>::get().into_inner()
        }
    }

    impl sp_api::Core<Block> for Runtime {
        fn version() -> RuntimeVersion {
            VERSION
        }

        fn execute_block(block: Block) {
            Executive::execute_block(block)
        }

        fn initialize_block(header: &<Block as BlockT>::Header) -> sp_runtime::ExtrinsicInclusionMode {
            Executive::initialize_block(header)
        }
    }

    impl sp_api::Metadata<Block> for Runtime {
        fn metadata() -> OpaqueMetadata {
            OpaqueMetadata::new(Runtime::metadata().into())
        }

        fn metadata_at_version(version: u32) -> Option<OpaqueMetadata> {
            Runtime::metadata_at_version(version)
        }

        fn metadata_versions() -> Vec<u32> {
            Runtime::metadata_versions()
        }
    }

    impl sp_block_builder::BlockBuilder<Block> for Runtime {
        fn apply_extrinsic(extrinsic: <Block as BlockT>::Extrinsic) -> ApplyExtrinsicResult {
            Executive::apply_extrinsic(extrinsic)
        }

        fn finalize_block() -> <Block as BlockT>::Header {
            Executive::finalize_block()
        }

        fn inherent_extrinsics(data: sp_inherents::InherentData) -> Vec<<Block as BlockT>::Extrinsic> {
            data.create_extrinsics()
        }

        fn check_inherents(
            block: Block,
            data: sp_inherents::InherentData,
        ) -> sp_inherents::CheckInherentsResult {
            data.check_extrinsics(&block)
        }
    }

    impl sp_transaction_pool::runtime_api::TaggedTransactionQueue<Block> for Runtime {
        fn validate_transaction(
            source: TransactionSource,
            tx: <Block as BlockT>::Extrinsic,
            block_hash: <Block as BlockT>::Hash,
        ) -> TransactionValidity {
            Executive::validate_transaction(source, tx, block_hash)
        }
    }

    impl sp_offchain::OffchainWorkerApi<Block> for Runtime {
        fn offchain_worker(header: &<Block as BlockT>::Header) {
            Executive::offchain_worker(header)
        }
    }

    impl sp_session::SessionKeys<Block> for Runtime {
        fn generate_session_keys(seed: Option<Vec<u8>>) -> Vec<u8> {
            SessionKeys::generate(seed)
        }

        fn decode_session_keys(
            encoded: Vec<u8>,
        ) -> Option<Vec<(Vec<u8>, KeyTypeId)>> {
            SessionKeys::decode_into_raw_public_keys(&encoded)
        }
    }

    impl frame_system_rpc_runtime_api::AccountNonceApi<Block, AccountId, Nonce> for Runtime {
        fn account_nonce(account: AccountId) -> Nonce {
            System::account_nonce(account)
        }
    }

    impl pallet_transaction_payment_rpc_runtime_api::TransactionPaymentApi<Block, Balance> for Runtime {
        fn query_info(
            uxt: <Block as BlockT>::Extrinsic,
            len: u32,
        ) -> pallet_transaction_payment_rpc_runtime_api::RuntimeDispatchInfo<Balance> {
            TransactionPayment::query_info(uxt, len)
        }
        fn query_fee_details(
            uxt: <Block as BlockT>::Extrinsic,
            len: u32,
        ) -> pallet_transaction_payment::FeeDetails<Balance> {
            TransactionPayment::query_fee_details(uxt, len)
        }
        fn query_weight_to_fee(weight: Weight) -> Balance {
            TransactionPayment::weight_to_fee(weight)
        }
        fn query_length_to_fee(length: u32) -> Balance {
            TransactionPayment::length_to_fee(length)
        }
    }

    impl pallet_transaction_payment_rpc_runtime_api::TransactionPaymentCallApi<Block, Balance, RuntimeCall>
        for Runtime
    {
        fn query_call_info(
            call: RuntimeCall,
            len: u32,
        ) -> pallet_transaction_payment::RuntimeDispatchInfo<Balance> {
            TransactionPayment::query_call_info(call, len)
        }
        fn query_call_fee_details(
            call: RuntimeCall,
            len: u32,
        ) -> pallet_transaction_payment::FeeDetails<Balance> {
            TransactionPayment::query_call_fee_details(call, len)
        }
        fn query_weight_to_fee(weight: Weight) -> Balance {
            TransactionPayment::weight_to_fee(weight)
        }
        fn query_length_to_fee(length: u32) -> Balance {
            TransactionPayment::length_to_fee(length)
        }
    }

    impl cumulus_primitives_core::CollectCollationInfo<Block> for Runtime {
        fn collect_collation_info(header: &<Block as BlockT>::Header) -> cumulus_primitives_core::CollationInfo {
            ParachainSystem::collect_collation_info(header)
        }
    }

    impl cumulus_primitives_aura::AuraUnincludedSegmentApi<Block> for Runtime {
        fn can_build_upon(
            included_hash: <Block as BlockT>::Hash,
            slot: cumulus_primitives_aura::Slot
        ) -> bool {
            ConsensusHook::can_build_upon(included_hash, slot)
        }
    }

    // ISMP.
    impl pallet_ismp_runtime_api::IsmpRuntimeApi<Block, <Block as BlockT>::Hash> for Runtime {
        /// Returns the host's state machine identifier.
        fn host_state_machine() -> StateMachine {
            <Runtime as pallet_ismp::Config>::HostStateMachine::get()
        }

        /// Return the challenge period timestamp.
        fn challenge_period(state_machine_id: StateMachineId) -> Option<u64> {
            pallet_ismp::Pallet::<Runtime>::challenge_period(state_machine_id)
        }

        /// Fetch all ISMP events in the block, should only be called from runtime-api.
        fn block_events() -> Vec<::ismp::events::Event> {
            pallet_ismp::Pallet::<Runtime>::block_events()
        }

        /// Fetch all ISMP events and their extrinsic metadata, should only be called from runtime-api.
        fn block_events_with_metadata() -> Vec<(::ismp::events::Event, Option<u32>)> {
            pallet_ismp::Pallet::<Runtime>::block_events_with_metadata()
        }

        /// Return the scale encoded consensus state.
        fn consensus_state(id: ConsensusClientId) -> Option<Vec<u8>> {
            pallet_ismp::Pallet::<Runtime>::consensus_states(id)
        }

        /// Return the timestamp this client was last updated in seconds.
        fn state_machine_update_time(height: StateMachineHeight) -> Option<u64> {
            pallet_ismp::Pallet::<Runtime>::state_machine_update_time(height)
        }

        /// Return the latest height of the state machine.
        fn latest_state_machine_height(id: StateMachineId) -> Option<u64> {
            pallet_ismp::Pallet::<Runtime>::latest_state_machine_height(id)
        }

        /// Get actual requests.
        fn requests(commitments: Vec<H256>) -> Vec<ismp::router::Request> {
            pallet_ismp::Pallet::<Runtime>::requests(commitments)
        }

        /// Get actual requests.
        fn responses(commitments: Vec<H256>) -> Vec<ismp::router::Response> {
            pallet_ismp::Pallet::<Runtime>::responses(commitments)
        }
    }

    impl ismp_parachain_runtime_api::IsmpParachainApi<Block> for Runtime {
        /// Returns the list of parachains who's consensus updates will be inserted by the inherent data provider.
        fn para_ids() -> Vec<u32> {
            IsmpParachain::para_ids()
        }

        /// Returns the current relay chain state.
        fn current_relay_chain_state() -> RelayChainState {
            IsmpParachain::current_relay_chain_state()
        }
    }

    #[cfg(feature = "try-runtime")]
    impl frame_try_runtime::TryRuntime<Block> for Runtime {
        fn on_runtime_upgrade(checks: frame_try_runtime::UpgradeCheckSelect) -> (Weight, Weight) {
            use super::configs::RuntimeBlockWeights;

            let weight = Executive::try_runtime_upgrade(checks).unwrap();
            (weight, RuntimeBlockWeights::get().max_block)
        }

        fn execute_block(
            block: Block,
            state_root_check: bool,
            signature_check: bool,
            select: frame_try_runtime::TryStateSelect,
        ) -> Weight {
            // NOTE: intentional unwrap: we don't want to propagate the error backwards, and want to
            // have a backtrace here.
            Executive::try_execute_block(block, state_root_check, signature_check, select).unwrap()
        }
    }

    #[cfg(feature = "runtime-benchmarks")]
    impl frame_benchmarking::Benchmark<Block> for Runtime {
        fn benchmark_metadata(extra: bool) -> (
            Vec<frame_benchmarking::BenchmarkList>,
            Vec<frame_support::traits::StorageInfo>,
        ) {
            use frame_benchmarking::BenchmarkList;
            use frame_support::traits::StorageInfoTrait;
            use frame_system_benchmarking::Pallet as SystemBench;
            use cumulus_pallet_session_benchmarking::Pallet as SessionBench;

            use pallet_xcm::benchmarking::Pallet as PalletXcmExtrinsicsBenchmark;

            use super::*;

            let mut list = Vec::<BenchmarkList>::new();
            list_benchmarks!(list, extra);

            let storage_info = AllPalletsWithSystem::storage_info();
            (list, storage_info)
        }

        fn dispatch_benchmark(
            config: frame_benchmarking::BenchmarkConfig
        ) -> Result<Vec<frame_benchmarking::BenchmarkBatch>, alloc::string::String> {
            use frame_benchmarking::{BenchmarkError, BenchmarkBatch};
            use frame_support::parameter_types;
            use cumulus_primitives_core::ParaId;
            use frame_system_benchmarking::Pallet as SystemBench;

            use super::{*, types::*, configs::*, constants::currency::XCAV};

            impl frame_system_benchmarking::Config for Runtime {
                fn setup_set_code_requirements(code: &Vec<u8>) -> Result<(), BenchmarkError> {
                    ParachainSystem::initialize_for_set_code_benchmark(code.len() as u32);
                    Ok(())
                }

                fn verify_set_code() {
                    System::assert_last_event(cumulus_pallet_parachain_system::Event::<Runtime>::ValidationFunctionStored.into());
                }
            }

            parameter_types! {
                pub const RandomParaId: ParaId = ParaId::new(43211234);
                pub ExistentialDepositAsset: Option<Asset> = Some((
                    RelayLocation::get(),
                    ExistentialDeposit::get()
                ).into());
                /// The base fee for the message delivery fees. Kusama is based for the reference.
                pub const ToParentBaseDeliveryFee: u128 = XCAV.saturating_mul(3);
            }
            pub type PriceForParentDelivery = polkadot_runtime_common::xcm_sender::ExponentialPrice<
                FeeAssetId,
                ToParentBaseDeliveryFee,
                TransactionByteFee,
                ParachainSystem,
            >;
            use pallet_xcm::benchmarking::Pallet as PalletXcmExtrinsicsBenchmark;
            use xcm::latest::prelude::{Asset, AssetId, Assets as AssetList, Fungible, Location, Parachain, Parent, ParentThen};
            impl pallet_xcm::benchmarking::Config for Runtime {
                type DeliveryHelper = cumulus_primitives_utility::ToParentDeliveryHelper<
                    xcm_config::XcmConfig,
                    ExistentialDepositAsset,
                    PriceForParentDelivery,
                >;

                fn reachable_dest() -> Option<Location> {
                    Some(Parent.into())
                }

                fn teleportable_asset_and_dest() -> Option<(Asset, Location)> {
                    None
                }

                fn reserve_transferable_asset_and_dest() -> Option<(Asset, Location)> {
                    Some((
                        Asset {
                            fun: Fungible(ExistentialDeposit::get()),
                            id: AssetId(Parent.into())
                        }.into(),
                        ParentThen(Parachain(RandomParaId::get().into()).into()).into(),
                    ))
                }

                fn set_up_complex_asset_transfer(
                ) -> Option<(AssetList, u32, Location, Box<dyn FnOnce()>)> {
                    None
                }

                fn get_asset() -> Asset {
                    Asset {
                        id: AssetId(Location::parent()),
                        fun: Fungible(ExistentialDeposit::get()),
                    }
                }
            }

            use cumulus_pallet_session_benchmarking::Pallet as SessionBench;
            impl cumulus_pallet_session_benchmarking::Config for Runtime {}

            use frame_support::traits::WhitelistedStorageKeys;
            let whitelist = AllPalletsWithSystem::whitelisted_storage_keys();

            let mut batches = Vec::<BenchmarkBatch>::new();
            let params = (&config, &whitelist);
            add_benchmarks!(params, batches);

            if batches.is_empty() { return Err("Benchmark not found for this pallet.".into()) }
            Ok(batches)
        }
    }

    impl sp_genesis_builder::GenesisBuilder<Block> for Runtime {
        fn build_state(config: Vec<u8>) -> sp_genesis_builder::Result {
            build_state::<RuntimeGenesisConfig>(config)
        }

        fn get_preset(id: &Option<sp_genesis_builder::PresetId>) -> Option<Vec<u8>> {
            get_preset::<RuntimeGenesisConfig>(id, |_| None)
        }

        fn preset_names() -> Vec<sp_genesis_builder::PresetId> {
            Default::default()
        }
    }

    impl kilt_runtime_api_did::Did<
        Block,
        DidIdentifier,
        AccountId,
        LinkableAccountId,
        Balance,
        Hash,
        BlockNumber,
        RuntimeCall
    > for Runtime {
        fn query_by_account(account: LinkableAccountId) -> Option<
            RawDidLinkedInfo<
                DidIdentifier,
                AccountId,
                LinkableAccountId,
                Balance,
                Hash,
                BlockNumber
            >
        > {
            pallet_did_lookup::ConnectedDids::<Runtime>::get(account)
                .and_then(|owner_info| {
                    did::Did::<Runtime>::get(&owner_info.did).map(|details| (owner_info, details))
                })
                .map(|(connection_record, details)| {
                    let accounts = pallet_did_lookup::ConnectedAccounts::<Runtime>::iter_key_prefix(&connection_record.did).collect();
                    let service_endpoints = did::ServiceEndpoints::<Runtime>::iter_prefix(&connection_record.did).map(|e| From::from(e.1)).collect();

                    RawDidLinkedInfo {
                        identifier: connection_record.did,
                        accounts,
                        service_endpoints,
                        details: details.into(),
                    }
                })
        }

        fn batch_query_by_account(accounts: Vec<LinkableAccountId>) -> Vec<Option<
            RawDidLinkedInfo<
                DidIdentifier,
                AccountId,
                LinkableAccountId,
                Balance,
                Hash,
                BlockNumber
            >
        >> {
            accounts.into_iter().map(Self::query_by_account).collect()
        }

        fn query(did: DidIdentifier) -> Option<
            RawDidLinkedInfo<
                DidIdentifier,
                AccountId,
                LinkableAccountId,
                Balance,
                Hash,
                BlockNumber
            >
        > {
            let details = did::Did::<Runtime>::get(&did)?;
            let accounts = pallet_did_lookup::ConnectedAccounts::<Runtime>::iter_key_prefix(&did).collect();
            let service_endpoints = did::ServiceEndpoints::<Runtime>::iter_prefix(&did).map(|e| From::from(e.1)).collect();

            Some(RawDidLinkedInfo {
                identifier: did,
                accounts,
                service_endpoints,
                details: details.into(),
            })
        }

        fn batch_query(dids: Vec<DidIdentifier>) -> Vec<Option<
            RawDidLinkedInfo<
                DidIdentifier,
                AccountId,
                LinkableAccountId,
                Balance,
                Hash,
                BlockNumber
            >
        >> {
            dids.into_iter().map(Self::query).collect()
        }
    }

    impl kilt_runtime_api_public_credentials::PublicCredentials<Block, Vec<u8>, Hash, CredentialEntry<Hash, DidIdentifier, BlockNumber, AccountId, Balance, AuthorizationId<<Runtime as delegation::Config>::DelegationNodeId>>, PublicCredentialsFilter<Hash, AccountId>, PublicCredentialsApiError> for Runtime {
        fn get_by_id(credential_id: Hash) -> Option<CredentialEntry<Hash, DidIdentifier, BlockNumber, AccountId, Balance, AuthorizationId<<Runtime as delegation::Config>::DelegationNodeId>>> {
            let subject = public_credentials::CredentialSubjects::<Runtime>::get(credential_id)?;
            public_credentials::Credentials::<Runtime>::get(subject, credential_id)
        }

        fn get_by_subject(subject: Vec<u8>, filter: Option<PublicCredentialsFilter<Hash, AccountId>>) -> Result<Vec<(Hash, CredentialEntry<Hash, DidIdentifier, BlockNumber, AccountId, Balance, AuthorizationId<<Runtime as delegation::Config>::DelegationNodeId>>)>, PublicCredentialsApiError> {
            let asset_did = AssetDid::try_from(subject).map_err(|_| PublicCredentialsApiError::InvalidSubjectId)?;
            let credentials_prefix = public_credentials::Credentials::<Runtime>::iter_prefix(asset_did);
            if let Some(credentials_filter) = filter {
                Ok(credentials_prefix.filter(|(_, entry)| credentials_filter.should_include(entry)).collect())
            } else {
                Ok(credentials_prefix.collect())
            }
        }
    }
}
