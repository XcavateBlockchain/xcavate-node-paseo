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
    pallet_prelude::*,
    sp_runtime::{traits::Zero, Percent, Saturating},
    traits::{
        fungible::{BalancedHold, Credit, Inspect, InspectHold, Mutate, MutateHold},
        tokens::{fungible, imbalance::OnUnbalanced, Balance, Precision, Preservation},
        EnsureOriginWithArg,
    },
    PalletId,
};

use sp_runtime::{
    traits::{AccountIdConversion, BlockNumberProvider, CheckedAdd},
    Permill,
};

use pallet_xcavate_whitelist::Role;

pub type NegativeImbalanceOf<T> =
    Credit<<T as frame_system::Config>::AccountId, <T as Config>::NativeCurrency>;

#[frame_support::pallet]
pub mod pallet {
    use super::*;
    use frame_system::pallet_prelude::*;

    /// Details of a region.
    #[derive(Encode, Decode, PartialEq, Eq, MaxEncodedLen, RuntimeDebug, TypeInfo)]
    #[scale_info(skip_type_params(T))]
    pub struct RegionInfo<AccountId, Balance, BlockNumber> {
        /// Account assigned as the regional operator responsible for this region.
        pub owner: AccountId,
        /// Collateral deposit locked by the regional operator as collateral.
        pub collateral: Balance,
        /// Number of active strikes recorded against the regional operator for misconduct.
        pub active_strikes: u8,
        /// Blocknumber after which the regional operator may be reassigned.
        pub next_owner_change: BlockNumber,
    }

    /// Details of a region proposal.
    #[derive(Encode, Decode, PartialEq, Eq, MaxEncodedLen, RuntimeDebug, TypeInfo)]
    #[scale_info(skip_type_params(T))]
    pub struct RegionProposal<T: Config> {
        /// Account that submitted this region proposal.
        pub proposer: T::AccountId,
        /// Block number when the proposal was created.
        pub created_at: BlockNumberFor<T>,
        /// Block number when the proposal expires.
        pub proposal_expiry: BlockNumberFor<T>,
        /// Collateral deposit locked by the proposer for the duration of the proposal.
        pub deposit: T::Balance,
    }

    /// Voting statistics for a proposal.
    #[derive(Encode, Decode, Clone, PartialEq, Eq, MaxEncodedLen, RuntimeDebug, TypeInfo)]
    #[scale_info(skip_type_params(T))]
    pub struct VoteStats<T: Config> {
        /// Total voting power allocated in favor of the proposal.
        pub yes_voting_power: T::Balance,
        /// Total voting power allocated against the proposal.
        pub no_voting_power: T::Balance,
        /// Total voting power allocated neutral.
        pub abstain_voting_power: T::Balance,
    }

    /// Info for region auctions.
    #[derive(Encode, Decode, Clone, PartialEq, Eq, MaxEncodedLen, RuntimeDebug, TypeInfo)]
    #[scale_info(skip_type_params(T))]
    pub struct RegionAuction<T: Config> {
        /// Account ID of the current highest bidder, if any.
        pub highest_bidder: Option<T::AccountId>,
        /// Amount of collateral locked by the highest bidder to be used as the regional deposit.
        pub collateral: T::Balance,
        /// Block number when the auction expires.
        pub auction_expiry: BlockNumberFor<T>,
    }

    /// Infos regarding the proposal of removing a region owner.
    #[derive(Encode, Decode, Clone, PartialEq, Eq, MaxEncodedLen, RuntimeDebug, TypeInfo)]
    #[scale_info(skip_type_params(T))]
    pub struct RemoveRegionOwnerProposal<T: Config> {
        /// Account that submitted this region owner removal proposal.
        pub proposer: T::AccountId,
        /// Block number when voting on this proposal will conclude.
        pub proposal_expiry: BlockNumberFor<T>,
        /// Collateral deposit locked by the proposer for the duration of the proposal.
        pub deposit: T::Balance,
    }

    /// Vote record of a user.
    #[derive(Encode, Decode, Clone, PartialEq, Eq, MaxEncodedLen, RuntimeDebug, TypeInfo)]
    #[scale_info(skip_type_params(T))]
    pub struct VoteRecord<T: Config> {
        /// The vote cast by the account (e.g., Yes or No).
        pub vote: Vote,
        /// Identifier of the region this vote applies to.
        pub region_id: RegionId,
        /// Voting power used for this vote.
        pub power: T::Balance,
    }

    /// Vote enum.
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

    /// Region identifiers for specific countries.
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
    #[repr(u16)]
    pub enum RegionIdentifier {
        England = 1,
        France = 2,
        Japan = 3,
        India = 4,
    }

    impl RegionIdentifier {
        /// Converts the identifier to its u16 value.
        pub fn into_u16(self) -> u16 {
            self as u16
        }
    }

    #[pallet::composite_enum]
    pub enum HoldReason {
        /// Funds are held for operating a region.
        #[codec(index = 0)]
        RegionDepositReserve,
        /// Funds are held for proposing to remove a regional operator.
        #[codec(index = 1)]
        RegionalOperatorRemovalReserve,
        /// Funds are held for proposing a new region.
        #[codec(index = 2)]
        RegionProposalReserve,
        /// Funds are held for voting for a region.
        #[codec(index = 3)]
        RegionVotingReserve,
        /// Funds are held for voting for removing a regional operator.
        #[codec(index = 4)]
        RegionOperatorRemovalVoting,
    }

    #[pallet::config]
    pub trait Config: frame_system::Config {
        type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;

        /// Type representing the weight of this pallet.
        type WeightInfo: WeightInfo;

        /// The balance type for currency operations.
        type Balance: Balance + TypeInfo;

        /// The currency used for deposits.
        type NativeCurrency: fungible::Inspect<Self::AccountId>
            + fungible::Mutate<Self::AccountId>
            + fungible::InspectHold<Self::AccountId, Balance = Self::Balance>
            + fungible::BalancedHold<Self::AccountId, Balance = Self::Balance>
            + fungible::hold::Inspect<Self::AccountId>
            + fungible::hold::Mutate<
                Self::AccountId,
                Reason = <Self as pallet::Config>::RuntimeHoldReason,
            >;

        /// The overarching hold reason.
        type RuntimeHoldReason: From<HoldReason>;

        /// The amount of time given to vote for a new region.
        #[pallet::constant]
        type RegionVotingTime: Get<BlockNumberFor<Self>>;

        /// The amount of time given to bid on a new region.
        #[pallet::constant]
        type RegionAuctionTime: Get<BlockNumberFor<Self>>;

        /// Threshold that needs to be reached to let a region get created.
        #[pallet::constant]
        type RegionThreshold: Get<Percent>;

        /// The amount of time given to vote against a region operator.
        #[pallet::constant]
        type RegionOperatorVotingTime: Get<BlockNumberFor<Self>>;

        /// The maximum amount of proposals per block.
        #[pallet::constant]
        type MaxProposalsForBlock: Get<u32>;

        /// The Treasury's pallet id, used for deriving its sovereign account ID.
        #[pallet::constant]
        type TreasuryId: Get<PalletId>;

        /// The minimum amount of collateral for a regional operator that will be slashed
        #[pallet::constant]
        type RegionSlashingAmount: Get<Self::Balance>;

        /// The time period required between region owner changes.
        #[pallet::constant]
        type RegionOwnerChangePeriod: Get<BlockNumberFor<Self>>;

        /// Handler for the unbalanced reduction when slashing a region owner.
        type Slash: OnUnbalanced<NegativeImbalanceOf<Self>>;

        /// Delay after a region owner resigns before a new auction can begin.
        #[pallet::constant]
        type RegionOwnerNoticePeriod: Get<BlockNumberFor<Self>>;

        /// Deposit amount for a remove regional operator proposal.
        #[pallet::constant]
        type RegionOwnerDisputeDeposit: Get<Self::Balance>;

        /// Minimum deposit for a region.
        #[pallet::constant]
        type MinimumRegionDeposit: Get<Self::Balance>;

        /// Deposit for a region proposal.
        #[pallet::constant]
        type RegionProposalDeposit: Get<Self::Balance>;

        /// Minimum voting amount.
        #[pallet::constant]
        type MinimumVotingAmount: Get<Self::Balance>;

        /// Origin type used to verify that an account has a specific Role.
        type PermissionOrigin: EnsureOriginWithArg<
            Self::RuntimeOrigin,
            Role,
            Success = Self::AccountId,
        >;

        /// Provider for the block number. Normally this is the `frame_system` pallet.
        type BlockNumberProvider: BlockNumberProvider<BlockNumber = BlockNumberFor<Self>>;

        /// Defines how many active strikes a regional operator can accumulate before a reelection can take place.
        type AllowedStrikes: Get<u8>;

        /// Minimum quorum that needs to be reached for a proposal to pass.
        #[pallet::constant]
        type MinVotingQuorum: Get<Permill>;
    }

    pub type RegionId = u16;
    pub type ProposalId = u64;
    pub type RegionInfoOf<T> = RegionInfo<
        <T as frame_system::Config>::AccountId,
        <T as pallet::Config>::Balance,
        BlockNumberFor<T>,
    >;

    #[pallet::pallet]
    pub struct Pallet<T>(_);

    /// Block number of the last region proposal made.
    #[pallet::storage]
    pub(super) type LastRegionProposalBlock<T: Config> =
        StorageValue<_, BlockNumberFor<T>, OptionQuery>;

    /// Active region proposals by region ID.
    #[pallet::storage]
    pub(super) type RegionProposals<T: Config> =
        StorageMap<_, Blake2_128Concat, ProposalId, RegionProposal<T>, OptionQuery>;

    /// Voting statistics for ongoing proposals.
    #[pallet::storage]
    pub(super) type OngoingRegionProposalVotes<T: Config> =
        StorageMap<_, Blake2_128Concat, ProposalId, VoteStats<T>, OptionQuery>;

    /// User votes on region proposals.
    #[pallet::storage]
    pub(super) type UserRegionVote<T: Config> = StorageDoubleMap<
        _,
        Blake2_128Concat,
        ProposalId,
        Blake2_128Concat,
        T::AccountId,
        VoteRecord<T>,
        OptionQuery,
    >;

    /// Active region auctions.
    #[pallet::storage]
    pub(super) type RegionAuctions<T: Config> =
        StorageMap<_, Blake2_128Concat, RegionId, RegionAuction<T>, OptionQuery>;

    /// Replacement auctions for regions.
    #[pallet::storage]
    pub(super) type RegionReplacementAuctions<T: Config> =
        StorageMap<_, Blake2_128Concat, RegionId, RegionAuction<T>, OptionQuery>;

    /// Details of active regions.
    #[pallet::storage]
    pub type RegionDetails<T: Config> =
        StorageMap<_, Blake2_128Concat, RegionId, RegionInfoOf<T>, OptionQuery>;

    /// Active proposals to remove region owners.
    #[pallet::storage]
    pub(super) type RegionOwnerProposals<T: Config> =
        StorageMap<_, Blake2_128Concat, ProposalId, RemoveRegionOwnerProposal<T>, OptionQuery>;

    /// Voting statistics for region owner removal proposals.
    #[pallet::storage]
    pub(super) type OngoingRegionOwnerProposalVotes<T: Config> =
        StorageMap<_, Blake2_128Concat, ProposalId, VoteStats<T>, OptionQuery>;

    /// User votes on region owner removal proposals.
    #[pallet::storage]
    pub(super) type UserRegionOwnerVote<T: Config> = StorageDoubleMap<
        _,
        Blake2_128Concat,
        ProposalId,
        Blake2_128Concat,
        T::AccountId,
        VoteRecord<T>,
        OptionQuery,
    >;

    /// Expiring rounds for region owner removal votings.
    #[pallet::storage]
    pub(super) type RegionOwnerRoundsExpiring<T: Config> = StorageMap<
        _,
        Blake2_128Concat,
        BlockNumberFor<T>,
        BoundedVec<RegionId, T::MaxProposalsForBlock>,
        ValueQuery,
    >;

    /// Expiring replacement auctions.
    #[pallet::storage]
    pub(super) type ReplacementAuctionExpiring<T: Config> = StorageMap<
        _,
        Blake2_128Concat,
        BlockNumberFor<T>,
        BoundedVec<RegionId, T::MaxProposalsForBlock>,
        ValueQuery,
    >;

    /// Counter of proposal IDs.
    #[pallet::storage]
    pub type ProposalCounter<T: Config> = StorageValue<_, ProposalId, ValueQuery>;

    /// Mapping from region ID to the active proposal ID for that region
    #[pallet::storage]
    pub type RegionProposalId<T: Config> =
        StorageMap<_, Blake2_128Concat, RegionId, ProposalId, OptionQuery>;

    /// Mapping from region ID to the active owner-change proposal ID for that region
    #[pallet::storage]
    pub type RegionOwnerProposalId<T: Config> =
        StorageMap<_, Blake2_128Concat, RegionId, ProposalId, OptionQuery>;

    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {
        /// A new region has been proposed.
        RegionProposed { region_id: RegionId, proposer: T::AccountId, proposal_id: ProposalId },
        /// Voted on region proposal.
        VotedOnRegionProposal {
            region_id: RegionId,
            proposal_id: ProposalId,
            voter: T::AccountId,
            vote: Vote,
            voting_power: T::Balance,
            new_yes_power: T::Balance,
            new_no_power: T::Balance,
            new_abstain_power: T::Balance,
        },
        /// New region has been created.
        RegionCreated { region_id: RegionId, owner: T::AccountId },
        /// No region has been created.
        NoRegionCreated { region_id: RegionId },
        /// An auction for a region has started.
        RegionAuctionStarted { region_id: RegionId },
        /// A region got rejected.
        RegionProposalRejected {
            region_id: RegionId,
            slashed_account: T::AccountId,
            amount: T::Balance,
        },
        /// A bid for a region got placed.
        BidSuccessfullyPlaced {
            region_id: RegionId,
            bidder: T::AccountId,
            new_leading_bid: T::Balance,
            previous_bidder: Option<T::AccountId>,
        },
        /// A proposal to remove the region owner has been proposed.
        RemoveRegionOwnerProposed {
            region_id: RegionId,
            proposal_id: ProposalId,
            proposer: T::AccountId,
            proposal_expiry: BlockNumberFor<T>,
        },
        /// Voted on proposal to remove region owner.
        VotedOnRegionOwnerProposal {
            region_id: RegionId,
            proposal_id: ProposalId,
            voter: T::AccountId,
            vote: Vote,
            voting_power: T::Balance,
            new_yes_power: T::Balance,
            new_no_power: T::Balance,
            new_abstain_power: T::Balance,
        },
        /// A proposal for removing the region owner got rejected.
        RegionOwnerRemovalRejected { region_id: RegionId },
        /// A regional operator has been slashed.
        RegionalOperatorSlashed {
            region_id: RegionId,
            slashed_account: T::AccountId,
            amount: T::Balance,
            new_collateral_balance: T::Balance,
            new_active_strikes: u8,
        },
        /// The region is now eligible for an owner change after the specified block.
        RegionOwnerChangeEnabled { region_id: RegionId, next_change_allowed: BlockNumberFor<T> },
        /// A bid for a region got placed.
        ReplacementBidSuccessfullyPlaced {
            region_id: RegionId,
            bidder: T::AccountId,
            new_leading_bid: T::Balance,
        },
        /// The owner of a region has been changed.
        RegionOwnerChanged {
            region_id: RegionId,
            new_owner: T::AccountId,
            next_owner_change: BlockNumberFor<T>,
        },
        /// The owner of a region has initiated resignation.
        RegionOwnerResignationInitiated {
            region_id: RegionId,
            region_owner: T::AccountId,
            next_owner_change: BlockNumberFor<T>,
        },
        /// Processing of a proposal failed.
        RegionOwnerProposalFailed { region_id: RegionId, error: DispatchResult },
        /// Processing of a region owner replacement failed.
        RegionOwnerReplacementFailed { region_id: RegionId, error: DispatchResult },
        /// A user has unfrozen his token.
        TokenUnlocked {
            region_id: RegionId,
            proposal_id: ProposalId,
            voter: T::AccountId,
            amount: T::Balance,
        },
    }

    #[pallet::error]
    pub enum Error<T> {
        /// Arithmetic overflow occurred.
        ArithmeticOverflow,
        /// Arithmetic underflow occurred.
        ArithmeticUnderflow,
        /// This Region is not known.
        RegionUnknown,
        /// No permission for the operation.
        NoPermission,
        /// The proposal is not ongoing.
        NotOngoing,
        /// There is no auction to bid on.
        NoOngoingAuction,
        /// The bid is lower than the current highest bid.
        BidTooLow,
        /// The bid is below the minimum.
        BidBelowMinimum,
        /// The voting has not ended yet.
        VotingStillOngoing,
        /// No Auction found.
        NoAuction,
        /// Auction is still ongoing.
        AuctionNotFinished,
        /// The proposal has already expired.
        ProposalExpired,
        /// Bid amount can not be zero.
        BidCannotBeZero,
        /// There is already a proposal ongoing for this region.
        ProposalAlreadyOngoing,
        /// There are already too many proposals in the ending block.
        TooManyProposals,
        /// Region owner can not be changed at the moment.
        RegionOwnerCantBeChanged,
        /// There are already too many auctions in the ending block.
        TooManyAuctions,
        /// Caller is not the region owner.
        NotRegionOwner,
        /// Owner would change before resignation period would be over.
        OwnerChangeAlreadyScheduled,
        /// The proposal could not be found.
        ProposalNotFound,
        /// The region has already been created.
        RegionAlreadyCreated,
        /// This region has an ongoing proposal.
        RegionProposalAlreadyExists,
        /// The caller does not have enough token to vote.
        NotEnoughTokenToVote,
        /// The auction does not have an winning bidder.
        RegionHasNoWinningBidder,
        /// The user has no token amount frozen.
        NoFrozenAmount,
        /// The token amount for voting is below minimum.
        BelowMinimumVotingAmount,
    }

    #[pallet::hooks]
    impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
        fn on_initialize(n: BlockNumberFor<T>) -> Weight {
            let mut weight = T::DbWeight::get().reads_writes(1, 1);

            let ended_region_owner_votings = RegionOwnerRoundsExpiring::<T>::take(n);
            // Processes ending votings for region owner removal proposals.
            ended_region_owner_votings.iter().for_each(|region_id| {
                weight = weight.saturating_add(T::DbWeight::get().reads_writes(6, 6));
                if let Err(e) = Self::finish_region_owner_proposal(*region_id) {
                    Self::deposit_event(Event::RegionOwnerProposalFailed {
                        region_id: *region_id,
                        error: Err(e),
                    });
                };
            });

            let ended_replacement_auction = ReplacementAuctionExpiring::<T>::take(n);
            // Processes ending auctions for region owner replacements.
            ended_replacement_auction.iter().for_each(|region_id| {
                weight = weight.saturating_add(T::DbWeight::get().reads_writes(3, 3));
                if let Err(e) = Self::finish_region_owner_replacement(*region_id, n) {
                    Self::deposit_event(Event::RegionOwnerReplacementFailed {
                        region_id: *region_id,
                        error: Err(e),
                    });
                };
            });
            weight
        }
    }

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        /// Creates a proposal for a new region.
        ///
        /// The origin must be Signed by a RegionalOperator and have sufficient funds.
        ///
        /// Parameters:
        /// - `region_identifier`: The id of the region the caller is proposing.
        ///
        /// Emits `RegionProposed` event when successful.
        #[pallet::call_index(0)]
        #[pallet::weight(<T as pallet::Config>::WeightInfo::propose_new_region())]
        pub fn propose_new_region(
            origin: OriginFor<T>,
            region_identifier: RegionIdentifier,
        ) -> DispatchResult {
            let signer = T::PermissionOrigin::ensure_origin(origin, &Role::RegionalOperator)?;
            let region_id = region_identifier.into_u16();
            // Ensure there is no ongoing proposal or existing region for the given region ID
            ensure!(
                !RegionProposalId::<T>::contains_key(region_id),
                Error::<T>::RegionProposalAlreadyExists
            );
            ensure!(!RegionDetails::<T>::contains_key(region_id), Error::<T>::RegionAlreadyCreated);

            let proposal_id = ProposalCounter::<T>::get();
            let current_block_number = T::BlockNumberProvider::current_block_number();

            // Hold the required deposit for the proposal
            let deposit_amount = T::RegionProposalDeposit::get();
            T::NativeCurrency::hold(
                &HoldReason::RegionProposalReserve.into(),
                &signer,
                deposit_amount,
            )?;

            // Create and store the region proposal
            let expiry_block =
                current_block_number.saturating_add(<T as Config>::RegionVotingTime::get());
            let proposal = RegionProposal {
                proposer: signer.clone(),
                created_at: current_block_number,
                proposal_expiry: expiry_block,
                deposit: deposit_amount,
            };
            let vote_stats = VoteStats {
                yes_voting_power: Zero::zero(),
                no_voting_power: Zero::zero(),
                abstain_voting_power: Zero::zero(),
            };
            RegionProposalId::<T>::insert(region_id, proposal_id);
            RegionProposals::<T>::insert(proposal_id, proposal);
            OngoingRegionProposalVotes::<T>::insert(proposal_id, vote_stats);
            LastRegionProposalBlock::<T>::put(current_block_number);

            // Increment the proposal counter
            let next_proposal_id =
                proposal_id.checked_add(1).ok_or(Error::<T>::ArithmeticOverflow)?;
            ProposalCounter::<T>::put(next_proposal_id);
            Self::deposit_event(Event::RegionProposed { region_id, proposer: signer, proposal_id });
            Ok(())
        }

        /// Lets a xcav holder vote on a proposal for a region.
        ///
        /// The origin must be signed and have sufficient funds.
        ///
        /// Parameters:
        /// - `region_id`: Id of the region.
        /// - `vote`: Must be either a Yes vote, a No vote or an Abstain vote.
        /// - `amount`: The amount that the caller is using for voting.
        ///
        /// Emits `VotedOnRegionProposal` event when successful.
        #[pallet::call_index(1)]
        #[pallet::weight(<T as pallet::Config>::WeightInfo::vote_on_region_proposal())]
        pub fn vote_on_region_proposal(
            origin: OriginFor<T>,
            region_id: RegionId,
            vote: Vote,
            #[pallet::compact] amount: T::Balance,
        ) -> DispatchResult {
            let signer = ensure_signed(origin)?;
            // Validate amount
            ensure!(amount >= T::MinimumVotingAmount::get(), Error::<T>::BelowMinimumVotingAmount);

            // Retrieve the ongoing proposal for the region
            let proposal_id =
                RegionProposalId::<T>::get(region_id).ok_or(Error::<T>::NotOngoing)?;
            let region_proposal =
                RegionProposals::<T>::get(proposal_id).ok_or(Error::<T>::NotOngoing)?;
            let current_block_number = T::BlockNumberProvider::current_block_number();
            ensure!(
                region_proposal.proposal_expiry > current_block_number,
                Error::<T>::ProposalExpired
            );

            // Verify sufficient funds for voting
            let free_balance = T::NativeCurrency::balance(&signer);
            let held_balance = T::NativeCurrency::balance_on_hold(
                &HoldReason::RegionVotingReserve.into(),
                &signer,
            );
            let total_available = free_balance.saturating_add(held_balance);
            ensure!(total_available >= amount, Error::<T>::NotEnoughTokenToVote);

            // Process the vote and update voting statistics
            let mut new_yes_power = Zero::zero();
            let mut new_no_power = Zero::zero();
            let mut new_abstain_power = Zero::zero();
            OngoingRegionProposalVotes::<T>::try_mutate(proposal_id, |maybe_current_vote| {
                let current_vote = maybe_current_vote.as_mut().ok_or(Error::<T>::NotOngoing)?;
                UserRegionVote::<T>::try_mutate(proposal_id, &signer, |maybe_vote_record| {
                    let (yes_power, no_power, abstain_power) = Self::process_vote(
                        current_vote,
                        maybe_vote_record,
                        &signer,
                        region_id,
                        &vote,
                        amount,
                        &HoldReason::RegionVotingReserve.into(),
                    )?;
                    new_yes_power = yes_power;
                    new_no_power = no_power;
                    new_abstain_power = abstain_power;
                    Ok::<(), DispatchError>(())
                })?;
                Ok::<(), DispatchError>(())
            })?;
            Self::deposit_event(Event::VotedOnRegionProposal {
                region_id,
                proposal_id,
                voter: signer,
                vote,
                voting_power: amount,
                new_yes_power,
                new_no_power,
                new_abstain_power,
            });
            Ok(())
        }

        /// Lets a voter unlock his locked token after voting on a region.
        ///
        /// The origin must be signed and have sufficient funds.
        ///
        /// Parameters:
        /// - `proposal_id`: Id of the region proposal.
        ///
        /// Emits `TokenUnlocked` event when successful.
        #[pallet::call_index(2)]
        #[pallet::weight(<T as pallet::Config>::WeightInfo::unlock_region_voting_token())]
        pub fn unlock_region_voting_token(
            origin: OriginFor<T>,
            proposal_id: ProposalId,
        ) -> DispatchResult {
            let signer = ensure_signed(origin)?;
            // Retrieve the vote record for the user
            let vote_record =
                UserRegionVote::<T>::get(proposal_id, &signer).ok_or(Error::<T>::NoFrozenAmount)?;

            // Check if the proposal has expired
            if let Some(proposal) = RegionProposals::<T>::get(proposal_id) {
                let current_block_number = T::BlockNumberProvider::current_block_number();
                ensure!(
                    proposal.proposal_expiry <= current_block_number,
                    Error::<T>::VotingStillOngoing
                );
            }

            // Release the locked voting tokens
            T::NativeCurrency::release(
                &HoldReason::RegionVotingReserve.into(),
                &signer,
                vote_record.power,
                Precision::Exact,
            )?;

            // Remove the vote record
            UserRegionVote::<T>::remove(proposal_id, &signer);

            Self::deposit_event(Event::TokenUnlocked {
                region_id: vote_record.region_id,
                proposal_id,
                voter: signer,
                amount: vote_record.power,
            });
            Ok(())
        }

        /// Lets a registered account bid on a region to become the regional operator.
        ///
        /// The origin must be Signed by a RegionalOperator and have sufficient funds.
        ///
        /// Parameters:
        /// - `region_id`: Id of the region.
        /// - `amount`: The amount that the caller is willing to bid and to have locked.
        ///
        /// Emits `BidSuccessfullyPlaced` event when successful.
        #[pallet::call_index(3)]
        #[pallet::weight(<T as pallet::Config>::WeightInfo::bid_on_region())]
        pub fn bid_on_region(
            origin: OriginFor<T>,
            region_id: RegionId,
            #[pallet::compact] amount: T::Balance,
        ) -> DispatchResult {
            let signer = T::PermissionOrigin::ensure_origin(origin, &Role::RegionalOperator)?;
            ensure!(!amount.is_zero(), Error::<T>::BidCannotBeZero);

            // Check if there's an ongoing proposal and finalize it if expired
            let proposal_id =
                RegionProposalId::<T>::get(region_id).ok_or(Error::<T>::NotOngoing)?;
            if let Some(region_proposal) = RegionProposals::<T>::get(proposal_id) {
                let current_block_number = T::BlockNumberProvider::current_block_number();
                ensure!(
                    region_proposal.proposal_expiry <= current_block_number,
                    Error::<T>::VotingStillOngoing
                );
                let auction_started =
                    Self::finalize_region_proposal(region_id, current_block_number)?;
                if !auction_started {
                    return Ok(());
                }
            }

            // Process the bid and update auction state
            RegionAuctions::<T>::try_mutate(region_id, |maybe_auction| -> DispatchResult {
                let auction = maybe_auction.as_mut().ok_or(Error::<T>::NoOngoingAuction)?;
                let current_block_number = T::BlockNumberProvider::current_block_number();
                let previous_highest_bidder = auction.highest_bidder.clone();
                ensure!(
                    auction.auction_expiry > current_block_number,
                    Error::<T>::NoOngoingAuction
                );
                // Process the bid
                Self::process_bid(
                    auction,
                    &signer,
                    amount,
                    &HoldReason::RegionDepositReserve.into(),
                )?;
                Self::deposit_event(Event::BidSuccessfullyPlaced {
                    region_id,
                    bidder: signer,
                    new_leading_bid: amount,
                    previous_bidder: previous_highest_bidder,
                });
                Ok::<(), DispatchError>(())
            })?;
            Ok(())
        }

        /// Creates a new region for the marketplace.
        ///
        /// The origin must be Signed by a RegionalOperator and have sufficient funds.
        ///
        /// Parameters:
        /// - `region_id`: Id of the region.
        ///
        /// Emits `RegionCreated` event when successful.
        #[pallet::call_index(4)]
        #[pallet::weight(<T as pallet::Config>::WeightInfo::create_new_region())]
        pub fn create_new_region(origin: OriginFor<T>, region_id: RegionId) -> DispatchResult {
            let signer = T::PermissionOrigin::ensure_origin(origin, &Role::RegionalOperator)?;
            let auction = RegionAuctions::<T>::get(region_id).ok_or(Error::<T>::NoAuction)?;
            let current_block_number = T::BlockNumberProvider::current_block_number();
            // Ensure the auction has finished
            ensure!(auction.auction_expiry <= current_block_number, Error::<T>::AuctionNotFinished);

            // Verify the caller is the winning bidder
            let region_owner =
                auction.highest_bidder.ok_or(Error::<T>::RegionHasNoWinningBidder)?;
            ensure!(region_owner == signer, Error::<T>::NotRegionOwner);

            // If the collateral is zero, no region is created
            if auction.collateral.is_zero() {
                Self::deposit_event(Event::<T>::NoRegionCreated { region_id });
                return Ok(());
            }

            // Set next owner change
            let next_owner_change =
                current_block_number.saturating_add(T::RegionOwnerChangePeriod::get());

            // Create and store region information
            let region_info = RegionInfo {
                owner: region_owner.clone(),
                collateral: auction.collateral,
                active_strikes: Zero::zero(),
                next_owner_change,
            };

            RegionDetails::<T>::insert(region_id, region_info);
            RegionAuctions::<T>::remove(region_id);
            RegionProposalId::<T>::remove(region_id);

            Self::deposit_event(Event::<T>::RegionCreated { region_id, owner: region_owner });

            Ok(())
        }

        /// Creates proposal to remove a region owner.
        ///
        /// The origin must be signed and have sufficient funds.
        ///
        /// Parameters:
        /// - `region_id`: The region where the region owner should be removed.
        ///
        /// Emits `RemoveRegionOwnerProposed` event when successful.
        #[pallet::call_index(8)]
        #[pallet::weight(<T as pallet::Config>::WeightInfo::propose_remove_regional_operator())]
        pub fn propose_remove_regional_operator(
            origin: OriginFor<T>,
            region_id: RegionId,
        ) -> DispatchResult {
            let signer = ensure_signed(origin)?;

            // Ensure the region exists and there is no ongoing removal proposal
            ensure!(RegionDetails::<T>::contains_key(region_id), Error::<T>::RegionUnknown);
            ensure!(
                RegionOwnerProposalId::<T>::get(region_id).is_none(),
                Error::<T>::ProposalAlreadyOngoing
            );

            // Hold the required deposit for the proposal
            let deposit_amount = T::RegionOwnerDisputeDeposit::get();
            T::NativeCurrency::hold(
                &HoldReason::RegionalOperatorRemovalReserve.into(),
                &signer,
                deposit_amount,
            )?;

            // Create and store the removal proposal
            let proposal_id = ProposalCounter::<T>::get();
            let current_block_number = T::BlockNumberProvider::current_block_number();
            let expiry_block =
                current_block_number.saturating_add(T::RegionOperatorVotingTime::get());
            let proposal = RemoveRegionOwnerProposal {
                proposer: signer.clone(),
                proposal_expiry: expiry_block,
                deposit: deposit_amount,
            };

            // Register the proposal expiry
            RegionOwnerRoundsExpiring::<T>::try_mutate(expiry_block, |region_ids| {
                region_ids.try_push(region_id).map_err(|_| Error::<T>::TooManyProposals)?;
                Ok::<(), DispatchError>(())
            })?;

            // Initialize vote stats
            let vote_stats = VoteStats {
                yes_voting_power: Zero::zero(),
                no_voting_power: Zero::zero(),
                abstain_voting_power: Zero::zero(),
            };
            RegionOwnerProposalId::<T>::insert(region_id, proposal_id);
            RegionOwnerProposals::<T>::insert(proposal_id, proposal);
            OngoingRegionOwnerProposalVotes::<T>::insert(proposal_id, vote_stats);

            // Increment the proposal counter
            let next_proposal_id =
                proposal_id.checked_add(1).ok_or(Error::<T>::ArithmeticOverflow)?;
            ProposalCounter::<T>::put(next_proposal_id);
            Self::deposit_event(Event::<T>::RemoveRegionOwnerProposed {
                region_id,
                proposal_id,
                proposer: signer,
                proposal_expiry: expiry_block,
            });
            Ok(())
        }

        /// Vote on proposal to remove a region owner.
        ///
        /// The origin must be signed and have sufficient funds.
        ///
        /// Parameters:
        /// - `region_id`: The region where the region owner should be removed.
        /// - `vote`: Must be either a Yes vote, a No vote or an Abstain vote.
        /// - `amount`: The amount that the caller is using for voting.
        ///
        /// Emits `VotedOnRegionOwnerProposal` event when successful.
        #[pallet::call_index(9)]
        #[pallet::weight(<T as pallet::Config>::WeightInfo::vote_on_remove_owner_proposal())]
        pub fn vote_on_remove_owner_proposal(
            origin: OriginFor<T>,
            region_id: RegionId,
            vote: Vote,
            #[pallet::compact] amount: T::Balance,
        ) -> DispatchResult {
            let signer = ensure_signed(origin)?;
            let proposal_id =
                RegionOwnerProposalId::<T>::get(region_id).ok_or(Error::<T>::NotOngoing)?;

            // // Verify sufficient funds for voting
            let free_balance = T::NativeCurrency::balance(&signer);
            let held_balance = T::NativeCurrency::balance_on_hold(
                &HoldReason::RegionOperatorRemovalVoting.into(),
                &signer,
            );
            let total_available = free_balance.saturating_add(held_balance);
            ensure!(total_available >= amount, Error::<T>::NotEnoughTokenToVote);
            ensure!(amount >= T::MinimumVotingAmount::get(), Error::<T>::BelowMinimumVotingAmount);

            // Process the vote and update voting statistics
            let mut new_yes_power = Zero::zero();
            let mut new_no_power = Zero::zero();
            let mut new_abstain_power = Zero::zero();
            OngoingRegionOwnerProposalVotes::<T>::try_mutate(proposal_id, |maybe_current_vote| {
                let current_vote = maybe_current_vote.as_mut().ok_or(Error::<T>::NotOngoing)?;
                UserRegionOwnerVote::<T>::try_mutate(proposal_id, &signer, |maybe_vote_record| {
                    let (yes_power, no_power, abstain_power) = Self::process_vote(
                        current_vote,
                        maybe_vote_record,
                        &signer,
                        region_id,
                        &vote,
                        amount,
                        &HoldReason::RegionOperatorRemovalVoting.into(),
                    )?;

                    new_yes_power = yes_power;
                    new_no_power = no_power;
                    new_abstain_power = abstain_power;

                    Ok::<(), DispatchError>(())
                })?;
                Ok::<(), DispatchError>(())
            })?;
            Self::deposit_event(Event::VotedOnRegionOwnerProposal {
                region_id,
                proposal_id,
                voter: signer,
                vote,
                voting_power: amount,
                new_yes_power,
                new_no_power,
                new_abstain_power,
            });
            Ok(())
        }

        /// Lets a voter unlock his locked token after voting on removal of a regional operator.
        ///
        /// The origin must be signed and have sufficient funds.
        ///
        /// Parameters:
        /// - `proposal_id`: Id of the region proposal.
        ///
        /// Emits `TokenUnlocked` event when successful.
        #[pallet::call_index(10)]
        #[pallet::weight(<T as pallet::Config>::WeightInfo::unlock_region_owner_removal_voting_token())]
        pub fn unlock_region_owner_removal_voting_token(
            origin: OriginFor<T>,
            proposal_id: ProposalId,
        ) -> DispatchResult {
            let signer = ensure_signed(origin)?;
            // Retrieve the vote record for the user
            let vote_record = UserRegionOwnerVote::<T>::get(proposal_id, &signer)
                .ok_or(Error::<T>::NoFrozenAmount)?;

            // Check if the proposal has expired
            if let Some(proposal) = RegionOwnerProposals::<T>::get(proposal_id) {
                let current_block_number = T::BlockNumberProvider::current_block_number();
                ensure!(
                    proposal.proposal_expiry <= current_block_number,
                    Error::<T>::VotingStillOngoing
                );
            }

            // Release the locked voting tokens
            T::NativeCurrency::release(
                &HoldReason::RegionOperatorRemovalVoting.into(),
                &signer,
                vote_record.power,
                Precision::Exact,
            )?;

            // Remove the vote record
            UserRegionOwnerVote::<T>::remove(proposal_id, &signer);

            Self::deposit_event(Event::TokenUnlocked {
                region_id: vote_record.region_id,
                proposal_id,
                voter: signer,
                amount: vote_record.power,
            });
            Ok(())
        }

        /// Lets a registered account bid on a region to become the new regional operator.
        ///
        /// The origin must be Signed by a RegionalOperator and have sufficient funds.
        ///
        /// Parameters:
        /// - `region_id`: The region where the region owner should be removed.
        /// - `amount`: The amount that the caller is willing to bid and to have locked.
        ///
        /// Emits `ReplacementBidSuccessfullyPlaced` event when successful.
        #[pallet::call_index(11)]
        #[pallet::weight(<T as pallet::Config>::WeightInfo::bid_on_region_replacement())]
        pub fn bid_on_region_replacement(
            origin: OriginFor<T>,
            region_id: RegionId,
            #[pallet::compact] amount: T::Balance,
        ) -> DispatchResult {
            let signer = T::PermissionOrigin::ensure_origin(origin, &Role::RegionalOperator)?;
            let region_info =
                RegionDetails::<T>::get(region_id).ok_or(Error::<T>::RegionUnknown)?;

            // Check if owner change is allowed
            let current_block_number = T::BlockNumberProvider::current_block_number();
            ensure!(
                region_info.next_owner_change < current_block_number,
                Error::<T>::RegionOwnerCantBeChanged
            );

            let mut new_auction = false;

            // Process or create the auction
            RegionReplacementAuctions::<T>::try_mutate(region_id, |maybe_auction| {
                let auction = maybe_auction.get_or_insert_with(|| {
                    new_auction = true;
                    // Calculate the minimum deposit
                    let minimum_deposit = T::MinimumRegionDeposit::get();
                    RegionAuction {
                        highest_bidder: None,
                        collateral: minimum_deposit,
                        auction_expiry: current_block_number
                            .saturating_add(T::RegionAuctionTime::get()),
                    }
                });
                ensure!(
                    auction.auction_expiry > current_block_number,
                    Error::<T>::NoOngoingAuction
                );
                // Process the bid
                Self::process_bid(
                    auction,
                    &signer,
                    amount,
                    &HoldReason::RegionDepositReserve.into(),
                )?;
                Ok::<(), DispatchError>(())
            })?;

            // Register expiry only if auction was newly created
            if new_auction {
                let expiry_block = current_block_number.saturating_add(T::RegionAuctionTime::get());
                ReplacementAuctionExpiring::<T>::try_mutate(expiry_block, |region_ids| {
                    region_ids.try_push(region_id).map_err(|_| Error::<T>::TooManyAuctions)?;
                    Ok::<(), DispatchError>(())
                })?;
            }

            Self::deposit_event(Event::ReplacementBidSuccessfullyPlaced {
                region_id,
                bidder: signer,
                new_leading_bid: amount,
            });
            Ok(())
        }

        /// Lets a regional operator resign.
        ///
        /// The origin must be Signed by a RegionalOperator and have sufficient funds.
        ///
        /// Parameters:
        /// - `region_id`: The region where the region wants to resign.
        ///
        /// Emits `RegionOwnerResignationInitiated` event when successful.
        #[pallet::call_index(12)]
        #[pallet::weight(<T as pallet::Config>::WeightInfo::initiate_region_owner_resignation())]
        pub fn initiate_region_owner_resignation(
            origin: OriginFor<T>,
            region_id: RegionId,
        ) -> DispatchResult {
            let signer = T::PermissionOrigin::ensure_origin(origin, &Role::RegionalOperator)?;

            // Update the region's next owner change block
            RegionDetails::<T>::try_mutate(region_id, |maybe_region| -> DispatchResult {
                let region_info = maybe_region.as_mut().ok_or(Error::<T>::RegionUnknown)?;
                ensure!(region_info.owner == signer, Error::<T>::NotRegionOwner);

                // Calculate the next owner change block
                let current_block_number = T::BlockNumberProvider::current_block_number();
                let next_owner_change =
                    current_block_number.saturating_add(T::RegionOwnerNoticePeriod::get());
                // Ensure no earlier owner change is scheduled
                ensure!(
                    region_info.next_owner_change > next_owner_change,
                    Error::<T>::OwnerChangeAlreadyScheduled
                );
                region_info.next_owner_change = next_owner_change;

                Self::deposit_event(Event::RegionOwnerResignationInitiated {
                    region_id,
                    region_owner: signer,
                    next_owner_change: region_info.next_owner_change,
                });
                Ok::<(), DispatchError>(())
            })
        }
    }

    impl<T: Config> Pallet<T> {
        /// Returns the treasury account ID.
        pub fn treasury_account_id() -> T::AccountId {
            T::TreasuryId::get().into_account_truncating()
        }

        /// Processes a region proposal after voting.
        /// Determines if the proposal meets the threshold and starts an auction or rejects it.
        fn finalize_region_proposal(
            region_id: RegionId,
            current_block_number: BlockNumberFor<T>,
        ) -> Result<bool, DispatchError> {
            // Retrieve and remove proposal data
            let proposal_id =
                RegionProposalId::<T>::get(region_id).ok_or(Error::<T>::NotOngoing)?;
            let voting_results =
                OngoingRegionProposalVotes::<T>::take(proposal_id).ok_or(Error::<T>::NotOngoing)?;
            let proposal = RegionProposals::<T>::take(proposal_id).ok_or(Error::<T>::NotOngoing)?;

            // Calculate total voting amount
            let total_voting_amount = voting_results
                .yes_voting_power
                .checked_add(&voting_results.no_voting_power)
                .ok_or(Error::<T>::ArithmeticOverflow)?
                .checked_add(&voting_results.abstain_voting_power)
                .ok_or(Error::<T>::ArithmeticOverflow)?;

            let total_issuance = T::NativeCurrency::total_issuance();

            // Check if the proposal meets the approval threshold
            let threshold_percent: T::Balance = T::RegionThreshold::get().deconstruct().into();
            let approval_base = voting_results
                .yes_voting_power
                .checked_add(&voting_results.no_voting_power)
                .ok_or(Error::<T>::ArithmeticOverflow)?;
            let meets_threshold = !total_voting_amount.is_zero()
                && voting_results.yes_voting_power.saturating_mul(100u32.into())
                    >= approval_base.saturating_mul(threshold_percent);

            // Check quorum.
            let meets_quorum =
                total_voting_amount > T::MinVotingQuorum::get().mul_floor(total_issuance);

            let auction_expiry_block =
                current_block_number.saturating_add(T::RegionAuctionTime::get());
            if meets_threshold && meets_quorum {
                // Release proposer's deposit
                T::NativeCurrency::release(
                    &HoldReason::RegionProposalReserve.into(),
                    &proposal.proposer,
                    proposal.deposit,
                    Precision::Exact,
                )?;

                // Send rewards from treasury if sufficient funds
                let treasury_account = Self::treasury_account_id();
                if T::NativeCurrency::balance(&treasury_account) >= proposal.deposit {
                    T::NativeCurrency::transfer(
                        &treasury_account,
                        &proposal.proposer,
                        proposal.deposit,
                        Preservation::Expendable,
                    )?;
                }

                // Start a new auction for the region
                let auction = RegionAuction {
                    highest_bidder: None,
                    collateral: T::MinimumRegionDeposit::get(),
                    auction_expiry: auction_expiry_block,
                };
                RegionAuctions::<T>::insert(region_id, auction);
                Self::deposit_event(Event::RegionAuctionStarted { region_id });
                Ok(true)
            } else {
                // Slash proposer's deposit for failed proposal
                let (imbalance, _remaining) = <T as pallet::Config>::NativeCurrency::slash(
                    &HoldReason::RegionProposalReserve.into(),
                    &proposal.proposer,
                    proposal.deposit,
                );
                T::Slash::on_unbalanced(imbalance);
                RegionProposalId::<T>::remove(region_id);
                Self::deposit_event(Event::RegionProposalRejected {
                    region_id,
                    slashed_account: proposal.proposer,
                    amount: proposal.deposit,
                });
                Ok(false)
            }
        }

        /// Processes a proposal for removing a regional operator.
        fn finish_region_owner_proposal(region_id: RegionId) -> DispatchResult {
            // Retrieve and remove proposal data
            let proposal_id =
                RegionOwnerProposalId::<T>::take(region_id).ok_or(Error::<T>::ProposalNotFound)?;
            let proposal =
                RegionOwnerProposals::<T>::take(proposal_id).ok_or(Error::<T>::ProposalNotFound)?;
            let voting_result = OngoingRegionOwnerProposalVotes::<T>::take(proposal_id)
                .ok_or(Error::<T>::ProposalNotFound)?;
            // Calculate total voting amount
            let total_voting_amount = voting_result
                .yes_voting_power
                .checked_add(&voting_result.no_voting_power)
                .ok_or(Error::<T>::ArithmeticOverflow)?
                .checked_add(&voting_result.abstain_voting_power)
                .ok_or(Error::<T>::ArithmeticOverflow)?;

            let total_issuance = T::NativeCurrency::total_issuance();

            // Check if the proposal meets the approval threshold
            let threshold_percent: T::Balance = T::RegionThreshold::get().deconstruct().into();
            let approval_base = voting_result
                .yes_voting_power
                .checked_add(&voting_result.no_voting_power)
                .ok_or(Error::<T>::ArithmeticOverflow)?;
            let meets_threshold = !total_voting_amount.is_zero()
                && voting_result.yes_voting_power.saturating_mul(100u32.into())
                    >= approval_base.saturating_mul(threshold_percent);

            // Check quorum.
            let meets_quorum =
                total_voting_amount > T::MinVotingQuorum::get().mul_floor(total_issuance);

            if meets_threshold && meets_quorum {
                // Enable owner change if strikes reach threshold
                let updated_strikes = Self::slash_region_owner(region_id)?;
                if updated_strikes >= T::AllowedStrikes::get() {
                    Self::enable_region_owner_change(region_id)?;
                }
                // Release proposer's deposit
                T::NativeCurrency::release(
                    &HoldReason::RegionalOperatorRemovalReserve.into(),
                    &proposal.proposer,
                    proposal.deposit,
                    Precision::Exact,
                )?;
            } else {
                // Slash proposer's deposit for failed proposal
                let (imbalance, _remaining) = <T as pallet::Config>::NativeCurrency::slash(
                    &HoldReason::RegionalOperatorRemovalReserve.into(),
                    &proposal.proposer,
                    proposal.deposit,
                );
                T::Slash::on_unbalanced(imbalance);

                Self::deposit_event(Event::RegionOwnerRemovalRejected { region_id });
            }

            Ok(())
        }

        /// Slashes a region owner for misconduct.
        fn slash_region_owner(region_id: RegionId) -> Result<u8, DispatchError> {
            let mut region_info =
                RegionDetails::<T>::get(region_id).ok_or(Error::<T>::RegionUnknown)?;
            let amount = <T as Config>::RegionSlashingAmount::get();
            let region_owner = region_info.owner.clone();

            // Slash the region owner's collateral
            let (imbalance, _remaining) = <T as pallet::Config>::NativeCurrency::slash(
                &HoldReason::RegionDepositReserve.into(),
                &region_owner,
                amount,
            );
            T::Slash::on_unbalanced(imbalance);

            // Update collateral and strikes
            region_info.collateral = region_info.collateral.saturating_sub(amount);
            region_info.active_strikes = region_info.active_strikes.saturating_add(1);

            let updated_strikes = region_info.active_strikes;
            let new_collateral_balance = region_info.collateral;
            let new_active_strikes = region_info.active_strikes;
            RegionDetails::<T>::insert(region_id, region_info);

            Self::deposit_event(Event::RegionalOperatorSlashed {
                region_id,
                slashed_account: region_owner,
                amount,
                new_collateral_balance,
                new_active_strikes,
            });
            Ok(updated_strikes)
        }

        /// Enables a region owner change after a threshold is met.
        /// Sets the next owner change to the current block.
        fn enable_region_owner_change(region_id: RegionId) -> DispatchResult {
            let mut region_info =
                RegionDetails::<T>::get(region_id).ok_or(Error::<T>::RegionUnknown)?;
            let current_block_number = T::BlockNumberProvider::current_block_number();

            // Update next owner change to current block
            region_info.next_owner_change = current_block_number;
            RegionDetails::<T>::insert(region_id, region_info);
            Self::deposit_event(Event::RegionOwnerChangeEnabled {
                region_id,
                next_change_allowed: current_block_number,
            });
            Ok(())
        }

        /// Processes a region owner replacement after an auction.
        fn finish_region_owner_replacement(
            region_id: RegionId,
            current_block_number: BlockNumberFor<T>,
        ) -> DispatchResult {
            let auction_info =
                RegionReplacementAuctions::<T>::take(region_id).ok_or(Error::<T>::NoAuction)?;
            let mut region_info =
                RegionDetails::<T>::get(region_id).ok_or(Error::<T>::RegionUnknown)?;

            // Update region owner if there is a winning bidder
            if let Some(new_owner) = auction_info.highest_bidder {
                // Release previous owner's collateral
                T::NativeCurrency::release(
                    &HoldReason::RegionDepositReserve.into(),
                    &region_info.owner,
                    region_info.collateral,
                    Precision::Exact,
                )?;
                let next_owner_change =
                    current_block_number.saturating_add(T::RegionOwnerChangePeriod::get());

                // Update region information
                region_info.owner = new_owner.clone();
                region_info.collateral = auction_info.collateral;
                region_info.next_owner_change = next_owner_change;
                region_info.active_strikes = Zero::zero();
                RegionDetails::<T>::insert(region_id, region_info);

                Self::deposit_event(Event::<T>::RegionOwnerChanged {
                    region_id,
                    new_owner,
                    next_owner_change,
                });
            }
            Ok(())
        }

        /// Processes a vote for a proposal.
        /// Handles updating vote records, holding funds, and calculating new voting powers.
        #[allow(clippy::type_complexity)]
        fn process_vote(
            current_vote: &mut VoteStats<T>,
            maybe_vote_record: &mut Option<VoteRecord<T>>,
            signer: &T::AccountId,
            region_id: RegionId,
            vote: &Vote,
            amount: T::Balance,
            hold_reason: &<T as Config>::RuntimeHoldReason,
        ) -> Result<(T::Balance, T::Balance, T::Balance), DispatchError> {
            // Release previous vote's funds if exists
            if let Some(previous_vote) = maybe_vote_record.take() {
                T::NativeCurrency::release(
                    hold_reason,
                    signer,
                    previous_vote.power,
                    Precision::Exact,
                )?;

                // Deduct previous voting power
                match previous_vote.vote {
                    Vote::Yes => {
                        current_vote.yes_voting_power.saturating_reduce(previous_vote.power)
                    }
                    Vote::No => current_vote.no_voting_power.saturating_reduce(previous_vote.power),
                    Vote::Abstain => {
                        current_vote.abstain_voting_power.saturating_reduce(previous_vote.power)
                    }
                }
            }

            // Hold funds for the new vote
            T::NativeCurrency::hold(hold_reason, signer, amount)?;

            // Update voting power based on vote type
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

            // Record the new vote
            *maybe_vote_record = Some(VoteRecord { vote: vote.clone(), region_id, power: amount });
            Ok((
                current_vote.yes_voting_power,
                current_vote.no_voting_power,
                current_vote.abstain_voting_power,
            ))
        }

        /// Processes a bid for a region auction.
        fn process_bid(
            auction: &mut RegionAuction<T>,
            signer: &T::AccountId,
            amount: T::Balance,
            hold_reason: &<T as Config>::RuntimeHoldReason,
        ) -> DispatchResult {
            match &auction.highest_bidder {
                Some(old_bidder) => {
                    // Must exceed current bid
                    ensure!(amount > auction.collateral, Error::<T>::BidTooLow);
                    if old_bidder == signer {
                        // If same bidder, hold additional amount
                        let additional = amount.saturating_sub(auction.collateral);
                        if additional > Zero::zero() {
                            T::NativeCurrency::hold(hold_reason, signer, additional)?;
                        }
                    } else {
                        // If new bidder, hold new amount and release old bidder's funds
                        T::NativeCurrency::hold(hold_reason, signer, amount)?;
                        T::NativeCurrency::release(
                            hold_reason,
                            old_bidder,
                            auction.collateral,
                            Precision::Exact,
                        )?;
                    }
                }
                None => {
                    // Ensure the bid meets the minimum requirement
                    ensure!(amount >= auction.collateral, Error::<T>::BidBelowMinimum);
                    T::NativeCurrency::hold(hold_reason, signer, amount)?;
                }
            }
            // Update auction state
            auction.highest_bidder = Some(signer.clone());
            auction.collateral = amount;
            Ok(())
        }
    }
}

/// Trait for querying region-related information and operations.
pub trait RegionTrait {
    type Info;

    // Retrieves the details of a specific region by its ID.
    fn get_region_details(region_id: RegionId) -> Option<Self::Info>;

    // Checks if a region does exist.
    fn is_region(region_id: RegionId) -> bool;
}

impl<T: Config> RegionTrait for Pallet<T> {
    type Info = RegionInfoOf<T>;

    /// Retrieves the details of a specific region by its ID.
    fn get_region_details(region_id: RegionId) -> Option<Self::Info> {
        RegionDetails::<T>::get(region_id)
    }

    /// Checks if a region does exist.
    fn is_region(region_id: RegionId) -> bool {
        RegionDetails::<T>::contains_key(region_id)
    }
}
