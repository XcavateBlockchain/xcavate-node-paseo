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
    sp_runtime::{
        traits::{AccountIdConversion, BlockNumberProvider},
        Percent, Saturating, TransactionOutcome,
    },
    storage::with_transaction,
    traits::{
        fungible::{BalancedHold, Credit, MutateHold},
        fungibles::MutateFreeze,
        tokens::{fungible, fungibles},
        tokens::{imbalance::OnUnbalanced, Balance, Precision},
        EnsureOriginWithArg,
    },
    PalletId,
};

use parity_scale_codec::{Codec, DecodeWithMemTracking};

use primitives::MarketplaceFreezeReason;

use pallet_real_world_asset::{
    traits::{
        PropertySharesInspect, PropertySharesManage, PropertySharesOwnership, PropertySharesSpvControl,
    },
    PropertyAssetDetails,
};

use pallet_xcavate_whitelist::Role;

pub type AccountIdOf<T> = <T as frame_system::Config>::AccountId;
pub type RuntimeHoldReasonOf<T> = <T as pallet_property_management::Config>::RuntimeHoldReason;

pub type NegativeImbalanceOf<T> =
    Credit<<T as frame_system::Config>::AccountId, <T as Config>::NativeCurrency>;

#[frame_support::pallet]
pub mod pallet {
    use super::*;
    use frame_support::pallet_prelude::*;
    use frame_system::pallet_prelude::*;

    #[pallet::pallet]
    pub struct Pallet<T>(_);

    /// Proposal with the proposal Details.
    #[cfg_attr(feature = "std", derive(serde::Serialize, serde::Deserialize))]
    #[derive(Encode, Decode, Clone, PartialEq, Eq, MaxEncodedLen, RuntimeDebug, TypeInfo)]
    #[scale_info(skip_type_params(T))]
    pub struct Proposal<T: Config> {
        /// Account ID of the proposer.
        pub proposer: AccountIdOf<T>,
        /// Amount requested in the proposal.
        pub amount: <T as pallet::Config>::Balance,
        /// Block number when the proposal was created.
        pub created_at: BlockNumberFor<T>,
        /// Metadata blob for the proposal.
        pub metadata: BoundedVec<u8, <T as pallet::Config>::StringLimit>,
    }

    /// Challenge with the challenge Details.
    #[cfg_attr(feature = "std", derive(serde::Serialize, serde::Deserialize))]
    #[derive(Encode, Decode, Clone, PartialEq, Eq, MaxEncodedLen, RuntimeDebug, TypeInfo)]
    #[scale_info(skip_type_params(T))]
    pub struct Challenge<T: Config> {
        /// Account ID of the proposer.
        pub proposer: AccountIdOf<T>,
        /// Block number when the challenge was created.
        pub created_at: BlockNumberFor<T>,
        /// Deposit amount held for the challenge.
        pub deposit_amount: <T as pallet::Config>::Balance,
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

    #[pallet::config]
    pub trait Config: frame_system::Config + pallet_property_management::Config {
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

        /// The currency used for deposits.
        type NativeCurrency: fungible::Inspect<AccountIdOf<Self>>
            + fungible::Mutate<AccountIdOf<Self>>
            + fungible::InspectHold<AccountIdOf<Self>, Balance = <Self as pallet::Config>::Balance>
            + fungible::MutateHold<
                AccountIdOf<Self>,
                Balance = <Self as pallet::Config>::Balance,
                Reason = RuntimeHoldReasonOf<Self>,
            > + fungible::BalancedHold<AccountIdOf<Self>, Balance = <Self as pallet::Config>::Balance>;

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

        /// The amount of time given to vote for a proposal.
        #[pallet::constant]
        type VotingTime: Get<BlockNumberFor<Self>>;

        /// The maximum amount of votes per block.
        #[pallet::constant]
        type MaxVotesForBlock: Get<u32>;

        /// The minimum amount of a letting agent that will be slashed.
        #[pallet::constant]
        type MinSlashingAmount: Get<<Self as pallet::Config>::Balance>;

        /// Threshold for high costs challenge votes.
        #[pallet::constant]
        type HighThreshold: Get<Percent>;

        /// Proposal amount to be considered a low proposal.
        #[pallet::constant]
        type LowProposal: Get<<Self as pallet::Config>::Balance>;

        /// Proposal amount to be considered a high proposal.
        #[pallet::constant]
        type HighProposal: Get<<Self as pallet::Config>::Balance>;

        /// The property governance's pallet id, used for deriving its sovereign account ID.
        #[pallet::constant]
        type MarketplacePalletId: Get<PalletId>;

        /// Handler for the unbalanced reduction when slashing a letting agent.
        type Slash: OnUnbalanced<NegativeImbalanceOf<Self>>;

        /// Property share management traits.
        type PropertyShares: PropertySharesManage<
                AccountIdOf<Self>,
                <Self as pallet::Config>::Balance,
                <Self as pallet::Config>::NftId,
                <Self as pallet::Config>::StringLimit,
                LocationId<Self>,
            > + PropertySharesOwnership<AccountIdOf<Self>>
            + PropertySharesSpvControl<
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

        /// Origin type used to verify that an account has a specific Role.
        type PermissionOrigin: EnsureOriginWithArg<
            Self::RuntimeOrigin,
            Role,
            Success = Self::AccountId,
        >;

        /// Minimum quorum that needs to be reached for a proposal to pass.
        #[pallet::constant]
        type MinVotingQuorum: Get<Percent>;

        /// Provider for the block number. Normally this is the `frame_system` pallet.
        type BlockNumberProvider: BlockNumberProvider<BlockNumber = BlockNumberFor<Self>>;

        /// Deposit amount to propose a challenge against a letting againt.
        #[pallet::constant]
        type ChallengeDeposit: Get<<Self as pallet::Config>::Balance>;

        /// The maximum length of a name or symbol stored on-chain.
        #[pallet::constant]
        type StringLimit: Get<u32>;

        /// The maximum length of data stored in for post codes.
        #[pallet::constant]
        type PostcodeLimit: Get<u32>;

        /// Cooldown period between automatic executions for a property.
        #[pallet::constant]
        type AutoExecutionCooldown: Get<BlockNumberFor<Self>>;
    }

    pub type ProposalId = u64;
    pub type LocationId<T> = BoundedVec<u8, <T as pallet::Config>::PostcodeLimit>;

    /// Counter for the next proposal ID.
    #[pallet::storage]
    pub(super) type ProposalCount<T> = StorageValue<_, ProposalId, ValueQuery>;

    /// Stores proposals by their ID, mapping to proposal details.
    #[pallet::storage]
    pub(super) type Proposals<T> =
        StorageMap<_, Blake2_128Concat, ProposalId, Proposal<T>, OptionQuery>;

    /// Maps challenge IDs to their details.
    #[pallet::storage]
    pub(super) type Challenges<T> =
        StorageMap<_, Blake2_128Concat, ProposalId, Challenge<T>, OptionQuery>;

    /// Tracks voting statistics for ongoing proposals.
    #[pallet::storage]
    pub(super) type OngoingProposalVotes<T> =
        StorageMap<_, Blake2_128Concat, ProposalId, VoteStats, OptionQuery>;

    /// Maps proposal and account IDs to user vote records.
    #[pallet::storage]
    pub(super) type UserProposalVote<T: Config> = StorageDoubleMap<
        _,
        Blake2_128Concat,
        ProposalId,
        Blake2_128Concat,
        AccountIdOf<T>,
        VoteRecord,
        OptionQuery,
    >;

    /// Maps asset IDs to their associated proposal IDs.
    #[pallet::storage]
    pub type AssetProposal<T: Config> =
        StorageMap<_, Blake2_128Concat, u32, ProposalId, OptionQuery>;

    /// Tracks voting statistics for ongoing challenges.
    #[pallet::storage]
    pub(super) type OngoingChallengeVotes<T> =
        StorageMap<_, Blake2_128Concat, ProposalId, VoteStats, OptionQuery>;

    /// Maps challenge proposal and account IDs to user vote records.
    #[pallet::storage]
    pub(super) type UserChallengeVote<T: Config> = StorageDoubleMap<
        _,
        Blake2_128Concat,
        ProposalId,
        Blake2_128Concat,
        AccountIdOf<T>,
        VoteRecord,
        OptionQuery,
    >;

    /// Maps asset IDs to their associated challenge proposal IDs.
    #[pallet::storage]
    pub type AssetLettingChallenge<T: Config> =
        StorageMap<_, Blake2_128Concat, u32, ProposalId, OptionQuery>;

    /// Stores asset IDs for proposal votings ending at a specific block.
    #[pallet::storage]
    pub type ProposalRoundsExpiring<T: Config> = StorageMap<
        _,
        Blake2_128Concat,
        BlockNumberFor<T>,
        BoundedVec<u32, T::MaxVotesForBlock>,
        ValueQuery,
    >;

    /// Stores asset IDs for challenge votings ending at a specific block.
    #[pallet::storage]
    pub type ChallengeRoundsExpiring<T: Config> = StorageMap<
        _,
        Blake2_128Concat,
        BlockNumberFor<T>,
        BoundedVec<u32, T::MaxVotesForBlock>,
        ValueQuery,
    >;

    /// Tracks the last block number when automatic executions were performed for each property.
    #[pallet::storage]
    pub(super) type LastAutoExecutionBlock<T: Config> =
        StorageMap<_, Blake2_128Concat, u32, BlockNumberFor<T>, OptionQuery>;

    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {
        /// A new proposal was created.
        Proposed { proposal_id: ProposalId, asset_id: u32, proposer: AccountIdOf<T> },
        /// A new challenge was initiated.
        Challenge { asset_id: u32, proposer: AccountIdOf<T>, proposal_id: ProposalId },
        /// A user voted on a proposal.
        VotedOnProposal { proposal_id: ProposalId, voter: AccountIdOf<T>, vote: Vote },
        /// A user voted on challenge.
        VotedOnChallenge { asset_id: u32, voter: AccountIdOf<T>, vote: Vote },
        /// A proposal was executed.
        ProposalExecuted { asset_id: u32, amount: <T as pallet::Config>::Balance },
        /// A letting agent was slashed.
        AgentSlashed { asset_id: u32, amount: <T as pallet::Config>::Balance },
        /// A letting agent was changed.
        AgentChanged { asset_id: u32 },
        /// A proposal was rejected.
        ProposalRejected { proposal_id: ProposalId },
        /// A challenge was rejected.
        ChallengeRejected { asset_id: u32 },
        /// A proposal failed to meet the voting threshold.
        ProposalThresHoldNotReached { proposal_id: ProposalId, required_threshold: Percent },
        /// Processing of a proposal failed.
        ProposalProcessingFailed { asset_id: u32, error: DispatchResult },
        /// Processing of a challenge failed.
        ChallengeProcessingFailed { asset_id: u32, error: DispatchResult },
        /// A user’s shares were unfrozen after voting.
        SharesUnfrozen { proposal_id: ProposalId, asset_id: u32, voter: AccountIdOf<T>, amount: u32 },
    }

    #[pallet::error]
    pub enum Error<T> {
        /// There are already too many proposals in the ending block.
        TooManyProposals,
        /// The proposal is not ongoing.
        NotOngoing,
        /// There is no letting agent for this property.
        NoLettingAgentFound,
        /// The caller is not authorized to call this extrinsic.
        NoPermission,
        /// Real estate asset does not exist.
        AssetNotFound,
        /// The real estate object could not be found.
        NoObjectFound,
        /// Arithmetic overflow occurred.
        ArithmeticOverflow,
        /// Share amount is zero.
        ZeroShareAmount,
        /// The letting agent has already too many assigned properties.
        TooManyAssignedProperties,
        /// A challenge against a letting agent is already ongoing.
        ChallengeAlreadyOngoing,
        /// The user has no share amount frozen.
        NoFrozenAmount,
        /// The voting is still ongoing.
        VotingStillOngoing,
        /// A proposal is already ongoing.
        ProposalOngoing,
        /// The amount for voting has to be higher than 0.
        ZeroVoteAmount,
        /// Proposal amount can not be 0.
        ZeroAmount,
        /// Automatic execution attempted too soon after the last one.
        AutoExecutionTooSoon,
    }

    #[pallet::hooks]
    impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
        fn on_initialize(n: frame_system::pallet_prelude::BlockNumberFor<T>) -> Weight {
            let mut weight = T::DbWeight::get().reads_writes(1, 1);

            // Finalizes proposals ending at the current block.
            let ended_votings = ProposalRoundsExpiring::<T>::take(n);
            ended_votings.iter().for_each(|asset_id| {
                weight = weight.saturating_add(T::DbWeight::get().reads_writes(4, 3));
                let result =
                    with_transaction(|| -> TransactionOutcome<Result<_, DispatchError>> {
                        let res = Self::finish_proposal(*asset_id);
                        match &res {
                            Ok(_) => TransactionOutcome::Commit(Ok(())),
                            Err(e) => {
                                Self::deposit_event(Event::ProposalProcessingFailed {
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

            // Finalizes challenges ending at the current block.
            let ended_challenge_votings = ChallengeRoundsExpiring::<T>::take(n);
            ended_challenge_votings.iter().for_each(|asset_id| {
                weight = weight.saturating_add(T::DbWeight::get().reads_writes(7, 9));
                let result =
                    with_transaction(|| -> TransactionOutcome<Result<_, DispatchError>> {
                        let res = Self::finish_challenge(*asset_id);
                        match &res {
                            Ok(_) => TransactionOutcome::Commit(Ok(())),
                            Err(e) => {
                                Self::deposit_event(Event::ChallengeProcessingFailed {
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
        /// Creates a proposal for a real estate object.
        /// Only the letting agent can propose.
        ///
        /// The origin must be Signed by a LettingAgent and have sufficient funds.
        ///
        /// Parameters:
        /// - `asset_id`: The asset id of the property.
        /// - `amount`: The amount the letting agent is asking for.
        /// - `data`: The data regarding this proposal.
        ///
        /// Emits `Proposed` event when successful.
        #[pallet::call_index(0)]
        #[pallet::weight(<T as pallet::Config>::WeightInfo::propose())]
        pub fn propose(
            origin: OriginFor<T>,
            asset_id: u32,
            amount: <T as pallet::Config>::Balance,
            data: BoundedVec<u8, <T as pallet::Config>::StringLimit>,
        ) -> DispatchResult {
            let signer = <T as pallet::Config>::PermissionOrigin::ensure_origin(
                origin,
                &Role::LettingAgent,
            )?;
            // Ensure the caller is the assigned letting agent for this property.
            ensure!(
                pallet_property_management::LettingStorage::<T>::get(asset_id)
                    .ok_or(Error::<T>::NoLettingAgentFound)?
                    == signer,
                Error::<T>::NoPermission
            );
            // Reject if there’s already a proposal for this property.
            ensure!(!AssetProposal::<T>::contains_key(asset_id), Error::<T>::ProposalOngoing);
            ensure!(amount > Zero::zero(), Error::<T>::ZeroAmount);

            // Create the proposal.
            let current_block_number =
                <T as pallet::Config>::BlockNumberProvider::current_block_number();
            let proposal = Proposal {
                proposer: signer.clone(),
                amount,
                created_at: current_block_number,
                metadata: data,
            };

            // If the amount is below the low proposal threshold, execute immediately.
            if amount <= <T as Config>::LowProposal::get() {
                // Ensure cooldown period has passed since last automatic execution.
                if let Some(last_exec_block) = LastAutoExecutionBlock::<T>::get(asset_id) {
                    let cooldown = T::AutoExecutionCooldown::get();
                    ensure!(
                        current_block_number > last_exec_block.saturating_add(cooldown),
                        Error::<T>::AutoExecutionTooSoon
                    );
                }

                // Record the last automatic execution block.
                LastAutoExecutionBlock::<T>::insert(asset_id, current_block_number);

                Self::execute_proposal(asset_id, proposal)?;
                return Ok(());
            }

            // Otherwise, register the proposal for voting.
            let proposal_id = ProposalCount::<T>::get();
            let expiry_block =
                current_block_number.saturating_add(<T as Config>::VotingTime::get());
            ProposalRoundsExpiring::<T>::try_mutate(expiry_block, |asset_ids| {
                asset_ids.try_push(asset_id).map_err(|_| Error::<T>::TooManyProposals)?;
                Ok::<(), DispatchError>(())
            })?;

            // Initialize vote stats and store proposal.
            let vote_stats =
                VoteStats { yes_voting_power: 0, no_voting_power: 0, abstain_voting_power: 0 };
            AssetProposal::<T>::insert(asset_id, proposal_id);
            Proposals::<T>::insert(proposal_id, proposal);
            OngoingProposalVotes::<T>::insert(proposal_id, vote_stats);

            let next_proposal_id =
                proposal_id.checked_add(1).ok_or(Error::<T>::ArithmeticOverflow)?;
            ProposalCount::<T>::put(next_proposal_id);
            Self::deposit_event(Event::Proposed { proposal_id, asset_id, proposer: signer });
            Ok(())
        }

        /// Creates an challenge against the letting agent of the real estate object.
        /// Only one of the owner of the property can propose.
        ///
        /// The origin must be Signed by a RealEstateInvestor and have sufficient funds.
        ///
        /// Parameters:
        /// - `asset_id`: The asset id of the property.
        ///
        /// Emits `Challenge` event when successful.
        #[pallet::call_index(1)]
        #[pallet::weight(<T as pallet::Config>::WeightInfo::challenge_against_letting_agent())]
        pub fn challenge_against_letting_agent(
            origin: OriginFor<T>,
            asset_id: u32,
        ) -> DispatchResult {
            let signer = <T as pallet::Config>::PermissionOrigin::ensure_origin(
                origin,
                &Role::RealEstateInvestor,
            )?;
            // Ensure the caller is one of the owners of the property
            let owner_list = <T as pallet::Config>::PropertyShares::get_property_owner(asset_id);
            ensure!(owner_list.contains(&signer), Error::<T>::NoPermission);
            ensure!(
                pallet_property_management::LettingStorage::<T>::get(asset_id).is_some(),
                Error::<T>::NoLettingAgentFound
            );
            ensure!(
                !AssetLettingChallenge::<T>::contains_key(asset_id),
                Error::<T>::ChallengeAlreadyOngoing
            );
            // Hold the challenge deposit from the caller.
            let deposit_amount = T::ChallengeDeposit::get();
            <T as pallet::Config>::NativeCurrency::hold(
                &<T as pallet_property_management::Config>::RuntimeHoldReason::from(
                    pallet_property_management::HoldReason::ChallengeReserve,
                ),
                &signer,
                deposit_amount,
            )?;

            let current_block_number =
                <T as pallet::Config>::BlockNumberProvider::current_block_number();
            let expiry_block =
                current_block_number.saturating_add(<T as Config>::VotingTime::get());

            let proposal_id = ProposalCount::<T>::get();
            ChallengeRoundsExpiring::<T>::try_mutate(expiry_block, |asset_ids| {
                asset_ids.try_push(asset_id).map_err(|_| Error::<T>::TooManyProposals)?;
                Ok::<(), DispatchError>(())
            })?;
            // Create the challenge record.
            let challenge = Challenge {
                proposer: signer.clone(),
                created_at: current_block_number,
                deposit_amount,
            };
            let vote_stats =
                VoteStats { yes_voting_power: 0, no_voting_power: 0, abstain_voting_power: 0 };

            // Register the challenge.
            AssetLettingChallenge::<T>::insert(asset_id, proposal_id);
            OngoingChallengeVotes::<T>::insert(proposal_id, vote_stats);
            Challenges::<T>::insert(proposal_id, challenge);

            let next_proposal_id =
                proposal_id.checked_add(1).ok_or(Error::<T>::ArithmeticOverflow)?;
            ProposalCount::<T>::put(next_proposal_id);

            Self::deposit_event(Event::Challenge { asset_id, proposer: signer, proposal_id });
            Ok(())
        }

        /// Lets owner of the real estate object vote on a proposal.
        ///
        /// The origin must be Signed by a RealEstateInvestor and have sufficient funds.
        ///
        /// Parameters:
        /// - `proposal_id`: The index of the proposal.
        /// - `vote`: Must be either a Yes vote or a No vote.
        /// - `amount`: The amount of property shares that the caller is using for voting.
        ///
        /// Emits `VotedOnProposal` event when successful.
        #[pallet::call_index(2)]
        #[pallet::weight(<T as pallet::Config>::WeightInfo::vote_on_proposal())]
        pub fn vote_on_proposal(
            origin: OriginFor<T>,
            asset_id: u32,
            vote: Vote,
            amount: u32,
        ) -> DispatchResult {
            let signer = <T as pallet::Config>::PermissionOrigin::ensure_origin(
                origin,
                &Role::RealEstateInvestor,
            )?;
            let proposal_id = AssetProposal::<T>::get(asset_id).ok_or(Error::<T>::NotOngoing)?;
            ensure!(Proposals::<T>::get(proposal_id).is_some(), Error::<T>::NotOngoing);
            // Ensure the caller is one of the owners of the property.
            let owner_list = <T as pallet::Config>::PropertyShares::get_property_owner(asset_id);
            ensure!(owner_list.contains(&signer), Error::<T>::NoPermission);

            // Ensure the vote amount is valid and the voter has enough shares.
            ensure!(amount > 0, Error::<T>::ZeroVoteAmount);
            let voting_power =
                <T as pallet::Config>::PropertyShares::get_share_balance(asset_id, &signer);
            ensure!(voting_power >= amount, Error::<T>::NoPermission);
            // Update the voting state for this proposal.
            OngoingProposalVotes::<T>::try_mutate(proposal_id, |maybe_current_vote| {
                let current_vote = maybe_current_vote.as_mut().ok_or(Error::<T>::NotOngoing)?;
                UserProposalVote::<T>::try_mutate(proposal_id, &signer, |maybe_vote_record| {
                    // Process the vote and update records.
                    Self::process_vote(
                        current_vote,
                        maybe_vote_record,
                        &signer,
                        asset_id,
                        &vote,
                        amount,
                        &MarketplaceFreezeReason::ProposalVoting,
                    )?;
                    Ok::<(), DispatchError>(())
                })?;
                Ok::<(), DispatchError>(())
            })?;
            Self::deposit_event(Event::VotedOnProposal { proposal_id, voter: signer, vote });
            Ok(())
        }

        /// Lets a voter unlock his locked shares after voting on a proposal.
        ///
        /// The origin must be signed and have sufficient funds.
        ///
        /// Parameters:
        /// - `proposal_id`: Id of the proposal.
        ///
        /// Emits `SharesUnfrozen` event when successful.
        #[pallet::call_index(3)]
        #[pallet::weight(<T as pallet::Config>::WeightInfo::unfreeze_proposal_shares())]
        pub fn unfreeze_proposal_shares(
            origin: OriginFor<T>,
            proposal_id: ProposalId,
        ) -> DispatchResult {
            let signer = ensure_signed(origin)?;
            let vote_record = UserProposalVote::<T>::get(proposal_id, &signer)
                .ok_or(Error::<T>::NoFrozenAmount)?;

            // Ensure the proposal voting has ended.
            ensure!(!Proposals::<T>::contains_key(proposal_id), Error::<T>::VotingStillOngoing);

            // Unfreeze the voter's shares.
            <T as pallet::Config>::AssetsFreezer::decrease_frozen(
                vote_record.asset_id,
                &MarketplaceFreezeReason::ProposalVoting,
                &signer,
                vote_record.power.into(),
            )?;

            UserProposalVote::<T>::remove(proposal_id, &signer);

            Self::deposit_event(Event::SharesUnfrozen {
                proposal_id,
                asset_id: vote_record.asset_id,
                voter: signer,
                amount: vote_record.power,
            });
            Ok(())
        }

        /// Lets owner of the real estate object vote on an challenge.
        ///
        /// The origin must be Signed by a RealEstateInvestor and have sufficient funds.
        ///
        /// Parameters:
        /// - `asset_id: u32`: The index of the challenge.
        /// - `vote`: Must be either a Yes vote or a No vote.
        /// - `amount`: The amount of property shares that the caller is using for voting.
        ///
        /// Emits `VotedOnChallenge` event when successful.
        #[pallet::call_index(4)]
        #[pallet::weight(<T as pallet::Config>::WeightInfo::vote_on_letting_agent_challenge())]
        pub fn vote_on_letting_agent_challenge(
            origin: OriginFor<T>,
            asset_id: u32,
            vote: Vote,
            amount: u32,
        ) -> DispatchResult {
            let signer = <T as pallet::Config>::PermissionOrigin::ensure_origin(
                origin,
                &Role::RealEstateInvestor,
            )?;
            let proposal_id =
                AssetLettingChallenge::<T>::get(asset_id).ok_or(Error::<T>::NotOngoing)?;
            ensure!(Challenges::<T>::get(proposal_id).is_some(), Error::<T>::NotOngoing);
            let owner_list = <T as pallet::Config>::PropertyShares::get_property_owner(asset_id);
            ensure!(owner_list.contains(&signer), Error::<T>::NoPermission);

            // Ensure the vote amount is valid and the voter has enough shares.
            ensure!(amount > 0, Error::<T>::ZeroVoteAmount);
            let voting_power =
                <T as pallet::Config>::PropertyShares::get_share_balance(asset_id, &signer);
            ensure!(voting_power >= amount, Error::<T>::NoPermission);

            // Update the voting state for this challenge.
            OngoingChallengeVotes::<T>::try_mutate(proposal_id, |maybe_current_vote| {
                let current_vote = maybe_current_vote.as_mut().ok_or(Error::<T>::NotOngoing)?;
                UserChallengeVote::<T>::try_mutate(proposal_id, &signer, |maybe_vote_record| {
                    // Process the vote and update records.
                    Self::process_vote(
                        current_vote,
                        maybe_vote_record,
                        &signer,
                        asset_id,
                        &vote,
                        amount,
                        &MarketplaceFreezeReason::ChallengeVoting,
                    )?;
                    Ok::<(), DispatchError>(())
                })?;
                Ok::<(), DispatchError>(())
            })?;
            Self::deposit_event(Event::VotedOnChallenge { asset_id, voter: signer, vote });
            Ok(())
        }

        /// Lets a voter unlock his locked shares after voting on a letting agent challenge.
        ///
        /// The origin must be signed and have sufficient funds.
        ///
        /// Parameters:
        /// - `proposal_id`: Id of the letting agent challenge.
        ///
        /// Emits `SharesUnfrozen` event when successful.
        #[pallet::call_index(5)]
        #[pallet::weight(<T as pallet::Config>::WeightInfo::unfreeze_challenge_shares())]
        pub fn unfreeze_challenge_shares(
            origin: OriginFor<T>,
            proposal_id: ProposalId,
        ) -> DispatchResult {
            let signer = ensure_signed(origin)?;
            let vote_record = UserChallengeVote::<T>::get(proposal_id, &signer)
                .ok_or(Error::<T>::NoFrozenAmount)?;

            // Ensure the challenge voting has ended.
            ensure!(!Challenges::<T>::contains_key(proposal_id), Error::<T>::VotingStillOngoing);

            // Unfreeze the voter's shares.
            <T as pallet::Config>::AssetsFreezer::decrease_frozen(
                vote_record.asset_id,
                &MarketplaceFreezeReason::ChallengeVoting,
                &signer,
                vote_record.power.into(),
            )?;

            UserChallengeVote::<T>::remove(proposal_id, &signer);

            Self::deposit_event(Event::SharesUnfrozen {
                proposal_id,
                asset_id: vote_record.asset_id,
                voter: signer,
                amount: vote_record.power,
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

        /// Slashes a letting agent for a property by a configured minimum amount.
        fn slash_letting_agent(asset_id: u32, letting_agent: AccountIdOf<T>) -> DispatchResult {
            let amount = <T as Config>::MinSlashingAmount::get();

            let (imbalance, _remaining) = <T as pallet::Config>::NativeCurrency::slash(
                &<T as pallet_property_management::Config>::RuntimeHoldReason::from(
                    pallet_property_management::HoldReason::LettingAgent,
                ),
                &letting_agent,
                amount,
            );

            <T as pallet::Config>::Slash::on_unbalanced(imbalance);

            Self::deposit_event(Event::AgentSlashed { asset_id, amount });
            Ok(())
        }

        /// Replaces the letting agent for a property.
        fn change_letting_agent(asset_id: u32) -> DispatchResult {
            let _ = pallet_property_management::Pallet::<T>::remove_bad_letting_agent(asset_id);
            Self::deposit_event(Event::AgentChanged { asset_id });
            Ok(())
        }

        /// Finalizes a proposal by evaluating its voting results and executing or rejecting it.
        fn finish_proposal(asset_id: u32) -> DispatchResult {
            // Retrieve and validate active proposal.
            let proposal_id = AssetProposal::<T>::take(asset_id).ok_or(Error::<T>::NotOngoing)?;
            let voting_results = OngoingProposalVotes::<T>::take(proposal_id);
            let proposals = Proposals::<T>::take(proposal_id);
            if let Some(proposal) = proposals {
                if let Some(voting_result) = voting_results {
                    // Retrieve property details and validate share supply.
                    let asset_details =
                        <T as pallet::Config>::PropertyShares::get_property_asset_info(asset_id);
                    if let Some(asset_details) = asset_details {
                        let total_supply = asset_details.share_amount;
                        ensure!(total_supply > 0, Error::<T>::ZeroShareAmount);

                        let total_votes = voting_result
                            .yes_voting_power
                            .saturating_add(voting_result.no_voting_power)
                            .saturating_add(voting_result.abstain_voting_power);
                        ensure!(total_supply > Zero::zero(), Error::<T>::NoObjectFound);

                        // Check if quorum requirement is met.
                        let quorum_percent: u32 =
                            <T as pallet::Config>::MinVotingQuorum::get().deconstruct().into();

                        let meets_quorum = total_votes.saturating_mul(100u32)
                            > total_supply.saturating_mul(quorum_percent);

                        // Check if approval threshold is met.
                        let mut meets_threshold = true;
                        let approval_base = voting_result
                            .yes_voting_power
                            .saturating_add(voting_result.no_voting_power);
                        if proposal.amount >= <T as Config>::HighProposal::get() {
                            let high_threshold_percent: u32 =
                                <T as Config>::HighThreshold::get().deconstruct().into();
                            meets_threshold = voting_result.yes_voting_power.saturating_mul(100u32)
                                >= approval_base.saturating_mul(high_threshold_percent);
                        }

                        // Determine proposal outcome based on votes, threshold, and quorum.
                        if voting_result.yes_voting_power > voting_result.no_voting_power
                            && meets_threshold
                            && meets_quorum
                        {
                            // Proposal approved; execute it.
                            let _ = Self::execute_proposal(asset_id, proposal);
                        } else if voting_result.yes_voting_power <= voting_result.no_voting_power
                            || !meets_quorum
                        {
                            // Proposal rejected due to insufficient votes or quorum.
                            Self::deposit_event(Event::ProposalRejected { proposal_id });
                        } else {
                            // Proposal rejected due to not meeting high threshold.
                            Self::deposit_event(Event::ProposalThresHoldNotReached {
                                proposal_id,
                                required_threshold: <T as Config>::HighThreshold::get(),
                            });
                        }
                    }
                }
            }
            Ok(())
        }

        /// Finalizes a letting agent challenge by evaluating its voting results and slashing or refunding.
        fn finish_challenge(asset_id: u32) -> DispatchResult {
            // Retrieve and validate active challenge and voting results.
            let proposal_id =
                AssetLettingChallenge::<T>::take(asset_id).ok_or(Error::<T>::NotOngoing)?;
            let challenge_info =
                Challenges::<T>::take(proposal_id).ok_or(Error::<T>::NotOngoing)?;
            let voting_result =
                OngoingChallengeVotes::<T>::take(proposal_id).ok_or(Error::<T>::NotOngoing)?;
            let asset_details =
                <T as pallet::Config>::PropertyShares::get_property_asset_info(asset_id)
                    .ok_or(Error::<T>::AssetNotFound)?;
            let total_supply = asset_details.share_amount;
            ensure!(total_supply > 0, Error::<T>::ZeroShareAmount);

            // Calculate total votes and check quorum.
            let total_votes = voting_result
                .yes_voting_power
                .saturating_add(voting_result.no_voting_power)
                .saturating_add(voting_result.abstain_voting_power);
            let quorum_percent: u32 =
                <T as pallet::Config>::MinVotingQuorum::get().deconstruct().into();
            let meets_quorum =
                total_votes.saturating_mul(100u32) > total_supply.saturating_mul(quorum_percent);

            // Determine challenge outcome based on votes and quorum.
            if voting_result.yes_voting_power > voting_result.no_voting_power && meets_quorum {
                // Challenge approved; slash letting agent and handle strikes.
                let letting_agent = pallet_property_management::LettingStorage::<T>::get(asset_id)
                    .ok_or(Error::<T>::NoLettingAgentFound)?;
                let mut letting_info =
                    pallet_property_management::LettingInfo::<T>::get(letting_agent.clone())
                        .ok_or(Error::<T>::NoLettingAgentFound)?;
                Self::slash_letting_agent(asset_id, letting_agent.clone())?;
                // Increment active strikes for the letting agent.
                let active_strikes = letting_info
                    .active_strikes
                    .get(&asset_id)
                    .copied()
                    .unwrap_or(0)
                    .saturating_add(1);
                if let Some(entry) = letting_info.active_strikes.get_mut(&asset_id) {
                    *entry = active_strikes;
                } else {
                    letting_info
                        .active_strikes
                        .try_insert(asset_id, active_strikes)
                        .map_err(|_| Error::<T>::TooManyAssignedProperties)?;
                }

                // If 3 or more strikes, change the letting agent.
                if active_strikes >= 3 {
                    let _ = Self::change_letting_agent(asset_id);
                    letting_info.active_strikes.remove(&asset_id);
                }
                // Release the challenge deposit of the proposer.
                <T as pallet::Config>::NativeCurrency::release(
                    &<T as pallet_property_management::Config>::RuntimeHoldReason::from(
                        pallet_property_management::HoldReason::ChallengeReserve,
                    ),
                    &challenge_info.proposer,
                    challenge_info.deposit_amount,
                    Precision::Exact,
                )?;
                pallet_property_management::LettingInfo::<T>::insert(letting_agent, letting_info);
            } else {
                // Challenge rejected; slash the proposer’s deposit.
                let (imbalance, _remaining) = <T as pallet::Config>::NativeCurrency::slash(
                    &<T as pallet_property_management::Config>::RuntimeHoldReason::from(
                        pallet_property_management::HoldReason::ChallengeReserve,
                    ),
                    &challenge_info.proposer,
                    challenge_info.deposit_amount,
                );
                <T as pallet::Config>::Slash::on_unbalanced(imbalance);
                Self::deposit_event(Event::ChallengeRejected { asset_id });
            }
            Ok(())
        }

        /// Executes a proposal once it has passed the voting process
        fn execute_proposal(asset_id: u32, proposal: Proposal<T>) -> DispatchResult {
            let proposal_amount = proposal.amount;

            Self::deposit_event(Event::ProposalExecuted { asset_id, amount: proposal_amount });

            Ok(())
        }

        /// Processes a user's vote on a proposal or challenge
        fn process_vote(
            current_vote: &mut VoteStats,
            maybe_vote_record: &mut Option<VoteRecord>,
            signer: &T::AccountId,
            asset_id: u32,
            vote: &Vote,
            amount: u32,
            freeze_reason: &MarketplaceFreezeReason,
        ) -> DispatchResult {
            // If the user already voted before, undo their previous vote.
            if let Some(previous_vote) = maybe_vote_record.take() {
                // Unfreeze the previously frozen shares.
                <T as pallet::Config>::AssetsFreezer::decrease_frozen(
                    asset_id,
                    freeze_reason,
                    signer,
                    previous_vote.power.into(),
                )?;

                // Adjust the current vote counts based on the previous vote.
                match previous_vote.vote {
                    Vote::Yes => {
                        current_vote.yes_voting_power =
                            current_vote.yes_voting_power.saturating_sub(previous_vote.power)
                    }
                    Vote::No => {
                        current_vote.no_voting_power =
                            current_vote.no_voting_power.saturating_sub(previous_vote.power)
                    }
                    Vote::Abstain => {
                        current_vote.abstain_voting_power =
                            current_vote.abstain_voting_power.saturating_sub(previous_vote.power)
                    }
                }
            }

            // Freeze the new voting shares for this vote.
            <T as pallet::Config>::AssetsFreezer::increase_frozen(
                asset_id,
                freeze_reason,
                signer,
                amount.into(),
            )?;

            // Add the new voting power to the correct side (Yes or No).
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

            // Store the new vote record for the voter.
            *maybe_vote_record = Some(VoteRecord { vote: vote.clone(), asset_id, power: amount });
            Ok(())
        }
    }
}

sp_api::decl_runtime_apis! {
    pub trait PropertyGovernanceApi<AccountId>
    where
        AccountId: Codec
    {
        fn get_governance_account_id() -> AccountId;
    }
}
