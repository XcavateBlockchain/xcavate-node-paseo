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

use crate::{mock::*, Error};
use crate::{
    traits::{
        PropertySharesInspect, PropertySharesManage, PropertySharesOwnership, PropertySharesSpvControl,
    },
    PropertyAssetDetails, PropertyAssetInfo, PropertyOwner, PropertyOwnerShares,
};
use frame_support::{
    assert_noop, assert_ok,
    traits::{OnFinalize, OnInitialize},
    BoundedBTreeSet,
};
use pallet_regions::RegionIdentifier;
use sp_runtime::{ArithmeticError, Permill, TokenError};
use std::collections::BTreeSet;

macro_rules! bvec {
	($( $x:tt )*) => {
		vec![$( $x )*].try_into().unwrap()
	}
}

fn run_to_block(n: u64) {
    while System::block_number() < n {
        if System::block_number() > 0 {
            RealWorldAsset::on_finalize(System::block_number());
            System::on_finalize(System::block_number());
        }
        System::reset_events();
        System::set_block_number(System::block_number() + 1);
        System::on_initialize(System::block_number());
        RealWorldAsset::on_initialize(System::block_number());
    }
}

fn new_region_helper() {
    assert_ok!(XcavateWhitelist::assign_role(
        RuntimeOrigin::signed([20; 32].into()),
        [8; 32].into(),
        pallet_xcavate_whitelist::Role::RegionalOperator
    ));
    assert_ok!(XcavateWhitelist::assign_role(
        RuntimeOrigin::signed([20; 32].into()),
        [8; 32].into(),
        pallet_xcavate_whitelist::Role::RealEstateInvestor
    ));
    assert_ok!(Regions::propose_new_region(
        RuntimeOrigin::signed([8; 32].into()),
        RegionIdentifier::Japan
    ));
    assert_ok!(Regions::vote_on_region_proposal(
        RuntimeOrigin::signed([8; 32].into()),
        3,
        pallet_regions::Vote::Yes,
        100_000
    ));
    run_to_block(31);
    assert_ok!(Regions::bid_on_region(RuntimeOrigin::signed([8; 32].into()), 3, 100_000));
    run_to_block(61);
    assert_ok!(Regions::create_new_region(
        RuntimeOrigin::signed([8; 32].into()),
        3,
        30,
        Permill::from_percent(3)
    ));
    assert_ok!(Regions::create_new_location(
        RuntimeOrigin::signed([8; 32].into()),
        3,
        bvec![10, 10]
    ));
}

// create_property_shares tests

#[test]
fn create_property_shares_works() {
    new_test_ext().execute_with(|| {
        assert_ok!(XcavateWhitelist::add_admin(RuntimeOrigin::root(), [20; 32].into(),));
        new_region_helper();
        assert_ok!(RealWorldAsset::create_property_shares(
            &[0; 32].into(),
            3,
            bvec![10, 10],
            10,
            1_000,
            bvec![22, 22]
        ));
        assert_eq!(LocalAssets::balance(0, &RealWorldAsset::property_account_id(0)), 10);
        assert_eq!(Nfts::owner(0, 0).unwrap(), RealWorldAsset::property_account_id(0));
        assert_eq!(
            PropertyAssetInfo::<Test>::get(0).unwrap(),
            PropertyAssetDetails {
                collection_id: 0,
                item_id: 0,
                namespace_id: 0,
                region: 3,
                location: bvec![10, 10],
                price: 1_000,
                share_amount: 10,
                spv_created: false,
                finalized: false,
            }
        );
    })
}

// burn_property_shares tests

#[test]
fn burn_property_shares_works() {
    new_test_ext().execute_with(|| {
        assert_ok!(XcavateWhitelist::add_admin(RuntimeOrigin::root(), [20; 32].into(),));
        new_region_helper();
        assert_ok!(RealWorldAsset::create_property_shares(
            &[0; 32].into(),
            3,
            bvec![10, 10],
            10,
            1_000,
            bvec![22, 22]
        ));
        assert_eq!(LocalAssets::balance(0, &RealWorldAsset::property_account_id(0)), 10);
        assert_ok!(RealWorldAsset::burn_property_shares(0));
        assert_eq!(LocalAssets::balance(0, &RealWorldAsset::property_account_id(0)), 0);
        assert_eq!(Nfts::owner(0, 0).is_none(), true);
        assert_eq!(PropertyAssetInfo::<Test>::get(0).is_none(), true);
    })
}

#[test]
fn burn_property_shares_fails() {
    new_test_ext().execute_with(|| {
        assert_ok!(XcavateWhitelist::add_admin(RuntimeOrigin::root(), [20; 32].into(),));
        new_region_helper();
        assert_noop!(
            RealWorldAsset::burn_property_shares(0),
            Error::<Test>::PropertyAssetNotRegistered
        );
        assert_ok!(RealWorldAsset::create_property_shares(
            &[0; 32].into(),
            3,
            bvec![10, 10],
            10,
            1_000,
            bvec![22, 22]
        ));
        assert_ok!(RealWorldAsset::do_distribute_property_shares_to_owner(0, &[1; 32].into(), 10));
        assert_eq!(LocalAssets::balance(0, &[1; 32].into()), 10);
        assert_noop!(RealWorldAsset::burn_property_shares(0), TokenError::FundsUnavailable);
    })
}

// distribute_property_shares_to_owner tests

#[test]
fn distribute_property_shares_to_owner_works() {
    new_test_ext().execute_with(|| {
        assert_ok!(XcavateWhitelist::add_admin(RuntimeOrigin::root(), [20; 32].into(),));
        new_region_helper();
        assert_ok!(RealWorldAsset::create_property_shares(
            &[0; 32].into(),
            3,
            bvec![10, 10],
            10,
            1_000,
            bvec![22, 22]
        ));
        assert_ok!(RealWorldAsset::distribute_property_shares_to_owner(0, &[1; 32].into(), 4));
        assert_ok!(RealWorldAsset::distribute_property_shares_to_owner(0, &[2; 32].into(), 6));
        assert_eq!(LocalAssets::balance(0, &RealWorldAsset::property_account_id(0)), 0);
        assert_eq!(LocalAssets::balance(0, &[1; 32].into()), 4);
        assert_eq!(LocalAssets::balance(0, &[2; 32].into()), 6);
        assert_eq!(
            PropertyOwner::<Test>::get(0),
            BoundedBTreeSet::<_, MaxPropertyShares>::try_from(
                [[1; 32].into(), [2; 32].into()].into_iter().collect::<BTreeSet<_>>()
            )
            .unwrap()
        );
        assert_eq!(PropertyOwnerShares::<Test>::get::<u32, AccountId>(0, [1; 32].into()), 4);
        assert_eq!(PropertyOwnerShares::<Test>::get::<u32, AccountId>(0, [2; 32].into()), 6);
    })
}

#[test]
fn distribute_property_shares_to_owner_fails() {
    new_test_ext().execute_with(|| {
        assert_ok!(XcavateWhitelist::add_admin(RuntimeOrigin::root(), [20; 32].into(),));
        new_region_helper();
        assert_ok!(RealWorldAsset::create_property_shares(
            &[0; 32].into(),
            3,
            bvec![10, 10],
            10,
            1_000,
            bvec![22, 22]
        ));
        assert_eq!(LocalAssets::balance(0, &RealWorldAsset::property_account_id(0)), 10);
        assert_noop!(
            RealWorldAsset::distribute_property_shares_to_owner(0, &[1; 32].into(), 11),
            ArithmeticError::Underflow
        );
        assert_eq!(LocalAssets::balance(0, &RealWorldAsset::property_account_id(0)), 10);
    })
}

// transfer_property_shares tests

#[test]
fn transfer_property_shares_works() {
    new_test_ext().execute_with(|| {
        assert_ok!(XcavateWhitelist::add_admin(RuntimeOrigin::root(), [20; 32].into(),));
        new_region_helper();
        assert_ok!(RealWorldAsset::create_property_shares(
            &[0; 32].into(),
            3,
            bvec![10, 10],
            10,
            1_000,
            bvec![22, 22]
        ));
        assert_ok!(RealWorldAsset::distribute_property_shares_to_owner(0, &[1; 32].into(), 4));
        assert_ok!(RealWorldAsset::distribute_property_shares_to_owner(0, &[2; 32].into(), 6));
        assert_ok!(RealWorldAsset::transfer_property_shares(
            0,
            &[2; 32].into(),
            &[2; 32].into(),
            &[3; 32].into(),
            3
        ));
        assert_eq!(LocalAssets::balance(0, &[2; 32].into()), 3);
        assert_eq!(LocalAssets::balance(0, &[3; 32].into()), 3);
        assert_ok!(RealWorldAsset::transfer_property_shares(
            0,
            &[2; 32].into(),
            &[2; 32].into(),
            &[3; 32].into(),
            3
        ));
        assert_eq!(
            PropertyOwner::<Test>::get(0),
            BoundedBTreeSet::<_, MaxPropertyShares>::try_from(
                [[1; 32].into(), [3; 32].into()].into_iter().collect::<BTreeSet<_>>()
            )
            .unwrap()
        );
        assert_eq!(LocalAssets::balance(0, &[2; 32].into()), 0);
        assert_eq!(LocalAssets::balance(0, &[3; 32].into()), 6);
        assert_ok!(RealWorldAsset::transfer_property_shares(
            0,
            &[1; 32].into(),
            &[3; 32].into(),
            &[0; 32].into(),
            3
        ));
        assert_eq!(LocalAssets::balance(0, &[0; 32].into()), 3);
        assert_eq!(LocalAssets::balance(0, &[1; 32].into()), 4);
        assert_eq!(LocalAssets::balance(0, &[3; 32].into()), 3);
        assert_eq!(PropertyOwnerShares::<Test>::get::<u32, AccountId>(0, [0; 32].into()), 3);
        assert_eq!(PropertyOwnerShares::<Test>::get::<u32, AccountId>(0, [1; 32].into()), 1);
        assert_eq!(PropertyOwnerShares::<Test>::get::<u32, AccountId>(0, [3; 32].into()), 6);
    })
}

#[test]
fn transfer_property_shares_fails() {
    new_test_ext().execute_with(|| {
        assert_ok!(XcavateWhitelist::add_admin(RuntimeOrigin::root(), [20; 32].into(),));
        new_region_helper();
        assert_ok!(RealWorldAsset::create_property_shares(
            &[0; 32].into(),
            3,
            bvec![10, 10],
            10,
            1_000,
            bvec![22, 22]
        ));
        assert_ok!(RealWorldAsset::distribute_property_shares_to_owner(0, &[1; 32].into(), 4));
        assert_ok!(RealWorldAsset::distribute_property_shares_to_owner(0, &[2; 32].into(), 6));
        assert_noop!(
            RealWorldAsset::transfer_property_shares(
                0,
                &[2; 32].into(),
                &[2; 32].into(),
                &[3; 32].into(),
                7
            ),
            Error::<Test>::NotEnoughShares
        );
        assert_noop!(
            RealWorldAsset::transfer_property_shares(
                0,
                &[1; 32].into(),
                &[2; 32].into(),
                &[3; 32].into(),
                6
            ),
            Error::<Test>::NotEnoughShares
        );
        assert_eq!(LocalAssets::balance(0, &[1; 32].into()), 4);
        assert_eq!(LocalAssets::balance(0, &[2; 32].into()), 6);
    })
}

// take_property_shares tests

#[test]
fn take_property_shares_works() {
    new_test_ext().execute_with(|| {
        assert_ok!(XcavateWhitelist::add_admin(RuntimeOrigin::root(), [20; 32].into(),));
        new_region_helper();
        assert_ok!(RealWorldAsset::create_property_shares(
            &[0; 32].into(),
            3,
            bvec![10, 10],
            10,
            1_000,
            bvec![22, 22]
        ));
        assert_ok!(RealWorldAsset::distribute_property_shares_to_owner(0, &[1; 32].into(), 4));
        assert_eq!(PropertyOwnerShares::<Test>::get::<u32, AccountId>(0, [1; 32].into()), 4);
        assert_eq!(RealWorldAsset::take_property_shares(0, &[1; 32].into()), 4);
        assert_eq!(PropertyOwnerShares::<Test>::get::<u32, AccountId>(0, [1; 32].into()), 0);
    })
}

#[test]
fn remove_share_ownership_works() {
    new_test_ext().execute_with(|| {
        assert_ok!(XcavateWhitelist::add_admin(RuntimeOrigin::root(), [20; 32].into(),));
        new_region_helper();
        assert_ok!(RealWorldAsset::create_property_shares(
            &[0; 32].into(),
            3,
            bvec![10, 10],
            10,
            1_000,
            bvec![22, 22]
        ));
        assert_ok!(RealWorldAsset::distribute_property_shares_to_owner(0, &[1; 32].into(), 4));
        assert_eq!(PropertyOwnerShares::<Test>::get::<u32, AccountId>(0, [1; 32].into()), 4);
        assert_eq!(RealWorldAsset::take_property_shares(0, &[1; 32].into()), 4);
        assert_eq!(PropertyOwnerShares::<Test>::get::<u32, AccountId>(0, [1; 32].into()), 0);
    })
}

// clear_share_owners tests

#[test]
fn clear_share_owners_works() {
    new_test_ext().execute_with(|| {
        assert_ok!(XcavateWhitelist::add_admin(RuntimeOrigin::root(), [20; 32].into(),));
        new_region_helper();
        assert_ok!(RealWorldAsset::create_property_shares(
            &[0; 32].into(),
            3,
            bvec![10, 10],
            10,
            1_000,
            bvec![22, 22]
        ));
        assert_ok!(RealWorldAsset::distribute_property_shares_to_owner(0, &[1; 32].into(), 4));
        assert_ok!(RealWorldAsset::distribute_property_shares_to_owner(0, &[2; 32].into(), 6));
        assert_eq!(
            PropertyOwner::<Test>::get(0),
            BoundedBTreeSet::<_, MaxPropertyShares>::try_from(
                [[1; 32].into(), [2; 32].into()].into_iter().collect::<BTreeSet<_>>()
            )
            .unwrap()
        );
        assert_ok!(RealWorldAsset::clear_share_owners(0));
        assert_eq!(
            PropertyOwner::<Test>::get(0),
            BoundedBTreeSet::<_, MaxPropertyShares>::try_from(
                [].into_iter().collect::<BTreeSet<_>>()
            )
            .unwrap()
        );
    })
}

// register_spv tests

#[test]
fn register_spv_works() {
    new_test_ext().execute_with(|| {
        assert_ok!(XcavateWhitelist::add_admin(RuntimeOrigin::root(), [20; 32].into(),));
        new_region_helper();
        assert_ok!(RealWorldAsset::create_property_shares(
            &[0; 32].into(),
            3,
            bvec![10, 10],
            10,
            1_000,
            bvec![22, 22]
        ));
        assert_eq!(
            PropertyAssetInfo::<Test>::get(0).unwrap(),
            PropertyAssetDetails {
                collection_id: 0,
                item_id: 0,
                namespace_id: 0,
                region: 3,
                location: bvec![10, 10],
                price: 1_000,
                share_amount: 10,
                spv_created: false,
                finalized: false,
            }
        );
        assert_ok!(RealWorldAsset::register_spv(0));
        assert_eq!(
            PropertyAssetInfo::<Test>::get(0).unwrap(),
            PropertyAssetDetails {
                collection_id: 0,
                item_id: 0,
                namespace_id: 0,
                region: 3,
                location: bvec![10, 10],
                price: 1_000,
                share_amount: 10,
                spv_created: true,
                finalized: false,
            }
        );
    })
}

#[test]
fn register_spv_fails() {
    new_test_ext().execute_with(|| {
        assert_ok!(XcavateWhitelist::add_admin(RuntimeOrigin::root(), [20; 32].into(),));
        new_region_helper();
        assert_noop!(RealWorldAsset::register_spv(0), Error::<Test>::PropertyAssetNotRegistered);
    })
}

#[test]
fn getter_function_works() {
    new_test_ext().execute_with(|| {
        assert_ok!(XcavateWhitelist::add_admin(RuntimeOrigin::root(), [20; 32].into(),));
        new_region_helper();
        assert_ok!(RealWorldAsset::create_property_shares(
            &[0; 32].into(),
            3,
            bvec![10, 10],
            10,
            1_000,
            bvec![22, 22]
        ));
        assert_ok!(RealWorldAsset::distribute_property_shares_to_owner(0, &[1; 32].into(), 4));
        assert_ok!(RealWorldAsset::distribute_property_shares_to_owner(0, &[2; 32].into(), 6));
        assert_eq!(
            RealWorldAsset::get_property_asset_info(0).unwrap(),
            PropertyAssetDetails {
                collection_id: 0,
                item_id: 0,
                namespace_id: 0,
                region: 3,
                location: bvec![10, 10],
                price: 1_000,
                share_amount: 10,
                spv_created: false,
                finalized: false,
            }
        );
        assert_eq!(
            PropertyOwner::<Test>::get(0),
            BoundedBTreeSet::<_, MaxPropertyShares>::try_from(
                [[1; 32].into(), [2; 32].into()].into_iter().collect::<BTreeSet<_>>()
            )
            .unwrap()
        );
        assert_eq!(RealWorldAsset::take_property_shares(0, &[1; 32].into()), 4);
        assert_eq!(RealWorldAsset::get_property_asset_info(1).is_none(), true);
        assert_eq!(
            PropertyOwner::<Test>::get(1),
            BoundedBTreeSet::<_, MaxPropertyShares>::try_from(
                [].into_iter().collect::<BTreeSet<_>>()
            )
            .unwrap()
        );
        assert_eq!(RealWorldAsset::take_property_shares(1, &[3; 32].into()), 0);
    })
}
