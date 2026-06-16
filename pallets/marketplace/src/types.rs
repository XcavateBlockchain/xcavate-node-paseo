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

use crate::*;
use frame_support::pallet_prelude::*;
use frame_system::pallet_prelude::*;

use frame_support::sp_runtime::Permill;
use frame_support::{sp_runtime::RuntimeDebug, DefaultNoBound};
use parity_scale_codec::{Decode, DecodeWithMemTracking, Encode, MaxEncodedLen};
use scale_info::TypeInfo;

/// Stores details about a real estate property listing in the marketplace.
#[derive(Encode, Decode, PartialEq, Eq, MaxEncodedLen, RuntimeDebug, TypeInfo)]
#[scale_info(skip_type_params(T))]
pub struct PropertyListingDetails<NftId, NftCollectionId, T: Config> {
    /// The account of the real estate developer who listed the property.
    pub real_estate_developer: AccountIdOf<T>,
    /// The price per share for the property.
    pub share_price: <T as pallet::Config>::Balance,
    /// Funds collected from investors, mapped by asset ID.
    pub collected_funds: BoundedBTreeMap<
        u32,
        <T as pallet::Config>::Balance,
        <T as pallet::Config>::MaxAcceptedAssets,
    >,
    /// Taxes collected, mapped by asset ID.
    pub collected_tax: BoundedBTreeMap<
        u32,
        <T as pallet::Config>::Balance,
        <T as pallet::Config>::MaxAcceptedAssets,
    >,
    /// Fees collected, mapped by asset ID.
    pub collected_fees: BoundedBTreeMap<
        u32,
        <T as pallet::Config>::Balance,
        <T as pallet::Config>::MaxAcceptedAssets,
    >,
    /// The asset ID representing the property.
    pub asset_id: u32,
    /// The unique ID of the NFT representing the property.
    pub item_id: NftId,
    /// The NFT collection ID of the region of the property.
    pub collection_id: NftCollectionId,
    /// The total number of shares issued for the property.
    pub share_amount: u32,
    /// The number of shares currently listed for sale.
    pub listed_share_amount: u32,
    /// Indicates whether the developer pays the tax (true) or it’s passed to investors (false).
    pub tax_paid_by_developer: bool,
    /// The tax rate applied to the property sale.
    pub tax: Permill,
    /// The block number when the listing expires.
    pub listing_expiry: BlockNumberFor<T>,
    /// Mapping of investor accounts to their funds and fees paid.
    pub investor_funds: BoundedBTreeMap<
        AccountIdOf<T>,
        ShareOwnerFunds<T>,
        <T as pallet::Config>::MaxPropertyShares,
    >,
    /// The block number when claims expire, if applicable.
    pub claim_expiry: Option<BlockNumberFor<T>>,
    /// The number of times the property has been relisted.
    pub relist_count: u8,
    /// The number of shares that remain unclaimed.
    pub unclaimed_share_amount: u32,
}

/// Infos regarding the listing of a share.
#[cfg_attr(feature = "std", derive(serde::Serialize, serde::Deserialize))]
#[derive(Encode, Decode, Clone, PartialEq, Eq, MaxEncodedLen, RuntimeDebug, TypeInfo)]
#[scale_info(skip_type_params(T))]
pub struct ShareListingDetails<NftId, NftCollectionId, T: Config> {
    /// The account ID of the seller listing the shares.
    pub seller: AccountIdOf<T>,
    /// The price per share for the listing.
    pub share_price: <T as pallet::Config>::Balance,
    /// The asset ID of the property shares being listed.
    pub asset_id: u32,
    /// The unique ID of the NFT representing the property.
    pub item_id: NftId,
    /// The NFT collection ID of the region of the property.
    pub collection_id: NftCollectionId,
    /// The number of shares listed for sale.
    pub amount: u32,
}

/// Infos regarding an offer.
#[cfg_attr(feature = "std", derive(serde::Serialize, serde::Deserialize))]
#[derive(Encode, Decode, Clone, PartialEq, Eq, MaxEncodedLen, RuntimeDebug, TypeInfo)]
#[scale_info(skip_type_params(T))]
pub struct OfferDetails<T: Config> {
    /// The price per share offered.
    pub share_price: <T as pallet::Config>::Balance,
    /// Amount of shares the offer is for.
    pub amount: u32,
    /// The asset ID of the payment currency.
    pub payment_assets: u32,
    /// A unique nonce to differentiate offers.
    pub nonce: u64,
}

/// Details about the lawyers involved in the legal process of a property listing.
#[derive(
    Encode,
    Decode,
    DecodeWithMemTracking,
    CloneNoBound,
    PartialEqNoBound,
    EqNoBound,
    MaxEncodedLen,
    RuntimeDebugNoBound,
    TypeInfo,
)]
#[scale_info(skip_type_params(T))]
pub struct PropertyLawyerDetails<T: Config> {
    /// The lawyer representing the real estate developer, if any.
    pub real_estate_developer_lawyer: Option<AccountIdOf<T>>,
    /// The lawyer representing the SPV, if any.
    pub spv_lawyer: Option<AccountIdOf<T>>,
    /// The status of the developer’s legal documents.
    pub real_estate_developer_status: DocumentStatus,
    /// The status of the SPV’s legal documents.
    pub spv_status: DocumentStatus,
    /// Costs incurred by the developer’s lawyer, mapped by asset ID.
    pub real_estate_developer_lawyer_costs: BoundedBTreeMap<
        u32,
        <T as pallet::Config>::Balance,
        <T as pallet::Config>::MaxAcceptedAssets,
    >,
    /// Costs incurred by the SPV’s lawyer, mapped by asset ID.
    pub spv_lawyer_costs: BoundedBTreeMap<
        u32,
        <T as pallet::Config>::Balance,
        <T as pallet::Config>::MaxAcceptedAssets,
    >,
    /// The block number when the legal process expires.
    pub legal_process_expiry: BlockNumberFor<T>,
    /// Indicates if this is a second attempt at the legal process.
    pub second_attempt: bool,
}

/// Details about a share owner’s investment in a property.
#[derive(
    Encode, Decode, Clone, PartialEq, Eq, MaxEncodedLen, RuntimeDebug, TypeInfo, DefaultNoBound,
)]
#[scale_info(skip_type_params(T))]
pub struct ShareOwnerDetails<T: Config> {
    /// The number of shares purchased by the investor.
    pub share_amount: u32,
    /// Funds paid by the investor, mapped by asset ID.
    pub paid_funds: BoundedBTreeMap<
        u32,
        <T as pallet::Config>::Balance,
        <T as pallet::Config>::MaxAcceptedAssets,
    >,
    /// Taxes paid by the owner, mapped by asset ID.
    pub paid_tax: BoundedBTreeMap<
        u32,
        <T as pallet::Config>::Balance,
        <T as pallet::Config>::MaxAcceptedAssets,
    >,
    /// At which relisted count the shares were purchased.
    pub relist_count: u8,
}

/// Details about funds and fees paid by a share owner.
#[derive(
    Encode, Decode, Clone, PartialEq, Eq, MaxEncodedLen, RuntimeDebug, TypeInfo, DefaultNoBound,
)]
#[scale_info(skip_type_params(T))]
pub struct ShareOwnerFunds<T: Config> {
    /// Funds paid by the investor, mapped by asset ID.
    pub paid_funds: BoundedBTreeMap<
        u32,
        <T as pallet::Config>::Balance,
        <T as pallet::Config>::MaxAcceptedAssets,
    >,
    /// Fees paid by the investor, mapped by asset ID.
    pub paid_fee: BoundedBTreeMap<
        u32,
        <T as pallet::Config>::Balance,
        <T as pallet::Config>::MaxAcceptedAssets,
    >,
}

/// Infos regarding refunds and lawyer details.
#[derive(Encode, Decode, Clone, MaxEncodedLen, RuntimeDebug, TypeInfo)]
#[scale_info(skip_type_params(T))]
pub struct RefundInfos<T: Config> {
    /// The number of that have been sold.
    pub refund_amount: u32,
    /// The legal details associated with the refund process.
    pub property_lawyer_details: PropertyLawyerDetails<T>,
}

/// Implementation for OfferDetails to calculate the total offer amount.
impl<T: Config> OfferDetails<T>
where
    <T as pallet::Config>::Balance: CheckedMul + TryFrom<u128>,
{
    pub fn get_total_amount(&self) -> Result<<T as pallet::Config>::Balance, Error<T>> {
        let amount_in_balance: <T as pallet::Config>::Balance = (self.amount as u128).into();

        self.share_price.checked_mul(&amount_in_balance).ok_or(Error::<T>::MultiplyError)
    }
}

/// Details about a proposed lawyer for the real estate developer.
#[derive(Encode, Decode, Clone, MaxEncodedLen, RuntimeDebug, TypeInfo)]
#[scale_info(skip_type_params(T))]
pub struct ProposedDeveloperLawyer<T: Config> {
    /// The account ID of the proposed lawyer.
    pub lawyer: AccountIdOf<T>,
    /// The cost of the lawyer’s services.
    pub costs: <T as pallet::Config>::Balance,
}

/// Details about a proposed lawyer for the SPV.
#[derive(Encode, Decode, Clone, MaxEncodedLen, RuntimeDebug, TypeInfo)]
#[scale_info(skip_type_params(T))]
pub struct ProposedSpvLawyer<T: Config> {
    /// The account ID of the proposed lawyer.
    pub lawyer: AccountIdOf<T>,
    /// The asset ID of the associated property.
    pub asset_id: u32,
    /// The cost of the lawyer’s services.
    pub costs: <T as pallet::Config>::Balance,
    /// The block number when the proposal expires.
    pub expiry_block: BlockNumberFor<T>,
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

/// A struct containing all payouts for the final settlement of a primary sale.
#[derive(
    Encode,
    Decode,
    DecodeWithMemTracking,
    CloneNoBound,
    PartialEqNoBound,
    EqNoBound,
    MaxEncodedLen,
    RuntimeDebugNoBound,
    TypeInfo,
)]
#[scale_info(skip_type_params(T))]
pub struct FinalSettlementPayouts<T: Config> {
    /// Payout to the real estate developer.
    pub developer_payout: BoundedBTreeMap<
        u32,
        <T as pallet::Config>::Balance,
        <T as pallet::Config>::MaxAcceptedAssets,
    >,
    /// The account ID of the real estate developer.
    pub developer_account: AccountIdOf<T>,
    /// Payout to the SPV lawyer.
    pub spv_lawyer_payout: BoundedBTreeMap<
        u32,
        <T as pallet::Config>::Balance,
        <T as pallet::Config>::MaxAcceptedAssets,
    >,
    /// The account ID of the SPV lawyer.
    pub spv_lawyer_account: AccountIdOf<T>,
    /// Payout to the treasury.
    pub treasury_payout: BoundedBTreeMap<
        u32,
        <T as pallet::Config>::Balance,
        <T as pallet::Config>::MaxAcceptedAssets,
    >,
    /// The account ID of the treasury.
    pub treasury_account: AccountIdOf<T>,
    /// Payout to the region owner.
    pub region_owner_payout: BoundedBTreeMap<
        u32,
        <T as pallet::Config>::Balance,
        <T as pallet::Config>::MaxAcceptedAssets,
    >,
    /// The account ID of the region owner.
    pub region_owner_account: AccountIdOf<T>,
}

/// // Represents the action taken on an offer.
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
pub enum Offer {
    Accept,
    Reject,
}

/// Indicates the side of the legal process (Real Estate Developer or SPV).
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
pub enum LegalProperty {
    RealEstateDeveloperSide,
    SpvSide,
}

/// Represents the status of legal documents (Pending, Approved, or Rejected).
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
pub enum DocumentStatus {
    Pending,
    Approved,
    Rejected,
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
