#![cfg_attr(not(feature = "std"), no_std)]

use frame_support::{pallet_prelude::DispatchResult, traits::VariantCount};
use parity_scale_codec::{Decode, DecodeWithMemTracking, Encode, MaxEncodedLen};
use scale_info::TypeInfo;
use sp_runtime::{FixedPointNumber, RuntimeDebug};

#[derive(
    Encode,
    Decode,
    DecodeWithMemTracking,
    Copy,
    Clone,
    Eq,
    PartialEq,
    Ord,
    PartialOrd,
    MaxEncodedLen,
    TypeInfo,
    RuntimeDebug,
)]
pub enum MarketplaceHoldReason {
    ModulePurchase,
    Marketplace,
    Listing,
    Auction,
}

impl VariantCount for MarketplaceHoldReason {
    // Intentionally set below the actual count of variants, to allow testing for `can_freeze`
    const VARIANT_COUNT: u32 = 2;
}

#[derive(
    Encode,
    Decode,
    DecodeWithMemTracking,
    Copy,
    Clone,
    Eq,
    PartialEq,
    Ord,
    PartialOrd,
    MaxEncodedLen,
    TypeInfo,
    RuntimeDebug,
)]
pub enum MarketplaceFreezeReason {
    SpvLawyerVoting,
    LettingAgentVoting,
    ProposalVoting,
    ChallengeVoting,
}

impl VariantCount for MarketplaceFreezeReason {
    // Intentionally set below the actual count of variants, to allow testing for `can_freeze`
    const VARIANT_COUNT: u32 = 4;
}

pub trait IncomeSettlement {
    type AccountId;
    fn settle_income(account: Self::AccountId, asset_id: u32) -> DispatchResult;
}

pub trait AssetPriceProvider {
    type Price: FixedPointNumber;
    fn get_price(asset_id: u32) -> Option<Self::Price>;
}

pub trait AssetMetadataProvider {
    type AssetId;

    /// Returns the number of decimals for the asset, if available.
    fn get_decimals(asset_id: Self::AssetId) -> Option<u8>;
}
