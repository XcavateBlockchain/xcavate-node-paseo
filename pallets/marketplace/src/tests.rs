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

use crate::{mock::*, Error, Event, *};
use crate::{
    OngoingObjectListing, OngoingOffers, PropertyLawyer, RefundClaimedShare, RefundShare,
    ShareListings, ShareOwner,
};
use frame_support::{
    assert_noop, assert_ok,
    traits::{
        fungible::Inspect as FungibleInspect,
        fungible::InspectHold,
        fungibles::InspectHold as FungiblesInspectHold,
        fungibles::{Inspect, InspectFreeze},
        OnFinalize, OnInitialize,
    },
};
use pallet_real_world_asset::{
    Error as RealWorldAssetError, NextAssetId, NextNftId, PropertyAssetInfo, PropertyOwner,
    PropertyOwnerShares,
};
use pallet_regions::{RealEstateLawyer, RegionDetails, RegionIdentifier};
use sp_runtime::{traits::BadOrigin, Permill, TokenError};

macro_rules! bvec {
	($( $x:tt )*) => {
		vec![$( $x )*].try_into().unwrap()
	}
}

fn run_to_block(n: u64) {
    while System::block_number() < n {
        if System::block_number() > 0 {
            Marketplace::on_finalize(System::block_number());
            System::on_finalize(System::block_number());
        }
        System::reset_events();
        System::set_block_number(System::block_number() + 1);
        System::on_initialize(System::block_number());
        Marketplace::on_initialize(System::block_number());
    }
}

fn new_region_helper() {
    assert_ok!(XcavateWhitelist::assign_role(
        RuntimeOrigin::signed([20; 32].into()),
        [8; 32].into(),
        pallet_xcavate_whitelist::Role::RegionalOperator
    ));
    assert_ok!(Regions::propose_new_region(
        RuntimeOrigin::signed([8; 32].into()),
        RegionIdentifier::Japan
    ));
    assert_ok!(Regions::vote_on_region_proposal(
        RuntimeOrigin::signed([8; 32].into()),
        3,
        pallet_regions::Vote::Yes,
        1_000_000
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
}

// adjust_listing_duration tests from pallet regions
// we test it with the marketplace since listing duration directly affects listings

#[test]
fn adjust_listing_duration_works() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);
        assert_ok!(XcavateWhitelist::add_admin(RuntimeOrigin::root(), [20; 32].into(),));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [8; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));
        new_region_helper();
        assert_eq!(RegionDetails::<Test>::get(3).unwrap().listing_duration, 30);
        assert_ok!(Regions::create_new_location(
            RuntimeOrigin::signed([8; 32].into()),
            3,
            bvec![10, 10]
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [0; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateDeveloper
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [1; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));
        assert_ok!(Marketplace::list_property(
            RuntimeOrigin::signed([0; 32].into()),
            3,
            bvec![10, 10],
            10_000,
            100,
            bvec![22, 22],
            false
        ));
        assert_ok!(Regions::adjust_listing_duration(RuntimeOrigin::signed([8; 32].into()), 3, 50,));
        assert_ok!(Marketplace::list_property(
            RuntimeOrigin::signed([0; 32].into()),
            3,
            bvec![10, 10],
            10_000,
            100,
            bvec![22, 22],
            false
        ));
        // First listing should have expiry at block 91 (1 + 30 + 60)
        assert_eq!(OngoingObjectListing::<Test>::get(0).unwrap().listing_expiry, 91);
        // Second listing should have expiry at block 111 (1 + 50 + 60)
        assert_eq!(OngoingObjectListing::<Test>::get(1).unwrap().listing_expiry, 111);
        run_to_block(92);
        assert_noop!(
            Marketplace::buy_property_shares(RuntimeOrigin::signed([1; 32].into()), 0, 30, 1984),
            Error::<Test>::ListingExpired
        );
        // Second listing should still be ongoing.
        assert_ok!(Marketplace::buy_property_shares(
            RuntimeOrigin::signed([1; 32].into()),
            1,
            30,
            1984
        ));
    })
}

// adjust_region_tax tests from pallet regions
// we test it with the marketplace since the tax directly affects listings

#[test]
fn adjust_region_tax_works() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);
        assert_ok!(XcavateWhitelist::add_admin(RuntimeOrigin::root(), [20; 32].into(),));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [8; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));
        new_region_helper();
        assert_eq!(RegionDetails::<Test>::get(3).unwrap().listing_duration, 30);
        assert_ok!(Regions::create_new_location(
            RuntimeOrigin::signed([8; 32].into()),
            3,
            bvec![10, 10]
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [0; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateDeveloper
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [1; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));
        assert_ok!(Marketplace::list_property(
            RuntimeOrigin::signed([0; 32].into()),
            3,
            bvec![10, 10],
            10_000,
            100,
            bvec![22, 22],
            false
        ));
        assert_ok!(Regions::adjust_region_tax(
            RuntimeOrigin::signed([8; 32].into()),
            3,
            Permill::from_percent(9)
        ));
        assert_ok!(Marketplace::list_property(
            RuntimeOrigin::signed([0; 32].into()),
            3,
            bvec![10, 10],
            10_000,
            100,
            bvec![22, 22],
            false
        ));
        assert_eq!(OngoingObjectListing::<Test>::get(0).unwrap().tax, Permill::from_percent(3));
        assert_eq!(OngoingObjectListing::<Test>::get(1).unwrap().tax, Permill::from_percent(9));
        assert_ok!(Marketplace::buy_property_shares(
            RuntimeOrigin::signed([1; 32].into()),
            0,
            10,
            1984
        ));
        assert_eq!(AssetsHolder::total_balance_on_hold(1984, &[1; 32].into()), 104_000);
        assert_ok!(Marketplace::buy_property_shares(
            RuntimeOrigin::signed([1; 32].into()),
            1,
            10,
            1984
        ));
        assert_eq!(AssetsHolder::total_balance_on_hold(1984, &[1; 32].into()), 214_000);
    })
}

// list_property tests

#[test]
fn list_property_works() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);
        assert_ok!(XcavateWhitelist::add_admin(RuntimeOrigin::root(), [20; 32].into(),));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [8; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));
        new_region_helper();
        assert_ok!(Regions::create_new_location(
            RuntimeOrigin::signed([8; 32].into()),
            3,
            bvec![10, 10]
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [0; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateDeveloper
        ));
        assert_ok!(Marketplace::list_property(
            RuntimeOrigin::signed([0; 32].into()),
            3,
            bvec![10, 10],
            10_000,
            100,
            bvec![22, 22],
            false
        ));
        assert_eq!(
            Balances::balance_on_hold(&HoldReason::ListingDepositReserve.into(), &([0; 32].into())),
            200_000
        );
        assert_eq!(OngoingObjectListing::<Test>::get(0).unwrap().listed_share_amount, 100);
        assert_eq!(NextNftId::<Test>::get(0), 1);
        assert_eq!(NextNftId::<Test>::get(1), 0);
        assert_eq!(NextAssetId::<Test>::get(), 1);
        assert_eq!(OngoingObjectListing::<Test>::get(0).is_some(), true);
        assert_eq!(PropertyAssetInfo::<Test>::get(0).is_some(), true);
        assert_eq!(Nfts::owner(0, 0).unwrap(), Marketplace::property_account_id(0));
    })
}

#[test]
fn list_property_fails() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);
        assert_ok!(XcavateWhitelist::add_admin(RuntimeOrigin::root(), [20; 32].into(),));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [0; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateDeveloper
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [8; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));
        assert_noop!(
            Marketplace::list_property(
                RuntimeOrigin::signed([0; 32].into()),
                3,
                bvec![10, 10],
                10_000,
                100,
                bvec![22, 22],
                false
            ),
            Error::<Test>::RegionUnknown
        );
        new_region_helper();
        assert_noop!(
            Marketplace::list_property(
                RuntimeOrigin::signed([0; 32].into()),
                3,
                bvec![10, 10],
                10_000,
                100,
                bvec![22, 22],
                false
            ),
            Error::<Test>::LocationUnknown
        );
        assert_ok!(Regions::create_new_location(
            RuntimeOrigin::signed([8; 32].into()),
            3,
            bvec![10, 10]
        ));
        assert_noop!(
            Marketplace::list_property(
                RuntimeOrigin::signed([0; 32].into()),
                3,
                bvec![10, 10],
                10_000,
                251,
                bvec![22, 22],
                false
            ),
            Error::<Test>::TooManyShares
        );
        assert_noop!(
            Marketplace::list_property(
                RuntimeOrigin::signed([0; 32].into()),
                3,
                bvec![10, 10],
                10_000,
                99,
                bvec![22, 22],
                false
            ),
            Error::<Test>::ShareAmountTooLow
        );
        assert_noop!(
            Marketplace::list_property(
                RuntimeOrigin::signed([0; 32].into()),
                3,
                bvec![10, 10],
                10_000,
                0,
                bvec![22, 22],
                false
            ),
            Error::<Test>::AmountCannotBeZero
        );
        assert_ok!(XcavateWhitelist::set_permission(
            RuntimeOrigin::signed([20; 32].into()),
            [0; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateDeveloper,
            pallet_xcavate_whitelist::AccessPermission::Revoked,
        ));
        assert_noop!(
            Marketplace::list_property(
                RuntimeOrigin::signed([0; 32].into()),
                3,
                bvec![10, 10],
                10_000,
                100,
                bvec![22, 22],
                false
            ),
            BadOrigin
        );
    })
}

// buy_property_shares tests

#[test]
fn buy_property_shares_works() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);
        assert_ok!(XcavateWhitelist::add_admin(RuntimeOrigin::root(), [20; 32].into(),));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [8; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));
        new_region_helper();
        assert_ok!(Regions::create_new_location(
            RuntimeOrigin::signed([8; 32].into()),
            3,
            bvec![10, 10]
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [6; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [14; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateDeveloper
        ));
        // Test buying property shares with high balance
        assert_ok!(Balances::force_set_balance(
            RuntimeOrigin::root(),
            sp_runtime::MultiAddress::Id([14; 32].into()),
            200_000_000_000_000_000_000
        ));
        assert_ok!(Marketplace::list_property(
            RuntimeOrigin::signed([14; 32].into()),
            3,
            bvec![10, 10],
            10_000_000_000_000_000,
            100,
            bvec![22, 22],
            false
        ));
        assert_ok!(Marketplace::buy_property_shares(
            RuntimeOrigin::signed([6; 32].into()),
            0,
            30,
            1984
        ));
        assert_eq!(OngoingObjectListing::<Test>::get(0).unwrap().listed_share_amount, 70);
        assert_eq!(
            ShareOwner::<Test>::get::<AccountId, u32>([6; 32].into(), 0).unwrap().share_amount,
            30
        );
        assert_eq!(Balances::free_balance(&([6; 32].into())), 5_000);
        assert_eq!(ForeignAssets::total_balance(1984, &[6; 32].into()), 1_500_000_000_000_000_000);
        assert_eq!(ForeignAssets::balance(1984, &[6; 32].into()), 1_188_000_000_000_000_000);
        assert_eq!(
            AssetsHolder::total_balance_on_hold(1984, &[6; 32].into()),
            312_000_000_000_000_000
        );
        System::assert_last_event(
            Event::PropertySharesBought {
                listing_index: 0,
                asset_id: 0,
                buyer: [6; 32].into(),
                amount_purchased: 30,
                price_paid: 300_000_000_000_000_000,
                tax_paid: 9_000_000_000_000_000,
                payment_asset: 1984,
                new_shares_remaining: 70,
            }
            .into(),
        );
    })
}

#[test]
fn buy_property_shares_works_2() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);
        assert_ok!(XcavateWhitelist::add_admin(RuntimeOrigin::root(), [20; 32].into(),));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [8; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));
        new_region_helper();
        assert_ok!(Regions::create_new_location(
            RuntimeOrigin::signed([8; 32].into()),
            3,
            bvec![10, 10]
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [0; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateDeveloper
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [1; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [2; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [30; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));
        assert_ok!(Marketplace::list_property(
            RuntimeOrigin::signed([0; 32].into()),
            3,
            bvec![10, 10],
            10_000,
            200,
            bvec![22, 22],
            false
        ));
        // An account can not have 50 % or more of the property shares
        assert_noop!(
            Marketplace::buy_property_shares(RuntimeOrigin::signed([1; 32].into()), 0, 100, 1984),
            Error::<Test>::ExceedsMaxOwnership
        );
        assert_ok!(Marketplace::buy_property_shares(
            RuntimeOrigin::signed([1; 32].into()),
            0,
            99,
            1984
        ));
        // An account can not have 50 % or more of the property shares
        assert_noop!(
            Marketplace::buy_property_shares(RuntimeOrigin::signed([1; 32].into()), 0, 1, 1984),
            Error::<Test>::ExceedsMaxOwnership
        );
        assert_eq!(
            ShareOwner::<Test>::get::<AccountId, u32>([1; 32].into(), 0).unwrap().share_amount,
            99
        );
    })
}

#[test]
fn buy_property_shares_works_developer_covers_tax() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);
        assert_ok!(XcavateWhitelist::add_admin(RuntimeOrigin::root(), [20; 32].into(),));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [8; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));
        new_region_helper();
        assert_ok!(Regions::create_new_location(
            RuntimeOrigin::signed([8; 32].into()),
            3,
            bvec![10, 10]
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [0; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateDeveloper
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [1; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));
        assert_ok!(Marketplace::list_property(
            RuntimeOrigin::signed([0; 32].into()),
            3,
            bvec![10, 10],
            10_000,
            100,
            bvec![22, 22],
            true
        ));
        assert_ok!(Marketplace::buy_property_shares(
            RuntimeOrigin::signed([1; 32].into()),
            0,
            30,
            1984
        ));
        assert_eq!(OngoingObjectListing::<Test>::get(0).unwrap().listed_share_amount, 70);
        assert_eq!(
            ShareOwner::<Test>::get::<AccountId, u32>([1; 32].into(), 0).unwrap().share_amount,
            30
        );
        assert_eq!(ForeignAssets::total_balance(1984, &[1; 32].into()), 1_500_000);
        // Buyer still have to pay 1 % fee even though developer covers tax
        assert_eq!(ForeignAssets::balance(1984, &[1; 32].into()), 1_197_000);
        assert_eq!(AssetsHolder::total_balance_on_hold(1984, &[1; 32].into()), 303_000);
        assert_eq!(ShareOwner::<Test>::get::<AccountId, u32>([6; 32].into(), 0), None);
        System::assert_last_event(
            Event::PropertySharesBought {
                listing_index: 0,
                asset_id: 0,
                buyer: [1; 32].into(),
                amount_purchased: 30,
                price_paid: 300_000,
                tax_paid: 0,
                payment_asset: 1984,
                new_shares_remaining: 70,
            }
            .into(),
        );
    })
}

#[test]
fn buy_property_shares_doesnt_work() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);
        assert_ok!(XcavateWhitelist::add_admin(RuntimeOrigin::root(), [20; 32].into(),));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [0; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));
        assert_noop!(
            Marketplace::buy_property_shares(RuntimeOrigin::signed([0; 32].into()), 1, 1, 1984),
            Error::<Test>::ShareNotForSale
        );
    })
}

#[test]
fn buy_property_shares_doesnt_work_2() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);
        assert_ok!(XcavateWhitelist::add_admin(RuntimeOrigin::root(), [20; 32].into(),));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [8; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));
        new_region_helper();
        assert_ok!(Regions::create_new_location(
            RuntimeOrigin::signed([8; 32].into()),
            3,
            bvec![10, 10]
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [0; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateDeveloper
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [1; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));
        assert_ok!(Marketplace::list_property(
            RuntimeOrigin::signed([0; 32].into()),
            3,
            bvec![10, 10],
            10_000,
            100,
            bvec![22, 22],
            false
        ));
        assert_noop!(
            Marketplace::buy_property_shares(RuntimeOrigin::signed([1; 32].into()), 0, 101, 1984),
            Error::<Test>::NotEnoughSharesAvailable
        );
        assert_noop!(
            Marketplace::buy_property_shares(RuntimeOrigin::signed([1; 32].into()), 0, 50, 1984),
            Error::<Test>::ExceedsMaxOwnership
        );
        assert_noop!(
            Marketplace::buy_property_shares(RuntimeOrigin::signed([1; 32].into()), 0, 30, 1985),
            Error::<Test>::PaymentAssetNotSupported
        );
        assert_ok!(XcavateWhitelist::set_permission(
            RuntimeOrigin::signed([20; 32].into()),
            [1; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor,
            pallet_xcavate_whitelist::AccessPermission::Revoked,
        ));
        assert_noop!(
            Marketplace::buy_property_shares(RuntimeOrigin::signed([1; 32].into()), 0, 40, 1984),
            BadOrigin
        );
        assert_ok!(XcavateWhitelist::set_permission(
            RuntimeOrigin::signed([20; 32].into()),
            [1; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor,
            pallet_xcavate_whitelist::AccessPermission::Compliant,
        ));
        run_to_block(92);
        assert_noop!(
            Marketplace::buy_property_shares(RuntimeOrigin::signed([1; 32].into()), 0, 30, 1984),
            Error::<Test>::ListingExpired
        );
        assert_ok!(Marketplace::list_property(
            RuntimeOrigin::signed([0; 32].into()),
            3,
            bvec![10, 10],
            1_000,
            250,
            bvec![22, 22],
            false
        ));
        assert_noop!(
            Marketplace::buy_property_shares(RuntimeOrigin::signed([1; 32].into()), 1, 125, 1984),
            Error::<Test>::ExceedsMaxOwnership
        );
        assert_ok!(Marketplace::buy_property_shares(
            RuntimeOrigin::signed([1; 32].into()),
            1,
            124,
            1984
        ));
    })
}

#[test]
fn buy_property_shares_fails_insufficient_balance() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);
        assert_ok!(XcavateWhitelist::add_admin(RuntimeOrigin::root(), [20; 32].into(),));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [8; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));
        new_region_helper();
        assert_ok!(Regions::create_new_location(
            RuntimeOrigin::signed([8; 32].into()),
            3,
            bvec![10, 10]
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [14; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateDeveloper
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [4; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));
        assert_ok!(Balances::force_set_balance(
            RuntimeOrigin::root(),
            sp_runtime::MultiAddress::Id([14; 32].into()),
            200_000_000_000_000_000_000
        ));
        assert_ok!(Marketplace::list_property(
            RuntimeOrigin::signed([14; 32].into()),
            3,
            bvec![10, 10],
            10_000_000_000_000_000,
            100,
            bvec![22, 22],
            false
        ));
        assert_noop!(
            Marketplace::buy_property_shares(RuntimeOrigin::signed([4; 32].into()), 0, 30, 1984),
            TokenError::FundsUnavailable
        );
        assert_eq!(ForeignAssets::balance(1984, &[4; 32].into()), 50);
        assert_eq!(AssetsHolder::total_balance_on_hold(1984, &[6; 32].into()), 0);
    })
}

#[test]
fn listing_and_selling_multiple_objects() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);
        // Region setup and role assignments
        assert_ok!(XcavateWhitelist::add_admin(RuntimeOrigin::root(), [20; 32].into(),));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [8; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));
        new_region_helper();
        assert_ok!(Regions::create_new_location(
            RuntimeOrigin::signed([8; 32].into()),
            3,
            bvec![10, 10]
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [15; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateDeveloper
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [2; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateDeveloper
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [2; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [1; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [30; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [31; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [0; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateDeveloper
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [10; 32].into(),
            pallet_xcavate_whitelist::Role::Lawyer
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [11; 32].into(),
            pallet_xcavate_whitelist::Role::Lawyer
        ));
        assert_ok!(Regions::register_lawyer(RuntimeOrigin::signed([10; 32].into()), 3,));
        assert_ok!(Regions::register_lawyer(RuntimeOrigin::signed([11; 32].into()), 3,));

        // List multiple properties
        assert_ok!(Marketplace::list_property(
            RuntimeOrigin::signed([15; 32].into()),
            3,
            bvec![10, 10],
            10_000,
            100,
            bvec![22, 22],
            false
        ));
        assert_ok!(Marketplace::list_property(
            RuntimeOrigin::signed([2; 32].into()),
            3,
            bvec![10, 10],
            10_000,
            100,
            bvec![22, 22],
            false
        ));
        assert_ok!(Marketplace::list_property(
            RuntimeOrigin::signed([0; 32].into()),
            3,
            bvec![10, 10],
            10_000,
            100,
            bvec![22, 22],
            false
        ));

        // Purchase and process second listing
        assert_ok!(Marketplace::buy_property_shares(
            RuntimeOrigin::signed([1; 32].into()),
            1,
            20,
            1984
        ));
        assert_ok!(Marketplace::buy_property_shares(
            RuntimeOrigin::signed([30; 32].into()),
            1,
            30,
            1984
        ));
        assert_ok!(Marketplace::buy_property_shares(
            RuntimeOrigin::signed([31; 32].into()),
            1,
            30,
            1984
        ));
        assert_eq!(PropertyLawyer::<Test>::get(1).is_some(), false);
        assert_ok!(Marketplace::buy_property_shares(
            RuntimeOrigin::signed([1; 32].into()),
            1,
            20,
            1984
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [5; 32].into(),
            pallet_xcavate_whitelist::Role::SpvConfirmation
        ));
        assert_ok!(Marketplace::create_spv(RuntimeOrigin::signed([5; 32].into()), 1,));
        assert_ok!(Marketplace::claim_property_shares(RuntimeOrigin::signed([1; 32].into()), 1));
        assert_ok!(Marketplace::claim_property_shares(RuntimeOrigin::signed([30; 32].into()), 1));
        assert_ok!(Marketplace::claim_property_shares(RuntimeOrigin::signed([31; 32].into()), 1));
        assert_eq!(PropertyLawyer::<Test>::get(1).is_some(), true);
        assert_ok!(Marketplace::lawyer_claim_property(
            RuntimeOrigin::signed([10; 32].into()),
            1,
            crate::LegalProperty::RealEstateDeveloperSide,
            4_000,
        ));
        assert_ok!(Marketplace::approve_developer_lawyer(
            RuntimeOrigin::signed([2; 32].into()),
            1,
            true,
        ));
        assert_ok!(Marketplace::lawyer_claim_property(
            RuntimeOrigin::signed([11; 32].into()),
            1,
            crate::LegalProperty::SpvSide,
            4_000,
        ));
        assert_ok!(Marketplace::vote_on_spv_lawyer(
            RuntimeOrigin::signed([1; 32].into()),
            1,
            crate::Vote::Yes,
            40,
        ));
        assert_ok!(Marketplace::vote_on_spv_lawyer(
            RuntimeOrigin::signed([30; 32].into()),
            1,
            crate::Vote::Yes,
            20,
        ));

        // Purchase third listing
        assert_ok!(Marketplace::buy_property_shares(
            RuntimeOrigin::signed([1; 32].into()),
            2,
            10,
            1984
        ));
        assert_ok!(Marketplace::buy_property_shares(
            RuntimeOrigin::signed([1; 32].into()),
            2,
            10,
            1984
        ));
        assert_ok!(Marketplace::buy_property_shares(
            RuntimeOrigin::signed([2; 32].into()),
            2,
            30,
            1984
        ));

        // List fourth property
        assert_ok!(Marketplace::list_property(
            RuntimeOrigin::signed([15; 32].into()),
            3,
            bvec![10, 10],
            10_000,
            100,
            bvec![22, 22],
            false
        ));

        assert_ok!(Marketplace::buy_property_shares(
            RuntimeOrigin::signed([2; 32].into()),
            0,
            33,
            1984
        ));
        run_to_block(91);
        assert_ok!(Marketplace::finalize_spv_lawyer(RuntimeOrigin::signed([1; 32].into()), 1,),);
        assert_ok!(Marketplace::lawyer_confirm_documents(
            RuntimeOrigin::signed([10; 32].into()),
            1,
            true,
        ));
        System::assert_last_event(
            Event::DocumentsConfirmed {
                signer: [10; 32].into(),
                listing_id: 1,
                legal_side: LegalProperty::RealEstateDeveloperSide,
                approve: true,
            }
            .into(),
        );
        assert_ok!(Marketplace::lawyer_confirm_documents(
            RuntimeOrigin::signed([11; 32].into()),
            1,
            true,
        ));

        // Final assertions
        assert_eq!(OngoingObjectListing::<Test>::get(0).unwrap().listed_share_amount, 67);
        assert_eq!(OngoingObjectListing::<Test>::get(2).unwrap().listed_share_amount, 50);
        assert_eq!(OngoingObjectListing::<Test>::get(3).unwrap().listed_share_amount, 100);
        assert_eq!(
            ShareOwner::<Test>::get::<AccountId, u32>([2; 32].into(), 2).unwrap().share_amount,
            30
        );
        assert_eq!(ShareOwner::<Test>::get::<AccountId, u32>([1; 32].into(), 1), None);
        assert_eq!(PropertyOwnerShares::<Test>::get::<u32, AccountId>(1, [1; 32].into()), 40);
    });
}

// claim_property_shares tests

#[test]
fn claim_property_shares_works() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);
        // Region setup and role assignments
        assert_ok!(XcavateWhitelist::add_admin(RuntimeOrigin::root(), [20; 32].into(),));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [8; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));
        new_region_helper();
        assert_ok!(Regions::create_new_location(
            RuntimeOrigin::signed([8; 32].into()),
            3,
            bvec![10, 10]
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [0; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateDeveloper
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [1; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [2; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [30; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));

        // List property and purchase shares
        assert_ok!(Marketplace::list_property(
            RuntimeOrigin::signed([0; 32].into()),
            3,
            bvec![10, 10],
            10_000,
            100,
            bvec![22, 22],
            false
        ));
        assert_ok!(Marketplace::buy_property_shares(
            RuntimeOrigin::signed([1; 32].into()),
            0,
            40,
            1984
        ));
        assert_ok!(Marketplace::buy_property_shares(
            RuntimeOrigin::signed([2; 32].into()),
            0,
            30,
            1984
        ));
        assert_ok!(Marketplace::buy_property_shares(
            RuntimeOrigin::signed([30; 32].into()),
            0,
            30,
            1984
        ));
        assert_eq!(
            OngoingObjectListing::<Test>::get(0)
                .unwrap()
                .investor_funds
                .get(&[1; 32].into())
                .clone(),
            None
        );
        // Funds should still be on hold before claiming
        assert_eq!(ForeignAssets::balance(1984, &[1; 32].into()), 1_084_000);
        assert_eq!(AssetsHolder::total_balance_on_hold(1984, &[1; 32].into()), 416_000);
        assert_eq!(ForeignAssets::balance(1984, &Marketplace::property_account_id(0)), 0);
        assert_eq!(LocalAssets::balance(0, &[1; 32].into()), 0);
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [5; 32].into(),
            pallet_xcavate_whitelist::Role::SpvConfirmation
        ));
        assert_ok!(Marketplace::create_spv(RuntimeOrigin::signed([5; 32].into()), 0,));

        assert_ok!(Marketplace::claim_property_shares(RuntimeOrigin::signed([1; 32].into()), 0,));
        assert_eq!(
            OngoingObjectListing::<Test>::get(0).unwrap().collected_funds.get(&1984).copied(),
            Some(400_000)
        );
        assert_eq!(
            OngoingObjectListing::<Test>::get(0).unwrap().collected_fees.get(&1984).copied(),
            Some(4_000)
        );
        assert_eq!(
            OngoingObjectListing::<Test>::get(0).unwrap().collected_tax.get(&1984).copied(),
            Some(12_000)
        );
        // Funds are now in the spv account
        assert_eq!(ForeignAssets::balance(1984, &[1; 32].into()), 1_084_000);
        assert_eq!(AssetsHolder::total_balance_on_hold(1984, &[1; 32].into()), 0);
        assert_eq!(ForeignAssets::balance(1984, &Marketplace::property_account_id(0)), 416_000);
        assert_eq!(LocalAssets::balance(0, &[1; 32].into()), 40);
        assert_eq!(LocalAssets::balance(0, &[2; 32].into()), 0);
        assert_eq!(LocalAssets::balance(0, &Marketplace::property_account_id(0)), 60);
        assert_eq!(
            OngoingObjectListing::<Test>::get(0)
                .unwrap()
                .investor_funds
                .get(&[1; 32].into())
                .clone()
                .unwrap()
                .paid_funds
                .get(&1984)
                .unwrap(),
            &412_000_u128
        );
        assert_eq!(
            OngoingObjectListing::<Test>::get(0)
                .unwrap()
                .investor_funds
                .get(&[1; 32].into())
                .clone()
                .unwrap()
                .paid_funds
                .get(&1337),
            None
        );
        assert_eq!(ShareOwner::<Test>::get::<AccountId, u32>([1; 32].into(), 0), None);
        assert_eq!(
            OngoingObjectListing::<Test>::get(0)
                .unwrap()
                .investor_funds
                .get(&[2; 32].into())
                .clone(),
            None
        );
        assert_eq!(PropertyLawyer::<Test>::get(0).is_some(), false);
        assert_ok!(Marketplace::claim_property_shares(RuntimeOrigin::signed([2; 32].into()), 0,));
        assert_ok!(Marketplace::claim_property_shares(RuntimeOrigin::signed([30; 32].into()), 0,));
        assert_eq!(
            OngoingObjectListing::<Test>::get(0).unwrap().collected_funds.get(&1984).copied(),
            Some(1_000_000)
        );
        assert_eq!(
            OngoingObjectListing::<Test>::get(0).unwrap().collected_fees.get(&1984).copied(),
            Some(10_000)
        );
        assert_eq!(
            OngoingObjectListing::<Test>::get(0).unwrap().collected_tax.get(&1984).copied(),
            Some(30_000)
        );
        assert_eq!(LocalAssets::balance(0, &[2; 32].into()), 30);
        assert_eq!(LocalAssets::balance(0, &Marketplace::property_account_id(0)), 0);
        assert_eq!(ForeignAssets::balance(1984, &Marketplace::property_account_id(0)), 1040_000);
        assert_eq!(PropertyLawyer::<Test>::get(0).is_some(), true);
        assert_eq!(
            OngoingObjectListing::<Test>::get(0)
                .unwrap()
                .investor_funds
                .get(&[2; 32].into())
                .clone()
                .unwrap()
                .paid_funds
                .get(&1984)
                .unwrap(),
            &309_000_u128
        );
        System::assert_last_event(
            Event::PropertySharesClaimed {
                listing_id: 0,
                asset_id: 0,
                owner: [30; 32].into(),
                amount: 30,
            }
            .into(),
        );
    })
}

#[test]
fn claim_property_shares_works_2() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);
        // Region setup and role assignments
        assert_ok!(XcavateWhitelist::add_admin(RuntimeOrigin::root(), [20; 32].into(),));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [8; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));
        new_region_helper();
        assert_ok!(Regions::create_new_location(
            RuntimeOrigin::signed([8; 32].into()),
            3,
            bvec![10, 10]
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [0; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateDeveloper
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [1; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [2; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [30; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));

        // List property and purchase shares
        assert_ok!(Marketplace::list_property(
            RuntimeOrigin::signed([0; 32].into()),
            3,
            bvec![10, 10],
            10_000,
            100,
            bvec![22, 22],
            false
        ));
        assert_ok!(Marketplace::buy_property_shares(
            RuntimeOrigin::signed([1; 32].into()),
            0,
            40,
            1984
        ));
        assert_ok!(Marketplace::buy_property_shares(
            RuntimeOrigin::signed([2; 32].into()),
            0,
            30,
            1984
        ));
        assert_ok!(Marketplace::buy_property_shares(
            RuntimeOrigin::signed([30; 32].into()),
            0,
            30,
            1984
        ));
        assert_eq!(
            OngoingObjectListing::<Test>::get(0)
                .unwrap()
                .investor_funds
                .get(&[1; 32].into())
                .clone(),
            None
        );
        assert_eq!(ForeignAssets::balance(1984, &[1; 32].into()), 1_084_000);
        assert_eq!(AssetsHolder::total_balance_on_hold(1984, &[1; 32].into()), 416_000);
        assert_eq!(ForeignAssets::balance(1984, &Marketplace::property_account_id(0)), 0);
        assert_eq!(LocalAssets::balance(0, &[1; 32].into()), 0);
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [5; 32].into(),
            pallet_xcavate_whitelist::Role::SpvConfirmation
        ));
        assert_ok!(Marketplace::create_spv(RuntimeOrigin::signed([5; 32].into()), 0,));
        assert_ok!(Marketplace::claim_property_shares(RuntimeOrigin::signed([1; 32].into()), 0,));
        assert_ok!(Marketplace::claim_property_shares(RuntimeOrigin::signed([30; 32].into()), 0,));
        assert_eq!(
            OngoingObjectListing::<Test>::get(0).unwrap().collected_funds.get(&1984).copied(),
            Some(700_000)
        );
        assert_eq!(
            OngoingObjectListing::<Test>::get(0).unwrap().collected_fees.get(&1984).copied(),
            Some(7_000)
        );
        assert_eq!(
            OngoingObjectListing::<Test>::get(0).unwrap().collected_tax.get(&1984).copied(),
            Some(21_000)
        );
        assert_eq!(PropertyOwnerShares::<Test>::get::<u32, AccountId>(0, [1; 32].into()), 40);
        assert_eq!(ForeignAssets::balance(1984, &[1; 32].into()), 1_084_000);
        assert_eq!(AssetsHolder::total_balance_on_hold(1984, &[1; 32].into()), 0);
        assert_eq!(ForeignAssets::balance(1984, &Marketplace::property_account_id(0)), 728_000);
        assert_eq!(LocalAssets::balance(0, &[1; 32].into()), 40);
        assert_eq!(LocalAssets::balance(0, &[2; 32].into()), 0);
        assert_eq!(LocalAssets::balance(0, &[30; 32].into()), 30);
        assert_eq!(LocalAssets::balance(0, &Marketplace::property_account_id(0)), 30);
        assert_eq!(
            OngoingObjectListing::<Test>::get(0)
                .unwrap()
                .investor_funds
                .get(&[1; 32].into())
                .clone()
                .unwrap()
                .paid_funds
                .get(&1984)
                .unwrap(),
            &412_000_u128
        );
        assert_eq!(
            OngoingObjectListing::<Test>::get(0)
                .unwrap()
                .investor_funds
                .get(&[1; 32].into())
                .clone()
                .unwrap()
                .paid_funds
                .get(&1337),
            None
        );
        assert_eq!(
            OngoingObjectListing::<Test>::get(0)
                .unwrap()
                .investor_funds
                .get(&[30; 32].into())
                .clone()
                .unwrap()
                .paid_funds
                .get(&1984)
                .unwrap(),
            &309_000_u128
        );
        assert_eq!(ShareOwner::<Test>::get::<AccountId, u32>([1; 32].into(), 0), None);
        assert_eq!(
            OngoingObjectListing::<Test>::get(0)
                .unwrap()
                .investor_funds
                .get(&[2; 32].into())
                .clone(),
            None
        );
        let expiry = frame_system::Pallet::<Test>::block_number() + ClaimWindowTime::get() + 1;
        run_to_block(expiry);
        // Not all shares have been claimed therefore unclaimed shares can be relisted on the marketplace
        assert_ok!(Marketplace::finalize_claim_window(RuntimeOrigin::signed([1; 32].into()), 0,));
        assert_eq!(ShareOwner::<Test>::get::<AccountId, u32>([30; 32].into(), 0), None);
        assert_ok!(Marketplace::buy_property_shares(
            RuntimeOrigin::signed([30; 32].into()),
            0,
            10,
            1984
        ));
        assert_eq!(
            ShareOwner::<Test>::get::<AccountId, u32>([30; 32].into(), 0).unwrap().share_amount,
            10
        );
        // An investor that did not claim his shares on time can unlock his locked funds.
        // To buy property shares the investor needs to unlock his funds first.
        assert_ok!(Marketplace::withdraw_unclaimed(RuntimeOrigin::signed([2; 32].into()), 0,));
        assert_ok!(Marketplace::buy_property_shares(
            RuntimeOrigin::signed([2; 32].into()),
            0,
            20,
            1984
        ));
        assert_ok!(Marketplace::claim_property_shares(RuntimeOrigin::signed([30; 32].into()), 0,));
        assert_eq!(
            OngoingObjectListing::<Test>::get(0).unwrap().collected_funds.get(&1984).copied(),
            Some(800_000)
        );
        assert_eq!(
            OngoingObjectListing::<Test>::get(0).unwrap().collected_fees.get(&1984).copied(),
            Some(8_000)
        );
        assert_eq!(
            OngoingObjectListing::<Test>::get(0).unwrap().collected_tax.get(&1984).copied(),
            Some(24_000)
        );
        assert_eq!(PropertyOwnerShares::<Test>::get::<u32, AccountId>(0, [1; 32].into()), 40);
        assert_ok!(Marketplace::claim_property_shares(RuntimeOrigin::signed([2; 32].into()), 0,));
        assert_eq!(PropertyOwnerShares::<Test>::get::<u32, AccountId>(0, [2; 32].into()), 20);
        assert_eq!(
            OngoingObjectListing::<Test>::get(0)
                .unwrap()
                .investor_funds
                .get(&[30; 32].into())
                .clone()
                .unwrap()
                .paid_funds
                .get(&1984)
                .unwrap(),
            &412_000_u128
        );
    })
}

#[test]
fn claim_property_shares_fails() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);
        // Region setup and role assignments
        assert_ok!(XcavateWhitelist::add_admin(RuntimeOrigin::root(), [20; 32].into(),));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [8; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));
        new_region_helper();
        assert_ok!(Regions::create_new_location(
            RuntimeOrigin::signed([8; 32].into()),
            3,
            bvec![10, 10]
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [0; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateDeveloper
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [1; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [2; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [30; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [31; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [6; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));
        assert_noop!(
            Marketplace::claim_property_shares(RuntimeOrigin::signed([1; 32].into()), 0,),
            Error::<Test>::ListingNotFound
        );

        // List property and purchase shares
        assert_ok!(Marketplace::list_property(
            RuntimeOrigin::signed([0; 32].into()),
            3,
            bvec![10, 10],
            10_000,
            100,
            bvec![22, 22],
            false
        ));
        assert_ok!(Marketplace::buy_property_shares(
            RuntimeOrigin::signed([1; 32].into()),
            0,
            48,
            1984
        ));
        assert_ok!(Marketplace::buy_property_shares(
            RuntimeOrigin::signed([30; 32].into()),
            0,
            25,
            1984
        ));
        assert_ok!(Marketplace::buy_property_shares(
            RuntimeOrigin::signed([31; 32].into()),
            0,
            26,
            1984
        ));
        assert_noop!(
            Marketplace::claim_property_shares(RuntimeOrigin::signed([1; 32].into()), 0,),
            RealWorldAssetError::<Test>::SpvNotCreated
        );
        assert_ok!(Marketplace::buy_property_shares(
            RuntimeOrigin::signed([6; 32].into()),
            0,
            1,
            1984
        ));
        assert_noop!(
            Marketplace::claim_property_shares(RuntimeOrigin::signed([1; 32].into()), 0,),
            RealWorldAssetError::<Test>::SpvNotCreated
        );
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [5; 32].into(),
            pallet_xcavate_whitelist::Role::SpvConfirmation
        ));
        assert_ok!(Marketplace::create_spv(RuntimeOrigin::signed([5; 32].into()), 0,));
        assert_noop!(
            Marketplace::claim_property_shares(RuntimeOrigin::signed([3; 32].into()), 0,),
            BadOrigin
        );
        assert_noop!(
            Marketplace::claim_property_shares(RuntimeOrigin::signed([2; 32].into()), 0,),
            Error::<Test>::ShareOwnerNotFound
        );
        assert_ok!(Marketplace::claim_property_shares(RuntimeOrigin::signed([1; 32].into()), 0,));
        assert_ok!(Marketplace::claim_property_shares(RuntimeOrigin::signed([30; 32].into()), 0,));
        assert_ok!(Marketplace::claim_property_shares(RuntimeOrigin::signed([31; 32].into()), 0,));
        assert_noop!(
            Marketplace::claim_property_shares(RuntimeOrigin::signed([1; 32].into()), 0,),
            Error::<Test>::ShareOwnerNotFound
        );
        let expiry = frame_system::Pallet::<Test>::block_number() + ClaimWindowTime::get() + 1;
        run_to_block(expiry);
        assert_noop!(
            Marketplace::claim_property_shares(RuntimeOrigin::signed([1; 32].into()), 0,),
            Error::<Test>::ClaimWindowExpired
        );
        assert_ok!(Marketplace::finalize_claim_window(RuntimeOrigin::signed([1; 32].into()), 0,));
        assert_noop!(
            Marketplace::claim_property_shares(RuntimeOrigin::signed([1; 32].into()), 0,),
            Error::<Test>::NoClaimWindow
        );
        assert_ok!(Marketplace::buy_property_shares(
            RuntimeOrigin::signed([1; 32].into()),
            0,
            1,
            1984
        ));
        assert_noop!(
            Marketplace::claim_property_shares(RuntimeOrigin::signed([6; 32].into()), 0,),
            Error::<Test>::NoValidSharesToClaim
        );
        assert_ok!(XcavateWhitelist::set_permission(
            RuntimeOrigin::signed([20; 32].into()),
            [1; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor,
            pallet_xcavate_whitelist::AccessPermission::Revoked,
        ));
        assert_noop!(
            Marketplace::claim_property_shares(RuntimeOrigin::signed([1; 32].into()), 0,),
            BadOrigin
        );
    })
}

#[test]
fn relist_unclaimed_property_shares_works() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);
        // Region setup and role assignments
        assert_ok!(XcavateWhitelist::add_admin(RuntimeOrigin::root(), [20; 32].into(),));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [8; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));
        new_region_helper();
        assert_ok!(Regions::create_new_location(
            RuntimeOrigin::signed([8; 32].into()),
            3,
            bvec![10, 10]
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [0; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateDeveloper
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [1; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [2; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [6; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [30; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));

        // List property and purchase shares
        assert_ok!(Marketplace::list_property(
            RuntimeOrigin::signed([0; 32].into()),
            3,
            bvec![10, 10],
            10_000,
            100,
            bvec![22, 22],
            false
        ));
        assert_ok!(Marketplace::buy_property_shares(
            RuntimeOrigin::signed([1; 32].into()),
            0,
            40,
            1984
        ));
        assert_ok!(Marketplace::buy_property_shares(
            RuntimeOrigin::signed([2; 32].into()),
            0,
            30,
            1984
        ));
        assert_ok!(Marketplace::buy_property_shares(
            RuntimeOrigin::signed([30; 32].into()),
            0,
            30,
            1984
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [5; 32].into(),
            pallet_xcavate_whitelist::Role::SpvConfirmation
        ));
        assert_ok!(Marketplace::create_spv(RuntimeOrigin::signed([5; 32].into()), 0,));
        assert_ok!(Marketplace::claim_property_shares(RuntimeOrigin::signed([1; 32].into()), 0,));
        assert_ok!(Marketplace::claim_property_shares(RuntimeOrigin::signed([30; 32].into()), 0,));
        assert_eq!(OngoingObjectListing::<Test>::get(0).unwrap().listed_share_amount, 0);
        assert_eq!(OngoingObjectListing::<Test>::get(0).unwrap().unclaimed_share_amount, 30);
        let expiry = frame_system::Pallet::<Test>::block_number() + ClaimWindowTime::get() + 1;
        run_to_block(expiry);
        assert_ok!(Marketplace::finalize_claim_window(RuntimeOrigin::signed([1; 32].into()), 0,));
        assert_eq!(OngoingObjectListing::<Test>::get(0).unwrap().listed_share_amount, 30);
        assert_eq!(
            OngoingObjectListing::<Test>::get(0).unwrap().listing_expiry,
            frame_system::Pallet::<Test>::block_number() + ClaimWindowTime::get()
        );
        assert_ok!(Marketplace::buy_property_shares(
            RuntimeOrigin::signed([6; 32].into()),
            0,
            30,
            1984
        ));
        assert_ok!(Marketplace::claim_property_shares(RuntimeOrigin::signed([6; 32].into()), 0,));
        assert_eq!(
            OngoingObjectListing::<Test>::get(0).unwrap().collected_funds.get(&1984).copied(),
            Some(1_000_000)
        );
        assert_eq!(
            OngoingObjectListing::<Test>::get(0).unwrap().collected_fees.get(&1984).copied(),
            Some(10_000)
        );
        assert_eq!(
            OngoingObjectListing::<Test>::get(0).unwrap().collected_tax.get(&1984).copied(),
            Some(30_000)
        );
        assert_eq!(LocalAssets::balance(0, &[6; 32].into()), 30);
    })
}

// finalize_claim_window tests

#[test]
fn finalize_claim_window_works() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);
        assert_ok!(XcavateWhitelist::add_admin(RuntimeOrigin::root(), [20; 32].into(),));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [8; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));
        new_region_helper();
        assert_ok!(Regions::create_new_location(
            RuntimeOrigin::signed([8; 32].into()),
            3,
            bvec![10, 10]
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [0; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateDeveloper
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [1; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [2; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [30; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));
        assert_ok!(Marketplace::list_property(
            RuntimeOrigin::signed([0; 32].into()),
            3,
            bvec![10, 10],
            10_000,
            100,
            bvec![22, 22],
            false
        ));
        assert_ok!(Marketplace::buy_property_shares(
            RuntimeOrigin::signed([1; 32].into()),
            0,
            40,
            1984
        ));
        assert_ok!(Marketplace::buy_property_shares(
            RuntimeOrigin::signed([2; 32].into()),
            0,
            30,
            1984
        ));
        assert_ok!(Marketplace::buy_property_shares(
            RuntimeOrigin::signed([30; 32].into()),
            0,
            30,
            1984
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [5; 32].into(),
            pallet_xcavate_whitelist::Role::SpvConfirmation
        ));
        assert_ok!(Marketplace::create_spv(RuntimeOrigin::signed([5; 32].into()), 0,));
        assert_ok!(Marketplace::claim_property_shares(RuntimeOrigin::signed([1; 32].into()), 0,));
        assert_ok!(Marketplace::claim_property_shares(RuntimeOrigin::signed([30; 32].into()), 0,));
        assert_eq!(OngoingObjectListing::<Test>::get(0).unwrap().listed_share_amount, 0);
        assert_eq!(OngoingObjectListing::<Test>::get(0).unwrap().unclaimed_share_amount, 30);
        let expiry = frame_system::Pallet::<Test>::block_number() + ClaimWindowTime::get() + 1;
        run_to_block(expiry);
        assert_ok!(Marketplace::finalize_claim_window(RuntimeOrigin::signed([3; 32].into()), 0,));
        assert_eq!(OngoingObjectListing::<Test>::get(0).unwrap().listed_share_amount, 30);
        assert_eq!(
            OngoingObjectListing::<Test>::get(0).unwrap().listing_expiry,
            frame_system::Pallet::<Test>::block_number() + ClaimWindowTime::get()
        );
        assert_eq!(OngoingObjectListing::<Test>::get(0).unwrap().relist_count, 1);
        assert_eq!(OngoingObjectListing::<Test>::get(0).unwrap().unclaimed_share_amount, 0);
        assert_ok!(Marketplace::buy_property_shares(
            RuntimeOrigin::signed([30; 32].into()),
            0,
            15,
            1984
        ));
    })
}

#[test]
fn finalize_claim_window_fails() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);
        assert_ok!(XcavateWhitelist::add_admin(RuntimeOrigin::root(), [20; 32].into(),));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [8; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));
        new_region_helper();
        assert_ok!(Regions::create_new_location(
            RuntimeOrigin::signed([8; 32].into()),
            3,
            bvec![10, 10]
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [0; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateDeveloper
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [1; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [2; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [30; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));
        assert_noop!(
            Marketplace::finalize_claim_window(RuntimeOrigin::signed([1; 32].into()), 0,),
            Error::<Test>::ListingNotFound
        );
        assert_ok!(Marketplace::list_property(
            RuntimeOrigin::signed([0; 32].into()),
            3,
            bvec![10, 10],
            10_000,
            100,
            bvec![22, 22],
            false
        ));
        assert_noop!(
            Marketplace::finalize_claim_window(RuntimeOrigin::signed([1; 32].into()), 0,),
            Error::<Test>::NoClaimWindow
        );
        assert_ok!(Marketplace::buy_property_shares(
            RuntimeOrigin::signed([1; 32].into()),
            0,
            40,
            1984
        ));
        assert_ok!(Marketplace::buy_property_shares(
            RuntimeOrigin::signed([2; 32].into()),
            0,
            30,
            1984
        ));
        assert_ok!(Marketplace::buy_property_shares(
            RuntimeOrigin::signed([30; 32].into()),
            0,
            30,
            1984
        ));
        assert_noop!(
            Marketplace::finalize_claim_window(RuntimeOrigin::signed([1; 32].into()), 0,),
            Error::<Test>::NoClaimWindow
        );
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [5; 32].into(),
            pallet_xcavate_whitelist::Role::SpvConfirmation
        ));
        assert_ok!(Marketplace::create_spv(RuntimeOrigin::signed([5; 32].into()), 0,));
        assert_noop!(
            Marketplace::finalize_claim_window(RuntimeOrigin::signed([1; 32].into()), 0,),
            Error::<Test>::ClaimWindowNotExpired
        );
        assert_ok!(Marketplace::claim_property_shares(RuntimeOrigin::signed([1; 32].into()), 0,));
        assert_ok!(Marketplace::claim_property_shares(RuntimeOrigin::signed([30; 32].into()), 0,));
        let expiry = frame_system::Pallet::<Test>::block_number() + ClaimWindowTime::get() + 1;
        run_to_block(expiry);
        assert_ok!(Marketplace::finalize_claim_window(RuntimeOrigin::signed([1; 32].into()), 0,));
        assert_eq!(OngoingObjectListing::<Test>::get(0).unwrap().relist_count, 1);
        assert_ok!(Marketplace::withdraw_unclaimed(RuntimeOrigin::signed([2; 32].into()), 0,));
        assert_ok!(Marketplace::buy_property_shares(
            RuntimeOrigin::signed([2; 32].into()),
            0,
            30,
            1984
        ));
        let expiry = frame_system::Pallet::<Test>::block_number() + ClaimWindowTime::get() + 1;
        run_to_block(expiry);
        assert_ok!(Marketplace::finalize_claim_window(RuntimeOrigin::signed([1; 32].into()), 0,));
        assert_eq!(OngoingObjectListing::<Test>::get(0).unwrap().relist_count, 2);
        assert_noop!(
            Marketplace::finalize_claim_window(RuntimeOrigin::signed([1; 32].into()), 0,),
            Error::<Test>::NoClaimWindow
        );
    })
}

#[test]
fn finalize_claim_window_fails_2() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);
        assert_ok!(XcavateWhitelist::add_admin(RuntimeOrigin::root(), [20; 32].into(),));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [8; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));
        new_region_helper();
        assert_ok!(Regions::create_new_location(
            RuntimeOrigin::signed([8; 32].into()),
            3,
            bvec![10, 10]
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [0; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateDeveloper
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [1; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [2; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [30; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));
        assert_noop!(
            Marketplace::finalize_claim_window(RuntimeOrigin::signed([1; 32].into()), 0,),
            Error::<Test>::ListingNotFound
        );
        assert_ok!(Marketplace::list_property(
            RuntimeOrigin::signed([0; 32].into()),
            3,
            bvec![10, 10],
            10_000,
            100,
            bvec![22, 22],
            false
        ));
        assert_noop!(
            Marketplace::finalize_claim_window(RuntimeOrigin::signed([1; 32].into()), 0,),
            Error::<Test>::NoClaimWindow
        );
        assert_ok!(Marketplace::buy_property_shares(
            RuntimeOrigin::signed([1; 32].into()),
            0,
            40,
            1984
        ));
        assert_ok!(Marketplace::buy_property_shares(
            RuntimeOrigin::signed([2; 32].into()),
            0,
            30,
            1984
        ));
        assert_ok!(Marketplace::buy_property_shares(
            RuntimeOrigin::signed([30; 32].into()),
            0,
            30,
            1984
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [5; 32].into(),
            pallet_xcavate_whitelist::Role::SpvConfirmation
        ));
        assert_ok!(Marketplace::create_spv(RuntimeOrigin::signed([5; 32].into()), 0,));
        assert_ok!(Marketplace::claim_property_shares(RuntimeOrigin::signed([1; 32].into()), 0,));
        assert_ok!(Marketplace::claim_property_shares(RuntimeOrigin::signed([2; 32].into()), 0,));
        assert_ok!(Marketplace::claim_property_shares(RuntimeOrigin::signed([30; 32].into()), 0,));
        assert_noop!(
            Marketplace::finalize_claim_window(RuntimeOrigin::signed([1; 32].into()), 0,),
            Error::<Test>::NoClaimWindow
        );
    })
}

// create_spv tests

#[test]
fn create_spv_works() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);
        assert_ok!(XcavateWhitelist::add_admin(RuntimeOrigin::root(), [20; 32].into(),));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [8; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));
        new_region_helper();
        assert_ok!(Regions::create_new_location(
            RuntimeOrigin::signed([8; 32].into()),
            3,
            bvec![10, 10]
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [0; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateDeveloper
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [1; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [30; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [31; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));
        assert_ok!(Marketplace::list_property(
            RuntimeOrigin::signed([0; 32].into()),
            3,
            bvec![10, 10],
            10_000,
            100,
            bvec![22, 22],
            false
        ));
        assert_ok!(Marketplace::buy_property_shares(
            RuntimeOrigin::signed([1; 32].into()),
            0,
            45,
            1984
        ));
        assert_ok!(Marketplace::buy_property_shares(
            RuntimeOrigin::signed([30; 32].into()),
            0,
            30,
            1984
        ));
        assert_ok!(Marketplace::buy_property_shares(
            RuntimeOrigin::signed([31; 32].into()),
            0,
            25,
            1984
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [2; 32].into(),
            pallet_xcavate_whitelist::Role::SpvConfirmation
        ));
        assert_eq!(PropertyAssetInfo::<Test>::get(0).unwrap().spv_created, false);
        let expiry = frame_system::Pallet::<Test>::block_number() + ClaimWindowTime::get();
        assert_ok!(Marketplace::create_spv(RuntimeOrigin::signed([2; 32].into()), 0,));
        assert_eq!(PropertyAssetInfo::<Test>::get(0).unwrap().spv_created, true);
        assert_eq!(OngoingObjectListing::<Test>::get(0).unwrap().claim_expiry, Some(expiry));
    })
}

#[test]
fn create_spv_fails() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);
        assert_ok!(XcavateWhitelist::add_admin(RuntimeOrigin::root(), [20; 32].into(),));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [8; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));
        new_region_helper();
        assert_ok!(Regions::create_new_location(
            RuntimeOrigin::signed([8; 32].into()),
            3,
            bvec![10, 10]
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [0; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateDeveloper
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [1; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [30; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [31; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [2; 32].into(),
            pallet_xcavate_whitelist::Role::SpvConfirmation
        ));
        assert_noop!(
            Marketplace::create_spv(RuntimeOrigin::signed([2; 32].into()), 0,),
            Error::<Test>::NoObjectFound
        );
        assert_ok!(Marketplace::list_property(
            RuntimeOrigin::signed([0; 32].into()),
            3,
            bvec![10, 10],
            10_000,
            100,
            bvec![22, 22],
            false
        ));
        assert_ok!(Marketplace::buy_property_shares(
            RuntimeOrigin::signed([1; 32].into()),
            0,
            48,
            1984
        ));
        assert_ok!(Marketplace::buy_property_shares(
            RuntimeOrigin::signed([30; 32].into()),
            0,
            25,
            1984
        ));
        assert_ok!(Marketplace::buy_property_shares(
            RuntimeOrigin::signed([31; 32].into()),
            0,
            26,
            1984
        ));
        assert_noop!(
            Marketplace::create_spv(RuntimeOrigin::signed([2; 32].into()), 0,),
            Error::<Test>::PropertyHasNotBeenSoldYet
        );
        assert_ok!(Marketplace::buy_property_shares(
            RuntimeOrigin::signed([1; 32].into()),
            0,
            1,
            1984
        ));
        assert_noop!(Marketplace::create_spv(RuntimeOrigin::signed([1; 32].into()), 0,), BadOrigin);
    })
}

// lawyer_claim_property tests

#[test]
fn claim_property_works1() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);

        // Region setup and role assignments
        assert_ok!(XcavateWhitelist::add_admin(RuntimeOrigin::root(), [20; 32].into(),));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [8; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));
        new_region_helper();
        assert_ok!(Regions::create_new_location(
            RuntimeOrigin::signed([8; 32].into()),
            3,
            bvec![10, 10]
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [0; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateDeveloper
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [1; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [30; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [31; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [10; 32].into(),
            pallet_xcavate_whitelist::Role::Lawyer
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [11; 32].into(),
            pallet_xcavate_whitelist::Role::Lawyer
        ));
        assert_ok!(Regions::register_lawyer(RuntimeOrigin::signed([10; 32].into()), 3,));
        assert_ok!(Regions::register_lawyer(RuntimeOrigin::signed([11; 32].into()), 3,));

        // Listing and purchasing property shares
        assert_ok!(Marketplace::list_property(
            RuntimeOrigin::signed([0; 32].into()),
            3,
            bvec![10, 10],
            10_000,
            100,
            bvec![22, 22],
            false
        ));
        assert_ok!(Marketplace::buy_property_shares(
            RuntimeOrigin::signed([1; 32].into()),
            0,
            45,
            1984
        ));
        assert_ok!(Marketplace::buy_property_shares(
            RuntimeOrigin::signed([30; 32].into()),
            0,
            30,
            1984
        ));
        assert_ok!(Marketplace::buy_property_shares(
            RuntimeOrigin::signed([31; 32].into()),
            0,
            25,
            1984
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [5; 32].into(),
            pallet_xcavate_whitelist::Role::SpvConfirmation
        ));
        assert_ok!(Marketplace::create_spv(RuntimeOrigin::signed([5; 32].into()), 0,));
        assert_ok!(Marketplace::claim_property_shares(RuntimeOrigin::signed([1; 32].into()), 0,));
        assert_ok!(Marketplace::claim_property_shares(RuntimeOrigin::signed([30; 32].into()), 0,));
        assert_ok!(Marketplace::claim_property_shares(RuntimeOrigin::signed([31; 32].into()), 0,));

        // Legal process
        assert_ok!(Marketplace::lawyer_claim_property(
            RuntimeOrigin::signed([10; 32].into()),
            0,
            crate::LegalProperty::RealEstateDeveloperSide,
            4_000,
        ));
        assert_eq!(ProposedLawyers::<Test>::get(0).unwrap().lawyer, [10; 32].into());
        assert_eq!(ProposedLawyers::<Test>::get(0).unwrap().costs, 4_000);
        assert_eq!(PropertyLawyer::<Test>::get(0).unwrap().real_estate_developer_lawyer, None);
        assert_ok!(Marketplace::lawyer_claim_property(
            RuntimeOrigin::signed([11; 32].into()),
            0,
            crate::LegalProperty::SpvSide,
            4_000,
        ));
        assert_eq!(SpvLawyerProposal::<Test>::get(0).unwrap().lawyer, [11; 32].into());
        assert_eq!(SpvLawyerProposal::<Test>::get(0).unwrap().expiry_block, 91);
        assert_eq!(OngoingLawyerVoting::<Test>::get(0).is_some(), true);
        assert_eq!(PropertyLawyer::<Test>::get(0).unwrap().spv_lawyer, None);
        assert_eq!(ListingSpvProposal::<Test>::get(0).unwrap(), 0);
    })
}

#[test]
fn claim_property_works2() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);

        // Region setup and role assignments
        assert_ok!(XcavateWhitelist::add_admin(RuntimeOrigin::root(), [20; 32].into(),));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [8; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));
        new_region_helper();
        assert_ok!(Regions::create_new_location(
            RuntimeOrigin::signed([8; 32].into()),
            3,
            bvec![10, 10]
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [0; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateDeveloper
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [1; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [2; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [30; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [9; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [10; 32].into(),
            pallet_xcavate_whitelist::Role::Lawyer
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [11; 32].into(),
            pallet_xcavate_whitelist::Role::Lawyer
        ));
        assert_ok!(Regions::register_lawyer(RuntimeOrigin::signed([10; 32].into()), 3,));
        assert_ok!(Regions::register_lawyer(RuntimeOrigin::signed([11; 32].into()), 3,));

        // Property listing and purchases
        assert_ok!(Marketplace::list_property(
            RuntimeOrigin::signed([0; 32].into()),
            3,
            bvec![10, 10],
            10_000,
            200,
            bvec![22, 22],
            false
        ));
        assert_ok!(Marketplace::buy_property_shares(
            RuntimeOrigin::signed([1; 32].into()),
            0,
            1,
            1984
        ));
        assert_ok!(Marketplace::buy_property_shares(
            RuntimeOrigin::signed([9; 32].into()),
            0,
            98,
            1337
        ));
        assert_ok!(Marketplace::buy_property_shares(
            RuntimeOrigin::signed([2; 32].into()),
            0,
            50,
            1337
        ));
        assert_ok!(Marketplace::buy_property_shares(
            RuntimeOrigin::signed([30; 32].into()),
            0,
            51,
            1337
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [5; 32].into(),
            pallet_xcavate_whitelist::Role::SpvConfirmation
        ));
        assert_ok!(Marketplace::create_spv(RuntimeOrigin::signed([5; 32].into()), 0,));
        assert_ok!(Marketplace::claim_property_shares(RuntimeOrigin::signed([1; 32].into()), 0,));
        assert_ok!(Marketplace::claim_property_shares(RuntimeOrigin::signed([9; 32].into()), 0,));
        assert_ok!(Marketplace::claim_property_shares(RuntimeOrigin::signed([2; 32].into()), 0,));
        assert_ok!(Marketplace::claim_property_shares(RuntimeOrigin::signed([30; 32].into()), 0,));

        // Legal process
        assert_ok!(Marketplace::lawyer_claim_property(
            RuntimeOrigin::signed([10; 32].into()),
            0,
            crate::LegalProperty::RealEstateDeveloperSide,
            15_000,
        ));
        assert_ok!(Marketplace::approve_developer_lawyer(
            RuntimeOrigin::signed([0; 32].into()),
            0,
            true
        ));
        assert_eq!(
            PropertyLawyer::<Test>::get(0).unwrap().real_estate_developer_lawyer,
            Some([10; 32].into())
        );
        assert_ok!(Marketplace::lawyer_claim_property(
            RuntimeOrigin::signed([11; 32].into()),
            0,
            crate::LegalProperty::SpvSide,
            16_000,
        ));
        assert_ok!(Marketplace::vote_on_spv_lawyer(
            RuntimeOrigin::signed([1; 32].into()),
            0,
            crate::Vote::Yes,
            1
        ));
        assert_ok!(Marketplace::vote_on_spv_lawyer(
            RuntimeOrigin::signed([9; 32].into()),
            0,
            crate::Vote::Abstain,
            98
        ));
        assert_ok!(Marketplace::vote_on_spv_lawyer(
            RuntimeOrigin::signed([2; 32].into()),
            0,
            crate::Vote::Yes,
            45
        ));
        run_to_block(91);
        assert_ok!(Marketplace::finalize_spv_lawyer(RuntimeOrigin::signed([1; 32].into()), 0,));
        assert_eq!(PropertyLawyer::<Test>::get(0).unwrap().spv_lawyer, Some([11; 32].into()));
        assert_eq!(
            PropertyLawyer::<Test>::get(0).unwrap().spv_lawyer_costs.get(&1984).unwrap(),
            &0u128
        );
        assert_eq!(
            PropertyLawyer::<Test>::get(0).unwrap().spv_lawyer_costs.get(&1337).unwrap(),
            &16_000u128
        );
        assert_eq!(
            PropertyLawyer::<Test>::get(0)
                .unwrap()
                .real_estate_developer_lawyer_costs
                .get(&1984)
                .unwrap(),
            &0u128
        );
        assert_eq!(
            PropertyLawyer::<Test>::get(0)
                .unwrap()
                .real_estate_developer_lawyer_costs
                .get(&1337)
                .unwrap(),
            &15_000u128
        );
    })
}

#[test]
fn claim_property_fails() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);

        // Region setup and role assignments
        assert_ok!(XcavateWhitelist::add_admin(RuntimeOrigin::root(), [20; 32].into(),));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [8; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));
        new_region_helper();
        assert_ok!(Regions::create_new_location(
            RuntimeOrigin::signed([8; 32].into()),
            3,
            bvec![10, 10]
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [0; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateDeveloper
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [1; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [2; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [30; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [10; 32].into(),
            pallet_xcavate_whitelist::Role::Lawyer
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [11; 32].into(),
            pallet_xcavate_whitelist::Role::Lawyer
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [12; 32].into(),
            pallet_xcavate_whitelist::Role::Lawyer
        ));
        assert_ok!(Regions::register_lawyer(RuntimeOrigin::signed([10; 32].into()), 3,));
        assert_ok!(Regions::register_lawyer(RuntimeOrigin::signed([11; 32].into()), 3,));

        // First Property listing and purchases
        assert_ok!(Marketplace::list_property(
            RuntimeOrigin::signed([0; 32].into()),
            3,
            bvec![10, 10],
            10_000,
            100,
            bvec![22, 22],
            false
        ));
        assert_ok!(Marketplace::buy_property_shares(
            RuntimeOrigin::signed([1; 32].into()),
            0,
            40,
            1984
        ));
        assert_ok!(Marketplace::buy_property_shares(
            RuntimeOrigin::signed([2; 32].into()),
            0,
            30,
            1984
        ));
        assert_ok!(Marketplace::buy_property_shares(
            RuntimeOrigin::signed([30; 32].into()),
            0,
            29,
            1984
        ));
        assert_noop!(
            Marketplace::lawyer_claim_property(
                RuntimeOrigin::signed([10; 32].into()),
                0,
                crate::LegalProperty::RealEstateDeveloperSide,
                4_000,
            ),
            Error::<Test>::InvalidIndex
        );
        assert_ok!(Marketplace::buy_property_shares(
            RuntimeOrigin::signed([1; 32].into()),
            0,
            1,
            1984
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [5; 32].into(),
            pallet_xcavate_whitelist::Role::SpvConfirmation
        ));
        assert_ok!(Marketplace::create_spv(RuntimeOrigin::signed([5; 32].into()), 0,));
        assert_ok!(Marketplace::claim_property_shares(RuntimeOrigin::signed([1; 32].into()), 0,));
        assert_ok!(Marketplace::claim_property_shares(RuntimeOrigin::signed([2; 32].into()), 0,));
        assert_ok!(Marketplace::claim_property_shares(RuntimeOrigin::signed([30; 32].into()), 0,));

        // Legal process failure cases
        assert_noop!(
            Marketplace::lawyer_claim_property(
                RuntimeOrigin::signed([9; 32].into()),
                0,
                crate::LegalProperty::RealEstateDeveloperSide,
                4_000,
            ),
            BadOrigin
        );
        assert_noop!(
            Marketplace::lawyer_claim_property(
                RuntimeOrigin::signed([10; 32].into()),
                0,
                crate::LegalProperty::RealEstateDeveloperSide,
                11_000,
            ),
            Error::<Test>::CostsTooHigh
        );
        assert_ok!(Marketplace::lawyer_claim_property(
            RuntimeOrigin::signed([10; 32].into()),
            0,
            crate::LegalProperty::RealEstateDeveloperSide,
            4_000,
        ));
        assert_eq!(ProposedLawyers::<Test>::get(0).unwrap().lawyer, [10; 32].into());
        assert_noop!(
            Marketplace::lawyer_claim_property(
                RuntimeOrigin::signed([11; 32].into()),
                0,
                crate::LegalProperty::RealEstateDeveloperSide,
                4_000,
            ),
            Error::<Test>::LawyerProposalOngoing
        );
        assert_noop!(
            Marketplace::lawyer_claim_property(
                RuntimeOrigin::signed([10; 32].into()),
                0,
                crate::LegalProperty::SpvSide,
                4_000,
            ),
            Error::<Test>::NoPermission
        );
        assert_ok!(Marketplace::approve_developer_lawyer(
            RuntimeOrigin::signed([0; 32].into()),
            0,
            true,
        ));
        assert_eq!(
            PropertyLawyer::<Test>::get(0).unwrap().real_estate_developer_lawyer,
            Some([10; 32].into())
        );
        assert_noop!(
            Marketplace::lawyer_claim_property(
                RuntimeOrigin::signed([11; 32].into()),
                0,
                crate::LegalProperty::RealEstateDeveloperSide,
                4_000,
            ),
            Error::<Test>::LawyerJobTaken
        );
        assert_noop!(
            Marketplace::lawyer_claim_property(
                RuntimeOrigin::signed([10; 32].into()),
                0,
                crate::LegalProperty::SpvSide,
                4_000,
            ),
            Error::<Test>::NoPermission
        );
        assert_eq!(PropertyLawyer::<Test>::get(0).unwrap().spv_lawyer, None);

        // Set up new region
        assert_ok!(Regions::propose_new_region(
            RuntimeOrigin::signed([8; 32].into()),
            RegionIdentifier::France
        ));
        assert_ok!(Regions::vote_on_region_proposal(
            RuntimeOrigin::signed([8; 32].into()),
            2,
            pallet_regions::Vote::Yes,
            1_000_000
        ));
        run_to_block(91);
        assert_ok!(Regions::bid_on_region(RuntimeOrigin::signed([8; 32].into()), 2, 100_000));
        run_to_block(121);
        assert_ok!(Regions::create_new_region(
            RuntimeOrigin::signed([8; 32].into()),
            2,
            30,
            Permill::from_percent(3)
        ));
        assert_ok!(Regions::create_new_location(
            RuntimeOrigin::signed([8; 32].into()),
            2,
            bvec![20, 10]
        ));

        // Second Property listing and purchases
        assert_ok!(Marketplace::list_property(
            RuntimeOrigin::signed([0; 32].into()),
            2,
            bvec![20, 10],
            1_000,
            100,
            bvec![22, 22],
            false
        ));
        assert_ok!(Marketplace::buy_property_shares(
            RuntimeOrigin::signed([1; 32].into()),
            1,
            30,
            1984
        ));
        assert_ok!(Marketplace::buy_property_shares(
            RuntimeOrigin::signed([2; 32].into()),
            1,
            40,
            1984
        ));
        assert_ok!(Marketplace::buy_property_shares(
            RuntimeOrigin::signed([30; 32].into()),
            1,
            30,
            1984
        ));
        assert_ok!(Marketplace::create_spv(RuntimeOrigin::signed([5; 32].into()), 1,));
        assert_ok!(Marketplace::claim_property_shares(RuntimeOrigin::signed([1; 32].into()), 1,));
        assert_ok!(Marketplace::claim_property_shares(RuntimeOrigin::signed([2; 32].into()), 1,));
        assert_ok!(Marketplace::claim_property_shares(RuntimeOrigin::signed([30; 32].into()), 1,));

        // Legal process failure cases for second property
        assert_noop!(
            Marketplace::lawyer_claim_property(
                RuntimeOrigin::signed([10; 32].into()),
                1,
                crate::LegalProperty::RealEstateDeveloperSide,
                400,
            ),
            Error::<Test>::WrongRegion
        );
        assert_ok!(Regions::register_lawyer(RuntimeOrigin::signed([12; 32].into()), 2,));
        assert_ok!(XcavateWhitelist::set_permission(
            RuntimeOrigin::signed([20; 32].into()),
            [12; 32].into(),
            pallet_xcavate_whitelist::Role::Lawyer,
            pallet_xcavate_whitelist::AccessPermission::Revoked,
        ));
        assert_noop!(
            Marketplace::lawyer_claim_property(
                RuntimeOrigin::signed([12; 32].into()),
                1,
                crate::LegalProperty::RealEstateDeveloperSide,
                400,
            ),
            BadOrigin
        );
        assert_ok!(XcavateWhitelist::set_permission(
            RuntimeOrigin::signed([20; 32].into()),
            [12; 32].into(),
            pallet_xcavate_whitelist::Role::Lawyer,
            pallet_xcavate_whitelist::AccessPermission::Compliant,
        ));
        assert_ok!(Marketplace::lawyer_claim_property(
            RuntimeOrigin::signed([12; 32].into()),
            1,
            crate::LegalProperty::SpvSide,
            400,
        ));
        assert_noop!(
            Marketplace::lawyer_claim_property(
                RuntimeOrigin::signed([12; 32].into()),
                1,
                crate::LegalProperty::RealEstateDeveloperSide,
                400,
            ),
            Error::<Test>::NoPermission
        );
    })
}

#[test]
fn claim_property_works_fails_2() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);
        // Region setup and role assignments
        assert_ok!(XcavateWhitelist::add_admin(RuntimeOrigin::root(), [20; 32].into(),));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [8; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));
        new_region_helper();
        assert_ok!(Regions::create_new_location(
            RuntimeOrigin::signed([8; 32].into()),
            3,
            bvec![10, 10]
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [0; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateDeveloper
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [1; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [2; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [30; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [9; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [10; 32].into(),
            pallet_xcavate_whitelist::Role::Lawyer
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [11; 32].into(),
            pallet_xcavate_whitelist::Role::Lawyer
        ));
        assert_ok!(Regions::register_lawyer(RuntimeOrigin::signed([10; 32].into()), 3,));
        assert_ok!(Regions::register_lawyer(RuntimeOrigin::signed([11; 32].into()), 3,));

        // Property listing and purchases
        assert_ok!(Marketplace::list_property(
            RuntimeOrigin::signed([0; 32].into()),
            3,
            bvec![10, 10],
            10_000,
            200,
            bvec![22, 22],
            false
        ));
        assert_ok!(Marketplace::buy_property_shares(
            RuntimeOrigin::signed([1; 32].into()),
            0,
            1,
            1984
        ));
        assert_ok!(Marketplace::buy_property_shares(
            RuntimeOrigin::signed([9; 32].into()),
            0,
            98,
            1337
        ));
        assert_ok!(Marketplace::buy_property_shares(
            RuntimeOrigin::signed([2; 32].into()),
            0,
            50,
            1337
        ));
        assert_ok!(Marketplace::buy_property_shares(
            RuntimeOrigin::signed([30; 32].into()),
            0,
            51,
            1337
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [5; 32].into(),
            pallet_xcavate_whitelist::Role::SpvConfirmation
        ));
        assert_ok!(Marketplace::create_spv(RuntimeOrigin::signed([5; 32].into()), 0,));
        assert_ok!(Marketplace::claim_property_shares(RuntimeOrigin::signed([1; 32].into()), 0,));
        assert_ok!(Marketplace::claim_property_shares(RuntimeOrigin::signed([9; 32].into()), 0,));
        assert_ok!(Marketplace::claim_property_shares(RuntimeOrigin::signed([2; 32].into()), 0,));
        assert_ok!(Marketplace::claim_property_shares(RuntimeOrigin::signed([30; 32].into()), 0,));

        // Legal process
        let expiry = frame_system::Pallet::<Test>::block_number() + ClaimWindowTime::get() + 1;
        run_to_block(expiry);
        assert_noop!(
            Marketplace::lawyer_claim_property(
                RuntimeOrigin::signed([11; 32].into()),
                0,
                crate::LegalProperty::SpvSide,
                16_000,
            ),
            Error::<Test>::LegalProcessFailed
        );
        assert_noop!(
            Marketplace::lawyer_claim_property(
                RuntimeOrigin::signed([10; 32].into()),
                0,
                crate::LegalProperty::RealEstateDeveloperSide,
                15_000,
            ),
            Error::<Test>::LegalProcessFailed
        );
    })
}

// approve_developer_lawyer tests

#[test]
fn approve_developer_lawyer_works() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);
        // Region setup and role assignments
        assert_ok!(XcavateWhitelist::add_admin(RuntimeOrigin::root(), [20; 32].into(),));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [8; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));
        new_region_helper();
        assert_ok!(Regions::create_new_location(
            RuntimeOrigin::signed([8; 32].into()),
            3,
            bvec![10, 10]
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [0; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateDeveloper
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [1; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [2; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [30; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [10; 32].into(),
            pallet_xcavate_whitelist::Role::Lawyer
        ));
        assert_ok!(Regions::register_lawyer(RuntimeOrigin::signed([10; 32].into()), 3,));

        // Property listing and purchases
        assert_ok!(Marketplace::list_property(
            RuntimeOrigin::signed([0; 32].into()),
            3,
            bvec![10, 10],
            10_000,
            100,
            bvec![22, 22],
            false
        ));
        assert_ok!(Marketplace::buy_property_shares(
            RuntimeOrigin::signed([1; 32].into()),
            0,
            40,
            1984
        ));
        assert_ok!(Marketplace::buy_property_shares(
            RuntimeOrigin::signed([2; 32].into()),
            0,
            30,
            1984
        ));
        assert_ok!(Marketplace::buy_property_shares(
            RuntimeOrigin::signed([30; 32].into()),
            0,
            30,
            1984
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [5; 32].into(),
            pallet_xcavate_whitelist::Role::SpvConfirmation
        ));
        assert_ok!(Marketplace::create_spv(RuntimeOrigin::signed([5; 32].into()), 0,));
        assert_ok!(Marketplace::claim_property_shares(RuntimeOrigin::signed([1; 32].into()), 0,));
        assert_ok!(Marketplace::claim_property_shares(RuntimeOrigin::signed([2; 32].into()), 0,));
        assert_ok!(Marketplace::claim_property_shares(RuntimeOrigin::signed([30; 32].into()), 0,));

        // Legal process
        assert_ok!(Marketplace::lawyer_claim_property(
            RuntimeOrigin::signed([10; 32].into()),
            0,
            crate::LegalProperty::RealEstateDeveloperSide,
            4_000,
        ));
        assert_ok!(Marketplace::approve_developer_lawyer(
            RuntimeOrigin::signed([0; 32].into()),
            0,
            false
        ));
        assert_ok!(Marketplace::list_property(
            RuntimeOrigin::signed([0; 32].into()),
            3,
            bvec![10, 10],
            1_000,
            100,
            bvec![22, 22],
            false
        ));
        assert_ok!(Marketplace::buy_property_shares(
            RuntimeOrigin::signed([1; 32].into()),
            1,
            40,
            1984
        ));
        assert_ok!(Marketplace::buy_property_shares(
            RuntimeOrigin::signed([2; 32].into()),
            1,
            30,
            1984
        ));
        assert_ok!(Marketplace::buy_property_shares(
            RuntimeOrigin::signed([30; 32].into()),
            1,
            30,
            1984
        ));
        assert_ok!(Marketplace::create_spv(RuntimeOrigin::signed([5; 32].into()), 1,));
        assert_ok!(Marketplace::claim_property_shares(RuntimeOrigin::signed([1; 32].into()), 1,));
        assert_ok!(Marketplace::claim_property_shares(RuntimeOrigin::signed([2; 32].into()), 1,));
        assert_ok!(Marketplace::claim_property_shares(RuntimeOrigin::signed([30; 32].into()), 1,));
        assert_eq!(
            RealEstateLawyer::<Test>::get::<AccountId>([10; 32].into()).unwrap().active_cases,
            0
        );
        assert_eq!(ProposedLawyers::<Test>::get(0).is_none(), true);
        assert_eq!(PropertyLawyer::<Test>::get(0).unwrap().real_estate_developer_lawyer, None);
        assert_ok!(Marketplace::lawyer_claim_property(
            RuntimeOrigin::signed([10; 32].into()),
            0,
            crate::LegalProperty::RealEstateDeveloperSide,
            3_000,
        ));
        assert_ok!(Marketplace::approve_developer_lawyer(
            RuntimeOrigin::signed([0; 32].into()),
            0,
            true
        ));
        assert_eq!(
            RealEstateLawyer::<Test>::get::<AccountId>([10; 32].into()).unwrap().active_cases,
            1
        );
        assert_ok!(Marketplace::lawyer_claim_property(
            RuntimeOrigin::signed([10; 32].into()),
            1,
            crate::LegalProperty::RealEstateDeveloperSide,
            300,
        ));
        assert_ok!(Marketplace::approve_developer_lawyer(
            RuntimeOrigin::signed([0; 32].into()),
            1,
            true
        ));
        assert_eq!(
            RealEstateLawyer::<Test>::get::<AccountId>([10; 32].into()).unwrap().active_cases,
            2
        );
        assert_eq!(ProposedLawyers::<Test>::get(0).is_none(), true);
        assert_eq!(
            PropertyLawyer::<Test>::get(0).unwrap().real_estate_developer_lawyer,
            Some([10; 32].into())
        );
        assert_eq!(
            PropertyLawyer::<Test>::get(0)
                .unwrap()
                .real_estate_developer_lawyer_costs
                .get(&1984)
                .unwrap(),
            &3_000u128
        );
    })
}

#[test]
fn approve_developer_lawyer_works_2() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);
        // Region setup and role assignments
        assert_ok!(XcavateWhitelist::add_admin(RuntimeOrigin::root(), [20; 32].into(),));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [8; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));
        new_region_helper();
        assert_ok!(Regions::create_new_location(
            RuntimeOrigin::signed([8; 32].into()),
            3,
            bvec![10, 10]
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [0; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateDeveloper
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [1; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [2; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [30; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [10; 32].into(),
            pallet_xcavate_whitelist::Role::Lawyer
        ));
        assert_ok!(Regions::register_lawyer(RuntimeOrigin::signed([10; 32].into()), 3,));

        // Property listing and purchases
        assert_ok!(Marketplace::list_property(
            RuntimeOrigin::signed([0; 32].into()),
            3,
            bvec![10, 10],
            10_000,
            100,
            bvec![22, 22],
            false
        ));
        assert_ok!(Marketplace::buy_property_shares(
            RuntimeOrigin::signed([1; 32].into()),
            0,
            40,
            1984
        ));
        assert_ok!(Marketplace::buy_property_shares(
            RuntimeOrigin::signed([2; 32].into()),
            0,
            30,
            1984
        ));
        assert_ok!(Marketplace::buy_property_shares(
            RuntimeOrigin::signed([30; 32].into()),
            0,
            30,
            1984
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [5; 32].into(),
            pallet_xcavate_whitelist::Role::SpvConfirmation
        ));
        assert_ok!(Marketplace::create_spv(RuntimeOrigin::signed([5; 32].into()), 0,));
        assert_ok!(Marketplace::claim_property_shares(RuntimeOrigin::signed([1; 32].into()), 0,));
        assert_ok!(Marketplace::claim_property_shares(RuntimeOrigin::signed([2; 32].into()), 0,));
        assert_ok!(Marketplace::claim_property_shares(RuntimeOrigin::signed([30; 32].into()), 0,));

        assert_ok!(Marketplace::lawyer_claim_property(
            RuntimeOrigin::signed([10; 32].into()),
            0,
            crate::LegalProperty::RealEstateDeveloperSide,
            4_000,
        ));
        let expiry = frame_system::Pallet::<Test>::block_number() + ClaimWindowTime::get() + 1;
        run_to_block(expiry);
        assert_ok!(Marketplace::approve_developer_lawyer(
            RuntimeOrigin::signed([0; 32].into()),
            0,
            true
        ));
        assert_eq!(ProposedLawyers::<Test>::get(0).is_none(), true);
        assert_eq!(PropertyLawyer::<Test>::get(0).unwrap().real_estate_developer_lawyer, None);
        assert_eq!(
            PropertyLawyer::<Test>::get(0)
                .unwrap()
                .real_estate_developer_lawyer_costs
                .get(&1984)
                .unwrap(),
            &0u128
        );
    })
}

#[test]
fn approve_developer_lawyer_fails() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);
        // Region setup and role assignments
        assert_ok!(XcavateWhitelist::add_admin(RuntimeOrigin::root(), [20; 32].into(),));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [8; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));
        new_region_helper();
        assert_ok!(Regions::create_new_location(
            RuntimeOrigin::signed([8; 32].into()),
            3,
            bvec![10, 10]
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [0; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateDeveloper
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [1; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [30; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [31; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [10; 32].into(),
            pallet_xcavate_whitelist::Role::Lawyer
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [10; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateDeveloper
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [11; 32].into(),
            pallet_xcavate_whitelist::Role::Lawyer
        ));
        assert_ok!(Regions::register_lawyer(RuntimeOrigin::signed([10; 32].into()), 3,));
        assert_noop!(
            Marketplace::approve_developer_lawyer(RuntimeOrigin::signed([0; 32].into()), 0, true),
            Error::<Test>::ListingNotFound
        );

        // Property listing and purchases
        assert_ok!(Marketplace::list_property(
            RuntimeOrigin::signed([0; 32].into()),
            3,
            bvec![10, 10],
            10_000,
            100,
            bvec![22, 22],
            false
        ));
        assert_noop!(
            Marketplace::approve_developer_lawyer(RuntimeOrigin::signed([0; 32].into()), 0, true),
            Error::<Test>::InvalidIndex
        );
        assert_ok!(Marketplace::buy_property_shares(
            RuntimeOrigin::signed([1; 32].into()),
            0,
            40,
            1984
        ));
        assert_ok!(Marketplace::buy_property_shares(
            RuntimeOrigin::signed([30; 32].into()),
            0,
            30,
            1984
        ));
        assert_ok!(Marketplace::buy_property_shares(
            RuntimeOrigin::signed([31; 32].into()),
            0,
            30,
            1984
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [5; 32].into(),
            pallet_xcavate_whitelist::Role::SpvConfirmation
        ));
        assert_ok!(Marketplace::create_spv(RuntimeOrigin::signed([5; 32].into()), 0,));
        assert_ok!(Marketplace::claim_property_shares(RuntimeOrigin::signed([1; 32].into()), 0,));
        assert_ok!(Marketplace::claim_property_shares(RuntimeOrigin::signed([30; 32].into()), 0,));
        assert_ok!(Marketplace::claim_property_shares(RuntimeOrigin::signed([31; 32].into()), 0,));

        // Legal process
        assert_noop!(
            Marketplace::approve_developer_lawyer(RuntimeOrigin::signed([0; 32].into()), 0, true),
            Error::<Test>::NoLawyerProposed
        );
        assert_ok!(Marketplace::lawyer_claim_property(
            RuntimeOrigin::signed([10; 32].into()),
            0,
            crate::LegalProperty::RealEstateDeveloperSide,
            4_000,
        ));
        assert_noop!(
            Marketplace::approve_developer_lawyer(RuntimeOrigin::signed([10; 32].into()), 0, false),
            Error::<Test>::NoPermission
        );
    })
}

// vote_on_spv_lawyer tests

#[test]
fn vote_on_spv_lawyer_works() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);
        // Region setup and role assignments
        assert_ok!(XcavateWhitelist::add_admin(RuntimeOrigin::root(), [20; 32].into(),));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [8; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));
        new_region_helper();
        assert_ok!(Regions::create_new_location(
            RuntimeOrigin::signed([8; 32].into()),
            3,
            bvec![10, 10]
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [0; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateDeveloper
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [1; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [2; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [30; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [5; 32].into(),
            pallet_xcavate_whitelist::Role::SpvConfirmation
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [10; 32].into(),
            pallet_xcavate_whitelist::Role::Lawyer
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [11; 32].into(),
            pallet_xcavate_whitelist::Role::Lawyer
        ));
        assert_ok!(Regions::register_lawyer(RuntimeOrigin::signed([10; 32].into()), 3,));

        // Property listing and purchases
        assert_ok!(Marketplace::list_property(
            RuntimeOrigin::signed([0; 32].into()),
            3,
            bvec![10, 10],
            10_000,
            100,
            bvec![22, 22],
            false
        ));
        assert_ok!(Marketplace::buy_property_shares(
            RuntimeOrigin::signed([1; 32].into()),
            0,
            40,
            1984
        ));
        assert_ok!(Marketplace::buy_property_shares(
            RuntimeOrigin::signed([2; 32].into()),
            0,
            40,
            1984
        ));
        assert_ok!(Marketplace::buy_property_shares(
            RuntimeOrigin::signed([30; 32].into()),
            0,
            20,
            1984
        ));
        assert_ok!(Marketplace::create_spv(RuntimeOrigin::signed([5; 32].into()), 0,));
        assert_ok!(Marketplace::claim_property_shares(RuntimeOrigin::signed([1; 32].into()), 0,));
        assert_ok!(Marketplace::claim_property_shares(RuntimeOrigin::signed([2; 32].into()), 0,));
        assert_ok!(Marketplace::claim_property_shares(RuntimeOrigin::signed([30; 32].into()), 0,));

        // Legal process
        assert_ok!(Marketplace::lawyer_claim_property(
            RuntimeOrigin::signed([10; 32].into()),
            0,
            crate::LegalProperty::SpvSide,
            4_000,
        ));
        assert_ok!(Marketplace::vote_on_spv_lawyer(
            RuntimeOrigin::signed([1; 32].into()),
            0,
            crate::Vote::No,
            40
        ));
        assert_eq!(
            AssetsFreezer::balance_frozen(
                0,
                &MarketplaceFreezeReason::SpvLawyerVoting,
                &[1; 32].into()
            ),
            40
        );
        assert_ok!(Marketplace::vote_on_spv_lawyer(
            RuntimeOrigin::signed([2; 32].into()),
            0,
            crate::Vote::Yes,
            40
        ));
        assert_eq!(
            AssetsFreezer::balance_frozen(
                0,
                &MarketplaceFreezeReason::SpvLawyerVoting,
                &[2; 32].into()
            ),
            40
        );
        assert_ok!(Marketplace::vote_on_spv_lawyer(
            RuntimeOrigin::signed([30; 32].into()),
            0,
            crate::Vote::No,
            20
        ));
        assert_eq!(
            AssetsFreezer::balance_frozen(
                0,
                &MarketplaceFreezeReason::SpvLawyerVoting,
                &[30; 32].into()
            ),
            20
        );
        assert_eq!(
            OngoingLawyerVoting::<Test>::get(0).unwrap(),
            VoteStats { yes_voting_power: 40, no_voting_power: 60, abstain_voting_power: 0 }
        );
        assert_eq!(
            UserLawyerVote::<Test>::get::<u64, AccountId>(0, [1; 32].into()).unwrap().vote,
            crate::Vote::No
        );

        // Investor can change vote
        assert_ok!(Marketplace::vote_on_spv_lawyer(
            RuntimeOrigin::signed([1; 32].into()),
            0,
            crate::Vote::Yes,
            20
        ));
        assert_eq!(
            AssetsFreezer::balance_frozen(
                0,
                &MarketplaceFreezeReason::SpvLawyerVoting,
                &[1; 32].into()
            ),
            20
        );
        assert_eq!(
            OngoingLawyerVoting::<Test>::get(0).unwrap(),
            VoteStats { yes_voting_power: 60, no_voting_power: 20, abstain_voting_power: 0 }
        );
        assert_eq!(
            UserLawyerVote::<Test>::get::<u64, AccountId>(0, [1; 32].into()).unwrap().vote,
            crate::Vote::Yes
        );
    })
}

#[test]
fn vote_on_spv_lawyer_fails() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);
        // Region setup and role assignments
        assert_ok!(XcavateWhitelist::add_admin(RuntimeOrigin::root(), [20; 32].into(),));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [8; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));
        new_region_helper();
        assert_ok!(Regions::create_new_location(
            RuntimeOrigin::signed([8; 32].into()),
            3,
            bvec![10, 10]
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [0; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateDeveloper
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [0; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [1; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [2; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [30; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [10; 32].into(),
            pallet_xcavate_whitelist::Role::Lawyer
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [11; 32].into(),
            pallet_xcavate_whitelist::Role::Lawyer
        ));
        assert_ok!(Regions::register_lawyer(RuntimeOrigin::signed([10; 32].into()), 3,));
        assert_noop!(
            Marketplace::vote_on_spv_lawyer(
                RuntimeOrigin::signed([1; 32].into()),
                0,
                crate::Vote::No,
                20,
            ),
            Error::<Test>::NoLawyerProposed
        );

        // Property listing and purchases
        assert_ok!(Marketplace::list_property(
            RuntimeOrigin::signed([0; 32].into()),
            3,
            bvec![10, 10],
            10_000,
            100,
            bvec![22, 22],
            false
        ));
        assert_ok!(Marketplace::buy_property_shares(
            RuntimeOrigin::signed([1; 32].into()),
            0,
            40,
            1984
        ));
        assert_ok!(Marketplace::buy_property_shares(
            RuntimeOrigin::signed([2; 32].into()),
            0,
            40,
            1984
        ));
        assert_ok!(Marketplace::buy_property_shares(
            RuntimeOrigin::signed([30; 32].into()),
            0,
            20,
            1984
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [5; 32].into(),
            pallet_xcavate_whitelist::Role::SpvConfirmation
        ));
        assert_ok!(Marketplace::create_spv(RuntimeOrigin::signed([5; 32].into()), 0,));
        assert_ok!(Marketplace::claim_property_shares(RuntimeOrigin::signed([1; 32].into()), 0,));
        assert_ok!(Marketplace::claim_property_shares(RuntimeOrigin::signed([2; 32].into()), 0,));
        assert_ok!(Marketplace::claim_property_shares(RuntimeOrigin::signed([30; 32].into()), 0,));

        // Legal process
        assert_noop!(
            Marketplace::vote_on_spv_lawyer(
                RuntimeOrigin::signed([1; 32].into()),
                0,
                crate::Vote::No,
                40
            ),
            Error::<Test>::NoLawyerProposed
        );
        assert_ok!(Marketplace::lawyer_claim_property(
            RuntimeOrigin::signed([10; 32].into()),
            0,
            crate::LegalProperty::SpvSide,
            4_000,
        ));
        assert_noop!(
            Marketplace::vote_on_spv_lawyer(
                RuntimeOrigin::signed([0; 32].into()),
                0,
                crate::Vote::No,
                100
            ),
            Error::<Test>::NotEnoughShares
        );
        assert_noop!(
            Marketplace::vote_on_spv_lawyer(
                RuntimeOrigin::signed([0; 32].into()),
                0,
                crate::Vote::No,
                0
            ),
            Error::<Test>::ZeroVoteAmount
        );
        assert_eq!(
            AssetsFreezer::balance_frozen(
                0,
                &MarketplaceFreezeReason::SpvLawyerVoting,
                &[0; 32].into()
            ),
            0
        );
        run_to_block(100);
        assert_noop!(
            Marketplace::vote_on_spv_lawyer(
                RuntimeOrigin::signed([0; 32].into()),
                0,
                crate::Vote::No,
                12
            ),
            Error::<Test>::VotingExpired
        );
    })
}

// vote_on_spv_lawyer tests

#[test]
fn finalize_spv_lawyer_works() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);
        // Region setup and role assignments
        assert_ok!(XcavateWhitelist::add_admin(RuntimeOrigin::root(), [20; 32].into(),));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [8; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));
        new_region_helper();
        assert_ok!(Regions::create_new_location(
            RuntimeOrigin::signed([8; 32].into()),
            3,
            bvec![10, 10]
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [0; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateDeveloper
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [0; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [1; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [2; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [30; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [10; 32].into(),
            pallet_xcavate_whitelist::Role::Lawyer
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [11; 32].into(),
            pallet_xcavate_whitelist::Role::Lawyer
        ));
        assert_ok!(Regions::register_lawyer(RuntimeOrigin::signed([10; 32].into()), 3,));

        // Property listing and purchases
        assert_ok!(Marketplace::list_property(
            RuntimeOrigin::signed([0; 32].into()),
            3,
            bvec![10, 10],
            10_000,
            100,
            bvec![22, 22],
            false
        ));
        assert_ok!(Marketplace::buy_property_shares(
            RuntimeOrigin::signed([1; 32].into()),
            0,
            40,
            1984
        ));
        assert_ok!(Marketplace::buy_property_shares(
            RuntimeOrigin::signed([2; 32].into()),
            0,
            40,
            1337
        ));
        assert_ok!(Marketplace::buy_property_shares(
            RuntimeOrigin::signed([30; 32].into()),
            0,
            20,
            1984
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [5; 32].into(),
            pallet_xcavate_whitelist::Role::SpvConfirmation
        ));
        assert_ok!(Marketplace::create_spv(RuntimeOrigin::signed([5; 32].into()), 0,));
        assert_ok!(Marketplace::claim_property_shares(RuntimeOrigin::signed([1; 32].into()), 0,));
        assert_ok!(Marketplace::claim_property_shares(RuntimeOrigin::signed([2; 32].into()), 0,));
        assert_ok!(Marketplace::claim_property_shares(RuntimeOrigin::signed([30; 32].into()), 0,));

        // Legal process
        // First voting round
        assert_ok!(Marketplace::lawyer_claim_property(
            RuntimeOrigin::signed([10; 32].into()),
            0,
            crate::LegalProperty::SpvSide,
            4_000,
        ));
        assert_eq!(ListingSpvProposal::<Test>::get(0).unwrap(), 0);
        assert_ok!(Marketplace::vote_on_spv_lawyer(
            RuntimeOrigin::signed([1; 32].into()),
            0,
            crate::Vote::No,
            40
        ));
        assert_ok!(Marketplace::vote_on_spv_lawyer(
            RuntimeOrigin::signed([2; 32].into()),
            0,
            crate::Vote::Yes,
            40
        ));
        assert_ok!(Marketplace::vote_on_spv_lawyer(
            RuntimeOrigin::signed([30; 32].into()),
            0,
            crate::Vote::No,
            20
        ));
        run_to_block(91);
        // First proposal fails, not enough yes votes
        assert_ok!(Marketplace::finalize_spv_lawyer(RuntimeOrigin::signed([3; 32].into()), 0,));
        // Second voting round
        assert_ok!(Marketplace::lawyer_claim_property(
            RuntimeOrigin::signed([10; 32].into()),
            0,
            crate::LegalProperty::SpvSide,
            4_000,
        ));
        assert_eq!(ListingSpvProposal::<Test>::get(0).unwrap(), 1);
        run_to_block(121);
        // Second proposal fails, no votes cast
        assert_ok!(Marketplace::finalize_spv_lawyer(RuntimeOrigin::signed([1; 32].into()), 0,));
        assert_eq!(OngoingLawyerVoting::<Test>::get(0).is_none(), true);
        assert_ok!(Marketplace::unfreeze_spv_lawyer_shares(
            RuntimeOrigin::signed([1; 32].into()),
            0,
        ));
        assert_ok!(Marketplace::unfreeze_spv_lawyer_shares(
            RuntimeOrigin::signed([2; 32].into()),
            0,
        ));
        assert_ok!(Marketplace::unfreeze_spv_lawyer_shares(
            RuntimeOrigin::signed([30; 32].into()),
            0,
        ));
        assert_eq!(
            UserLawyerVote::<Test>::get::<u64, AccountId>(0, [1; 32].into()).is_none(),
            true
        );
        assert_eq!(SpvLawyerProposal::<Test>::get(0).is_none(), true);
        assert_eq!(PropertyLawyer::<Test>::get(0).unwrap().spv_lawyer, None);
        // Third voting round
        assert_ok!(Marketplace::lawyer_claim_property(
            RuntimeOrigin::signed([10; 32].into()),
            0,
            crate::LegalProperty::SpvSide,
            3_000,
        ));
        assert_eq!(ListingSpvProposal::<Test>::get(0).unwrap(), 2);
        assert_ok!(Marketplace::vote_on_spv_lawyer(
            RuntimeOrigin::signed([1; 32].into()),
            0,
            crate::Vote::Yes,
            40
        ));
        assert_ok!(Marketplace::vote_on_spv_lawyer(
            RuntimeOrigin::signed([2; 32].into()),
            0,
            crate::Vote::No,
            40
        ));
        assert_ok!(Marketplace::vote_on_spv_lawyer(
            RuntimeOrigin::signed([30; 32].into()),
            0,
            crate::Vote::Yes,
            20
        ));
        run_to_block(151);
        // Third proposal passes
        assert_ok!(Marketplace::finalize_spv_lawyer(RuntimeOrigin::signed([2; 32].into()), 0,));
        assert_eq!(OngoingLawyerVoting::<Test>::get(0).is_none(), true);
        assert_eq!(
            UserLawyerVote::<Test>::get::<u64, AccountId>(2, [1; 32].into()).is_none(),
            false
        );
        assert_eq!(SpvLawyerProposal::<Test>::get(0).is_none(), true);
        assert_eq!(ListingSpvProposal::<Test>::get(0).is_none(), true);
        assert_eq!(PropertyLawyer::<Test>::get(0).unwrap().spv_lawyer, Some([10; 32].into()));
        assert_eq!(
            PropertyLawyer::<Test>::get(0).unwrap().spv_lawyer_costs.get(&1337).unwrap(),
            &3_000u128
        );
    })
}

#[test]
fn finalize_spv_lawyer_works2() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);
        // Region setup and role assignments
        assert_ok!(XcavateWhitelist::add_admin(RuntimeOrigin::root(), [20; 32].into(),));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [8; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));
        new_region_helper();
        assert_ok!(Regions::create_new_location(
            RuntimeOrigin::signed([8; 32].into()),
            3,
            bvec![10, 10]
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [0; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateDeveloper
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [0; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [1; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [2; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [30; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [10; 32].into(),
            pallet_xcavate_whitelist::Role::Lawyer
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [11; 32].into(),
            pallet_xcavate_whitelist::Role::Lawyer
        ));
        assert_ok!(Regions::register_lawyer(RuntimeOrigin::signed([10; 32].into()), 3,));

        // Property listing and purchases
        assert_ok!(Marketplace::list_property(
            RuntimeOrigin::signed([0; 32].into()),
            3,
            bvec![10, 10],
            10_000,
            150,
            bvec![22, 22],
            false
        ));
        assert_ok!(Marketplace::buy_property_shares(
            RuntimeOrigin::signed([1; 32].into()),
            0,
            60,
            1984
        ));
        assert_ok!(Marketplace::buy_property_shares(
            RuntimeOrigin::signed([2; 32].into()),
            0,
            70,
            1337
        ));
        assert_ok!(Marketplace::buy_property_shares(
            RuntimeOrigin::signed([30; 32].into()),
            0,
            20,
            1337
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [5; 32].into(),
            pallet_xcavate_whitelist::Role::SpvConfirmation
        ));
        assert_ok!(Marketplace::create_spv(RuntimeOrigin::signed([5; 32].into()), 0,));
        assert_ok!(Marketplace::claim_property_shares(RuntimeOrigin::signed([1; 32].into()), 0,));
        assert_ok!(Marketplace::claim_property_shares(RuntimeOrigin::signed([2; 32].into()), 0,));
        assert_ok!(Marketplace::claim_property_shares(RuntimeOrigin::signed([30; 32].into()), 0,));

        // Legal process
        // First voting round
        assert_ok!(Marketplace::lawyer_claim_property(
            RuntimeOrigin::signed([10; 32].into()),
            0,
            crate::LegalProperty::SpvSide,
            4_000,
        ));
        assert_eq!(ListingSpvProposal::<Test>::get(0).unwrap(), 0);
        assert_ok!(Marketplace::vote_on_spv_lawyer(
            RuntimeOrigin::signed([1; 32].into()),
            0,
            crate::Vote::No,
            10
        ));
        assert_ok!(Marketplace::vote_on_spv_lawyer(
            RuntimeOrigin::signed([2; 32].into()),
            0,
            crate::Vote::Yes,
            60
        ));
        run_to_block(91);
        // First proposal fails, quorum not met
        assert_ok!(Marketplace::finalize_spv_lawyer(RuntimeOrigin::signed([0; 32].into()), 0,));
        assert_eq!(OngoingLawyerVoting::<Test>::get(0).is_none(), true);
        assert_ok!(Marketplace::unfreeze_spv_lawyer_shares(
            RuntimeOrigin::signed([1; 32].into()),
            0,
        ));
        assert_ok!(Marketplace::unfreeze_spv_lawyer_shares(
            RuntimeOrigin::signed([2; 32].into()),
            0,
        ));
        assert_eq!(
            UserLawyerVote::<Test>::get::<u64, AccountId>(0, [1; 32].into()).is_none(),
            true
        );
        assert_eq!(SpvLawyerProposal::<Test>::get(0).is_none(), true);
        assert_eq!(PropertyLawyer::<Test>::get(0).unwrap().spv_lawyer, None);
        // Second voting round
        assert_ok!(Marketplace::lawyer_claim_property(
            RuntimeOrigin::signed([10; 32].into()),
            0,
            crate::LegalProperty::SpvSide,
            3_000,
        ));
        assert_eq!(ListingSpvProposal::<Test>::get(0).unwrap(), 1);
        assert_ok!(Marketplace::vote_on_spv_lawyer(
            RuntimeOrigin::signed([1; 32].into()),
            0,
            crate::Vote::Yes,
            60
        ));
        assert_ok!(Marketplace::vote_on_spv_lawyer(
            RuntimeOrigin::signed([2; 32].into()),
            0,
            crate::Vote::No,
            15
        ));
        run_to_block(121);
        // Second proposal fails, quorum not met
        assert_ok!(Marketplace::finalize_spv_lawyer(RuntimeOrigin::signed([2; 32].into()), 0,));
        assert_ok!(Marketplace::unfreeze_spv_lawyer_shares(
            RuntimeOrigin::signed([1; 32].into()),
            1,
        ));
        assert_ok!(Marketplace::unfreeze_spv_lawyer_shares(
            RuntimeOrigin::signed([2; 32].into()),
            1,
        ));
        assert_eq!(OngoingLawyerVoting::<Test>::get(0).is_none(), true);
        assert_eq!(
            UserLawyerVote::<Test>::get::<u64, AccountId>(1, [1; 32].into()).is_none(),
            true
        );
        // Third voting round
        assert_ok!(Marketplace::lawyer_claim_property(
            RuntimeOrigin::signed([10; 32].into()),
            0,
            crate::LegalProperty::SpvSide,
            3_000,
        ));
        assert_ok!(Marketplace::vote_on_spv_lawyer(
            RuntimeOrigin::signed([1; 32].into()),
            0,
            crate::Vote::Yes,
            60
        ));
        assert_ok!(Marketplace::vote_on_spv_lawyer(
            RuntimeOrigin::signed([2; 32].into()),
            0,
            crate::Vote::No,
            55
        ));
        run_to_block(151);
        assert_ok!(Marketplace::finalize_spv_lawyer(RuntimeOrigin::signed([2; 32].into()), 0,));
        assert_eq!(SpvLawyerProposal::<Test>::get(0).is_none(), true);
        assert_eq!(ListingSpvProposal::<Test>::get(0).is_none(), true);
        assert_eq!(PropertyLawyer::<Test>::get(0).unwrap().spv_lawyer, Some([10; 32].into()));
        assert_eq!(
            PropertyLawyer::<Test>::get(0).unwrap().spv_lawyer_costs.get(&1337).unwrap(),
            &3_000u128
        );
    })
}

#[test]
fn finalize_spv_lawyer_works3() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);
        // Region setup and role assignments
        assert_ok!(XcavateWhitelist::add_admin(RuntimeOrigin::root(), [20; 32].into(),));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [8; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));
        new_region_helper();
        assert_ok!(Regions::create_new_location(
            RuntimeOrigin::signed([8; 32].into()),
            3,
            bvec![10, 10]
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [0; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateDeveloper
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [0; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [1; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [2; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [30; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [10; 32].into(),
            pallet_xcavate_whitelist::Role::Lawyer
        ));
        assert_ok!(Regions::register_lawyer(RuntimeOrigin::signed([10; 32].into()), 3,));

        // Property listing and purchases
        assert_ok!(Marketplace::list_property(
            RuntimeOrigin::signed([0; 32].into()),
            3,
            bvec![10, 10],
            10_000,
            100,
            bvec![22, 22],
            false
        ));
        assert_ok!(Marketplace::buy_property_shares(
            RuntimeOrigin::signed([1; 32].into()),
            0,
            40,
            1984
        ));
        assert_ok!(Marketplace::buy_property_shares(
            RuntimeOrigin::signed([2; 32].into()),
            0,
            40,
            1337
        ));
        assert_ok!(Marketplace::buy_property_shares(
            RuntimeOrigin::signed([30; 32].into()),
            0,
            20,
            1984
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [5; 32].into(),
            pallet_xcavate_whitelist::Role::SpvConfirmation
        ));
        assert_ok!(Marketplace::create_spv(RuntimeOrigin::signed([5; 32].into()), 0,));
        assert_ok!(Marketplace::claim_property_shares(RuntimeOrigin::signed([1; 32].into()), 0,));
        assert_ok!(Marketplace::claim_property_shares(RuntimeOrigin::signed([2; 32].into()), 0,));
        assert_ok!(Marketplace::claim_property_shares(RuntimeOrigin::signed([30; 32].into()), 0,));

        // Legal process
        assert_ok!(Marketplace::lawyer_claim_property(
            RuntimeOrigin::signed([10; 32].into()),
            0,
            crate::LegalProperty::SpvSide,
            4_000,
        ));
        assert_eq!(ListingSpvProposal::<Test>::get(0).unwrap(), 0);
        assert_ok!(Marketplace::vote_on_spv_lawyer(
            RuntimeOrigin::signed([1; 32].into()),
            0,
            crate::Vote::Yes,
            40
        ));
        assert_ok!(Marketplace::vote_on_spv_lawyer(
            RuntimeOrigin::signed([2; 32].into()),
            0,
            crate::Vote::Yes,
            40
        ));
        assert_ok!(Marketplace::vote_on_spv_lawyer(
            RuntimeOrigin::signed([30; 32].into()),
            0,
            crate::Vote::No,
            20
        ));
        let expiry = frame_system::Pallet::<Test>::block_number() + ClaimWindowTime::get() + 1;
        run_to_block(expiry);
        assert_ok!(Marketplace::finalize_spv_lawyer(RuntimeOrigin::signed([0; 32].into()), 0,));
        assert_eq!(PropertyLawyer::<Test>::get(0).unwrap().spv_lawyer, None);
        assert_eq!(SpvLawyerProposal::<Test>::get(0).is_none(), true);
        assert_eq!(ListingSpvProposal::<Test>::get(0).is_none(), true);
    })
}

#[test]
fn finalize_spv_lawyer_fails() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);
        // Region setup and role assignments
        assert_ok!(XcavateWhitelist::add_admin(RuntimeOrigin::root(), [20; 32].into(),));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [8; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));
        new_region_helper();
        assert_ok!(Regions::create_new_location(
            RuntimeOrigin::signed([8; 32].into()),
            3,
            bvec![10, 10]
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [0; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateDeveloper
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [0; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [1; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [2; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [30; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [10; 32].into(),
            pallet_xcavate_whitelist::Role::Lawyer
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [11; 32].into(),
            pallet_xcavate_whitelist::Role::Lawyer
        ));
        assert_ok!(Regions::register_lawyer(RuntimeOrigin::signed([10; 32].into()), 3,));
        assert_noop!(
            Marketplace::finalize_spv_lawyer(RuntimeOrigin::signed([0; 32].into()), 0,),
            Error::<Test>::NoLawyerProposed
        );

        // Property listing and purchases
        assert_ok!(Marketplace::list_property(
            RuntimeOrigin::signed([0; 32].into()),
            3,
            bvec![10, 10],
            10_000,
            100,
            bvec![22, 22],
            false
        ));
        assert_ok!(Marketplace::buy_property_shares(
            RuntimeOrigin::signed([1; 32].into()),
            0,
            45,
            1984
        ));
        assert_ok!(Marketplace::buy_property_shares(
            RuntimeOrigin::signed([2; 32].into()),
            0,
            40,
            1337
        ));
        assert_ok!(Marketplace::buy_property_shares(
            RuntimeOrigin::signed([30; 32].into()),
            0,
            15,
            1984
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [5; 32].into(),
            pallet_xcavate_whitelist::Role::SpvConfirmation
        ));
        assert_ok!(Marketplace::create_spv(RuntimeOrigin::signed([5; 32].into()), 0,));
        assert_ok!(Marketplace::claim_property_shares(RuntimeOrigin::signed([1; 32].into()), 0,));
        assert_ok!(Marketplace::claim_property_shares(RuntimeOrigin::signed([2; 32].into()), 0,));
        assert_ok!(Marketplace::claim_property_shares(RuntimeOrigin::signed([30; 32].into()), 0,));

        // Legal process
        assert_noop!(
            Marketplace::finalize_spv_lawyer(RuntimeOrigin::signed([0; 32].into()), 0,),
            Error::<Test>::NoLawyerProposed
        );
        assert_ok!(Marketplace::lawyer_claim_property(
            RuntimeOrigin::signed([10; 32].into()),
            0,
            crate::LegalProperty::SpvSide,
            4_000,
        ));
        assert_ok!(Marketplace::vote_on_spv_lawyer(
            RuntimeOrigin::signed([1; 32].into()),
            0,
            crate::Vote::No,
            45
        ));
        assert_ok!(Marketplace::vote_on_spv_lawyer(
            RuntimeOrigin::signed([2; 32].into()),
            0,
            crate::Vote::Yes,
            40
        ));
        assert_noop!(
            Marketplace::finalize_spv_lawyer(RuntimeOrigin::signed([0; 32].into()), 0,),
            Error::<Test>::VotingStillOngoing
        );
    })
}

// remove_lawyer_claim tests

#[test]
fn remove_lawyer_claim_works() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);
        // Region setup and role assignments
        assert_ok!(XcavateWhitelist::add_admin(RuntimeOrigin::root(), [20; 32].into(),));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [8; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));
        new_region_helper();
        assert_ok!(Regions::create_new_location(
            RuntimeOrigin::signed([8; 32].into()),
            3,
            bvec![10, 10]
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [0; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateDeveloper
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [1; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [30; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [31; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [10; 32].into(),
            pallet_xcavate_whitelist::Role::Lawyer
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [11; 32].into(),
            pallet_xcavate_whitelist::Role::Lawyer
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [12; 32].into(),
            pallet_xcavate_whitelist::Role::Lawyer
        ));
        assert_ok!(Regions::register_lawyer(RuntimeOrigin::signed([10; 32].into()), 3,));
        assert_ok!(Regions::register_lawyer(RuntimeOrigin::signed([11; 32].into()), 3,));
        assert_ok!(Regions::register_lawyer(RuntimeOrigin::signed([12; 32].into()), 3,));

        // Property listing and purchases
        assert_ok!(Marketplace::list_property(
            RuntimeOrigin::signed([0; 32].into()),
            3,
            bvec![10, 10],
            10_000,
            100,
            bvec![22, 22],
            false
        ));
        assert_ok!(Marketplace::buy_property_shares(
            RuntimeOrigin::signed([1; 32].into()),
            0,
            40,
            1984
        ));
        assert_ok!(Marketplace::buy_property_shares(
            RuntimeOrigin::signed([30; 32].into()),
            0,
            30,
            1984
        ));
        assert_ok!(Marketplace::buy_property_shares(
            RuntimeOrigin::signed([31; 32].into()),
            0,
            30,
            1984
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [5; 32].into(),
            pallet_xcavate_whitelist::Role::SpvConfirmation
        ));
        assert_ok!(Marketplace::create_spv(RuntimeOrigin::signed([5; 32].into()), 0,));
        assert_ok!(Marketplace::claim_property_shares(RuntimeOrigin::signed([1; 32].into()), 0,));
        assert_ok!(Marketplace::claim_property_shares(RuntimeOrigin::signed([30; 32].into()), 0,));
        assert_ok!(Marketplace::claim_property_shares(RuntimeOrigin::signed([31; 32].into()), 0,));

        // Legal process
        assert_ok!(Marketplace::lawyer_claim_property(
            RuntimeOrigin::signed([10; 32].into()),
            0,
            crate::LegalProperty::RealEstateDeveloperSide,
            4_000,
        ));
        assert_ok!(Marketplace::approve_developer_lawyer(
            RuntimeOrigin::signed([0; 32].into()),
            0,
            true
        ));
        assert_eq!(
            PropertyLawyer::<Test>::get(0).unwrap().real_estate_developer_lawyer,
            Some([10; 32].into())
        );
        assert_ok!(Marketplace::lawyer_claim_property(
            RuntimeOrigin::signed([11; 32].into()),
            0,
            crate::LegalProperty::SpvSide,
            4_000,
        ));
        assert_ok!(Marketplace::vote_on_spv_lawyer(
            RuntimeOrigin::signed([1; 32].into()),
            0,
            crate::Vote::Yes,
            40
        ));
        assert_ok!(Marketplace::vote_on_spv_lawyer(
            RuntimeOrigin::signed([30; 32].into()),
            0,
            crate::Vote::Yes,
            30
        ));
        run_to_block(91);
        assert_ok!(Marketplace::finalize_spv_lawyer(RuntimeOrigin::signed([1; 32].into()), 0,));
        assert_eq!(PropertyLawyer::<Test>::get(0).unwrap().spv_lawyer, Some([11; 32].into()));
        assert_ok!(Marketplace::remove_lawyer_claim(RuntimeOrigin::signed([10; 32].into()), 0,));
        assert_eq!(PropertyLawyer::<Test>::get(0).unwrap().real_estate_developer_lawyer, None);
        assert_eq!(PropertyLawyer::<Test>::get(0).unwrap().spv_lawyer, Some([11; 32].into()));
        assert_ok!(Marketplace::lawyer_claim_property(
            RuntimeOrigin::signed([12; 32].into()),
            0,
            crate::LegalProperty::RealEstateDeveloperSide,
            4_000,
        ));
        assert_ok!(Marketplace::approve_developer_lawyer(
            RuntimeOrigin::signed([0; 32].into()),
            0,
            true
        ));
        assert_eq!(
            PropertyLawyer::<Test>::get(0).unwrap().real_estate_developer_lawyer,
            Some([12; 32].into())
        );
    })
}

#[test]
fn remove_lawyer_claim_fails() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);
        // Region setup and role assignments
        assert_ok!(XcavateWhitelist::add_admin(RuntimeOrigin::root(), [20; 32].into(),));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [8; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));
        new_region_helper();
        assert_ok!(Regions::create_new_location(
            RuntimeOrigin::signed([8; 32].into()),
            3,
            bvec![10, 10]
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [0; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateDeveloper
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [1; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [30; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [31; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [10; 32].into(),
            pallet_xcavate_whitelist::Role::Lawyer
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [11; 32].into(),
            pallet_xcavate_whitelist::Role::Lawyer
        ));

        // Property listing and purchases
        assert_ok!(Marketplace::list_property(
            RuntimeOrigin::signed([0; 32].into()),
            3,
            bvec![10, 10],
            10_000,
            100,
            bvec![22, 22],
            false
        ));
        assert_ok!(Marketplace::buy_property_shares(
            RuntimeOrigin::signed([1; 32].into()),
            0,
            40,
            1984
        ));
        assert_ok!(Marketplace::buy_property_shares(
            RuntimeOrigin::signed([30; 32].into()),
            0,
            30,
            1984
        ));
        assert_ok!(Marketplace::buy_property_shares(
            RuntimeOrigin::signed([31; 32].into()),
            0,
            30,
            1984
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [5; 32].into(),
            pallet_xcavate_whitelist::Role::SpvConfirmation
        ));
        assert_ok!(Marketplace::create_spv(RuntimeOrigin::signed([5; 32].into()), 0,));
        assert_ok!(Marketplace::claim_property_shares(RuntimeOrigin::signed([1; 32].into()), 0,));
        assert_ok!(Marketplace::claim_property_shares(RuntimeOrigin::signed([30; 32].into()), 0,));
        assert_ok!(Marketplace::claim_property_shares(RuntimeOrigin::signed([31; 32].into()), 0,));

        // Legal process
        assert_noop!(
            Marketplace::remove_lawyer_claim(RuntimeOrigin::signed([10; 32].into()), 0,),
            Error::<Test>::NoPermission
        );
        assert_ok!(Regions::register_lawyer(RuntimeOrigin::signed([10; 32].into()), 3,));
        assert_noop!(
            Marketplace::remove_lawyer_claim(RuntimeOrigin::signed([10; 32].into()), 1,),
            Error::<Test>::InvalidIndex
        );
        assert_ok!(Marketplace::lawyer_claim_property(
            RuntimeOrigin::signed([10; 32].into()),
            0,
            crate::LegalProperty::RealEstateDeveloperSide,
            4_000,
        ));
        assert_ok!(Marketplace::approve_developer_lawyer(
            RuntimeOrigin::signed([0; 32].into()),
            0,
            true
        ));
        assert_ok!(Marketplace::lawyer_confirm_documents(
            RuntimeOrigin::signed([10; 32].into()),
            0,
            true,
        ));
        assert_noop!(
            Marketplace::remove_lawyer_claim(RuntimeOrigin::signed([10; 32].into()), 0,),
            Error::<Test>::AlreadyConfirmed
        );
    })
}

// lawyer_confirm_documents tests

#[test]
fn finalize_property_deal() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);

        // Region setup and role assignments
        assert_ok!(XcavateWhitelist::add_admin(RuntimeOrigin::root(), [20; 32].into(),));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [8; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));
        new_region_helper();
        assert_ok!(Regions::create_new_location(
            RuntimeOrigin::signed([8; 32].into()),
            3,
            bvec![10, 10]
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [0; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateDeveloper
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [1; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [2; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [30; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [10; 32].into(),
            pallet_xcavate_whitelist::Role::Lawyer
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [11; 32].into(),
            pallet_xcavate_whitelist::Role::Lawyer
        ));
        assert_ok!(Regions::register_lawyer(RuntimeOrigin::signed([10; 32].into()), 3,));
        assert_ok!(Regions::register_lawyer(RuntimeOrigin::signed([11; 32].into()), 3,));

        // Property listing and purchases
        assert_ok!(Marketplace::list_property(
            RuntimeOrigin::signed([0; 32].into()),
            3,
            bvec![10, 10],
            10_000,
            100,
            bvec![22, 22],
            false
        ));
        assert_eq!(
            Balances::balance_on_hold(&HoldReason::ListingDepositReserve.into(), &([0; 32].into())),
            200_000
        );
        assert_ok!(Marketplace::buy_property_shares(
            RuntimeOrigin::signed([1; 32].into()),
            0,
            25,
            1984
        ));
        assert_ok!(Marketplace::buy_property_shares(
            RuntimeOrigin::signed([1; 32].into()),
            0,
            20,
            1337
        ));
        assert_ok!(Marketplace::buy_property_shares(
            RuntimeOrigin::signed([2; 32].into()),
            0,
            20,
            1984
        ));
        assert_ok!(Marketplace::buy_property_shares(
            RuntimeOrigin::signed([2; 32].into()),
            0,
            20,
            1337
        ));
        assert_ok!(Marketplace::buy_property_shares(
            RuntimeOrigin::signed([30; 32].into()),
            0,
            15,
            1984
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [5; 32].into(),
            pallet_xcavate_whitelist::Role::SpvConfirmation
        ));
        assert_ok!(Marketplace::create_spv(RuntimeOrigin::signed([5; 32].into()), 0,));
        assert_ok!(Marketplace::claim_property_shares(RuntimeOrigin::signed([1; 32].into()), 0,));
        assert_ok!(Marketplace::claim_property_shares(RuntimeOrigin::signed([2; 32].into()), 0,));
        assert_ok!(Marketplace::claim_property_shares(RuntimeOrigin::signed([30; 32].into()), 0,));

        // Legal process
        assert_ok!(Marketplace::lawyer_claim_property(
            RuntimeOrigin::signed([10; 32].into()),
            0,
            crate::LegalProperty::RealEstateDeveloperSide,
            4_000,
        ));
        assert_ok!(Marketplace::approve_developer_lawyer(
            RuntimeOrigin::signed([0; 32].into()),
            0,
            true
        ));
        assert_eq!(
            PropertyLawyer::<Test>::get(0).unwrap().real_estate_developer_lawyer,
            Some([10; 32].into())
        );
        assert_ok!(Marketplace::lawyer_claim_property(
            RuntimeOrigin::signed([11; 32].into()),
            0,
            crate::LegalProperty::SpvSide,
            4_000,
        ));
        assert_ok!(Marketplace::vote_on_spv_lawyer(
            RuntimeOrigin::signed([1; 32].into()),
            0,
            crate::Vote::Yes,
            45
        ));
        assert_ok!(Marketplace::vote_on_spv_lawyer(
            RuntimeOrigin::signed([30; 32].into()),
            0,
            crate::Vote::Yes,
            15
        ));
        run_to_block(91);
        assert_ok!(Marketplace::finalize_spv_lawyer(RuntimeOrigin::signed([1; 32].into()), 0,));
        assert_eq!(PropertyLawyer::<Test>::get(0).unwrap().spv_lawyer, Some([11; 32].into()));
        assert_ok!(Marketplace::lawyer_confirm_documents(
            RuntimeOrigin::signed([10; 32].into()),
            0,
            true,
        ));
        assert_eq!(
            PropertyLawyer::<Test>::get(0).unwrap().real_estate_developer_status,
            crate::DocumentStatus::Approved
        );
        assert_eq!(LocalAssets::balance(0, &Marketplace::property_account_id(0)), 0);
        assert_eq!(OngoingObjectListing::<Test>::get(0).unwrap().asset_id, 0);
        assert_ok!(Marketplace::lawyer_confirm_documents(
            RuntimeOrigin::signed([11; 32].into()),
            0,
            true,
        ));

        // Final assertions after property deal finalization
        assert_eq!(
            RealEstateLawyer::<Test>::get::<AccountId>([10; 32].into()).unwrap().active_cases,
            0
        );
        assert_eq!(
            Balances::balance_on_hold(&HoldReason::ListingDepositReserve.into(), &([0; 32].into())),
            0
        );
        assert_eq!(PropertyLawyer::<Test>::get(1).is_some(), false);
        assert_eq!(ForeignAssets::balance(1984, &[0; 32].into()), 20_594_000);
        assert_eq!(ForeignAssets::balance(1337, &[0; 32].into()), 20_396_000);
        assert_eq!(ForeignAssets::balance(1984, &Marketplace::treasury_account_id()), 6_000);
        assert_eq!(ForeignAssets::balance(1984, &[8; 32].into()), 6_000);
        assert_eq!(ForeignAssets::balance(1337, &Marketplace::treasury_account_id()), 2_000);
        assert_eq!(ForeignAssets::balance(1337, &[8; 32].into()), 2_000);
        assert_eq!(ForeignAssets::balance(1984, &[1; 32].into()), 1_240_000);
        assert_eq!(ForeignAssets::balance(1984, &[2; 32].into()), 942_000);
        assert_eq!(ForeignAssets::balance(1984, &[30; 32].into()), 1_044_000);
        assert_eq!(ForeignAssets::balance(1984, &[10; 32].into()), 0);
        assert_eq!(ForeignAssets::balance(1984, &[11; 32].into()), 18_000);
        assert_eq!(ForeignAssets::balance(1337, &[1; 32].into()), 1_292_000);
        assert_eq!(ForeignAssets::balance(1337, &[2; 32].into()), 942_000);
        assert_eq!(ForeignAssets::balance(1337, &[10; 32].into()), 0);
        assert_eq!(ForeignAssets::balance(1337, &[11; 32].into()), 16_000);
        assert_eq!(PropertyAssetInfo::<Test>::get(0).unwrap().spv_created, true);
        assert_eq!(OngoingObjectListing::<Test>::get(0), None);
        assert_eq!(ShareOwner::<Test>::get::<AccountId, u32>([1; 32].into(), 0), None);
        assert_eq!(LocalAssets::balance(0, &[1; 32].into()), 45);
        assert_eq!(LocalAssets::balance(0, &[2; 32].into()), 40);
        assert_eq!(LocalAssets::balance(0, &[30; 32].into()), 15);
    })
}

#[test]
fn finalize_property_deal_2() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);
        // Region setup and role assignments
        assert_ok!(XcavateWhitelist::add_admin(RuntimeOrigin::root(), [20; 32].into(),));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [8; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));
        new_region_helper();
        assert_ok!(Regions::create_new_location(
            RuntimeOrigin::signed([8; 32].into()),
            3,
            bvec![10, 10]
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [0; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateDeveloper
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [1; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [2; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [30; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [10; 32].into(),
            pallet_xcavate_whitelist::Role::Lawyer
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [11; 32].into(),
            pallet_xcavate_whitelist::Role::Lawyer
        ));
        assert_ok!(Regions::register_lawyer(RuntimeOrigin::signed([10; 32].into()), 3,));
        assert_ok!(Regions::register_lawyer(RuntimeOrigin::signed([11; 32].into()), 3,));

        // Property listing and purchases
        assert_ok!(Marketplace::list_property(
            RuntimeOrigin::signed([0; 32].into()),
            3,
            bvec![10, 10],
            10_000,
            100,
            bvec![22, 22],
            false
        ));
        assert_ok!(Marketplace::buy_property_shares(
            RuntimeOrigin::signed([1; 32].into()),
            0,
            40,
            1337
        ));
        assert_ok!(Marketplace::buy_property_shares(
            RuntimeOrigin::signed([2; 32].into()),
            0,
            30,
            1337
        ));
        assert_ok!(Marketplace::buy_property_shares(
            RuntimeOrigin::signed([30; 32].into()),
            0,
            30,
            1337
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [5; 32].into(),
            pallet_xcavate_whitelist::Role::SpvConfirmation
        ));
        assert_ok!(Marketplace::create_spv(RuntimeOrigin::signed([5; 32].into()), 0,));
        assert_ok!(Marketplace::claim_property_shares(RuntimeOrigin::signed([1; 32].into()), 0,));
        assert_ok!(Marketplace::claim_property_shares(RuntimeOrigin::signed([2; 32].into()), 0,));
        assert_ok!(Marketplace::claim_property_shares(RuntimeOrigin::signed([30; 32].into()), 0,));

        // Legal process
        assert_ok!(Marketplace::lawyer_claim_property(
            RuntimeOrigin::signed([10; 32].into()),
            0,
            crate::LegalProperty::RealEstateDeveloperSide,
            4_000,
        ));
        assert_ok!(Marketplace::approve_developer_lawyer(
            RuntimeOrigin::signed([0; 32].into()),
            0,
            true
        ));
        assert_eq!(
            PropertyLawyer::<Test>::get(0).unwrap().real_estate_developer_lawyer,
            Some([10; 32].into())
        );
        assert_ok!(Marketplace::lawyer_claim_property(
            RuntimeOrigin::signed([11; 32].into()),
            0,
            crate::LegalProperty::SpvSide,
            4_000,
        ));
        assert_ok!(Marketplace::vote_on_spv_lawyer(
            RuntimeOrigin::signed([1; 32].into()),
            0,
            crate::Vote::Yes,
            40
        ));
        assert_ok!(Marketplace::vote_on_spv_lawyer(
            RuntimeOrigin::signed([2; 32].into()),
            0,
            crate::Vote::Yes,
            30
        ));
        run_to_block(91);
        assert_ok!(Marketplace::finalize_spv_lawyer(RuntimeOrigin::signed([1; 32].into()), 0,));
        assert_eq!(PropertyLawyer::<Test>::get(0).unwrap().spv_lawyer, Some([11; 32].into()));
        assert_ok!(Marketplace::lawyer_confirm_documents(
            RuntimeOrigin::signed([10; 32].into()),
            0,
            true,
        ));
        assert_eq!(
            PropertyLawyer::<Test>::get(0).unwrap().real_estate_developer_status,
            crate::DocumentStatus::Approved
        );
        assert_ok!(Marketplace::lawyer_confirm_documents(
            RuntimeOrigin::signed([11; 32].into()),
            0,
            true,
        ));

        // Final assertions after property deal finalization
        assert_eq!(PropertyLawyer::<Test>::get(1).is_some(), false);
        assert_eq!(ForeignAssets::balance(1984, &[0; 32].into()), 20_000_000);
        assert_eq!(ForeignAssets::balance(1337, &[0; 32].into()), 20_990_000);
        assert_eq!(ForeignAssets::balance(1984, &Marketplace::treasury_account_id()), 0);
        assert_eq!(ForeignAssets::balance(1337, &Marketplace::treasury_account_id()), 8000);
        assert_eq!(ForeignAssets::balance(1337, &[8; 32].into()), 8000);
        assert_eq!(ForeignAssets::balance(1984, &[1; 32].into()), 1_500_000);
        assert_eq!(ForeignAssets::balance(1984, &[10; 32].into()), 0);
        assert_eq!(ForeignAssets::balance(1984, &[11; 32].into()), 0);
        assert_eq!(ForeignAssets::balance(1337, &[1; 32].into()), 1_084_000);
        assert_eq!(ForeignAssets::balance(1337, &[2; 32].into()), 838_000);
        assert_eq!(ForeignAssets::balance(1337, &[30; 32].into()), 888_000);
        assert_eq!(ForeignAssets::balance(1337, &[10; 32].into()), 0);
        assert_eq!(ForeignAssets::balance(1337, &[11; 32].into()), 34_000);
        assert_eq!(PropertyAssetInfo::<Test>::get(0).unwrap().spv_created, true);
        assert_eq!(OngoingObjectListing::<Test>::get(0), None);
        assert_eq!(LocalAssets::balance(0, &[1; 32].into()), 40);
        assert_eq!(LocalAssets::balance(0, &[2; 32].into()), 30);
        assert_eq!(LocalAssets::balance(0, &[30; 32].into()), 30);
    })
}

#[test]
fn finalize_property_deal_3() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);

        // Region setup and role assignments
        assert_ok!(XcavateWhitelist::add_admin(RuntimeOrigin::root(), [20; 32].into(),));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [8; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [8; 32].into(),
            pallet_xcavate_whitelist::Role::RegionalOperator
        ));
        assert_ok!(Regions::propose_new_region(
            RuntimeOrigin::signed([8; 32].into()),
            RegionIdentifier::Japan
        ));
        assert_ok!(Regions::vote_on_region_proposal(
            RuntimeOrigin::signed([8; 32].into()),
            3,
            pallet_regions::Vote::Yes,
            1_000_000
        ));
        run_to_block(31);
        assert_ok!(Regions::bid_on_region(RuntimeOrigin::signed([8; 32].into()), 3, 100_000));
        run_to_block(61);
        assert_ok!(Regions::create_new_region(
            RuntimeOrigin::signed([8; 32].into()),
            3,
            30,
            Permill::from_parts(32_500)
        ));
        assert_ok!(Regions::create_new_location(
            RuntimeOrigin::signed([8; 32].into()),
            3,
            bvec![10, 10]
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [0; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateDeveloper
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [1; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [2; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [30; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [10; 32].into(),
            pallet_xcavate_whitelist::Role::Lawyer
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [11; 32].into(),
            pallet_xcavate_whitelist::Role::Lawyer
        ));
        assert_ok!(Regions::register_lawyer(RuntimeOrigin::signed([10; 32].into()), 3,));
        assert_ok!(Regions::register_lawyer(RuntimeOrigin::signed([11; 32].into()), 3,));

        // Property listing and purchases
        assert_ok!(Marketplace::list_property(
            RuntimeOrigin::signed([0; 32].into()),
            3,
            bvec![10, 10],
            10_000,
            100,
            bvec![22, 22],
            true
        ));
        assert_eq!(
            Balances::balance_on_hold(&HoldReason::ListingDepositReserve.into(), &([0; 32].into())),
            200_000
        );
        assert_ok!(Marketplace::buy_property_shares(
            RuntimeOrigin::signed([1; 32].into()),
            0,
            20,
            1984
        ));
        assert_ok!(Marketplace::buy_property_shares(
            RuntimeOrigin::signed([1; 32].into()),
            0,
            20,
            1337
        ));
        assert_ok!(Marketplace::buy_property_shares(
            RuntimeOrigin::signed([2; 32].into()),
            0,
            20,
            1337
        ));
        assert_ok!(Marketplace::buy_property_shares(
            RuntimeOrigin::signed([2; 32].into()),
            0,
            20,
            1984
        ));
        assert_ok!(Marketplace::buy_property_shares(
            RuntimeOrigin::signed([30; 32].into()),
            0,
            20,
            1984
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [5; 32].into(),
            pallet_xcavate_whitelist::Role::SpvConfirmation
        ));
        assert_ok!(Marketplace::create_spv(RuntimeOrigin::signed([5; 32].into()), 0,));
        assert_ok!(Marketplace::claim_property_shares(RuntimeOrigin::signed([1; 32].into()), 0,));
        assert_ok!(Marketplace::claim_property_shares(RuntimeOrigin::signed([2; 32].into()), 0,));
        assert_ok!(Marketplace::claim_property_shares(RuntimeOrigin::signed([30; 32].into()), 0,));

        // Legal process
        assert_ok!(Marketplace::lawyer_claim_property(
            RuntimeOrigin::signed([10; 32].into()),
            0,
            crate::LegalProperty::RealEstateDeveloperSide,
            4_000,
        ));
        assert_ok!(Marketplace::approve_developer_lawyer(
            RuntimeOrigin::signed([0; 32].into()),
            0,
            true
        ));
        assert_eq!(
            PropertyLawyer::<Test>::get(0).unwrap().real_estate_developer_lawyer,
            Some([10; 32].into())
        );
        assert_ok!(Marketplace::lawyer_claim_property(
            RuntimeOrigin::signed([11; 32].into()),
            0,
            crate::LegalProperty::SpvSide,
            4_000,
        ));
        assert_ok!(Marketplace::vote_on_spv_lawyer(
            RuntimeOrigin::signed([1; 32].into()),
            0,
            crate::Vote::Yes,
            40
        ));
        assert_ok!(Marketplace::vote_on_spv_lawyer(
            RuntimeOrigin::signed([2; 32].into()),
            0,
            crate::Vote::Yes,
            40
        ));
        run_to_block(91);
        assert_ok!(Marketplace::finalize_spv_lawyer(RuntimeOrigin::signed([1; 32].into()), 0,));
        assert_eq!(PropertyLawyer::<Test>::get(0).unwrap().spv_lawyer, Some([11; 32].into()));
        assert_ok!(Marketplace::lawyer_confirm_documents(
            RuntimeOrigin::signed([10; 32].into()),
            0,
            true,
        ));
        assert_eq!(
            PropertyLawyer::<Test>::get(0).unwrap().real_estate_developer_status,
            crate::DocumentStatus::Approved
        );
        assert_eq!(LocalAssets::balance(0, &Marketplace::property_account_id(0)), 0);
        assert_eq!(OngoingObjectListing::<Test>::get(0).unwrap().asset_id, 0);
        assert_eq!(
            OngoingObjectListing::<Test>::get(0).unwrap().collected_tax.get(&1984).copied(),
            Some(19_500)
        );
        assert_eq!(
            OngoingObjectListing::<Test>::get(0).unwrap().collected_tax.get(&1337).copied(),
            Some(13_000)
        );
        assert_ok!(Marketplace::lawyer_confirm_documents(
            RuntimeOrigin::signed([11; 32].into()),
            0,
            true,
        ));

        // Final assertions after property deal finalization
        assert_eq!(
            Balances::balance_on_hold(&HoldReason::ListingDepositReserve.into(), &([0; 32].into())),
            0
        );
        assert_eq!(PropertyLawyer::<Test>::get(1).is_some(), false);
        assert_eq!(ForeignAssets::balance(1984, &[0; 32].into()), 20_574_500);
        assert_eq!(ForeignAssets::balance(1337, &[0; 32].into()), 20_383_000);
        assert_eq!(ForeignAssets::balance(1984, &Marketplace::treasury_account_id()), 6_000);
        assert_eq!(ForeignAssets::balance(1984, &[8; 32].into()), 6_000);
        assert_eq!(ForeignAssets::balance(1337, &Marketplace::treasury_account_id()), 2_000);
        assert_eq!(ForeignAssets::balance(1337, &[8; 32].into()), 2_000);
        assert_eq!(ForeignAssets::balance(1984, &[1; 32].into()), 1_298_000);
        assert_eq!(ForeignAssets::balance(1984, &[2; 32].into()), 948_000);
        assert_eq!(ForeignAssets::balance(1984, &[10; 32].into()), 19_500);
        assert_eq!(ForeignAssets::balance(1984, &[11; 32].into()), 0);
        assert_eq!(ForeignAssets::balance(1337, &[1; 32].into()), 1_298_000);
        assert_eq!(ForeignAssets::balance(1337, &[2; 32].into()), 948_000);
        assert_eq!(ForeignAssets::balance(1337, &[10; 32].into()), 13_000);
        assert_eq!(ForeignAssets::balance(1337, &[11; 32].into()), 4_000);
        assert_eq!(PropertyAssetInfo::<Test>::get(0).unwrap().spv_created, true);
        assert_eq!(OngoingObjectListing::<Test>::get(0), None);
        assert_eq!(ShareOwner::<Test>::get::<AccountId, u32>([1; 32].into(), 0), None);
        assert_eq!(LocalAssets::balance(0, &[1; 32].into()), 40);
    })
}

#[test]
fn finalize_property_deal_4() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);
        // Region setup and role assignments
        assert_ok!(XcavateWhitelist::add_admin(RuntimeOrigin::root(), [20; 32].into(),));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [8; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [8; 32].into(),
            pallet_xcavate_whitelist::Role::RegionalOperator
        ));
        assert_ok!(Regions::propose_new_region(
            RuntimeOrigin::signed([8; 32].into()),
            RegionIdentifier::Japan
        ));
        assert_ok!(Regions::vote_on_region_proposal(
            RuntimeOrigin::signed([8; 32].into()),
            3,
            pallet_regions::Vote::Yes,
            1_000_000
        ));
        run_to_block(31);
        assert_ok!(Regions::bid_on_region(RuntimeOrigin::signed([8; 32].into()), 3, 100_000));
        run_to_block(61);
        assert_ok!(Regions::create_new_region(
            RuntimeOrigin::signed([8; 32].into()),
            3,
            30,
            Permill::from_parts(32_500)
        ));
        assert_ok!(Regions::create_new_location(
            RuntimeOrigin::signed([8; 32].into()),
            3,
            bvec![10, 10]
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [0; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateDeveloper
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [1; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [2; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [30; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [10; 32].into(),
            pallet_xcavate_whitelist::Role::Lawyer
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [11; 32].into(),
            pallet_xcavate_whitelist::Role::Lawyer
        ));
        assert_ok!(Regions::register_lawyer(RuntimeOrigin::signed([10; 32].into()), 3,));
        assert_ok!(Regions::register_lawyer(RuntimeOrigin::signed([11; 32].into()), 3,));

        // Property listing and purchases
        assert_ok!(Marketplace::list_property(
            RuntimeOrigin::signed([0; 32].into()),
            3,
            bvec![10, 10],
            10_000,
            100,
            bvec![22, 22],
            true
        ));
        assert_eq!(
            Balances::balance_on_hold(&HoldReason::ListingDepositReserve.into(), &([0; 32].into())),
            200_000
        );
        assert_ok!(Marketplace::buy_property_shares(
            RuntimeOrigin::signed([1; 32].into()),
            0,
            40,
            1984
        ));
        assert_ok!(Marketplace::buy_property_shares(
            RuntimeOrigin::signed([2; 32].into()),
            0,
            40,
            1337
        ));
        assert_ok!(Marketplace::buy_property_shares(
            RuntimeOrigin::signed([30; 32].into()),
            0,
            20,
            1984
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [5; 32].into(),
            pallet_xcavate_whitelist::Role::SpvConfirmation
        ));
        assert_ok!(Marketplace::create_spv(RuntimeOrigin::signed([5; 32].into()), 0,));
        assert_ok!(Marketplace::claim_property_shares(RuntimeOrigin::signed([1; 32].into()), 0,));
        assert_ok!(Marketplace::claim_property_shares(RuntimeOrigin::signed([2; 32].into()), 0,));
        assert_ok!(Marketplace::claim_property_shares(RuntimeOrigin::signed([30; 32].into()), 0,));

        // Legal process
        assert_ok!(Marketplace::lawyer_claim_property(
            RuntimeOrigin::signed([10; 32].into()),
            0,
            crate::LegalProperty::RealEstateDeveloperSide,
            4_000,
        ));
        assert_ok!(Marketplace::approve_developer_lawyer(
            RuntimeOrigin::signed([0; 32].into()),
            0,
            true
        ));
        assert_eq!(
            PropertyLawyer::<Test>::get(0).unwrap().real_estate_developer_lawyer,
            Some([10; 32].into())
        );
        assert_ok!(Marketplace::lawyer_claim_property(
            RuntimeOrigin::signed([11; 32].into()),
            0,
            crate::LegalProperty::SpvSide,
            4_000,
        ));
        assert_ok!(Marketplace::vote_on_spv_lawyer(
            RuntimeOrigin::signed([1; 32].into()),
            0,
            crate::Vote::Yes,
            40
        ));
        assert_ok!(Marketplace::vote_on_spv_lawyer(
            RuntimeOrigin::signed([30; 32].into()),
            0,
            crate::Vote::Yes,
            20
        ));
        run_to_block(91);
        assert_ok!(Marketplace::finalize_spv_lawyer(RuntimeOrigin::signed([1; 32].into()), 0,));
        assert_eq!(PropertyLawyer::<Test>::get(0).unwrap().spv_lawyer, Some([11; 32].into()));
        assert_ok!(Marketplace::lawyer_confirm_documents(
            RuntimeOrigin::signed([10; 32].into()),
            0,
            true,
        ));
        assert_eq!(
            PropertyLawyer::<Test>::get(0).unwrap().real_estate_developer_status,
            crate::DocumentStatus::Approved
        );
        assert_eq!(LocalAssets::balance(40, &Marketplace::property_account_id(0)), 0);
        assert_eq!(OngoingObjectListing::<Test>::get(0).unwrap().asset_id, 0);
        assert_eq!(
            OngoingObjectListing::<Test>::get(0).unwrap().collected_tax.get(&1984).copied(),
            Some(19_500)
        );
        assert_eq!(
            OngoingObjectListing::<Test>::get(0).unwrap().collected_tax.get(&1337).copied(),
            Some(13_000)
        );
        assert_ok!(ForeignAssets::transfer(
            RuntimeOrigin::signed([1; 32].into()),
            parity_scale_codec::Compact(1337),
            sp_runtime::MultiAddress::Id(Marketplace::property_account_id(0)),
            404_000
        ));
        assert_ok!(Marketplace::lawyer_confirm_documents(
            RuntimeOrigin::signed([11; 32].into()),
            0,
            true,
        ));

        // Final assertions after property deal finalization
        assert_eq!(
            Balances::balance_on_hold(&HoldReason::ListingDepositReserve.into(), &([0; 32].into())),
            0
        );
        assert_eq!(PropertyLawyer::<Test>::get(1).is_some(), false);
        assert_eq!(ForeignAssets::balance(1984, &[0; 32].into()), 20_574_500);
        assert_eq!(ForeignAssets::balance(1337, &[0; 32].into()), 20_383_000);
        assert_eq!(ForeignAssets::balance(1984, &Marketplace::treasury_account_id()), 6_000);
        assert_eq!(ForeignAssets::balance(1984, &[8; 32].into()), 6_000);
        assert_eq!(ForeignAssets::balance(1337, &Marketplace::treasury_account_id()), 2_000);
        assert_eq!(ForeignAssets::balance(1337, &[8; 32].into()), 2_000);
        assert_eq!(ForeignAssets::balance(1984, &[1; 32].into()), 1_096_000);
        assert_eq!(ForeignAssets::balance(1984, &[30; 32].into()), 998_000);
        assert_eq!(ForeignAssets::balance(1984, &[10; 32].into()), 19_500);
        assert_eq!(ForeignAssets::balance(1984, &[11; 32].into()), 0);
        assert_eq!(ForeignAssets::balance(1337, &[1; 32].into()), 1_096_000);
        assert_eq!(ForeignAssets::balance(1337, &[10; 32].into()), 13_000);
        assert_eq!(ForeignAssets::balance(1337, &[11; 32].into()), 4_000);
        assert_eq!(PropertyAssetInfo::<Test>::get(0).unwrap().spv_created, true);
        assert_eq!(OngoingObjectListing::<Test>::get(0), None);
        assert_eq!(ShareOwner::<Test>::get::<AccountId, u32>([1; 32].into(), 0), None);
        assert_eq!(LocalAssets::balance(0, &[1; 32].into()), 40);
        assert_eq!(LocalAssets::balance(0, &[30; 32].into()), 20);
    })
}

#[test]
fn reject_contract_and_refund() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);
        // Region setup and role assignments
        assert_ok!(XcavateWhitelist::add_admin(RuntimeOrigin::root(), [20; 32].into(),));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [8; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));
        new_region_helper();
        assert_ok!(Regions::create_new_location(
            RuntimeOrigin::signed([8; 32].into()),
            3,
            bvec![10, 10]
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [0; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateDeveloper
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [1; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [2; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [30; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [10; 32].into(),
            pallet_xcavate_whitelist::Role::Lawyer
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [11; 32].into(),
            pallet_xcavate_whitelist::Role::Lawyer
        ));
        assert_ok!(Regions::register_lawyer(RuntimeOrigin::signed([10; 32].into()), 3,));
        assert_ok!(Regions::register_lawyer(RuntimeOrigin::signed([11; 32].into()), 3,));

        // Property listing and purchases
        assert_ok!(Marketplace::list_property(
            RuntimeOrigin::signed([0; 32].into()),
            3,
            bvec![10, 10],
            10_000,
            100,
            bvec![22, 22],
            false
        ));
        assert_ok!(Marketplace::buy_property_shares(
            RuntimeOrigin::signed([1; 32].into()),
            0,
            25,
            1984
        ));
        assert_ok!(Marketplace::buy_property_shares(
            RuntimeOrigin::signed([1; 32].into()),
            0,
            20,
            1337
        ));
        assert_ok!(Marketplace::buy_property_shares(
            RuntimeOrigin::signed([2; 32].into()),
            0,
            30,
            1984
        ));
        assert_ok!(Marketplace::buy_property_shares(
            RuntimeOrigin::signed([30; 32].into()),
            0,
            25,
            1984
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [5; 32].into(),
            pallet_xcavate_whitelist::Role::SpvConfirmation
        ));
        assert_ok!(Marketplace::create_spv(RuntimeOrigin::signed([5; 32].into()), 0,));
        assert_ok!(Marketplace::claim_property_shares(RuntimeOrigin::signed([1; 32].into()), 0));
        assert_ok!(Marketplace::claim_property_shares(RuntimeOrigin::signed([2; 32].into()), 0));
        assert_ok!(Marketplace::claim_property_shares(RuntimeOrigin::signed([30; 32].into()), 0));

        // Legal process
        assert_ok!(Marketplace::lawyer_claim_property(
            RuntimeOrigin::signed([10; 32].into()),
            0,
            crate::LegalProperty::RealEstateDeveloperSide,
            4_000,
        ));
        assert_ok!(Marketplace::approve_developer_lawyer(
            RuntimeOrigin::signed([0; 32].into()),
            0,
            true
        ));
        assert_eq!(
            PropertyLawyer::<Test>::get(0).unwrap().real_estate_developer_lawyer,
            Some([10; 32].into())
        );
        assert_ok!(Marketplace::lawyer_claim_property(
            RuntimeOrigin::signed([11; 32].into()),
            0,
            crate::LegalProperty::SpvSide,
            4_000,
        ));
        assert_ok!(Marketplace::vote_on_spv_lawyer(
            RuntimeOrigin::signed([1; 32].into()),
            0,
            crate::Vote::Yes,
            45
        ));
        assert_ok!(Marketplace::vote_on_spv_lawyer(
            RuntimeOrigin::signed([2; 32].into()),
            0,
            crate::Vote::Yes,
            30
        ));
        run_to_block(91);
        assert_ok!(Marketplace::finalize_spv_lawyer(RuntimeOrigin::signed([1; 32].into()), 0,));
        assert_ok!(Marketplace::unfreeze_spv_lawyer_shares(
            RuntimeOrigin::signed([1; 32].into()),
            0,
        ));
        assert_ok!(Marketplace::unfreeze_spv_lawyer_shares(
            RuntimeOrigin::signed([2; 32].into()),
            0,
        ));
        assert_eq!(PropertyLawyer::<Test>::get(0).unwrap().spv_lawyer, Some([11; 32].into()));
        assert_ok!(Marketplace::lawyer_confirm_documents(
            RuntimeOrigin::signed([10; 32].into()),
            0,
            false,
        ));
        assert_eq!(
            PropertyLawyer::<Test>::get(0).unwrap().real_estate_developer_status,
            crate::DocumentStatus::Rejected
        );
        assert_eq!(pallet_nfts::Item::<Test>::get(0, 0).is_none(), false);

        assert_eq!(AssetsHolder::total_balance_on_hold(1984, &[1; 32].into()), 0);
        assert_eq!(AssetsHolder::total_balance_on_hold(1337, &[1; 32].into()), 0);
        assert_eq!(ForeignAssets::balance(1984, &[1; 32].into()), 1_240_000);
        assert_eq!(ForeignAssets::balance(1337, &[1; 32].into()), 1_292_000);
        assert_ok!(Marketplace::lawyer_confirm_documents(
            RuntimeOrigin::signed([11; 32].into()),
            0,
            false,
        ));
        assert_eq!(RefundShare::<Test>::get(0).unwrap().refund_amount, 100);
        assert_eq!(LocalAssets::balance(0, &[1; 32].into()), 45);
        assert_eq!(
            Balances::balance_on_hold(&HoldReason::ListingDepositReserve.into(), &([0; 32].into())),
            200_000
        );

        // Withdraw refunds
        assert_ok!(Marketplace::withdraw_rejected(RuntimeOrigin::signed([1; 32].into()), 0));
        assert_ok!(Marketplace::withdraw_rejected(RuntimeOrigin::signed([2; 32].into()), 0));
        assert_ok!(Marketplace::withdraw_rejected(RuntimeOrigin::signed([30; 32].into()), 0));

        assert_eq!(
            Balances::balance_on_hold(&HoldReason::ListingDepositReserve.into(), &([0; 32].into())),
            0
        );
        assert_eq!(RefundShare::<Test>::get(0).is_none(), true);
        assert_eq!(PropertyLawyer::<Test>::get(1).is_some(), false);
        assert_eq!(ForeignAssets::balance(1984, &[0; 32].into()), 20_000_000);
        // Fees still got paid even after rejection
        assert_eq!(ForeignAssets::balance(1984, &Marketplace::treasury_account_id()), 6000);
        assert_eq!(ForeignAssets::balance(1337, &Marketplace::treasury_account_id()), 0);
        assert_eq!(ForeignAssets::balance(1984, &Marketplace::property_account_id(0)), 0);
        assert_eq!(ForeignAssets::balance(1984, &[1; 32].into()), 1_497_500);
        assert_eq!(ForeignAssets::balance(1337, &[1; 32].into()), 1_498_000);
        assert_eq!(ForeignAssets::balance(1984, &[2; 32].into()), 1_147_000);
        assert_eq!(ForeignAssets::balance(1984, &[30; 32].into()), 1_197_500);
        assert_eq!(ForeignAssets::balance(1984, &[11; 32].into()), 2_000);
        assert_eq!(ForeignAssets::balance(1337, &[11; 32].into()), 2_000);
        assert_eq!(PropertyAssetInfo::<Test>::get(0).is_none(), true);
        assert_eq!(OngoingObjectListing::<Test>::get(0), None);
        assert_eq!(ShareOwner::<Test>::get::<AccountId, u32>([1; 32].into(), 0), None);
        // Property NFT burned
        assert_eq!(pallet_nfts::Item::<Test>::get(0, 0).is_none(), true);
        assert_eq!(ForeignAssets::balance(1984, &Marketplace::property_account_id(0)), 0);
        assert_eq!(Balances::free_balance(&(Marketplace::property_account_id(0))), 0);
        assert_eq!(Balances::balance(&(Marketplace::property_account_id(0))), 0);
        assert_eq!(Nfts::owner(0, 0), None);
        assert_eq!(PropertyAssetInfo::<Test>::get(0), None);
        assert_eq!(
            RealEstateLawyer::<Test>::get::<AccountId>([10; 32].into()).unwrap().active_cases,
            0
        );
    })
}

#[test]
fn reject_contract_and_refund_2() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);
        // Region setup and role assignments
        assert_ok!(XcavateWhitelist::add_admin(RuntimeOrigin::root(), [20; 32].into(),));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [8; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));
        new_region_helper();
        assert_ok!(Regions::create_new_location(
            RuntimeOrigin::signed([8; 32].into()),
            3,
            bvec![10, 10]
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [0; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateDeveloper
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [1; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [2; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [7; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [10; 32].into(),
            pallet_xcavate_whitelist::Role::Lawyer
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [11; 32].into(),
            pallet_xcavate_whitelist::Role::Lawyer
        ));
        assert_ok!(Regions::register_lawyer(RuntimeOrigin::signed([10; 32].into()), 3,));
        assert_ok!(Regions::register_lawyer(RuntimeOrigin::signed([11; 32].into()), 3,));

        // Property listing and purchases
        assert_ok!(Marketplace::list_property(
            RuntimeOrigin::signed([0; 32].into()),
            3,
            bvec![10, 10],
            10_000,
            100,
            bvec![22, 22],
            false
        ));
        assert_ok!(Marketplace::buy_property_shares(
            RuntimeOrigin::signed([1; 32].into()),
            0,
            30,
            1984
        ));
        assert_ok!(Marketplace::buy_property_shares(
            RuntimeOrigin::signed([2; 32].into()),
            0,
            30,
            1984
        ));
        assert_ok!(Marketplace::buy_property_shares(
            RuntimeOrigin::signed([7; 32].into()),
            0,
            40,
            1337
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [5; 32].into(),
            pallet_xcavate_whitelist::Role::SpvConfirmation
        ));
        assert_ok!(Marketplace::create_spv(RuntimeOrigin::signed([5; 32].into()), 0,));
        assert_ok!(Marketplace::claim_property_shares(RuntimeOrigin::signed([1; 32].into()), 0,));
        assert_ok!(Marketplace::claim_property_shares(RuntimeOrigin::signed([2; 32].into()), 0,));
        assert_ok!(Marketplace::claim_property_shares(RuntimeOrigin::signed([7; 32].into()), 0,));

        // Legal process
        assert_ok!(Marketplace::lawyer_claim_property(
            RuntimeOrigin::signed([10; 32].into()),
            0,
            crate::LegalProperty::RealEstateDeveloperSide,
            4_000,
        ));
        assert_ok!(Marketplace::approve_developer_lawyer(
            RuntimeOrigin::signed([0; 32].into()),
            0,
            true
        ));
        assert_eq!(
            PropertyLawyer::<Test>::get(0).unwrap().real_estate_developer_lawyer,
            Some([10; 32].into())
        );
        assert_ok!(Marketplace::lawyer_claim_property(
            RuntimeOrigin::signed([11; 32].into()),
            0,
            crate::LegalProperty::SpvSide,
            4_000,
        ));
        assert_ok!(Marketplace::vote_on_spv_lawyer(
            RuntimeOrigin::signed([1; 32].into()),
            0,
            crate::Vote::Yes,
            30
        ));
        assert_ok!(Marketplace::vote_on_spv_lawyer(
            RuntimeOrigin::signed([2; 32].into()),
            0,
            crate::Vote::Yes,
            30
        ));
        run_to_block(91);
        assert_ok!(Marketplace::finalize_spv_lawyer(RuntimeOrigin::signed([1; 32].into()), 0,));
        assert_ok!(Marketplace::unfreeze_spv_lawyer_shares(
            RuntimeOrigin::signed([1; 32].into()),
            0,
        ));
        assert_ok!(Marketplace::unfreeze_spv_lawyer_shares(
            RuntimeOrigin::signed([2; 32].into()),
            0,
        ));
        assert_eq!(PropertyLawyer::<Test>::get(0).unwrap().spv_lawyer, Some([11; 32].into()));
        assert_ok!(Marketplace::lawyer_confirm_documents(
            RuntimeOrigin::signed([10; 32].into()),
            0,
            false,
        ));
        assert_eq!(
            PropertyLawyer::<Test>::get(0).unwrap().real_estate_developer_status,
            crate::DocumentStatus::Rejected
        );
        assert_eq!(pallet_nfts::Item::<Test>::get(0, 0).is_none(), false);
        assert_eq!(ForeignAssets::balance(1984, &[1; 32].into()), 1_188_000);
        assert_eq!(ForeignAssets::balance(1984, &[2; 32].into()), 838_000);
        assert_eq!(ForeignAssets::balance(1337, &[7; 32].into()), 84_000);
        assert_eq!(ForeignAssets::balance(1337, &[1; 32].into()), 1_500_000);
        assert_ok!(Marketplace::lawyer_confirm_documents(
            RuntimeOrigin::signed([11; 32].into()),
            0,
            false,
        ));

        // Fianl assertions after rejection
        assert_eq!(RefundShare::<Test>::get(0).unwrap().refund_amount, 100);
        assert_eq!(LocalAssets::balance(0, &[1; 32].into()), 30);
        assert_eq!(ShareOwner::<Test>::get::<AccountId, u32>([1; 32].into(), 0), None);
        assert_ok!(Marketplace::withdraw_rejected(RuntimeOrigin::signed([1; 32].into()), 0));
        assert_eq!(RefundShare::<Test>::get(0).unwrap().refund_amount, 70);
        assert_eq!(ForeignAssets::balance(1984, &[0; 32].into()), 20_000_000);
        assert_eq!(ForeignAssets::balance(1984, &Marketplace::treasury_account_id()), 0);
        assert_eq!(ForeignAssets::balance(1984, &[1; 32].into()), 1_497_000);
        assert_eq!(ForeignAssets::balance(1984, &[11; 32].into()), 0);
        assert_eq!(ForeignAssets::balance(1337, &[11; 32].into()), 0);
        assert_eq!(ShareOwner::<Test>::get::<AccountId, u32>([1; 32].into(), 0), None);
        assert_eq!(pallet_nfts::Item::<Test>::get(0, 0).is_none(), false);
        assert_eq!(ForeignAssets::balance(1984, &Marketplace::property_account_id(0)), 315000);
        assert_ok!(Marketplace::withdraw_rejected(RuntimeOrigin::signed([2; 32].into()), 0));
        assert_ok!(Marketplace::withdraw_rejected(RuntimeOrigin::signed([7; 32].into()), 0));
        assert_eq!(pallet_nfts::Item::<Test>::get(0, 0).is_none(), true);
        assert_eq!(ForeignAssets::balance(1984, &Marketplace::property_account_id(0)), 0);
        assert_eq!(ForeignAssets::balance(1984, &Marketplace::treasury_account_id()), 6000);
        assert_eq!(ForeignAssets::balance(1337, &Marketplace::treasury_account_id()), 0);
        assert_eq!(ForeignAssets::balance(1337, &[11; 32].into()), 4000);
        assert_eq!(ForeignAssets::balance(1984, &[11; 32].into()), 0);
        assert_eq!(RefundShare::<Test>::get(0).is_none(), true);
        assert!(PropertyLawyer::<Test>::get(0).is_none());
    })
}

// withdraw_legal_process_expired tests

#[test]
fn withdraw_legal_process_expired_works() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);
        // Region setup and role assignments
        assert_ok!(XcavateWhitelist::add_admin(
            RuntimeOrigin::root(),
            [20; 32].into(),
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [8; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));
        new_region_helper();
        assert_ok!(Regions::create_new_location(
            RuntimeOrigin::signed([8; 32].into()),
            3,
            bvec![10, 10]
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [0; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateDeveloper
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [1; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [2; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [30; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [31; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [10; 32].into(),
            pallet_xcavate_whitelist::Role::Lawyer
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [11; 32].into(),
            pallet_xcavate_whitelist::Role::Lawyer
        ));
        assert_ok!(Regions::register_lawyer(
            RuntimeOrigin::signed([10; 32].into()),
            3,
        ));
        assert_ok!(Regions::register_lawyer(
            RuntimeOrigin::signed([11; 32].into()),
            3,
        ));

        // Property listing and purchases
        assert_ok!(Marketplace::list_property(
            RuntimeOrigin::signed([0; 32].into()),
            3,
            bvec![10, 10],
            10_000,
            100,
            bvec![22, 22],
            false
        ));
        assert_ok!(Marketplace::buy_property_shares(
            RuntimeOrigin::signed([1; 32].into()),
            0,
            40,
            1337
        ));
        assert_ok!(Marketplace::buy_property_shares(
            RuntimeOrigin::signed([2; 32].into()),
            0,
            10,
            1984
        ));
        assert_ok!(Marketplace::buy_property_shares(
            RuntimeOrigin::signed([30; 32].into()),
            0,
            40,
            1984
        ));
        assert_ok!(Marketplace::buy_property_shares(
            RuntimeOrigin::signed([31; 32].into()),
            0,
            10,
            1984
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [5; 32].into(),
            pallet_xcavate_whitelist::Role::SpvConfirmation
        ));
        assert_ok!(Marketplace::create_spv(
            RuntimeOrigin::signed([5; 32].into()),
            0,
        ));
        assert_ok!(Marketplace::claim_property_shares(
            RuntimeOrigin::signed([1; 32].into()),
            0
        ));
        assert_ok!(Marketplace::claim_property_shares(
            RuntimeOrigin::signed([2; 32].into()),
            0
        ));
        assert_ok!(Marketplace::claim_property_shares(
            RuntimeOrigin::signed([30; 32].into()),
            0
        ));
        assert_ok!(Marketplace::claim_property_shares(
            RuntimeOrigin::signed([31; 32].into()),
            0
        ));

        // Legal process
        assert_ok!(Marketplace::lawyer_claim_property(
            RuntimeOrigin::signed([10; 32].into()),
            0,
            crate::LegalProperty::RealEstateDeveloperSide,
            4_000,
        ));
        assert_ok!(Marketplace::approve_developer_lawyer(
            RuntimeOrigin::signed([0; 32].into()),
            0,
            true
        ));
        assert_ok!(Marketplace::lawyer_claim_property(
            RuntimeOrigin::signed([11; 32].into()),
            0,
            crate::LegalProperty::SpvSide,
            4_000,
        ));
        assert_ok!(Marketplace::vote_on_spv_lawyer(
            RuntimeOrigin::signed([1; 32].into()),
            0,
            crate::Vote::Yes,
            40
        ));
        assert_ok!(Marketplace::vote_on_spv_lawyer(
            RuntimeOrigin::signed([30; 32].into()),
            0,
            crate::Vote::Yes,
            20
        ));
        run_to_block(91);
        assert_ok!(Marketplace::finalize_spv_lawyer(
            RuntimeOrigin::signed([1; 32].into()),
            0,
        ));
        assert_ok!(Marketplace::unfreeze_spv_lawyer_shares(
            RuntimeOrigin::signed([1; 32].into()),
            0,
        ));
        assert_ok!(Marketplace::unfreeze_spv_lawyer_shares(
            RuntimeOrigin::signed([30; 32].into()),
            0,
        ));
        assert_eq!(PropertyLawyer::<Test>::get(0).unwrap().legal_process_expiry, 161);
        run_to_block(162);

        // Legal process expired
        assert_noop!(
            Marketplace::lawyer_confirm_documents(
                RuntimeOrigin::signed([10; 32].into()),
                0,
                false,
            ),
            Error::<Test>::LegalProcessFailed
        );

        // Withdraw after legal process expiry
        assert_ok!(Marketplace::withdraw_legal_process_expired(
            RuntimeOrigin::signed([1; 32].into()),
            0,
        ));
        assert_eq!(ForeignAssets::balance(1984, &[1; 32].into()), 1_500_000);
        assert_eq!(ForeignAssets::balance(1337, &[1; 32].into()), 1_500_000);
        assert_ok!(Marketplace::withdraw_legal_process_expired(
            RuntimeOrigin::signed([2; 32].into()),
            0,
        ));
        assert_eq!(ForeignAssets::balance(1984, &[2; 32].into()), 1_150_000);
        assert_ok!(Marketplace::withdraw_legal_process_expired(
            RuntimeOrigin::signed([30; 32].into()),
            0,
        ));
        assert_ok!(Marketplace::withdraw_legal_process_expired(
            RuntimeOrigin::signed([31; 32].into()),
            0,
        ));
        assert_eq!(
            ForeignAssets::balance(1984, &Marketplace::property_account_id(0)),
            0
        );
        assert_eq!(
            ForeignAssets::balance(1337, &Marketplace::property_account_id(0)),
            0
        );
        assert_eq!(Nfts::owner(0, 0), None);
        assert_eq!(PropertyAssetInfo::<Test>::get(0), None);
        assert_eq!(OngoingObjectListing::<Test>::get(0), None);
        assert_eq!(PropertyLawyer::<Test>::get(0), None);
        assert_eq!(RefundLegalExpired::<Test>::get(0), None);
        assert_eq!(PropertyOwner::<Test>::get(0).len(), 0);
        assert_eq!(RealEstateLawyer::<Test>::get::<AccountId>([10; 32].into()).unwrap().active_cases, 0);
        assert_eq!(RealEstateLawyer::<Test>::get::<AccountId>([11; 32].into()).unwrap().active_cases, 0);
    })
}

#[test]
fn withdraw_legal_process_expired_works_2() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);
        // Region setup and role assignments
        assert_ok!(XcavateWhitelist::add_admin(RuntimeOrigin::root(), [20; 32].into(),));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [8; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));
        new_region_helper();
        assert_ok!(Regions::create_new_location(
            RuntimeOrigin::signed([8; 32].into()),
            3,
            bvec![10, 10]
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [0; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateDeveloper
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [1; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [2; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [30; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));

        // Property listing and purchases
        assert_ok!(Marketplace::list_property(
            RuntimeOrigin::signed([0; 32].into()),
            3,
            bvec![10, 10],
            10_000,
            100,
            bvec![22, 22],
            false
        ));
        assert_ok!(Marketplace::buy_property_shares(
            RuntimeOrigin::signed([1; 32].into()),
            0,
            40,
            1984
        ));
        assert_ok!(Marketplace::buy_property_shares(
            RuntimeOrigin::signed([2; 32].into()),
            0,
            20,
            1984
        ));
        assert_ok!(Marketplace::buy_property_shares(
            RuntimeOrigin::signed([30; 32].into()),
            0,
            40,
            1984
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [5; 32].into(),
            pallet_xcavate_whitelist::Role::SpvConfirmation
        ));
        assert_ok!(Marketplace::create_spv(RuntimeOrigin::signed([5; 32].into()), 0,));
        assert_ok!(Marketplace::claim_property_shares(RuntimeOrigin::signed([1; 32].into()), 0));
        assert_ok!(Marketplace::claim_property_shares(RuntimeOrigin::signed([2; 32].into()), 0));
        assert_ok!(Marketplace::claim_property_shares(RuntimeOrigin::signed([30; 32].into()), 0));

        // Legal process
        assert_eq!(PropertyLawyer::<Test>::get(0).unwrap().legal_process_expiry, 161);
        run_to_block(162);

        // Withdraw after legal process expiry
        assert_ok!(Marketplace::withdraw_legal_process_expired(
            RuntimeOrigin::signed([1; 32].into()),
            0,
        ));
        assert_eq!(ForeignAssets::balance(1984, &[1; 32].into()), 1_500_000);
        assert_eq!(ForeignAssets::balance(1337, &[1; 32].into()), 1_500_000);
        assert_ok!(Marketplace::withdraw_legal_process_expired(
            RuntimeOrigin::signed([2; 32].into()),
            0,
        ));
        assert_eq!(ForeignAssets::balance(1984, &[2; 32].into()), 1_150_000);
        assert_ok!(Marketplace::withdraw_legal_process_expired(
            RuntimeOrigin::signed([30; 32].into()),
            0,
        ));
        assert_eq!(ForeignAssets::balance(1984, &Marketplace::property_account_id(0)), 0);
        assert_eq!(ForeignAssets::balance(1337, &Marketplace::property_account_id(0)), 0);
        assert_eq!(Nfts::owner(0, 0), None);
        assert_eq!(PropertyAssetInfo::<Test>::get(0), None);
        assert_eq!(OngoingObjectListing::<Test>::get(0), None);
        assert_eq!(PropertyLawyer::<Test>::get(0), None);
        assert_eq!(RefundLegalExpired::<Test>::get(0), None);
        assert_eq!(PropertyOwner::<Test>::get(0).len(), 0);
    })
}

#[test]
fn withdraw_legal_process_expired_fails() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);
        // Region setup and role assignments
        assert_ok!(XcavateWhitelist::add_admin(RuntimeOrigin::root(), [20; 32].into(),));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [8; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));
        new_region_helper();
        assert_ok!(Regions::create_new_location(
            RuntimeOrigin::signed([8; 32].into()),
            3,
            bvec![10, 10]
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [0; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateDeveloper
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [1; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [2; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [30; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [10; 32].into(),
            pallet_xcavate_whitelist::Role::Lawyer
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [11; 32].into(),
            pallet_xcavate_whitelist::Role::Lawyer
        ));
        assert_ok!(Regions::register_lawyer(RuntimeOrigin::signed([10; 32].into()), 3,));
        assert_ok!(Regions::register_lawyer(RuntimeOrigin::signed([11; 32].into()), 3,));
        assert_noop!(
            Marketplace::withdraw_legal_process_expired(RuntimeOrigin::signed([1; 32].into()), 0),
            Error::<Test>::ListingNotFound
        );

        // Property listing and purchases
        assert_ok!(Marketplace::list_property(
            RuntimeOrigin::signed([0; 32].into()),
            3,
            bvec![10, 10],
            10_000,
            100,
            bvec![22, 22],
            false
        ));
        assert_ok!(Marketplace::buy_property_shares(
            RuntimeOrigin::signed([1; 32].into()),
            0,
            20,
            1984
        ));
        assert_ok!(Marketplace::buy_property_shares(
            RuntimeOrigin::signed([1; 32].into()),
            0,
            25,
            1337
        ));
        assert_ok!(Marketplace::buy_property_shares(
            RuntimeOrigin::signed([2; 32].into()),
            0,
            20,
            1984
        ));
        assert_ok!(Marketplace::buy_property_shares(
            RuntimeOrigin::signed([2; 32].into()),
            0,
            25,
            1337
        ));
        assert_ok!(Marketplace::buy_property_shares(
            RuntimeOrigin::signed([30; 32].into()),
            0,
            10,
            1984
        ));
        assert_noop!(
            Marketplace::withdraw_legal_process_expired(RuntimeOrigin::signed([1; 32].into()), 0),
            Error::<Test>::NoSharesOwned
        );
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [5; 32].into(),
            pallet_xcavate_whitelist::Role::SpvConfirmation
        ));
        assert_ok!(Marketplace::create_spv(RuntimeOrigin::signed([5; 32].into()), 0,));
        assert_ok!(Marketplace::claim_property_shares(RuntimeOrigin::signed([1; 32].into()), 0));
        assert_ok!(Marketplace::claim_property_shares(RuntimeOrigin::signed([2; 32].into()), 0));
        assert_ok!(Marketplace::claim_property_shares(RuntimeOrigin::signed([30; 32].into()), 0));

        // Legal process
        assert_ok!(Marketplace::lawyer_claim_property(
            RuntimeOrigin::signed([10; 32].into()),
            0,
            crate::LegalProperty::RealEstateDeveloperSide,
            4_000,
        ));
        assert_ok!(Marketplace::approve_developer_lawyer(
            RuntimeOrigin::signed([0; 32].into()),
            0,
            true
        ));
        assert_ok!(Marketplace::lawyer_claim_property(
            RuntimeOrigin::signed([11; 32].into()),
            0,
            crate::LegalProperty::SpvSide,
            4_000,
        ));
        assert_ok!(Marketplace::vote_on_spv_lawyer(
            RuntimeOrigin::signed([1; 32].into()),
            0,
            crate::Vote::Yes,
            40
        ));
        assert_ok!(Marketplace::vote_on_spv_lawyer(
            RuntimeOrigin::signed([2; 32].into()),
            0,
            crate::Vote::Yes,
            40
        ));
        run_to_block(91);
        assert_ok!(Marketplace::finalize_spv_lawyer(RuntimeOrigin::signed([1; 32].into()), 0,));
        assert_eq!(PropertyLawyer::<Test>::get(0).unwrap().legal_process_expiry, 161);
        assert_noop!(
            Marketplace::withdraw_legal_process_expired(RuntimeOrigin::signed([1; 32].into()), 0),
            Error::<Test>::LegalProcessOngoing
        );
        run_to_block(162);
        assert_noop!(
            Marketplace::withdraw_legal_process_expired(RuntimeOrigin::signed([3; 32].into()), 0,),
            BadOrigin
        );
    })
}

#[test]
fn second_attempt_works() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);

        // Region setup and role assignments
        assert_ok!(XcavateWhitelist::add_admin(RuntimeOrigin::root(), [20; 32].into(),));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [8; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));
        new_region_helper();
        assert_ok!(Regions::create_new_location(
            RuntimeOrigin::signed([8; 32].into()),
            3,
            bvec![10, 10]
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [0; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateDeveloper
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [1; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [2; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [30; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [10; 32].into(),
            pallet_xcavate_whitelist::Role::Lawyer
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [11; 32].into(),
            pallet_xcavate_whitelist::Role::Lawyer
        ));
        assert_ok!(Regions::register_lawyer(RuntimeOrigin::signed([10; 32].into()), 3,));
        assert_ok!(Regions::register_lawyer(RuntimeOrigin::signed([11; 32].into()), 3,));

        // Property listing and purchases
        assert_ok!(Marketplace::list_property(
            RuntimeOrigin::signed([0; 32].into()),
            3,
            bvec![10, 10],
            10_000,
            100,
            bvec![22, 22],
            false
        ));
        assert_ok!(Marketplace::buy_property_shares(
            RuntimeOrigin::signed([1; 32].into()),
            0,
            40,
            1984
        ));
        assert_ok!(Marketplace::buy_property_shares(
            RuntimeOrigin::signed([2; 32].into()),
            0,
            30,
            1984
        ));
        assert_ok!(Marketplace::buy_property_shares(
            RuntimeOrigin::signed([30; 32].into()),
            0,
            30,
            1984
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [5; 32].into(),
            pallet_xcavate_whitelist::Role::SpvConfirmation
        ));
        assert_ok!(Marketplace::create_spv(RuntimeOrigin::signed([5; 32].into()), 0,));
        assert_ok!(Marketplace::claim_property_shares(RuntimeOrigin::signed([1; 32].into()), 0));
        assert_ok!(Marketplace::claim_property_shares(RuntimeOrigin::signed([2; 32].into()), 0));
        assert_ok!(Marketplace::claim_property_shares(RuntimeOrigin::signed([30; 32].into()), 0));

        // Legal process
        assert_ok!(Marketplace::lawyer_claim_property(
            RuntimeOrigin::signed([10; 32].into()),
            0,
            crate::LegalProperty::RealEstateDeveloperSide,
            4_000,
        ));
        assert_ok!(Marketplace::approve_developer_lawyer(
            RuntimeOrigin::signed([0; 32].into()),
            0,
            true
        ));
        assert_eq!(
            PropertyLawyer::<Test>::get(0).unwrap().real_estate_developer_lawyer,
            Some([10; 32].into())
        );
        assert_ok!(Marketplace::lawyer_claim_property(
            RuntimeOrigin::signed([11; 32].into()),
            0,
            crate::LegalProperty::SpvSide,
            4_000,
        ));
        assert_ok!(Marketplace::vote_on_spv_lawyer(
            RuntimeOrigin::signed([1; 32].into()),
            0,
            crate::Vote::Yes,
            40
        ));
        assert_ok!(Marketplace::vote_on_spv_lawyer(
            RuntimeOrigin::signed([2; 32].into()),
            0,
            crate::Vote::Yes,
            30
        ));
        run_to_block(91);
        assert_ok!(Marketplace::finalize_spv_lawyer(RuntimeOrigin::signed([1; 32].into()), 0,));
        assert_ok!(Marketplace::unfreeze_spv_lawyer_shares(
            RuntimeOrigin::signed([1; 32].into()),
            0,
        ));
        assert_ok!(Marketplace::unfreeze_spv_lawyer_shares(
            RuntimeOrigin::signed([2; 32].into()),
            0,
        ));
        assert_eq!(PropertyLawyer::<Test>::get(0).unwrap().spv_lawyer, Some([11; 32].into()));
        assert_ok!(Marketplace::lawyer_confirm_documents(
            RuntimeOrigin::signed([10; 32].into()),
            0,
            true,
        ));
        assert_eq!(
            PropertyLawyer::<Test>::get(0).unwrap().real_estate_developer_status,
            crate::DocumentStatus::Approved
        );
        assert_eq!(pallet_nfts::Item::<Test>::get(0, 0).is_none(), false);
        assert_ok!(Marketplace::lawyer_confirm_documents(
            RuntimeOrigin::signed([11; 32].into()),
            0,
            false,
        ));
        assert_eq!(PropertyLawyer::<Test>::get(0).unwrap().second_attempt, true);
        assert_ok!(Marketplace::lawyer_confirm_documents(
            RuntimeOrigin::signed([10; 32].into()),
            0,
            false,
        ));
        assert_eq!(
            PropertyLawyer::<Test>::get(0).unwrap().real_estate_developer_status,
            crate::DocumentStatus::Rejected
        );
        assert_eq!(pallet_nfts::Item::<Test>::get(0, 0).is_none(), false);
        assert_ok!(Marketplace::lawyer_confirm_documents(
            RuntimeOrigin::signed([11; 32].into()),
            0,
            true,
        ));

        // Refund and final assertions after rejection
        assert_ok!(Marketplace::withdraw_rejected(RuntimeOrigin::signed([1; 32].into()), 0));
        assert_ok!(Marketplace::withdraw_rejected(RuntimeOrigin::signed([2; 32].into()), 0));
        assert_ok!(Marketplace::withdraw_rejected(RuntimeOrigin::signed([30; 32].into()), 0));
        assert_eq!(ForeignAssets::balance(1984, &[0; 32].into()), 20_000_000);
        assert_eq!(ForeignAssets::balance(1984, &Marketplace::treasury_account_id()), 6000);
        assert_eq!(ForeignAssets::balance(1984, &[1; 32].into()), 1_496_000);
        assert_eq!(ForeignAssets::balance(1984, &[2; 32].into()), 1_147_000);
        assert_eq!(ForeignAssets::balance(1984, &[30; 32].into()), 1_197_000);
        assert_eq!(ForeignAssets::balance(1984, &[11; 32].into()), 4_000);
        assert_eq!(PropertyAssetInfo::<Test>::get(0).is_none(), true);
        assert_eq!(OngoingObjectListing::<Test>::get(0), None);
        assert_eq!(ShareOwner::<Test>::get::<AccountId, u32>([1; 32].into(), 0), None);
        assert_eq!(pallet_nfts::Item::<Test>::get(0, 0).is_none(), true);
    })
}

#[test]
fn lawyer_confirm_documents_fails() {
    new_test_ext().execute_with(|| {
		System::set_block_number(1);
        // Region setup and role assignments
        assert_ok!(XcavateWhitelist::add_admin(
            RuntimeOrigin::root(),
            [20; 32].into(),
        ));
		assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [8; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));
		new_region_helper();
		assert_ok!(Regions::create_new_location(RuntimeOrigin::signed([8; 32].into()), 3, bvec![10, 10]));
		assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [0; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateDeveloper
        ));
		assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [1; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [2; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [30; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [10; 32].into(),
            pallet_xcavate_whitelist::Role::Lawyer
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [11; 32].into(),
            pallet_xcavate_whitelist::Role::Lawyer
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [12; 32].into(),
            pallet_xcavate_whitelist::Role::Lawyer
        ));
		assert_ok!(Regions::register_lawyer(RuntimeOrigin::signed([10; 32].into()), 3));
		assert_ok!(Regions::register_lawyer(RuntimeOrigin::signed([11; 32].into()), 3));
		assert_ok!(Regions::register_lawyer(RuntimeOrigin::signed([12; 32].into()), 3));

        // Property listing and purchases
		assert_ok!(Marketplace::list_property(
			RuntimeOrigin::signed([0; 32].into()),
			3,
			bvec![10, 10],
			10_000,
			100,
			bvec![22, 22],
			false
		));
		assert_ok!(Marketplace::buy_property_shares(RuntimeOrigin::signed([1; 32].into()), 0, 40, 1984));
        assert_ok!(Marketplace::buy_property_shares(RuntimeOrigin::signed([2; 32].into()), 0, 30, 1984));
        assert_ok!(Marketplace::buy_property_shares(RuntimeOrigin::signed([30; 32].into()), 0, 30, 1984));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [5; 32].into(),
            pallet_xcavate_whitelist::Role::SpvConfirmation
        ));
        assert_ok!(Marketplace::create_spv(
            RuntimeOrigin::signed([5; 32].into()),
            0,
        ));
        assert_ok!(Marketplace::claim_property_shares(RuntimeOrigin::signed([1; 32].into()), 0));
        assert_ok!(Marketplace::claim_property_shares(RuntimeOrigin::signed([2; 32].into()), 0));
        assert_ok!(Marketplace::claim_property_shares(RuntimeOrigin::signed([30; 32].into()), 0));

        // Legal process
		assert_ok!(Marketplace::lawyer_claim_property(
			RuntimeOrigin::signed([10; 32].into()),
			0,
			crate::LegalProperty::RealEstateDeveloperSide,
			4_000,
		));
        assert_ok!(Marketplace::approve_developer_lawyer(
			RuntimeOrigin::signed([0; 32].into()),
			0,
            true
		));
		assert_eq!(PropertyLawyer::<Test>::get(0).unwrap().real_estate_developer_lawyer, Some([10; 32].into()));
		assert_ok!(Marketplace::lawyer_claim_property(
			RuntimeOrigin::signed([11; 32].into()),
			0,
			crate::LegalProperty::SpvSide,
			4_000,
		));
        assert_ok!(Marketplace::vote_on_spv_lawyer(
			RuntimeOrigin::signed([1; 32].into()),
			0,
            crate::Vote::Yes,
            40
		));
        assert_ok!(Marketplace::vote_on_spv_lawyer(
			RuntimeOrigin::signed([2; 32].into()),
			0,
            crate::Vote::Yes,
            30
		));
        run_to_block(91);
        assert_ok!(Marketplace::finalize_spv_lawyer(
			RuntimeOrigin::signed([1; 32].into()),
			0,
		));
		assert_eq!(PropertyLawyer::<Test>::get(0).unwrap().spv_lawyer, Some([11; 32].into()));
		assert_noop!(Marketplace::lawyer_confirm_documents(
			RuntimeOrigin::signed([10; 32].into()),
			1,
			false,
		), Error::<Test>::InvalidIndex);
		assert_noop!(Marketplace::lawyer_confirm_documents(
			RuntimeOrigin::signed([12; 32].into()),
			0,
			false,
		), Error::<Test>::NoPermission);
		assert_ok!(Marketplace::lawyer_confirm_documents(
			RuntimeOrigin::signed([10; 32].into()),
			0,
			false,
		));
		assert_noop!(Marketplace::lawyer_confirm_documents(
			RuntimeOrigin::signed([10; 32].into()),
			0,
			true,
		), Error::<Test>::AlreadyConfirmed);
        run_to_block(200);
        assert_noop!(Marketplace::lawyer_confirm_documents(
			RuntimeOrigin::signed([11; 32].into()),
			0,
			true,
		), Error::<Test>::LegalProcessFailed);
	})
}

// relist_shares tests

#[test]
fn relist_shares_works() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);
        // Region setup and role assignments
        assert_ok!(XcavateWhitelist::add_admin(RuntimeOrigin::root(), [20; 32].into(),));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [8; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));
        new_region_helper();
        assert_ok!(Regions::create_new_location(
            RuntimeOrigin::signed([8; 32].into()),
            3,
            bvec![10, 10]
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [0; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateDeveloper
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [1; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [2; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [30; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [10; 32].into(),
            pallet_xcavate_whitelist::Role::Lawyer
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [11; 32].into(),
            pallet_xcavate_whitelist::Role::Lawyer
        ));
        assert_ok!(Regions::register_lawyer(RuntimeOrigin::signed([10; 32].into()), 3,));
        assert_ok!(Regions::register_lawyer(RuntimeOrigin::signed([11; 32].into()), 3,));

        // Property listing and purchases
        assert_ok!(Marketplace::list_property(
            RuntimeOrigin::signed([0; 32].into()),
            3,
            bvec![10, 10],
            10_000,
            100,
            bvec![22, 22],
            false
        ));
        assert_ok!(Marketplace::buy_property_shares(
            RuntimeOrigin::signed([1; 32].into()),
            0,
            40,
            1984
        ));
        assert_ok!(Marketplace::buy_property_shares(
            RuntimeOrigin::signed([2; 32].into()),
            0,
            30,
            1984
        ));
        assert_ok!(Marketplace::buy_property_shares(
            RuntimeOrigin::signed([30; 32].into()),
            0,
            30,
            1984
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [5; 32].into(),
            pallet_xcavate_whitelist::Role::SpvConfirmation
        ));
        assert_ok!(Marketplace::create_spv(RuntimeOrigin::signed([5; 32].into()), 0,));
        assert_ok!(Marketplace::claim_property_shares(RuntimeOrigin::signed([1; 32].into()), 0,));
        assert_ok!(Marketplace::claim_property_shares(RuntimeOrigin::signed([2; 32].into()), 0,));
        assert_ok!(Marketplace::claim_property_shares(RuntimeOrigin::signed([30; 32].into()), 0,));

        // Legal process
        assert_ok!(Marketplace::lawyer_claim_property(
            RuntimeOrigin::signed([10; 32].into()),
            0,
            crate::LegalProperty::RealEstateDeveloperSide,
            4_000,
        ));
        assert_ok!(Marketplace::approve_developer_lawyer(
            RuntimeOrigin::signed([0; 32].into()),
            0,
            true
        ));
        assert_ok!(Marketplace::lawyer_claim_property(
            RuntimeOrigin::signed([11; 32].into()),
            0,
            crate::LegalProperty::SpvSide,
            4_000,
        ));
        assert_ok!(Marketplace::vote_on_spv_lawyer(
            RuntimeOrigin::signed([1; 32].into()),
            0,
            crate::Vote::Yes,
            40
        ));
        assert_ok!(Marketplace::vote_on_spv_lawyer(
            RuntimeOrigin::signed([2; 32].into()),
            0,
            crate::Vote::Yes,
            30
        ));
        run_to_block(91);
        assert_ok!(Marketplace::finalize_spv_lawyer(RuntimeOrigin::signed([1; 32].into()), 0,));
        assert_ok!(Marketplace::unfreeze_spv_lawyer_shares(
            RuntimeOrigin::signed([1; 32].into()),
            0,
        ));
        assert_ok!(Marketplace::lawyer_confirm_documents(
            RuntimeOrigin::signed([10; 32].into()),
            0,
            true,
        ));
        assert_ok!(Marketplace::lawyer_confirm_documents(
            RuntimeOrigin::signed([11; 32].into()),
            0,
            true,
        ));
        assert_eq!(PropertyAssetInfo::<Test>::get(0).unwrap().spv_created, true);

        // Relist shares on secondary market
        assert_ok!(Marketplace::relist_shares(RuntimeOrigin::signed([1; 32].into()), 0, 1000, 1));
        assert_eq!(ShareListings::<Test>::get(1).is_some(), true);
        assert_eq!(ShareListings::<Test>::get(1).unwrap().item_id, 0);
        assert_eq!(LocalAssets::balance(0, &[1; 32].into()), 39);
        assert_eq!(LocalAssets::balance(0, &Marketplace::property_account_id(0)), 1);
    })
}

#[test]
fn relist_property_shares_not_created_with_marketplace_fails() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);
        assert_ok!(XcavateWhitelist::add_admin(RuntimeOrigin::root(), [20; 32].into(),));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [0; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateDeveloper
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [0; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));
        assert_ok!(Nfts::create(
            RuntimeOrigin::signed([0; 32].into()),
            sp_runtime::MultiAddress::Id([0; 32].into()),
            Default::default()
        ));
        assert_ok!(Nfts::mint(
            RuntimeOrigin::signed([0; 32].into()),
            0,
            0,
            sp_runtime::MultiAddress::Id([0; 32].into()),
            None
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [8; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));
        new_region_helper();
        assert_noop!(
            Marketplace::relist_shares(RuntimeOrigin::signed([0; 32].into()), 0, 1000, 1),
            RealWorldAssetError::<Test>::PropertyNotFound
        );
    })
}

#[test]
fn relist_shares_fails() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);
        // Region setup and role assignments
        assert_ok!(XcavateWhitelist::add_admin(RuntimeOrigin::root(), [20; 32].into(),));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [8; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));
        new_region_helper();
        assert_ok!(Regions::create_new_location(
            RuntimeOrigin::signed([8; 32].into()),
            3,
            bvec![10, 10]
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [0; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateDeveloper
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [0; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [1; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [2; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [30; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [10; 32].into(),
            pallet_xcavate_whitelist::Role::Lawyer
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [11; 32].into(),
            pallet_xcavate_whitelist::Role::Lawyer
        ));
        assert_ok!(Regions::register_lawyer(RuntimeOrigin::signed([10; 32].into()), 3,));
        assert_ok!(Regions::register_lawyer(RuntimeOrigin::signed([11; 32].into()), 3,));
        // Property listing and purchases
        assert_ok!(Marketplace::list_property(
            RuntimeOrigin::signed([0; 32].into()),
            3,
            bvec![10, 10],
            10_000,
            100,
            bvec![22, 22],
            false
        ));
        assert_ok!(Marketplace::buy_property_shares(
            RuntimeOrigin::signed([1; 32].into()),
            0,
            40,
            1984
        ));
        assert_ok!(Marketplace::buy_property_shares(
            RuntimeOrigin::signed([2; 32].into()),
            0,
            30,
            1984
        ));
        assert_ok!(Marketplace::buy_property_shares(
            RuntimeOrigin::signed([30; 32].into()),
            0,
            30,
            1984
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [5; 32].into(),
            pallet_xcavate_whitelist::Role::SpvConfirmation
        ));
        assert_ok!(Marketplace::create_spv(RuntimeOrigin::signed([5; 32].into()), 0,));
        assert_ok!(Marketplace::claim_property_shares(RuntimeOrigin::signed([1; 32].into()), 0,));
        assert_ok!(Marketplace::claim_property_shares(RuntimeOrigin::signed([2; 32].into()), 0,));
        assert_ok!(Marketplace::claim_property_shares(RuntimeOrigin::signed([30; 32].into()), 0,));
        // Legal process
        assert_noop!(
            Marketplace::relist_shares(RuntimeOrigin::signed([1; 32].into()), 0, 1000, 10),
            RealWorldAssetError::<Test>::PropertyNotFinalized
        );
        assert_ok!(Marketplace::lawyer_claim_property(
            RuntimeOrigin::signed([10; 32].into()),
            0,
            crate::LegalProperty::RealEstateDeveloperSide,
            4_000,
        ));
        assert_ok!(Marketplace::approve_developer_lawyer(
            RuntimeOrigin::signed([0; 32].into()),
            0,
            true
        ));
        assert_ok!(Marketplace::lawyer_claim_property(
            RuntimeOrigin::signed([11; 32].into()),
            0,
            crate::LegalProperty::SpvSide,
            4_000,
        ));
        assert_ok!(Marketplace::vote_on_spv_lawyer(
            RuntimeOrigin::signed([1; 32].into()),
            0,
            crate::Vote::Yes,
            40
        ));
        assert_ok!(Marketplace::vote_on_spv_lawyer(
            RuntimeOrigin::signed([2; 32].into()),
            0,
            crate::Vote::Yes,
            30
        ));
        run_to_block(91);
        assert_ok!(Marketplace::finalize_spv_lawyer(RuntimeOrigin::signed([1; 32].into()), 0,));
        assert_ok!(Marketplace::lawyer_confirm_documents(
            RuntimeOrigin::signed([10; 32].into()),
            0,
            true,
        ));
        assert_ok!(Marketplace::lawyer_confirm_documents(
            RuntimeOrigin::signed([11; 32].into()),
            0,
            true,
        ));
        assert_eq!(PropertyAssetInfo::<Test>::get(0).unwrap().spv_created, true);
        // Relist shares on secondary market failure cases
        assert_noop!(
            Marketplace::relist_shares(RuntimeOrigin::signed([0; 32].into()), 0, 1000, 1),
            TokenError::FundsUnavailable
        );
        assert_noop!(
            Marketplace::relist_shares(RuntimeOrigin::signed([1; 32].into()), 0, 1000, 0),
            Error::<Test>::AmountCannotBeZero
        );
        assert_noop!(
            Marketplace::relist_shares(RuntimeOrigin::signed([1; 32].into()), 0, 0, 1),
            Error::<Test>::InvalidSharePrice
        );
        assert_ok!(XcavateWhitelist::set_permission(
            RuntimeOrigin::signed([20; 32].into()),
            [1; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor,
            pallet_xcavate_whitelist::AccessPermission::Revoked,
        ));
        assert_noop!(
            Marketplace::relist_shares(RuntimeOrigin::signed([1; 32].into()), 0, 1000, 20),
            BadOrigin
        );
    })
}

// buy_relisted_shares tests

#[test]
fn buy_relisted_shares_works() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);
        // Region setup and role assignments
        assert_ok!(XcavateWhitelist::add_admin(RuntimeOrigin::root(), [20; 32].into(),));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [8; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));
        new_region_helper();
        assert_ok!(Regions::create_new_location(
            RuntimeOrigin::signed([8; 32].into()),
            3,
            bvec![10, 10]
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [0; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateDeveloper
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [1; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [2; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [3; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [30; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [31; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [10; 32].into(),
            pallet_xcavate_whitelist::Role::Lawyer
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [11; 32].into(),
            pallet_xcavate_whitelist::Role::Lawyer
        ));
        assert_ok!(Regions::register_lawyer(RuntimeOrigin::signed([10; 32].into()), 3,));
        assert_ok!(Regions::register_lawyer(RuntimeOrigin::signed([11; 32].into()), 3,));
        // Property listing and purchases
        assert_ok!(Marketplace::list_property(
            RuntimeOrigin::signed([0; 32].into()),
            3,
            bvec![10, 10],
            10_000,
            100,
            bvec![22, 22],
            false
        ));
        assert_ok!(Marketplace::buy_property_shares(
            RuntimeOrigin::signed([1; 32].into()),
            0,
            3,
            1984
        ));
        assert_ok!(Marketplace::buy_property_shares(
            RuntimeOrigin::signed([2; 32].into()),
            0,
            47,
            1984
        ));
        assert_ok!(Marketplace::buy_property_shares(
            RuntimeOrigin::signed([30; 32].into()),
            0,
            30,
            1984
        ));
        assert_ok!(Marketplace::buy_property_shares(
            RuntimeOrigin::signed([31; 32].into()),
            0,
            20,
            1984
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [5; 32].into(),
            pallet_xcavate_whitelist::Role::SpvConfirmation
        ));
        assert_ok!(Marketplace::create_spv(RuntimeOrigin::signed([5; 32].into()), 0,));
        assert_ok!(Marketplace::claim_property_shares(RuntimeOrigin::signed([1; 32].into()), 0,));
        assert_ok!(Marketplace::claim_property_shares(RuntimeOrigin::signed([2; 32].into()), 0,));
        assert_ok!(Marketplace::claim_property_shares(RuntimeOrigin::signed([30; 32].into()), 0,));
        assert_ok!(Marketplace::claim_property_shares(RuntimeOrigin::signed([31; 32].into()), 0,));
        // Legal process
        assert_ok!(Marketplace::lawyer_claim_property(
            RuntimeOrigin::signed([10; 32].into()),
            0,
            crate::LegalProperty::RealEstateDeveloperSide,
            4_000,
        ));
        assert_ok!(Marketplace::approve_developer_lawyer(
            RuntimeOrigin::signed([0; 32].into()),
            0,
            true
        ));
        assert_ok!(Marketplace::lawyer_claim_property(
            RuntimeOrigin::signed([11; 32].into()),
            0,
            crate::LegalProperty::SpvSide,
            4_000,
        ));
        assert_ok!(Marketplace::vote_on_spv_lawyer(
            RuntimeOrigin::signed([1; 32].into()),
            0,
            crate::Vote::Yes,
            3
        ));
        assert_ok!(Marketplace::vote_on_spv_lawyer(
            RuntimeOrigin::signed([2; 32].into()),
            0,
            crate::Vote::Yes,
            44
        ));
        assert_ok!(Marketplace::vote_on_spv_lawyer(
            RuntimeOrigin::signed([30; 32].into()),
            0,
            crate::Vote::Yes,
            4
        ));
        run_to_block(91);
        assert_ok!(Marketplace::finalize_spv_lawyer(RuntimeOrigin::signed([1; 32].into()), 0,));
        assert_ok!(Marketplace::unfreeze_spv_lawyer_shares(
            RuntimeOrigin::signed([1; 32].into()),
            0,
        ));
        assert_ok!(Marketplace::lawyer_confirm_documents(
            RuntimeOrigin::signed([10; 32].into()),
            0,
            true,
        ));
        assert_ok!(Marketplace::lawyer_confirm_documents(
            RuntimeOrigin::signed([11; 32].into()),
            0,
            true,
        ));
        assert_eq!(ForeignAssets::balance(1984, &([0; 32].into())), 20990000);
        assert_eq!(ForeignAssets::balance(1984, &Marketplace::treasury_account_id()), 8000);
        assert_eq!(ForeignAssets::balance(1984, &([8; 32].into())), 8000);
        assert_eq!(ForeignAssets::balance(1984, &([1; 32].into())), 1_468_800);
        assert_eq!(PropertyAssetInfo::<Test>::get(0).unwrap().spv_created, true);

        // Relist shares on secondary market and buy relisted shares
        assert_ok!(Marketplace::relist_shares(RuntimeOrigin::signed([2; 32].into()), 0, 1000, 3));
        assert_ok!(Marketplace::buy_relisted_shares(
            RuntimeOrigin::signed([3; 32].into()),
            1,
            2,
            1984
        ));
        assert_eq!(ForeignAssets::balance(1984, &([3; 32].into())), 3_000);
        assert_eq!(LocalAssets::balance(0, &[3; 32].into()), 2);
        assert_eq!(ShareListings::<Test>::get(1).is_some(), true);
        assert_ok!(Marketplace::buy_relisted_shares(
            RuntimeOrigin::signed([3; 32].into()),
            1,
            1,
            1984
        ));
        assert_eq!(ForeignAssets::balance(1984, &([3; 32].into())), 2_000);
        assert_eq!(ShareListings::<Test>::get(1).is_some(), false);
        assert_ok!(Marketplace::relist_shares(RuntimeOrigin::signed([1; 32].into()), 0, 500, 1));
        assert_ok!(Marketplace::buy_relisted_shares(
            RuntimeOrigin::signed([3; 32].into()),
            2,
            1,
            1984
        ));
        assert_eq!(ShareListings::<Test>::get(0).is_some(), false);
        assert_eq!(PropertyOwner::<Test>::get(0).len(), 5);
        assert_eq!(PropertyOwnerShares::<Test>::get::<u32, AccountId>(0, [1; 32].into()), 2);
        assert_eq!(PropertyOwnerShares::<Test>::get::<u32, AccountId>(0, [3; 32].into()), 4);
        assert_eq!(ForeignAssets::balance(1984, &([1; 32].into())), 1_469_295);
        assert_eq!(ForeignAssets::balance(1984, &([3; 32].into())), 1_500);
        assert_eq!(LocalAssets::balance(0, &[1; 32].into()), 2);
        assert_eq!(LocalAssets::balance(0, &[3; 32].into()), 4);
    })
}

#[test]
fn buy_relisted_shares_fails() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);
        // Region setup and role assignments
        assert_ok!(XcavateWhitelist::add_admin(RuntimeOrigin::root(), [20; 32].into(),));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [8; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));
        new_region_helper();
        assert_ok!(Regions::create_new_location(
            RuntimeOrigin::signed([8; 32].into()),
            3,
            bvec![10, 10]
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [0; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateDeveloper
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [1; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [2; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [3; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [30; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [10; 32].into(),
            pallet_xcavate_whitelist::Role::Lawyer
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [11; 32].into(),
            pallet_xcavate_whitelist::Role::Lawyer
        ));
        assert_ok!(Regions::register_lawyer(RuntimeOrigin::signed([10; 32].into()), 3,));
        assert_ok!(Regions::register_lawyer(RuntimeOrigin::signed([11; 32].into()), 3,));

        // Property listing and purchases
        assert_ok!(Marketplace::list_property(
            RuntimeOrigin::signed([0; 32].into()),
            3,
            bvec![10, 10],
            10_000,
            100,
            bvec![22, 22],
            false
        ));
        assert_ok!(Marketplace::buy_property_shares(
            RuntimeOrigin::signed([1; 32].into()),
            0,
            40,
            1984
        ));
        assert_ok!(Marketplace::buy_property_shares(
            RuntimeOrigin::signed([2; 32].into()),
            0,
            30,
            1984
        ));
        assert_ok!(Marketplace::buy_property_shares(
            RuntimeOrigin::signed([30; 32].into()),
            0,
            30,
            1984
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [5; 32].into(),
            pallet_xcavate_whitelist::Role::SpvConfirmation
        ));
        assert_ok!(Marketplace::create_spv(RuntimeOrigin::signed([5; 32].into()), 0,));
        assert_ok!(Marketplace::claim_property_shares(RuntimeOrigin::signed([1; 32].into()), 0,));
        assert_ok!(Marketplace::claim_property_shares(RuntimeOrigin::signed([2; 32].into()), 0,));
        assert_ok!(Marketplace::claim_property_shares(RuntimeOrigin::signed([30; 32].into()), 0,));

        // Legal process
        assert_ok!(Marketplace::lawyer_claim_property(
            RuntimeOrigin::signed([10; 32].into()),
            0,
            crate::LegalProperty::RealEstateDeveloperSide,
            4_000,
        ));
        assert_ok!(Marketplace::approve_developer_lawyer(
            RuntimeOrigin::signed([0; 32].into()),
            0,
            true
        ));
        assert_ok!(Marketplace::lawyer_claim_property(
            RuntimeOrigin::signed([11; 32].into()),
            0,
            crate::LegalProperty::SpvSide,
            4_000,
        ));
        assert_ok!(Marketplace::vote_on_spv_lawyer(
            RuntimeOrigin::signed([1; 32].into()),
            0,
            crate::Vote::Yes,
            40
        ));
        assert_ok!(Marketplace::vote_on_spv_lawyer(
            RuntimeOrigin::signed([2; 32].into()),
            0,
            crate::Vote::Yes,
            30
        ));
        run_to_block(91);
        assert_ok!(Marketplace::finalize_spv_lawyer(RuntimeOrigin::signed([1; 32].into()), 0,));
        assert_ok!(Marketplace::unfreeze_spv_lawyer_shares(
            RuntimeOrigin::signed([1; 32].into()),
            0,
        ));
        assert_ok!(Marketplace::unfreeze_spv_lawyer_shares(
            RuntimeOrigin::signed([2; 32].into()),
            0,
        ));
        assert_ok!(Marketplace::lawyer_confirm_documents(
            RuntimeOrigin::signed([10; 32].into()),
            0,
            true,
        ));
        assert_ok!(Marketplace::lawyer_confirm_documents(
            RuntimeOrigin::signed([11; 32].into()),
            0,
            true,
        ));
        assert_eq!(ForeignAssets::balance(1984, &([0; 32].into())), 20990000);
        assert_eq!(ForeignAssets::balance(1984, &Marketplace::treasury_account_id()), 8_000);
        assert_eq!(ForeignAssets::balance(1984, &([8; 32].into())), 8_000);
        assert_eq!(ForeignAssets::balance(1984, &([1; 32].into())), 1_084_000);
        assert_eq!(PropertyAssetInfo::<Test>::get(0).unwrap().spv_created, true);

        // Relist shares on secondary market and buy relisted shares failure cases
        assert_noop!(
            Marketplace::buy_relisted_shares(RuntimeOrigin::signed([3; 32].into()), 1, 1, 1984),
            Error::<Test>::ShareNotForSale
        );
        assert_ok!(Marketplace::relist_shares(RuntimeOrigin::signed([1; 32].into()), 0, 500, 1));
        assert_noop!(
            Marketplace::buy_relisted_shares(RuntimeOrigin::signed([3; 32].into()), 1, 1, 1983),
            Error::<Test>::PaymentAssetNotSupported
        );
        assert_ok!(XcavateWhitelist::set_permission(
            RuntimeOrigin::signed([20; 32].into()),
            [3; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor,
            pallet_xcavate_whitelist::AccessPermission::Revoked,
        ));
        assert_noop!(
            Marketplace::buy_relisted_shares(RuntimeOrigin::signed([3; 32].into()), 1, 1, 1984),
            BadOrigin
        );
        assert_ok!(Marketplace::relist_shares(RuntimeOrigin::signed([2; 32].into()), 0, 1_000, 20));
        assert_eq!(ShareListings::<Test>::get(2).unwrap().amount, 20);
        assert_eq!(PropertyOwnerShares::<Test>::get::<u32, AccountId>(0, [1; 32].into()), 40);
        assert_noop!(
            Marketplace::buy_relisted_shares(RuntimeOrigin::signed([1; 32].into()), 2, 10, 1984),
            Error::<Test>::ExceedsMaxOwnership
        );
    })
}

// make_offer tests

#[test]
fn make_offer_works() {
    new_test_ext().execute_with(|| {
        // Region setup and role assignments
        assert_ok!(XcavateWhitelist::add_admin(RuntimeOrigin::root(), [20; 32].into(),));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [8; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));
        new_region_helper();
        assert_ok!(Regions::create_new_location(
            RuntimeOrigin::signed([8; 32].into()),
            3,
            bvec![10, 10]
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [0; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateDeveloper
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [1; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [2; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [30; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [31; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [10; 32].into(),
            pallet_xcavate_whitelist::Role::Lawyer
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [11; 32].into(),
            pallet_xcavate_whitelist::Role::Lawyer
        ));
        assert_ok!(Regions::register_lawyer(RuntimeOrigin::signed([10; 32].into()), 3,));
        assert_ok!(Regions::register_lawyer(RuntimeOrigin::signed([11; 32].into()), 3,));
        // Property listing and purchases
        assert_ok!(Marketplace::list_property(
            RuntimeOrigin::signed([0; 32].into()),
            3,
            bvec![10, 10],
            10_000,
            100,
            bvec![22, 22],
            false
        ));
        assert_ok!(Marketplace::buy_property_shares(
            RuntimeOrigin::signed([1; 32].into()),
            0,
            40,
            1984
        ));
        assert_ok!(Marketplace::buy_property_shares(
            RuntimeOrigin::signed([30; 32].into()),
            0,
            30,
            1984
        ));
        assert_ok!(Marketplace::buy_property_shares(
            RuntimeOrigin::signed([31; 32].into()),
            0,
            30,
            1984
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [5; 32].into(),
            pallet_xcavate_whitelist::Role::SpvConfirmation
        ));
        assert_ok!(Marketplace::create_spv(RuntimeOrigin::signed([5; 32].into()), 0,));
        assert_ok!(Marketplace::claim_property_shares(RuntimeOrigin::signed([1; 32].into()), 0,));
        assert_ok!(Marketplace::claim_property_shares(RuntimeOrigin::signed([30; 32].into()), 0,));
        assert_ok!(Marketplace::claim_property_shares(RuntimeOrigin::signed([31; 32].into()), 0,));

        // Legal process
        assert_ok!(Marketplace::lawyer_claim_property(
            RuntimeOrigin::signed([10; 32].into()),
            0,
            crate::LegalProperty::RealEstateDeveloperSide,
            4_000,
        ));
        assert_ok!(Marketplace::approve_developer_lawyer(
            RuntimeOrigin::signed([0; 32].into()),
            0,
            true
        ));
        assert_ok!(Marketplace::lawyer_claim_property(
            RuntimeOrigin::signed([11; 32].into()),
            0,
            crate::LegalProperty::SpvSide,
            4_000,
        ));
        assert_ok!(Marketplace::vote_on_spv_lawyer(
            RuntimeOrigin::signed([1; 32].into()),
            0,
            crate::Vote::Yes,
            40
        ));
        assert_ok!(Marketplace::vote_on_spv_lawyer(
            RuntimeOrigin::signed([30; 32].into()),
            0,
            crate::Vote::Yes,
            30
        ));
        run_to_block(91);
        assert_ok!(Marketplace::finalize_spv_lawyer(RuntimeOrigin::signed([1; 32].into()), 0,));
        assert_ok!(Marketplace::unfreeze_spv_lawyer_shares(
            RuntimeOrigin::signed([1; 32].into()),
            0,
        ));
        assert_ok!(Marketplace::lawyer_confirm_documents(
            RuntimeOrigin::signed([10; 32].into()),
            0,
            true,
        ));
        assert_ok!(Marketplace::lawyer_confirm_documents(
            RuntimeOrigin::signed([11; 32].into()),
            0,
            true,
        ));

        // Secondary market
        assert_ok!(Marketplace::relist_shares(RuntimeOrigin::signed([1; 32].into()), 0, 500, 1));
        assert_ok!(Marketplace::make_offer(
            RuntimeOrigin::signed([2; 32].into()),
            1,
            2000,
            1,
            1984
        ));
        assert_eq!(ShareListings::<Test>::get(1).is_some(), true);
        assert_eq!(OngoingOffers::<Test>::get::<u32, AccountId>(1, [2; 32].into()).is_some(), true);
        assert_eq!(ForeignAssets::balance(1984, &([2; 32].into())), 1_148_000);
        assert_eq!(ForeignAssets::total_balance(1984, &[2; 32].into()), 1_150_000);
        assert_eq!(ForeignAssets::balance(1984, &Marketplace::property_account_id(0)), 0);
        assert_eq!(AssetsHolder::total_balance_on_hold(1984, &[2; 32].into()), 2000);
    })
}

#[test]
fn make_offer_fails() {
    new_test_ext().execute_with(|| {
        // Region setup and role assignments
        assert_ok!(XcavateWhitelist::add_admin(RuntimeOrigin::root(), [20; 32].into(),));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [8; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));
        new_region_helper();
        assert_ok!(Regions::create_new_location(
            RuntimeOrigin::signed([8; 32].into()),
            3,
            bvec![10, 10]
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [0; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateDeveloper
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [1; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [2; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [3; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [30; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [31; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [10; 32].into(),
            pallet_xcavate_whitelist::Role::Lawyer
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [11; 32].into(),
            pallet_xcavate_whitelist::Role::Lawyer
        ));
        assert_ok!(Regions::register_lawyer(RuntimeOrigin::signed([10; 32].into()), 3,));
        assert_ok!(Regions::register_lawyer(RuntimeOrigin::signed([11; 32].into()), 3,));

        // Property listing and purchases
        assert_ok!(Marketplace::list_property(
            RuntimeOrigin::signed([0; 32].into()),
            3,
            bvec![10, 10],
            10_000,
            100,
            bvec![22, 22],
            false
        ));
        assert_ok!(Marketplace::buy_property_shares(
            RuntimeOrigin::signed([1; 32].into()),
            0,
            40,
            1984
        ));
        assert_ok!(Marketplace::buy_property_shares(
            RuntimeOrigin::signed([30; 32].into()),
            0,
            30,
            1984
        ));
        assert_ok!(Marketplace::buy_property_shares(
            RuntimeOrigin::signed([31; 32].into()),
            0,
            30,
            1984
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [5; 32].into(),
            pallet_xcavate_whitelist::Role::SpvConfirmation
        ));
        assert_ok!(Marketplace::create_spv(RuntimeOrigin::signed([5; 32].into()), 0,));
        assert_ok!(Marketplace::claim_property_shares(RuntimeOrigin::signed([1; 32].into()), 0,));
        assert_ok!(Marketplace::claim_property_shares(RuntimeOrigin::signed([30; 32].into()), 0,));
        assert_ok!(Marketplace::claim_property_shares(RuntimeOrigin::signed([31; 32].into()), 0,));

        // Legal process
        assert_ok!(Marketplace::lawyer_claim_property(
            RuntimeOrigin::signed([10; 32].into()),
            0,
            crate::LegalProperty::RealEstateDeveloperSide,
            4_000,
        ));
        assert_ok!(Marketplace::approve_developer_lawyer(
            RuntimeOrigin::signed([0; 32].into()),
            0,
            true
        ));
        assert_ok!(Marketplace::lawyer_claim_property(
            RuntimeOrigin::signed([11; 32].into()),
            0,
            crate::LegalProperty::SpvSide,
            4_000,
        ));
        assert_ok!(Marketplace::vote_on_spv_lawyer(
            RuntimeOrigin::signed([1; 32].into()),
            0,
            crate::Vote::Yes,
            40
        ));
        assert_ok!(Marketplace::vote_on_spv_lawyer(
            RuntimeOrigin::signed([30; 32].into()),
            0,
            crate::Vote::Yes,
            30
        ));
        run_to_block(91);
        assert_ok!(Marketplace::finalize_spv_lawyer(RuntimeOrigin::signed([1; 32].into()), 0,));
        assert_ok!(Marketplace::unfreeze_spv_lawyer_shares(
            RuntimeOrigin::signed([1; 32].into()),
            0,
        ));
        assert_ok!(Marketplace::lawyer_confirm_documents(
            RuntimeOrigin::signed([10; 32].into()),
            0,
            true,
        ));
        assert_ok!(Marketplace::lawyer_confirm_documents(
            RuntimeOrigin::signed([11; 32].into()),
            0,
            true,
        ));

        // Secondary market
        assert_noop!(
            Marketplace::make_offer(RuntimeOrigin::signed([2; 32].into()), 1, 200, 1, 1984),
            Error::<Test>::ShareNotForSale
        );
        assert_ok!(Marketplace::relist_shares(RuntimeOrigin::signed([1; 32].into()), 0, 500, 1));
        assert_noop!(
            Marketplace::make_offer(RuntimeOrigin::signed([2; 32].into()), 1, 200, 2, 1984),
            Error::<Test>::NotEnoughSharesAvailable
        );
        assert_noop!(
            Marketplace::make_offer(RuntimeOrigin::signed([2; 32].into()), 1, 200, 1, 100),
            Error::<Test>::PaymentAssetNotSupported
        );
        assert_noop!(
            Marketplace::make_offer(RuntimeOrigin::signed([2; 32].into()), 1, 200, 0, 1984),
            Error::<Test>::AmountCannotBeZero
        );
        assert_noop!(
            Marketplace::make_offer(RuntimeOrigin::signed([2; 32].into()), 1, 0, 1, 1984),
            Error::<Test>::InvalidSharePrice
        );
        assert_ok!(XcavateWhitelist::set_permission(
            RuntimeOrigin::signed([20; 32].into()),
            [2; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor,
            pallet_xcavate_whitelist::AccessPermission::Revoked,
        ));
        assert_noop!(
            Marketplace::make_offer(RuntimeOrigin::signed([2; 32].into()), 1, 200, 1, 1984),
            BadOrigin
        );
        assert_ok!(XcavateWhitelist::set_permission(
            RuntimeOrigin::signed([20; 32].into()),
            [2; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor,
            pallet_xcavate_whitelist::AccessPermission::Compliant,
        ));
        assert_ok!(Marketplace::make_offer(RuntimeOrigin::signed([2; 32].into()), 1, 200, 1, 1984));
        assert_ok!(Marketplace::make_offer(RuntimeOrigin::signed([3; 32].into()), 1, 300, 1, 1984));
        assert_noop!(
            Marketplace::make_offer(RuntimeOrigin::signed([2; 32].into()), 1, 400, 1, 1984),
            Error::<Test>::OnlyOneOfferPerUser
        );
        assert_eq!(
            OngoingOffers::<Test>::get::<u32, AccountId>(1, [2; 32].into()).unwrap().share_price,
            200
        );
        assert_eq!(
            OngoingOffers::<Test>::get::<u32, AccountId>(1, [3; 32].into()).unwrap().share_price,
            300
        );
    })
}

// handle_offer tests

#[test]
fn handle_offer_works() {
    new_test_ext().execute_with(|| {
        // Region setup and role assignments
        assert_ok!(XcavateWhitelist::add_admin(RuntimeOrigin::root(), [20; 32].into(),));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [8; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));
        new_region_helper();
        assert_ok!(Regions::create_new_location(
            RuntimeOrigin::signed([8; 32].into()),
            3,
            bvec![10, 10]
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [0; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateDeveloper
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [1; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [2; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [3; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [30; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [31; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [10; 32].into(),
            pallet_xcavate_whitelist::Role::Lawyer
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [11; 32].into(),
            pallet_xcavate_whitelist::Role::Lawyer
        ));
        assert_ok!(Regions::register_lawyer(RuntimeOrigin::signed([10; 32].into()), 3,));
        assert_ok!(Regions::register_lawyer(RuntimeOrigin::signed([11; 32].into()), 3,));

        // Property listing and purchases
        assert_ok!(Marketplace::list_property(
            RuntimeOrigin::signed([0; 32].into()),
            3,
            bvec![10, 10],
            10_000,
            100,
            bvec![22, 22],
            false
        ));
        assert_ok!(Marketplace::buy_property_shares(
            RuntimeOrigin::signed([1; 32].into()),
            0,
            40,
            1984
        ));
        assert_ok!(Marketplace::buy_property_shares(
            RuntimeOrigin::signed([30; 32].into()),
            0,
            30,
            1984
        ));
        assert_ok!(Marketplace::buy_property_shares(
            RuntimeOrigin::signed([31; 32].into()),
            0,
            30,
            1984
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [5; 32].into(),
            pallet_xcavate_whitelist::Role::SpvConfirmation
        ));
        assert_ok!(Marketplace::create_spv(RuntimeOrigin::signed([5; 32].into()), 0,));
        assert_ok!(Marketplace::claim_property_shares(RuntimeOrigin::signed([1; 32].into()), 0,));
        assert_ok!(Marketplace::claim_property_shares(RuntimeOrigin::signed([30; 32].into()), 0,));
        assert_ok!(Marketplace::claim_property_shares(RuntimeOrigin::signed([31; 32].into()), 0,));

        // Legal process
        assert_ok!(Marketplace::lawyer_claim_property(
            RuntimeOrigin::signed([10; 32].into()),
            0,
            crate::LegalProperty::RealEstateDeveloperSide,
            4_000,
        ));
        assert_ok!(Marketplace::approve_developer_lawyer(
            RuntimeOrigin::signed([0; 32].into()),
            0,
            true
        ));
        assert_ok!(Marketplace::lawyer_claim_property(
            RuntimeOrigin::signed([11; 32].into()),
            0,
            crate::LegalProperty::SpvSide,
            4_000,
        ));
        assert_ok!(Marketplace::vote_on_spv_lawyer(
            RuntimeOrigin::signed([1; 32].into()),
            0,
            crate::Vote::Yes,
            40
        ));
        assert_ok!(Marketplace::vote_on_spv_lawyer(
            RuntimeOrigin::signed([30; 32].into()),
            0,
            crate::Vote::Yes,
            30
        ));
        run_to_block(91);
        assert_ok!(Marketplace::finalize_spv_lawyer(RuntimeOrigin::signed([1; 32].into()), 0,));
        assert_ok!(Marketplace::unfreeze_spv_lawyer_shares(
            RuntimeOrigin::signed([1; 32].into()),
            0,
        ));
        assert_ok!(Marketplace::lawyer_confirm_documents(
            RuntimeOrigin::signed([10; 32].into()),
            0,
            true,
        ));
        assert_ok!(Marketplace::lawyer_confirm_documents(
            RuntimeOrigin::signed([11; 32].into()),
            0,
            true,
        ));

        // Secondary market
        assert_ok!(Marketplace::relist_shares(RuntimeOrigin::signed([1; 32].into()), 0, 5000, 20));
        assert_ok!(Marketplace::make_offer(RuntimeOrigin::signed([2; 32].into()), 1, 200, 1, 1984));
        assert_ok!(Marketplace::make_offer(RuntimeOrigin::signed([3; 32].into()), 1, 150, 1, 1337));
        assert_eq!(AssetsHolder::total_balance_on_hold(1984, &[2; 32].into()), 200);
        assert_eq!(AssetsHolder::total_balance_on_hold(1337, &[3; 32].into()), 150);
        assert_ok!(Marketplace::handle_offer(
            RuntimeOrigin::signed([1; 32].into()),
            1,
            [2; 32].into(),
            crate::Offer::Reject,
            0
        ));
        assert_eq!(AssetsHolder::total_balance_on_hold(1984, &[2; 32].into()), 0);
        assert_ok!(Marketplace::cancel_offer(RuntimeOrigin::signed([3; 32].into()), 1));
        assert_eq!(ForeignAssets::balance(1984, &([2; 32].into())), 1_150_000);
        assert_eq!(ShareListings::<Test>::get(1).is_some(), true);
        assert_eq!(OngoingOffers::<Test>::get::<u32, AccountId>(1, [2; 32].into()).is_none(), true);
        assert_ok!(Marketplace::make_offer(
            RuntimeOrigin::signed([2; 32].into()),
            1,
            2000,
            10,
            1984
        ));
        assert_eq!(ForeignAssets::total_balance(1984, &([2; 32].into())), 1_150_000);
        assert_eq!(ForeignAssets::balance(1984, &([2; 32].into())), 1_130_000);
        assert_eq!(AssetsHolder::total_balance_on_hold(1984, &[2; 32].into()), 20000);
        assert_ok!(Marketplace::handle_offer(
            RuntimeOrigin::signed([1; 32].into()),
            1,
            [2; 32].into(),
            crate::Offer::Accept,
            2
        ));
        assert_eq!(ShareListings::<Test>::get(1).unwrap().amount, 10);
        assert_eq!(OngoingOffers::<Test>::get::<u32, AccountId>(1, [2; 32].into()).is_none(), true);
        assert_eq!(ForeignAssets::balance(1984, &Marketplace::property_account_id(1)), 0);
        assert_eq!(LocalAssets::balance(0, &([1; 32].into())), 20);
        assert_eq!(LocalAssets::balance(0, &([2; 32].into())), 10);
        assert_eq!(ForeignAssets::balance(0, &Marketplace::property_account_id(0)), 0);
        assert_eq!(ForeignAssets::balance(1984, &([1; 32].into())), 1_103_800);
        assert_eq!(ForeignAssets::balance(1984, &([2; 32].into())), 1_130_000);
    })
}

#[test]
fn handle_offer_fails() {
    new_test_ext().execute_with(|| {
        // Region setup and role assignments
        assert_ok!(XcavateWhitelist::add_admin(RuntimeOrigin::root(), [20; 32].into(),));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [8; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));
        new_region_helper();
        assert_ok!(Regions::create_new_location(
            RuntimeOrigin::signed([8; 32].into()),
            3,
            bvec![10, 10]
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [0; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateDeveloper
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [1; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [2; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [30; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [31; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [10; 32].into(),
            pallet_xcavate_whitelist::Role::Lawyer
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [11; 32].into(),
            pallet_xcavate_whitelist::Role::Lawyer
        ));
        assert_ok!(Regions::register_lawyer(RuntimeOrigin::signed([10; 32].into()), 3,));
        assert_ok!(Regions::register_lawyer(RuntimeOrigin::signed([11; 32].into()), 3,));

        // Property listing and purchases
        assert_ok!(Marketplace::list_property(
            RuntimeOrigin::signed([0; 32].into()),
            3,
            bvec![10, 10],
            10_000,
            100,
            bvec![22, 22],
            false
        ));
        assert_ok!(Marketplace::buy_property_shares(
            RuntimeOrigin::signed([1; 32].into()),
            0,
            40,
            1984
        ));
        assert_ok!(Marketplace::buy_property_shares(
            RuntimeOrigin::signed([30; 32].into()),
            0,
            30,
            1984
        ));
        assert_ok!(Marketplace::buy_property_shares(
            RuntimeOrigin::signed([31; 32].into()),
            0,
            30,
            1984
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [5; 32].into(),
            pallet_xcavate_whitelist::Role::SpvConfirmation
        ));
        assert_ok!(Marketplace::create_spv(RuntimeOrigin::signed([5; 32].into()), 0,));
        assert_ok!(Marketplace::claim_property_shares(RuntimeOrigin::signed([1; 32].into()), 0,));
        assert_ok!(Marketplace::claim_property_shares(RuntimeOrigin::signed([30; 32].into()), 0,));
        assert_ok!(Marketplace::claim_property_shares(RuntimeOrigin::signed([31; 32].into()), 0,));

        // Legal process
        assert_ok!(Marketplace::lawyer_claim_property(
            RuntimeOrigin::signed([10; 32].into()),
            0,
            crate::LegalProperty::RealEstateDeveloperSide,
            4_000,
        ));
        assert_ok!(Marketplace::approve_developer_lawyer(
            RuntimeOrigin::signed([0; 32].into()),
            0,
            true
        ));
        assert_ok!(Marketplace::lawyer_claim_property(
            RuntimeOrigin::signed([11; 32].into()),
            0,
            crate::LegalProperty::SpvSide,
            4_000,
        ));
        assert_ok!(Marketplace::vote_on_spv_lawyer(
            RuntimeOrigin::signed([1; 32].into()),
            0,
            crate::Vote::Yes,
            40
        ));
        assert_ok!(Marketplace::vote_on_spv_lawyer(
            RuntimeOrigin::signed([30; 32].into()),
            0,
            crate::Vote::Yes,
            30
        ));
        run_to_block(91);
        assert_ok!(Marketplace::finalize_spv_lawyer(RuntimeOrigin::signed([1; 32].into()), 0,));
        assert_ok!(Marketplace::unfreeze_spv_lawyer_shares(
            RuntimeOrigin::signed([1; 32].into()),
            0,
        ));
        assert_ok!(Marketplace::unfreeze_spv_lawyer_shares(
            RuntimeOrigin::signed([30; 32].into()),
            0,
        ));
        assert_ok!(Marketplace::lawyer_confirm_documents(
            RuntimeOrigin::signed([10; 32].into()),
            0,
            true,
        ));
        assert_ok!(Marketplace::lawyer_confirm_documents(
            RuntimeOrigin::signed([11; 32].into()),
            0,
            true,
        ));

        // Secondary market
        assert_noop!(
            Marketplace::handle_offer(
                RuntimeOrigin::signed([1; 32].into()),
                1,
                [2; 32].into(),
                crate::Offer::Reject,
                0
            ),
            Error::<Test>::ShareNotForSale
        );
        assert_ok!(Marketplace::relist_shares(RuntimeOrigin::signed([1; 32].into()), 0, 5000, 2));
        assert_noop!(
            Marketplace::handle_offer(
                RuntimeOrigin::signed([1; 32].into()),
                1,
                [2; 32].into(),
                crate::Offer::Reject,
                0
            ),
            Error::<Test>::OfferNotFound
        );
        assert_ok!(Marketplace::make_offer(RuntimeOrigin::signed([2; 32].into()), 1, 200, 1, 1984));
        assert_noop!(
            Marketplace::handle_offer(
                RuntimeOrigin::signed([2; 32].into()),
                1,
                [2; 32].into(),
                crate::Offer::Accept,
                0
            ),
            Error::<Test>::NoPermission
        );
        assert_ok!(Marketplace::relist_shares(RuntimeOrigin::signed([30; 32].into()), 0, 5000, 20));
        assert_ok!(Marketplace::make_offer(
            RuntimeOrigin::signed([1; 32].into()),
            2,
            200,
            15,
            1984
        ));
        assert_eq!(PropertyOwnerShares::<Test>::get::<u32, AccountId>(0, [1; 32].into()), 40);
        assert_noop!(
            Marketplace::handle_offer(
                RuntimeOrigin::signed([30; 32].into()),
                2,
                [1; 32].into(),
                crate::Offer::Accept,
                1
            ),
            Error::<Test>::ExceedsMaxOwnership
        );
        assert_eq!(PropertyOwnerShares::<Test>::get::<u32, AccountId>(0, [1; 32].into()), 40);
        assert_ok!(Marketplace::make_offer(RuntimeOrigin::signed([2; 32].into()), 2, 200, 1, 1984));
        assert_ok!(XcavateWhitelist::set_permission(
            RuntimeOrigin::signed([20; 32].into()),
            [30; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor,
            pallet_xcavate_whitelist::AccessPermission::Revoked,
        ));
        assert_noop!(
            Marketplace::handle_offer(
                RuntimeOrigin::signed([30; 32].into()),
                2,
                [2; 32].into(),
                crate::Offer::Accept,
                2
            ),
            BadOrigin
        );
        assert_ok!(XcavateWhitelist::set_permission(
            RuntimeOrigin::signed([20; 32].into()),
            [30; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor,
            pallet_xcavate_whitelist::AccessPermission::Compliant,
        ));
        assert_ok!(Marketplace::cancel_offer(RuntimeOrigin::signed([2; 32].into()), 2));
        assert_ok!(Marketplace::make_offer(RuntimeOrigin::signed([2; 32].into()), 2, 1, 1, 1984));
        assert_noop!(
            Marketplace::handle_offer(
                RuntimeOrigin::signed([30; 32].into()),
                2,
                [2; 32].into(),
                crate::Offer::Accept,
                2
            ),
            Error::<Test>::InvalidOfferNonce
        );
    })
}

// cancel_offer tests

#[test]
fn cancel_offer_works() {
    new_test_ext().execute_with(|| {
        // Region setup and role assignments
        assert_ok!(XcavateWhitelist::add_admin(RuntimeOrigin::root(), [20; 32].into(),));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [8; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));
        new_region_helper();
        assert_ok!(Regions::create_new_location(
            RuntimeOrigin::signed([8; 32].into()),
            3,
            bvec![10, 10]
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [0; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateDeveloper
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [1; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [2; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [30; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [31; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [10; 32].into(),
            pallet_xcavate_whitelist::Role::Lawyer
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [11; 32].into(),
            pallet_xcavate_whitelist::Role::Lawyer
        ));
        assert_ok!(Regions::register_lawyer(RuntimeOrigin::signed([10; 32].into()), 3,));
        assert_ok!(Regions::register_lawyer(RuntimeOrigin::signed([11; 32].into()), 3,));
        // Property listing and purchases
        assert_ok!(Marketplace::list_property(
            RuntimeOrigin::signed([0; 32].into()),
            3,
            bvec![10, 10],
            10_000,
            100,
            bvec![22, 22],
            false
        ));
        assert_ok!(Marketplace::buy_property_shares(
            RuntimeOrigin::signed([1; 32].into()),
            0,
            40,
            1984
        ));
        assert_ok!(Marketplace::buy_property_shares(
            RuntimeOrigin::signed([30; 32].into()),
            0,
            30,
            1984
        ));
        assert_ok!(Marketplace::buy_property_shares(
            RuntimeOrigin::signed([31; 32].into()),
            0,
            30,
            1984
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [5; 32].into(),
            pallet_xcavate_whitelist::Role::SpvConfirmation
        ));
        assert_ok!(Marketplace::create_spv(RuntimeOrigin::signed([5; 32].into()), 0,));
        assert_ok!(Marketplace::claim_property_shares(RuntimeOrigin::signed([1; 32].into()), 0,));
        assert_ok!(Marketplace::claim_property_shares(RuntimeOrigin::signed([30; 32].into()), 0,));
        assert_ok!(Marketplace::claim_property_shares(RuntimeOrigin::signed([31; 32].into()), 0,));

        // Legal process
        assert_ok!(Marketplace::lawyer_claim_property(
            RuntimeOrigin::signed([10; 32].into()),
            0,
            crate::LegalProperty::RealEstateDeveloperSide,
            4_000,
        ));
        assert_ok!(Marketplace::approve_developer_lawyer(
            RuntimeOrigin::signed([0; 32].into()),
            0,
            true
        ));
        assert_ok!(Marketplace::lawyer_claim_property(
            RuntimeOrigin::signed([11; 32].into()),
            0,
            crate::LegalProperty::SpvSide,
            4_000,
        ));
        assert_ok!(Marketplace::vote_on_spv_lawyer(
            RuntimeOrigin::signed([1; 32].into()),
            0,
            crate::Vote::Yes,
            40
        ));
        assert_ok!(Marketplace::vote_on_spv_lawyer(
            RuntimeOrigin::signed([30; 32].into()),
            0,
            crate::Vote::Yes,
            30
        ));
        run_to_block(91);
        assert_ok!(Marketplace::finalize_spv_lawyer(RuntimeOrigin::signed([1; 32].into()), 0,));
        assert_ok!(Marketplace::unfreeze_spv_lawyer_shares(
            RuntimeOrigin::signed([1; 32].into()),
            0,
        ));
        assert_ok!(Marketplace::lawyer_confirm_documents(
            RuntimeOrigin::signed([10; 32].into()),
            0,
            true,
        ));
        assert_ok!(Marketplace::lawyer_confirm_documents(
            RuntimeOrigin::signed([11; 32].into()),
            0,
            true,
        ));

        // Secondary market
        assert_ok!(Marketplace::relist_shares(RuntimeOrigin::signed([1; 32].into()), 0, 500, 1));
        assert_ok!(Marketplace::make_offer(
            RuntimeOrigin::signed([2; 32].into()),
            1,
            2000,
            1,
            1984
        ));
        assert_eq!(ShareListings::<Test>::get(1).is_some(), true);
        assert_eq!(OngoingOffers::<Test>::get::<u32, AccountId>(1, [2; 32].into()).is_some(), true);
        assert_eq!(ForeignAssets::balance(1984, &([2; 32].into())), 1_148_000);
        assert_eq!(ForeignAssets::total_balance(1984, &([2; 32].into())), 1_150_000);
        assert_eq!(AssetsHolder::total_balance_on_hold(1984, &[2; 32].into()), 2000);
        assert_ok!(Marketplace::cancel_offer(RuntimeOrigin::signed([2; 32].into()), 1));
        assert_eq!(ShareListings::<Test>::get(1).is_some(), true);
        assert_eq!(
            OngoingOffers::<Test>::get::<u32, AccountId>(1, [2; 32].into()).is_some(),
            false
        );
        assert_eq!(ForeignAssets::balance(1984, &([2; 32].into())), 1_150_000);
        assert_eq!(ForeignAssets::balance(1984, &Marketplace::property_account_id(1)), 0);
    })
}

#[test]
fn cancel_offer_fails() {
    new_test_ext().execute_with(|| {
        // Region setup and role assignments
        assert_ok!(XcavateWhitelist::add_admin(RuntimeOrigin::root(), [20; 32].into(),));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [8; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));
        new_region_helper();
        assert_ok!(Regions::create_new_location(
            RuntimeOrigin::signed([8; 32].into()),
            3,
            bvec![10, 10]
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [0; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateDeveloper
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [1; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [2; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [30; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [31; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [10; 32].into(),
            pallet_xcavate_whitelist::Role::Lawyer
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [11; 32].into(),
            pallet_xcavate_whitelist::Role::Lawyer
        ));
        assert_ok!(Regions::register_lawyer(RuntimeOrigin::signed([10; 32].into()), 3,));
        assert_ok!(Regions::register_lawyer(RuntimeOrigin::signed([11; 32].into()), 3,));

        // Property listing and purchases
        assert_ok!(Marketplace::list_property(
            RuntimeOrigin::signed([0; 32].into()),
            3,
            bvec![10, 10],
            10_000,
            100,
            bvec![22, 22],
            false
        ));
        assert_ok!(Marketplace::buy_property_shares(
            RuntimeOrigin::signed([1; 32].into()),
            0,
            40,
            1984
        ));
        assert_ok!(Marketplace::buy_property_shares(
            RuntimeOrigin::signed([30; 32].into()),
            0,
            30,
            1984
        ));
        assert_ok!(Marketplace::buy_property_shares(
            RuntimeOrigin::signed([31; 32].into()),
            0,
            30,
            1984
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [5; 32].into(),
            pallet_xcavate_whitelist::Role::SpvConfirmation
        ));
        assert_ok!(Marketplace::create_spv(RuntimeOrigin::signed([5; 32].into()), 0,));
        assert_ok!(Marketplace::claim_property_shares(RuntimeOrigin::signed([1; 32].into()), 0,));
        assert_ok!(Marketplace::claim_property_shares(RuntimeOrigin::signed([30; 32].into()), 0,));
        assert_ok!(Marketplace::claim_property_shares(RuntimeOrigin::signed([31; 32].into()), 0,));

        // Legal process
        assert_ok!(Marketplace::lawyer_claim_property(
            RuntimeOrigin::signed([10; 32].into()),
            0,
            crate::LegalProperty::RealEstateDeveloperSide,
            4_000,
        ));
        assert_ok!(Marketplace::approve_developer_lawyer(
            RuntimeOrigin::signed([0; 32].into()),
            0,
            true
        ));
        assert_ok!(Marketplace::lawyer_claim_property(
            RuntimeOrigin::signed([11; 32].into()),
            0,
            crate::LegalProperty::SpvSide,
            4_000,
        ));
        assert_ok!(Marketplace::vote_on_spv_lawyer(
            RuntimeOrigin::signed([1; 32].into()),
            0,
            crate::Vote::Yes,
            40
        ));
        assert_ok!(Marketplace::vote_on_spv_lawyer(
            RuntimeOrigin::signed([30; 32].into()),
            0,
            crate::Vote::Yes,
            30
        ));
        run_to_block(91);
        assert_ok!(Marketplace::finalize_spv_lawyer(RuntimeOrigin::signed([1; 32].into()), 0,));
        assert_ok!(Marketplace::unfreeze_spv_lawyer_shares(
            RuntimeOrigin::signed([1; 32].into()),
            0,
        ));
        assert_ok!(Marketplace::lawyer_confirm_documents(
            RuntimeOrigin::signed([10; 32].into()),
            0,
            true,
        ));
        assert_ok!(Marketplace::lawyer_confirm_documents(
            RuntimeOrigin::signed([11; 32].into()),
            0,
            true,
        ));

        // Secondary market
        assert_ok!(Marketplace::relist_shares(RuntimeOrigin::signed([1; 32].into()), 0, 500, 1));
        assert_noop!(
            Marketplace::cancel_offer(RuntimeOrigin::signed([2; 32].into()), 1),
            Error::<Test>::OfferNotFound
        );
        assert_ok!(Marketplace::make_offer(
            RuntimeOrigin::signed([2; 32].into()),
            1,
            2000,
            1,
            1984
        ));
        assert_eq!(ShareListings::<Test>::get(1).is_some(), true);
        assert_eq!(OngoingOffers::<Test>::get::<u32, AccountId>(1, [2; 32].into()).is_some(), true);
        assert_eq!(ForeignAssets::total_balance(1984, &([2; 32].into())), 1_150_000);
        assert_eq!(ForeignAssets::balance(1984, &([2; 32].into())), 1_148_000);
        assert_eq!(AssetsHolder::total_balance_on_hold(1984, &[2; 32].into()), 2000);
        assert_noop!(
            Marketplace::cancel_offer(RuntimeOrigin::signed([1; 32].into()), 1),
            Error::<Test>::OfferNotFound
        );
    })
}

// upgrade_object tests

#[test]
fn upgrade_object_works() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);
        assert_ok!(XcavateWhitelist::add_admin(RuntimeOrigin::root(), [20; 32].into(),));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [8; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));
        new_region_helper();
        assert_ok!(Regions::create_new_location(
            RuntimeOrigin::signed([8; 32].into()),
            3,
            bvec![10, 10]
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [0; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateDeveloper
        ));
        assert_ok!(Marketplace::list_property(
            RuntimeOrigin::signed([0; 32].into()),
            3,
            bvec![10, 10],
            10_000,
            100,
            bvec![22, 22],
            false
        ));
        assert_ok!(Marketplace::upgrade_object(RuntimeOrigin::signed([0; 32].into()), 0, 30000));
        assert_eq!(OngoingObjectListing::<Test>::get(0).unwrap().share_price, 30000);
    })
}

#[test]
fn upgrade_object_and_distribute_works() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);
        // Region setup and role assignments
        assert_ok!(XcavateWhitelist::add_admin(RuntimeOrigin::root(), [20; 32].into(),));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [8; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));
        new_region_helper();
        assert_ok!(Regions::create_new_location(
            RuntimeOrigin::signed([8; 32].into()),
            3,
            bvec![10, 10]
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [0; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateDeveloper
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [1; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [2; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [30; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [10; 32].into(),
            pallet_xcavate_whitelist::Role::Lawyer
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [11; 32].into(),
            pallet_xcavate_whitelist::Role::Lawyer
        ));
        assert_ok!(Regions::register_lawyer(RuntimeOrigin::signed([10; 32].into()), 3,));
        assert_ok!(Regions::register_lawyer(RuntimeOrigin::signed([11; 32].into()), 3,));
        // Property listing and purchases
        assert_ok!(Marketplace::list_property(
            RuntimeOrigin::signed([0; 32].into()),
            3,
            bvec![10, 10],
            10_000,
            100,
            bvec![22, 22],
            false
        ));
        assert_ok!(Marketplace::buy_property_shares(
            RuntimeOrigin::signed([1; 32].into()),
            0,
            40,
            1984
        ));
        assert_ok!(Marketplace::buy_property_shares(
            RuntimeOrigin::signed([30; 32].into()),
            0,
            10,
            1984
        ));
        assert_ok!(Marketplace::upgrade_object(RuntimeOrigin::signed([0; 32].into()), 0, 20_000));
        assert_ok!(Marketplace::buy_property_shares(
            RuntimeOrigin::signed([2; 32].into()),
            0,
            40,
            1984
        ));
        assert_ok!(Marketplace::buy_property_shares(
            RuntimeOrigin::signed([30; 32].into()),
            0,
            10,
            1984
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [5; 32].into(),
            pallet_xcavate_whitelist::Role::SpvConfirmation
        ));
        assert_ok!(Marketplace::create_spv(RuntimeOrigin::signed([5; 32].into()), 0,));
        assert_ok!(Marketplace::claim_property_shares(RuntimeOrigin::signed([1; 32].into()), 0,));
        assert_ok!(Marketplace::claim_property_shares(RuntimeOrigin::signed([2; 32].into()), 0,));
        assert_ok!(Marketplace::claim_property_shares(RuntimeOrigin::signed([30; 32].into()), 0,));

        // Legal process
        assert_ok!(Marketplace::lawyer_claim_property(
            RuntimeOrigin::signed([10; 32].into()),
            0,
            crate::LegalProperty::RealEstateDeveloperSide,
            4_000,
        ));
        assert_ok!(Marketplace::approve_developer_lawyer(
            RuntimeOrigin::signed([0; 32].into()),
            0,
            true
        ));
        assert_ok!(Marketplace::lawyer_claim_property(
            RuntimeOrigin::signed([11; 32].into()),
            0,
            crate::LegalProperty::SpvSide,
            4_000,
        ));
        assert_ok!(Marketplace::vote_on_spv_lawyer(
            RuntimeOrigin::signed([1; 32].into()),
            0,
            crate::Vote::Yes,
            40
        ));
        assert_ok!(Marketplace::vote_on_spv_lawyer(
            RuntimeOrigin::signed([2; 32].into()),
            0,
            crate::Vote::Yes,
            40
        ));
        run_to_block(91);
        assert_ok!(Marketplace::finalize_spv_lawyer(RuntimeOrigin::signed([1; 32].into()), 0,));
        assert_ok!(Marketplace::lawyer_confirm_documents(
            RuntimeOrigin::signed([10; 32].into()),
            0,
            true,
        ));
        assert_ok!(Marketplace::lawyer_confirm_documents(
            RuntimeOrigin::signed([11; 32].into()),
            0,
            true,
        ));

        // Check balances after finalization
        assert_eq!(ForeignAssets::balance(1984, &([0; 32].into())), 21485000);
        assert_eq!(ForeignAssets::balance(1984, &Marketplace::treasury_account_id()), 13000);
        assert_eq!(ForeignAssets::balance(1984, &([8; 32].into())), 13000);
        assert_eq!(ForeignAssets::balance(1984, &([1; 32].into())), 1_084_000);
        assert_eq!(ForeignAssets::balance(1984, &([2; 32].into())), 318_000);
        assert_eq!(ForeignAssets::balance(1984, &([30; 32].into())), 888_000);

        assert_eq!(PropertyAssetInfo::<Test>::get(0).unwrap().spv_created, true);
        assert_eq!(OngoingObjectListing::<Test>::get(0), None);
    })
}

#[test]
fn upgrade_object_for_relisted_nft_fails() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);
        // Region setup and role assignments
        assert_ok!(XcavateWhitelist::add_admin(RuntimeOrigin::root(), [20; 32].into(),));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [8; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));
        new_region_helper();
        assert_ok!(Regions::create_new_location(
            RuntimeOrigin::signed([8; 32].into()),
            3,
            bvec![10, 10]
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [0; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateDeveloper
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [0; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [1; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [2; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [10; 32].into(),
            pallet_xcavate_whitelist::Role::Lawyer
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [11; 32].into(),
            pallet_xcavate_whitelist::Role::Lawyer
        ));
        assert_ok!(Regions::register_lawyer(RuntimeOrigin::signed([10; 32].into()), 3,));
        assert_ok!(Regions::register_lawyer(RuntimeOrigin::signed([11; 32].into()), 3,));

        // Property listing and purchases
        assert_ok!(Marketplace::list_property(
            RuntimeOrigin::signed([0; 32].into()),
            3,
            bvec![10, 10],
            10_000,
            100,
            bvec![22, 22],
            false
        ));
        assert_ok!(Marketplace::buy_property_shares(
            RuntimeOrigin::signed([0; 32].into()),
            0,
            40,
            1984
        ));
        assert_ok!(Marketplace::buy_property_shares(
            RuntimeOrigin::signed([1; 32].into()),
            0,
            30,
            1984
        ));
        assert_ok!(Marketplace::buy_property_shares(
            RuntimeOrigin::signed([2; 32].into()),
            0,
            30,
            1984
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [5; 32].into(),
            pallet_xcavate_whitelist::Role::SpvConfirmation
        ));
        assert_ok!(Marketplace::create_spv(RuntimeOrigin::signed([5; 32].into()), 0,));
        assert_ok!(Marketplace::claim_property_shares(RuntimeOrigin::signed([0; 32].into()), 0,));
        assert_ok!(Marketplace::claim_property_shares(RuntimeOrigin::signed([1; 32].into()), 0,));
        assert_ok!(Marketplace::claim_property_shares(RuntimeOrigin::signed([2; 32].into()), 0,));

        // Legal process
        assert_ok!(Marketplace::lawyer_claim_property(
            RuntimeOrigin::signed([10; 32].into()),
            0,
            crate::LegalProperty::RealEstateDeveloperSide,
            4_000,
        ));
        assert_ok!(Marketplace::approve_developer_lawyer(
            RuntimeOrigin::signed([0; 32].into()),
            0,
            true
        ));
        assert_ok!(Marketplace::lawyer_claim_property(
            RuntimeOrigin::signed([11; 32].into()),
            0,
            crate::LegalProperty::SpvSide,
            4_000,
        ));
        assert_ok!(Marketplace::vote_on_spv_lawyer(
            RuntimeOrigin::signed([0; 32].into()),
            0,
            crate::Vote::Yes,
            40
        ));
        assert_ok!(Marketplace::vote_on_spv_lawyer(
            RuntimeOrigin::signed([1; 32].into()),
            0,
            crate::Vote::Yes,
            30
        ));
        run_to_block(91);
        assert_ok!(Marketplace::finalize_spv_lawyer(RuntimeOrigin::signed([0; 32].into()), 0,),);
        assert_ok!(Marketplace::unfreeze_spv_lawyer_shares(
            RuntimeOrigin::signed([0; 32].into()),
            0,
        ));
        assert_ok!(Marketplace::lawyer_confirm_documents(
            RuntimeOrigin::signed([10; 32].into()),
            0,
            true,
        ));
        assert_ok!(Marketplace::lawyer_confirm_documents(
            RuntimeOrigin::signed([11; 32].into()),
            0,
            true,
        ));

        // Secondary market
        assert_ok!(Marketplace::relist_shares(RuntimeOrigin::signed([0; 32].into()), 0, 1000, 1));
        assert_noop!(
            Marketplace::upgrade_object(RuntimeOrigin::signed([0; 32].into()), 1, 300),
            Error::<Test>::ShareNotForSale
        );
    })
}

#[test]
fn upgrade_object_fails() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);
        // Region setup and role assignments
        assert_ok!(XcavateWhitelist::add_admin(RuntimeOrigin::root(), [20; 32].into(),));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [8; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));
        new_region_helper();
        assert_ok!(Regions::create_new_location(
            RuntimeOrigin::signed([8; 32].into()),
            3,
            bvec![10, 10]
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [0; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateDeveloper
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [0; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [1; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [30; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [31; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));
        assert_noop!(
            Marketplace::upgrade_object(RuntimeOrigin::signed([0; 32].into()), 0, 300),
            Error::<Test>::ShareNotForSale
        );
        // First property listing and purchases
        assert_ok!(Marketplace::list_property(
            RuntimeOrigin::signed([0; 32].into()),
            3,
            bvec![10, 10],
            10_000,
            100,
            bvec![22, 22],
            false
        ));
        assert_noop!(
            Marketplace::upgrade_object(RuntimeOrigin::signed([0; 32].into()), 0, 0),
            Error::<Test>::InvalidSharePrice
        );
        assert_ok!(Marketplace::buy_property_shares(
            RuntimeOrigin::signed([0; 32].into()),
            0,
            40,
            1984
        ));
        assert_ok!(Marketplace::buy_property_shares(
            RuntimeOrigin::signed([30; 32].into()),
            0,
            30,
            1984
        ));
        assert_ok!(Marketplace::buy_property_shares(
            RuntimeOrigin::signed([31; 32].into()),
            0,
            30,
            1984
        ));
        assert_noop!(
            Marketplace::upgrade_object(RuntimeOrigin::signed([0; 32].into()), 0, 300),
            Error::<Test>::PropertyAlreadySold
        );

        // Second property listing to test expired listing
        assert_ok!(Marketplace::list_property(
            RuntimeOrigin::signed([0; 32].into()),
            3,
            bvec![10, 10],
            10_000,
            100,
            bvec![22, 22],
            false
        ));
        run_to_block(100);
        assert_noop!(
            Marketplace::upgrade_object(RuntimeOrigin::signed([0; 32].into()), 1, 300),
            Error::<Test>::ListingExpired
        );
    })
}

// delist_shares tests

#[test]
fn delist_single_share_works() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);
        // Region setup and role assignments
        assert_ok!(XcavateWhitelist::add_admin(RuntimeOrigin::root(), [20; 32].into(),));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [8; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));
        new_region_helper();
        assert_ok!(Regions::create_new_location(
            RuntimeOrigin::signed([8; 32].into()),
            3,
            bvec![10, 10]
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [0; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateDeveloper
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [1; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [2; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [30; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [31; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [10; 32].into(),
            pallet_xcavate_whitelist::Role::Lawyer
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [11; 32].into(),
            pallet_xcavate_whitelist::Role::Lawyer
        ));
        assert_ok!(Regions::register_lawyer(RuntimeOrigin::signed([10; 32].into()), 3,));
        assert_ok!(Regions::register_lawyer(RuntimeOrigin::signed([11; 32].into()), 3,));

        // Property listing and purchases
        assert_ok!(Marketplace::list_property(
            RuntimeOrigin::signed([0; 32].into()),
            3,
            bvec![10, 10],
            10_000,
            100,
            bvec![22, 22],
            false
        ));
        assert_ok!(Marketplace::buy_property_shares(
            RuntimeOrigin::signed([1; 32].into()),
            0,
            40,
            1984
        ));
        assert_ok!(Marketplace::buy_property_shares(
            RuntimeOrigin::signed([30; 32].into()),
            0,
            30,
            1984
        ));
        assert_ok!(Marketplace::buy_property_shares(
            RuntimeOrigin::signed([31; 32].into()),
            0,
            30,
            1984
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [5; 32].into(),
            pallet_xcavate_whitelist::Role::SpvConfirmation
        ));
        assert_ok!(Marketplace::create_spv(RuntimeOrigin::signed([5; 32].into()), 0,));
        assert_ok!(Marketplace::claim_property_shares(RuntimeOrigin::signed([1; 32].into()), 0,));
        assert_ok!(Marketplace::claim_property_shares(RuntimeOrigin::signed([30; 32].into()), 0,));
        assert_ok!(Marketplace::claim_property_shares(RuntimeOrigin::signed([31; 32].into()), 0,));

        // Legal process
        assert_ok!(Marketplace::lawyer_claim_property(
            RuntimeOrigin::signed([10; 32].into()),
            0,
            crate::LegalProperty::RealEstateDeveloperSide,
            4_000,
        ));
        assert_ok!(Marketplace::approve_developer_lawyer(
            RuntimeOrigin::signed([0; 32].into()),
            0,
            true
        ));
        assert_ok!(Marketplace::lawyer_claim_property(
            RuntimeOrigin::signed([11; 32].into()),
            0,
            crate::LegalProperty::SpvSide,
            4_000,
        ));
        assert_ok!(Marketplace::vote_on_spv_lawyer(
            RuntimeOrigin::signed([1; 32].into()),
            0,
            crate::Vote::Yes,
            40
        ));
        assert_ok!(Marketplace::vote_on_spv_lawyer(
            RuntimeOrigin::signed([30; 32].into()),
            0,
            crate::Vote::Yes,
            30
        ));
        run_to_block(91);
        assert_ok!(Marketplace::finalize_spv_lawyer(RuntimeOrigin::signed([1; 32].into()), 0,));
        assert_ok!(Marketplace::unfreeze_spv_lawyer_shares(
            RuntimeOrigin::signed([1; 32].into()),
            0,
        ));
        assert_ok!(Marketplace::lawyer_confirm_documents(
            RuntimeOrigin::signed([10; 32].into()),
            0,
            true,
        ));
        assert_ok!(Marketplace::lawyer_confirm_documents(
            RuntimeOrigin::signed([11; 32].into()),
            0,
            true,
        ));

        // Secondary market
        assert_ok!(Marketplace::relist_shares(RuntimeOrigin::signed([1; 32].into()), 0, 1000, 1));
        assert_eq!(LocalAssets::balance(0, &[1; 32].into()), 39);
        assert_eq!(LocalAssets::balance(0, &Marketplace::property_account_id(0)), 1);
        assert_ok!(Marketplace::delist_shares(RuntimeOrigin::signed([1; 32].into()), 1));
        assert_eq!(ShareListings::<Test>::get(0), None);
        assert_ok!(Marketplace::relist_shares(RuntimeOrigin::signed([1; 32].into()), 0, 1000, 3));
        assert_eq!(LocalAssets::balance(0, &[1; 32].into()), 37);
        assert_eq!(LocalAssets::balance(0, &Marketplace::property_account_id(0)), 3);
        assert_ok!(Marketplace::buy_relisted_shares(
            RuntimeOrigin::signed([2; 32].into()),
            2,
            2,
            1984
        ));
        assert_eq!(LocalAssets::balance(0, &Marketplace::property_account_id(0)), 1);
        assert_ok!(Marketplace::delist_shares(RuntimeOrigin::signed([1; 32].into()), 2));
        assert_eq!(LocalAssets::balance(0, &[2; 32].into()), 2);
        assert_eq!(LocalAssets::balance(0, &[1; 32].into()), 38);
    })
}

#[test]
fn delist_fails() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);
        // Region setup and role assignments
        assert_ok!(XcavateWhitelist::add_admin(RuntimeOrigin::root(), [20; 32].into(),));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [8; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));
        new_region_helper();
        assert_ok!(Regions::create_new_location(
            RuntimeOrigin::signed([8; 32].into()),
            3,
            bvec![10, 10]
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [0; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateDeveloper
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [1; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [2; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [30; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [4; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [10; 32].into(),
            pallet_xcavate_whitelist::Role::Lawyer
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [11; 32].into(),
            pallet_xcavate_whitelist::Role::Lawyer
        ));
        assert_ok!(Regions::register_lawyer(RuntimeOrigin::signed([10; 32].into()), 3,));
        assert_ok!(Regions::register_lawyer(RuntimeOrigin::signed([11; 32].into()), 3,));

        // Property listing and purchases
        assert_ok!(Marketplace::list_property(
            RuntimeOrigin::signed([0; 32].into()),
            3,
            bvec![10, 10],
            10_000,
            100,
            bvec![22, 22],
            false
        ));
        assert_ok!(Marketplace::buy_property_shares(
            RuntimeOrigin::signed([1; 32].into()),
            0,
            40,
            1984
        ));
        assert_ok!(Marketplace::buy_property_shares(
            RuntimeOrigin::signed([2; 32].into()),
            0,
            30,
            1984
        ));
        assert_ok!(Marketplace::buy_property_shares(
            RuntimeOrigin::signed([30; 32].into()),
            0,
            30,
            1984
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [5; 32].into(),
            pallet_xcavate_whitelist::Role::SpvConfirmation
        ));
        assert_ok!(Marketplace::create_spv(RuntimeOrigin::signed([5; 32].into()), 0,));
        assert_ok!(Marketplace::claim_property_shares(RuntimeOrigin::signed([1; 32].into()), 0,));
        assert_ok!(Marketplace::claim_property_shares(RuntimeOrigin::signed([2; 32].into()), 0,));
        assert_ok!(Marketplace::claim_property_shares(RuntimeOrigin::signed([30; 32].into()), 0,));

        // Legal process
        assert_ok!(Marketplace::lawyer_claim_property(
            RuntimeOrigin::signed([10; 32].into()),
            0,
            crate::LegalProperty::RealEstateDeveloperSide,
            4_000,
        ));
        assert_ok!(Marketplace::approve_developer_lawyer(
            RuntimeOrigin::signed([0; 32].into()),
            0,
            true
        ));
        assert_ok!(Marketplace::lawyer_claim_property(
            RuntimeOrigin::signed([11; 32].into()),
            0,
            crate::LegalProperty::SpvSide,
            4_000,
        ));
        assert_ok!(Marketplace::vote_on_spv_lawyer(
            RuntimeOrigin::signed([1; 32].into()),
            0,
            crate::Vote::Yes,
            40
        ));
        assert_ok!(Marketplace::vote_on_spv_lawyer(
            RuntimeOrigin::signed([2; 32].into()),
            0,
            crate::Vote::Yes,
            30
        ));
        run_to_block(91);
        assert_ok!(Marketplace::finalize_spv_lawyer(RuntimeOrigin::signed([1; 32].into()), 0,));
        assert_ok!(Marketplace::unfreeze_spv_lawyer_shares(
            RuntimeOrigin::signed([1; 32].into()),
            0,
        ));
        assert_ok!(Marketplace::lawyer_confirm_documents(
            RuntimeOrigin::signed([10; 32].into()),
            0,
            true,
        ));
        assert_ok!(Marketplace::lawyer_confirm_documents(
            RuntimeOrigin::signed([11; 32].into()),
            0,
            true,
        ));

        // Secondary market
        assert_ok!(Marketplace::relist_shares(RuntimeOrigin::signed([1; 32].into()), 0, 1000, 1));
        assert_noop!(
            Marketplace::delist_shares(RuntimeOrigin::signed([4; 32].into()), 1),
            Error::<Test>::NoPermission
        );
        assert_noop!(
            Marketplace::delist_shares(RuntimeOrigin::signed([1; 32].into()), 2),
            Error::<Test>::ShareNotForSale
        );
    })
}

// Tests for listing multiple objects in different regions
#[test]
fn listing_objects_in_different_regions() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);
        // Region setup and role assignments
        assert_ok!(XcavateWhitelist::add_admin(RuntimeOrigin::root(), [20; 32].into(),));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [8; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));
        // Create first region
        new_region_helper();
        // Create second region
        assert_ok!(Regions::propose_new_region(
            RuntimeOrigin::signed([8; 32].into()),
            RegionIdentifier::France
        ));
        assert_ok!(Regions::vote_on_region_proposal(
            RuntimeOrigin::signed([8; 32].into()),
            2,
            pallet_regions::Vote::Yes,
            1_000_000
        ));
        run_to_block(91);
        assert_ok!(Regions::bid_on_region(RuntimeOrigin::signed([8; 32].into()), 2, 100_000));
        run_to_block(121);
        assert_ok!(Regions::create_new_region(
            RuntimeOrigin::signed([8; 32].into()),
            2,
            30,
            Permill::from_percent(3)
        ));
        // Create third region
        assert_ok!(Regions::propose_new_region(
            RuntimeOrigin::signed([8; 32].into()),
            RegionIdentifier::India
        ));
        assert_ok!(Regions::vote_on_region_proposal(
            RuntimeOrigin::signed([8; 32].into()),
            4,
            pallet_regions::Vote::Yes,
            1_000_000
        ));
        run_to_block(151);
        assert_ok!(Regions::bid_on_region(RuntimeOrigin::signed([8; 32].into()), 4, 100_000));
        run_to_block(181);
        assert_ok!(Regions::create_new_region(
            RuntimeOrigin::signed([8; 32].into()),
            4,
            30,
            Permill::from_percent(3)
        ));
        assert_ok!(Regions::create_new_location(
            RuntimeOrigin::signed([8; 32].into()),
            3,
            bvec![10, 10]
        ));
        assert_ok!(Regions::create_new_location(
            RuntimeOrigin::signed([8; 32].into()),
            2,
            bvec![10, 10]
        ));
        assert_ok!(Regions::create_new_location(
            RuntimeOrigin::signed([8; 32].into()),
            4,
            bvec![10, 10]
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [0; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateDeveloper
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [1; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [2; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [30; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [31; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [10; 32].into(),
            pallet_xcavate_whitelist::Role::Lawyer
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [11; 32].into(),
            pallet_xcavate_whitelist::Role::Lawyer
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [12; 32].into(),
            pallet_xcavate_whitelist::Role::Lawyer
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [13; 32].into(),
            pallet_xcavate_whitelist::Role::Lawyer
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [14; 32].into(),
            pallet_xcavate_whitelist::Role::Lawyer
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [15; 32].into(),
            pallet_xcavate_whitelist::Role::Lawyer
        ));
        assert_ok!(Regions::register_lawyer(RuntimeOrigin::signed([10; 32].into()), 3,));
        assert_ok!(Regions::register_lawyer(RuntimeOrigin::signed([11; 32].into()), 3,));
        assert_ok!(Regions::register_lawyer(RuntimeOrigin::signed([12; 32].into()), 2,));
        assert_ok!(Regions::register_lawyer(RuntimeOrigin::signed([13; 32].into()), 2,));
        assert_ok!(Regions::register_lawyer(RuntimeOrigin::signed([14; 32].into()), 4,));
        assert_ok!(Regions::register_lawyer(RuntimeOrigin::signed([15; 32].into()), 4,));
        // Listing of different properties
        assert_ok!(Marketplace::list_property(
            RuntimeOrigin::signed([0; 32].into()),
            3,
            bvec![10, 10],
            10_000,
            100,
            bvec![22, 22],
            false
        ));
        assert_ok!(Marketplace::list_property(
            RuntimeOrigin::signed([0; 32].into()),
            2,
            bvec![10, 10],
            10_000,
            100,
            bvec![22, 22],
            false
        ));
        assert_ok!(Marketplace::list_property(
            RuntimeOrigin::signed([0; 32].into()),
            4,
            bvec![10, 10],
            10_000,
            100,
            bvec![22, 22],
            false
        ));

        // First property purchases
        assert_ok!(Marketplace::buy_property_shares(
            RuntimeOrigin::signed([1; 32].into()),
            1,
            40,
            1984
        ));
        assert_ok!(Marketplace::buy_property_shares(
            RuntimeOrigin::signed([30; 32].into()),
            1,
            30,
            1984
        ));
        assert_ok!(Marketplace::buy_property_shares(
            RuntimeOrigin::signed([31; 32].into()),
            1,
            30,
            1984
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [5; 32].into(),
            pallet_xcavate_whitelist::Role::SpvConfirmation
        ));
        assert_ok!(Marketplace::create_spv(RuntimeOrigin::signed([5; 32].into()), 1,));
        assert_ok!(Marketplace::claim_property_shares(RuntimeOrigin::signed([1; 32].into()), 1,));
        assert_ok!(Marketplace::claim_property_shares(RuntimeOrigin::signed([30; 32].into()), 1,));
        assert_ok!(Marketplace::claim_property_shares(RuntimeOrigin::signed([31; 32].into()), 1,));

        // Second property purchases
        assert_ok!(Marketplace::buy_property_shares(
            RuntimeOrigin::signed([2; 32].into()),
            2,
            40,
            1984
        ));
        assert_ok!(Marketplace::buy_property_shares(
            RuntimeOrigin::signed([30; 32].into()),
            2,
            30,
            1984
        ));
        assert_ok!(Marketplace::buy_property_shares(
            RuntimeOrigin::signed([31; 32].into()),
            2,
            30,
            1984
        ));
        assert_ok!(Marketplace::create_spv(RuntimeOrigin::signed([5; 32].into()), 2,));
        assert_ok!(Marketplace::claim_property_shares(RuntimeOrigin::signed([2; 32].into()), 2,));
        assert_ok!(Marketplace::claim_property_shares(RuntimeOrigin::signed([30; 32].into()), 2,));
        assert_ok!(Marketplace::claim_property_shares(RuntimeOrigin::signed([31; 32].into()), 2,));

        // Legal process for first property
        assert_ok!(Marketplace::lawyer_claim_property(
            RuntimeOrigin::signed([12; 32].into()),
            1,
            crate::LegalProperty::RealEstateDeveloperSide,
            4_000,
        ));
        assert_ok!(Marketplace::approve_developer_lawyer(
            RuntimeOrigin::signed([0; 32].into()),
            1,
            true
        ));
        assert_ok!(Marketplace::lawyer_claim_property(
            RuntimeOrigin::signed([13; 32].into()),
            1,
            crate::LegalProperty::SpvSide,
            4_000,
        ));
        assert_ok!(Marketplace::vote_on_spv_lawyer(
            RuntimeOrigin::signed([1; 32].into()),
            1,
            crate::Vote::Yes,
            40
        ));
        assert_ok!(Marketplace::vote_on_spv_lawyer(
            RuntimeOrigin::signed([30; 32].into()),
            1,
            crate::Vote::Yes,
            30
        ));
        run_to_block(221);
        assert_ok!(Marketplace::finalize_spv_lawyer(RuntimeOrigin::signed([1; 32].into()), 1,));
        assert_ok!(Marketplace::unfreeze_spv_lawyer_shares(
            RuntimeOrigin::signed([1; 32].into()),
            0,
        ));
        assert_ok!(Marketplace::lawyer_confirm_documents(
            RuntimeOrigin::signed([12; 32].into()),
            1,
            true,
        ));
        assert_ok!(Marketplace::lawyer_confirm_documents(
            RuntimeOrigin::signed([13; 32].into()),
            1,
            true,
        ));

        // Legal process for second property
        assert_ok!(Marketplace::lawyer_claim_property(
            RuntimeOrigin::signed([14; 32].into()),
            2,
            crate::LegalProperty::RealEstateDeveloperSide,
            4_000,
        ));
        assert_ok!(Marketplace::lawyer_claim_property(
            RuntimeOrigin::signed([15; 32].into()),
            2,
            crate::LegalProperty::SpvSide,
            4_000,
        ));
        assert_ok!(Marketplace::approve_developer_lawyer(
            RuntimeOrigin::signed([0; 32].into()),
            2,
            true
        ));
        assert_ok!(Marketplace::vote_on_spv_lawyer(
            RuntimeOrigin::signed([2; 32].into()),
            2,
            crate::Vote::Yes,
            40
        ));
        assert_ok!(Marketplace::vote_on_spv_lawyer(
            RuntimeOrigin::signed([30; 32].into()),
            2,
            crate::Vote::Yes,
            30
        ));
        run_to_block(251);
        assert_ok!(Marketplace::finalize_spv_lawyer(RuntimeOrigin::signed([2; 32].into()), 2,));
        assert_ok!(Marketplace::lawyer_confirm_documents(
            RuntimeOrigin::signed([14; 32].into()),
            2,
            true,
        ));
        assert_ok!(Marketplace::lawyer_confirm_documents(
            RuntimeOrigin::signed([15; 32].into()),
            2,
            true,
        ));
        assert_eq!(PropertyAssetInfo::<Test>::get(1).unwrap().spv_created, true);
        assert_eq!(PropertyAssetInfo::<Test>::get(2).unwrap().spv_created, true);

        // Secondary market purchases
        assert_ok!(Marketplace::relist_shares(RuntimeOrigin::signed([1; 32].into()), 1, 1000, 40));
        assert_ok!(Marketplace::buy_relisted_shares(
            RuntimeOrigin::signed([2; 32].into()),
            3,
            40,
            1984
        ));
        assert_eq!(LocalAssets::balance(1, &[2; 32].into()), 40);
        assert_eq!(LocalAssets::balance(2, &[2; 32].into()), 40);
    })
}

// cancel_property_purchase tests

#[test]
fn cancel_property_purchase_works() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);
        // Region setup and role assignments
        assert_ok!(XcavateWhitelist::add_admin(RuntimeOrigin::root(), [20; 32].into(),));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [8; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));
        new_region_helper();
        assert_ok!(Regions::create_new_location(
            RuntimeOrigin::signed([8; 32].into()),
            3,
            bvec![10, 10]
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [0; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateDeveloper
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [1; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [2; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));

        // Property listing and purchases
        assert_ok!(Marketplace::list_property(
            RuntimeOrigin::signed([0; 32].into()),
            3,
            bvec![10, 10],
            10_000,
            100,
            bvec![22, 22],
            false
        ));
        assert_ok!(Marketplace::buy_property_shares(
            RuntimeOrigin::signed([1; 32].into()),
            0,
            30,
            1984
        ));
        assert_ok!(Marketplace::buy_property_shares(
            RuntimeOrigin::signed([2; 32].into()),
            0,
            30,
            1984
        ));
        assert_eq!(OngoingObjectListing::<Test>::get(0).unwrap().listed_share_amount, 40);
        assert_eq!(
            ShareOwner::<Test>::get::<AccountId, u32>([1; 32].into(), 0).unwrap().share_amount,
            30
        );
        assert_eq!(
            ShareOwner::<Test>::get::<AccountId, u32>([2; 32].into(), 0).unwrap().share_amount,
            30
        );
        assert_eq!(ForeignAssets::balance(1984, &[1; 32].into()), 1_188_000);
        assert_eq!(ForeignAssets::total_balance(1984, &[1; 32].into()), 1_500_000);
        assert_eq!(AssetsHolder::total_balance_on_hold(1984, &[1; 32].into()), 312_000);
        assert_eq!(AssetsHolder::total_balance_on_hold(1984, &[2; 32].into()), 312_000);

        // Cancel property purchase
        assert_ok!(Marketplace::cancel_property_purchase(RuntimeOrigin::signed([1; 32].into()), 0));
        assert_eq!(OngoingObjectListing::<Test>::get(0).unwrap().listed_share_amount, 70);
        assert_eq!(AssetsHolder::total_balance_on_hold(1984, &[1; 32].into()), 0);
        assert_eq!(ForeignAssets::balance(1984, &[1; 32].into()), 1_500_000);
    })
}

#[test]
fn cancel_property_purchase_fails() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);
        // Region setup and role assignments
        assert_ok!(XcavateWhitelist::add_admin(RuntimeOrigin::root(), [20; 32].into(),));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [8; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));
        new_region_helper();
        assert_ok!(Regions::create_new_location(
            RuntimeOrigin::signed([8; 32].into()),
            3,
            bvec![10, 10]
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [0; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateDeveloper
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [1; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [2; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [3; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [30; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));
        assert_noop!(
            Marketplace::cancel_property_purchase(RuntimeOrigin::signed([1; 32].into()), 0),
            Error::<Test>::ListingNotFound
        );

        // Property listing and purchases
        assert_ok!(Marketplace::list_property(
            RuntimeOrigin::signed([0; 32].into()),
            3,
            bvec![10, 10],
            10_000,
            100,
            bvec![22, 22],
            false
        ));
        assert_ok!(Marketplace::buy_property_shares(
            RuntimeOrigin::signed([1; 32].into()),
            0,
            30,
            1984
        ));
        assert_ok!(Marketplace::buy_property_shares(
            RuntimeOrigin::signed([2; 32].into()),
            0,
            30,
            1984
        ));
        assert_noop!(
            Marketplace::cancel_property_purchase(RuntimeOrigin::signed([3; 32].into()), 0),
            Error::<Test>::ShareOwnerNotFound
        );
        assert_ok!(Marketplace::buy_property_shares(
            RuntimeOrigin::signed([30; 32].into()),
            0,
            40,
            1984
        ));
        assert_noop!(
            Marketplace::cancel_property_purchase(RuntimeOrigin::signed([1; 32].into()), 0),
            Error::<Test>::PropertyAlreadySold
        );
    })
}

#[test]
fn cancel_property_purchase_fails_2() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);
        // Region setup and role assignments
        assert_ok!(XcavateWhitelist::add_admin(RuntimeOrigin::root(), [20; 32].into(),));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [8; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));
        new_region_helper();
        assert_ok!(Regions::create_new_location(
            RuntimeOrigin::signed([8; 32].into()),
            3,
            bvec![10, 10]
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [0; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateDeveloper
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [1; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [2; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));
        // Property listing and purchases
        assert_ok!(Marketplace::list_property(
            RuntimeOrigin::signed([0; 32].into()),
            3,
            bvec![10, 10],
            10_000,
            100,
            bvec![22, 22],
            false
        ));
        assert_ok!(Marketplace::buy_property_shares(
            RuntimeOrigin::signed([1; 32].into()),
            0,
            30,
            1984
        ));
        assert_ok!(Marketplace::buy_property_shares(
            RuntimeOrigin::signed([2; 32].into()),
            0,
            40,
            1984
        ));
        run_to_block(100);
        assert_noop!(
            Marketplace::cancel_property_purchase(RuntimeOrigin::signed([1; 32].into()), 0),
            Error::<Test>::ListingExpired
        );
    })
}

// withdraw_expired tests

#[test]
fn withdraw_expired_works() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);
        // Region setup and role assignments
        assert_ok!(XcavateWhitelist::add_admin(RuntimeOrigin::root(), [20; 32].into(),));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [8; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));
        new_region_helper();
        assert_ok!(Regions::create_new_location(
            RuntimeOrigin::signed([8; 32].into()),
            3,
            bvec![10, 10]
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [0; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateDeveloper
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [1; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));

        // Property listing and purchases
        assert_ok!(Marketplace::list_property(
            RuntimeOrigin::signed([0; 32].into()),
            3,
            bvec![10, 10],
            10_000,
            100,
            bvec![22, 22],
            false
        ));
        assert_ok!(Marketplace::buy_property_shares(
            RuntimeOrigin::signed([1; 32].into()),
            0,
            30,
            1984
        ));
        assert_eq!(OngoingObjectListing::<Test>::get(0).unwrap().listed_share_amount, 70);
        assert_eq!(
            ShareOwner::<Test>::get::<AccountId, u32>([1; 32].into(), 0).unwrap().share_amount,
            30
        );
        assert_eq!(ForeignAssets::balance(1984, &[1; 32].into()), 1_188_000);
        assert_eq!(ForeignAssets::total_balance(1984, &[1; 32].into()), 1_500_000);
        assert_eq!(AssetsHolder::total_balance_on_hold(1984, &[1; 32].into()), 312_000);
        run_to_block(100);

        // Withdraw expired listing
        assert_ok!(Marketplace::withdraw_expired(RuntimeOrigin::signed([1; 32].into()), 0));
        assert_eq!(OngoingObjectListing::<Test>::get(0), None);
        assert_eq!(ShareOwner::<Test>::get::<AccountId, u32>([1; 32].into(), 0), None);
        assert_eq!(ForeignAssets::balance(1984, &[1; 32].into()), 1_500_000);
        assert_eq!(AssetsHolder::total_balance_on_hold(1984, &[1; 32].into()), 0);
    })
}

#[test]
fn withdraw_expired_works_2() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);
        // Region setup and role assignments
        assert_ok!(XcavateWhitelist::add_admin(RuntimeOrigin::root(), [20; 32].into(),));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [8; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));
        new_region_helper();
        assert_ok!(Regions::create_new_location(
            RuntimeOrigin::signed([8; 32].into()),
            3,
            bvec![10, 10]
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [0; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateDeveloper
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [1; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [2; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [3; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));

        // Property listing and purchases
        assert_ok!(Marketplace::list_property(
            RuntimeOrigin::signed([0; 32].into()),
            3,
            bvec![10, 10],
            1_000,
            100,
            bvec![22, 22],
            false
        ));
        assert_ok!(Marketplace::buy_property_shares(
            RuntimeOrigin::signed([1; 32].into()),
            0,
            30,
            1984
        ));
        assert_ok!(Marketplace::buy_property_shares(
            RuntimeOrigin::signed([2; 32].into()),
            0,
            20,
            1984
        ));
        assert_ok!(Marketplace::buy_property_shares(
            RuntimeOrigin::signed([3; 32].into()),
            0,
            4,
            1984
        ));
        assert_eq!(OngoingObjectListing::<Test>::get(0).unwrap().listed_share_amount, 46);
        assert_eq!(
            ShareOwner::<Test>::get::<AccountId, u32>([1; 32].into(), 0).unwrap().share_amount,
            30
        );
        assert_eq!(ForeignAssets::balance(1984, &[1; 32].into()), 1_468_800);
        assert_eq!(ForeignAssets::total_balance(1984, &[1; 32].into()), 1_500_000);
        assert_eq!(ForeignAssets::balance(1984, &[2; 32].into()), 1_129_200);
        assert_eq!(ForeignAssets::total_balance(1984, &[2; 32].into()), 1_150_000);
        assert_eq!(ForeignAssets::balance(1984, &[3; 32].into()), 840);
        assert_eq!(ForeignAssets::total_balance(1984, &[3; 32].into()), 5_000);
        assert_eq!(AssetsHolder::total_balance_on_hold(1984, &[1; 32].into()), 31_200);
        assert_eq!(AssetsHolder::total_balance_on_hold(1984, &[2; 32].into()), 20_800);
        assert_eq!(AssetsHolder::total_balance_on_hold(1984, &[3; 32].into()), 4_160);
        run_to_block(100);

        // Withdraw expired listing
        assert_ok!(Marketplace::withdraw_expired(RuntimeOrigin::signed([1; 32].into()), 0));
        assert_eq!(OngoingObjectListing::<Test>::get(0).unwrap().listed_share_amount, 76);
        assert_eq!(ShareOwner::<Test>::get::<AccountId, u32>([1; 32].into(), 0), None);
        assert_eq!(
            ShareOwner::<Test>::get::<AccountId, u32>([3; 32].into(), 0).unwrap().share_amount,
            4
        );
        assert_eq!(ForeignAssets::balance(1984, &[1; 32].into()), 1_500_000);
        assert_eq!(AssetsHolder::total_balance_on_hold(1984, &[1; 32].into()), 0);
        assert_ok!(Marketplace::withdraw_expired(RuntimeOrigin::signed([2; 32].into()), 0));
        assert_eq!(
            Balances::balance_on_hold(&HoldReason::ListingDepositReserve.into(), &([0; 32].into())),
            200_000
        );
        assert_eq!(Balances::free_balance(&(Marketplace::property_account_id(0))), 99);
        assert_eq!(Balances::balance(&(Marketplace::property_account_id(0))), 99);
        assert_ok!(Marketplace::withdraw_expired(RuntimeOrigin::signed([3; 32].into()), 0));
        assert_eq!(Balances::free_balance(&(Marketplace::property_account_id(0))), 0);
        assert_eq!(Balances::balance(&(Marketplace::property_account_id(0))), 0);
        assert_eq!(ShareOwner::<Test>::get::<AccountId, u32>([1; 32].into(), 0), None);
        assert_eq!(ForeignAssets::balance(1984, &[3; 32].into()), 5_000);
        assert_eq!(AssetsHolder::total_balance_on_hold(1984, &[3; 32].into()), 0);
        assert_eq!(
            Balances::balance_on_hold(&HoldReason::ListingDepositReserve.into(), &([0; 32].into())),
            0
        );
    })
}

#[test]
fn withdraw_expired_fails() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);
        // Region setup and role assignments
        assert_ok!(XcavateWhitelist::add_admin(RuntimeOrigin::root(), [20; 32].into(),));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [8; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));
        new_region_helper();
        assert_ok!(Regions::create_new_location(
            RuntimeOrigin::signed([8; 32].into()),
            3,
            bvec![10, 10]
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [0; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateDeveloper
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [1; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [2; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [30; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));
        assert_noop!(
            Marketplace::withdraw_expired(RuntimeOrigin::signed([1; 32].into()), 0),
            Error::<Test>::ListingNotFound
        );

        // Property listing and purchases
        assert_ok!(Marketplace::list_property(
            RuntimeOrigin::signed([0; 32].into()),
            3,
            bvec![10, 10],
            10_000,
            100,
            bvec![22, 22],
            false
        ));
        assert_noop!(
            Marketplace::withdraw_expired(RuntimeOrigin::signed([1; 32].into()), 0),
            Error::<Test>::ListingNotExpired
        );
        assert_ok!(Marketplace::buy_property_shares(
            RuntimeOrigin::signed([1; 32].into()),
            0,
            40,
            1984
        ));
        assert_ok!(Marketplace::buy_property_shares(
            RuntimeOrigin::signed([2; 32].into()),
            0,
            30,
            1984
        ));
        assert_ok!(Marketplace::buy_property_shares(
            RuntimeOrigin::signed([30; 32].into()),
            0,
            30,
            1984
        ));
        run_to_block(100);

        // Withdraw expired listing
        assert_noop!(
            Marketplace::withdraw_expired(RuntimeOrigin::signed([1; 32].into()), 0),
            Error::<Test>::PropertyAlreadySold
        );
    })
}

#[test]
fn withdraw_expired_fails_2() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);
        // Region setup and role assignments
        assert_ok!(XcavateWhitelist::add_admin(RuntimeOrigin::root(), [20; 32].into(),));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [8; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));
        new_region_helper();
        assert_ok!(Regions::create_new_location(
            RuntimeOrigin::signed([8; 32].into()),
            3,
            bvec![10, 10]
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [0; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateDeveloper
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [0; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [1; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [2; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));

        // Property listing and purchases
        assert_ok!(Marketplace::list_property(
            RuntimeOrigin::signed([0; 32].into()),
            3,
            bvec![10, 10],
            10_000,
            100,
            bvec![22, 22],
            false
        ));
        assert_ok!(Marketplace::buy_property_shares(
            RuntimeOrigin::signed([1; 32].into()),
            0,
            39,
            1984
        ));
        run_to_block(100);

        // Withdraw expired listing
        assert_noop!(
            Marketplace::withdraw_expired(RuntimeOrigin::signed([2; 32].into()), 0),
            Error::<Test>::ShareOwnerNotFound
        );
    })
}

// send_property_shares tests

#[test]
fn send_property_shares_works() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);
        // Region setup and role assignments
        assert_ok!(XcavateWhitelist::add_admin(RuntimeOrigin::root(), [20; 32].into(),));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [8; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));
        new_region_helper();
        assert_ok!(Regions::create_new_location(
            RuntimeOrigin::signed([8; 32].into()),
            3,
            bvec![10, 10]
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [0; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateDeveloper
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [1; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [2; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [3; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [30; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [31; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [10; 32].into(),
            pallet_xcavate_whitelist::Role::Lawyer
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [11; 32].into(),
            pallet_xcavate_whitelist::Role::Lawyer
        ));
        assert_ok!(Regions::register_lawyer(RuntimeOrigin::signed([10; 32].into()), 3,));
        assert_ok!(Regions::register_lawyer(RuntimeOrigin::signed([11; 32].into()), 3,));

        // Property listing and purchases
        assert_ok!(Marketplace::list_property(
            RuntimeOrigin::signed([0; 32].into()),
            3,
            bvec![10, 10],
            10_000,
            100,
            bvec![22, 22],
            false
        ));
        assert_ok!(Marketplace::buy_property_shares(
            RuntimeOrigin::signed([1; 32].into()),
            0,
            40,
            1984
        ));
        assert_ok!(Marketplace::buy_property_shares(
            RuntimeOrigin::signed([30; 32].into()),
            0,
            30,
            1984
        ));
        assert_ok!(Marketplace::buy_property_shares(
            RuntimeOrigin::signed([31; 32].into()),
            0,
            30,
            1984
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [5; 32].into(),
            pallet_xcavate_whitelist::Role::SpvConfirmation
        ));
        assert_ok!(Marketplace::create_spv(RuntimeOrigin::signed([5; 32].into()), 0,));
        assert_ok!(Marketplace::claim_property_shares(RuntimeOrigin::signed([1; 32].into()), 0));
        assert_ok!(Marketplace::claim_property_shares(RuntimeOrigin::signed([30; 32].into()), 0));
        assert_ok!(Marketplace::claim_property_shares(RuntimeOrigin::signed([31; 32].into()), 0));

        // Legal process
        assert_ok!(Marketplace::lawyer_claim_property(
            RuntimeOrigin::signed([10; 32].into()),
            0,
            crate::LegalProperty::RealEstateDeveloperSide,
            4_000,
        ));
        assert_ok!(Marketplace::approve_developer_lawyer(
            RuntimeOrigin::signed([0; 32].into()),
            0,
            true
        ));
        assert_ok!(Marketplace::lawyer_claim_property(
            RuntimeOrigin::signed([11; 32].into()),
            0,
            crate::LegalProperty::SpvSide,
            4_000,
        ));
        assert_ok!(Marketplace::vote_on_spv_lawyer(
            RuntimeOrigin::signed([1; 32].into()),
            0,
            crate::Vote::Yes,
            40
        ));
        assert_ok!(Marketplace::vote_on_spv_lawyer(
            RuntimeOrigin::signed([30; 32].into()),
            0,
            crate::Vote::Yes,
            30
        ));
        run_to_block(91);
        assert_ok!(Marketplace::finalize_spv_lawyer(RuntimeOrigin::signed([1; 32].into()), 0,));
        assert_ok!(Marketplace::unfreeze_spv_lawyer_shares(
            RuntimeOrigin::signed([1; 32].into()),
            0,
        ));
        assert_ok!(Marketplace::lawyer_confirm_documents(
            RuntimeOrigin::signed([10; 32].into()),
            0,
            true,
        ));
        assert_ok!(Marketplace::lawyer_confirm_documents(
            RuntimeOrigin::signed([11; 32].into()),
            0,
            true,
        ));

        // Sending property shares
        assert_eq!(PropertyOwnerShares::<Test>::get::<u32, AccountId>(0, [1; 32].into()), 40);
        assert_eq!(LocalAssets::balance(0, &[1; 32].into()), 40);
        assert_eq!(PropertyOwner::<Test>::get(0).len(), 3);
        assert_ok!(Marketplace::send_property_shares(
            RuntimeOrigin::signed([1; 32].into()),
            0,
            [2; 32].into(),
            20
        ));
        assert_eq!(LocalAssets::balance(0, &[1; 32].into()), 20);
        assert_eq!(PropertyOwner::<Test>::get(0).len(), 4);
        assert_eq!(PropertyOwnerShares::<Test>::get::<u32, AccountId>(0, [1; 32].into()), 20);
        assert_eq!(LocalAssets::balance(0, &[2; 32].into()), 20);
        assert_eq!(PropertyOwnerShares::<Test>::get::<u32, AccountId>(0, [2; 32].into()), 20);
        assert_ok!(Marketplace::send_property_shares(
            RuntimeOrigin::signed([1; 32].into()),
            0,
            [3; 32].into(),
            20
        ));
        assert_ok!(Marketplace::send_property_shares(
            RuntimeOrigin::signed([2; 32].into()),
            0,
            [3; 32].into(),
            20
        ));
        assert_eq!(LocalAssets::balance(0, &[1; 32].into()), 0);
        assert_eq!(PropertyOwner::<Test>::get(0).len(), 3);
        assert_eq!(LocalAssets::balance(0, &[2; 32].into()), 0);
        assert_eq!(PropertyOwnerShares::<Test>::get::<u32, AccountId>(0, [2; 32].into()), 0);
        assert_eq!(LocalAssets::balance(0, &[3; 32].into()), 40);
        assert_eq!(PropertyOwnerShares::<Test>::get::<u32, AccountId>(0, [3; 32].into()), 40);
    })
}

#[test]
fn send_property_shares_works_2() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);
        // Region setup and role assignments
        assert_ok!(XcavateWhitelist::add_admin(RuntimeOrigin::root(), [20; 32].into(),));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [8; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));
        new_region_helper();
        assert_ok!(Regions::create_new_location(
            RuntimeOrigin::signed([8; 32].into()),
            3,
            bvec![10, 10]
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [0; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateDeveloper
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [1; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [30; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [31; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [10; 32].into(),
            pallet_xcavate_whitelist::Role::Lawyer
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [11; 32].into(),
            pallet_xcavate_whitelist::Role::Lawyer
        ));
        assert_ok!(Regions::register_lawyer(RuntimeOrigin::signed([10; 32].into()), 3,));
        assert_ok!(Regions::register_lawyer(RuntimeOrigin::signed([11; 32].into()), 3,));

        // Property listing and purchases
        assert_ok!(Marketplace::list_property(
            RuntimeOrigin::signed([0; 32].into()),
            3,
            bvec![10, 10],
            5_000,
            200,
            bvec![22, 22],
            false
        ));
        assert_ok!(Marketplace::buy_property_shares(
            RuntimeOrigin::signed([1; 32].into()),
            0,
            80,
            1984
        ));
        assert_ok!(Marketplace::buy_property_shares(
            RuntimeOrigin::signed([30; 32].into()),
            0,
            60,
            1984
        ));
        assert_ok!(Marketplace::buy_property_shares(
            RuntimeOrigin::signed([31; 32].into()),
            0,
            60,
            1984
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [5; 32].into(),
            pallet_xcavate_whitelist::Role::SpvConfirmation
        ));
        assert_ok!(Marketplace::create_spv(RuntimeOrigin::signed([5; 32].into()), 0,));
        assert_ok!(Marketplace::claim_property_shares(RuntimeOrigin::signed([1; 32].into()), 0));
        assert_ok!(Marketplace::claim_property_shares(RuntimeOrigin::signed([30; 32].into()), 0));
        assert_ok!(Marketplace::claim_property_shares(RuntimeOrigin::signed([31; 32].into()), 0));

        // Legal process
        assert_ok!(Marketplace::lawyer_claim_property(
            RuntimeOrigin::signed([10; 32].into()),
            0,
            crate::LegalProperty::RealEstateDeveloperSide,
            4_000,
        ));
        assert_ok!(Marketplace::approve_developer_lawyer(
            RuntimeOrigin::signed([0; 32].into()),
            0,
            true
        ));
        assert_ok!(Marketplace::lawyer_claim_property(
            RuntimeOrigin::signed([11; 32].into()),
            0,
            crate::LegalProperty::SpvSide,
            4_000,
        ));
        assert_ok!(Marketplace::vote_on_spv_lawyer(
            RuntimeOrigin::signed([1; 32].into()),
            0,
            crate::Vote::Yes,
            80
        ));
        assert_ok!(Marketplace::vote_on_spv_lawyer(
            RuntimeOrigin::signed([30; 32].into()),
            0,
            crate::Vote::Yes,
            45
        ));
        run_to_block(91);
        assert_ok!(Marketplace::finalize_spv_lawyer(RuntimeOrigin::signed([1; 32].into()), 0,));
        assert_ok!(Marketplace::unfreeze_spv_lawyer_shares(
            RuntimeOrigin::signed([1; 32].into()),
            0,
        ));
        assert_ok!(Marketplace::unfreeze_spv_lawyer_shares(
            RuntimeOrigin::signed([30; 32].into()),
            0,
        ));
        assert_ok!(Marketplace::lawyer_confirm_documents(
            RuntimeOrigin::signed([10; 32].into()),
            0,
            true,
        ));
        assert_ok!(Marketplace::lawyer_confirm_documents(
            RuntimeOrigin::signed([11; 32].into()),
            0,
            true,
        ));

        // Sending property shares
        assert_eq!(PropertyOwnerShares::<Test>::get::<u32, AccountId>(0, [1; 32].into()), 80);
        assert_ok!(Marketplace::send_property_shares(
            RuntimeOrigin::signed([30; 32].into()),
            0,
            [1; 32].into(),
            19
        ));
        assert_eq!(PropertyOwnerShares::<Test>::get::<u32, AccountId>(0, [1; 32].into()), 99);
        assert_noop!(
            Marketplace::send_property_shares(
                RuntimeOrigin::signed([30; 32].into()),
                0,
                [1; 32].into(),
                19
            ),
            Error::<Test>::ExceedsMaxOwnership
        );
        assert_noop!(
            Marketplace::send_property_shares(
                RuntimeOrigin::signed([1; 32].into()),
                0,
                [30; 32].into(),
                80
            ),
            Error::<Test>::ExceedsMaxOwnership
        );
        assert_ok!(Marketplace::send_property_shares(
            RuntimeOrigin::signed([1; 32].into()),
            0,
            [30; 32].into(),
            35
        ));
    })
}

#[test]
fn send_property_shares_fails() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);
        // Region setup and role assignments
        assert_ok!(XcavateWhitelist::add_admin(RuntimeOrigin::root(), [20; 32].into(),));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [8; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));
        new_region_helper();
        assert_ok!(Regions::create_new_location(
            RuntimeOrigin::signed([8; 32].into()),
            3,
            bvec![10, 10]
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [0; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateDeveloper
        ));
        assert_noop!(
            Marketplace::send_property_shares(
                RuntimeOrigin::signed([1; 32].into()),
                0,
                [2; 32].into(),
                20
            ),
            BadOrigin
        );
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [1; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [30; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [31; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));

        // Property listing and purchases
        assert_ok!(Marketplace::list_property(
            RuntimeOrigin::signed([0; 32].into()),
            3,
            bvec![10, 10],
            10_000,
            100,
            bvec![22, 22],
            false
        ));
        assert_ok!(Marketplace::buy_property_shares(
            RuntimeOrigin::signed([1; 32].into()),
            0,
            40,
            1984
        ));
        assert_ok!(Marketplace::buy_property_shares(
            RuntimeOrigin::signed([30; 32].into()),
            0,
            30,
            1984
        ));
        assert_ok!(Marketplace::buy_property_shares(
            RuntimeOrigin::signed([31; 32].into()),
            0,
            30,
            1984
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [5; 32].into(),
            pallet_xcavate_whitelist::Role::SpvConfirmation
        ));
        assert_ok!(Marketplace::create_spv(RuntimeOrigin::signed([5; 32].into()), 0,));
        assert_ok!(Marketplace::claim_property_shares(RuntimeOrigin::signed([1; 32].into()), 0));
        assert_ok!(Marketplace::claim_property_shares(RuntimeOrigin::signed([30; 32].into()), 0));
        assert_ok!(Marketplace::claim_property_shares(RuntimeOrigin::signed([31; 32].into()), 0));
        assert_noop!(
            Marketplace::send_property_shares(
                RuntimeOrigin::signed([1; 32].into()),
                0,
                [2; 32].into(),
                20
            ),
            Error::<Test>::UserNotCompliant
        );
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [2; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));
        assert_ok!(XcavateWhitelist::set_permission(
            RuntimeOrigin::signed([20; 32].into()),
            [2; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor,
            pallet_xcavate_whitelist::AccessPermission::Revoked,
        ));
        assert_noop!(
            Marketplace::send_property_shares(
                RuntimeOrigin::signed([1; 32].into()),
                0,
                [2; 32].into(),
                20
            ),
            Error::<Test>::UserNotCompliant
        );
        assert_noop!(
            Marketplace::send_property_shares(
                RuntimeOrigin::signed([1; 32].into()),
                1,
                [1; 32].into(),
                20
            ),
            Error::<Test>::NoObjectFound
        );
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [10; 32].into(),
            pallet_xcavate_whitelist::Role::Lawyer
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [11; 32].into(),
            pallet_xcavate_whitelist::Role::Lawyer
        ));

        // Legal process
        assert_ok!(Regions::register_lawyer(RuntimeOrigin::signed([10; 32].into()), 3,));
        assert_ok!(Regions::register_lawyer(RuntimeOrigin::signed([11; 32].into()), 3,));
        assert_ok!(Marketplace::lawyer_claim_property(
            RuntimeOrigin::signed([10; 32].into()),
            0,
            crate::LegalProperty::RealEstateDeveloperSide,
            4_000,
        ));
        assert_ok!(Marketplace::approve_developer_lawyer(
            RuntimeOrigin::signed([0; 32].into()),
            0,
            true
        ));
        assert_ok!(Marketplace::lawyer_claim_property(
            RuntimeOrigin::signed([11; 32].into()),
            0,
            crate::LegalProperty::SpvSide,
            4_000,
        ));
        assert_ok!(Marketplace::vote_on_spv_lawyer(
            RuntimeOrigin::signed([1; 32].into()),
            0,
            crate::Vote::Yes,
            40
        ));
        assert_ok!(Marketplace::vote_on_spv_lawyer(
            RuntimeOrigin::signed([30; 32].into()),
            0,
            crate::Vote::Yes,
            30
        ));
        run_to_block(91);
        assert_ok!(Marketplace::finalize_spv_lawyer(RuntimeOrigin::signed([1; 32].into()), 0,));
        assert_ok!(Marketplace::lawyer_confirm_documents(
            RuntimeOrigin::signed([10; 32].into()),
            0,
            true,
        ));
        assert_ok!(Marketplace::lawyer_confirm_documents(
            RuntimeOrigin::signed([11; 32].into()),
            0,
            true,
        ));

        // Sending property shares failure cases
        assert_noop!(
            Marketplace::send_property_shares(
                RuntimeOrigin::signed([2; 32].into()),
                0,
                [1; 32].into(),
                5
            ),
            BadOrigin
        );
        assert_ok!(XcavateWhitelist::set_permission(
            RuntimeOrigin::signed([20; 32].into()),
            [2; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor,
            pallet_xcavate_whitelist::AccessPermission::Compliant,
        ));
        assert_noop!(
            Marketplace::send_property_shares(
                RuntimeOrigin::signed([2; 32].into()),
                0,
                [1; 32].into(),
                5
            ),
            RealWorldAssetError::<Test>::NotEnoughShares
        );
        assert_noop!(
            Marketplace::send_property_shares(
                RuntimeOrigin::signed([2; 32].into()),
                0,
                [1; 32].into(),
                30
            ),
            Error::<Test>::ExceedsMaxOwnership
        );
    })
}

#[test]
fn send_property_shares_fails_if_relist() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);
        // Region setup and role assignments
        assert_ok!(XcavateWhitelist::add_admin(RuntimeOrigin::root(), [20; 32].into(),));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [8; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));
        new_region_helper();
        assert_ok!(Regions::create_new_location(
            RuntimeOrigin::signed([8; 32].into()),
            3,
            bvec![10, 10]
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [0; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateDeveloper
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [1; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [2; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [3; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [30; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [10; 32].into(),
            pallet_xcavate_whitelist::Role::Lawyer
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [11; 32].into(),
            pallet_xcavate_whitelist::Role::Lawyer
        ));
        assert_ok!(Regions::register_lawyer(RuntimeOrigin::signed([10; 32].into()), 3,));
        assert_ok!(Regions::register_lawyer(RuntimeOrigin::signed([11; 32].into()), 3,));

        // Property listing and purchases
        assert_ok!(Marketplace::list_property(
            RuntimeOrigin::signed([0; 32].into()),
            3,
            bvec![10, 10],
            10_000,
            100,
            bvec![22, 22],
            false
        ));
        assert_ok!(Marketplace::buy_property_shares(
            RuntimeOrigin::signed([1; 32].into()),
            0,
            20,
            1984
        ));
        assert_ok!(Marketplace::buy_property_shares(
            RuntimeOrigin::signed([2; 32].into()),
            0,
            40,
            1984
        ));
        assert_ok!(Marketplace::buy_property_shares(
            RuntimeOrigin::signed([30; 32].into()),
            0,
            40,
            1984
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [5; 32].into(),
            pallet_xcavate_whitelist::Role::SpvConfirmation
        ));
        assert_ok!(Marketplace::create_spv(RuntimeOrigin::signed([5; 32].into()), 0,));
        assert_ok!(Marketplace::claim_property_shares(RuntimeOrigin::signed([1; 32].into()), 0));
        assert_ok!(Marketplace::claim_property_shares(RuntimeOrigin::signed([2; 32].into()), 0));
        assert_ok!(Marketplace::claim_property_shares(RuntimeOrigin::signed([30; 32].into()), 0));

        // Legal process
        assert_ok!(Marketplace::lawyer_claim_property(
            RuntimeOrigin::signed([10; 32].into()),
            0,
            crate::LegalProperty::RealEstateDeveloperSide,
            4_000,
        ));
        assert_ok!(Marketplace::approve_developer_lawyer(
            RuntimeOrigin::signed([0; 32].into()),
            0,
            true
        ));
        assert_ok!(Marketplace::lawyer_claim_property(
            RuntimeOrigin::signed([11; 32].into()),
            0,
            crate::LegalProperty::SpvSide,
            4_000,
        ));
        assert_ok!(Marketplace::vote_on_spv_lawyer(
            RuntimeOrigin::signed([2; 32].into()),
            0,
            crate::Vote::Yes,
            40
        ));
        assert_ok!(Marketplace::vote_on_spv_lawyer(
            RuntimeOrigin::signed([30; 32].into()),
            0,
            crate::Vote::Yes,
            40
        ));
        run_to_block(91);
        assert_ok!(Marketplace::finalize_spv_lawyer(RuntimeOrigin::signed([2; 32].into()), 0,));
        assert_ok!(Marketplace::lawyer_confirm_documents(
            RuntimeOrigin::signed([10; 32].into()),
            0,
            true,
        ));
        assert_ok!(Marketplace::lawyer_confirm_documents(
            RuntimeOrigin::signed([11; 32].into()),
            0,
            true,
        ));

        // Sending property shares failure case
        assert_eq!(PropertyOwnerShares::<Test>::get::<u32, AccountId>(0, [1; 32].into()), 20);
        assert_eq!(LocalAssets::balance(0, &[1; 32].into()), 20);
        assert_eq!(PropertyOwner::<Test>::get(0).len(), 3);
        assert_ok!(Marketplace::relist_shares(RuntimeOrigin::signed([1; 32].into()), 0, 1000, 15));
        assert_noop!(
            Marketplace::send_property_shares(
                RuntimeOrigin::signed([1; 32].into()),
                0,
                [2; 32].into(),
                9
            ),
            RealWorldAssetError::<Test>::NotEnoughShares
        );
    })
}

// withdraw_deposit_unsold tests

#[test]
fn withdraw_deposit_unsold_works() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);
        assert_ok!(XcavateWhitelist::add_admin(RuntimeOrigin::root(), [20; 32].into(),));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [8; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));
        new_region_helper();
        assert_ok!(Regions::create_new_location(
            RuntimeOrigin::signed([8; 32].into()),
            3,
            bvec![10, 10]
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [0; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateDeveloper
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [0; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));
        assert_ok!(Marketplace::list_property(
            RuntimeOrigin::signed([0; 32].into()),
            3,
            bvec![10, 10],
            10_000,
            100,
            bvec![22, 22],
            false
        ));
        assert_eq!(
            Balances::balance_on_hold(&HoldReason::ListingDepositReserve.into(), &([0; 32].into())),
            200_000
        );
        run_to_block(100);
        assert_ok!(Marketplace::withdraw_deposit_unsold(RuntimeOrigin::signed([0; 32].into()), 0));
        assert_eq!(OngoingObjectListing::<Test>::get(0), None);
        assert_eq!(pallet_nfts::Item::<Test>::get(0, 0).is_none(), true);
        assert_eq!(
            Balances::balance_on_hold(&HoldReason::ListingDepositReserve.into(), &([0; 32].into())),
            0
        );
    })
}

#[test]
fn withdraw_deposit_unsold_fails() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);
        assert_ok!(XcavateWhitelist::add_admin(RuntimeOrigin::root(), [20; 32].into(),));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [8; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));
        new_region_helper();
        assert_ok!(Regions::create_new_location(
            RuntimeOrigin::signed([8; 32].into()),
            3,
            bvec![10, 10]
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [0; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateDeveloper
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [0; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [1; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));
        assert_ok!(Marketplace::list_property(
            RuntimeOrigin::signed([0; 32].into()),
            3,
            bvec![10, 10],
            10_000,
            100,
            bvec![22, 22],
            false
        ));
        assert_eq!(
            Balances::balance_on_hold(&HoldReason::ListingDepositReserve.into(), &([0; 32].into())),
            200_000
        );
        run_to_block(20);
        assert_noop!(
            Marketplace::withdraw_deposit_unsold(RuntimeOrigin::signed([0; 32].into()), 0),
            Error::<Test>::ListingNotExpired
        );
        assert_ok!(Marketplace::buy_property_shares(
            RuntimeOrigin::signed([1; 32].into()),
            0,
            10,
            1984
        ));
        assert_noop!(
            Marketplace::withdraw_deposit_unsold(RuntimeOrigin::signed([0; 32].into()), 1),
            Error::<Test>::ListingNotFound
        );
        run_to_block(100);
        assert_noop!(
            Marketplace::withdraw_deposit_unsold(RuntimeOrigin::signed([0; 32].into()), 0),
            Error::<Test>::SharesNotReturned
        );
    })
}

#[test]
fn withdraw_deposit_unsold_fails_2() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);
        assert_ok!(XcavateWhitelist::add_admin(RuntimeOrigin::root(), [20; 32].into(),));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [8; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));
        new_region_helper();
        assert_ok!(Regions::create_new_location(
            RuntimeOrigin::signed([8; 32].into()),
            3,
            bvec![10, 10]
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [0; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateDeveloper
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [0; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [1; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [2; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [30; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));
        assert_ok!(Marketplace::list_property(
            RuntimeOrigin::signed([0; 32].into()),
            3,
            bvec![10, 10],
            10_000,
            100,
            bvec![22, 22],
            false
        ));
        assert_eq!(
            Balances::balance_on_hold(&HoldReason::ListingDepositReserve.into(), &([0; 32].into())),
            200_000
        );
        run_to_block(20);
        assert_ok!(Marketplace::buy_property_shares(
            RuntimeOrigin::signed([1; 32].into()),
            0,
            40,
            1984
        ));
        assert_ok!(Marketplace::buy_property_shares(
            RuntimeOrigin::signed([2; 32].into()),
            0,
            30,
            1984
        ));
        assert_ok!(Marketplace::buy_property_shares(
            RuntimeOrigin::signed([30; 32].into()),
            0,
            30,
            1984
        ));
        run_to_block(100);
        assert_noop!(
            Marketplace::withdraw_deposit_unsold(RuntimeOrigin::signed([0; 32].into()), 0),
            Error::<Test>::PropertyAlreadySold
        );
    })
}

// withdraw_unclaimed tests

#[test]
fn withdraw_unclaimed_works() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);
        // Region setup and role assignments
        assert_ok!(XcavateWhitelist::add_admin(RuntimeOrigin::root(), [20; 32].into(),));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [8; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));
        new_region_helper();
        assert_ok!(Regions::create_new_location(
            RuntimeOrigin::signed([8; 32].into()),
            3,
            bvec![10, 10]
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [0; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateDeveloper
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [1; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [2; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [30; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));

        // Property listing and purchases
        assert_ok!(Marketplace::list_property(
            RuntimeOrigin::signed([0; 32].into()),
            3,
            bvec![10, 10],
            10_000,
            100,
            bvec![22, 22],
            false
        ));
        assert_ok!(Marketplace::buy_property_shares(
            RuntimeOrigin::signed([1; 32].into()),
            0,
            40,
            1984
        ));
        assert_ok!(Marketplace::buy_property_shares(
            RuntimeOrigin::signed([2; 32].into()),
            0,
            30,
            1984
        ));
        assert_ok!(Marketplace::buy_property_shares(
            RuntimeOrigin::signed([30; 32].into()),
            0,
            30,
            1984
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [5; 32].into(),
            pallet_xcavate_whitelist::Role::SpvConfirmation
        ));
        assert_ok!(Marketplace::create_spv(RuntimeOrigin::signed([5; 32].into()), 0,));
        assert_ok!(Marketplace::claim_property_shares(RuntimeOrigin::signed([1; 32].into()), 0,));
        assert_ok!(Marketplace::claim_property_shares(RuntimeOrigin::signed([30; 32].into()), 0,));
        assert_eq!(OngoingObjectListing::<Test>::get(0).unwrap().listed_share_amount, 0);
        assert_eq!(OngoingObjectListing::<Test>::get(0).unwrap().unclaimed_share_amount, 30);
        let expiry = frame_system::Pallet::<Test>::block_number() + ClaimWindowTime::get() + 1;
        run_to_block(expiry);

        // Withdraw unclaimed
        assert_ok!(Marketplace::finalize_claim_window(RuntimeOrigin::signed([1; 32].into()), 0,));
        assert_eq!(ForeignAssets::balance(1984, &[2; 32].into()), 838_000);
        assert_eq!(AssetsHolder::total_balance_on_hold(1984, &[2; 32].into()), 312_000);
        assert!(ShareOwner::<Test>::get::<AccountId, u32>([2; 32].into(), 0).is_some());
        assert_ok!(Marketplace::withdraw_unclaimed(RuntimeOrigin::signed([2; 32].into()), 0,));
        assert_eq!(ForeignAssets::balance(1984, &[2; 32].into()), 1_150_000);
        assert_eq!(AssetsHolder::total_balance_on_hold(1984, &[2; 32].into()), 0);
        assert!(ShareOwner::<Test>::get::<AccountId, u32>([2; 32].into(), 0).is_none());
    })
}

#[test]
fn withdraw_unclaimed_fails() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);
        // Region setup and role assignments
        assert_ok!(XcavateWhitelist::add_admin(RuntimeOrigin::root(), [20; 32].into(),));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [8; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));
        new_region_helper();
        assert_ok!(Regions::create_new_location(
            RuntimeOrigin::signed([8; 32].into()),
            3,
            bvec![10, 10]
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [0; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateDeveloper
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [1; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [2; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [30; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));

        // Property listing and purchases
        assert_ok!(Marketplace::list_property(
            RuntimeOrigin::signed([0; 32].into()),
            3,
            bvec![10, 10],
            10_000,
            100,
            bvec![22, 22],
            false
        ));
        assert_ok!(Marketplace::buy_property_shares(
            RuntimeOrigin::signed([1; 32].into()),
            0,
            40,
            1984
        ));
        assert_ok!(Marketplace::buy_property_shares(
            RuntimeOrigin::signed([30; 32].into()),
            0,
            30,
            1984
        ));
        assert_noop!(
            Marketplace::withdraw_unclaimed(RuntimeOrigin::signed([2; 32].into()), 0,),
            Error::<Test>::ShareOwnerNotFound
        );
        assert_ok!(Marketplace::buy_property_shares(
            RuntimeOrigin::signed([2; 32].into()),
            0,
            30,
            1984
        ));
        assert_noop!(
            Marketplace::withdraw_unclaimed(RuntimeOrigin::signed([2; 32].into()), 0,),
            Error::<Test>::NoPermission
        );
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [5; 32].into(),
            pallet_xcavate_whitelist::Role::SpvConfirmation
        ));
        assert_ok!(Marketplace::create_spv(RuntimeOrigin::signed([5; 32].into()), 0,));
        assert_ok!(Marketplace::claim_property_shares(RuntimeOrigin::signed([1; 32].into()), 0,));
        let expiry = frame_system::Pallet::<Test>::block_number() + ClaimWindowTime::get() + 1;
        run_to_block(expiry);

        // Withdraw unclaimed failures
        assert_noop!(
            Marketplace::withdraw_unclaimed(RuntimeOrigin::signed([2; 32].into()), 0,),
            Error::<Test>::NoPermission
        );
        assert_ok!(Marketplace::finalize_claim_window(RuntimeOrigin::signed([1; 32].into()), 0,));
        assert_ok!(Marketplace::withdraw_unclaimed(RuntimeOrigin::signed([2; 32].into()), 0,));
        assert_noop!(
            Marketplace::withdraw_unclaimed(RuntimeOrigin::signed([2; 32].into()), 0,),
            Error::<Test>::ShareOwnerNotFound
        );
    })
}

// withdraw_claiming_expired tests

#[test]
fn withdraw_claiming_expired_works() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);
        // Region setup and role assignments
        assert_ok!(XcavateWhitelist::add_admin(RuntimeOrigin::root(), [20; 32].into(),));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [8; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));
        new_region_helper();
        assert_ok!(Regions::create_new_location(
            RuntimeOrigin::signed([8; 32].into()),
            3,
            bvec![10, 10]
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [0; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateDeveloper
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [1; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [2; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [30; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));

        // Property listing and purchases
        assert_ok!(Marketplace::list_property(
            RuntimeOrigin::signed([0; 32].into()),
            3,
            bvec![10, 10],
            10_000,
            100,
            bvec![22, 22],
            false
        ));
        assert_ok!(Marketplace::buy_property_shares(
            RuntimeOrigin::signed([1; 32].into()),
            0,
            40,
            1984
        ));
        assert_ok!(Marketplace::buy_property_shares(
            RuntimeOrigin::signed([2; 32].into()),
            0,
            30,
            1984
        ));
        assert_ok!(Marketplace::buy_property_shares(
            RuntimeOrigin::signed([30; 32].into()),
            0,
            30,
            1984
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [5; 32].into(),
            pallet_xcavate_whitelist::Role::SpvConfirmation
        ));
        assert_ok!(Marketplace::create_spv(RuntimeOrigin::signed([5; 32].into()), 0,));
        assert_ok!(Marketplace::claim_property_shares(RuntimeOrigin::signed([1; 32].into()), 0,));
        assert_ok!(Marketplace::claim_property_shares(RuntimeOrigin::signed([30; 32].into()), 0,));
        let expiry = frame_system::Pallet::<Test>::block_number() + ClaimWindowTime::get() + 1;
        run_to_block(expiry);

        // First relisting on primary market
        assert_ok!(Marketplace::finalize_claim_window(RuntimeOrigin::signed([1; 32].into()), 0,));
        assert_ok!(Marketplace::withdraw_unclaimed(RuntimeOrigin::signed([2; 32].into()), 0,));
        assert_ok!(Marketplace::buy_property_shares(
            RuntimeOrigin::signed([2; 32].into()),
            0,
            30,
            1984
        ));
        let expiry = frame_system::Pallet::<Test>::block_number() + ClaimWindowTime::get() + 1;
        run_to_block(expiry);

        // Withdraw claiming expired
        assert_ok!(Marketplace::finalize_claim_window(RuntimeOrigin::signed([1; 32].into()), 0,));
        assert_eq!(RefundClaimedShare::<Test>::get(0).unwrap(), 70);
        assert_eq!(ForeignAssets::balance(1984, &[1; 32].into()), 1_084_000);
        assert_eq!(ForeignAssets::balance(1984, &Marketplace::property_account_id(0)), 728_000);
        assert_ok!(Marketplace::withdraw_claiming_expired(
            RuntimeOrigin::signed([1; 32].into()),
            0,
        ));
        assert_ok!(Marketplace::withdraw_claiming_expired(
            RuntimeOrigin::signed([30; 32].into()),
            0,
        ));
        assert_eq!(ForeignAssets::balance(1984, &[1; 32].into()), 1_500_000);
        assert_eq!(ForeignAssets::balance(1984, &Marketplace::property_account_id(0)), 0);
        assert_eq!(PropertyAssetInfo::<Test>::get(0).is_none(), true);
        assert_eq!(OngoingObjectListing::<Test>::get(0), None);
        assert_eq!(ShareOwner::<Test>::get::<AccountId, u32>([1; 32].into(), 0), None);
        assert_eq!(pallet_nfts::Item::<Test>::get(0, 0).is_none(), true);
        assert_eq!(Balances::balance(&(Marketplace::property_account_id(0))), 0);
        assert_eq!(Nfts::owner(0, 0), None);
        assert_eq!(PropertyAssetInfo::<Test>::get(0), None);
    })
}

#[test]
fn withdraw_claiming_expired_fails() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);
        // Region setup and role assignments
        assert_ok!(XcavateWhitelist::add_admin(RuntimeOrigin::root(), [20; 32].into(),));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [8; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));
        new_region_helper();
        assert_ok!(Regions::create_new_location(
            RuntimeOrigin::signed([8; 32].into()),
            3,
            bvec![10, 10]
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [0; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateDeveloper
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [1; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [2; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [30; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));
        assert_noop!(
            Marketplace::withdraw_claiming_expired(RuntimeOrigin::signed([1; 32].into()), 0,),
            Error::<Test>::SharesNotRefunded
        );

        // Property listing and purchases
        assert_ok!(Marketplace::list_property(
            RuntimeOrigin::signed([0; 32].into()),
            3,
            bvec![10, 10],
            10_000,
            100,
            bvec![22, 22],
            false
        ));
        assert_ok!(Marketplace::buy_property_shares(
            RuntimeOrigin::signed([1; 32].into()),
            0,
            40,
            1984
        ));
        assert_ok!(Marketplace::buy_property_shares(
            RuntimeOrigin::signed([2; 32].into()),
            0,
            30,
            1984
        ));
        assert_ok!(Marketplace::buy_property_shares(
            RuntimeOrigin::signed([30; 32].into()),
            0,
            30,
            1984
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [5; 32].into(),
            pallet_xcavate_whitelist::Role::SpvConfirmation
        ));
        assert_ok!(Marketplace::create_spv(RuntimeOrigin::signed([5; 32].into()), 0,));
        assert_ok!(Marketplace::claim_property_shares(RuntimeOrigin::signed([1; 32].into()), 0,));
        assert_ok!(Marketplace::claim_property_shares(RuntimeOrigin::signed([30; 32].into()), 0,));
        let expiry = frame_system::Pallet::<Test>::block_number() + ClaimWindowTime::get() + 1;
        run_to_block(expiry);

        // First relisting on primary market
        assert_ok!(Marketplace::finalize_claim_window(RuntimeOrigin::signed([1; 32].into()), 0,));
        assert_ok!(Marketplace::withdraw_unclaimed(RuntimeOrigin::signed([2; 32].into()), 0,));
        assert_ok!(Marketplace::buy_property_shares(
            RuntimeOrigin::signed([2; 32].into()),
            0,
            30,
            1984
        ));
        let expiry = frame_system::Pallet::<Test>::block_number() + ClaimWindowTime::get() + 1;
        run_to_block(expiry);

        // Withdraw claiming expired failures
        assert_noop!(
            Marketplace::withdraw_claiming_expired(RuntimeOrigin::signed([1; 32].into()), 0,),
            Error::<Test>::SharesNotRefunded
        );
        assert_ok!(Marketplace::finalize_claim_window(RuntimeOrigin::signed([1; 32].into()), 0,));
        assert_noop!(
            Marketplace::withdraw_claiming_expired(RuntimeOrigin::signed([0; 32].into()), 0,),
            BadOrigin
        );
        assert_ok!(Marketplace::withdraw_claiming_expired(
            RuntimeOrigin::signed([1; 32].into()),
            0,
        ));
    })
}
