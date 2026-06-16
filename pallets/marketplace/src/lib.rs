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
pub use weights::WeightInfo;

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

pub mod types;

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;
pub mod weights;
pub use weights::*;

use frame_support::{
    storage::bounded_btree_map::BoundedBTreeMap,
    traits::{
        fungible::{Inspect, Mutate, MutateHold},
        fungibles::Mutate as FungiblesMutate,
        fungibles::MutateFreeze,
        fungibles::MutateHold as FungiblesHold,
        tokens::Preservation,
        tokens::{fungible, fungibles, Balance, Precision, WithdrawConsequence},
        EnsureOriginWithArg,
    },
    PalletId,
};

use frame_support::sp_runtime::{
    traits::{
        AccountIdConversion, BlockNumberProvider, CheckedAdd, CheckedDiv, CheckedMul, CheckedSub,
        Zero,
    },
    Perbill, Percent, Permill, Saturating,
};

use parity_scale_codec::Codec;

use primitives::{IncomeSettlement, MarketplaceFreezeReason, MarketplaceHoldReason};

use types::*;

use pallet_real_world_asset::{
    traits::{
        PropertySharesInspect, PropertySharesManage, PropertySharesOwnership, PropertySharesSpvControl,
    },
    PropertyAssetDetails,
};

use pallet_xcavate_whitelist::{Role, RolePermission};

use pallet_regions::{LawyerInfo, LawyerManagement, RegionInfo, RegionTrait};

type AccountIdOf<T> = <T as frame_system::Config>::AccountId;

pub type LocalAssetIdOf<T> = <<T as pallet::Config>::LocalCurrency as fungibles::Inspect<
    <T as frame_system::Config>::AccountId,
>>::AssetId;

pub type ForeignAssetIdOf<T> = <<T as pallet::Config>::ForeignCurrency as fungibles::Inspect<
    <T as frame_system::Config>::AccountId,
>>::AssetId;

#[frame_support::pallet]
pub mod pallet {
    use super::*;
    use frame_support::pallet_prelude::*;
    use frame_system::pallet_prelude::*;

    #[pallet::pallet]
    pub struct Pallet<T>(_);

    #[pallet::composite_enum]
    pub enum HoldReason {
        #[codec(index = 0)]
        ListingDepositReserve,
    }

    /// The module configuration trait.
    #[pallet::config]
    pub trait Config: frame_system::Config + pallet_nft_fractionalization::Config {
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
            + fungible::BalancedHold<AccountIdOf<Self>, Balance = <Self as pallet::Config>::Balance>
            + fungible::hold::Inspect<Self::AccountId>
            + fungible::hold::Mutate<
                Self::AccountId,
                Reason = <Self as pallet::Config>::RuntimeHoldReason,
            >;

        /// The overarching hold reason.
        type RuntimeHoldReason: From<HoldReason>;

        /// The currency for property shares.
        type LocalCurrency: fungibles::InspectEnumerable<
                AccountIdOf<Self>,
                Balance = <Self as pallet::Config>::Balance,
                AssetId = u32,
            > + fungibles::metadata::Inspect<AccountIdOf<Self>, AssetId = u32>
            + fungibles::metadata::Mutate<AccountIdOf<Self>, AssetId = u32>
            + fungibles::Mutate<AccountIdOf<Self>, Balance = <Self as pallet::Config>::Balance>
            + fungibles::Inspect<AccountIdOf<Self>, Balance = <Self as pallet::Config>::Balance>;

        /// The currency for payments.
        type ForeignCurrency: fungibles::InspectEnumerable<
                AccountIdOf<Self>,
                Balance = <Self as pallet::Config>::Balance,
                AssetId = u32,
            > + fungibles::metadata::Inspect<AccountIdOf<Self>, AssetId = u32>
            + fungibles::metadata::Mutate<AccountIdOf<Self>, AssetId = u32>
            + fungibles::Mutate<AccountIdOf<Self>, Balance = <Self as pallet::Config>::Balance>
            + fungibles::Inspect<AccountIdOf<Self>, Balance = <Self as pallet::Config>::Balance>;

        /// Handler for holding foreign assets.
        type ForeignAssetsHolder: fungibles::MutateHold<
                AccountIdOf<Self>,
                AssetId = u32,
                Balance = <Self as pallet::Config>::Balance,
                Reason = MarketplaceHoldReason,
            > + fungibles::InspectHold<AccountIdOf<Self>, AssetId = u32>;

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

        /// Pallet ID for deriving the marketplace's sovereign account.
        #[pallet::constant]
        type PalletId: Get<PalletId>;

        /// The minimum amount of shares of a property.
        #[pallet::constant]
        type MinPropertyShares: Get<u32>;

        /// The maximum amount of shares of a property.
        #[pallet::constant]
        type MaxPropertyShares: Get<u32>;

        /// Asset id type from pallet NFT fractionalization.
        type AssetId: IsType<<Self as pallet_nft_fractionalization::Config>::AssetId>
            + Parameter
            + From<u32>
            + Ord
            + Copy;

        /// The Trasury's pallet ID, used for deriving its sovereign account ID.
        #[pallet::constant]
        type TreasuryId: Get<PalletId>;

        /// Deposit required for listing a property.
        #[pallet::constant]
        type ListingDeposit: Get<<Self as pallet::Config>::Balance>;

        /// The fee percentage charged by the marketplace (e.g., 1% as Perbill).
        #[pallet::constant]
        type MarketplaceFeePercentage: Get<Perbill>;

        /// Accepted assets for payments (e.g., USDC, USDT).
        #[pallet::constant]
        type AcceptedAssets: Get<[u32; 2]>;

        /// Maximum number of accepted assets.
        #[pallet::constant]
        type MaxAcceptedAssets: Get<u32>;

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

        /// The amount of time given to vote for a lawyer proposal.
        #[pallet::constant]
        type LawyerVotingTime: Get<BlockNumberFor<Self>>;

        /// The amount of time given for the lawyer to handle the legal process.
        #[pallet::constant]
        type LegalProcessTime: Get<BlockNumberFor<Self>>;

        /// Whitelist for role-based permissions.
        type Whitelist: pallet_xcavate_whitelist::RolePermission<Self::AccountId>;

        /// Origin type used to verify that an account has a specific Role.
        type PermissionOrigin: EnsureOriginWithArg<
            Self::RuntimeOrigin,
            Role,
            Success = Self::AccountId,
        >;

        /// Origin type used to verify that an account has a specific Role and is compliant.
        type CompliantOrigin: EnsureOriginWithArg<
            Self::RuntimeOrigin,
            Role,
            Success = Self::AccountId,
        >;

        /// Minimum quorum that needs to be reached for a proposal to pass.
        #[pallet::constant]
        type MinVotingQuorum: Get<Percent>;

        /// Time window for claiming property shares.
        #[pallet::constant]
        type ClaimWindow: Get<BlockNumberFor<Self>>;

        /// Maximum attempts to relist unclaimed shares.
        #[pallet::constant]
        type MaxRelistAttempts: Get<u8>;

        /// Provider for the block number. Normally this is the `frame_system` pallet.
        type BlockNumberProvider: BlockNumberProvider<BlockNumber = BlockNumberFor<Self>>;

        /// Income settlement for distributing funds.
        type IncomeSettlement: IncomeSettlement<AccountId = Self::AccountId>;

        /// Provider for region and lawyer information.
        type RegionProvider: RegionTrait<
                Info = RegionInfo<
                    AccountIdOf<Self>,
                    <Self as pallet::Config>::Balance,
                    BlockNumberFor<Self>,
                    <Self as pallet::Config>::NftCollectionId,
                >,
                LocationIdentifier = LocationId<Self>,
            > + LawyerManagement<
                AccountIdOf<Self>,
                LawyerInfo = LawyerInfo<<Self as pallet::Config>::Balance>,
            >;

        /// The maximum length of a name or symbol stored on-chain.
        #[pallet::constant]
        type StringLimit: Get<u32>;

        /// The maximum length of data stored in for post codes.
        #[pallet::constant]
        type PostcodeLimit: Get<u32>;

        /// The maximum ownership percentage allowed for any single investor.
        #[pallet::constant]
        type MaxOwnershipPercentage: Get<Perbill>;
    }

    pub type RegionId = u16;
    pub type ListingId = u32;
    pub type ProposalId = u64;
    pub type LocationId<T> = BoundedVec<u8, <T as pallet::Config>::PostcodeLimit>;

    pub(super) type PropertyListingDetailsType<T> = PropertyListingDetails<
        <T as pallet::Config>::NftId,
        <T as pallet::Config>::NftCollectionId,
        T,
    >;

    pub(super) type ListingDetailsType<T> = ShareListingDetails<
        <T as pallet::Config>::NftId,
        <T as pallet::Config>::NftCollectionId,
        T,
    >;

    /// Storage for the next listing ID.
    #[pallet::storage]
    pub(super) type NextListingId<T: Config> = StorageValue<_, ListingId, ValueQuery>;

    /// Storage for the next offer nonce.
    #[pallet::storage]
    pub(super) type NextOfferNonce<T: Config> = StorageValue<_, u64, ValueQuery>;

    /// Storage for ongoing property listings, mapping listing ID to details.
    #[pallet::storage]
    pub(super) type OngoingObjectListing<T: Config> =
        StorageMap<_, Blake2_128Concat, ListingId, PropertyListingDetailsType<T>, OptionQuery>;

    /// Storage for share ownership, mapping account ID and listing ID to share amounts.
    #[pallet::storage]
    pub(super) type ShareOwner<T: Config> = StorageDoubleMap<
        _,
        Blake2_128Concat,
        AccountIdOf<T>,
        Blake2_128Concat,
        ListingId,
        ShareOwnerDetails<T>,
        OptionQuery,
    >;

    /// Storage for share listings, mapping listing ID to listing details.
    #[pallet::storage]
    pub(super) type ShareListings<T: Config> =
        StorageMap<_, Blake2_128Concat, ListingId, ListingDetailsType<T>, OptionQuery>;

    /// Storage for ongoing offers, mapping listing ID and offeror to offer details.
    #[pallet::storage]
    pub(super) type OngoingOffers<T: Config> = StorageDoubleMap<
        _,
        Blake2_128Concat,
        ListingId,
        Blake2_128Concat,
        AccountIdOf<T>,
        OfferDetails<T>,
        OptionQuery,
    >;

    /// Storage for lawyer details related to a listing.
    #[pallet::storage]
    pub type PropertyLawyer<T: Config> =
        StorageMap<_, Blake2_128Concat, ListingId, PropertyLawyerDetails<T>, OptionQuery>;

    /// Storage for refund information.
    #[pallet::storage]
    pub type RefundShare<T: Config> =
        StorageMap<_, Blake2_128Concat, ListingId, RefundInfos<T>, OptionQuery>;

    /// Stores required infos in case of a refund.
    #[pallet::storage]
    pub type RefundClaimedShare<T: Config> =
        StorageMap<_, Blake2_128Concat, ListingId, u32, OptionQuery>;

    /// Stores required infos in case of a refund is a legal process expired.
    #[pallet::storage]
    pub type RefundLegalExpired<T: Config> =
        StorageMap<_, Blake2_128Concat, ListingId, u32, OptionQuery>;

    /// Storage for listing deposits, mapping listing ID to depositor and amount.
    #[pallet::storage]
    pub type ListingDeposits<T: Config> = StorageMap<
        _,
        Blake2_128Concat,
        ListingId,
        (AccountIdOf<T>, <T as pallet::Config>::Balance),
    >;

    /// Mapping of the listing to the real estate developer lawyer proposals.
    #[pallet::storage]
    pub type ProposedLawyers<T: Config> =
        StorageMap<_, Blake2_128Concat, ListingId, ProposedDeveloperLawyer<T>, OptionQuery>;

    /// Mapping of listing to the ongoing spv lawyer proposal.
    #[pallet::storage]
    pub type SpvLawyerProposal<T: Config> =
        StorageMap<_, Blake2_128Concat, ProposalId, ProposedSpvLawyer<T>, OptionQuery>;

    /// Storage for ongoing lawyer voting statistics.
    #[pallet::storage]
    pub type OngoingLawyerVoting<T: Config> =
        StorageMap<_, Blake2_128Concat, ProposalId, VoteStats, OptionQuery>;

    /// Storage for ongoing lawyer voting statistics.
    #[pallet::storage]
    pub(super) type UserLawyerVote<T: Config> = StorageDoubleMap<
        _,
        Blake2_128Concat,
        ProposalId,
        Blake2_128Concat,
        AccountIdOf<T>,
        VoteRecord,
        OptionQuery,
    >;

    /// Storage for mapping listings to SPV proposals.
    #[pallet::storage]
    pub type ListingSpvProposal<T: Config> =
        StorageMap<_, Blake2_128Concat, ListingId, ProposalId, OptionQuery>;

    /// Counter of proposal ids.
    #[pallet::storage]
    pub type ProposalCounter<T: Config> = StorageValue<_, ProposalId, ValueQuery>;

    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {
        /// A new property has been listed on the marketplace.
        ObjectListed {
            listing_index: ListingId,
            collection_index: <T as pallet::Config>::NftCollectionId,
            item_index: <T as pallet::Config>::NftId,
            asset_id: u32,
            share_price: <T as pallet::Config>::Balance,
            share_amount: u32,
            total_valuation: <T as pallet::Config>::Balance,
            seller: AccountIdOf<T>,
            tax_paid_by_developer: bool,
            listing_expiry: BlockNumberFor<T>,
            metadata_blob: BoundedVec<u8, <T as pallet::Config>::StringLimit>,
        },
        /// Relisted shares have been bought.
        RelistedSharesBought {
            listing_index: ListingId,
            asset_id: u32,
            buyer: AccountIdOf<T>,
            seller: AccountIdOf<T>,
            price: <T as pallet::Config>::Balance,
            amount: u32,
            payment_asset: u32,
            new_amount_remaining: u32,
        },
        /// Property shares have been purchased.
        PropertySharesBought {
            listing_index: ListingId,
            asset_id: u32,
            buyer: AccountIdOf<T>,
            amount_purchased: u32,
            price_paid: <T as pallet::Config>::Balance,
            tax_paid: <T as pallet::Config>::Balance,
            payment_asset: u32,
            new_shares_remaining: u32,
        },
        /// Shares have been relisted.
        SharesRelisted {
            listing_index: ListingId,
            asset_id: u32,
            price: <T as pallet::Config>::Balance,
            share_amount: u32,
            seller: AccountIdOf<T>,
        },
        /// The property has been delisted.
        ListingDelisted { listing_index: ListingId },
        /// The price of the listed object has been updated.
        ObjectUpdated { listing_index: ListingId, new_price: <T as pallet::Config>::Balance },
        /// A new offer has been created.
        OfferCreated {
            listing_id: ListingId,
            offeror: AccountIdOf<T>,
            price: <T as pallet::Config>::Balance,
            amount: u32,
            payment_asset: u32,
        },
        /// An offer has been cancelled.
        OfferCancelled { listing_id: ListingId, account_id: AccountIdOf<T> },
        /// A real estate developer lawyer has proposed handling a property.
        DeveloperLawyerProposed {
            listing_id: ListingId,
            lawyer: AccountIdOf<T>,
            proposed_cost: <T as pallet::Config>::Balance,
        },
        /// An SPV lawyer has proposed handling a property.
        SpvLawyerProposed {
            listing_id: ListingId,
            lawyer: AccountIdOf<T>,
            proposed_cost: <T as pallet::Config>::Balance,
            expiry_block: BlockNumberFor<T>,
        },
        /// A lawyer stepped back from a legal case.
        LawyerRemovedFromCase { lawyer: AccountIdOf<T>, listing_id: ListingId },
        /// Documents have been approved or rejected.
        DocumentsConfirmed {
            signer: AccountIdOf<T>,
            listing_id: ListingId,
            legal_side: LegalProperty,
            approve: bool,
        },
        /// A property sale has been completed.
        PrimarySaleCompleted {
            listing_id: ListingId,
            asset_id: u32,
            payouts: FinalSettlementPayouts<T>,
        },
        /// Funds has been withdrawn.
        RejectedFundsWithdrawn { signer: AccountIdOf<T>, listing_id: ListingId },
        /// Funds have been refunded after expired listing.
        ExpiredFundsWithdrawn { signer: AccountIdOf<T>, listing_id: ListingId },
        /// An offer has been accepted.
        OfferAccepted {
            listing_id: ListingId,
            offeror: AccountIdOf<T>,
            amount: u32,
            price: <T as pallet::Config>::Balance,
        },
        /// An offer has been Rejected.
        OfferRejected {
            listing_id: ListingId,
            offeror: AccountIdOf<T>,
            amount: u32,
            price: <T as pallet::Config>::Balance,
        },
        /// A investment has been cancelled.
        InvestmentCancelled {
            listing_id: ListingId,
            investor: AccountIdOf<T>,
            amount_returned: u32,
            new_shares_remaining: u32,
            refunds: BoundedBTreeMap<
                u32,
                (<T as pallet::Config>::Balance, <T as pallet::Config>::Balance), // (principal, tax)
                <T as pallet::Config>::MaxAcceptedAssets,
            >,
        },
        /// Property shares have been transferred.
        PropertySharesSent {
            asset_id: u32,
            sender: AccountIdOf<T>,
            receiver: AccountIdOf<T>,
            amount: u32,
        },
        /// The deposit of the real estate developer has been released.
        DeveloperDepositReturned {
            listing_id: ListingId,
            developer: AccountIdOf<T>,
            amount: <T as pallet::Config>::Balance,
        },
        /// Someone has voted on a lawyer proposal.
        VotedOnLawyer {
            listing_id: ListingId,
            voter: AccountIdOf<T>,
            vote: Vote,
            voting_power: u32,
            new_yes_power: u32,
            new_no_power: u32,
            new_abstain_power: u32,
            proposal_id: ProposalId,
        },
        /// A real estate lawyer proposal has been finalized.
        RealEstateLawyerProposalFinalized {
            listing_id: ListingId,
            lawyer: AccountIdOf<T>,
            is_approved: bool,
        },
        /// An SPV lawyer vote has been finalized.
        SpvLawyerVoteFinalized {
            listing_id: ListingId,
            lawyer: AccountIdOf<T>,
            is_approved: bool,
            final_yes_power: u32,
            final_no_power: u32,
            final_abstain_power: u32,
        },
        /// Property shares have been claimed.
        PropertySharesClaimed {
            listing_id: ListingId,
            asset_id: u32,
            owner: AccountIdOf<T>,
            amount: u32,
        },
        /// An SPV has been created for a property.
        SpvCreated { listing_id: ListingId, asset_id: u32 },
        /// All shares of a property have been sold.
        PrimarySaleSoldOut { listing_id: ListingId, asset_id: u32 },
        /// All property shares have been claimed.
        AllPropertySharesClaimed {
            listing_id: ListingId,
            asset_id: u32,
            legal_process_expiry_block: BlockNumberFor<T>,
        },
        /// A user has unfrozen his shares.
        SharesUnfrozen { proposal_id: ProposalId, asset_id: u32, voter: AccountIdOf<T>, amount: u32 },
        /// Lawyer costs have been allocated.
        LawyerCostsAllocated {
            listing_id: ListingId,
            lawyer_account: AccountIdOf<T>,
            lawyer_type: LegalProperty,
            costs: BoundedBTreeMap<
                u32,
                <T as pallet::Config>::Balance,
                <T as pallet::Config>::MaxAcceptedAssets,
            >,
        },
        /// Unclaimed shares have been relisted.
        UnclaimedRelisted { listing_id: ListingId, amount: u32, relist_count: u8 },
        /// Unclaimed shares have been withdrawn.
        UnclaimedSharesWithdrawn {
            listing_id: ListingId,
            investor: AccountIdOf<T>,
            refunds: BoundedBTreeMap<
                u32,
                (<T as pallet::Config>::Balance, <T as pallet::Config>::Balance), // (principal, tax)
                <T as pallet::Config>::MaxAcceptedAssets,
            >,
        },
        /// A sale has been cancelled due to unclaimed shares.
        SaleCancelledUnclaimed { listing_id: ListingId, unclaimed_amount: u32 },
    }

    // Errors inform users that something went wrong.
    #[pallet::error]
    pub enum Error<T> {
        /// This index is not taken.
        InvalidIndex,
        /// The buyer doesn't have enough funds.
        NotEnoughFunds,
        /// Not enough shares available to buy.
        NotEnoughSharesAvailable,
        /// Error by dividing a number.
        DivisionError,
        /// Error by multiplying a number.
        MultiplyError,
        /// No sufficient permission.
        NoPermission,
        /// User did not pass the kyc.
        UserNotCompliant,
        /// Underflow in arithmetic operations.
        ArithmeticUnderflow,
        /// Overflow in arithmetic operations.
        ArithmeticOverflow,
        /// The share is not for sale.
        ShareNotForSale,
        /// This Region is not known.
        RegionUnknown,
        /// The location is not registered.
        LocationUnknown,
        /// The object can not be divided in so many shares.
        TooManyShares,
        /// The object needs more shares.
        ShareAmountTooLow,
        /// A user can only make one offer per listing.
        OnlyOneOfferPerUser,
        /// The lawyer job has already been taken.
        LawyerJobTaken,
        /// A lawyer has not been set.
        LawyerNotFound,
        /// The lawyer already submitted his answer.
        AlreadyConfirmed,
        /// The costs of the lawyer can't be that high.
        CostsTooHigh,
        /// This Asset is not supported for payment.
        AssetNotSupported,
        /// This Asset is not supported for payment.
        PaymentAssetNotSupported,
        /// Exceeds maximum allowed entries.
        ExceedsMaxEntries,
        /// The property is not refunded.
        SharesNotRefunded,
        /// The property is already sold.
        PropertyAlreadySold,
        /// Listing has already expired.
        ListingExpired,
        /// Signer has not bought any shares.
        NoSharesBought,
        /// The listing has not expired.
        ListingNotExpired,
        /// Price of a share can not be zero.
        InvalidSharePrice,
        /// Share amount can not be zero.
        AmountCannotBeZero,
        /// Marketplace fee needs to be below 100 %.
        InvalidFeePercentage,
        /// The sender has not enough shares.
        NotEnoughShares,
        /// Shares have not been returned yet.
        SharesNotReturned,
        /// The real estate object could not be found.
        NoObjectFound,
        /// The lawyer has no permission for this region.
        WrongRegion,
        /// Share owner has not been found.
        ShareOwnerNotFound,
        /// No lawyer has been proposed to vote on.
        NoLawyerProposed,
        /// There is already a lawyer proposal ongoing.
        LawyerProposalOngoing,
        /// The propal has expired.
        VotingExpired,
        /// The voting is still ongoing.
        VotingStillOngoing,
        /// Property has not been sold yet.
        PropertyHasNotBeenSoldYet,
        /// The legal process was not finished on time.
        LegalProcessFailed,
        /// The legal process is currently ongoing.
        LegalProcessOngoing,
        /// The user has no share amount frozen.
        NoFrozenAmount,
        /// The user has no share amount frozen.
        NoClaimWindow,
        /// The claim window already expired.
        ClaimWindowExpired,
        /// The claim period is still ongoing.
        ClaimWindowNotExpired,
        /// The user still has unclaimed shares.
        StillHasUnclaimedShares,
        /// The user does not have any valid shares to claim.
        NoValidSharesToClaim,
        /// The user is not allowed to own too many shares of a certain property.
        ExceedsMaxOwnership,
        /// The listing does not exist.
        ListingNotFound,
        /// All property shares have already been claimed.
        AllSharesClaimed,
        /// The offer does not exist.
        OfferNotFound,
        /// The user does not own any shares of the property.
        NoSharesOwned,
        /// There are not enough shares available to refund.
        InsufficientRefundableShares,
        /// The amount for voting has to be higher than 0.
        ZeroVoteAmount,
        /// The nonce does not match the nonce for this offer.
        InvalidOfferNonce,
    }

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        /// List a real estate object. A new nft gets minted.
        /// This function calls the nfts-pallet to mint a new nft and sets the Metadata.
        ///
        /// The origin must be Signed by a RealEstateDeveloper and have sufficient funds.
        ///
        /// Parameters:
        /// - `region`: The region where the object is located.
        /// - `location`: The location where the object is located.
        /// - `share_price`: The price of a single share.
        /// - `share_amount`: The amount of shares for a object.
        /// - `data`: The Metadata of the nft.
        /// - `tax_paid_by_developer`: Bool if the tax is paid by the real estate developer or not.
        ///
        /// Emits `ObjectListed` event when successful
        #[pallet::call_index(0)]
        #[pallet::weight(<T as pallet::Config>::WeightInfo::list_property(
            <T as pallet::Config>::StringLimit::get()
        ))]
        pub fn list_property(
            origin: OriginFor<T>,
            region: RegionId,
            location: LocationId<T>,
            share_price: <T as pallet::Config>::Balance,
            share_amount: u32,
            data: BoundedVec<u8, <T as pallet::Config>::StringLimit>,
            tax_paid_by_developer: bool,
        ) -> DispatchResult {
            let signer = <T as pallet::Config>::CompliantOrigin::ensure_origin(
                origin,
                &Role::RealEstateDeveloper,
            )?;
            // Validate share bounds
            ensure!(share_amount > 0, Error::<T>::AmountCannotBeZero);
            ensure!(
                share_amount <= <T as pallet::Config>::MaxPropertyShares::get(),
                Error::<T>::TooManyShares
            );
            ensure!(share_amount >= T::MinPropertyShares::get(), Error::<T>::ShareAmountTooLow);
            ensure!(!share_price.is_zero(), Error::<T>::InvalidSharePrice);

            let region_info = <T as pallet::Config>::RegionProvider::get_region_details(region)
                .ok_or(Error::<T>::RegionUnknown)?;
            ensure!(
                <T as pallet::Config>::RegionProvider::location_registered(
                    region,
                    location.clone()
                ),
                Error::<T>::LocationUnknown
            );
            // Set up listing details
            let listing_id = NextListingId::<T>::get();
            let current_block_number =
                <T as pallet::Config>::BlockNumberProvider::current_block_number();
            let listing_duration = region_info.listing_duration;
            let listing_expiry = current_block_number.saturating_add(listing_duration);

            // Initialize funds map for accepted assets
            let mut collected_funds = BoundedBTreeMap::default();
            for &asset_id in T::AcceptedAssets::get().iter() {
                collected_funds
                    .try_insert(asset_id, Zero::zero())
                    .map_err(|_| Error::<T>::ExceedsMaxEntries)?;
            }

            // Calculate total property price
            let property_price = share_price
                .checked_mul(&((share_amount as u128).into()))
                .ok_or(Error::<T>::MultiplyError)?;
            let deposit_amount = T::ListingDeposit::get();

            // Check if signer has sufficient funds for deposit
            match <T as pallet::Config>::NativeCurrency::can_withdraw(&signer, deposit_amount) {
                WithdrawConsequence::Success => {}
                _ => return Err(Error::<T>::NotEnoughFunds.into()),
            }

            // Create property shares and store details
            let (item_id, asset_number) = T::PropertyShares::create_property_shares(
                &signer,
                region,
                location,
                share_amount,
                property_price,
                data.clone(),
            )?;

            // Build and store listing details
            let property_details = PropertyListingDetails {
                real_estate_developer: signer.clone(),
                share_price,
                collected_funds: collected_funds.clone(),
                collected_tax: collected_funds.clone(),
                collected_fees: collected_funds,
                asset_id: asset_number,
                item_id,
                collection_id: region_info.collection_id,
                share_amount,
                listed_share_amount: share_amount,
                tax_paid_by_developer,
                tax: region_info.tax,
                listing_expiry,
                investor_funds: Default::default(),
                claim_expiry: None,
                relist_count: Zero::zero(),
                unclaimed_share_amount: Zero::zero(),
            };
            OngoingObjectListing::<T>::insert(listing_id, property_details);

            // Hold the listing deposit
            <T as pallet::Config>::NativeCurrency::hold(
                &HoldReason::ListingDepositReserve.into(),
                &signer,
                deposit_amount,
            )?;
            ListingDeposits::<T>::insert(listing_id, (&signer, deposit_amount));

            let next_listing_id = Self::next_listing_id(listing_id)?;

            NextListingId::<T>::put(next_listing_id);

            Self::deposit_event(Event::<T>::ObjectListed {
                listing_index: listing_id,
                collection_index: region_info.collection_id,
                item_index: item_id,
                asset_id: asset_number,
                share_price,
                share_amount,
                total_valuation: property_price,
                seller: signer,
                tax_paid_by_developer,
                listing_expiry,
                metadata_blob: data,
            });
            Ok(())
        }

        /// Buy listed shares from the marketplace.
        ///
        /// The origin must be Signed by a compliant RealEstateInvestor and have sufficient funds.
        ///
        /// Parameters:
        /// - `listing_id`: The listing that the investor wants to buy shares from.
        /// - `amount`: The amount of shares that the investor wants to buy.
        /// - `payment_asset`: Asset in which the investor wants to pay.
        ///
        /// Emits `PropertySharesBought` event when successful.
        #[pallet::call_index(1)]
        #[pallet::weight(<T as pallet::Config>::WeightInfo::buy_property_shares_all_shares(
            <T as pallet::Config>::MaxPropertyShares::get(),
            <T as pallet::Config>::AcceptedAssets::get().len() as u32,
        ))]
        pub fn buy_property_shares(
            origin: OriginFor<T>,
            listing_id: ListingId,
            amount: u32,
            payment_asset: u32,
        ) -> DispatchResult {
            let signer = <T as pallet::Config>::CompliantOrigin::ensure_origin(
                origin,
                &Role::RealEstateInvestor,
            )?;

            // Validate input parameters
            ensure!(amount > 0, Error::<T>::AmountCannotBeZero);
            let accepted_payment_assets = T::AcceptedAssets::get();
            ensure!(
                accepted_payment_assets.contains(&payment_asset),
                Error::<T>::PaymentAssetNotSupported
            );

            // Retrieve and validate listing details
            let mut property_details =
                OngoingObjectListing::<T>::get(listing_id).ok_or(Error::<T>::ShareNotForSale)?;
            ensure!(
                property_details.listed_share_amount >= amount,
                Error::<T>::NotEnoughSharesAvailable
            );
            ensure!(
                property_details.listing_expiry
                    > <T as pallet::Config>::BlockNumberProvider::current_block_number(),
                Error::<T>::ListingExpired
            );
            let asset_details =
                T::PropertyShares::get_property_asset_info(property_details.asset_id)
                    .ok_or(Error::<T>::NoObjectFound)?;

            // Calculate fees and taxes
            let fee_percent = T::MarketplaceFeePercentage::get();
            ensure!(fee_percent < Perbill::from_percent(100), Error::<T>::InvalidFeePercentage);
            let tax_percent = property_details.tax;
            let total_supply = property_details.share_amount;
            let max_shares = T::MaxOwnershipPercentage::get().mul_floor(total_supply);
            let transfer_price = property_details
                .share_price
                .checked_mul(&((amount as u128).into()))
                .ok_or(Error::<T>::MultiplyError)?;
            // Rounding up to not undercharge for protocol fees.
            let fee = fee_percent.mul_ceil(transfer_price);
            // Rounding up to not undercharge for property tax.
            let tax = tax_percent.mul_ceil(transfer_price);

            let base_price =
                transfer_price.checked_add(&fee).ok_or(Error::<T>::ArithmeticOverflow)?;
            let total_transfer_price = if property_details.tax_paid_by_developer {
                base_price
            } else {
                base_price.checked_add(&tax).ok_or(Error::<T>::ArithmeticOverflow)?
            };

            // Hold funds for the purchase
            T::ForeignAssetsHolder::hold(
                payment_asset,
                &MarketplaceHoldReason::Marketplace,
                &signer,
                total_transfer_price,
            )?;

            // Update share amounts in listing
            property_details.listed_share_amount = property_details
                .listed_share_amount
                .checked_sub(amount)
                .ok_or(Error::<T>::ArithmeticUnderflow)?;
            property_details.unclaimed_share_amount = property_details
                .unclaimed_share_amount
                .checked_add(amount)
                .ok_or(Error::<T>::ArithmeticOverflow)?;
            // Update or create share owner details
            ShareOwner::<T>::try_mutate_exists(&signer, listing_id, |maybe_share_owner_details| {
                if maybe_share_owner_details.is_none() {
                    let initial_funds = Self::create_initial_funds()?;
                    *maybe_share_owner_details = Some(ShareOwnerDetails {
                        share_amount: 0,
                        paid_funds: initial_funds.clone(),
                        paid_tax: initial_funds,
                        relist_count: property_details.relist_count,
                    });
                }
                let share_owner_details =
                    maybe_share_owner_details.as_mut().ok_or(Error::<T>::ShareOwnerNotFound)?;
                // Check that the relist count matches to prevent buying shares if he still has unclaimed shares
                ensure!(
                    share_owner_details.relist_count == property_details.relist_count,
                    Error::<T>::StillHasUnclaimedShares
                );
                // Ensure max ownership share is not exceeded
                let claimed_share_amount = <T as pallet::Config>::PropertyShares::get_share_balance(
                    property_details.asset_id,
                    &signer,
                );
                let new_share_amount = share_owner_details
                    .share_amount
                    .checked_add(amount)
                    .ok_or(Error::<T>::ArithmeticOverflow)?;
                let total_investor_share_amount = new_share_amount
                    .checked_add(claimed_share_amount)
                    .ok_or(Error::<T>::ArithmeticOverflow)?;
                ensure!(total_investor_share_amount < max_shares, Error::<T>::ExceedsMaxOwnership);
                share_owner_details.share_amount = new_share_amount;
                // Update paid funds and tax
                Self::update_map(
                    &mut share_owner_details.paid_funds,
                    payment_asset,
                    transfer_price,
                )?;

                if !property_details.tax_paid_by_developer {
                    Self::update_map(&mut share_owner_details.paid_tax, payment_asset, tax)?;
                }

                Ok::<(), DispatchError>(())
            })?;

            // Handle sold-out case
            let asset_id = property_details.asset_id;
            let tax_paid_by_developer = property_details.tax_paid_by_developer;
            let listed_shares = property_details.listed_share_amount;
            if listed_shares == 0 {
                if asset_details.spv_created {
                    let current_block_number =
                        <T as pallet::Config>::BlockNumberProvider::current_block_number();
                    let expiry_block = current_block_number.saturating_add(T::ClaimWindow::get());
                    property_details.claim_expiry = Some(expiry_block);
                }
                Self::deposit_event(Event::<T>::PrimarySaleSoldOut { listing_id, asset_id });
            }

            OngoingObjectListing::<T>::insert(listing_id, &property_details);
            Self::deposit_event(Event::<T>::PropertySharesBought {
                listing_index: listing_id,
                asset_id,
                buyer: signer,
                amount_purchased: amount,
                price_paid: transfer_price,
                tax_paid: if !tax_paid_by_developer { tax } else { 0u128.into() },
                payment_asset,
                new_shares_remaining: listed_shares,
            });
            Ok(())
        }

        /// Claim purchased property shares once all shares are sold.
        ///
        /// The origin must be Signed by a compliant RealEstateInvestor and have sufficient funds.
        ///
        /// Parameters:
        /// - `listing_id`: The listing that the investor wants to claim shares from.
        ///
        /// Emits `PropertySharesClaimed` event when successful.
        #[pallet::call_index(2)]
        #[pallet::weight(<T as pallet::Config>::WeightInfo::claim_property_shares())]
        pub fn claim_property_shares(origin: OriginFor<T>, listing_id: ListingId) -> DispatchResult {
            let signer = <T as pallet::Config>::CompliantOrigin::ensure_origin(
                origin,
                &Role::RealEstateInvestor,
            )?;
            let mut property_details =
                OngoingObjectListing::<T>::get(listing_id).ok_or(Error::<T>::ListingNotFound)?;
            // Ensure SPV has been created for this property before allowing claims
            T::PropertyShares::ensure_spv_created(property_details.asset_id)?;
            let claim_expiry = property_details.claim_expiry.ok_or(Error::<T>::NoClaimWindow)?;
            let current_block_number =
                <T as pallet::Config>::BlockNumberProvider::current_block_number();
            ensure!(current_block_number < claim_expiry, Error::<T>::ClaimWindowExpired);
            // Retrieve share details for this investor and ensure they are eligible to claim
            let share_details =
                ShareOwner::<T>::take(&signer, listing_id).ok_or(Error::<T>::ShareOwnerNotFound)?;
            ensure!(
                share_details.relist_count == property_details.relist_count,
                Error::<T>::NoValidSharesToClaim
            );
            let property_account = Self::property_account_id(property_details.asset_id);
            let fee_percent = T::MarketplaceFeePercentage::get();
            ensure!(fee_percent < Perbill::from_percent(100), Error::<T>::InvalidFeePercentage);

            let tax_percent = if property_details.tax_paid_by_developer {
                property_details.tax
            } else {
                Permill::zero()
            };

            // Process each payment asset
            for (asset, paid_funds) in
                share_details.paid_funds.iter().filter(|(_, funds)| !funds.is_zero())
            {
                let default = Zero::zero();
                let paid_tax = share_details.paid_tax.get(asset).copied().unwrap_or(default);
                // Calculate investor's fee as 1% of paid_funds, rounding up to prevent undercharging
                let investor_fee = fee_percent.mul_ceil(*paid_funds);

                // Update collected funds, fees, and tax in property details
                Self::update_map(&mut property_details.collected_funds, *asset, *paid_funds)?;
                Self::update_map(&mut property_details.collected_fees, *asset, investor_fee)?;
                if !property_details.tax_paid_by_developer {
                    Self::update_map(&mut property_details.collected_tax, *asset, paid_tax)?;
                } else {
                    let tax = tax_percent.mul_ceil(*paid_funds);
                    Self::update_map(&mut property_details.collected_tax, *asset, tax)?;
                }

                // Total amount to unfreeze (paid_funds + fee + tax)
                let total_investor_amount = paid_funds
                    .checked_add(&investor_fee)
                    .ok_or(Error::<T>::ArithmeticOverflow)?
                    .checked_add(&paid_tax)
                    .ok_or(Error::<T>::ArithmeticOverflow)?;

                // Release held funds
                T::ForeignAssetsHolder::release(
                    *asset,
                    &MarketplaceHoldReason::Marketplace,
                    &signer,
                    total_investor_amount,
                    Precision::Exact,
                )?;

                // Transfer funds to property account
                Self::transfer_funds(&signer, &property_account, total_investor_amount, *asset)?;

                // Track net contribution (price + tax) for final settlement
                let investor_net_contribution =
                    paid_funds.checked_add(&paid_tax).ok_or(Error::<T>::ArithmeticOverflow)?;

                // Update or insert investor funds in property details
                match property_details.investor_funds.get_mut(&signer) {
                    Some(share_funds) => {
                        let paid_funds = &mut share_funds.paid_funds;
                        if let Some(existing) = paid_funds.get_mut(asset) {
                            *existing = existing
                                .checked_add(&investor_net_contribution)
                                .ok_or(Error::<T>::ArithmeticOverflow)?;
                        } else {
                            paid_funds
                                .try_insert(*asset, investor_net_contribution)
                                .map_err(|_| Error::<T>::ExceedsMaxEntries)?;
                        }
                        let paid_fee = &mut share_funds.paid_fee;
                        if let Some(existing) = paid_fee.get_mut(asset) {
                            *existing = existing
                                .checked_add(&investor_fee)
                                .ok_or(Error::<T>::ArithmeticOverflow)?;
                        } else {
                            paid_fee
                                .try_insert(*asset, investor_fee)
                                .map_err(|_| Error::<T>::ExceedsMaxEntries)?;
                        }
                    }
                    None => {
                        let mut paid_funds = BoundedBTreeMap::new();
                        paid_funds
                            .try_insert(*asset, investor_net_contribution)
                            .map_err(|_| Error::<T>::ExceedsMaxEntries)?;
                        let mut paid_fee = BoundedBTreeMap::new();
                        paid_fee
                            .try_insert(*asset, investor_fee)
                            .map_err(|_| Error::<T>::ExceedsMaxEntries)?;

                        let new_entry = ShareOwnerFunds { paid_funds, paid_fee };
                        property_details
                            .investor_funds
                            .try_insert(signer.clone(), new_entry)
                            .map_err(|_| Error::<T>::ExceedsMaxEntries)?;
                    }
                }
            }

            // Distribute property shares
            let share_amount = share_details.share_amount;
            let asset_id = property_details.asset_id;

            T::PropertyShares::distribute_property_shares_to_owner(asset_id, &signer, share_amount)?;
            property_details.unclaimed_share_amount = property_details
                .unclaimed_share_amount
                .checked_sub(share_amount)
                .ok_or(Error::<T>::ArithmeticUnderflow)?;

            // If all shares have been claimed, trigger legal process setup.
            if property_details.unclaimed_share_amount.is_zero() {
                ensure!(
                    PropertyLawyer::<T>::get(listing_id).is_none(),
                    Error::<T>::LegalProcessOngoing
                );
                // Initialize legal process funds and set expiry time.
                let initial_funds = Self::create_initial_funds()?;
                let expiry_block = current_block_number.saturating_add(T::LegalProcessTime::get());
                let property_lawyer_details = PropertyLawyerDetails {
                    real_estate_developer_lawyer: None,
                    spv_lawyer: None,
                    real_estate_developer_status: DocumentStatus::Pending,
                    spv_status: DocumentStatus::Pending,
                    real_estate_developer_lawyer_costs: initial_funds.clone(),
                    spv_lawyer_costs: initial_funds,
                    legal_process_expiry: expiry_block,
                    second_attempt: false,
                };
                property_details.claim_expiry = None;
                PropertyLawyer::<T>::insert(listing_id, property_lawyer_details);
                Self::deposit_event(Event::<T>::AllPropertySharesClaimed {
                    listing_id,
                    asset_id,
                    legal_process_expiry_block: expiry_block,
                });
            }

            OngoingObjectListing::<T>::insert(listing_id, property_details);
            Self::deposit_event(Event::<T>::PropertySharesClaimed {
                listing_id,
                asset_id,
                owner: signer,
                amount: share_amount,
            });
            Ok(())
        }

        /// Finalizes a claim period once it is over.
        ///
        /// The origin must be signed and have sufficient funds.
        ///
        /// Parameters:
        /// - `listing_id`: The listing that the investor wants to finalize the claim window from
        ///
        /// Emits `PropertySharesClaimed` event when successful.
        #[pallet::call_index(3)]
        #[pallet::weight(<T as pallet::Config>::WeightInfo::finalize_claim_window())]
        pub fn finalize_claim_window(
            origin: OriginFor<T>,
            listing_id: ListingId,
        ) -> DispatchResult {
            let _ = ensure_signed(origin)?;
            let mut property_details =
                OngoingObjectListing::<T>::get(listing_id).ok_or(Error::<T>::ListingNotFound)?;
            // Ensure there claiming window is expired.
            let claim_expiry = property_details.claim_expiry.ok_or(Error::<T>::NoClaimWindow)?;
            let current_block = <T as pallet::Config>::BlockNumberProvider::current_block_number();
            ensure!(current_block > claim_expiry, Error::<T>::ClaimWindowNotExpired);

            let unclaimed_amount = property_details.unclaimed_share_amount;
            ensure!(unclaimed_amount > 0, Error::<T>::AllSharesClaimed);
            // CASE 1: Max relist attempts reached -> cancel sale and refund buyers
            if property_details.relist_count >= T::MaxRelistAttempts::get() {
                property_details.relist_count = property_details
                    .relist_count
                    .checked_add(1)
                    .ok_or(Error::<T>::ArithmeticOverflow)?;
                property_details.claim_expiry = None;
                OngoingObjectListing::<T>::insert(listing_id, &property_details);
                // Record total number of shares actually sold (for refund purposes).
                RefundClaimedShare::<T>::insert(
                    listing_id,
                    property_details.share_amount.saturating_sub(unclaimed_amount),
                );
                Self::deposit_event(Event::<T>::SaleCancelledUnclaimed {
                    listing_id,
                    unclaimed_amount,
                });
            // CASE 2: Relist unclaimed shares and reopen claim window
            } else {
                // Add unclaimed shares back to listed_share_amount for relisting.
                property_details.listed_share_amount = property_details
                    .listed_share_amount
                    .checked_add(unclaimed_amount)
                    .ok_or(Error::<T>::ArithmeticOverflow)?;
                property_details.relist_count = property_details
                    .relist_count
                    .checked_add(1)
                    .ok_or(Error::<T>::ArithmeticOverflow)?;
                // Reset unclaimed shares and claim window
                property_details.unclaimed_share_amount = 0;
                property_details.claim_expiry = None;
                let possible_listing_expiry = current_block.saturating_add(T::ClaimWindow::get());
                if property_details.listing_expiry < possible_listing_expiry {
                    property_details.listing_expiry = possible_listing_expiry;
                }
                OngoingObjectListing::<T>::insert(listing_id, &property_details);
                Self::deposit_event(Event::<T>::UnclaimedRelisted {
                    listing_id,
                    amount: unclaimed_amount,
                    relist_count: property_details.relist_count,
                });
            }
            Ok(())
        }

        /// Confirm that a spv has been created.
        ///
        /// The origin must be Signed by a SpvConfirmation and have sufficient funds.
        ///
        /// Parameters:
        /// - `listing_id`: The listing that the spv has been created for.
        ///
        /// Emits `SpvCreated` event when successful.
        #[pallet::call_index(4)]
        #[pallet::weight(<T as pallet::Config>::WeightInfo::create_spv())]
        pub fn create_spv(origin: OriginFor<T>, listing_id: ListingId) -> DispatchResult {
            let _ = <T as pallet::Config>::PermissionOrigin::ensure_origin(
                origin,
                &Role::SpvConfirmation,
            )?;
            let mut property_details =
                OngoingObjectListing::<T>::get(listing_id).ok_or(Error::<T>::NoObjectFound)?;
            // Ensure property has been fully sold.
            ensure!(
                property_details.listed_share_amount.is_zero(),
                Error::<T>::PropertyHasNotBeenSoldYet
            );
            let asset_id = property_details.asset_id;
            T::PropertyShares::ensure_spv_not_created(asset_id)?;
            // Register the SPV for this property.
            T::PropertyShares::register_spv(asset_id)?;
            // Set a claim window for investors to claim their shares after SPV creation.
            let current_block_number =
                <T as pallet::Config>::BlockNumberProvider::current_block_number();
            let expiry_block = current_block_number.saturating_add(T::ClaimWindow::get());
            property_details.claim_expiry = Some(expiry_block);
            OngoingObjectListing::<T>::insert(listing_id, property_details);
            Self::deposit_event(Event::<T>::SpvCreated { listing_id, asset_id });
            Ok(())
        }

        /// Relist shares on the marketplace.
        /// The property must be registered on the marketplace.
        ///
        /// The origin must be Signed by a RealEstateInvestor and have sufficient funds.
        ///
        /// Parameters:
        /// - `region`: The region where the object is located.
        /// - `item_id`: The item id of the nft.
        /// - `share_price`: The price of a single share.
        /// - `amount`: The amount of shares of the real estate object that should be listed.
        ///
        /// Emits `SharesRelisted` event when successful
        #[pallet::call_index(5)]
        #[pallet::weight(<T as pallet::Config>::WeightInfo::relist_shares())]
        pub fn relist_shares(
            origin: OriginFor<T>,
            asset_id: u32,
            share_price: <T as pallet::Config>::Balance,
            amount: u32,
        ) -> DispatchResult {
            let signer = <T as pallet::Config>::CompliantOrigin::ensure_origin(
                origin,
                &Role::RealEstateInvestor,
            )?;

            // Validate input parameters
            ensure!(amount > 0, Error::<T>::AmountCannotBeZero);
            ensure!(!share_price.is_zero(), Error::<T>::InvalidSharePrice);

            // Ensure property is finalized and get details
            let asset_details = T::PropertyShares::get_if_property_finalized(asset_id)?;

            // Transfer shares from seller to property account to hold during listing
            let property_account = Self::property_account_id(asset_id);
            <T as pallet::Config>::LocalCurrency::transfer(
                asset_id,
                &signer,
                &property_account,
                amount.into(),
                Preservation::Expendable,
            )?;

            // Create new listing
            let listing_id = NextListingId::<T>::get();
            let share_listing = ShareListingDetails {
                seller: signer.clone(),
                share_price,
                asset_id,
                item_id: asset_details.item_id,
                collection_id: asset_details.collection_id,
                amount,
            };
            ShareListings::<T>::insert(listing_id, share_listing);
            let next_listing_id = Self::next_listing_id(listing_id)?;
            NextListingId::<T>::put(next_listing_id);

            Self::deposit_event(Event::<T>::SharesRelisted {
                listing_index: listing_id,
                asset_id,
                price: share_price,
                share_amount: amount,
                seller: signer,
            });
            Ok(())
        }

        /// Buy shares from the marketplace.
        ///
        /// The origin must be Signed by a compliant RealEstateInvestor and have sufficient funds.
        ///
        /// Parameters:
        /// - `listing_id`: The listing that the investor wants to buy from.
        /// - `amount`: The amount of shares the investor wants to buy.
        /// - `payment_asset`: Asset in which the investor wants to pay.
        ///
        /// Emits `RelistedSharesBought` event when successful.
        #[pallet::call_index(6)]
        #[pallet::weight(<T as pallet::Config>::WeightInfo::buy_relisted_shares())]
        pub fn buy_relisted_shares(
            origin: OriginFor<T>,
            listing_id: ListingId,
            amount: u32,
            payment_asset: u32,
        ) -> DispatchResult {
            let buyer = <T as pallet::Config>::CompliantOrigin::ensure_origin(
                origin,
                &Role::RealEstateInvestor,
            )?;

            // Validate input parameters
            ensure!(
                T::AcceptedAssets::get().contains(&payment_asset),
                Error::<T>::PaymentAssetNotSupported
            );
            ensure!(amount > 0, Error::<T>::AmountCannotBeZero);

            // Retrieve and validate listing details
            let listing_details =
                ShareListings::<T>::take(listing_id).ok_or(Error::<T>::ShareNotForSale)?;
            ensure!(listing_details.amount >= amount, Error::<T>::NotEnoughSharesAvailable);

            // Restrict ownership to prevent exceeding limits
            Self::restrict_ownership(listing_details.asset_id, &buyer, amount)?;

            // Calculate total price
            let price = listing_details
                .share_price
                .checked_mul(&((amount as u128).into()))
                .ok_or(Error::<T>::MultiplyError)?;
            // Process the share purchase
            Self::buying_shares_process(
                listing_id,
                &buyer,
                &buyer,
                listing_details,
                price,
                amount,
                payment_asset,
            )?;
            Ok(())
        }

        /// Lets an investor cancel the property shares purchase.
        ///
        /// The origin must be Signed by a RealEstateInvestor and have sufficient funds.
        ///
        /// Parameters:
        /// - `listing_id`: The listing that the investor wants to buy from.
        ///
        /// Emits `InvestmentCancelled` event when successful.
        #[pallet::call_index(7)]
        #[pallet::weight(<T as pallet::Config>::WeightInfo::cancel_property_purchase())]
        pub fn cancel_property_purchase(
            origin: OriginFor<T>,
            listing_id: ListingId,
        ) -> DispatchResult {
            let signer = <T as pallet::Config>::PermissionOrigin::ensure_origin(
                origin,
                &Role::RealEstateInvestor,
            )?;
            let mut property_details =
                OngoingObjectListing::<T>::get(listing_id).ok_or(Error::<T>::ListingNotFound)?;
            // Ensure the listing has not expired (investor can only cancel while active).
            ensure!(
                property_details.listing_expiry
                    > <T as pallet::Config>::BlockNumberProvider::current_block_number(),
                Error::<T>::ListingExpired
            );
            // Ensure there are still shares available (cannot cancel after all are sold).
            ensure!(
                !property_details.listed_share_amount.is_zero(),
                Error::<T>::PropertyAlreadySold
            );

            // Retrieve share details for this investor and ensure they have shares to cancel.
            let share_details: ShareOwnerDetails<T> =
                ShareOwner::<T>::take(&signer, listing_id).ok_or(Error::<T>::ShareOwnerNotFound)?;
            ensure!(!share_details.share_amount.is_zero(), Error::<T>::NoSharesBought);

            // Process refunds
            let refunds = Self::unfreeze_shares_with_refunds(&share_details, &signer)?;
            // Add the cancelled share amount back to the listing so others can buy them.
            property_details.listed_share_amount = property_details
                .listed_share_amount
                .checked_add(share_details.share_amount)
                .ok_or(Error::<T>::ArithmeticOverflow)?;

            OngoingObjectListing::<T>::insert(listing_id, &property_details);

            Self::deposit_event(Event::<T>::InvestmentCancelled {
                listing_id,
                investor: signer,
                amount_returned: share_details.share_amount,
                new_shares_remaining: property_details.listed_share_amount,
                refunds,
            });
            Ok(())
        }

        /// Created an offer for a share listing.
        ///
        /// The origin must be Signed by a compliant RealEstateInvestor and have sufficient funds.
        ///
        /// Parameters:
        /// - `listing_id`: The listing that the investor wants to buy from.
        /// - `offer_price`: The offer price for shares that are offered.
        /// - `amount`: The amount of shares that the investor wants to buy.
        /// - `payment_asset`: Asset in which the investor wants to pay.
        ///
        /// Emits `OfferCreated` event when successful.
        #[pallet::call_index(8)]
        #[pallet::weight(<T as pallet::Config>::WeightInfo::make_offer())]
        pub fn make_offer(
            origin: OriginFor<T>,
            listing_id: ListingId,
            offer_price: <T as pallet::Config>::Balance,
            amount: u32,
            payment_asset: u32,
        ) -> DispatchResult {
            let signer = <T as pallet::Config>::CompliantOrigin::ensure_origin(
                origin,
                &Role::RealEstateInvestor,
            )?;

            // Validate input parameters
            ensure!(amount > 0, Error::<T>::AmountCannotBeZero);
            ensure!(!offer_price.is_zero(), Error::<T>::InvalidSharePrice);
            ensure!(
                T::AcceptedAssets::get().contains(&payment_asset),
                Error::<T>::PaymentAssetNotSupported
            );
            // Prevent duplicate offers from the same user for the same listing.
            ensure!(
                OngoingOffers::<T>::get(listing_id, &signer).is_none(),
                Error::<T>::OnlyOneOfferPerUser
            );

            // Retrieve and validate listing details
            let listing_details =
                ShareListings::<T>::get(listing_id).ok_or(Error::<T>::ShareNotForSale)?;
            ensure!(listing_details.amount >= amount, Error::<T>::NotEnoughSharesAvailable);
            let offer_nonce = NextOfferNonce::<T>::get();
            let price = offer_price
                .checked_mul(&((amount as u128).into()))
                .ok_or(Error::<T>::MultiplyError)?;

            // Hold funds from the investor’s account until the offer is accepted/rejected or cancelled.
            T::ForeignAssetsHolder::hold(
                payment_asset,
                &MarketplaceHoldReason::Marketplace,
                &signer,
                price,
            )?;

            // Generate unique nonce and store offer
            let offer_details = OfferDetails {
                share_price: offer_price,
                amount,
                payment_assets: payment_asset,
                nonce: offer_nonce,
            };
            let next_offer_nonce =
                offer_nonce.checked_add(1).ok_or(Error::<T>::ArithmeticOverflow)?;
            NextOfferNonce::<T>::put(next_offer_nonce);
            OngoingOffers::<T>::insert(listing_id, &signer, offer_details);

            Self::deposit_event(Event::<T>::OfferCreated {
                listing_id,
                offeror: signer,
                price: offer_price,
                amount,
                payment_asset,
            });
            Ok(())
        }

        /// Lets the investor handle an offer.
        ///
        /// The origin must be Signed by a RealEstateInvestor and have sufficient funds.
        ///
        /// Parameters:
        /// - `listing_id`: The listing that the investor wants to buy from.
        /// - `offeror`: AccountId of the person that the seller wants to handle the offer from.
        /// - `offer`: Enum for offer which is either Accept or Reject.
        ///
        /// Emits `OfferAccepted` event when offer gets accepted successfully.
        /// Emits `OfferRejected` event when offer gets rejected successfully.
        #[pallet::call_index(9)]
        #[pallet::weight(<T as pallet::Config>::WeightInfo::handle_offer())]
        pub fn handle_offer(
            origin: OriginFor<T>,
            listing_id: ListingId,
            offeror: AccountIdOf<T>,
            offer: Offer,
            offer_nonce: u64,
        ) -> DispatchResult {
            let signer = <T as pallet::Config>::CompliantOrigin::ensure_origin(
                origin,
                &Role::RealEstateInvestor,
            )?;

            // Retrieve and verify ownership
            let listing_details =
                ShareListings::<T>::get(listing_id).ok_or(Error::<T>::ShareNotForSale)?;
            ensure!(listing_details.seller == signer, Error::<T>::NoPermission);
            let offer_details = OngoingOffers::<T>::take(listing_id, offeror.clone())
                .ok_or(Error::<T>::OfferNotFound)?;
            // Validate offer nonce to prevent front-running attacks.
            ensure!(offer_details.nonce == offer_nonce, Error::<T>::InvalidOfferNonce);
            ensure!(
                listing_details.amount >= offer_details.amount,
                Error::<T>::NotEnoughSharesAvailable
            );
            let price = offer_details.get_total_amount()?;
            // Release the held funds from the investor’s account.
            T::ForeignAssetsHolder::release(
                offer_details.payment_assets,
                &MarketplaceHoldReason::Marketplace,
                &offeror,
                price,
                Precision::Exact,
            )?;
            match offer {
                Offer::Accept => {
                    // Restrict ownership to prevent exceeding limits
                    Self::restrict_ownership(
                        listing_details.asset_id,
                        &offeror,
                        offer_details.amount,
                    )?;
                    // Process the share purchase.
                    Self::buying_shares_process(
                        listing_id,
                        &offeror,
                        &offeror,
                        listing_details,
                        price,
                        offer_details.amount,
                        offer_details.payment_assets,
                    )?;
                    Self::deposit_event(Event::<T>::OfferAccepted {
                        listing_id,
                        offeror,
                        amount: offer_details.amount,
                        price,
                    });
                }
                Offer::Reject => {
                    Self::deposit_event(Event::<T>::OfferRejected {
                        listing_id,
                        offeror,
                        amount: offer_details.amount,
                        price,
                    });
                }
            }
            Ok(())
        }

        /// Lets the investor cancel an offer.
        ///
        /// The origin must be Signed by a RealEstateInvestor and have sufficient funds.
        ///
        /// Parameters:
        /// - `listing_id`: The listing that the investor wants to buy from.
        ///
        /// Emits `OfferCancelled` event when successful.
        #[pallet::call_index(10)]
        #[pallet::weight(<T as pallet::Config>::WeightInfo::cancel_offer())]
        pub fn cancel_offer(origin: OriginFor<T>, listing_id: ListingId) -> DispatchResult {
            let signer = <T as pallet::Config>::PermissionOrigin::ensure_origin(
                origin,
                &Role::RealEstateInvestor,
            )?;
            // Retrieve and remove the offer details.
            let offer_details =
                OngoingOffers::<T>::take(listing_id, &signer).ok_or(Error::<T>::OfferNotFound)?;
            let price = offer_details.get_total_amount()?;
            // Release the held funds back to the investor since the offer is being cancelled.
            T::ForeignAssetsHolder::release(
                offer_details.payment_assets,
                &MarketplaceHoldReason::Marketplace,
                &signer,
                price,
                Precision::Exact,
            )?;
            Self::deposit_event(Event::<T>::OfferCancelled { listing_id, account_id: signer });
            Ok(())
        }

        /// Lets the investor withdraw his funds after a property deal was unsuccessful.
        ///
        /// The origin must be Signed by a RealEstateInvestor and have sufficient funds.
        ///
        /// Parameters:
        /// - `listing_id`: The listing that the investor wants to withdraw from.
        ///
        /// Emits `RejectedFundsWithdrawn` event when successful.
        #[pallet::call_index(11)]
        #[pallet::weight(<T as pallet::Config>::WeightInfo::withdraw_rejected())]
        pub fn withdraw_rejected(origin: OriginFor<T>, listing_id: ListingId) -> DispatchResult {
            let signer = <T as pallet::Config>::PermissionOrigin::ensure_origin(
                origin,
                &Role::RealEstateInvestor,
            )?;
            // Retrieve refund info and listing details
            let mut refund_infos =
                RefundShare::<T>::get(listing_id).ok_or(Error::<T>::SharesNotRefunded)?;
            let property_details =
                OngoingObjectListing::<T>::get(listing_id).ok_or(Error::<T>::ListingNotFound)?;
            let property_account = Self::property_account_id(property_details.asset_id);
            // Get investor's current share balance for this listing.
            let share_amount = <T as pallet::Config>::PropertyShares::get_share_balance(
                property_details.asset_id,
                &signer,
            );
            ensure!(!share_amount.is_zero(), Error::<T>::NoSharesOwned);

            // Update refund tracker
            refund_infos.refund_amount = refund_infos
                .refund_amount
                .checked_sub(share_amount)
                .ok_or(Error::<T>::InsufficientRefundableShares)?;

            // Refund payments in all accepted assets (USDC, USDT, etc.)
            for &asset in T::AcceptedAssets::get().iter() {
                if let Some(investor_funds) = property_details.investor_funds.get(&signer).cloned()
                {
                    if let Some(paid_funds) = investor_funds.paid_funds.get(&asset).copied() {
                        // Transfer funds to owner account
                        Self::transfer_funds(&property_account, &signer, paid_funds, asset)?;
                    }
                }
            }
            // Transfer property shares back from investor to property account (burn preparation).
            <T as pallet::Config>::LocalCurrency::transfer(
                property_details.asset_id,
                &signer,
                &property_account,
                share_amount.into(),
                Preservation::Expendable,
            )?;
            // If all shares have been refunded, burn the property shares/nft and clean up storage.
            if refund_infos.refund_amount == 0 {
                T::PropertyShares::burn_property_shares(property_details.asset_id)?;
                Self::refund_investors_with_fees(
                    &property_details,
                    refund_infos.property_lawyer_details,
                )?;
                // Release the listing deposit back to the real estate developer.
                let (depositor, deposit_amount) =
                    ListingDeposits::<T>::take(listing_id).ok_or(Error::<T>::ListingNotFound)?;
                <T as pallet::Config>::NativeCurrency::release(
                    &HoldReason::ListingDepositReserve.into(),
                    &depositor,
                    deposit_amount,
                    Precision::Exact,
                )?;
                // If property account still holds native currency, transfer it to the developer.
                let native_balance =
                    <T as pallet::Config>::NativeCurrency::balance(&property_account);
                if !native_balance.is_zero() {
                    <T as pallet::Config>::NativeCurrency::transfer(
                        &property_account,
                        &property_details.real_estate_developer,
                        native_balance,
                        Preservation::Expendable,
                    )?;
                }
                OngoingObjectListing::<T>::remove(listing_id);
                RefundShare::<T>::remove(listing_id);
            } else {
                RefundShare::<T>::insert(listing_id, refund_infos);
            }
            // Remove ownership record
            T::PropertyShares::remove_property_share_ownership(property_details.asset_id, &signer)?;
            Self::deposit_event(Event::<T>::RejectedFundsWithdrawn { signer, listing_id });
            Ok(())
        }

        /// Lets the investor withdraw his funds after a property deal expired.
        ///
        /// The origin must be Signed by a RealEstateInvestor and have sufficient funds.
        ///
        /// Parameters:
        /// - `listing_id`: The listing that the investor wants to withdraw from.
        ///
        /// Emits `ExpiredFundsWithdrawn` event when successful.
        #[pallet::call_index(12)]
        #[pallet::weight(<T as pallet::Config>::WeightInfo::withdraw_legal_process_expired())]
        pub fn withdraw_legal_process_expired(
            origin: OriginFor<T>,
            listing_id: ListingId,
        ) -> DispatchResult {
            let signer = <T as pallet::Config>::PermissionOrigin::ensure_origin(
                origin,
                &Role::RealEstateInvestor,
            )?;
            let property_details =
                OngoingObjectListing::<T>::get(listing_id).ok_or(Error::<T>::ListingNotFound)?;
            let property_account = Self::property_account_id(property_details.asset_id);
            // Get investor's share balance for this listing.
            let share_amount = <T as pallet::Config>::PropertyShares::get_share_balance(
                property_details.asset_id,
                &signer,
            );
            ensure!(!share_amount.is_zero(), Error::<T>::NoSharesOwned);

            // Determine refundable amount, initializing if this is the first withdrawal.
            let mut refund_infos = match RefundLegalExpired::<T>::get(listing_id) {
                Some(refund_infos) => refund_infos,
                None => {
                    let property_lawyer_details =
                        PropertyLawyer::<T>::get(listing_id).ok_or(Error::<T>::SharesNotRefunded)?;
                    let current_block_number =
                        <T as pallet::Config>::BlockNumberProvider::current_block_number();
                    ensure!(
                        property_lawyer_details.legal_process_expiry < current_block_number,
                        Error::<T>::LegalProcessOngoing
                    );

                    // Decrement active cases for assigned lawyers.
                    if let Some(real_estate_developer_lawyer_id) =
                        property_lawyer_details.real_estate_developer_lawyer
                    {
                        <T as pallet::Config>::RegionProvider::decrement_active_cases(
                            &real_estate_developer_lawyer_id,
                        )?;
                    }
                    if let Some(spv_lawyer_id) = property_lawyer_details.spv_lawyer {
                        <T as pallet::Config>::RegionProvider::decrement_active_cases(
                            &spv_lawyer_id,
                        )?;
                    }

                    PropertyLawyer::<T>::remove(listing_id);
                    RefundLegalExpired::<T>::insert(listing_id, property_details.share_amount);
                    property_details.share_amount
                }
            };

            refund_infos = refund_infos
                .checked_sub(share_amount)
                .ok_or(Error::<T>::InsufficientRefundableShares)?;

            // Refund payments in all accepted assets (USDC, USDT, etc.)
            for &asset in T::AcceptedAssets::get().iter() {
                if let Some(investor_funds) = property_details.investor_funds.get(&signer).cloned()
                {
                    if let Some(paid_funds) = investor_funds.paid_funds.get(&asset).copied() {
                        if let Some(paid_fee) = investor_funds.paid_fee.get(&asset).copied() {
                            // Refund both funds + paid fees.
                            let transfer_amount = paid_funds
                                .checked_add(&paid_fee)
                                .ok_or(Error::<T>::ArithmeticOverflow)?;
                            Self::transfer_funds(
                                &property_account,
                                &signer,
                                transfer_amount,
                                asset,
                            )?;
                        } else {
                            // Refund only paid funds (no fees).
                            Self::transfer_funds(&property_account, &signer, paid_funds, asset)?;
                        }
                    }
                }
            }
            // Transfer property shares back from investor to property account (burn preparation).
            <T as pallet::Config>::LocalCurrency::transfer(
                property_details.asset_id,
                &signer,
                &property_account,
                share_amount.into(),
                Preservation::Expendable,
            )?;
            // If all shares have been refunded, burn the property shares/nft and clean up storage.
            if refund_infos == 0 {
                T::PropertyShares::burn_property_shares(property_details.asset_id)?;
                T::PropertyShares::clear_share_owners(property_details.asset_id)?;
                // Refund the original listing deposit back to the real estate developer.
                let (depositor, deposit_amount) =
                    ListingDeposits::<T>::take(listing_id).ok_or(Error::<T>::ListingNotFound)?;
                <T as pallet::Config>::NativeCurrency::release(
                    &HoldReason::ListingDepositReserve.into(),
                    &depositor,
                    deposit_amount,
                    Precision::Exact,
                )?;
                // If property account still holds native currency, transfer it to the developer.
                let native_balance =
                    <T as pallet::Config>::NativeCurrency::balance(&property_account);
                if !native_balance.is_zero() {
                    <T as pallet::Config>::NativeCurrency::transfer(
                        &property_account,
                        &property_details.real_estate_developer,
                        native_balance,
                        Preservation::Expendable,
                    )?;
                }
                OngoingObjectListing::<T>::remove(listing_id);
                RefundLegalExpired::<T>::remove(listing_id);
            } else {
                RefundLegalExpired::<T>::insert(listing_id, refund_infos);
            }
            // Remove ownership record
            T::PropertyShares::remove_property_share_ownership(property_details.asset_id, &signer)?;
            Self::deposit_event(Event::<T>::ExpiredFundsWithdrawn { signer, listing_id });
            Ok(())
        }

        /// Lets the investor unfreeze his funds after a property listing expired.
        ///
        /// The origin must be Signed by a RealEstateInvestor and have sufficient funds.
        ///
        /// Parameters:
        /// - `listing_id`: The listing that the investor wants to buy from.
        ///
        /// Emits `ExpiredFundsWithdrawn` event when successful.
        #[pallet::call_index(13)]
        #[pallet::weight(<T as pallet::Config>::WeightInfo::withdraw_expired())]
        pub fn withdraw_expired(origin: OriginFor<T>, listing_id: ListingId) -> DispatchResult {
            let signer = <T as pallet::Config>::PermissionOrigin::ensure_origin(
                origin,
                &Role::RealEstateInvestor,
            )?;
            let mut property_details =
                OngoingObjectListing::<T>::get(listing_id).ok_or(Error::<T>::ListingNotFound)?;
            // Ensure the listing has expired.
            ensure!(
                property_details.listing_expiry
                    < <T as pallet::Config>::BlockNumberProvider::current_block_number(),
                Error::<T>::ListingNotExpired
            );

            // Ensure that shares were not fully sold already (if they are, listing is no longer refundable).
            ensure!(
                !property_details.listed_share_amount.is_zero(),
                Error::<T>::PropertyAlreadySold
            );

            // Retrieve investor's purchase record
            let share_details =
                ShareOwner::<T>::take(&signer, listing_id).ok_or(Error::<T>::ShareOwnerNotFound)?;
            ensure!(!share_details.share_amount.is_zero(), Error::<T>::NoSharesBought,);

            // Unfreeze investor's funds for this listing (refund for paid assets like USDT/USDC).
            Self::unfreeze_shares(&share_details, &signer)?;

            // Add the withdrawn share amount back to the listing.
            property_details.listed_share_amount = property_details
                .listed_share_amount
                .checked_add(share_details.share_amount)
                .ok_or(Error::<T>::ArithmeticOverflow)?;

            // Check if all shares are returned
            if property_details.listed_share_amount >= property_details.share_amount {
                // Burn all property shares since listing is over.
                T::PropertyShares::burn_property_shares(property_details.asset_id)?;
                // Refund original deposit to the listing creator.
                let (depositor, deposit_amount) =
                    ListingDeposits::<T>::take(listing_id).ok_or(Error::<T>::ListingNotFound)?;
                <T as pallet::Config>::NativeCurrency::release(
                    &HoldReason::ListingDepositReserve.into(),
                    &depositor,
                    deposit_amount,
                    Precision::Exact,
                )?;
                // Transfer any remaining native currency from property account back to developer.
                let property_account = Self::property_account_id(property_details.asset_id);
                let native_balance =
                    <T as pallet::Config>::NativeCurrency::balance(&property_account);
                if !native_balance.is_zero() {
                    <T as pallet::Config>::NativeCurrency::transfer(
                        &property_account,
                        &property_details.real_estate_developer,
                        native_balance,
                        Preservation::Expendable,
                    )?;
                }
                OngoingObjectListing::<T>::remove(listing_id);
            } else {
                OngoingObjectListing::<T>::insert(listing_id, &property_details);
            }
            Self::deposit_event(Event::<T>::ExpiredFundsWithdrawn { signer, listing_id });
            Ok(())
        }

        /// Lets the real estate developer withdraw his deposit in case no shares have been sold.
        ///
        /// The origin must be Signed by a RealEstateInvestor and have sufficient funds.
        ///
        /// Parameters:
        /// - `listing_id`: The listing that the caller wants to withdraw the deposit from.
        ///
        /// Emits `DeveloperDepositReturned` event when successful.
        #[pallet::call_index(14)]
        #[pallet::weight(<T as pallet::Config>::WeightInfo::withdraw_deposit_unsold())]
        pub fn withdraw_deposit_unsold(
            origin: OriginFor<T>,
            listing_id: ListingId,
        ) -> DispatchResult {
            let signer = <T as pallet::Config>::PermissionOrigin::ensure_origin(
                origin,
                &Role::RealEstateDeveloper,
            )?;
            let property_details =
                OngoingObjectListing::<T>::get(listing_id).ok_or(Error::<T>::ListingNotFound)?;
            // Ensure that the caller is the real estate developer.
            ensure!(property_details.real_estate_developer == signer, Error::<T>::NoPermission);
            // Ensure the listing has expired.
            ensure!(
                property_details.listing_expiry
                    < <T as pallet::Config>::BlockNumberProvider::current_block_number(),
                Error::<T>::ListingNotExpired
            );
            ensure!(
                !property_details.listed_share_amount.is_zero(),
                Error::<T>::PropertyAlreadySold
            );
            // Ensure that ALL shares have been returned to the pool (no partial sales).
            ensure!(
                property_details.listed_share_amount >= property_details.share_amount,
                Error::<T>::SharesNotReturned
            );

            // Burn property shares since the entire listing was unsold and is now closed.
            T::PropertyShares::burn_property_shares(property_details.asset_id)?;
            // Release developer's deposit that was initially locked for the listing.
            let (depositor, deposit_amount) =
                ListingDeposits::<T>::take(listing_id).ok_or(Error::<T>::ListingNotFound)?;
            <T as pallet::Config>::NativeCurrency::release(
                &HoldReason::ListingDepositReserve.into(),
                &depositor,
                deposit_amount,
                Precision::Exact,
            )?;
            // If the property account still has native currency, transfer it back to the developer.
            let property_account = Self::property_account_id(property_details.asset_id);
            let native_balance = <T as pallet::Config>::NativeCurrency::balance(&property_account);
            if !native_balance.is_zero() {
                <T as pallet::Config>::NativeCurrency::transfer(
                    &property_account,
                    &property_details.real_estate_developer,
                    native_balance,
                    Preservation::Expendable,
                )?;
            }
            OngoingObjectListing::<T>::remove(listing_id);
            Self::deposit_event(Event::<T>::DeveloperDepositReturned {
                listing_id,
                developer: signer,
                amount: deposit_amount,
            });
            Ok(())
        }

        /// Lets the real estate investor withdraw his funds in case the sale is cancelled.
        ///
        /// The origin must be Signed by a RealEstateInvestor and have sufficient funds.
        ///
        /// Parameters:
        /// - `listing_id`: The listing that the caller wants to withdraw the funds from.
        ///
        /// Emits `RejectedFundsWithdrawn` event when successful.
        #[pallet::call_index(15)]
        #[pallet::weight(<T as pallet::Config>::WeightInfo::withdraw_claiming_expired())]
        pub fn withdraw_claiming_expired(
            origin: OriginFor<T>,
            listing_id: ListingId,
        ) -> DispatchResult {
            let signer = <T as pallet::Config>::PermissionOrigin::ensure_origin(
                origin,
                &Role::RealEstateInvestor,
            )?;
            let mut refund_amount =
                RefundClaimedShare::<T>::get(listing_id).ok_or(Error::<T>::SharesNotRefunded)?;
            let property_details =
                OngoingObjectListing::<T>::get(listing_id).ok_or(Error::<T>::ListingNotFound)?;
            let property_account = Self::property_account_id(property_details.asset_id);
            // Get investor's share balance for this listing.
            let share_amount = <T as pallet::Config>::PropertyShares::get_share_balance(
                property_details.asset_id,
                &signer,
            );
            ensure!(!share_amount.is_zero(), Error::<T>::NoSharesOwned);
            // Update refund tracker.
            refund_amount = refund_amount
                .checked_sub(share_amount)
                .ok_or(Error::<T>::InsufficientRefundableShares)?;
            // Refund payments in all accepted assets (USDC, USDT, etc.) including any paid fees.
            if let Some(investor_funds) = property_details.investor_funds.get(&signer) {
                for (asset, paid_funds) in investor_funds.paid_funds.iter() {
                    let paid_fees = investor_funds.paid_fee.get(asset).copied().unwrap_or_default();
                    let transfer_amount =
                        paid_funds.checked_add(&paid_fees).ok_or(Error::<T>::ArithmeticOverflow)?;
                    // Transfer funds back from property account to investor.
                    Self::transfer_funds(&property_account, &signer, transfer_amount, *asset)?;
                }
            }
            // Transfer property shares back from investor to property account (burn preparation).
            <T as pallet::Config>::LocalCurrency::transfer(
                property_details.asset_id,
                &signer,
                &property_account,
                share_amount.into(),
                Preservation::Expendable,
            )?;
            // If all shares have been refunded, burn the property shares/nft and clean up storage.
            if refund_amount == 0 {
                T::PropertyShares::burn_property_shares(property_details.asset_id)?;
                T::PropertyShares::clear_share_owners(property_details.asset_id)?;
                // Refund the original listing deposit back to the real estate developer.
                let (depositor, deposit_amount) =
                    ListingDeposits::<T>::take(listing_id).ok_or(Error::<T>::ListingNotFound)?;
                <T as pallet::Config>::NativeCurrency::release(
                    &HoldReason::ListingDepositReserve.into(),
                    &depositor,
                    deposit_amount,
                    Precision::Exact,
                )?;
                // If property account still holds native currency, transfer it to the developer.
                let native_balance =
                    <T as pallet::Config>::NativeCurrency::balance(&property_account);
                if !native_balance.is_zero() {
                    <T as pallet::Config>::NativeCurrency::transfer(
                        &property_account,
                        &property_details.real_estate_developer,
                        native_balance,
                        Preservation::Expendable,
                    )?;
                }
                OngoingObjectListing::<T>::remove(listing_id);
                RefundClaimedShare::<T>::remove(listing_id);
            } else {
                RefundClaimedShare::<T>::insert(listing_id, refund_amount);
            }
            // Remove ownership record
            T::PropertyShares::remove_property_share_ownership(property_details.asset_id, &signer)?;
            Self::deposit_event(Event::<T>::RejectedFundsWithdrawn { signer, listing_id });
            Ok(())
        }

        /// Lets the real estate investor unfreeze his funds in case the claiming window expired.
        ///
        /// The origin must be Signed by a RealEstateInvestor and have sufficient funds.
        ///
        /// Parameters:
        /// - `listing_id`: The listing that the caller wants to unfreeze the funds from.
        ///
        /// Emits `UnclaimedSharesWithdrawn` event when successful.
        #[pallet::call_index(16)]
        #[pallet::weight(<T as pallet::Config>::WeightInfo::withdraw_unclaimed())]
        pub fn withdraw_unclaimed(origin: OriginFor<T>, listing_id: ListingId) -> DispatchResult {
            let signer = <T as pallet::Config>::PermissionOrigin::ensure_origin(
                origin,
                &Role::RealEstateInvestor,
            )?;
            // Retrieve investor's purchase record.
            let share_details: ShareOwnerDetails<T> =
                ShareOwner::<T>::take(&signer, listing_id).ok_or(Error::<T>::ShareOwnerNotFound)?;
            ensure!(!share_details.share_amount.is_zero(), Error::<T>::NoSharesBought);
            // If property listing still exists, ensure it has been relisted at least once
            // since this investor's original purchase attempt (otherwise withdrawal is not allowed).
            if let Some(property_details) = OngoingObjectListing::<T>::get(listing_id) {
                ensure!(
                    property_details.relist_count > share_details.relist_count,
                    Error::<T>::NoPermission
                );
            }

            // Unfreeze investor's funds for this listing (refund for paid assets like USDT/USDC).
            let refunds = Self::unfreeze_shares_with_refunds(&share_details, &signer)?;

            Self::deposit_event(Event::<T>::UnclaimedSharesWithdrawn {
                listing_id,
                investor: signer,
                refunds,
            });
            Ok(())
        }

        /// Allows a real estate developer to update the price of a listed property.
        ///
        /// The origin must be Signed by a RealEstateDeveloper and have sufficient funds.
        ///
        /// Parameters:
        /// - `listing_id`: The listing that the seller wants to update.
        /// - `new_price`: The new price of the object.
        ///
        /// Emits `ObjectUpdated` event when successful.
        #[pallet::call_index(17)]
        #[pallet::weight(<T as pallet::Config>::WeightInfo::upgrade_object())]
        pub fn upgrade_object(
            origin: OriginFor<T>,
            listing_id: ListingId,
            new_price: <T as pallet::Config>::Balance,
        ) -> DispatchResult {
            let signer = <T as pallet::Config>::PermissionOrigin::ensure_origin(
                origin,
                &Role::RealEstateDeveloper,
            )?;
            // Validate new price.
            ensure!(!new_price.is_zero(), Error::<T>::InvalidSharePrice);
            // Ensure that the property is not already sold (in which case price update is not allowed).
            ensure!(
                PropertyLawyer::<T>::get(listing_id).is_none(),
                Error::<T>::PropertyAlreadySold
            );
            // Update the price of the ongoing listing after validating permissions and expiry.
            OngoingObjectListing::<T>::try_mutate(listing_id, |maybe_property_details| {
                let property_details =
                    maybe_property_details.as_mut().ok_or(Error::<T>::ShareNotForSale)?;
                ensure!(
                    property_details.listing_expiry
                        > <T as pallet::Config>::BlockNumberProvider::current_block_number(),
                    Error::<T>::ListingExpired
                );
                ensure!(property_details.real_estate_developer == signer, Error::<T>::NoPermission);
                // Ensure shares have not all been sold (otherwise price change is irrelevant).
                ensure!(
                    !property_details.listed_share_amount.is_zero(),
                    Error::<T>::PropertyAlreadySold
                );
                // Update the share price.
                property_details.share_price = new_price;
                Ok::<(), DispatchError>(())
            })?;
            Self::deposit_event(Event::<T>::ObjectUpdated { listing_index: listing_id, new_price });
            Ok(())
        }

        /// Allows a real estate investor to delist (remove) relisted shares from the marketplace.
        ///
        /// The origin must be Signed by a RealEstateInvestor and have sufficient funds.
        ///
        /// Parameters:
        /// - `listing_id`: The listing that the seller wants to delist.
        ///
        /// Emits `ListingDelisted` event when successful.
        #[pallet::call_index(18)]
        #[pallet::weight(<T as pallet::Config>::WeightInfo::delist_shares())]
        pub fn delist_shares(origin: OriginFor<T>, listing_id: ListingId) -> DispatchResult {
            let signer = <T as pallet::Config>::PermissionOrigin::ensure_origin(
                origin,
                &Role::RealEstateInvestor,
            )?;
            // Retrieve and remove the listing details.
            let listing_details =
                ShareListings::<T>::take(listing_id).ok_or(Error::<T>::ShareNotForSale)?;
            // Ensure that the caller is the original seller.
            ensure!(listing_details.seller == signer, Error::<T>::NoPermission);
            let share_amount = listing_details.amount.into();
            // Get the property account (escrow account holding the shares).
            let property_account = Self::property_account_id(listing_details.asset_id);
            // Transfer the shares back from the property account to the investor.
            <T as pallet::Config>::LocalCurrency::transfer(
                listing_details.asset_id,
                &property_account,
                &signer,
                share_amount,
                Preservation::Expendable,
            )?;
            Self::deposit_event(Event::<T>::ListingDelisted { listing_index: listing_id });
            Ok(())
        }

        /// Lets a lawyer claim a property to handle the legal work.
        ///
        /// The origin must be Signed by a Lawyer and have sufficient funds.
        ///
        /// Parameters:
        /// - `listing_id`: The listing from the property.
        /// - `legal_side`: The side that the lawyer wants to represent.
        /// - `costs`: The costs thats the lawyer demands for his work.
        ///
        /// Emits `DeveloperLawyerProposed` event or `SpvLawyerProposed` event when successful.
        #[pallet::call_index(19)]
        #[pallet::weight(<T as pallet::Config>::WeightInfo::lawyer_claim_property())]
        pub fn lawyer_claim_property(
            origin: OriginFor<T>,
            listing_id: ListingId,
            legal_side: LegalProperty,
            costs: <T as pallet::Config>::Balance,
        ) -> DispatchResult {
            let signer =
                <T as pallet::Config>::CompliantOrigin::ensure_origin(origin, &Role::Lawyer)?;
            // Retrieve lawyer region info.
            let lawyer_region = <T as pallet::Config>::RegionProvider::get_lawyer_info(&signer)
                .ok_or(Error::<T>::NoPermission)?;
            let property_details =
                OngoingObjectListing::<T>::get(listing_id).ok_or(Error::<T>::ListingNotFound)?;
            let asset_details =
                T::PropertyShares::get_property_asset_info(property_details.asset_id)
                    .ok_or(Error::<T>::NoObjectFound)?;
            // Ensure lawyer operates in the same region as the property.
            ensure!(lawyer_region.region == asset_details.region, Error::<T>::WrongRegion);
            let property_lawyer_details =
                PropertyLawyer::<T>::get(listing_id).ok_or(Error::<T>::InvalidIndex)?;
            // Ensure legal process has not expired (lawyers can only claim during active process).
            let current_block_number =
                <T as pallet::Config>::BlockNumberProvider::current_block_number();
            ensure!(
                property_lawyer_details.legal_process_expiry >= current_block_number,
                Error::<T>::LegalProcessFailed
            );

            // Ensure total fees collected are sufficient to cover the lawyer's demanded costs.
            let mut collected_fees: <T as pallet::Config>::Balance = Zero::zero();
            for asset_id in T::AcceptedAssets::get() {
                let fee = property_details
                    .collected_fees
                    .get(&asset_id)
                    .ok_or(Error::<T>::AssetNotSupported)?;
                collected_fees =
                    collected_fees.checked_add(fee).ok_or(Error::<T>::ArithmeticOverflow)?;
            }
            ensure!(collected_fees >= costs, Error::<T>::CostsTooHigh);
            // Handle lawyer proposal based on the side they want to represent.
            match legal_side {
                LegalProperty::RealEstateDeveloperSide => {
                    // Ensure that no other proposal is ongoing and that the job is still available.
                    ensure!(
                        !ProposedLawyers::<T>::contains_key(listing_id),
                        Error::<T>::LawyerProposalOngoing
                    );
                    ensure!(
                        property_lawyer_details.real_estate_developer_lawyer.is_none(),
                        Error::<T>::LawyerJobTaken
                    );
                    // Ensure the same lawyer is not already proposed/assigned for spv side.
                    ensure!(
                        property_lawyer_details.spv_lawyer.as_ref() != Some(&signer),
                        Error::<T>::NoPermission
                    );
                    // Prevent conflict with pending SPV proposal
                    if let Some(proposal_id) = ListingSpvProposal::<T>::get(listing_id) {
                        if let Some(lawyer_proposal) = SpvLawyerProposal::<T>::get(proposal_id) {
                            ensure!(lawyer_proposal.lawyer != signer, Error::<T>::NoPermission);
                        }
                    }
                    ProposedLawyers::<T>::insert(
                        listing_id,
                        ProposedDeveloperLawyer { lawyer: signer.clone(), costs },
                    );
                    Self::deposit_event(Event::<T>::DeveloperLawyerProposed {
                        listing_id,
                        lawyer: signer,
                        proposed_cost: costs,
                    });
                }
                LegalProperty::SpvSide => {
                    // Ensure SPV has been created for the property.
                    T::PropertyShares::ensure_spv_created(property_details.asset_id)?;
                    // Ensure that no other proposal is ongoing and that the job is still available.
                    ensure!(
                        !ListingSpvProposal::<T>::contains_key(listing_id),
                        Error::<T>::LawyerProposalOngoing
                    );
                    ensure!(
                        property_lawyer_details.spv_lawyer.is_none(),
                        Error::<T>::LawyerJobTaken
                    );
                    // Ensure the same lawyer is not already proposed/assigned for developer side.
                    ensure!(
                        property_lawyer_details.real_estate_developer_lawyer.as_ref()
                            != Some(&signer),
                        Error::<T>::NoPermission
                    );
                    // Prevent conflict with developer proposal
                    if let Some(lawyer_proposal) = ProposedLawyers::<T>::get(listing_id) {
                        ensure!(lawyer_proposal.lawyer != signer, Error::<T>::NoPermission);
                    }
                    let proposal_id = ProposalCounter::<T>::get();
                    let expiry_block =
                        current_block_number.saturating_add(T::LawyerVotingTime::get());
                    // Store proposal details in storage.
                    ListingSpvProposal::<T>::insert(listing_id, proposal_id);
                    SpvLawyerProposal::<T>::insert(
                        proposal_id,
                        ProposedSpvLawyer {
                            lawyer: signer.clone(),
                            asset_id: property_details.asset_id,
                            costs,
                            expiry_block,
                        },
                    );
                    OngoingLawyerVoting::<T>::insert(
                        proposal_id,
                        VoteStats {
                            yes_voting_power: 0,
                            no_voting_power: 0,
                            abstain_voting_power: 0,
                        },
                    );
                    let next_proposal_id =
                        proposal_id.checked_add(1).ok_or(Error::<T>::ArithmeticOverflow)?;
                    ProposalCounter::<T>::put(next_proposal_id);
                    Self::deposit_event(Event::<T>::SpvLawyerProposed {
                        listing_id,
                        lawyer: signer,
                        proposed_cost: costs,
                        expiry_block,
                    });
                }
            }
            Ok(())
        }

        /// Allows a share holder (real estate investor) to vote on the lawyer that will represent the SPV.
        ///
        /// The origin must be Signed by a RealEstateInvestor and have sufficient funds.
        ///
        /// Parameters:
        /// - `listing_id`: The listing from the property.
        /// - `vote`: Must be either a Yes vote or a No vote.
        /// - `amount`: The amount of property shares that the investor is using for voting.
        ///
        /// Emits `VotedOnLawyer` event when successful.
        #[pallet::call_index(20)]
        #[pallet::weight(<T as pallet::Config>::WeightInfo::vote_on_spv_lawyer())]
        pub fn vote_on_spv_lawyer(
            origin: OriginFor<T>,
            listing_id: ListingId,
            vote: Vote,
            amount: u32,
        ) -> DispatchResult {
            let signer = <T as pallet::Config>::PermissionOrigin::ensure_origin(
                origin,
                &Role::RealEstateInvestor,
            )?;
            // Validate vote amount.
            ensure!(amount > 0, Error::<T>::ZeroVoteAmount);
            // Load proposal and ensure it's active.
            let proposal_id =
                ListingSpvProposal::<T>::get(listing_id).ok_or(Error::<T>::NoLawyerProposed)?;
            let proposal_details =
                SpvLawyerProposal::<T>::get(proposal_id).ok_or(Error::<T>::NoLawyerProposed)?;
            let current_block_number =
                <T as pallet::Config>::BlockNumberProvider::current_block_number();
            ensure!(
                proposal_details.expiry_block > current_block_number,
                Error::<T>::VotingExpired
            );
            // Check voter has enough share balance to vote with the specified amount.
            let voting_power =
                T::PropertyShares::get_share_balance(proposal_details.asset_id, &signer);
            ensure!(voting_power >= amount, Error::<T>::NotEnoughShares);

            let mut new_yes_power = 0u32;
            let mut new_no_power = 0u32;
            let mut new_abstain_power = 0u32;

            // Update the ongoing voting stats for this proposal.
            OngoingLawyerVoting::<T>::try_mutate(proposal_id, |maybe_current_vote| {
                let current_vote =
                    maybe_current_vote.as_mut().ok_or(Error::<T>::NoLawyerProposed)?;

                // Update user's vote record and adjust voting power accordingly.
                UserLawyerVote::<T>::try_mutate(proposal_id, &signer, |maybe_vote_record| {
                    // If user had a previous vote, unfreeze their shares and update tallies.
                    if let Some(previous_vote) = maybe_vote_record.take() {
                        T::AssetsFreezer::decrease_frozen(
                            proposal_details.asset_id,
                            &MarketplaceFreezeReason::SpvLawyerVoting,
                            &signer,
                            previous_vote.power.into(),
                        )?;

                        // Adjust global voting power tallies by subtracting old vote.
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
                        proposal_details.asset_id,
                        &MarketplaceFreezeReason::SpvLawyerVoting,
                        &signer,
                        amount.into(),
                    )?;

                    // Update the voting tallies with the new vote.
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

                    *maybe_vote_record = Some(VoteRecord {
                        vote: vote.clone(),
                        asset_id: proposal_details.asset_id,
                        power: amount,
                    });

                    new_yes_power = current_vote.yes_voting_power;
                    new_no_power = current_vote.no_voting_power;
                    new_abstain_power = current_vote.abstain_voting_power;

                    Ok::<(), DispatchError>(())
                })?;

                Ok::<(), DispatchError>(())
            })?;
            Self::deposit_event(Event::VotedOnLawyer {
                listing_id,
                voter: signer,
                vote,
                voting_power,
                new_yes_power,
                new_no_power,
                new_abstain_power,
                proposal_id,
            });
            Ok(())
        }

        /// Allows the Real Estate Developer to approve or reject a proposed lawyer.
        ///
        /// The origin must be Signed by a RealEstateDeveloper and have sufficient funds.
        ///
        /// Parameters:
        /// - `listing_id`: The listing from the property.
        /// - `approve`: Approves or rejects the lawyer.
        ///
        /// Emits `RealEstateLawyerProposalFinalized` event when successful.
        #[pallet::call_index(21)]
        #[pallet::weight(<T as pallet::Config>::WeightInfo::approve_developer_lawyer())]
        pub fn approve_developer_lawyer(
            origin: OriginFor<T>,
            listing_id: ListingId,
            approve: bool,
        ) -> DispatchResult {
            let signer = <T as pallet::Config>::PermissionOrigin::ensure_origin(
                origin,
                &Role::RealEstateDeveloper,
            )?;
            let property_details =
                OngoingObjectListing::<T>::get(listing_id).ok_or(Error::<T>::ListingNotFound)?;
            // Ensure that the caller is the real estate developer.
            ensure!(signer == property_details.real_estate_developer, Error::<T>::NoPermission);

            // Get current lawyer details and active proposal for this listing.
            let mut property_lawyer_details =
                PropertyLawyer::<T>::get(listing_id).ok_or(Error::<T>::InvalidIndex)?;
            let proposal =
                ProposedLawyers::<T>::get(listing_id).ok_or(Error::<T>::NoLawyerProposed)?;
            let current_block_number =
                <T as pallet::Config>::BlockNumberProvider::current_block_number();
            let expired = current_block_number > property_lawyer_details.legal_process_expiry;

            // If approved and legal process not expired, assign the lawyer and allocate costs.
            if approve && !expired {
                property_lawyer_details.real_estate_developer_lawyer =
                    Some(proposal.lawyer.clone());

                // Allocate the lawyer's costs from the collected fees.
                let allocated_costs = Self::allocate_fees(
                    &mut property_lawyer_details.real_estate_developer_lawyer_costs,
                    &property_details.collected_fees,
                    proposal.costs,
                )?;
                Self::deposit_event(Event::<T>::LawyerCostsAllocated {
                    listing_id,
                    lawyer_account: proposal.lawyer.clone(),
                    lawyer_type: LegalProperty::RealEstateDeveloperSide,
                    costs: allocated_costs,
                });
                <T as pallet::Config>::RegionProvider::increment_active_cases(&proposal.lawyer)?;
                PropertyLawyer::<T>::insert(listing_id, property_lawyer_details);
            }
            // Remove the proposal from storage regardless of approval or rejection.
            ProposedLawyers::<T>::remove(listing_id);
            Self::deposit_event(Event::RealEstateLawyerProposalFinalized {
                listing_id,
                lawyer: proposal.lawyer,
                is_approved: approve,
            });
            Ok(())
        }

        /// Finalizes the spv lawyer voting.
        ///
        /// The origin must be signed and have sufficient funds.
        ///
        /// Parameters:
        /// - `listing_id`: The listing from the property.
        ///
        /// Emits `SpvLawyerVoteFinalized` event when successful.
        #[pallet::call_index(22)]
        #[pallet::weight(<T as pallet::Config>::WeightInfo::finalize_spv_lawyer())]
        pub fn finalize_spv_lawyer(origin: OriginFor<T>, listing_id: ListingId) -> DispatchResult {
            let _ = ensure_signed(origin)?;
            // Load proposal and ensure voting period has expired.
            let proposal_id =
                ListingSpvProposal::<T>::get(listing_id).ok_or(Error::<T>::NoLawyerProposed)?;
            let proposal =
                SpvLawyerProposal::<T>::get(proposal_id).ok_or(Error::<T>::NoLawyerProposed)?;
            let current_block_number =
                <T as pallet::Config>::BlockNumberProvider::current_block_number();
            ensure!(proposal.expiry_block <= current_block_number, Error::<T>::VotingStillOngoing);

            // Retrieve ongoing voting results and property details.
            let voting_result =
                OngoingLawyerVoting::<T>::get(proposal_id).ok_or(Error::<T>::NoLawyerProposed)?;
            let property_details =
                OngoingObjectListing::<T>::get(listing_id).ok_or(Error::<T>::ListingNotFound)?;
            let mut property_lawyer_details =
                PropertyLawyer::<T>::get(listing_id).ok_or(Error::<T>::InvalidIndex)?;
            // Fetch asset details to compute quorum.
            let asset_details = <T as pallet::Config>::PropertyShares::get_property_asset_info(
                property_details.asset_id,
            )
            .ok_or(Error::<T>::NoObjectFound)?;
            let total_votes = voting_result
                .yes_voting_power
                .saturating_add(voting_result.no_voting_power)
                .saturating_add(voting_result.abstain_voting_power);
            let total_supply = asset_details.share_amount;

            // There must be a nonzero supply for voting to be meaningful.
            ensure!(total_supply > Zero::zero(), Error::<T>::NoObjectFound);

            // Determine if quorum is met and if the lawyer is approved.
            let quorum_percent: u32 = T::MinVotingQuorum::get().deconstruct().into();
            let meets_quorum =
                total_votes.saturating_mul(100u32) > total_supply.saturating_mul(quorum_percent);
            let is_approved =
                voting_result.yes_voting_power > voting_result.no_voting_power && meets_quorum;

            let expired = current_block_number > property_lawyer_details.legal_process_expiry;

            // If approved and legal process not expired, assign the lawyer and allocate costs.
            if is_approved && !expired {
                property_lawyer_details.spv_lawyer = Some(proposal.lawyer.clone());

                // Allocate the lawyer's costs from the collected fees.
                let allocated_costs = Self::allocate_fees(
                    &mut property_lawyer_details.spv_lawyer_costs,
                    &property_details.collected_fees,
                    proposal.costs,
                )?;
                Self::deposit_event(Event::<T>::LawyerCostsAllocated {
                    listing_id,
                    lawyer_account: proposal.lawyer.clone(),
                    lawyer_type: LegalProperty::SpvSide,
                    costs: allocated_costs,
                });
                <T as pallet::Config>::RegionProvider::increment_active_cases(&proposal.lawyer)?;
                PropertyLawyer::<T>::insert(listing_id, property_lawyer_details.clone());
            }
            // Clean up proposal and voting records from storage.
            SpvLawyerProposal::<T>::remove(proposal_id);
            OngoingLawyerVoting::<T>::remove(proposal_id);
            ListingSpvProposal::<T>::remove(listing_id);

            Self::deposit_event(Event::SpvLawyerVoteFinalized {
                listing_id,
                lawyer: proposal.lawyer,
                is_approved,
                final_yes_power: voting_result.yes_voting_power,
                final_no_power: voting_result.no_voting_power,
                final_abstain_power: voting_result.abstain_voting_power,
            });
            Ok(())
        }

        /// Lets a voter unlock his locked shares after voting on a spv lawyer.
        ///
        /// The origin must be signed and have sufficient funds.
        ///
        /// Parameters:
        /// - `proposal_id`: Id of the spv lawyer proposal.
        ///
        /// Emits `SharesUnfrozen` event when successful.
        #[pallet::call_index(23)]
        #[pallet::weight(<T as pallet::Config>::WeightInfo::unfreeze_spv_lawyer_shares())]
        pub fn unfreeze_spv_lawyer_shares(
            origin: OriginFor<T>,
            proposal_id: ProposalId,
        ) -> DispatchResult {
            let signer = ensure_signed(origin)?;
            // Retrieve the vote record.
            let vote_record =
                UserLawyerVote::<T>::get(proposal_id, &signer).ok_or(Error::<T>::NoFrozenAmount)?;

            // Ensure voting period has expired.
            if let Some(proposal) = SpvLawyerProposal::<T>::get(proposal_id) {
                let current_block_number =
                    <T as pallet::Config>::BlockNumberProvider::current_block_number();
                ensure!(
                    proposal.expiry_block <= current_block_number,
                    Error::<T>::VotingStillOngoing
                );
            }

            // Unfreeze the voter's shares.
            T::AssetsFreezer::decrease_frozen(
                vote_record.asset_id,
                &MarketplaceFreezeReason::SpvLawyerVoting,
                &signer,
                vote_record.power.into(),
            )?;

            // Remove vote record.
            UserLawyerVote::<T>::remove(proposal_id, &signer);

            Self::deposit_event(Event::SharesUnfrozen {
                proposal_id,
                asset_id: vote_record.asset_id,
                voter: signer,
                amount: vote_record.power,
            });
            Ok(())
        }

        /// Lets a lawyer step back from a case.
        ///
        /// The origin must be Signed by a Lawyer and have sufficient funds.
        ///
        /// Parameters:
        /// - `listing_id`: The listing from the property.
        ///
        /// Emits `LawyerRemovedFromCase` event when successful.
        #[pallet::call_index(24)]
        #[pallet::weight(<T as pallet::Config>::WeightInfo::remove_lawyer_claim())]
        pub fn remove_lawyer_claim(origin: OriginFor<T>, listing_id: ListingId) -> DispatchResult {
            let signer =
                <T as pallet::Config>::PermissionOrigin::ensure_origin(origin, &Role::Lawyer)?;
            // Ensure the caller is a registered lawyer.
            ensure!(
                <T as pallet::Config>::RegionProvider::get_lawyer_info(&signer).is_some(),
                Error::<T>::NoPermission
            );
            let mut property_lawyer_details =
                PropertyLawyer::<T>::get(listing_id).ok_or(Error::<T>::InvalidIndex)?;
            // Check if the caller is the developer's lawyer or SPV lawyer.
            if property_lawyer_details.real_estate_developer_lawyer.as_ref() == Some(&signer) {
                // Can only step back if documents are still pending.
                ensure!(
                    property_lawyer_details.real_estate_developer_status == DocumentStatus::Pending,
                    Error::<T>::AlreadyConfirmed
                );
                property_lawyer_details.real_estate_developer_lawyer = None;
            } else if property_lawyer_details.spv_lawyer.as_ref() == Some(&signer) {
                // Can only step back if documents are still pending.
                ensure!(
                    property_lawyer_details.spv_status == DocumentStatus::Pending,
                    Error::<T>::AlreadyConfirmed
                );
                property_lawyer_details.spv_lawyer = None;
            } else {
                return Err(Error::<T>::NoPermission.into());
            }
            // Decrement active cases for the lawyer stepping back.
            <T as pallet::Config>::RegionProvider::decrement_active_cases(&signer)?;
            PropertyLawyer::<T>::insert(listing_id, property_lawyer_details);
            Self::deposit_event(Event::<T>::LawyerRemovedFromCase { lawyer: signer, listing_id });
            Ok(())
        }

        /// Allows a lawyer to confirm or reject a legal case.
        ///
        /// The origin must be Signed by a Lawyer and have sufficient funds.
        ///
        /// Parameters:
        /// - `listing_id`: The listing from the property.
        /// - `approve`: Approves or Rejects the case.
        ///
        /// Emits `DocumentsConfirmed` event when successful.
        #[pallet::call_index(25)]
        #[pallet::weight(<T as pallet::Config>::WeightInfo::lawyer_confirm_documents(
            <T as pallet::Config>::MaxPropertyShares::get(),
        ))]
        pub fn lawyer_confirm_documents(
            origin: OriginFor<T>,
            listing_id: ListingId,
            approve: bool,
        ) -> DispatchResult {
            let signer =
                <T as pallet::Config>::PermissionOrigin::ensure_origin(origin, &Role::Lawyer)?;
            let mut property_lawyer_details =
                PropertyLawyer::<T>::take(listing_id).ok_or(Error::<T>::InvalidIndex)?;
            let current_block_number =
                <T as pallet::Config>::BlockNumberProvider::current_block_number();
            // Ensure legal process has not expired (lawyers can only confirm during active process).
            ensure!(
                property_lawyer_details.legal_process_expiry >= current_block_number,
                Error::<T>::LegalProcessFailed
            );
            // Check if the caller is the developer's lawyer or SPV lawyer.
            if property_lawyer_details.real_estate_developer_lawyer.as_ref() == Some(&signer) {
                ensure!(
                    property_lawyer_details.real_estate_developer_status == DocumentStatus::Pending,
                    Error::<T>::AlreadyConfirmed
                );
                // Update developer lawyer status based on approval/rejection.
                property_lawyer_details.real_estate_developer_status =
                    if approve { DocumentStatus::Approved } else { DocumentStatus::Rejected };
                Self::deposit_event(Event::<T>::DocumentsConfirmed {
                    signer,
                    listing_id,
                    legal_side: LegalProperty::RealEstateDeveloperSide,
                    approve,
                });
            } else if property_lawyer_details.spv_lawyer.as_ref() == Some(&signer) {
                ensure!(
                    property_lawyer_details.spv_status == DocumentStatus::Pending,
                    Error::<T>::AlreadyConfirmed
                );
                // Update SPV lawyer status based on approval/rejection.
                property_lawyer_details.spv_status =
                    if approve { DocumentStatus::Approved } else { DocumentStatus::Rejected };
                Self::deposit_event(Event::<T>::DocumentsConfirmed {
                    signer,
                    listing_id,
                    legal_side: LegalProperty::SpvSide,
                    approve,
                });
            } else {
                return Err(Error::<T>::NoPermission.into());
            }

            // Retrieve current statuses to determine next steps.
            let developer_status = property_lawyer_details.real_estate_developer_status.clone();
            let spv_status = property_lawyer_details.spv_status.clone();

            // Handle all possible status combinations.
            match (developer_status, spv_status) {
                (DocumentStatus::Approved, DocumentStatus::Approved) => {
                    // Both lawyers approve: execute the deal.
                    Self::execute_deal(listing_id, property_lawyer_details.clone())?;
                }
                (DocumentStatus::Rejected, DocumentStatus::Rejected) => {
                    // Both lawyers reject: process refund.
                    Self::reject_and_refund(listing_id, &property_lawyer_details)?;
                }
                (DocumentStatus::Approved, DocumentStatus::Rejected) => {
                    // One approves, one rejects: allow second attempt or refund.
                    if !property_lawyer_details.second_attempt {
                        property_lawyer_details.spv_status = DocumentStatus::Pending;
                        property_lawyer_details.real_estate_developer_status =
                            DocumentStatus::Pending;
                        property_lawyer_details.second_attempt = true;
                        PropertyLawyer::<T>::insert(listing_id, property_lawyer_details);
                    } else {
                        Self::reject_and_refund(listing_id, &property_lawyer_details)?;
                    }
                }
                (DocumentStatus::Rejected, DocumentStatus::Approved) => {
                    // One approves, one rejects: allow second attempt or refund.
                    if !property_lawyer_details.second_attempt {
                        property_lawyer_details.spv_status = DocumentStatus::Pending;
                        property_lawyer_details.real_estate_developer_status =
                            DocumentStatus::Pending;
                        property_lawyer_details.second_attempt = true;
                        PropertyLawyer::<T>::insert(listing_id, property_lawyer_details);
                    } else {
                        Self::reject_and_refund(listing_id, &property_lawyer_details)?;
                    }
                }
                _ => {
                    // Still pending: just update the storage.
                    PropertyLawyer::<T>::insert(listing_id, property_lawyer_details);
                }
            }
            Ok(())
        }

        /// Allows a sender to transfer property shares to another account.
        ///
        /// The origin must be Signed by a RealEstateInvestor and have sufficient funds.
        ///
        /// Parameters:
        /// - `asset_id`: The asset id of the property.
        /// - `receiver`: AccountId of the person that the seller wants to handle the offer from.
        /// - `share_amount`: The amount of shares the sender wants to send.
        ///
        /// Emits `DocumentsConfirmed` event when successful.
        #[pallet::call_index(26)]
        #[pallet::weight(<T as pallet::Config>::WeightInfo::send_property_shares())]
        pub fn send_property_shares(
            origin: OriginFor<T>,
            asset_id: u32,
            receiver: AccountIdOf<T>,
            share_amount: u32,
        ) -> DispatchResult {
            // Verify the caller is a compliant RealEstateInvestor.
            let sender = <T as pallet::Config>::CompliantOrigin::ensure_origin(
                origin,
                &Role::RealEstateInvestor,
            )?;
            // Ensure the receiver is compliant with the RealEstateInvestor role.
            ensure!(
                <T as pallet::Config>::Whitelist::is_compliant(&receiver, Role::RealEstateInvestor),
                Error::<T>::UserNotCompliant
            );

            Self::restrict_ownership(asset_id, &receiver, share_amount)?;
            // Settle any pending income for both sender and receiver before the transfer.
            T::IncomeSettlement::settle_income(sender.clone(), asset_id)?;
            T::IncomeSettlement::settle_income(receiver.clone(), asset_id)?;
            // Execute the share transfer.
            T::PropertyShares::transfer_property_shares(
                asset_id,
                &sender,
                &sender,
                &receiver,
                share_amount,
            )?;

            Self::deposit_event(Event::<T>::PropertySharesSent {
                asset_id,
                sender,
                receiver,
                amount: share_amount,
            });
            Ok(())
        }
    }

    impl<T: Config> Pallet<T> {
        /// Returns the account ID of the pallet.
        pub fn account_id() -> AccountIdOf<T> {
            <T as pallet::Config>::PalletId::get().into_account_truncating()
        }

        /// Returns the account ID for a specific property based on its asset ID.
        pub fn property_account_id(asset_id: u32) -> AccountIdOf<T> {
            <T as pallet::Config>::PalletId::get().into_sub_account_truncating(("pr", asset_id))
        }

        /// Returns the account ID of the treasury pallet.
        pub fn treasury_account_id() -> AccountIdOf<T> {
            <T as pallet::Config>::TreasuryId::get().into_account_truncating()
        }

        /// Calculates the next listing ID by incrementing the provided ID.
        pub fn next_listing_id(listing_id: ListingId) -> Result<ListingId, Error<T>> {
            listing_id.checked_add(1).ok_or(Error::<T>::ArithmeticOverflow)
        }

        /// Executes the deal by distributing shares to owners and funds to the real estate developer.
        ///
        /// Called when all shares (100%) of a collection are sold and both lawyers approve the deal.
        fn execute_deal(
            listing_id: u32,
            property_lawyer_details: PropertyLawyerDetails<T>,
        ) -> DispatchResult {
            // Retrieve property listing and asset details.
            let property_details =
                OngoingObjectListing::<T>::take(listing_id).ok_or(Error::<T>::ListingNotFound)?;
            let asset_details =
                T::PropertyShares::get_property_asset_info(property_details.asset_id)
                    .ok_or(Error::<T>::NoObjectFound)?;
            let treasury_id = Self::treasury_account_id();
            let property_account = Self::property_account_id(property_details.asset_id);
            let region =
                <T as pallet::Config>::RegionProvider::get_region_details(asset_details.region)
                    .ok_or(Error::<T>::RegionUnknown)?;

            // Verify and retrieve lawyer accounts
            let real_estate_developer_lawyer_id = property_lawyer_details
                .real_estate_developer_lawyer
                .ok_or(Error::<T>::LawyerNotFound)?;
            let spv_lawyer_id =
                property_lawyer_details.spv_lawyer.ok_or(Error::<T>::LawyerNotFound)?;
            // Decrement active cases for both lawyers as the deal is being finalized.
            <T as pallet::Config>::RegionProvider::decrement_active_cases(
                &real_estate_developer_lawyer_id,
            )?;
            <T as pallet::Config>::RegionProvider::decrement_active_cases(&spv_lawyer_id)?;

            let mut developer_payout = BoundedBTreeMap::new();
            let mut spv_lawyer_payout = BoundedBTreeMap::new();
            let mut treasury_payout = BoundedBTreeMap::new();
            let mut region_owner_payout = BoundedBTreeMap::new();

            // Distribute funds from property account for each asset.
            for &asset in T::AcceptedAssets::get().iter() {
                // Fetch total collected funds, lawyer costs, tax, and fees for the asset.
                let total_collected_funds = property_details
                    .collected_funds
                    .get(&asset)
                    .copied()
                    .ok_or(Error::<T>::AssetNotSupported)?;
                let spv_lawyer_costs = property_lawyer_details
                    .spv_lawyer_costs
                    .get(&asset)
                    .copied()
                    .ok_or(Error::<T>::AssetNotSupported)?;
                let tax = property_details
                    .collected_tax
                    .get(&asset)
                    .copied()
                    .ok_or(Error::<T>::AssetNotSupported)?;
                let collected_fees = property_details
                    .collected_fees
                    .get(&asset)
                    .copied()
                    .ok_or(Error::<T>::AssetNotSupported)?;

                // Validate marketplace fee percentage.
                let fee_percentage = T::MarketplaceFeePercentage::get();
                ensure!(
                    fee_percentage <= Perbill::from_percent(100),
                    Error::<T>::InvalidFeePercentage
                );

                // Calculate developer’s share.
                let developer_percentage = Perbill::from_percent(100)
                    .checked_sub(&fee_percentage)
                    .ok_or(Error::<T>::ArithmeticUnderflow)?;
                let mut developer_amount = developer_percentage.mul_floor(total_collected_funds);

                // Determine tax allocation based on who pays the tax.
                let (developer_lawyer_tax, spv_lawyer_tax) =
                    if property_details.tax_paid_by_developer {
                        developer_amount = developer_amount
                            .checked_sub(&tax)
                            .ok_or(Error::<T>::ArithmeticUnderflow)?;
                        (tax, Zero::zero())
                    } else {
                        (Zero::zero(), tax)
                    };

                // Calculate SPV lawyer's total amount including tax.
                let spv_lawyer_amount = spv_lawyer_costs
                    .checked_add(&spv_lawyer_tax)
                    .ok_or(Error::<T>::ArithmeticOverflow)?;

                // Calculate protocol fees and split between treasury and region owner
                let protocol_fees = total_collected_funds
                    .checked_div(&(100u128.into()))
                    .ok_or(Error::<T>::DivisionError)?
                    .checked_add(&collected_fees)
                    .ok_or(Error::<T>::ArithmeticOverflow)?
                    .saturating_sub(spv_lawyer_costs);
                let region_owner_amount =
                    protocol_fees.checked_div(&(2u128.into())).ok_or(Error::<T>::DivisionError)?;
                let treasury_amount = protocol_fees.saturating_sub(region_owner_amount);

                // Perform fund transfers to all parties.
                Self::transfer_funds(
                    &property_account,
                    &property_details.real_estate_developer,
                    developer_amount,
                    asset,
                )?;
                if !developer_lawyer_tax.is_zero() {
                    Self::transfer_funds(
                        &property_account,
                        &real_estate_developer_lawyer_id,
                        developer_lawyer_tax,
                        asset,
                    )?;
                }
                Self::transfer_funds(&property_account, &spv_lawyer_id, spv_lawyer_amount, asset)?;
                Self::transfer_funds(&property_account, &treasury_id, treasury_amount, asset)?;
                Self::transfer_funds(&property_account, &region.owner, region_owner_amount, asset)?;

                // Record payouts based on asset type
                developer_payout
                    .try_insert(asset, developer_amount)
                    .map_err(|_| Error::<T>::ExceedsMaxEntries)?;
                spv_lawyer_payout
                    .try_insert(asset, spv_lawyer_amount)
                    .map_err(|_| Error::<T>::ExceedsMaxEntries)?;
                treasury_payout
                    .try_insert(asset, treasury_amount)
                    .map_err(|_| Error::<T>::ExceedsMaxEntries)?;
                region_owner_payout
                    .try_insert(asset, region_owner_amount)
                    .map_err(|_| Error::<T>::ExceedsMaxEntries)?;
            }
            // Finalize the property share
            T::PropertyShares::finalize_property(property_details.asset_id)?;
            // Release the listing deposit
            if let Some((depositor, deposit_amount)) = ListingDeposits::<T>::take(listing_id) {
                <T as pallet::Config>::NativeCurrency::release(
                    &HoldReason::ListingDepositReserve.into(),
                    &depositor,
                    deposit_amount,
                    Precision::Exact,
                )?;
            }

            let payouts = FinalSettlementPayouts {
                developer_payout,
                developer_account: property_details.real_estate_developer.clone(),
                spv_lawyer_payout,
                spv_lawyer_account: spv_lawyer_id.clone(),
                treasury_payout,
                treasury_account: treasury_id.clone(),
                region_owner_payout,
                region_owner_account: region.owner.clone(),
            };

            Self::deposit_event(Event::<T>::PrimarySaleCompleted {
                listing_id,
                asset_id: property_details.asset_id,
                payouts,
            });
            Ok(())
        }

        /// Processes a refund when a deal is rejected by lawyers.
        fn reject_and_refund(
            listing_id: u32,
            property_lawyer_details: &PropertyLawyerDetails<T>,
        ) -> DispatchResult {
            let property_details =
                OngoingObjectListing::<T>::get(listing_id).ok_or(Error::<T>::ListingNotFound)?;
            // Verify and retrieve lawyer accounts.
            let real_estate_developer_lawyer_id = property_lawyer_details
                .real_estate_developer_lawyer
                .clone()
                .ok_or(Error::<T>::LawyerNotFound)?;
            let spv_lawyer_id =
                property_lawyer_details.spv_lawyer.clone().ok_or(Error::<T>::LawyerNotFound)?;
            // Decrement active case counts for both lawyers.
            <T as pallet::Config>::RegionProvider::decrement_active_cases(
                &real_estate_developer_lawyer_id,
            )?;
            <T as pallet::Config>::RegionProvider::decrement_active_cases(&spv_lawyer_id)?;
            // Store refund information.
            RefundShare::<T>::insert(
                listing_id,
                RefundInfos {
                    refund_amount: property_details.share_amount,
                    property_lawyer_details: property_lawyer_details.clone(),
                },
            );
            Ok(())
        }

        /// Refunds investors and distributes fees when a deal is rejected.
        fn refund_investors_with_fees(
            property_details: &PropertyListingDetailsType<T>,
            property_lawyer_details: PropertyLawyerDetails<T>,
        ) -> DispatchResult {
            let property_account = Self::property_account_id(property_details.asset_id);
            let treasury_id = Self::treasury_account_id();
            let spv_lawyer_id =
                property_lawyer_details.spv_lawyer.ok_or(Error::<T>::LawyerNotFound)?;

            // Distribute funds for each accepted asset.
            for asset in T::AcceptedAssets::get().iter() {
                // Fetch fees and lawyer costs.
                let fees = property_details
                    .collected_fees
                    .get(asset)
                    .copied()
                    .ok_or(Error::<T>::AssetNotSupported)?;
                let lawyer_costs = property_lawyer_details
                    .spv_lawyer_costs
                    .get(asset)
                    .copied()
                    .ok_or(Error::<T>::AssetNotSupported)?;

                // Calculate treasury amount after lawyer costs.
                let treasury_amount =
                    fees.checked_sub(&lawyer_costs).ok_or(Error::<T>::ArithmeticUnderflow)?;

                // Transfer funds to treasury and SPV lawyer.
                Self::transfer_funds(&property_account, &treasury_id, treasury_amount, *asset)?;
                Self::transfer_funds(&property_account, &spv_lawyer_id, lawyer_costs, *asset)?;
            }
            // Clear ownership tracking
            T::PropertyShares::clear_share_owners(property_details.asset_id)?;
            Ok(())
        }

        /// Processes the purchase of relisted shares.
        fn buying_shares_process(
            listing_id: u32,
            transfer_from: &AccountIdOf<T>,
            account: &AccountIdOf<T>,
            mut listing_details: ListingDetailsType<T>,
            price: <T as pallet::Config>::Balance,
            amount: u32,
            payment_asset: u32,
        ) -> DispatchResult {
            // Calculate and distribute fees.
            Self::calculate_fees(price, transfer_from, &listing_details.seller, payment_asset)?;
            let property_account = Self::property_account_id(listing_details.asset_id);
            // Settle any pending income for both buyer and seller.
            T::IncomeSettlement::settle_income(
                listing_details.seller.clone(),
                listing_details.asset_id,
            )?;
            T::IncomeSettlement::settle_income(account.clone(), listing_details.asset_id)?;
            // Transfer property shares to buyer.
            T::PropertyShares::transfer_property_shares(
                listing_details.asset_id,
                &listing_details.seller,
                &property_account,
                account,
                amount,
            )?;
            // Update remaining share amount in the listing.
            listing_details.amount = listing_details
                .amount
                .checked_sub(amount)
                .ok_or(Error::<T>::ArithmeticUnderflow)?;
            if listing_details.amount > 0 {
                ShareListings::<T>::insert(listing_id, listing_details.clone());
            }
            Self::deposit_event(Event::<T>::RelistedSharesBought {
                listing_index: listing_id,
                asset_id: listing_details.asset_id,
                buyer: account.clone(),
                seller: listing_details.seller,
                price: listing_details.share_price,
                amount,
                payment_asset,
                new_amount_remaining: listing_details.amount,
            });
            Ok(())
        }

        /// Unfreezes shares held for a share owner.
        fn unfreeze_shares(
            share_details: &ShareOwnerDetails<T>,
            signer: &AccountIdOf<T>,
        ) -> DispatchResult {
            for asset in T::AcceptedAssets::get().iter() {
                if let Some(paid_funds) = share_details.paid_funds.get(asset).copied() {
                    if paid_funds.is_zero() {
                        continue;
                    }
                    let paid_tax = share_details.paid_tax.get(asset).copied().unwrap_or_default();

                    // Calculate refund and investor fee (1% of paid funds)
                    let refund_amount =
                        paid_funds.checked_add(&paid_tax).ok_or(Error::<T>::ArithmeticOverflow)?;
                    let investor_fee = paid_funds
                        .checked_div(&(100u128.into()))
                        .ok_or(Error::<T>::DivisionError)?;
                    let total_investor_amount = refund_amount
                        .checked_add(&investor_fee)
                        .ok_or(Error::<T>::ArithmeticOverflow)?;

                    // Release funds.
                    T::ForeignAssetsHolder::release(
                        *asset,
                        &MarketplaceHoldReason::Marketplace,
                        signer,
                        total_investor_amount,
                        Precision::Exact,
                    )?;
                }
            }
            Ok(())
        }

        /// Unfreezes shares and returns refund details
        #[allow(clippy::type_complexity)]
        fn unfreeze_shares_with_refunds(
            share_details: &ShareOwnerDetails<T>,
            signer: &AccountIdOf<T>,
        ) -> Result<
            BoundedBTreeMap<
                u32,
                (<T as pallet::Config>::Balance, <T as pallet::Config>::Balance), // (principal, tax)
                <T as pallet::Config>::MaxAcceptedAssets,
            >,
            DispatchError,
        > {
            let mut refunds = BoundedBTreeMap::new();

            // Unfreeze funds for each accepted asset and track refunds.
            for asset in T::AcceptedAssets::get().iter() {
                if let Some(paid_funds) = share_details.paid_funds.get(asset).copied() {
                    if paid_funds.is_zero() {
                        continue;
                    }

                    let default = Zero::zero();
                    let paid_tax = share_details.paid_tax.get(asset).copied().unwrap_or(default);

                    // Calculate refund and investor fee (1% of paid funds)
                    let refund_amount =
                        paid_funds.checked_add(&paid_tax).ok_or(Error::<T>::ArithmeticOverflow)?;
                    let investor_fee = paid_funds
                        .checked_div(&(100u128.into()))
                        .ok_or(Error::<T>::DivisionError)?;
                    let total_investor_amount = refund_amount
                        .checked_add(&investor_fee)
                        .ok_or(Error::<T>::ArithmeticOverflow)?;

                    // Release funds
                    T::ForeignAssetsHolder::release(
                        *asset,
                        &MarketplaceHoldReason::Marketplace,
                        signer,
                        total_investor_amount,
                        Precision::Exact,
                    )?;

                    // Record refund details
                    refunds
                        .try_insert(*asset, (paid_funds, paid_tax))
                        .map_err(|_| Error::<T>::ExceedsMaxEntries)?;
                }
            }
            Ok(refunds)
        }

        /// Calculates and distributes fees for a share purchase.
        fn calculate_fees(
            price: <T as pallet::Config>::Balance,
            sender: &AccountIdOf<T>,
            receiver: &AccountIdOf<T>,
            asset: u32,
        ) -> DispatchResult {
            let fee_percent = T::MarketplaceFeePercentage::get();
            ensure!(fee_percent < Perbill::from_percent(100), Error::<T>::InvalidFeePercentage);

            // Calculate fees as fee_percent of price, rounding up to prevent undercharging
            let fees = fee_percent.mul_ceil(price);
            let treasury_id = Self::treasury_account_id();
            let seller_part = price.checked_sub(&fees).ok_or(Error::<T>::ArithmeticUnderflow)?;

            // Transfer fees to treasury and remaining amount to seller.
            Self::transfer_funds(sender, &treasury_id, fees, asset)?;
            Self::transfer_funds(sender, receiver, seller_part, asset)?;
            Ok(())
        }

        /// Transfers funds between accounts for a specific asset.
        fn transfer_funds(
            from: &AccountIdOf<T>,
            to: &AccountIdOf<T>,
            amount: <T as pallet::Config>::Balance,
            asset: u32,
        ) -> DispatchResult {
            if !amount.is_zero() {
                T::ForeignCurrency::transfer(asset, from, to, amount, Preservation::Expendable)
                    .map_err(|_| Error::<T>::NotEnoughFunds)?;
            }
            Ok(())
        }

        /// Creates an initial funds map for accepted assets.
        fn create_initial_funds() -> Result<
            BoundedBTreeMap<
                u32,
                <T as pallet::Config>::Balance,
                <T as pallet::Config>::MaxAcceptedAssets,
            >,
            DispatchError,
        > {
            let mut map = BoundedBTreeMap::default();
            // Initialize all accepted assets with zero balance.
            for &asset in T::AcceptedAssets::get().iter() {
                map.try_insert(asset, Zero::zero()).map_err(|_| Error::<T>::ExceedsMaxEntries)?;
            }
            Ok(map)
        }

        /// Updates the funds map by adding value to the specified asset.
        fn update_map(
            map: &mut BoundedBTreeMap<
                u32,
                <T as pallet::Config>::Balance,
                <T as pallet::Config>::MaxAcceptedAssets,
            >,
            asset: u32,
            value: <T as pallet::Config>::Balance,
        ) -> DispatchResult {
            // Update existing entry
            if let Some(existing) = map.get_mut(&asset) {
                *existing = existing.checked_add(&value).ok_or(Error::<T>::ArithmeticOverflow)?;
            } else {
                // Insert new entry if it doesn't exist
                map.try_insert(asset, value).map_err(|_| Error::<T>::ExceedsMaxEntries)?;
            }
            Ok(())
        }

        /// Allocates fees for lawyer costs.
        fn allocate_fees(
            lawyer_costs: &mut BoundedBTreeMap<
                u32,
                <T as pallet::Config>::Balance,
                <T as pallet::Config>::MaxAcceptedAssets,
            >,
            collected_fees: &BoundedBTreeMap<
                u32,
                <T as pallet::Config>::Balance,
                <T as pallet::Config>::MaxAcceptedAssets,
            >,
            total_costs: <T as pallet::Config>::Balance,
        ) -> Result<
            BoundedBTreeMap<
                u32,
                <T as pallet::Config>::Balance,
                <T as pallet::Config>::MaxAcceptedAssets,
            >,
            DispatchError,
        > {
            let mut remaining_costs = total_costs;
            let mut allocated_costs = BoundedBTreeMap::new();

            // Calculate total available fees.
            let mut total_fees: <T as pallet::Config>::Balance = Zero::zero();
            for asset in T::AcceptedAssets::get().iter() {
                let fee = collected_fees.get(asset).copied().unwrap_or(Zero::zero());
                total_fees = total_fees.checked_add(&fee).ok_or(Error::<T>::ArithmeticOverflow)?;
            }
            ensure!(total_fees >= total_costs, Error::<T>::CostsTooHigh);

            // Allocate costs across assets based on available fees.
            for asset in T::AcceptedAssets::get().iter() {
                if remaining_costs.is_zero() {
                    break;
                }
                let fee = collected_fees.get(asset).copied().unwrap_or(Zero::zero());
                if !fee.is_zero() {
                    let cost_to_allocate =
                        if fee >= remaining_costs { remaining_costs } else { fee };
                    // Update lawyer costs.
                    lawyer_costs
                        .try_insert(*asset, cost_to_allocate)
                        .map_err(|_| Error::<T>::ExceedsMaxEntries)?;
                    // Track allocated costs.
                    allocated_costs
                        .try_insert(*asset, cost_to_allocate)
                        .map_err(|_| Error::<T>::ExceedsMaxEntries)?;
                    remaining_costs = remaining_costs
                        .checked_sub(&cost_to_allocate)
                        .ok_or(Error::<T>::ArithmeticUnderflow)?;
                }
            }
            // Ensure full allocation
            ensure!(remaining_costs.is_zero(), Error::<T>::CostsTooHigh);
            Ok(allocated_costs)
        }

        /// Restricts share ownership to prevent exceeding maximum ownership limits.
        fn restrict_ownership(
            asset_id: u32,
            account: &AccountIdOf<T>,
            amount: u32,
        ) -> DispatchResult {
            let property_info =
                <T as pallet::Config>::PropertyShares::get_property_asset_info(asset_id)
                    .ok_or(Error::<T>::NoObjectFound)?;
            // Calculate maximum allowable shares.
            let max_shares = T::MaxOwnershipPercentage::get().mul_floor(property_info.share_amount);
            // Current + new amount
            let owned_shares =
                <T as pallet::Config>::PropertyShares::get_share_balance(asset_id, account);
            let new_share_amount =
                owned_shares.checked_add(amount).ok_or(Error::<T>::ArithmeticOverflow)?;
            // Ensure ownership does not exceed maximum limit.
            ensure!(new_share_amount < max_shares, Error::<T>::ExceedsMaxOwnership);
            Ok(())
        }
    }
}

sp_api::decl_runtime_apis! {
    pub trait NftMarketplaceApi<AccountId>
    where
        AccountId: Codec
    {
        fn get_marketplace_account_id() -> AccountId;
    }
}
