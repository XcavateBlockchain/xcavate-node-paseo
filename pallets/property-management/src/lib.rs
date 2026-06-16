// Xcavate Protocol - https://xcavate.io/
// Copyright (C) 2025, Xcavate Foundation

// The Xcavate Protocol is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// The Xcavate Protocol is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with this program.  If not, see <https://www.gnu.org/licenses/>.

#![cfg_attr(not(feature = "std"), no_std)]

pub use pallet::*;

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;
pub mod weights;
pub use weights::*;

use frame_support::{
    storage::with_transaction,
    traits::{
        fungible::MutateHold,
        fungibles::Mutate as FungiblesMutate,
        fungibles::{Inspect, MutateFreeze},
        tokens::Preservation,
        tokens::{fungible, fungibles, Balance, Precision},
        EnsureOriginWithArg,
    },
    PalletId,
};

use frame_support::sp_runtime::{
    traits::{AccountIdConversion, BlockNumberProvider, Zero},
    Percent, Saturating, TransactionOutcome,
};

use parity_scale_codec::Codec;

use pallet_real_world_asset::{
    traits::{PropertySharesInspect, PropertySharesSpvControl},
    PropertyAssetDetails,
};

use primitives::{IncomeSettlement, MarketplaceFreezeReason};

use pallet_regions::{RegionInfo, RegionTrait};
use pallet_xcavate_whitelist::Role;

type AccountIdOf<T> = <T as frame_system::Config>::AccountId;
pub type RuntimeHoldReasonOf<T> = <T as Config>::RuntimeHoldReason;

pub type ForeignAssetIdOf<T> = <<T as Config>::ForeignCurrency as fungibles::Inspect<
    <T as frame_system::Config>::AccountId,
>>::AssetId;

#[frame_support::pallet]
pub mod pallet {
    use super::*;
    use frame_support::pallet_prelude::*;
    use frame_system::pallet_prelude::*;

    #[pallet::pallet]
    pub struct Pallet<T>(_);

    /// A reason for the pallet placing a hold on funds.
    #[pallet::composite_enum]
    pub enum HoldReason {
        /// Funds are held to register for letting agent.
        #[codec(index = 0)]
        LettingAgent,
        /// Funds are held to propose a challenge for the letting agent.
        #[codec(index = 1)]
        ChallengeReserve,
    }

    /// Info for the letting agent.
    #[cfg_attr(feature = "std", derive(serde::Serialize, serde::Deserialize))]
    #[derive(Encode, Decode, Clone, PartialEq, Eq, MaxEncodedLen, RuntimeDebug, TypeInfo)]
    #[scale_info(skip_type_params(T))]
    pub struct LettingAgentInfo<T: Config> {
        /// Region ID where the letting agent is registered.
        pub region: u16,
        /// Locations managed by the letting agent, mapped to their details.
        pub locations: BoundedBTreeMap<LocationId<T>, LocationInfo<T>, T::MaxLocations>,
        /// Strikes against the letting agent, keyed by property asset ID.
        pub active_strikes: BoundedBTreeMap<u32, u8, T::MaxProperties>,
    }

    /// Voting statistics for a proposal.
    #[cfg_attr(feature = "std", derive(serde::Serialize, serde::Deserialize))]
    #[derive(Encode, Decode, Clone, PartialEq, Eq, MaxEncodedLen, RuntimeDebug, TypeInfo)]
    pub struct VoteStats {
        /// Total voting power allocated in favor of the proposal.
        pub yes_voting_power: u32,
        /// Total voting power allocated against the proposal.
        pub no_voting_power: u32,
        /// Total voting power allocated neutral.
        pub abstain_voting_power: u32,
    }

    /// Represents a proposal to assign a letting agent to a property.
    #[derive(Encode, Decode, Clone, MaxEncodedLen, RuntimeDebug, TypeInfo)]
    #[scale_info(skip_type_params(T))]
    pub struct ProposedLettingAgent<T: Config> {
        /// Account ID of the proposed letting agent.
        pub letting_agent: AccountIdOf<T>,
        /// Location (postcode) of the property.
        pub location: LocationId<T>,
        /// Block number when the proposal expires.
        pub expiry_block: BlockNumberFor<T>,
    }

    /// Information about a specific location managed by a letting agent.
    #[cfg_attr(feature = "std", derive(serde::Serialize, serde::Deserialize))]
    #[derive(Encode, Decode, Clone, PartialEq, Eq, MaxEncodedLen, RuntimeDebug, TypeInfo)]
    #[scale_info(skip_type_params(T))]
    pub struct LocationInfo<T: Config> {
        /// Number of properties assigned to this letting agent in the location.
        pub assigned_properties: u32,
        /// Deposit locked for this location.
        pub deposit: <T as pallet::Config>::Balance,
    }

    /// Records a user’s vote on a proposal.
    #[derive(Encode, Decode, Clone, PartialEq, Eq, MaxEncodedLen, RuntimeDebug, TypeInfo)]
    #[scale_info(skip_type_params(T))]
    pub struct VoteRecord {
        /// The vote cast (Yes or No).
        pub vote: Vote,
        /// The asset ID of the property associated with the vote.
        pub asset_id: u32,
        /// The voting power used for the vote.
        pub power: u32,
    }

    /// Notice of resignation from a letting agent for a property.
    #[cfg_attr(feature = "std", derive(serde::Serialize, serde::Deserialize))]
    #[derive(Encode, Decode, Clone, PartialEq, Eq, MaxEncodedLen, RuntimeDebug, TypeInfo)]
    #[scale_info(skip_type_params(T))]
    pub struct ResignationNotice<T: Config> {
        /// Account ID of the resigning letting agent.
        pub letting_agent: AccountIdOf<T>,
        /// Block number when resignation takes effect.
        pub resignation_block: BlockNumberFor<T>,
    }

    /// Represents a vote on a proposal (Yes, No or Abstain).
    #[cfg_attr(feature = "std", derive(serde::Serialize, serde::Deserialize))]
    #[derive(
        Encode,
        Decode,
        DecodeWithMemTracking,
        Clone,
        PartialEq,
        Eq,
        MaxEncodedLen,
        RuntimeDebug,
        TypeInfo,
    )]
    pub enum Vote {
        Yes,
        No,
        Abstain,
    }

    #[pallet::config]
    pub trait Config: frame_system::Config {
        /// Because this pallet emits events, it depends on the runtime's definition of an event.
        type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;

        /// Type representing the weight of this pallet.
        type WeightInfo: WeightInfo;

        /// The type used to represent balances.
        type Balance: Balance
            + TypeInfo
            + From<u128>
            + Into<<Self as pallet::Config>::Balance>
            + Default;

        /// The overarching hold reason.
        type RuntimeHoldReason: From<HoldReason>;

        /// The currency used for deposits.
        type NativeCurrency: fungible::Inspect<AccountIdOf<Self>>
            + fungible::Mutate<AccountIdOf<Self>>
            + fungible::InspectHold<AccountIdOf<Self>, Balance = <Self as pallet::Config>::Balance>
            + fungible::MutateHold<
                AccountIdOf<Self>,
                Balance = <Self as pallet::Config>::Balance,
                Reason = RuntimeHoldReasonOf<Self>,
            > + fungible::BalancedHold<AccountIdOf<Self>, Balance = <Self as pallet::Config>::Balance>;

        /// The currency for payments.
        type ForeignCurrency: fungibles::InspectEnumerable<
                AccountIdOf<Self>,
                Balance = <Self as pallet::Config>::Balance,
                AssetId = u32,
            > + fungibles::metadata::Inspect<AccountIdOf<Self>, AssetId = u32>
            + fungibles::metadata::Mutate<AccountIdOf<Self>, AssetId = u32>
            + fungibles::Mutate<AccountIdOf<Self>, Balance = <Self as pallet::Config>::Balance>
            + fungibles::Inspect<AccountIdOf<Self>, Balance = <Self as pallet::Config>::Balance>;

        /// Handler for freezing assets.
        type AssetsFreezer: fungibles::MutateFreeze<
            AccountIdOf<Self>,
            AssetId = u32,
            Balance = <Self as pallet::Config>::Balance,
            Id = MarketplaceFreezeReason,
        >;

        /// Identifier for the NFT collection.
        type NftCollectionId: Member + Parameter + MaxEncodedLen + Copy;

        /// The type for NFT item IDs.
        type NftId: Member + Parameter + MaxEncodedLen + Copy + Default + CheckedAdd + One;

        /// The property marketplace's pallet id, used for deriving its sovereign account ID.
        #[pallet::constant]
        type MarketplacePalletId: Get<PalletId>;

        /// Minimum deposit for letting agent registration.
        #[pallet::constant]
        type LettingAgentDeposit: Get<<Self as pallet::Config>::Balance>;

        /// Maximum properties a letting agent can manage.
        #[pallet::constant]
        type MaxProperties: Get<u32>;

        /// Maximum locations a letting agent can handle.
        #[pallet::constant]
        type MaxLocations: Get<u32>;

        /// Accepted assets for payments (e.g., USDC, USDT).
        #[pallet::constant]
        type AcceptedAssets: Get<[u32; 2]>;

        /// Property share management traits.
        type PropertyShares: PropertySharesSpvControl<
                PropertyAssetInfo = PropertyAssetDetails<
                    <Self as pallet::Config>::NftId,
                    <Self as pallet::Config>::NftCollectionId,
                    <Self as pallet::Config>::Balance,
                    LocationId<Self>,
                >,
            > + PropertySharesInspect<
                AccountIdOf<Self>,
                PropertyAssetInfo = PropertyAssetDetails<
                    <Self as pallet::Config>::NftId,
                    <Self as pallet::Config>::NftCollectionId,
                    <Self as pallet::Config>::Balance,
                    LocationId<Self>,
                >,
            >;

        /// Voting duration for letting agent proposals.
        #[pallet::constant]
        type LettingAgentVotingTime: Get<BlockNumberFor<Self>>;

        /// Origin type used to verify that an account has a specific Role.
        type PermissionOrigin: EnsureOriginWithArg<
            Self::RuntimeOrigin,
            Role,
            Success = Self::AccountId,
        >;

        /// Minimum quorum that needs to be reached for a proposal to pass.
        #[pallet::constant]
        type MinVotingQuorum: Get<Percent>;

        /// Notice period for letting agent resignation.
        #[pallet::constant]
        type LettingAgentNoticePeriod: Get<BlockNumberFor<Self>>;

        /// Maximum resignation notices per block.
        #[pallet::constant]
        type MaxNoticesPerBlock: Get<u32>;

        /// Provider for the block number. Normally this is the `frame_system` pallet.
        type BlockNumberProvider: BlockNumberProvider<BlockNumber = BlockNumberFor<Self>>;

        /// Provider for region information.
        type RegionProvider: RegionTrait<
            Info = RegionInfo<
                AccountIdOf<Self>,
                <Self as pallet::Config>::Balance,
                BlockNumberFor<Self>,
                <Self as pallet::Config>::NftCollectionId,
            >,
            LocationIdentifier = LocationId<Self>,
        >;

        /// The maximum length of data stored in for post codes.
        #[pallet::constant]
        type PostcodeLimit: Get<u32>;
    }

    pub type ProposalId = u64;
    pub type LocationId<T> = BoundedVec<u8, <T as pallet::Config>::PostcodeLimit>;

    /// Maps property asset IDs to their letting agent.
    #[pallet::storage]
    pub type LettingStorage<T> = StorageMap<_, Blake2_128Concat, u32, AccountIdOf<T>, OptionQuery>;

    /// Maps asset IDs to accumulated property income.
    #[pallet::storage]
    pub type PropertyIncome<T> =
        StorageMap<_, Blake2_128Concat, u32, <T as pallet::Config>::Balance, ValueQuery>;

    /// Maps accounts and asset IDs to income checkpoints.
    #[pallet::storage]
    pub type OwnerCheckpoints<T> = StorageDoubleMap<
        _,
        Blake2_128Concat,
        AccountIdOf<T>,
        Blake2_128Concat,
        u32,
        <T as pallet::Config>::Balance,
        ValueQuery,
    >;

    /// Maps letting agent accounts to their info.
    #[pallet::storage]
    pub type LettingInfo<T: Config> =
        StorageMap<_, Blake2_128Concat, AccountIdOf<T>, LettingAgentInfo<T>, OptionQuery>;

    /// Maps proposal IDs to letting agent proposals.
    #[pallet::storage]
    pub type LettingAgentProposal<T: Config> =
        StorageMap<_, Blake2_128Concat, ProposalId, ProposedLettingAgent<T>, OptionQuery>;

    /// Maps proposal IDs to voting stats.
    #[pallet::storage]
    pub type OngoingLettingAgentVoting<T: Config> =
        StorageMap<_, Blake2_128Concat, ProposalId, VoteStats, OptionQuery>;

    /// Maps proposal and account IDs to user votes.
    #[pallet::storage]
    pub(super) type UserLettingAgentVote<T: Config> = StorageDoubleMap<
        _,
        Blake2_128Concat,
        ProposalId,
        Blake2_128Concat,
        AccountIdOf<T>,
        VoteRecord,
        OptionQuery,
    >;

    /// Maps asset IDs to active letting agent proposal IDs.
    #[pallet::storage]
    pub type AssetLettingProposal<T: Config> =
        StorageMap<_, Blake2_128Concat, u32, ProposalId, OptionQuery>;

    /// Counter of proposal IDs.
    #[pallet::storage]
    pub type ProposalCounter<T: Config> = StorageValue<_, ProposalId, ValueQuery>;

    /// Maps asset IDs to resignation notices.
    #[pallet::storage]
    pub type ResignationNotices<T: Config> =
        StorageMap<_, Blake2_128Concat, u32, ResignationNotice<T>, OptionQuery>;

    /// Queue of resignation notices expiring at specific block numbers.
    #[pallet::storage]
    pub type ResignationQueue<T: Config> = StorageMap<
        _,
        Blake2_128Concat,
        BlockNumberFor<T>,
        BoundedVec<u32, T::MaxNoticesPerBlock>,
        ValueQuery,
    >;

    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {
        /// A letting agent was registered in a region.
        LettingAgentAdded { region: u16, who: T::AccountId },
        /// A letting has been removed from a location.
        LettingAgentRemoved { location: LocationId<T>, who: T::AccountId },
        /// A letting agent was assigned to a property.
        LettingAgentSet { asset_id: u32, who: T::AccountId },
        /// Rental income was distributed for a property.
        IncomeDistributed { asset_id: u32, amount: <T as pallet::Config>::Balance },
        /// A user withdrew funds.
        WithdrawFunds { who: T::AccountId, amount: <T as pallet::Config>::Balance },
        /// A letting agent was proposed for a property.
        LettingAgentProposed { asset_id: u32, who: T::AccountId, proposal_id: ProposalId },
        /// A user voted on a letting agent proposal.
        VotedOnLettingAgent {
            asset_id: u32,
            proposal_id: ProposalId,
            voter: T::AccountId,
            vote: Vote,
        },
        /// A letting agent proposal was rejected.
        LettingAgentRejected { asset_id: u32, letting_agent: T::AccountId },
        /// A user’s frozen shares were released after voting.
        SharesUnfrozen { proposal_id: ProposalId, asset_id: u32, voter: AccountIdOf<T>, amount: u32 },
        /// A letting agent initiated resignation from a property.
        LettingAgentResignationInitiated {
            asset_id: u32,
            letting_agent: AccountIdOf<T>,
            resignation_block: BlockNumberFor<T>,
        },
        /// A letting agent’s resignation was finalized.
        LettingAgentResignationFinalized { asset_id: u32, letting_agent: AccountIdOf<T> },
        /// Processing of a letting agent resignation failed.
        ResignationProcessingFailed { asset_id: u32, error: DispatchResult },
    }

    #[pallet::error]
    pub enum Error<T> {
        /// Error by convertion to balance type.
        ConversionError,
        /// Error by dividing a number.
        DivisionError,
        /// Error by multiplying a number.
        MultiplyError,
        /// Arithmetic operation caused an overflow.
        ArithmeticOverflow,
        /// Arithmetic operation caused an underflow.
        ArithmeticUnderflow,
        /// The caller has no funds stored.
        UserHasNoFundsStored,
        /// The pallet has not enough funds.
        NotEnoughFunds,
        /// No letting agent could be selected.
        NoLettingAgentFound,
        /// The region is not registered.
        RegionUnknown,
        /// The letting agent is already active in too many locations.
        TooManyLocations,
        /// The caller is not authorized to call this extrinsic.
        NoPermission,
        /// The letting agent of this property is already set.
        LettingAgentAlreadySet,
        /// The real estate object could not be found.
        NoObjectFound,
        /// The account is not a letting agent of this location.
        AgentNotFound,
        /// The location is not registered.
        LocationUnknown,
        /// The letting agent is already assigned to this location.
        LettingAgentInLocation,
        /// This Asset is not supported for payment.
        PaymentAssetNotSupported,
        /// No letting agent has been proposed for this property.
        NoLettingAgentProposed,
        /// The propal has expired.
        VotingExpired,
        /// The voting is still ongoing.
        VotingStillOngoing,
        /// There is already a letting agent proposal ongoing.
        LettingAgentProposalOngoing,
        /// The letting agent has is not responsible for this location.
        LocationNotFound,
        /// The letting agent is not active in this location.
        LettingAgentNotActiveInLocation,
        /// Letting agent still has active properties in location.
        LettingAgentActive,
        /// The user has no share amount frozen.
        NoFrozenAmount,
        /// The voting amount must be greater than zero.
        ZeroVoteAmount,
        /// The distribution amount cannot be zero.
        ZeroDistributionAmount,
        /// The total share supply for the property cannot be zero.
        ZeroShareSupply,
        /// A resignation notice is already active for the property.
        ResignationAlreadyInitiated,
        /// Too many resignation notices for the block.
        TooManyNoticesPerBlock,
        /// No resignation notice found for the property.
        NoResignationNotice,
    }

    #[pallet::hooks]
    impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
        fn on_initialize(n: frame_system::pallet_prelude::BlockNumberFor<T>) -> Weight {
            let mut weight = T::DbWeight::get().reads_writes(1, 1);

            // Take all resignations scheduled for this block `n`.
            let expired_notices = ResignationQueue::<T>::take(n);
            // checks if there is a voting for a proposal ending in this block.
            expired_notices.iter().for_each(|asset_id| {
                weight = weight.saturating_add(T::DbWeight::get().reads_writes(1, 1));
                let result =
                    with_transaction(|| -> TransactionOutcome<Result<_, DispatchError>> {
                        // Finalize the resignation for the property.
                        let res = Self::finalize_resignation(*asset_id);
                        match &res {
                            Ok(_) => TransactionOutcome::Commit(Ok(())),
                            Err(e) => {
                                Self::deposit_event(Event::ResignationProcessingFailed {
                                    asset_id: *asset_id,
                                    error: Err(*e),
                                });
                                TransactionOutcome::Rollback(Ok(()))
                            }
                        }
                    });
                if let Err(e) = result {
                    log::error!("Transaction failed for asset_id {:?}: {:?}", asset_id, e);
                }
            });
            weight
        }
    }

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        /// Adds an account as a letting agent.
        ///
        /// The origin must be Signed by a LettingAgent and have sufficient funds.
        ///
        /// Parameters:
        /// - `region`: The region number where the letting agent should be added to.
        /// - `location`: The location number where the letting agent should be added to.
        /// - `letting_agent`: The account of the letting_agent.
        ///
        /// Emits `LettingAgentAdded` event when successful.
        #[pallet::call_index(0)]
        #[pallet::weight(<T as pallet::Config>::WeightInfo::add_letting_agent())]
        pub fn add_letting_agent(
            origin: OriginFor<T>,
            region: u16,
            location: LocationId<T>,
        ) -> DispatchResult {
            let signer = <T as pallet::Config>::PermissionOrigin::ensure_origin(
                origin,
                &Role::LettingAgent,
            )?;
            let _ = <T as pallet::Config>::RegionProvider::get_region_details(region)
                .ok_or(Error::<T>::RegionUnknown)?;
            ensure!(
                <T as pallet::Config>::RegionProvider::location_registered(
                    region,
                    location.clone()
                ),
                Error::<T>::LocationUnknown
            );

            // Lock the deposit from the letting agent.
            let deposit_amount = <T as Config>::LettingAgentDeposit::get();
            <T as pallet::Config>::NativeCurrency::hold(
                &HoldReason::LettingAgent.into(),
                &signer,
                deposit_amount,
            )?;

            // Update or create letting agent info for the caller.
            LettingInfo::<T>::mutate(&signer, |letting_info| {
                let mut info = letting_info.take().unwrap_or_else(|| LettingAgentInfo {
                    region,
                    locations: Default::default(),
                    active_strikes: Default::default(),
                });

                ensure!(
                    !info.locations.contains_key(&location),
                    Error::<T>::LettingAgentInLocation
                );

                info.locations
                    .try_insert(
                        location,
                        LocationInfo { assigned_properties: 0, deposit: deposit_amount },
                    )
                    .map_err(|_| Error::<T>::TooManyLocations)?;

                *letting_info = Some(info);
                Ok::<(), DispatchError>(())
            })?;

            Self::deposit_event(Event::<T>::LettingAgentAdded { region, who: signer });
            Ok(())
        }

        /// Removes a letting agent from a location.
        ///
        /// The origin must be Signed by a LettingAgent and have sufficient funds.
        ///
        /// Parameters:
        /// - `location`: The location where the letting agent should be removed from.
        ///
        /// Emits `LettingAgentRemoved` event when successful.
        #[pallet::call_index(1)]
        #[pallet::weight(<T as pallet::Config>::WeightInfo::remove_letting_agent())]
        pub fn remove_letting_agent(
            origin: OriginFor<T>,
            location: LocationId<T>,
        ) -> DispatchResult {
            let signer = <T as pallet::Config>::PermissionOrigin::ensure_origin(
                origin,
                &Role::LettingAgent,
            )?;

            // Update letting agent info for the caller.
            LettingInfo::<T>::try_mutate(&signer, |maybe_letting_info| {
                let letting_info = maybe_letting_info.as_mut().ok_or(Error::<T>::AgentNotFound)?;

                let location_info = letting_info
                    .locations
                    .remove(&location)
                    .ok_or(Error::<T>::LettingAgentNotActiveInLocation)?;

                ensure!(
                    location_info.assigned_properties.is_zero(),
                    Error::<T>::LettingAgentActive
                );

                // Release the deposit back to the letting agent.
                let deposit_amount = location_info.deposit;
                <T as pallet::Config>::NativeCurrency::release(
                    &HoldReason::LettingAgent.into(),
                    &signer,
                    deposit_amount,
                    Precision::Exact,
                )?;

                // If no locations remain, remove the letting agent entry entirely.
                if letting_info.locations.is_empty() {
                    *maybe_letting_info = None;
                }
                Ok::<(), DispatchError>(())
            })?;
            Self::deposit_event(Event::<T>::LettingAgentRemoved { location, who: signer });
            Ok(())
        }

        /// Propose a letting agent for a property.
        ///
        /// The origin must be Signed by a LettingAgent and have sufficient funds.
        ///
        /// Parameters:
        /// - `asset_id`: The asset id of the property.
        ///
        /// Emits `LettingAgentProposed` event when successful.
        #[pallet::call_index(2)]
        #[pallet::weight(<T as pallet::Config>::WeightInfo::letting_agent_claim_property())]
        pub fn letting_agent_claim_property(origin: OriginFor<T>, asset_id: u32) -> DispatchResult {
            let signer = <T as pallet::Config>::PermissionOrigin::ensure_origin(
                origin,
                &Role::LettingAgent,
            )?;
            ensure!(
                LettingStorage::<T>::get(asset_id).is_none(),
                Error::<T>::LettingAgentAlreadySet
            );
            ensure!(
                !AssetLettingProposal::<T>::contains_key(asset_id),
                Error::<T>::LettingAgentProposalOngoing
            );

            let property_info = T::PropertyShares::get_property_asset_info(asset_id)
                .ok_or(Error::<T>::NoObjectFound)?;
            // Ensure the letting agent is registered and has access to the property's location.
            let letting_info = LettingInfo::<T>::get(&signer).ok_or(Error::<T>::AgentNotFound)?;
            ensure!(
                letting_info.locations.contains_key(&property_info.location),
                Error::<T>::NoPermission
            );
            T::PropertyShares::ensure_property_finalized(asset_id)?;

            // Generate a new proposal ID and set expiry.
            let proposal_id = ProposalCounter::<T>::get();
            let current_block_number =
                <T as pallet::Config>::BlockNumberProvider::current_block_number();
            let expiry_block =
                current_block_number.saturating_add(T::LettingAgentVotingTime::get());

            // Record the proposal and initialize voting stats.
            AssetLettingProposal::<T>::insert(asset_id, proposal_id);
            LettingAgentProposal::<T>::insert(
                proposal_id,
                ProposedLettingAgent {
                    letting_agent: signer.clone(),
                    location: property_info.location,
                    expiry_block,
                },
            );
            OngoingLettingAgentVoting::<T>::insert(
                proposal_id,
                VoteStats { yes_voting_power: 0, no_voting_power: 0, abstain_voting_power: 0 },
            );

            let next_proposal_id =
                proposal_id.checked_add(1).ok_or(Error::<T>::ArithmeticOverflow)?;
            ProposalCounter::<T>::put(next_proposal_id);
            Self::deposit_event(Event::<T>::LettingAgentProposed {
                asset_id,
                who: signer,
                proposal_id,
            });
            Ok(())
        }

        /// Vote for a letting agent.
        ///
        /// The origin must be Signed by a RealEstateInvestor and have sufficient funds.
        ///
        /// Parameters:
        /// - `asset_id`: The asset id of the property.
        /// - `vote`: Must be either a Yes vote or a No vote.
        /// - `amount`: The amount of property shares that the investor is using for voting.
        ///
        /// Emits `VotedOnLettingAgent` event when successful.
        #[pallet::call_index(3)]
        #[pallet::weight(<T as pallet::Config>::WeightInfo::vote_on_letting_agent())]
        pub fn vote_on_letting_agent(
            origin: OriginFor<T>,
            asset_id: u32,
            vote: Vote,
            amount: u32,
        ) -> DispatchResult {
            let signer = <T as pallet::Config>::PermissionOrigin::ensure_origin(
                origin,
                &Role::RealEstateInvestor,
            )?;
            let proposal_id = AssetLettingProposal::<T>::get(asset_id)
                .ok_or(Error::<T>::NoLettingAgentProposed)?;
            let proposal_details = LettingAgentProposal::<T>::get(proposal_id)
                .ok_or(Error::<T>::NoLettingAgentProposed)?;
            ensure!(
                proposal_details.expiry_block
                    > <T as pallet::Config>::BlockNumberProvider::current_block_number(),
                Error::<T>::VotingExpired
            );

            // Ensure the vote amount is valid and the voter has enough shares.
            ensure!(amount > 0, Error::<T>::ZeroVoteAmount);
            let voting_power = T::PropertyShares::get_share_balance(asset_id, &signer);
            ensure!(voting_power >= amount, Error::<T>::NoPermission);

            // Update the voting state for this proposal.
            OngoingLettingAgentVoting::<T>::try_mutate(proposal_id, |maybe_current_vote| {
                let current_vote =
                    maybe_current_vote.as_mut().ok_or(Error::<T>::NoLettingAgentProposed)?;
                UserLettingAgentVote::<T>::try_mutate(proposal_id, &signer, |maybe_vote_record| {
                    // If the user had a previous vote, unfreeze those shares and adjust their previous vote.
                    if let Some(previous_vote) = maybe_vote_record.take() {
                        T::AssetsFreezer::decrease_frozen(
                            asset_id,
                            &MarketplaceFreezeReason::LettingAgentVoting,
                            &signer,
                            previous_vote.power.into(),
                        )?;

                        match previous_vote.vote {
                            Vote::Yes => {
                                current_vote.yes_voting_power = current_vote
                                    .yes_voting_power
                                    .saturating_sub(previous_vote.power)
                            }
                            Vote::No => {
                                current_vote.no_voting_power =
                                    current_vote.no_voting_power.saturating_sub(previous_vote.power)
                            }
                            Vote::Abstain => {
                                current_vote.abstain_voting_power = current_vote
                                    .abstain_voting_power
                                    .saturating_sub(previous_vote.power)
                            }
                        }
                    }

                    // Freeze the new voting amount.
                    T::AssetsFreezer::increase_frozen(
                        asset_id,
                        &MarketplaceFreezeReason::LettingAgentVoting,
                        &signer,
                        amount.into(),
                    )?;

                    // Add the new vote amount to the correct vote tally.
                    match vote {
                        Vote::Yes => {
                            current_vote.yes_voting_power =
                                current_vote.yes_voting_power.saturating_add(amount)
                        }
                        Vote::No => {
                            current_vote.no_voting_power =
                                current_vote.no_voting_power.saturating_add(amount)
                        }
                        Vote::Abstain => {
                            current_vote.abstain_voting_power =
                                current_vote.abstain_voting_power.saturating_add(amount)
                        }
                    }

                    // Store the new vote record for this user.
                    *maybe_vote_record =
                        Some(VoteRecord { vote: vote.clone(), asset_id, power: amount });
                    Ok::<(), DispatchError>(())
                })?;
                Ok::<(), DispatchError>(())
            })?;
            Self::deposit_event(Event::VotedOnLettingAgent {
                asset_id,
                proposal_id,
                voter: signer,
                vote,
            });
            Ok(())
        }

        /// Lets someone finalize the letting agent process.
        ///
        /// The origin must be signed and have sufficient funds.
        ///
        /// Parameters:
        /// - `asset_id`: The asset id of the property.
        ///
        /// Emits `LettingAgentSet` event when vote successful.
        /// Emits `LettingAgentRejected` event when vote unsuccessful.
        #[pallet::call_index(4)]
        #[pallet::weight(<T as pallet::Config>::WeightInfo::finalize_letting_agent())]
        pub fn finalize_letting_agent(origin: OriginFor<T>, asset_id: u32) -> DispatchResult {
            let _ = ensure_signed(origin)?;
            let proposal_id = AssetLettingProposal::<T>::get(asset_id)
                .ok_or(Error::<T>::NoLettingAgentProposed)?;
            let proposal = LettingAgentProposal::<T>::get(proposal_id)
                .ok_or(Error::<T>::NoLettingAgentProposed)?;

            // Ensure the voting period has ended.
            ensure!(
                proposal.expiry_block
                    <= <T as pallet::Config>::BlockNumberProvider::current_block_number(),
                Error::<T>::VotingStillOngoing
            );

            let voting_result = OngoingLettingAgentVoting::<T>::get(proposal_id)
                .ok_or(Error::<T>::NoLettingAgentProposed)?;

            // Fetch property details to calculate total supply and quorum.
            let asset_details =
                <T as pallet::Config>::PropertyShares::get_property_asset_info(asset_id)
                    .ok_or(Error::<T>::NoObjectFound)?;
            let total_votes = voting_result
                .yes_voting_power
                .saturating_add(voting_result.no_voting_power)
                .saturating_add(voting_result.abstain_voting_power);
            let total_supply = asset_details.share_amount;

            ensure!(total_supply > Zero::zero(), Error::<T>::NoObjectFound);

            // Determine if the proposal passes based on votes and quorum.
            let quorum_percent: u32 = T::MinVotingQuorum::get().deconstruct().into();
            let meets_quorum =
                total_votes.saturating_mul(100u32) > total_supply.saturating_mul(quorum_percent);

            // Check if the proposal passes: more Yes votes than No and quorum is met.
            if voting_result.yes_voting_power > voting_result.no_voting_power && meets_quorum {
                ensure!(
                    LettingStorage::<T>::get(asset_id).is_none(),
                    Error::<T>::LettingAgentAlreadySet
                );
                // Update letting agent info to reflect the new property assignment.
                LettingInfo::<T>::try_mutate(
                    proposal.letting_agent.clone(),
                    |maybe_letting_info| {
                        let letting_info =
                            maybe_letting_info.as_mut().ok_or(Error::<T>::AgentNotFound)?;
                        if let Some(location_info) =
                            letting_info.locations.get_mut(&proposal.location)
                        {
                            location_info.assigned_properties = location_info
                                .assigned_properties
                                .checked_add(1)
                                .ok_or(Error::<T>::ArithmeticOverflow)?;
                        } else {
                            return Err(Error::<T>::LocationNotFound.into());
                        }
                        LettingStorage::<T>::insert(asset_id, proposal.letting_agent.clone());
                        Self::deposit_event(Event::<T>::LettingAgentSet {
                            asset_id,
                            who: proposal.letting_agent,
                        });
                        Ok::<(), DispatchError>(())
                    },
                )?;
            } else {
                Self::deposit_event(Event::LettingAgentRejected {
                    asset_id,
                    letting_agent: proposal.letting_agent,
                });
            }
            AssetLettingProposal::<T>::remove(asset_id);
            LettingAgentProposal::<T>::remove(proposal_id);
            OngoingLettingAgentVoting::<T>::remove(proposal_id);

            Ok(())
        }

        /// Lets a voter unlock his locked shares after voting on a letting agent.
        ///
        /// The origin must be signed and have sufficient funds.
        ///
        /// Parameters:
        /// - `proposal_id`: Id of the letting agent proposal.
        ///
        /// Emits `SharesUnfrozen` event when successful.
        #[pallet::call_index(5)]
        #[pallet::weight(<T as pallet::Config>::WeightInfo::unfreeze_letting_voting_shares())]
        pub fn unfreeze_letting_voting_shares(
            origin: OriginFor<T>,
            proposal_id: ProposalId,
        ) -> DispatchResult {
            let signer = ensure_signed(origin)?;
            let vote_record = UserLettingAgentVote::<T>::get(proposal_id, &signer)
                .ok_or(Error::<T>::NoFrozenAmount)?;

            // Ensure voting period has expired.
            if let Some(proposal) = LettingAgentProposal::<T>::get(proposal_id) {
                ensure!(
                    proposal.expiry_block
                        <= <T as pallet::Config>::BlockNumberProvider::current_block_number(),
                    Error::<T>::VotingStillOngoing
                );
            }

            // Unfreeze the voter's shares.
            T::AssetsFreezer::decrease_frozen(
                vote_record.asset_id,
                &MarketplaceFreezeReason::LettingAgentVoting,
                &signer,
                vote_record.power.into(),
            )?;

            UserLettingAgentVote::<T>::remove(proposal_id, &signer);

            Self::deposit_event(Event::SharesUnfrozen {
                proposal_id,
                asset_id: vote_record.asset_id,
                voter: signer,
                amount: vote_record.power,
            });
            Ok(())
        }

        /// Lets the letting agent distribute the income for a property.
        ///
        /// The origin must be Signed by a LettingAgent and have sufficient funds.
        ///
        /// Parameters:
        /// - `asset_id`: The asset ID of the property.
        /// - `amount`: The amount of funds that should be distributed.
        ///
        /// Emits `IncomeDistributed` event when successful.
        #[pallet::call_index(6)]
        #[pallet::weight(<T as pallet::Config>::WeightInfo::distribute_income())]
        pub fn distribute_income(
            origin: OriginFor<T>,
            asset_id: u32,
            amount: <T as pallet::Config>::Balance,
            payment_asset: u32,
        ) -> DispatchResult {
            let signer = <T as pallet::Config>::PermissionOrigin::ensure_origin(
                origin,
                &Role::LettingAgent,
            )?;

            ensure!(amount > Zero::zero(), Error::<T>::ZeroDistributionAmount);
            // Verify that the letting agent is assigned to this property.
            let letting_agent =
                LettingStorage::<T>::get(asset_id).ok_or(Error::<T>::NoLettingAgentFound)?;
            ensure!(letting_agent == signer, Error::<T>::NoPermission);

            ensure!(
                T::AcceptedAssets::get().contains(&payment_asset),
                Error::<T>::PaymentAssetNotSupported
            );

            // Fetch property info (including share supply).
            let property_info = T::PropertyShares::get_property_asset_info(asset_id)
                .ok_or(Error::<T>::NoObjectFound)?;
            let total_supply = property_info.share_amount;
            ensure!(total_supply > 0, Error::<T>::ZeroShareSupply);

            // Transfer the funds from the letting agent to the property's account.
            <T as pallet::Config>::ForeignCurrency::transfer(
                payment_asset,
                &signer,
                &Self::property_account_id(asset_id),
                amount,
                Preservation::Expendable,
            )
            .map_err(|_| Error::<T>::NotEnoughFunds)?;

            // Calculate the income per share (distribution per share).
            let income_per_share =
                amount.checked_div(&total_supply.into()).ok_or(Error::<T>::DivisionError)?;

            // Update the stored cumulative income for this property.
            PropertyIncome::<T>::mutate(asset_id, |current_income| {
                *current_income = current_income
                    .checked_add(&income_per_share)
                    .ok_or(Error::<T>::ArithmeticOverflow)?;
                Ok::<(), DispatchError>(())
            })?;

            Self::deposit_event(Event::<T>::IncomeDistributed { asset_id, amount });
            Ok(())
        }

        /// Lets a property owner withdraw the distributed funds.
        ///
        /// The origin must be Signed by a RealEstateInvestor and have sufficient funds.
        ///
        /// Parameters:
        /// - `asset_id`: The asset ID of the property.
        ///
        /// Emits `WithdrawFunds` event when successful.
        #[pallet::call_index(7)]
        #[pallet::weight(<T as pallet::Config>::WeightInfo::claim_income())]
        pub fn claim_income(origin: OriginFor<T>, asset_id: u32) -> DispatchResult {
            let signer = <T as pallet::Config>::PermissionOrigin::ensure_origin(
                origin,
                &Role::RealEstateInvestor,
            )?;
            // Get the number of property shares the investor currently owns.
            let share_amount = T::PropertyShares::get_share_balance(asset_id, &signer);
            // Calculate the income delta since the last checkpoint.
            let (delta, checkpoint) = Self::get_delta(&signer, asset_id)?;
            // Ensure there is income to claim.
            ensure!(!delta.is_zero(), Error::<T>::UserHasNoFundsStored);
            // Settle the income by transferring funds and updating the checkpoint.
            Self::do_settle_income(signer, asset_id, share_amount, delta, checkpoint)?;
            Ok(())
        }

        /// Allows a LettingAgent to resign from managing a property.
        ///
        /// The origin must be Signed by a LettingAgent and have sufficient funds.
        ///
        /// Parameters:
        /// - `asset_id`: The asset ID of the property from which the agent wants to resign.
        ///
        /// Emits `LettingAgentResignationInitiated` event when successful.
        #[pallet::call_index(8)]
        #[pallet::weight(<T as pallet::Config>::WeightInfo::resign_from_property())]
        pub fn resign_from_property(origin: OriginFor<T>, asset_id: u32) -> DispatchResult {
            let signer = <T as pallet::Config>::PermissionOrigin::ensure_origin(
                origin,
                &Role::LettingAgent,
            )?;
            let letting_agent =
                LettingStorage::<T>::get(asset_id).ok_or(Error::<T>::NoLettingAgentFound)?;
            ensure!(letting_agent == signer, Error::<T>::NoPermission);

            // Prevent duplicate resignation requests for the same property.
            ensure!(
                ResignationNotices::<T>::get(asset_id).is_none(),
                Error::<T>::ResignationAlreadyInitiated
            );

            // Schedule the resignation after the notice period.
            let current_block = <T as pallet::Config>::BlockNumberProvider::current_block_number();
            let resignation_block =
                current_block.saturating_add(T::LettingAgentNoticePeriod::get());

            let notice = ResignationNotice { letting_agent: signer.clone(), resignation_block };
            // Add the asset ID to the resignation queue for the target block.
            ResignationQueue::<T>::try_mutate(resignation_block, |queue| {
                queue.try_push(asset_id).map_err(|_| Error::<T>::TooManyNoticesPerBlock)?;
                Ok::<(), DispatchError>(())
            })?;
            // Store the resignation notice.
            ResignationNotices::<T>::insert(asset_id, notice);
            Self::deposit_event(Event::<T>::LettingAgentResignationInitiated {
                asset_id,
                letting_agent: signer,
                resignation_block,
            });
            Ok(())
        }
    }

    impl<T: Config> Pallet<T> {
        /// Returns the account ID for a specific property based on its asset ID.
        pub fn property_account_id(asset_id: u32) -> AccountIdOf<T> {
            <T as pallet::Config>::MarketplacePalletId::get()
                .into_sub_account_truncating(("pr", asset_id))
        }

        /// Removes bad letting agents from a property.
        pub fn remove_bad_letting_agent(asset_id: u32) -> DispatchResult {
            // Remove letting agent from storage.
            let letting_agent =
                LettingStorage::<T>::take(asset_id).ok_or(Error::<T>::NoLettingAgentFound)?;
            let property_info = T::PropertyShares::get_property_asset_info(asset_id)
                .ok_or(Error::<T>::NoObjectFound)?;
            // Update letting agent info to reflect the property removal.
            LettingInfo::<T>::try_mutate(&letting_agent, |maybe_info| {
                let letting_info = maybe_info.as_mut().ok_or(Error::<T>::AgentNotFound)?;
                if let Some(location_info) = letting_info.locations.get_mut(&property_info.location)
                {
                    location_info.assigned_properties = location_info
                        .assigned_properties
                        .checked_sub(1)
                        .ok_or(Error::<T>::ArithmeticUnderflow)?;
                } else {
                    return Err(Error::<T>::LocationNotFound.into());
                }
                Ok::<(), DispatchError>(())
            })?;
            Ok(())
        }

        /// Finalizes the resignation of a letting agent from a property.
        pub fn finalize_resignation(asset_id: u32) -> DispatchResult {
            let notice =
                ResignationNotices::<T>::get(asset_id).ok_or(Error::<T>::NoResignationNotice)?;
            let property_info = T::PropertyShares::get_property_asset_info(asset_id)
                .ok_or(Error::<T>::NoObjectFound)?;
            // Update letting agent info to reflect the property removal.
            LettingInfo::<T>::try_mutate(&notice.letting_agent, |maybe_info| {
                let letting_info = maybe_info.as_mut().ok_or(Error::<T>::AgentNotFound)?;
                if let Some(location_info) = letting_info.locations.get_mut(&property_info.location)
                {
                    location_info.assigned_properties = location_info
                        .assigned_properties
                        .checked_sub(1)
                        .ok_or(Error::<T>::ArithmeticUnderflow)?;
                } else {
                    return Err(Error::<T>::LocationNotFound.into());
                }
                Ok::<(), DispatchError>(())
            })?;

            // Remove mappings after resignation is finalized.
            LettingStorage::<T>::remove(asset_id);
            ResignationNotices::<T>::remove(asset_id);

            Self::deposit_event(Event::LettingAgentResignationFinalized {
                asset_id,
                letting_agent: notice.letting_agent,
            });
            Ok(())
        }

        /// Retrieves the income delta and checkpoint for an account and asset.
        pub fn get_delta(
            account: &AccountIdOf<T>,
            asset_id: u32,
        ) -> Result<(<T as pallet::Config>::Balance, <T as pallet::Config>::Balance), DispatchError>
        {
            let income_per_share = PropertyIncome::<T>::get(asset_id);
            let checkpoint = OwnerCheckpoints::<T>::get(account, asset_id);
            let delta =
                income_per_share.checked_sub(&checkpoint).ok_or(Error::<T>::ArithmeticUnderflow)?;
            Ok((delta, checkpoint))
        }

        /// Settles income by transferring funds to the account and updating the checkpoint.
        pub fn do_settle_income(
            account: AccountIdOf<T>,
            asset_id: u32,
            share_amount: u32,
            delta: <T as pallet::Config>::Balance,
            checkpoint: <T as pallet::Config>::Balance,
        ) -> DispatchResult {
            ensure!(!share_amount.is_zero(), Error::<T>::UserHasNoFundsStored);
            // Calculate total owed = delta per share * number of shares owned.
            let amount =
                delta.checked_mul(&share_amount.into()).ok_or(Error::<T>::MultiplyError)?;

            // Track total transferred amount
            let mut total_transferred: <T as pallet::Config>::Balance = Zero::zero();
            let mut owed_amount = amount;

            // Attempt to transfer owed amount using accepted assets in order.
            for &payment_asset_id in T::AcceptedAssets::get().iter() {
                // Check available balance for this asset in the property account.
                let available_amount = <T as pallet::Config>::ForeignCurrency::balance(
                    payment_asset_id,
                    &Self::property_account_id(asset_id),
                );
                if available_amount.is_zero() {
                    continue;
                }

                // Determine how much to transfer (the lesser of owed or available).
                let transfer_amount = owed_amount.min(available_amount);
                <T as pallet::Config>::ForeignCurrency::transfer(
                    payment_asset_id,
                    &Self::property_account_id(asset_id),
                    &account,
                    transfer_amount,
                    Preservation::Expendable,
                )
                .map_err(|_| Error::<T>::NotEnoughFunds)?;
                // Update totals and remaining owed amount.
                total_transferred = total_transferred
                    .checked_add(&transfer_amount)
                    .ok_or(Error::<T>::ArithmeticOverflow)?;
                owed_amount = owed_amount
                    .checked_sub(&transfer_amount)
                    .ok_or(Error::<T>::ArithmeticUnderflow)?;
                // Break early if the full owed amount has been transferred.
                if owed_amount.is_zero() {
                    break;
                }
            }
            // If anything was transferred, update checkpoint & emit event.
            if !total_transferred.is_zero() {
                let transferred_per_share = total_transferred
                    .checked_div(&share_amount.into())
                    .ok_or(Error::<T>::DivisionError)?;
                let new_checkpoint = checkpoint
                    .checked_add(&transferred_per_share)
                    .ok_or(Error::<T>::ArithmeticOverflow)?;
                OwnerCheckpoints::<T>::insert(&account, asset_id, new_checkpoint);
                Self::deposit_event(Event::<T>::WithdrawFunds {
                    who: account,
                    amount: total_transferred,
                });
            }
            Ok(())
        }

        /// Updates the checkpoint for an account and asset.
        pub fn do_set_checkpoint(account: AccountIdOf<T>, asset_id: u32) -> DispatchResult {
            let income_per_share = PropertyIncome::<T>::get(asset_id);
            OwnerCheckpoints::<T>::insert(&account, asset_id, income_per_share);
            Ok(())
        }
    }
}

sp_api::decl_runtime_apis! {
    pub trait PropertyManagementApi<AccountId>
    where
        AccountId: Codec
    {
        fn get_management_account_id() -> AccountId;
    }
}

use frame_support::pallet_prelude::DispatchResult;

/// Implementation of income settlement logic for the pallet.
impl<T: Config> IncomeSettlement for Pallet<T> {
    type AccountId = AccountIdOf<T>;

    /// Settles income for a given account and property.
    fn settle_income(account: Self::AccountId, asset_id: u32) -> DispatchResult {
        let share_amount = T::PropertyShares::get_share_balance(asset_id, &account);
        if share_amount.is_zero() {
            // If the user has no shares, just update the checkpoint.
            Self::do_set_checkpoint(account, asset_id)?;
        } else {
            // Calculate the income delta and settle if there's any owed amount.
            let (delta, checkpoint) = Self::get_delta(&account, asset_id)?;
            if !delta.is_zero() {
                Self::do_settle_income(account, asset_id, share_amount, delta, checkpoint)?;
            } else {
                Self::do_set_checkpoint(account, asset_id)?;
            }
        }
        Ok(())
    }
}
