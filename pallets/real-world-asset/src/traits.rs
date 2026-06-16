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

use frame_support::pallet_prelude::*;

use super::*;

pub trait PropertySharesManage<AccountId, Balance, NftId, StringLimit, LocationId> {
    // Create new property shares by minting an NFT and fractionalizing it into shares.
    fn create_property_shares(
        funding_account: &AccountId,
        region: RegionId,
        location: LocationId,
        share_amount: u32,
        property_price: Balance,
        data: BoundedVec<u8, StringLimit>,
    ) -> Result<(NftId, u32), DispatchError>;

    // Burns a property’s NFT and removes its details.
    fn burn_property_shares(asset_id: u32) -> DispatchResult;
}

pub trait PropertySharesOwnership<AccountId> {
    // Transfer property shares from one account to another.
    fn transfer_property_shares(
        asset_id: u32,
        sender: &AccountId,
        funds_source: &AccountId,
        receiver: &AccountId,
        share_amount: u32,
    ) -> DispatchResult;

    // Distributes property shares to an investor from the property account.
    fn distribute_property_shares_to_owner(
        asset_id: u32,
        investor: &AccountId,
        share_amount: u32,
    ) -> DispatchResult;

    // Removes and returns the share balance of an owner for a property.
    fn take_property_shares(asset_id: u32, owner: &AccountId) -> u32;

    // Removes an account’s share ownership for a property.
    fn remove_property_share_ownership(asset_id: u32, account: &AccountId) -> DispatchResult;

    // Clears all share owners for a property.
    fn clear_share_owners(asset_id: u32) -> DispatchResult;
}

pub trait PropertySharesSpvControl {
    type PropertyAssetInfo;

    // Registers a Special Purpose Vehicle (SPV) for a property.
    fn register_spv(asset_id: u32) -> DispatchResult;

    // Finalizes a property’s sale process.
    fn finalize_property(asset_id: u32) -> DispatchResult;

    // Ensures the SPV for a property has not been created.
    fn ensure_spv_not_created(asset_id: u32) -> DispatchResult;

    // Ensures the SPV for a property has been created.
    fn ensure_spv_created(asset_id: u32) -> DispatchResult;

    // Retrieves property details if the SPV is not created.
    fn get_if_spv_not_created(asset_id: u32) -> Result<Self::PropertyAssetInfo, DispatchError>;

    // Retrieves property details if the sale is finalized.
    fn get_if_property_finalized(asset_id: u32) -> Result<Self::PropertyAssetInfo, DispatchError>;

    // Ensures the property sale has been finalized.
    fn ensure_property_finalized(asset_id: u32) -> DispatchResult;
}

pub trait PropertySharesInspect<AccountId> {
    type PropertyAssetInfo;
    type MaxPropertyShares;

    // Retrieves property asset details.
    fn get_property_asset_info(asset_id: u32) -> Option<Self::PropertyAssetInfo>;

    // Retrieves the list of share owners for a property.
    fn get_property_owner(asset_id: u32) -> BoundedBTreeSet<AccountId, Self::MaxPropertyShares>;

    // Retrieves the share balance of an account for a property.
    fn get_share_balance(asset_id: u32, owner: &AccountId) -> u32;
}

impl<T: Config>
    PropertySharesManage<
        AccountIdOf<T>,
        <T as pallet::Config>::Balance,
        <T as pallet::Config>::NftId,
        <T as pallet::Config>::StringLimit,
        LocationId<T>,
    > for Pallet<T>
{
    fn create_property_shares(
        funding_account: &AccountIdOf<T>,
        region: RegionId,
        location: LocationId<T>,
        share_amount: u32,
        property_price: <T as pallet::Config>::Balance,
        data: BoundedVec<u8, <T as pallet::Config>::StringLimit>,
    ) -> Result<(<T as pallet::Config>::NftId, u32), DispatchError> {
        Self::do_create_property_shares(
            funding_account,
            region,
            location,
            share_amount,
            property_price,
            data,
        )
    }

    fn burn_property_shares(asset_id: u32) -> DispatchResult {
        Self::do_burn_property_shares(asset_id)
    }
}

impl<T: Config> PropertySharesOwnership<AccountIdOf<T>> for Pallet<T> {
    fn transfer_property_shares(
        asset_id: u32,
        sender: &AccountIdOf<T>,
        funds_source: &AccountIdOf<T>,
        receiver: &AccountIdOf<T>,
        share_amount: u32,
    ) -> DispatchResult {
        Self::do_transfer_property_shares(asset_id, sender, funds_source, receiver, share_amount)
    }

    fn distribute_property_shares_to_owner(
        asset_id: u32,
        investor: &AccountIdOf<T>,
        share_amount: u32,
    ) -> DispatchResult {
        Self::do_distribute_property_shares_to_owner(asset_id, investor, share_amount)
    }

    fn take_property_shares(asset_id: u32, owner: &AccountIdOf<T>) -> u32 {
        Self::do_take_property_shares(asset_id, owner)
    }

    fn remove_property_share_ownership(asset_id: u32, account: &AccountIdOf<T>) -> DispatchResult {
        Self::do_remove_property_share_ownership(asset_id, account)
    }

    fn clear_share_owners(asset_id: u32) -> DispatchResult {
        Self::do_clear_share_owners(asset_id)
    }
}

impl<T: Config> PropertySharesSpvControl for Pallet<T> {
    type PropertyAssetInfo = PropertyAssetDetailsOf<T>;

    fn register_spv(asset_id: u32) -> DispatchResult {
        Self::do_register_spv(asset_id)
    }

    fn finalize_property(asset_id: u32) -> DispatchResult {
        Self::do_finalize_property(asset_id)
    }

    fn ensure_spv_not_created(asset_id: u32) -> DispatchResult {
        Self::do_ensure_spv_not_created(asset_id)
    }

    fn ensure_spv_created(asset_id: u32) -> DispatchResult {
        Self::do_ensure_spv_created(asset_id)
    }

    fn get_if_spv_not_created(asset_id: u32) -> Result<Self::PropertyAssetInfo, DispatchError> {
        Self::do_get_if_spv_not_created(asset_id)
    }

    fn get_if_property_finalized(asset_id: u32) -> Result<Self::PropertyAssetInfo, DispatchError> {
        Self::do_get_if_property_finalized(asset_id)
    }

    fn ensure_property_finalized(asset_id: u32) -> DispatchResult {
        Self::do_ensure_property_finalized(asset_id)
    }
}

impl<T: Config> PropertySharesInspect<AccountIdOf<T>> for Pallet<T> {
    type PropertyAssetInfo = PropertyAssetDetailsOf<T>;
    type MaxPropertyShares = T::MaxPropertyShares;

    fn get_property_asset_info(asset_id: u32) -> Option<Self::PropertyAssetInfo> {
        Self::do_get_property_asset_info(asset_id)
    }

    fn get_property_owner(
        asset_id: u32,
    ) -> BoundedBTreeSet<AccountIdOf<T>, Self::MaxPropertyShares> {
        Self::get_property_owner(asset_id)
    }

    fn get_share_balance(asset_id: u32, owner: &AccountIdOf<T>) -> u32 {
        Self::get_share_balance(asset_id, owner)
    }
}
