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

use pallet_nfts::{CollectionConfig, CollectionSettings, ItemConfig, ItemSettings, MintSettings};

use frame_support::{
    pallet_prelude::*,
    sp_runtime::{
        traits::{AccountIdConversion, BlockNumberProvider, Zero},
        Perbill, Permill, Saturating,
    },
    traits::{
        fungible,
        fungible::{BalancedHold, Credit, Inspect, Mutate, MutateHold},
        fungibles::{
            metadata::{MetadataDeposit, Mutate as MetadataMutate},
            Create as FungiblesCreate, Destroy, Inspect as FungiblesInspect,
            Mutate as FungiblesMutate, MutateHold as FungiblesHold,
        },
        nonfungibles_v2::Mutate as NonfungiblesMutate,
        nonfungibles_v2::{Create, Transfer},
        tokens::{
            fungibles, imbalance::OnUnbalanced, nonfungibles_v2, Balance, Fortitude, Precision,
            Preservation, Preservation::Preserve,
        },
        EnsureOriginWithArg,
    },
    PalletId,
};

use primitives::{AssetMetadataProvider, MarketplaceHoldReason};
use scale_info::prelude::{format, string::String};

use core::fmt::Display;
use pallet_education_regions::{RegionInfo, RegionTrait};
use pallet_xcavate_whitelist::{Role, RoleRemover};

type AccountIdOf<T> = <T as frame_system::Config>::AccountId;
pub type DepositOf<T> = <<T as Config>::NativeCurrency as fungible::Inspect<
    <T as frame_system::Config>::AccountId,
>>::Balance;

pub type LocalAssetIdOf<T> = <<T as Config>::LocalCurrency as fungibles::Inspect<
    <T as frame_system::Config>::AccountId,
>>::AssetId;

pub type NegativeImbalanceOf<T> =
    Credit<<T as frame_system::Config>::AccountId, <T as Config>::NativeCurrency>;

#[frame_support::pallet]
pub mod pallet {
    use super::*;
    use frame_system::pallet_prelude::*;

    #[pallet::pallet]
    pub struct Pallet<T>(_);

    #[pallet::composite_enum]
    pub enum HoldReason {
        #[codec(index = 0)]
        BookingReserve,
        #[codec(index = 1)]
        ModuleReserve,
        #[codec(index = 2)]
        ModuleDelivererReserve,
    }

    /// Details of a module.
    #[cfg_attr(feature = "std", derive(serde::Serialize, serde::Deserialize))]
    #[derive(Encode, Decode, Clone, PartialEq, Eq, MaxEncodedLen, RuntimeDebug, TypeInfo)]
    #[scale_info(skip_type_params(T))]
    pub struct ModuleDetails<NftId, NftCollectionId, T: Config> {
        /// The content creator of this module.
        pub creator: AccountIdOf<T>,
        /// The NFT collection ID for the module.
        pub collection_id: NftCollectionId,
        /// The NFT item ID representing the learning module.
        pub item_id: NftId,
        /// The asset ID representing the learning module.
        pub asset_id: u32,
        /// The region ID where this module is valid.
        pub region: RegionId,
        /// Deposit held for the module.
        pub deposit: <T as pallet::Config>::Balance,
        /// The total amount of token that are available.
        pub total_token_amount: u32,
        /// Token available for sponsors to buy.
        pub sponsor_allocation: u32,
        /// Token available for schools to book.
        pub school_allocation: u32,
        /// Token available for a university student to claim.
        pub university_student_allocation: u32,
        /// Price for the lecturer.
        pub price: <T as pallet::Config>::Balance,
        /// Percentage fee for the content creator.
        pub content_creator_percentage: Perbill,
        /// Percentage fee for the regional operator.
        pub regional_operator_percentage: Perbill,
        /// Percentage fee for the protocol.
        pub protocol_percentage: Perbill,
        /// Percentage fee for the dbs checks.
        pub dbs_percentage: Perbill,
        /// Total costs for the module including: lecturer costs, content creator costs,
        /// regional operator costs, protocol costs and dbs costs.
        pub total_module_price: <T as pallet::Config>::Balance,
        /// Block of creation.
        pub created_at: BlockNumberFor<T>,
    }

    /// Details of sponsored modules.
    #[cfg_attr(feature = "std", derive(serde::Serialize, serde::Deserialize))]
    #[derive(Encode, Decode, Clone, PartialEq, Eq, MaxEncodedLen, RuntimeDebug, TypeInfo)]
    #[scale_info(skip_type_params(T))]
    pub struct SponsoredModulesDetails<T: Config> {
        /// The account of the sponsor.
        pub sponsor: AccountIdOf<T>,
        /// The amount of sponsored modules.
        pub amount: u32,
        /// The asset used for payment.
        pub payment_asset: u32,
        /// Block of sponsoring.
        pub sponsored_at: BlockNumberFor<T>,
    }

    /// Details of a school booking.
    #[cfg_attr(feature = "std", derive(serde::Serialize, serde::Deserialize))]
    #[derive(Encode, Decode, Clone, PartialEq, Eq, MaxEncodedLen, RuntimeDebug, TypeInfo)]
    #[scale_info(skip_type_params(T))]
    pub struct BookingDetails<T: Config> {
        /// The sponsor id of the sponsored module.
        pub sponsor_id: SponsorId,
        /// The sponsor of this module.
        pub sponsor: AccountIdOf<T>,
        /// The school which booked this module.
        pub school: AccountIdOf<T>,
        /// The lecturer who is teaching the module.
        pub lecturer: Option<AccountIdOf<T>>,
        /// The asset used for payment.
        pub payment_asset: u32,
        /// Block of sponsoring.
        pub sponsored_at: BlockNumberFor<T>,
        /// The average score achieved by the students.
        pub score: Option<Permill>,
        /// Deposit held for the booking.
        pub deposit: <T as pallet::Config>::Balance,
        /// Block of booking.
        pub booked_at: BlockNumberFor<T>,
        /// Block of claiming.
        pub claimed_at: Option<BlockNumberFor<T>>,
        /// Further information.
        pub metadata: BoundedVec<u8, <T as pallet::Config>::StringLimit>,
    }

    /// Infos of a module deliverer.
    #[derive(Encode, Decode, Clone, PartialEq, Eq, MaxEncodedLen, RuntimeDebug, TypeInfo)]
    #[scale_info(skip_type_params(T))]
    pub struct ModuleDelivererInfo<Balance> {
        /// Collateral deposit locked by the module deliverer to claim modules.
        pub deposit: Balance,
        /// Number of currently active claims assigned to this module deliverer.
        pub active_claims: u32,
        /// Strikes against the module deliverer.
        pub active_strikes: u8,
        /// Successful deliveries by the module deliverer.
        pub successful_deliveries: u32,
    }

    /// Configure the pallet by specifying the parameters and types on which it depends.
    #[pallet::config]
    pub trait Config: frame_system::Config {
        /// Because this pallet emits events, it depends on the runtime's definition of an event.
        type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;
        /// Type representing the weight of this pallet.
        type WeightInfo: WeightInfo;
        /// The balance type for currency operations.
        type Balance: Balance + TypeInfo + From<u128>;
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

        /// The NFT pallet for managing collections and items.
        type Nfts: nonfungibles_v2::Inspect<
                Self::AccountId,
                ItemId = <Self as pallet::Config>::NftId,
                CollectionId = <Self as pallet::Config>::NftCollectionId,
            > + Transfer<Self::AccountId>
            + nonfungibles_v2::Mutate<Self::AccountId, ItemConfig>
            + nonfungibles_v2::Create<
                Self::AccountId,
                CollectionConfig<
                    Self::Balance,
                    BlockNumberFor<Self>,
                    <Self as pallet::Config>::NftCollectionId,
                >,
            >;

        /// Identifier for the NFT collection.
        type NftCollectionId: Member + Parameter + MaxEncodedLen + Copy + Display;

        /// The type used to identify an NFT within a collection.
        type NftId: Member + Parameter + MaxEncodedLen + Copy + Default + CheckedAdd + One + Display;

        /// The maximum amount of token of a module.
        #[pallet::constant]
        type MaxModuleToken: Get<u32>;

        /// The maximum length of a name or symbol stored on-chain.
        #[pallet::constant]
        type StringLimit: Get<u32>;

        type LocalCurrency: fungibles::Inspect<
                AccountIdOf<Self>,
                AssetId = u32,
                Balance = <Self as pallet::Config>::Balance,
            > + fungibles::Create<AccountIdOf<Self>>
            + fungibles::Destroy<AccountIdOf<Self>>
            + fungibles::Mutate<AccountIdOf<Self>>
            + MetadataMutate<AccountIdOf<Self>>
            + MetadataDeposit<DepositOf<Self>>;

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

        /// Price for a learning module.
        #[pallet::constant]
        type ModulePrice: Get<<Self as pallet::Config>::Balance>;

        /// Provider for the block number. Normally this is the `frame_system` pallet.
        type BlockNumberProvider: BlockNumberProvider<BlockNumber = BlockNumberFor<Self>>;

        /// The fee percentage charged by the content creator (e.g., 1% as Perbill).
        #[pallet::constant]
        type ContentCreatorPercentage: Get<Perbill>;
        /// The fee percentage charged by the regional operator (e.g., 1% as Perbill).
        #[pallet::constant]
        type RegionalOperatorPercentage: Get<Perbill>;
        /// The fee percentage charged by the protocol (e.g., 1% as Perbill).
        #[pallet::constant]
        type ProtocolPercentage: Get<Perbill>;
        /// The fee percentage charged for DBS checks (e.g., 1% as Perbill).
        #[pallet::constant]
        type DBSPercentage: Get<Perbill>;

        /// Pallet ID for deriving the education's sovereign account.
        #[pallet::constant]
        type PalletId: Get<PalletId>;

        /// The Treasury's pallet ID, used for deriving its sovereign account ID.
        #[pallet::constant]
        type TreasuryId: Get<PalletId>;

        /// Origin type used to verify that an account has a specific Role.
        type PermissionOrigin: EnsureOriginWithArg<
            Self::RuntimeOrigin,
            Role,
            Success = Self::AccountId,
        >;

        /// Accepted assets for payments (e.g., USDC, USDT).
        #[pallet::constant]
        type AcceptedAssets: Get<[u32; 3]>;

        /// Deposit required for booking a module.
        #[pallet::constant]
        type BookingDeposit: Get<<Self as pallet::Config>::Balance>;

        /// Deposit required for creating a module.
        #[pallet::constant]
        type ModuleDeposit: Get<<Self as pallet::Config>::Balance>;

        /// Provider for region information.
        type RegionProvider: RegionTrait<
            Info = RegionInfo<
                AccountIdOf<Self>,
                <Self as pallet::Config>::Balance,
                BlockNumberFor<Self>,
            >,
        >;

        /// The newly created asset's symbol.
        #[pallet::constant]
        type NewAssetSymbol: Get<BoundedVec<u8, Self::StringLimit>>;

        /// The newly created asset's name.
        #[pallet::constant]
        type NewAssetName: Get<BoundedVec<u8, Self::StringLimit>>;

        /// Handler for the unbalanced reduction when slashing a letting agent.
        type Slash: OnUnbalanced<NegativeImbalanceOf<Self>>;

        /// Maximum number of booking cancellations allowed before slashing.
        #[pallet::constant]
        type MaxCancellations: Get<u32>;

        /// Time window during which cancellations are counted for slashing.
        #[pallet::constant]
        type CancellationWindow: Get<BlockNumberFor<Self>>;

        /// Time window after which a sponsor can reclaim unused funds.
        #[pallet::constant]
        type SponsorshipWindow: Get<BlockNumberFor<Self>>;

        /// Base deposit required to register as a module deliverer.
        #[pallet::constant]
        type ModuleDelivererDeposit: Get<<Self as pallet::Config>::Balance>;

        /// Maximum number of active strikes before a lecturer is slashed.
        #[pallet::constant]
        type MaxAllowedStrikes: Get<u8>;

        /// Percentage of deposit slashed per strike (applied to base deposit).
        #[pallet::constant]
        type StrikeSlashPercentage: Get<Perbill>;

        /// Maximum number of old cancellations to clean up in one call.
        #[pallet::constant]
        type MaxCleanupPerCall: Get<u32>;

        /// Minimum impact score required to trigger payments.
        #[pallet::constant]
        type MinImpactScore: Get<Permill>;

        /// Number of successful deliveries needed to reduce one strike.
        #[pallet::constant]
        type SuccessfulDeliveriesForStrikeReduction: Get<u32>;

        /// Provider for region information.
        type RoleProvider: RoleRemover<AccountIdOf<Self>>;

        /// Asset metadata provider.
        type AssetMetadata: AssetMetadataProvider<AssetId = u32>;
    }

    pub type RegionId = u16;
    pub type ModuleId = u32;
    pub type SponsorId = u64;
    pub type BookingId = u64;
    pub type ModuleDetailsOf<T> =
        ModuleDetails<<T as pallet::Config>::NftId, <T as pallet::Config>::NftCollectionId, T>;

    /// Storage for the next Module ID.
    #[pallet::storage]
    pub(super) type NextModuleId<T: Config> = StorageValue<_, ModuleId, ValueQuery>;

    /// Storage for the next Sponsor ID.
    #[pallet::storage]
    pub type NextSponsorId<T: Config> = StorageValue<_, SponsorId, ValueQuery>;

    /// Storage for the next Booking ID.
    #[pallet::storage]
    pub type NextBookingId<T: Config> = StorageValue<_, BookingId, ValueQuery>;

    /// Stores the next available NFT ID for each collection.
    #[pallet::storage]
    pub type NextNftId<T: Config> = StorageMap<
        _,
        Blake2_128Concat,
        <T as pallet::Config>::NftCollectionId,
        <T as pallet::Config>::NftId,
        ValueQuery,
    >;

    /// Id of the possible next asset that would be used for
    /// module fractionalization.
    #[pallet::storage]
    pub type NextAssetId<T: Config> = StorageValue<_, u32, ValueQuery>;

    /// Mapping of the module id to the module details.
    #[pallet::storage]
    pub type ModuleInfo<T: Config> =
        StorageMap<_, Blake2_128Concat, ModuleId, ModuleDetailsOf<T>, OptionQuery>;

    /// Mapping of the module id and the sponsor to the amount of sponsored modules.
    #[pallet::storage]
    pub type SponsoredModules<T: Config> = StorageDoubleMap<
        _,
        Blake2_128Concat,
        ModuleId,
        Blake2_128Concat,
        SponsorId,
        SponsoredModulesDetails<T>,
        OptionQuery,
    >;

    /// Mapping of the module id and the sponsor to the Booking details.
    #[pallet::storage]
    pub type Bookings<T: Config> = StorageDoubleMap<
        _,
        Blake2_128Concat,
        ModuleId,
        Blake2_128Concat,
        BookingId,
        BookingDetails<T>,
        OptionQuery,
    >;

    /// Records all booking cancellations made by a school.
    #[pallet::storage]
    pub type SchoolCancellations<T: Config> = StorageDoubleMap<
        _,
        Blake2_128Concat,
        AccountIdOf<T>,
        Blake2_128Concat,
        (BlockNumberFor<T>, BookingId),
        ModuleId,
        OptionQuery,
    >;

    /// Tracks the total number of booking cancellations made by each school.
    #[pallet::storage]
    pub type BookingCancellationCounter<T: Config> =
        StorageMap<_, Blake2_128Concat, AccountIdOf<T>, u32, ValueQuery>;

    /// Registered module deliverer and their details.
    #[pallet::storage]
    pub type ModuleDeliverer<T: Config> = StorageMap<
        _,
        Blake2_128Concat,
        T::AccountId,
        ModuleDelivererInfo<<T as pallet::Config>::Balance>,
        OptionQuery,
    >;

    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {
        LearningModuleCreated {
            creator: AccountIdOf<T>,
            module_id: ModuleId,
            collection_id: <T as pallet::Config>::NftCollectionId,
            item_id: <T as pallet::Config>::NftId,
            token_amount: u32,
            metadata_blob: BoundedVec<u8, <T as pallet::Config>::StringLimit>,
            created_at: BlockNumberFor<T>,
        },
        ModuleSponsored {
            module_id: ModuleId,
            sponsor_id: SponsorId,
            sponsor: AccountIdOf<T>,
            module_amount: u32,
            sponsored_at: BlockNumberFor<T>,
        },
        ModuleBooked {
            module_id: ModuleId,
            sponsor_id: SponsorId,
            booking_id: BookingId,
            sponsor: AccountIdOf<T>,
            school: AccountIdOf<T>,
            booked_at: BlockNumberFor<T>,
        },
        BookingClaimed {
            module_id: ModuleId,
            booking_id: BookingId,
            lecturer: AccountIdOf<T>,
            claimed_at: BlockNumberFor<T>,
        },
        TestResultsSubmitted {
            module_id: ModuleId,
            booking_id: BookingId,
            lecturer: AccountIdOf<T>,
            score: Permill,
            lecturer_pay: <T as pallet::Config>::Balance,
        },
        StudentNftMinted {
            module_id: ModuleId,
            booking_id: BookingId,
            student: AccountIdOf<T>,
        },
        AdminsStored {
            regional_operator: AccountIdOf<T>,
            dbs_admin: AccountIdOf<T>,
        },
        FinishBookingProcess {
            school: AccountIdOf<T>,
            module_id: ModuleId,
            booking_id: BookingId,
        },
        UnsponsoredTokensBurned {
            module_id: ModuleId,
            creator: AccountIdOf<T>,
            amount: u32,
            remaining_allocation: u32,
        },
        ModuleRemoved {
            module_id: ModuleId,
            creator: AccountIdOf<T>,
        },
        BookingCancelled {
            school: AccountIdOf<T>,
            module_id: ModuleId,
            booking_id: BookingId,
            cancellation_count: u32,
        },
        OldCancellationsCleared {
            school: AccountIdOf<T>,
            removed: u32,
        },
        UnsponsoredTokensWithdrawn {
            module_id: ModuleId,
            sponsor: AccountIdOf<T>,
            amount: u32,
            payment_asset: u32,
            refunded: <T as pallet::Config>::Balance,
        },
        ModuleDelivererRegistered {
            module_deliverer: AccountIdOf<T>,
            deposit: <T as pallet::Config>::Balance,
        },
        ModuleDelivererUnregistered {
            module_deliverer: AccountIdOf<T>,
        },
        ClaimCancelled {
            lecturer: AccountIdOf<T>,
            module_id: ModuleId,
            booking_id: BookingId,
            active_strikes: u8,
        },
        StrikeReduced {
            lecturer: AccountIdOf<T>,
            new_strikes: u8,
        },
        ModuleDelivererDepositIncreased {
            module_deliverer: AccountIdOf<T>,
            old_deposit: <T as pallet::Config>::Balance,
            new_deposit: <T as pallet::Config>::Balance,
        },
    }

    // Errors inform users that something went wrong.
    #[pallet::error]
    pub enum Error<T> {
        /// Token amount can not be zero.
        AmountCannotBeZero,
        /// The object can not be divided in so many token.
        TooManyToken,
        /// Arithmetic overflow occurred during calculation.
        ArithmeticOverflow,
        /// Underflow in arithmetic operations.
        ArithmeticUnderflow,
        /// This module does not exist.
        ModuleNotAvailable,
        /// Error by multiplying a number.
        MultiplyError,
        /// There are not enough token available for purchase.
        NotEnoughTokenAvailable,
        /// There are not sponsored module available.
        NoFundedModulesFromSponsor,
        /// This booking is not available.
        NoBookingAvailable,
        /// There is already a lecturer for this booking.
        LecturerAlreadySet,
        /// There is no lecturer set.
        NoLecturerSet,
        /// The sender doesn't have enough funds.
        NotEnoughFunds,
        /// Can't mint student nfts without test results.
        NoTestResultsSubmitted,
        /// No sufficient permission.
        NoPermission,
        /// The score is already set.
        ScoreAlreadySet,
        /// This Region is not known.
        RegionUnknown,
        /// This Asset is not supported for payment.
        PaymentAssetNotSupported,
        /// Sponsor already funded this module with a different payment asset.
        PaymentAssetMismatch,
        /// Module creator cannot remove module with active tokens.
        CannotRemoveModuleWithActiveTokens,
        /// Does not have enough balance to burn the requested amount.
        InsufficientBalance,
        /// Requested burn amount exceeds available allocation.
        CannotBurnMoreThanAvailable,
        /// The sponsoring has not yet expired.
        SponsorshipWindowNotExpired,
        /// The module deliverer has already been registered.
        ModuleDelivererAlreadyRegistered,
        /// The module deliverer has active claims.
        ModuleDelivererStillActive,
        /// The module deliverer is not registered.
        ModuleDelivererNotRegistered,
        /// The school cannot claim the booking.
        SchoolCannotClaimOwnBooking,
        /// Module deliverer does not have sufficient deposit to claim the booking.
        InsufficientDepositToClaim,
        /// Error by retrieving asset metadata.
        AssetMetadataNotAvailable,
        ArithmeticError,
    }

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        /// Creates a new learning module. A new nft gets minted and fractionalized.
        /// This function calls the nfts-pallet to mint a new nft and sets the Metadata.
        /// Then it calls the assets pallet to fractionalize the module into fungible tokens.
        ///
        /// The origin must be Signed by a ModuleCreator and have sufficient funds.
        ///
        /// Parameters:
        /// - `module_amount`: The amount of tokens to fractionalize the learning module into.
        /// - `data`: The Metadata of the learning module.
        ///
        /// Emits `LearningModuleCreated` event when successful.
        #[pallet::call_index(0)]
        #[pallet::weight(<T as pallet::Config>::WeightInfo::create_module())]
        pub fn create_module(
            origin: OriginFor<T>,
            region: RegionId,
            module_amount: u32,
            data: BoundedVec<u8, <T as pallet::Config>::StringLimit>,
        ) -> DispatchResult {
            let signer = T::PermissionOrigin::ensure_origin(origin.clone(), &Role::ModuleCreator)?;
            ensure!(module_amount > 0, Error::<T>::AmountCannotBeZero);
            ensure!(
                module_amount <= <T as pallet::Config>::MaxModuleToken::get(),
                Error::<T>::TooManyToken
            );
            ensure!(T::RegionProvider::is_region(region), Error::<T>::RegionUnknown);

            let module_id = NextModuleId::<T>::get();
            let education_account = Self::account_id();

            let deposit_amount = T::ModuleDeposit::get();

            // Hold the module deposit
            T::NativeCurrency::hold(&HoldReason::ModuleReserve.into(), &signer, deposit_amount)?;

            // Create a new NFT collection for the learning module
            let collection_id = <T as pallet::Config>::Nfts::create_collection(
                &education_account,
                &education_account,
                &Self::default_collection_config(),
            )?;

            // Get next available NFT ID
            let item_id = NextNftId::<T>::get(collection_id);

            // Mint the NFT representing the learning module
            Self::mint_nft_with_metadata(
                &collection_id,
                &item_id,
                &signer,
                &data,
                &education_account,
            )?;

            // Find next available asset ID
            let mut asset_number: u32 = NextAssetId::<T>::get();
            let mut asset_id: LocalAssetIdOf<T> = asset_number;
            while T::LocalCurrency::asset_exists(asset_id) {
                asset_number = asset_number.checked_add(1).ok_or(Error::<T>::ArithmeticOverflow)?;
                asset_id = asset_number;
            }

            Self::do_create_asset(asset_number, education_account.clone())?;
            Self::do_mint_asset(asset_number, &signer, module_amount.into())?;
            Self::do_set_metadata(
                asset_number,
                &signer,
                &education_account,
                &collection_id,
                &item_id,
            )?;

            // Calculate pricing details (all stored at 100× scale to maintain integer precision).
            let price = T::ModulePrice::get().saturating_mul(100u128.into());
            let content_creator_percentage = T::ContentCreatorPercentage::get();
            let regional_operator_percentage = T::RegionalOperatorPercentage::get();
            let protocol_percentage = T::ProtocolPercentage::get();
            let dbs_percentage = T::DBSPercentage::get();

            let content_creator_part = content_creator_percentage.mul_ceil(price);
            let regional_operator_part = regional_operator_percentage.mul_ceil(price);
            let protocol_part = protocol_percentage.mul_ceil(price);
            let dbs_part = dbs_percentage.mul_ceil(price);
            let total_single_module_price = price
                .checked_add(&content_creator_part)
                .ok_or(Error::<T>::ArithmeticOverflow)?
                .checked_add(&regional_operator_part)
                .ok_or(Error::<T>::ArithmeticOverflow)?
                .checked_add(&protocol_part)
                .ok_or(Error::<T>::ArithmeticOverflow)?
                .checked_add(&dbs_part)
                .ok_or(Error::<T>::ArithmeticOverflow)?;

            let current_block_number =
                <T as pallet::Config>::BlockNumberProvider::current_block_number();

            // Store module details
            let learning_module = ModuleDetails {
                creator: signer.clone(),
                collection_id,
                item_id,
                asset_id: asset_number,
                region,
                deposit: deposit_amount,
                total_token_amount: module_amount,
                sponsor_allocation: module_amount,
                school_allocation: Zero::zero(),
                university_student_allocation: Zero::zero(),
                price,
                content_creator_percentage,
                regional_operator_percentage,
                protocol_percentage,
                dbs_percentage,
                total_module_price: total_single_module_price,
                created_at: current_block_number,
            };

            ModuleInfo::<T>::insert(module_id, learning_module);

            // Update storage for next IDs
            let next_item_id =
                item_id.checked_add(&One::one()).ok_or(Error::<T>::ArithmeticOverflow)?;
            let next_asset_number =
                asset_number.checked_add(1).ok_or(Error::<T>::ArithmeticOverflow)?;
            let next_module_id = module_id.checked_add(1).ok_or(Error::<T>::ArithmeticOverflow)?;

            NextNftId::<T>::insert(collection_id, next_item_id);
            NextAssetId::<T>::put(next_asset_number);
            NextModuleId::<T>::put(next_module_id);

            Self::deposit_event(Event::<T>::LearningModuleCreated {
                creator: signer,
                module_id,
                collection_id,
                item_id,
                token_amount: module_amount,
                metadata_blob: data,
                created_at: current_block_number,
            });
            Ok(())
        }

        /// Lets a sponsor sponsor learning modules.
        ///
        /// The origin must be Signed by a Sponsor and have sufficient funds.
        ///
        /// Parameters:
        /// - `module_id`: The ID of the learning module to sponsor.
        /// - `token_amount`: The amount of tokens to sponsor for the learning module.
        ///
        /// Emits `ModuleSponsored` event when successful.
        #[pallet::call_index(1)]
        #[pallet::weight(<T as pallet::Config>::WeightInfo::sponsor_module())]
        pub fn sponsor_module(
            origin: OriginFor<T>,
            module_id: ModuleId,
            token_amount: u32,
            payment_asset: u32,
        ) -> DispatchResult {
            let signer = T::PermissionOrigin::ensure_origin(origin, &Role::ModuleSponsor)?;

            ensure!(token_amount > 0, Error::<T>::AmountCannotBeZero);
            ensure!(
                T::AcceptedAssets::get().contains(&payment_asset),
                Error::<T>::PaymentAssetNotSupported
            );

            let mut module_details =
                ModuleInfo::<T>::get(module_id).ok_or(Error::<T>::ModuleNotAvailable)?;
            ensure!(
                token_amount <= module_details.sponsor_allocation,
                Error::<T>::NotEnoughTokenAvailable
            );

            let current_block_number =
                <T as pallet::Config>::BlockNumberProvider::current_block_number();

            let multiplier = Self::asset_decimal_multiplier(payment_asset)?;
            let price_per_token =
                Self::adjust_price_by_multiplier(module_details.total_module_price, multiplier)?;

            let total_price = price_per_token
                .checked_mul(&(token_amount as u128).into())
                .ok_or(Error::<T>::MultiplyError)?;

            // Hold funds for the purchase
            T::ForeignAssetsHolder::hold(
                payment_asset,
                &MarketplaceHoldReason::ModulePurchase,
                &signer,
                total_price,
            )?;

            // Transfer the module token from the content creator to the sponsor.
            <T as pallet::Config>::LocalCurrency::transfer(
                module_details.asset_id,
                &module_details.creator,
                &signer,
                token_amount.into(),
                Preservation::Expendable,
            )?;

            let sponsor_id = NextSponsorId::<T>::get();

            let sponsored_details = SponsoredModulesDetails {
                sponsor: signer.clone(),
                amount: token_amount,
                payment_asset,
                sponsored_at: current_block_number,
            };

            // Update storage values
            module_details.sponsor_allocation = module_details
                .sponsor_allocation
                .checked_sub(token_amount)
                .ok_or(Error::<T>::ArithmeticUnderflow)?;
            module_details.school_allocation = module_details
                .school_allocation
                .checked_add(token_amount)
                .ok_or(Error::<T>::ArithmeticOverflow)?;
            let next_sponsor_id =
                sponsor_id.checked_add(1).ok_or(Error::<T>::ArithmeticOverflow)?;

            SponsoredModules::<T>::insert(module_id, sponsor_id, &sponsored_details);
            ModuleInfo::<T>::insert(module_id, module_details);
            NextSponsorId::<T>::put(next_sponsor_id);

            Self::deposit_event(Event::<T>::ModuleSponsored {
                module_id,
                sponsor_id,
                sponsor: signer,
                module_amount: token_amount,
                sponsored_at: current_block_number,
            });
            Ok(())
        }

        /// Lets a school book a sponsored learning module.
        ///
        /// The origin must be Signed by a ModuleBooker and have sufficient funds.
        ///
        /// Parameters:
        /// - `module_id`: The ID of the learning module to book.
        /// - `sponsor`:  The account of the sponsor who funded the module.
        /// - `data`: Further information about the booking.
        ///
        /// Emits `ModuleBooked` event when successful.
        #[pallet::call_index(2)]
        #[pallet::weight(<T as pallet::Config>::WeightInfo::book_module())]
        pub fn book_module(
            origin: OriginFor<T>,
            module_id: ModuleId,
            sponsor_id: SponsorId,
            data: BoundedVec<u8, <T as pallet::Config>::StringLimit>,
        ) -> DispatchResult {
            let signer = T::PermissionOrigin::ensure_origin(origin, &Role::ModuleBooker)?;

            // Check module availability and sponsor funding
            let mut module_details =
                ModuleInfo::<T>::get(module_id).ok_or(Error::<T>::ModuleNotAvailable)?;
            ensure!(1 <= module_details.school_allocation, Error::<T>::NotEnoughTokenAvailable);
            let mut funded_by_sponsor = SponsoredModules::<T>::get(module_id, sponsor_id)
                .ok_or(Error::<T>::NoFundedModulesFromSponsor)?;
            ensure!(funded_by_sponsor.amount > 0, Error::<T>::NoFundedModulesFromSponsor);

            let booking_id = NextBookingId::<T>::get();
            let sponsor = funded_by_sponsor.sponsor.clone();

            // Decrease by 1 (one presentation booked)
            funded_by_sponsor.amount = funded_by_sponsor.amount.saturating_sub(1);
            module_details.school_allocation = module_details
                .school_allocation
                .checked_sub(1)
                .ok_or(Error::<T>::ArithmeticUnderflow)?;
            module_details.university_student_allocation = module_details
                .university_student_allocation
                .checked_add(1)
                .ok_or(Error::<T>::ArithmeticOverflow)?;

            let deposit_amount = T::BookingDeposit::get();

            // Hold the booking deposit
            T::NativeCurrency::hold(&HoldReason::BookingReserve.into(), &signer, deposit_amount)?;

            // Transfer the module token from the sponsor to the school.
            <T as pallet::Config>::LocalCurrency::transfer(
                module_details.asset_id,
                &sponsor,
                &signer,
                1u32.into(),
                Preservation::Expendable,
            )?;

            let current_block_number =
                <T as pallet::Config>::BlockNumberProvider::current_block_number();

            // Create booking details
            let booking_details = BookingDetails {
                sponsor_id,
                sponsor: sponsor.clone(),
                school: signer.clone(),
                lecturer: None,
                payment_asset: funded_by_sponsor.payment_asset,
                sponsored_at: funded_by_sponsor.sponsored_at,
                score: None,
                deposit: deposit_amount,
                booked_at: current_block_number,
                claimed_at: None,
                metadata: data,
            };

            // If zero → remove from storage entirely
            // If >0 → update
            if funded_by_sponsor.amount.is_zero() {
                SponsoredModules::<T>::remove(module_id, sponsor_id);
            } else {
                SponsoredModules::<T>::insert(module_id, sponsor_id, funded_by_sponsor);
            }

            let next_booking_id =
                booking_id.checked_add(1).ok_or(Error::<T>::ArithmeticOverflow)?;

            // Update storage values
            NextBookingId::<T>::put(next_booking_id);
            ModuleInfo::<T>::insert(module_id, module_details);
            Bookings::<T>::insert(module_id, booking_id, booking_details);

            Self::deposit_event(Event::<T>::ModuleBooked {
                module_id,
                sponsor_id,
                booking_id,
                sponsor,
                school: signer,
                booked_at: current_block_number,
            });
            Ok(())
        }

        /// Lets a university student claim a booked learning module.
        ///
        /// The origin must be Signed by a ModuleDeliverer and have sufficient funds.
        ///
        /// Parameters:
        /// - `module_id`: The ID of the learning module to claim.
        /// - `booking_id`: The ID of the booking to claim.
        ///
        /// Emits `BookingClaimed` event when successful.
        #[pallet::call_index(3)]
        #[pallet::weight(<T as pallet::Config>::WeightInfo::claim_booking())]
        pub fn claim_booking(
            origin: OriginFor<T>,
            module_id: ModuleId,
            booking_id: BookingId,
        ) -> DispatchResult {
            let signer = T::PermissionOrigin::ensure_origin(origin, &Role::ModuleDeliverer)?;

            let mut booking_details =
                Bookings::<T>::get(module_id, booking_id).ok_or(Error::<T>::NoBookingAvailable)?;
            ensure!(booking_details.lecturer.is_none(), Error::<T>::LecturerAlreadySet);
            ensure!(booking_details.school != signer, Error::<T>::SchoolCannotClaimOwnBooking);

            let mut module_deliverer_info = ModuleDeliverer::<T>::get(&signer)
                .ok_or(Error::<T>::ModuleDelivererNotRegistered)?;

            let slash_per_strike =
                T::StrikeSlashPercentage::get().mul_ceil(T::ModuleDelivererDeposit::get());
            let concurrent_claims = module_deliverer_info
                .active_claims
                .checked_add(1)
                .ok_or(Error::<T>::ArithmeticOverflow)?;

            let required_deposit = slash_per_strike
                .checked_mul(&concurrent_claims.into())
                .ok_or(Error::<T>::MultiplyError)?;

            ensure!(
                module_deliverer_info.deposit >= required_deposit,
                Error::<T>::InsufficientDepositToClaim
            );

            let mut module_details =
                ModuleInfo::<T>::get(module_id).ok_or(Error::<T>::ModuleNotAvailable)?;

            ensure!(
                1 <= module_details.university_student_allocation,
                Error::<T>::NotEnoughTokenAvailable
            );

            let current_block_number =
                <T as pallet::Config>::BlockNumberProvider::current_block_number();

            module_details.university_student_allocation = module_details
                .university_student_allocation
                .checked_sub(1)
                .ok_or(Error::<T>::ArithmeticUnderflow)?;

            // Update booking details
            booking_details.lecturer = Some(signer.clone());
            booking_details.claimed_at = Some(current_block_number);

            // Update module deliverer claims
            module_deliverer_info.active_claims = concurrent_claims;

            ModuleDeliverer::<T>::insert(&signer, module_deliverer_info);
            ModuleInfo::<T>::insert(module_id, module_details);
            Bookings::<T>::insert(module_id, booking_id, booking_details);

            Self::deposit_event(Event::<T>::BookingClaimed {
                module_id,
                booking_id,
                lecturer: signer,
                claimed_at: current_block_number,
            });
            Ok(())
        }

        /// Lets an AI agent submit test results for a claimed learning module.
        /// This function calls the nfts-pallet to mint new nfts and sets the Metadata.
        ///
        /// The origin must be Signed by a ModuleAIAgent and have sufficient funds.
        ///
        /// Parameters:
        /// - `module_id`: The ID of the learning module.
        /// - `booking_id`: The ID of the booking to submit results for.
        /// - `score`: The average score achieved by the high school students (as Permill).
        /// - `module_sponsor_data`: Metadata for the sponsor's NFT.
        /// - `module_booker_data`: Metadata for the school's NFT.
        ///
        /// Emits `TestResultsSubmitted` event when successful.
        #[pallet::call_index(4)]
        #[pallet::weight(<T as pallet::Config>::WeightInfo::submit_impact_score())]
        pub fn submit_impact_score(
            origin: OriginFor<T>,
            module_id: ModuleId,
            booking_id: BookingId,
            score: Permill,
            module_sponsor_data: BoundedVec<u8, <T as pallet::Config>::StringLimit>,
            module_booker_data: BoundedVec<u8, <T as pallet::Config>::StringLimit>,
            module_deliverer_data: BoundedVec<u8, <T as pallet::Config>::StringLimit>,
        ) -> DispatchResult {
            let _ = T::PermissionOrigin::ensure_origin(origin, &Role::ModuleAIAgent)?;

            let mut booking_details =
                Bookings::<T>::get(module_id, booking_id).ok_or(Error::<T>::NoBookingAvailable)?;
            let lecturer = booking_details.lecturer.clone().ok_or(Error::<T>::NoLecturerSet)?;
            ensure!(booking_details.score.is_none(), Error::<T>::ScoreAlreadySet);

            let module_details =
                ModuleInfo::<T>::get(module_id).ok_or(Error::<T>::ModuleNotAvailable)?;
            let region_info =
                <T as pallet::Config>::RegionProvider::get_region_details(module_details.region)
                    .ok_or(Error::<T>::RegionUnknown)?;
            let mut module_deliverer_info = ModuleDeliverer::<T>::get(&lecturer)
                .ok_or(Error::<T>::ModuleDelivererNotRegistered)?;
            let regional_operator = region_info.owner;

            let treasury_id = Self::treasury_account_id();
            let payment_asset = booking_details.payment_asset;

            // Burn the module token
            <T as pallet::Config>::LocalCurrency::burn_from(
                module_details.asset_id,
                &booking_details.school,
                1u32.into(),
                Preservation::Expendable,
                Precision::Exact,
                Fortitude::Force,
            )?;

            let multiplier = Self::asset_decimal_multiplier(payment_asset)?;
            let total_module_price =
                Self::adjust_price_by_multiplier(module_details.total_module_price, multiplier)?;

            // Release the locked funds from the sponsor
            T::ForeignAssetsHolder::release(
                payment_asset,
                &MarketplaceHoldReason::ModulePurchase,
                &booking_details.sponsor,
                total_module_price,
                Precision::Exact,
            )?;

            // Calculate payments
            let (content_creator_pay, regional_operator_pay, protocol_pay, lecturer_pay_part) =
                // Success path — everyone gets paid based on score
                if score >= T::MinImpactScore::get() {
                    let module_price =
                        Self::adjust_price_by_multiplier(module_details.price, multiplier)?;

                    // Use floor to make sure we don't overcharge the sponsor.
                    let content_creator_pay = score.mul_floor(
                        module_details.content_creator_percentage.mul_ceil(module_price),
                    );
                    let regional_operator_pay = score.mul_floor(
                        module_details.regional_operator_percentage.mul_ceil(module_price),
                    );
                    let protocol_pay =
                        score.mul_floor(module_details.protocol_percentage.mul_ceil(module_price));
                    let dbs_pay =
                        score.mul_floor(module_details.dbs_percentage.mul_ceil(module_price));

                    // Use floor to make sure we don't overcharge the sponsor.
                    let mut lecturer_pay = score.mul_floor(module_price);
                    lecturer_pay = lecturer_pay.checked_add(&dbs_pay).ok_or(Error::<T>::ArithmeticOverflow)?;

                    // Make sure we don't overcharge the sponsor.
                    let total_pay = lecturer_pay
                        .saturating_add(content_creator_pay)
                        .saturating_add(regional_operator_pay)
                        .saturating_add(protocol_pay);
                    let lecturer_pay_part = if total_pay <= total_module_price {
                        lecturer_pay
                    } else {
                        total_module_price
                            .checked_sub(&content_creator_pay)
                            .ok_or(Error::<T>::ArithmeticUnderflow)?
                            .checked_sub(&regional_operator_pay)
                            .ok_or(Error::<T>::ArithmeticUnderflow)?
                            .checked_sub(&protocol_pay)
                            .ok_or(Error::<T>::ArithmeticUnderflow)?
                    };

                    module_deliverer_info.successful_deliveries = module_deliverer_info.successful_deliveries
                        .checked_add(1).ok_or(Error::<T>::ArithmeticOverflow)?;
                    
                    // Reduce strikes every X successful deliveries
                    if module_deliverer_info.successful_deliveries % T::SuccessfulDeliveriesForStrikeReduction::get() == 0 {
                        module_deliverer_info.active_strikes = module_deliverer_info.active_strikes.saturating_sub(1);
                        Self::deposit_event(Event::StrikeReduced {
                            lecturer: lecturer.clone(),
                            new_strikes: module_deliverer_info.active_strikes,
                        });
                    }

                    (content_creator_pay, regional_operator_pay, protocol_pay, lecturer_pay_part)
                } else {
                    // Score < 50% → NO PAYMENTS
                    (0u32.into(), 0u32.into(), 0u32.into(), 0u32.into())
                };

            // Transfer the funds to the different parties
            Self::transfer_funds(
                &booking_details.sponsor,
                &module_details.creator,
                content_creator_pay,
                payment_asset,
            )?;
            Self::transfer_funds(
                &booking_details.sponsor,
                &treasury_id,
                protocol_pay,
                payment_asset,
            )?;
            Self::transfer_funds(
                &booking_details.sponsor,
                &regional_operator,
                regional_operator_pay,
                payment_asset,
            )?;
            Self::transfer_funds(
                &booking_details.sponsor,
                &lecturer,
                lecturer_pay_part,
                payment_asset,
            )?;

            // Mint NFTs for sponsor, school, and lecturer
            let education_account = Self::account_id();
            let sponsor_item_id = NextNftId::<T>::get(module_details.collection_id);
            Self::mint_nft_with_metadata(
                &module_details.collection_id,
                &sponsor_item_id,
                &booking_details.sponsor,
                &module_sponsor_data,
                &education_account,
            )?;

            let school_item_id =
                sponsor_item_id.checked_add(&One::one()).ok_or(Error::<T>::ArithmeticOverflow)?;
            Self::mint_nft_with_metadata(
                &module_details.collection_id,
                &school_item_id,
                &booking_details.school,
                &module_booker_data,
                &education_account,
            )?;

            let lecturer_item_id =
                school_item_id.checked_add(&One::one()).ok_or(Error::<T>::ArithmeticOverflow)?;
            Self::mint_nft_with_metadata(
                &module_details.collection_id,
                &lecturer_item_id,
                &lecturer,
                &module_deliverer_data,
                &education_account,
            )?;

            // Update storage.
            booking_details.score = Some(score);
            let next_item_id =
                lecturer_item_id.checked_add(&One::one()).ok_or(Error::<T>::ArithmeticOverflow)?;
            module_deliverer_info.active_claims = module_deliverer_info
                .active_claims
                .checked_sub(1)
                .ok_or(Error::<T>::ArithmeticUnderflow)?;
            ModuleDeliverer::<T>::insert(&lecturer, module_deliverer_info);
            Bookings::<T>::insert(module_id, booking_id, booking_details);
            NextNftId::<T>::insert(module_details.collection_id, next_item_id);

            Self::deposit_event(Event::TestResultsSubmitted {
                module_id,
                booking_id,
                lecturer,
                score,
                lecturer_pay: lecturer_pay_part,
            });
            Ok(())
        }

        /// Lets a school teacher mint the nft for a student.
        /// This function calls the nfts-pallet to mint a new nft and sets the Metadata.
        ///
        /// The origin must be Signed by a ModuleAIAgent and have sufficient funds.
        ///
        /// Parameters:
        /// - `module_id`: The ID of the learning module.
        /// - `booking_id`: The ID of the booking to mint the nft for.
        /// - `student`: The student the nft is minted for.
        /// - `module_recipient_data`: Metadata for the students's NFT.
        ///
        /// Emits `StudentNftMinted` event when successful.
        #[pallet::call_index(5)]
        #[pallet::weight(<T as pallet::Config>::WeightInfo::mint_recipient_nft())]
        pub fn mint_recipient_nft(
            origin: OriginFor<T>,
            module_id: ModuleId,
            booking_id: BookingId,
            student: AccountIdOf<T>,
            module_recipient_data: BoundedVec<u8, <T as pallet::Config>::StringLimit>,
        ) -> DispatchResult {
            let _ = T::PermissionOrigin::ensure_origin(origin, &Role::ModuleAIAgent)?;

            // Check booking details
            let booking_details =
                Bookings::<T>::get(module_id, booking_id).ok_or(Error::<T>::NoBookingAvailable)?;
            ensure!(booking_details.score.is_some(), Error::<T>::NoTestResultsSubmitted);

            let module_details =
                ModuleInfo::<T>::get(module_id).ok_or(Error::<T>::ModuleNotAvailable)?;

            // Get next available NFT ID and mint student NFT
            let education_account = Self::account_id();
            let item_id = NextNftId::<T>::get(module_details.collection_id);
            Self::mint_nft_with_metadata(
                &module_details.collection_id,
                &item_id,
                &student,
                &module_recipient_data,
                &education_account,
            )?;

            let next_item_id =
                item_id.checked_add(&One::one()).ok_or(Error::<T>::ArithmeticOverflow)?;

            NextNftId::<T>::insert(module_details.collection_id, next_item_id);

            Self::deposit_event(Event::<T>::StudentNftMinted { module_id, booking_id, student });
            Ok(())
        }

        /// Lets a school teacher release the locked deposit and free the storage.
        ///
        /// The origin must be Signed by a ModuleBooker and have sufficient funds.
        ///
        /// Parameters:
        /// - `module_id`: The ID of the learning module.
        /// - `booking_id`: The ID of the booking to release the locked token.
        ///
        /// Emits `FinishBookingProcess` event when successful.
        #[pallet::call_index(6)]
        #[pallet::weight(<T as pallet::Config>::WeightInfo::finish_booking_process())]
        pub fn finish_booking_process(
            origin: OriginFor<T>,
            module_id: ModuleId,
            booking_id: BookingId,
        ) -> DispatchResult {
            let signer = T::PermissionOrigin::ensure_origin(origin, &Role::ModuleBooker)?;

            // Check booking details
            let booking_details =
                Bookings::<T>::get(module_id, booking_id).ok_or(Error::<T>::NoBookingAvailable)?;
            ensure!(booking_details.school == signer, Error::<T>::NoPermission);
            ensure!(booking_details.score.is_some(), Error::<T>::NoTestResultsSubmitted);

            // Release the booking deposit back to the school.
            let deposit_amount = booking_details.deposit;
            <T as pallet::Config>::NativeCurrency::release(
                &HoldReason::BookingReserve.into(),
                &signer,
                deposit_amount,
                Precision::Exact,
            )?;

            Bookings::<T>::remove(module_id, booking_id);

            Self::deposit_event(Event::<T>::FinishBookingProcess {
                school: signer,
                module_id,
                booking_id,
            });
            Ok(())
        }

        /// Allows a module creator to burn unsponsored module tokens.
        ///
        /// The origin must be Signed by a ModuleCreator and have sufficient funds.
        ///
        /// Parameters:
        /// - `module_id`: The ID of the learning module.
        /// - `amount`: The number of unsponsored tokens to burn.
        ///
        /// Emits `UnsponsoredTokensBurned` event when successful.
        #[pallet::call_index(7)]
        #[pallet::weight(<T as pallet::Config>::WeightInfo::burn_unsponsored_token())]
        pub fn burn_unsponsored_token(
            origin: OriginFor<T>,
            module_id: ModuleId,
            amount: u32,
        ) -> DispatchResult {
            let signer = T::PermissionOrigin::ensure_origin(origin, &Role::ModuleCreator)?;

            // Load module and validate ownership
            let mut module_details =
                ModuleInfo::<T>::get(module_id).ok_or(Error::<T>::ModuleNotAvailable)?;
            ensure!(module_details.creator == signer, Error::<T>::NoPermission);

            ensure!(amount > 0, Error::<T>::AmountCannotBeZero);

            let creator_balance = T::LocalCurrency::balance(module_details.asset_id, &signer);

            // Ensure requested amount doesn't exceed actual balance or available allocation
            ensure!(creator_balance >= amount.into(), Error::<T>::InsufficientBalance);
            ensure!(
                amount <= module_details.sponsor_allocation,
                Error::<T>::CannotBurnMoreThanAvailable
            );

            // Burn the module token
            <T as pallet::Config>::LocalCurrency::burn_from(
                module_details.asset_id,
                &signer,
                amount.into(),
                Preservation::Expendable,
                Precision::Exact,
                Fortitude::Force,
            )?;

            // Update stored allocation
            module_details.sponsor_allocation = module_details
                .sponsor_allocation
                .checked_sub(amount)
                .ok_or(Error::<T>::ArithmeticUnderflow)?;
            let remaining_allocation = module_details.sponsor_allocation;
            ModuleInfo::<T>::insert(module_id, module_details);

            Self::deposit_event(Event::UnsponsoredTokensBurned {
                module_id,
                creator: signer,
                amount,
                remaining_allocation,
            });
            Ok(())
        }

        /// Allows a module creator to permanently remove a completed module from storage.
        ///
        /// The origin must be Signed by a ModuleCreator and have sufficient funds.
        ///
        /// Parameters:
        /// - `module_id`: The ID of the learning module to remove.
        ///
        /// Emits `ModuleRemoved` event when successful.
        #[pallet::call_index(8)]
        #[pallet::weight(<T as pallet::Config>::WeightInfo::remove_module())]
        pub fn remove_module(origin: OriginFor<T>, module_id: ModuleId) -> DispatchResult {
            let signer = T::PermissionOrigin::ensure_origin(origin, &Role::ModuleCreator)?;

            // Load module and validate ownership
            let module_details =
                ModuleInfo::<T>::get(module_id).ok_or(Error::<T>::ModuleNotAvailable)?;
            ensure!(module_details.creator == signer, Error::<T>::NoPermission);

            // Ensure no active tokens remain before allowing removal
            ensure!(
                T::LocalCurrency::total_issuance(module_details.asset_id).is_zero(),
                Error::<T>::CannotRemoveModuleWithActiveTokens
            );

            // Destroy the asset
            T::LocalCurrency::start_destroy(module_details.asset_id, None)?;
            T::LocalCurrency::finish_destroy(module_details.asset_id)?;

            // Release the module deposit back to the creator.
            <T as pallet::Config>::NativeCurrency::release(
                &HoldReason::ModuleReserve.into(),
                &signer,
                module_details.deposit,
                Precision::Exact,
            )?;

            // Clean up module storage
            ModuleInfo::<T>::remove(module_id);

            Self::deposit_event(Event::ModuleRemoved { module_id, creator: signer });
            Ok(())
        }

        /// Allows a sponsor to reclaim funds and return unused tokens for a specific sponsorship batch.
        ///
        /// The origin must be Signed by a ModuleSponsor and have sufficient funds.
        ///
        /// Parameters:
        /// - `module_id`: The ID of the learning module to remove.
        /// - `sponsor_id`: The ID of this sponsorship batch.
        /// - `amount`: The number of tokens to reclaim.
        ///
        /// Emits `UnsponsoredTokensWithdrawn` event when successful.
        #[pallet::call_index(9)]
        #[pallet::weight(<T as pallet::Config>::WeightInfo::reclaim_unused_sponsorship())]
        pub fn reclaim_unused_sponsorship(
            origin: OriginFor<T>,
            module_id: ModuleId,
            sponsor_id: SponsorId,
            amount: u32,
        ) -> DispatchResult {
            let signer = T::PermissionOrigin::ensure_origin(origin, &Role::ModuleSponsor)?;

            // Check module details and sponsor funding
            let mut module_details =
                ModuleInfo::<T>::get(module_id).ok_or(Error::<T>::ModuleNotAvailable)?;
            let mut funded_by_sponsor = SponsoredModules::<T>::get(module_id, sponsor_id)
                .ok_or(Error::<T>::NoFundedModulesFromSponsor)?;

            // Validate ownership and amount
            ensure!(funded_by_sponsor.sponsor == signer, Error::<T>::NoPermission);
            ensure!(!amount.is_zero(), Error::<T>::AmountCannotBeZero);
            ensure!(funded_by_sponsor.amount >= amount, Error::<T>::NotEnoughTokenAvailable);

            // Check sponsorship window
            let current_block_number =
                <T as pallet::Config>::BlockNumberProvider::current_block_number();
            let deadline =
                funded_by_sponsor.sponsored_at.saturating_add(T::SponsorshipWindow::get());
            ensure!(current_block_number > deadline, Error::<T>::SponsorshipWindowNotExpired);

            let payment_asset = funded_by_sponsor.payment_asset;

            let multiplier = Self::asset_decimal_multiplier(payment_asset)?;
            let price_per_token =
                Self::adjust_price_by_multiplier(module_details.total_module_price, multiplier)?;
            let total_price = price_per_token
                .checked_mul(&(amount as u128).into())
                .ok_or(Error::<T>::MultiplyError)?;

            // Release the locked funds back to the sponsor
            T::ForeignAssetsHolder::release(
                payment_asset,
                &MarketplaceHoldReason::ModulePurchase,
                &signer,
                total_price,
                Precision::Exact,
            )?;

            // Transfer the module token back to the content creator.
            <T as pallet::Config>::LocalCurrency::transfer(
                module_details.asset_id,
                &signer,
                &module_details.creator,
                amount.into(),
                Preservation::Expendable,
            )?;

            module_details.school_allocation = module_details
                .school_allocation
                .checked_sub(amount)
                .ok_or(Error::<T>::ArithmeticUnderflow)?;
            module_details.sponsor_allocation = module_details
                .sponsor_allocation
                .checked_add(amount)
                .ok_or(Error::<T>::ArithmeticOverflow)?;

            funded_by_sponsor.amount = funded_by_sponsor
                .amount
                .checked_sub(amount)
                .ok_or(Error::<T>::ArithmeticUnderflow)?;

            // Update or remove sponsor storage.
            if funded_by_sponsor.amount.is_zero() {
                SponsoredModules::<T>::remove(module_id, sponsor_id);
            } else {
                SponsoredModules::<T>::insert(module_id, sponsor_id, funded_by_sponsor);
            }

            ModuleInfo::<T>::insert(module_id, module_details);

            Self::deposit_event(Event::UnsponsoredTokensWithdrawn {
                module_id,
                sponsor: signer,
                amount,
                payment_asset,
                refunded: total_price,
            });

            Ok(())
        }

        /// Allows a school to cancel a booked module
        ///
        /// The origin must be Signed by a ModuleBooker and have sufficient funds.
        ///
        /// Parameters:
        /// - `module_id`: The ID of the learning module.
        /// - `booking_id`: The ID of the booking to cancel.
        ///
        /// Emits `BookingCancelled` event when successful.
        #[pallet::call_index(10)]
        #[pallet::weight(<T as pallet::Config>::WeightInfo::cancel_booking())]
        pub fn cancel_booking(
            origin: OriginFor<T>,
            module_id: ModuleId,
            booking_id: BookingId,
        ) -> DispatchResult {
            let signer = T::PermissionOrigin::ensure_origin(origin, &Role::ModuleBooker)?;

            // Load & validate
            let mut module_details =
                ModuleInfo::<T>::get(module_id).ok_or(Error::<T>::ModuleNotAvailable)?;
            let booking =
                Bookings::<T>::get(module_id, booking_id).ok_or(Error::<T>::NoBookingAvailable)?;
            ensure!(booking.school == signer, Error::<T>::NoPermission);

            let current_block_number =
                <T as pallet::Config>::BlockNumberProvider::current_block_number();

            // Increment cancellation counter
            let mut booking_cancellation_count = BookingCancellationCounter::<T>::get(&signer);
            booking_cancellation_count =
                booking_cancellation_count.checked_add(1).ok_or(Error::<T>::ArithmeticOverflow)?;

            // Transfer the module token back to the sponsor.
            <T as pallet::Config>::LocalCurrency::transfer(
                module_details.asset_id,
                &signer,
                &booking.sponsor,
                1u32.into(),
                Preservation::Expendable,
            )?;

            // Handle deposit: slash if threshold reached, otherwise refund
            if booking_cancellation_count >= T::MaxCancellations::get() {
                let (imbalance, _remaining) = <T as pallet::Config>::NativeCurrency::slash(
                    &HoldReason::BookingReserve.into(),
                    &signer,
                    booking.deposit,
                );
                T::Slash::on_unbalanced(imbalance);
            } else {
                // Release the listing deposit back to school.
                <T as pallet::Config>::NativeCurrency::release(
                    &HoldReason::BookingReserve.into(),
                    &signer,
                    booking.deposit,
                    Precision::Exact,
                )?;
            }

            // Reconstruct or update sponsor record
            let mut funded_by_sponsor = SponsoredModules::<T>::get(module_id, booking.sponsor_id)
                .unwrap_or(SponsoredModulesDetails {
                    sponsor: booking.sponsor,
                    amount: 0,
                    payment_asset: booking.payment_asset,
                    sponsored_at: booking.sponsored_at,
                });

            funded_by_sponsor.amount = funded_by_sponsor.amount.saturating_add(1);
            module_details.school_allocation = module_details
                .school_allocation
                .checked_add(1)
                .ok_or(Error::<T>::ArithmeticOverflow)?;

            // Handle lecturer claim rollback (if claimed)
            if let Some(lecturer) = booking.lecturer {
                let mut module_deliverer_info = ModuleDeliverer::<T>::get(&lecturer)
                    .ok_or(Error::<T>::ModuleDelivererNotRegistered)?;
                module_deliverer_info.active_claims = module_deliverer_info
                    .active_claims
                    .checked_sub(1)
                    .ok_or(Error::<T>::ArithmeticUnderflow)?;
                ModuleDeliverer::<T>::insert(&lecturer, module_deliverer_info);
            } else {
                module_details.university_student_allocation = module_details
                    .university_student_allocation
                    .checked_sub(1)
                    .ok_or(Error::<T>::ArithmeticUnderflow)?;
            }

            // Update storage
            BookingCancellationCounter::<T>::insert(&signer, booking_cancellation_count);
            SchoolCancellations::<T>::insert(
                &signer,
                (current_block_number, booking_id),
                module_id,
            );
            ModuleInfo::<T>::insert(module_id, module_details);
            SponsoredModules::<T>::insert(module_id, booking.sponsor_id, funded_by_sponsor);
            Bookings::<T>::remove(module_id, booking_id);

            Self::deposit_event(Event::BookingCancelled {
                school: signer,
                module_id,
                booking_id,
                cancellation_count: booking_cancellation_count,
            });
            Ok(())
        }

        /// Allows a school to clean up old cancellation records outside the active window.
        ///
        /// The origin must be Signed by a ModuleBooker and have sufficient funds.
        ///
        /// Parameters: None
        ///
        /// Emits `OldCancellationsCleared` event when successful.
        #[pallet::call_index(11)]
        #[pallet::weight(<T as pallet::Config>::WeightInfo::clear_old_cancellations())]
        pub fn clear_old_cancellations(origin: OriginFor<T>) -> DispatchResult {
            let signer = T::PermissionOrigin::ensure_origin(origin, &Role::ModuleBooker)?;

            let current_block_number =
                <T as pallet::Config>::BlockNumberProvider::current_block_number();
            let cutoff = current_block_number.saturating_sub(T::CancellationWindow::get());

            let mut removed = 0u32;

            // Iterate over school's cancellations (oldest first due to block number in key)
            // Limit to MaxCleanupPerCall to bound weight
            for ((block, booking_id), _) in SchoolCancellations::<T>::iter_prefix(&signer)
                .take(T::MaxCleanupPerCall::get() as usize)
            {
                if block < cutoff {
                    SchoolCancellations::<T>::remove(&signer, (block, booking_id));
                    removed = removed.saturating_add(1);
                }
            }

            // Only update counter and emit event if we actually removed something
            if removed > 0 {
                let mut booking_cancellation_count = BookingCancellationCounter::<T>::get(&signer);

                booking_cancellation_count = booking_cancellation_count
                    .checked_sub(removed)
                    .ok_or(Error::<T>::ArithmeticUnderflow)?;
                BookingCancellationCounter::<T>::insert(&signer, booking_cancellation_count);
                Self::deposit_event(Event::OldCancellationsCleared { school: signer, removed });
            }
            Ok(())
        }

        /// Allows a lecturer (module deliverer) to cancel a claimed booking.
        ///
        /// The origin must be Signed by a ModuleDeliverer and have sufficient funds.
        ///
        /// Parameters:
        /// - `module_id`: The ID of the learning module.
        /// - `booking_id`: The ID of the claimed booking to cancel.
        ///
        /// Emits `ClaimCancelled` event when successful.
        #[pallet::call_index(12)]
        #[pallet::weight(<T as pallet::Config>::WeightInfo::cancel_claim())]
        pub fn cancel_claim(
            origin: OriginFor<T>,
            module_id: ModuleId,
            booking_id: BookingId,
        ) -> DispatchResult {
            let signer = T::PermissionOrigin::ensure_origin(origin, &Role::ModuleDeliverer)?;

            // Check booking details
            let mut module_details =
                ModuleInfo::<T>::get(module_id).ok_or(Error::<T>::ModuleNotAvailable)?;
            let mut booking =
                Bookings::<T>::get(module_id, booking_id).ok_or(Error::<T>::NoBookingAvailable)?;
            ensure!(booking.lecturer == Some(signer.clone()), Error::<T>::NoPermission);

            let mut module_deliverer_info = ModuleDeliverer::<T>::get(&signer)
                .ok_or(Error::<T>::ModuleDelivererNotRegistered)?;

            // Apply strike for cancelling a claimed booking
            module_deliverer_info.active_strikes = module_deliverer_info
                .active_strikes
                .checked_add(1)
                .ok_or(Error::<T>::ArithmeticOverflow)?;

            let active_strikes_amount = module_deliverer_info.active_strikes;

            // Slash lecturer if the strike amount exceeds MaxAllowedStrikes
            if active_strikes_amount >= T::MaxAllowedStrikes::get() {
                let slash_this_time =
                    T::StrikeSlashPercentage::get().mul_ceil(T::ModuleDelivererDeposit::get());

                let slash_amount = slash_this_time.min(module_deliverer_info.deposit);

                module_deliverer_info.deposit = module_deliverer_info
                    .deposit
                    .checked_sub(&slash_amount)
                    .ok_or(Error::<T>::ArithmeticUnderflow)?;

                let (imbalance, _remaining) = <T as pallet::Config>::NativeCurrency::slash(
                    &HoldReason::ModuleDelivererReserve.into(),
                    &signer,
                    slash_amount,
                );

                T::Slash::on_unbalanced(imbalance);
            }

            // Reset booking and free resources
            booking.lecturer = None;
            booking.claimed_at = None;

            module_deliverer_info.active_claims = module_deliverer_info
                .active_claims
                .checked_sub(1)
                .ok_or(Error::<T>::ArithmeticUnderflow)?;

            module_details.university_student_allocation = module_details
                .university_student_allocation
                .checked_add(1)
                .ok_or(Error::<T>::ArithmeticOverflow)?;

            // Persist all changes
            Bookings::<T>::insert(module_id, booking_id, booking);
            ModuleDeliverer::<T>::insert(&signer, module_deliverer_info);
            ModuleInfo::<T>::insert(module_id, module_details);

            Self::deposit_event(Event::ClaimCancelled {
                lecturer: signer,
                module_id,
                booking_id,
                active_strikes: active_strikes_amount,
            });
            Ok(())
        }

        /// Registers or updates a module deliverer with the current required deposit.
        ///
        /// The origin must be Signed by a ModuleDeliverer and have sufficient funds.
        ///
        /// Parameters: None
        ///
        /// - Emits `ModuleDelivererRegistered` for new registration
        /// - Emits `ModuleDelivererDepositIncreased` when deposit is topped up
        #[pallet::call_index(13)]
        #[pallet::weight(<T as pallet::Config>::WeightInfo::register_module_deliverer())]
        pub fn register_module_deliverer(origin: OriginFor<T>) -> DispatchResult {
            let signer = T::PermissionOrigin::ensure_origin(origin, &Role::ModuleDeliverer)?;

            let required_deposit = T::ModuleDelivererDeposit::get();
            if let Some(mut info) = ModuleDeliverer::<T>::get(&signer) {
                // Already registered — increase deposit if needed
                let additional = required_deposit.saturating_sub(info.deposit);

                if !additional.is_zero() {
                    T::NativeCurrency::hold(
                        &HoldReason::ModuleDelivererReserve.into(),
                        &signer,
                        additional,
                    )?;

                    let old_deposit = info.deposit;
                    info.deposit = required_deposit;

                    ModuleDeliverer::<T>::insert(&signer, info);

                    Self::deposit_event(Event::ModuleDelivererDepositIncreased {
                        module_deliverer: signer,
                        old_deposit,
                        new_deposit: required_deposit,
                    });
                }
                Ok(())
            } else {
                T::NativeCurrency::hold(
                    &HoldReason::ModuleDelivererReserve.into(),
                    &signer,
                    required_deposit,
                )?;

                // Register the module deliverer
                let deliverer_info = ModuleDelivererInfo {
                    deposit: required_deposit,
                    active_claims: 0,
                    active_strikes: 0,
                    successful_deliveries: 0,
                };

                ModuleDeliverer::<T>::insert(&signer, deliverer_info);
                Self::deposit_event(Event::<T>::ModuleDelivererRegistered {
                    module_deliverer: signer,
                    deposit: required_deposit,
                });
                Ok(())
            }
        }

        /// Allows a registered module deliverer to permanently unregister and withdraw their deposit.
        ///
        /// The origin must be Signed by a ModuleDeliverer and have sufficient funds.
        ///
        /// Parameters: None
        ///
        /// Emits `ModuleDelivererUnregistered` event when successful.
        #[pallet::call_index(14)]
        #[pallet::weight(<T as pallet::Config>::WeightInfo::unregister_module_deliverer())]
        pub fn unregister_module_deliverer(origin: OriginFor<T>) -> DispatchResult {
            let signer = T::PermissionOrigin::ensure_origin(origin, &Role::ModuleDeliverer)?;
            let deliverer_info = ModuleDeliverer::<T>::get(&signer)
                .ok_or(Error::<T>::ModuleDelivererNotRegistered)?;
            // Ensure the module deliverer has no active claims
            ensure!(deliverer_info.active_claims.is_zero(), Error::<T>::ModuleDelivererStillActive);

            // Release the module deliverer's deposit
            T::NativeCurrency::release(
                &HoldReason::ModuleDelivererReserve.into(),
                &signer,
                deliverer_info.deposit,
                Precision::Exact,
            )?;

            // Remove the ModuleDeliverer role from the caller
            T::RoleProvider::role_removal(signer.clone(), Role::ModuleDeliverer)?;

            // Remove the module deliverer's registration
            ModuleDeliverer::<T>::remove(&signer);
            Self::deposit_event(Event::<T>::ModuleDelivererUnregistered {
                module_deliverer: signer,
            });
            Ok(())
        }
    }

    impl<T: Config> Pallet<T> {
        /// Returns the account ID of the pallet.
        pub fn account_id() -> AccountIdOf<T> {
            <T as pallet::Config>::PalletId::get().into_account_truncating()
        }

        /// Returns the account ID of the treasury pallet.
        pub fn treasury_account_id() -> AccountIdOf<T> {
            <T as pallet::Config>::TreasuryId::get().into_account_truncating()
        }

        /// Returns the decimal multiplier (10^decimals) for a given payment asset.
        fn asset_decimal_multiplier(payment_asset: u32) -> Result<u128, Error<T>> {
            let asset_decimals = T::AssetMetadata::get_decimals(payment_asset)
                .ok_or(Error::<T>::AssetMetadataNotAvailable)?;
            10u128.checked_pow(asset_decimals as u32).ok_or(Error::<T>::ArithmeticError)
        }

        /// Adjusts a 100×-scaled price to actual foreign asset units.
        ///
        /// Prices are stored at 100× scale for integer precision. This divides by 100
        /// and multiplies by the asset's decimal multiplier.
        fn adjust_price_by_multiplier(
            price: <T as pallet::Config>::Balance,
            multiplier: u128,
        ) -> Result<<T as pallet::Config>::Balance, Error<T>> {
            price
                .checked_mul(&multiplier.into())
                .ok_or(Error::<T>::MultiplyError)?
                .checked_div(&100u128.into())
                .ok_or(Error::<T>::ArithmeticError)
        }

        /// Mints an NFT into a collection and sets its metadata.
        ///
        /// `pallet_account` must be `Self::account_id()` — passed in by the caller so it is
        /// computed only once even when this function is called multiple times.
        fn mint_nft_with_metadata(
            collection_id: &<T as pallet::Config>::NftCollectionId,
            item_id: &<T as pallet::Config>::NftId,
            owner: &AccountIdOf<T>,
            data: &BoundedVec<u8, <T as pallet::Config>::StringLimit>,
            pallet_account: &AccountIdOf<T>,
        ) -> DispatchResult {
            <T as pallet::Config>::Nfts::mint_into(
                collection_id,
                item_id,
                owner,
                &Self::default_item_config(),
                true,
            )?;
            <T as pallet::Config>::Nfts::set_item_metadata(
                Some(pallet_account),
                collection_id,
                item_id,
                data,
            )
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

        /// Set the default collection configuration for creating a collection.
        fn default_collection_config(
        ) -> CollectionConfig<T::Balance, BlockNumberFor<T>, <T as pallet::Config>::NftCollectionId>
        {
            Self::collection_config_with_all_settings_disabled()
        }

        fn collection_config_with_all_settings_disabled(
        ) -> CollectionConfig<T::Balance, BlockNumberFor<T>, <T as pallet::Config>::NftCollectionId>
        {
            CollectionConfig {
                settings: CollectionSettings::from_disabled(
                    pallet_nfts::CollectionSetting::TransferableItems.into(),
                ),
                max_supply: None,
                mint_settings: MintSettings::default(),
            }
        }

        /// Set the default item configuration for minting a nft.
        fn default_item_config() -> ItemConfig {
            ItemConfig { settings: ItemSettings::all_enabled() }
        }

        /// Create the new asset.
        fn do_create_asset(asset_id: u32, admin: AccountIdOf<T>) -> DispatchResult {
            T::LocalCurrency::create(asset_id, admin, false, One::one())
        }

        /// Mint the `amount` of tokens with `asset_id` into the beneficiary's account.
        fn do_mint_asset(
            asset_id: u32,
            beneficiary: &AccountIdOf<T>,
            amount: <T as pallet::Config>::Balance,
        ) -> DispatchResult {
            T::LocalCurrency::mint_into(asset_id, beneficiary, amount)?;
            Ok(())
        }

        /// Set the metadata for the newly created asset.
        fn do_set_metadata(
            asset_id: u32,
            depositor: &AccountIdOf<T>,
            pallet_account: &AccountIdOf<T>,
            nft_collection_id: &<T as pallet::Config>::NftCollectionId,
            nft_id: &<T as pallet::Config>::NftId,
        ) -> DispatchResult {
            let name = format!(
                "{} {nft_collection_id}-{nft_id}",
                String::from_utf8_lossy(&T::NewAssetName::get())
            );
            let symbol: &[u8] = &T::NewAssetSymbol::get();
            let existential_deposit = T::NativeCurrency::minimum_balance();
            let pallet_account_balance = T::NativeCurrency::balance(pallet_account);

            if pallet_account_balance < existential_deposit {
                T::NativeCurrency::transfer(
                    depositor,
                    pallet_account,
                    existential_deposit,
                    Preserve,
                )?;
            }
            let metadata_deposit = T::LocalCurrency::calc_metadata_deposit(name.as_bytes(), symbol);
            if !metadata_deposit.is_zero() {
                T::NativeCurrency::transfer(depositor, pallet_account, metadata_deposit, Preserve)?;
            }
            T::LocalCurrency::set(asset_id, pallet_account, name.into(), symbol.into(), 0)
        }
    }
}
