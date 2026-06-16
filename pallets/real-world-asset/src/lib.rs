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

pub mod traits;

use frame_support::pallet_prelude::*;
use frame_support::transactional;

use frame_support::sp_runtime::traits::{AccountIdConversion, StaticLookup};
use frame_support::{
    traits::{
        fungible::Mutate,
        fungibles::Inspect as FungiblesInspect,
        fungibles::Mutate as FungiblesMutate,
        nonfungibles_v2::Mutate as NonfungiblesMutate,
        nonfungibles_v2::Transfer,
        tokens::{fungible, fungibles, nonfungibles_v2, Balance, Preservation},
    },
    PalletId,
};

use frame_system::RawOrigin;

use pallet_nfts::{CollectionConfig, ItemConfig, ItemSettings};

use pallet_regions::{RegionInfo, RegionTrait};

pub type AccountIdOf<T> = <T as frame_system::Config>::AccountId;

pub type LocalAssetIdOf<T> = <<T as Config>::LocalCurrency as fungibles::Inspect<
    <T as frame_system::Config>::AccountId,
>>::AssetId;

pub trait NamespaceManager<AccountId> {
    fn create_namespace_for_property(
        manager: &AccountId,
        real_world_asset_id: u32,
    ) -> Result<u128, DispatchError>;
}

impl<AccountId> NamespaceManager<AccountId> for () {
    fn create_namespace_for_property(
        _manager: &AccountId,
        _real_world_asset_id: u32,
    ) -> Result<u128, DispatchError> {
        Ok(0)
    }
}

#[frame_support::pallet]
pub mod pallet {
    use super::*;
    use frame_system::pallet_prelude::*;
    use sp_runtime::traits::{CheckedAdd, One};

    /// Details of a property asset in the Xcavate marketplace.
    /// Represents a fractionalized real estate asset with associated NFT and region data.
    #[cfg_attr(feature = "std", derive(serde::Serialize, serde::Deserialize))]
    #[derive(Encode, Decode, Clone, PartialEq, Eq, MaxEncodedLen, RuntimeDebug, TypeInfo)]
    #[scale_info(skip_type_params(T))]
    pub struct PropertyAssetDetails<NftId, NftCollectionId, Balance, LocationId> {
        /// The NFT collection ID for the property.
        pub collection_id: NftCollectionId,
        /// The NFT item ID for the property.
        pub item_id: NftId,
        /// The namespace ID used for bucket-backed property data.
        pub namespace_id: u128,
        /// The region ID where the property is located.
        pub region: RegionId,
        /// The location ID within the region.
        pub location: LocationId,
        /// The total price of the property.
        pub price: Balance,
        /// The total amount of shares representing shares in the property.
        pub share_amount: u32,
        /// Indicates if a Special Purpose Vehicle (SPV) has been created for this property.
        pub spv_created: bool,
        /// Indicates if the property sale has been finalized.
        pub finalized: bool,
    }

    #[pallet::config]
    pub trait Config: frame_system::Config + pallet_nft_fractionalization::Config {
        type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;

        /// The type used to represent balances.
        type Balance: Balance + TypeInfo + From<u128> + Default;

        /// The currency used for deposits.
        type NativeCurrency: fungible::Inspect<AccountIdOf<Self>>
            + fungible::Mutate<AccountIdOf<Self>>
            + fungible::InspectHold<AccountIdOf<Self>, Balance = <Self as pallet::Config>::Balance>
            + fungible::BalancedHold<AccountIdOf<Self>, Balance = <Self as pallet::Config>::Balance>;

        /// Identifier for the NFT collection.
        type NftCollectionId: Member + Parameter + MaxEncodedLen + Copy;

        /// The type for NFT item IDs.
        type NftId: Member + Parameter + MaxEncodedLen + Copy + Default + CheckedAdd + One;

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
                    <Self as pallet::Config>::Balance,
                    BlockNumberFor<Self>,
                    <Self as pallet::Config>::NftCollectionId,
                >,
            >;

        /// The pallet ID for deriving the marketplace's sovereign account.
        #[pallet::constant]
        type MarketplacePalletId: Get<PalletId>;

        /// The currency for property shares.
        type LocalCurrency: fungibles::InspectEnumerable<
                AccountIdOf<Self>,
                Balance = <Self as pallet::Config>::Balance,
                AssetId = u32,
            > + fungibles::metadata::Inspect<AccountIdOf<Self>, AssetId = u32>
            + fungibles::metadata::Mutate<AccountIdOf<Self>, AssetId = u32>
            + fungibles::Mutate<AccountIdOf<Self>, Balance = <Self as pallet::Config>::Balance>
            + fungibles::Inspect<AccountIdOf<Self>, Balance = <Self as pallet::Config>::Balance>;

        /// Collection ID type from pallet nft fractionalization.
        type FractionalizeCollectionId: IsType<<Self as pallet_nft_fractionalization::Config>::NftCollectionId>
            + Parameter
            + From<<Self as pallet::Config>::NftCollectionId>
            + Ord
            + Copy
            + MaxEncodedLen
            + Encode;

        /// Item ID type from pallet nft fractionalization.
        type FractionalizeItemId: IsType<<Self as pallet_nft_fractionalization::Config>::NftId>
            + Parameter
            + From<<Self as pallet::Config>::NftId>
            + Ord
            + Copy
            + MaxEncodedLen
            + Encode;

        /// Asset ID type from pallet nft fractionalization.
        type AssetId: IsType<<Self as pallet_nft_fractionalization::Config>::AssetId>
            + Parameter
            + From<u32>
            + Ord
            + Copy;

        /// The amount to fund a property account.
        #[pallet::constant]
        type PropertyAccountFundingAmount: Get<<Self as pallet::Config>::Balance>;

        /// The maximum number of shares for a property.
        #[pallet::constant]
        type MaxPropertyShares: Get<u32>;

        /// The maximum length of a name or symbol stored on-chain.
        #[pallet::constant]
        type StringLimit: Get<u32>;

        /// Provider for region information.
        type RegionProvider: RegionTrait<
            Info = RegionInfo<
                AccountIdOf<Self>,
                <Self as pallet::Config>::Balance,
                BlockNumberFor<Self>,
                <Self as pallet::Config>::NftCollectionId,
            >,
        >;

        /// The maximum length of data stored in for post codes.
        #[pallet::constant]
        type PostcodeLimit: Get<u32>;

        /// Namespace manager used to create and assign namespaces for property assets.
        type NamespaceManager: super::NamespaceManager<AccountIdOf<Self>>;
    }

    pub type FractionalizedAssetId<T> = <T as Config>::AssetId;
    pub type FractionalizeCollectionId<T> = <T as Config>::FractionalizeCollectionId;
    pub type FractionalizeItemId<T> = <T as Config>::FractionalizeItemId;
    pub type RegionId = u16;
    pub type LocationId<T> = BoundedVec<u8, <T as Config>::PostcodeLimit>;
    pub type PropertyAssetDetailsOf<T> = PropertyAssetDetails<
        <T as pallet::Config>::NftId,
        <T as pallet::Config>::NftCollectionId,
        <T as pallet::Config>::Balance,
        LocationId<T>,
    >;

    #[pallet::pallet]
    pub struct Pallet<T>(_);

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
    /// Nft fractionalization.
    #[pallet::storage]
    pub type NextAssetId<T: Config> = StorageValue<_, u32, ValueQuery>;

    /// Mapping of the assetid to the property details.
    #[pallet::storage]
    pub type PropertyAssetInfo<T: Config> =
        StorageMap<_, Blake2_128Concat, u32, PropertyAssetDetailsOf<T>, OptionQuery>;

    /// Mapping of the assetid to the vector of share holder.
    #[pallet::storage]
    pub type PropertyOwner<T: Config> = StorageMap<
        _,
        Blake2_128Concat,
        u32,
        BoundedBTreeSet<AccountIdOf<T>, T::MaxPropertyShares>,
        ValueQuery,
    >;

    /// Mapping of assetid and accountid to the amount of shares an account is holding of the asset.
    #[pallet::storage]
    pub type PropertyOwnerShares<T: Config> = StorageDoubleMap<
        _,
        Blake2_128Concat,
        u32,
        Blake2_128Concat,
        AccountIdOf<T>,
        u32,
        ValueQuery,
    >;

    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {
        /// Test
        PropertySharesCreated { asset_id: u32, namespace_id: u128 },
        /// The property nft got burned.
        PropertyNftBurned {
            collection_id: <T as pallet::Config>::NftCollectionId,
            item_id: <T as pallet::Config>::NftId,
            asset_id: u32,
        },
    }

    #[pallet::error]
    pub enum Error<T> {
        /// The specified region is not registered.
        RegionUnknown,
        /// Arithmetic overflow occurred during calculation.
        ArithmeticOverflow,
        /// The account lacks sufficient funds for the operation.
        NotEnoughFunds,
        /// The property asset is not registered.
        PropertyAssetNotRegistered,
        /// The sender does not hold enough shares.
        NotEnoughShares,
        /// The specified index is invalid.
        InvalidIndex,
        /// Too many share buyers for the property.
        TooManyShareBuyers,
        /// The property is not registered.
        PropertyNotFound,
        /// The Special Purpose Vehicle (SPV) is already created.
        SpvAlreadyCreated,
        /// The SPV has not been created.
        SpvNotCreated,
        /// The property has not been finalized.
        PropertyNotFinalized,
    }

    impl<T: Config> Pallet<T> {
        /// Returns the account ID of the marketplace pallet.
        pub fn account_id() -> AccountIdOf<T> {
            <T as pallet::Config>::MarketplacePalletId::get().into_account_truncating()
        }

        /// Returns the sub-account ID for a property, derived from its asset ID.
        pub fn property_account_id(asset_id: u32) -> AccountIdOf<T> {
            <T as pallet::Config>::MarketplacePalletId::get()
                .into_sub_account_truncating(("pr", asset_id))
        }

        /// Create new property shares by minting an NFT and fractionalizing it into shares.
        /// Transfers funding to the property account, mints an NFT, fractionalizes it, and stores details.
        #[transactional]
        pub(crate) fn do_create_property_shares(
            funding_account: &AccountIdOf<T>,
            region: RegionId,
            location: LocationId<T>,
            share_amount: u32,
            property_price: <T as pallet::Config>::Balance,
            data: BoundedVec<u8, <T as pallet::Config>::StringLimit>,
        ) -> Result<(<T as pallet::Config>::NftId, u32), DispatchError> {
            let region_info =
                T::RegionProvider::get_region_details(region).ok_or(Error::<T>::RegionUnknown)?;

            // Get next available NFT ID
            let item_id = NextNftId::<T>::get(region_info.collection_id);

            // Find next available asset ID for fractionalization
            let mut asset_number: u32 = NextAssetId::<T>::get();
            let mut asset_id: LocalAssetIdOf<T> = asset_number;
            while !T::LocalCurrency::total_issuance(asset_id).is_zero() {
                asset_number = asset_number.checked_add(1).ok_or(Error::<T>::ArithmeticOverflow)?;
                asset_id = asset_number;
            }
            let asset_id: FractionalizedAssetId<T> = asset_number.into();
            let pallet_account = Self::account_id();
            let property_account = Self::property_account_id(asset_number);

            <T as pallet::Config>::NativeCurrency::transfer(
                funding_account,
                &property_account,
                T::PropertyAccountFundingAmount::get(),
                Preservation::Expendable,
            )
            .map_err(|_| Error::<T>::NotEnoughFunds)?;

            // Mint NFT to property account
            <T as pallet::Config>::Nfts::mint_into(
                &region_info.collection_id,
                &item_id,
                &property_account,
                &Self::default_item_config(),
                true,
            )?;
            <T as pallet::Config>::Nfts::set_item_metadata(
                Some(&pallet_account),
                &region_info.collection_id,
                &item_id,
                &data,
            )?;

            // Fractionalize NFT
            let property_origin: OriginFor<T> = RawOrigin::Signed(property_account.clone()).into();
            let user_lookup = <T::Lookup as StaticLookup>::unlookup(property_account.clone());
            let fractionalize_collection_id =
                FractionalizeCollectionId::<T>::from(region_info.collection_id);
            let fractionalize_item_id = FractionalizeItemId::<T>::from(item_id);

            pallet_nft_fractionalization::Pallet::<T>::fractionalize(
                property_origin,
                fractionalize_collection_id.into(),
                fractionalize_item_id.into(),
                asset_id.into(),
                user_lookup,
                share_amount.into(),
            )?;

            let namespace_id =
                T::NamespaceManager::create_namespace_for_property(funding_account, asset_number)?;

            // Store asset details
            PropertyAssetInfo::<T>::insert(
                asset_number,
                PropertyAssetDetails {
                    collection_id: region_info.collection_id,
                    item_id,
                    namespace_id,
                    region,
                    location,
                    price: property_price,
                    share_amount,
                    spv_created: false,
                    finalized: false,
                },
            );

            let next_item_id =
                item_id.checked_add(&One::one()).ok_or(Error::<T>::ArithmeticOverflow)?;
            let next_asset_number =
                asset_number.checked_add(1).ok_or(Error::<T>::ArithmeticOverflow)?;

            NextNftId::<T>::insert(region_info.collection_id, next_item_id);
            NextAssetId::<T>::put(next_asset_number);
            Self::deposit_event(Event::<T>::PropertySharesCreated {
                asset_id: asset_number,
                namespace_id,
            });
            Ok((item_id, asset_number))
        }

        /// Burns a property’s NFT and removes its details.
        /// Unifies fractionalized shares, burns the NFT, and removes asset details from storage.
        pub(crate) fn do_burn_property_shares(asset_id: u32) -> DispatchResult {
            PropertyAssetInfo::<T>::try_mutate_exists(asset_id, |maybe_details| {
                let asset_details =
                    maybe_details.as_ref().ok_or(Error::<T>::PropertyAssetNotRegistered)?;
                // Unify fractionalized shares
                let pallet_account = Self::property_account_id(asset_id);
                let pallet_origin: OriginFor<T> = RawOrigin::Signed(pallet_account.clone()).into();
                let user_lookup = <T::Lookup as StaticLookup>::unlookup(pallet_account);
                let fractionalize_collection_id =
                    FractionalizeCollectionId::<T>::from(asset_details.collection_id);
                let fractionalize_item_id = FractionalizeItemId::<T>::from(asset_details.item_id);
                let fractionalize_asset_id = FractionalizedAssetId::<T>::from(asset_id);
                pallet_nft_fractionalization::Pallet::<T>::unify(
                    pallet_origin,
                    fractionalize_collection_id.into(),
                    fractionalize_item_id.into(),
                    fractionalize_asset_id.into(),
                    user_lookup,
                )?;
                // Burn NFT
                <T as pallet::Config>::Nfts::burn(
                    &asset_details.collection_id,
                    &asset_details.item_id,
                    None,
                )?;

                Self::deposit_event(Event::<T>::PropertyNftBurned {
                    collection_id: asset_details.collection_id,
                    item_id: asset_details.item_id,
                    asset_id,
                });
                // Remove asset details
                *maybe_details = None;
                Ok::<(), DispatchError>(())
            })?;
            Ok(())
        }

        /// Transfer property shares between accounts.
        /// Updates balances and ownership lists accordingly.
        pub(crate) fn do_transfer_property_shares(
            asset_id: u32,
            sender: &AccountIdOf<T>,
            funds_source: &AccountIdOf<T>,
            receiver: &AccountIdOf<T>,
            share_amount: u32,
        ) -> DispatchResult {
            let sender_balance = PropertyOwnerShares::<T>::get(asset_id, sender);
            let updated_sender_balance =
                sender_balance.checked_sub(share_amount).ok_or(Error::<T>::NotEnoughShares)?;

            // Transfer shares
            <T as pallet::Config>::LocalCurrency::transfer(
                asset_id,
                funds_source,
                receiver,
                share_amount.into(),
                Preservation::Expendable,
            )
            .map_err(|_| Error::<T>::NotEnoughShares)?;

            // Update sender's share balance or remove if zero
            if updated_sender_balance == 0 {
                PropertyOwnerShares::<T>::remove(asset_id, sender);
                PropertyOwner::<T>::try_mutate(asset_id, |owner_set| {
                    owner_set.remove(sender);
                    Ok::<(), DispatchError>(())
                })?;
            } else {
                PropertyOwnerShares::<T>::insert(asset_id, sender, updated_sender_balance);
            }

            // Update receiver's ownership
            let already_exists = PropertyOwner::<T>::try_mutate(asset_id, |owner_set| {
                if owner_set.contains(receiver) {
                    Ok::<bool, DispatchError>(true)
                } else {
                    owner_set
                        .try_insert(receiver.clone())
                        .map_err(|_| Error::<T>::TooManyShareBuyers)?;
                    Ok::<bool, DispatchError>(false)
                }
            })?;

            // Update receiver's share balance
            if already_exists {
                PropertyOwnerShares::<T>::try_mutate(asset_id, receiver, |receiver_balance| {
                    *receiver_balance = receiver_balance
                        .checked_add(share_amount)
                        .ok_or(Error::<T>::ArithmeticOverflow)?;
                    Ok::<(), DispatchError>(())
                })?;
            } else {
                PropertyOwnerShares::<T>::insert(asset_id, receiver, share_amount);
            }
            Ok(())
        }

        /// Distributes property shares to an investor from the property account.
        pub(crate) fn do_distribute_property_shares_to_owner(
            asset_id: u32,
            investor: &AccountIdOf<T>,
            share_amount: u32,
        ) -> DispatchResult {
            // Transfer shares from property account
            let property_account = Self::property_account_id(asset_id);
            <T as pallet::Config>::LocalCurrency::transfer(
                asset_id,
                &property_account,
                investor,
                share_amount.into(),
                Preservation::Expendable,
            )?;

            // Add investor to owner set if not present
            PropertyOwner::<T>::try_mutate(asset_id, |owners| {
                if !owners.contains(investor) {
                    owners
                        .try_insert(investor.clone())
                        .map_err(|_| Error::<T>::TooManyShareBuyers)?;
                }
                Ok::<(), DispatchError>(())
            })?;

            // Update investor's share balance
            let old_amount = PropertyOwnerShares::<T>::get(asset_id, investor);
            let new_amount =
                old_amount.checked_add(share_amount).ok_or(Error::<T>::ArithmeticOverflow)?;
            PropertyOwnerShares::<T>::insert(asset_id, investor, new_amount);
            Ok(())
        }

        /// Removes and returns the share balance of an owner for a property.
        pub(crate) fn do_take_property_shares(asset_id: u32, owner: &AccountIdOf<T>) -> u32 {
            PropertyOwnerShares::<T>::take(asset_id, owner)
        }

        /// Removes an account’s share ownership for a property.
        pub(crate) fn do_remove_property_share_ownership(
            asset_id: u32,
            account: &AccountIdOf<T>,
        ) -> DispatchResult {
            PropertyOwnerShares::<T>::remove(asset_id, account);
            Ok(())
        }

        /// Clears all share owners for a property.
        pub(crate) fn do_clear_share_owners(asset_id: u32) -> DispatchResult {
            PropertyOwner::<T>::remove(asset_id);
            Ok(())
        }

        /// Registers a Special Purpose Vehicle (SPV) for a property.
        pub(crate) fn do_register_spv(asset_id: u32) -> DispatchResult {
            PropertyAssetInfo::<T>::try_mutate(asset_id, |maybe_asset_details| {
                let asset_details =
                    maybe_asset_details.as_mut().ok_or(Error::<T>::PropertyAssetNotRegistered)?;
                asset_details.spv_created = true;
                Ok::<(), DispatchError>(())
            })
        }

        /// Finalizes a property’s sale process.
        pub(crate) fn do_finalize_property(asset_id: u32) -> DispatchResult {
            PropertyAssetInfo::<T>::try_mutate(asset_id, |maybe_asset_details| {
                let asset_details =
                    maybe_asset_details.as_mut().ok_or(Error::<T>::PropertyAssetNotRegistered)?;
                asset_details.finalized = true;
                Ok::<(), DispatchError>(())
            })
        }

        /// Ensures the SPV for a property has not been created.
        pub(crate) fn do_ensure_spv_not_created(asset_id: u32) -> DispatchResult {
            ensure!(
                !Self::do_get_property_asset_info(asset_id)
                    .ok_or(Error::<T>::PropertyNotFound)?
                    .spv_created,
                Error::<T>::SpvAlreadyCreated
            );
            Ok(())
        }

        /// Ensures the SPV for a property has been created.
        pub(crate) fn do_ensure_spv_created(asset_id: u32) -> DispatchResult {
            ensure!(
                Self::do_get_property_asset_info(asset_id)
                    .ok_or(Error::<T>::PropertyNotFound)?
                    .spv_created,
                Error::<T>::SpvNotCreated
            );
            Ok(())
        }

        /// Ensures the property sale has been finalized.
        pub(crate) fn do_ensure_property_finalized(asset_id: u32) -> DispatchResult {
            ensure!(
                Self::do_get_property_asset_info(asset_id)
                    .ok_or(Error::<T>::PropertyNotFound)?
                    .finalized,
                Error::<T>::PropertyNotFinalized
            );
            Ok(())
        }

        /// Retrieves property details if the SPV is not created.
        pub(crate) fn do_get_if_spv_not_created(
            asset_id: u32,
        ) -> Result<PropertyAssetDetailsOf<T>, DispatchError> {
            let asset_details =
                Self::do_get_property_asset_info(asset_id).ok_or(Error::<T>::PropertyNotFound)?;
            ensure!(!asset_details.spv_created, Error::<T>::SpvAlreadyCreated);
            Ok(asset_details)
        }

        /// Retrieves property details if the sale is finalized.
        pub(crate) fn do_get_if_property_finalized(
            asset_id: u32,
        ) -> Result<PropertyAssetDetailsOf<T>, DispatchError> {
            let asset_details =
                Self::do_get_property_asset_info(asset_id).ok_or(Error::<T>::PropertyNotFound)?;
            ensure!(asset_details.finalized, Error::<T>::PropertyNotFinalized);
            Ok(asset_details)
        }

        /// Retrieves property asset details.
        pub(crate) fn do_get_property_asset_info(
            asset_id: u32,
        ) -> Option<PropertyAssetDetailsOf<T>> {
            PropertyAssetInfo::<T>::get(asset_id)
        }

        /// Retrieves the list of share owners for a property.
        pub(crate) fn get_property_owner(
            asset_id: u32,
        ) -> BoundedBTreeSet<AccountIdOf<T>, T::MaxPropertyShares> {
            PropertyOwner::<T>::get(asset_id)
        }

        /// Retrieves the share balance of an account for a property.
        pub(crate) fn get_share_balance(asset_id: u32, owner: &AccountIdOf<T>) -> u32 {
            PropertyOwnerShares::<T>::get(asset_id, owner)
        }

        /// Set the default item configuration for minting a nft.
        fn default_item_config() -> ItemConfig {
            ItemConfig { settings: ItemSettings::all_enabled() }
        }
    }
}
