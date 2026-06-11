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
use crate::{ModuleInfo, NextModuleId};
use frame_support::{
    assert_noop, assert_ok,
    traits::{
        fungible::InspectHold as OtherInspectHold,
        fungibles::{Inspect, InspectHold},
    },
};
use pallet_education_regions::RegionIdentifier;
use pallet_xcavate_whitelist::RolePermission;
use sp_runtime::traits::BadOrigin;

macro_rules! bvec {
	($( $x:tt )*) => {
		vec![$( $x )*].try_into().unwrap()
	}
}

fn run_to_block(n: u64) {
    while System::block_number() < n {
        if System::block_number() > 0 {
            RealXEducation::on_finalize(System::block_number());
            System::on_finalize(System::block_number());
        }
        System::reset_events();
        System::set_block_number(System::block_number() + 1);
        System::on_initialize(System::block_number());
        RealXEducation::on_initialize(System::block_number());
    }
}

fn new_region_helper() {
    assert_ok!(XcavateWhitelist::assign_role(
        RuntimeOrigin::signed([20; 32].into()),
        [8; 32].into(),
        pallet_xcavate_whitelist::Role::RegionalOperator
    ));
    assert_ok!(EducationRegions::propose_new_region(
        RuntimeOrigin::signed([8; 32].into()),
        RegionIdentifier::Japan
    ));
    assert_ok!(EducationRegions::vote_on_region_proposal(
        RuntimeOrigin::signed([8; 32].into()),
        3,
        pallet_education_regions::Vote::Yes,
        100_000
    ));
    run_to_block(31);
    assert_ok!(EducationRegions::bid_on_region(RuntimeOrigin::signed([8; 32].into()), 3, 100_000));
    run_to_block(61);
    assert_ok!(EducationRegions::create_new_region(RuntimeOrigin::signed([8; 32].into()), 3,));
}

/// Sets up admin [20] and assigns all five standard roles:
/// creator=[1], sponsor=[2], school=[3], lecturer=[4], ai_agent=[5].
/// Also creates the test region.
fn setup_all_roles() {
    assert_ok!(XcavateWhitelist::add_admin(RuntimeOrigin::root(), [20; 32].into()));
    assert_ok!(XcavateWhitelist::assign_role(
        RuntimeOrigin::signed([20; 32].into()),
        [1; 32].into(),
        pallet_xcavate_whitelist::Role::ModuleCreator
    ));
    assert_ok!(XcavateWhitelist::assign_role(
        RuntimeOrigin::signed([20; 32].into()),
        [2; 32].into(),
        pallet_xcavate_whitelist::Role::ModuleSponsor
    ));
    assert_ok!(XcavateWhitelist::assign_role(
        RuntimeOrigin::signed([20; 32].into()),
        [3; 32].into(),
        pallet_xcavate_whitelist::Role::ModuleBooker
    ));
    assert_ok!(XcavateWhitelist::assign_role(
        RuntimeOrigin::signed([20; 32].into()),
        [4; 32].into(),
        pallet_xcavate_whitelist::Role::ModuleDeliverer
    ));
    assert_ok!(XcavateWhitelist::assign_role(
        RuntimeOrigin::signed([20; 32].into()),
        [5; 32].into(),
        pallet_xcavate_whitelist::Role::ModuleAIAgent
    ));
    new_region_helper();
}

/// Runs the full booking flow: register lecturer, create module (100 tokens),
/// sponsor 30 tokens with USDT (asset 1984), book one token, and claim it.
///
/// Assumes `setup_all_roles()` has already been called.
/// Returns the asset ID of the created module's fractional token.
fn setup_and_claim_booking() -> u32 {
    assert_ok!(RealXEducation::register_module_deliverer(RuntimeOrigin::signed([4; 32].into())));
    assert_ok!(RealXEducation::create_module(
        RuntimeOrigin::signed([1; 32].into()),
        3,
        100,
        bvec![1, 2, 3]
    ));
    let asset_id = ModuleInfo::<Test>::get(0).unwrap().asset_id;
    assert_ok!(RealXEducation::sponsor_module(RuntimeOrigin::signed([2; 32].into()), 0, 30, 1984));
    assert_ok!(RealXEducation::book_module(
        RuntimeOrigin::signed([3; 32].into()),
        0,
        0,
        bvec![4, 5, 6]
    ));
    assert_ok!(RealXEducation::claim_booking(RuntimeOrigin::signed([4; 32].into()), 0, 0));
    asset_id
}

// create_module tests

#[test]
fn create_module_works() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);
        let module_amount: u32 = 100;

        // Step 1: Setup permissions
        assert_ok!(XcavateWhitelist::add_admin(RuntimeOrigin::root(), [20; 32].into(),));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [1; 32].into(),
            pallet_xcavate_whitelist::Role::ModuleCreator
        ));
        new_region_helper();

        // Pre-check: ensure no module exists yet
        assert!(!ModuleInfo::<Test>::contains_key(0));
        assert_eq!(NextModuleId::<Test>::get(), 0);

        //  Step 2: Execute the extrinsic
        assert_ok!(RealXEducation::create_module(
            RuntimeOrigin::signed([1; 32].into()),
            3,
            module_amount,
            bvec![22, 22]
        ));

        // 1. Module was created with correct data
        let module = ModuleInfo::<Test>::get(0).expect("Module should exist");

        assert_eq!(module.creator, [1; 32].into());
        assert_eq!(module.total_token_amount, module_amount);
        assert_eq!(module.sponsor_allocation, module_amount);
        assert_eq!(module.school_allocation, 0);
        assert_eq!(module.university_student_allocation, 0);
        assert_eq!(module.asset_id, 0);

        // 2. NFT Collection & Item were created
        let collection_id = module.collection_id;
        let item_id = module.item_id;

        assert!(EducationNfts::collection_owner(collection_id).is_some());
        assert!(EducationNfts::owner(collection_id, item_id) == Some([1; 32].into()));

        // 3. Fractionalization happened correctly
        let fractional_asset_id = module.asset_id.into();

        // Creator received all fractional tokens
        assert_eq!(
            EducationAssets::balance(fractional_asset_id, &[1; 32].into()),
            module_amount.into()
        );

        // Total issuance matches
        assert_eq!(EducationAssets::total_issuance(fractional_asset_id), module_amount.into());

        // 4. Counters were incremented correctly
        assert_eq!(NextModuleId::<Test>::get(), 1);
        assert_eq!(NextAssetId::<Test>::get(), 1);
        assert_eq!(NextNftId::<Test>::get(collection_id), item_id + 1);

        // 5. Event was emitted
        System::assert_last_event(
            Event::LearningModuleCreated {
                creator: [1; 32].into(),
                module_id: 0,
                collection_id,
                item_id,
                token_amount: module_amount,
                metadata_blob: bvec![22, 22],
                created_at: 61,
            }
            .into(),
        );
    });
}

#[test]
fn create_module_fails() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);

        // No permission fails
        assert_noop!(
            RealXEducation::create_module(
                RuntimeOrigin::signed([1; 32].into()),
                3,
                <Test as Config>::MaxModuleToken::get(),
                bvec![22, 22]
            ),
            BadOrigin
        );

        assert_ok!(XcavateWhitelist::add_admin(RuntimeOrigin::root(), [20; 32].into(),));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [1; 32].into(),
            pallet_xcavate_whitelist::Role::ModuleCreator
        ));
        new_region_helper();

        // Exceeding max token fails
        assert_noop!(
            RealXEducation::create_module(
                RuntimeOrigin::signed([1; 32].into()),
                3,
                <Test as Config>::MaxModuleToken::get() + 1,
                bvec![22, 22]
            ),
            Error::<Test>::TooManyToken
        );

        // Creating 0 modules fails
        assert_noop!(
            RealXEducation::create_module(
                RuntimeOrigin::signed([1; 32].into()),
                3,
                0,
                bvec![22, 22]
            ),
            Error::<Test>::AmountCannotBeZero
        );

        // Creating content with max token works
        assert_ok!(RealXEducation::create_module(
            RuntimeOrigin::signed([1; 32].into()),
            3,
            <Test as Config>::MaxModuleToken::get(),
            bvec![22, 22]
        ));
    });
}

// create_module tests

#[test]
fn sponsor_module_works() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);
        let module_amount = 100u32;
        let purchase_amount = 30u32;

        // Step 1: Setup permissions
        assert_ok!(XcavateWhitelist::add_admin(RuntimeOrigin::root(), [20; 32].into(),));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [1; 32].into(),
            pallet_xcavate_whitelist::Role::ModuleCreator
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [2; 32].into(),
            pallet_xcavate_whitelist::Role::ModuleSponsor
        ));
        new_region_helper();

        // Step 2: Creator creates a module
        assert_ok!(RealXEducation::create_module(
            RuntimeOrigin::signed([1; 32].into()),
            3,
            module_amount,
            bvec![1, 2, 3]
        ));

        let module = ModuleInfo::<Test>::get(0).unwrap();
        let asset_id = module.asset_id.into();

        // Pre-check: creator owns all 100 tokens
        assert_eq!(EducationAssets::balance(asset_id, &[1; 32].into()), 100);
        assert_eq!(EducationAssets::balance(asset_id, &[2; 32].into()), 0);
        assert_eq!(NextSponsorId::<Test>::get(), 0);

        // Step 3: Sponsor purchases 30 tokens
        assert_ok!(RealXEducation::sponsor_module(
            RuntimeOrigin::signed([2; 32].into()),
            0,
            purchase_amount,
            1984
        ));

        // 1. Funds were held correctly
        let price_per_token = 1250;
        let multiplier = 10u128
            .checked_pow(AssetsMetadataWrapper::get_decimals(1984u32).unwrap().into())
            .unwrap();
        let expected_hold = price_per_token * purchase_amount as u128 * multiplier;

        let held = AssetsHolder::total_balance_on_hold(1984u32.into(), &[2; 32].into());
        assert_eq!(held, expected_hold);

        // 2. Fractional tokens transferred
        let updated_module = ModuleInfo::<Test>::get(0).unwrap();
        assert_eq!(updated_module.sponsor_allocation, 70);
        assert_eq!(updated_module.school_allocation, 30);

        // 3. SponsoredModules tracking works
        assert_eq!(NextSponsorId::<Test>::get(), 1);
        assert_eq!(
            SponsoredModules::<Test>::get(0, 0).unwrap(),
            SponsoredModulesDetails {
                sponsor: [2; 32].into(),
                amount: purchase_amount,
                payment_asset: 1984,
                sponsored_at: 61,
            }
        );

        // 4. Event emitted
        System::assert_last_event(
            Event::ModuleSponsored {
                module_id: 0,
                sponsor_id: 0,
                sponsor: [2; 32].into(),
                module_amount: purchase_amount,
                sponsored_at: 61,
            }
            .into(),
        );
    });
}

#[test]
fn sponsor_module_works_different_asset() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);
        let module_amount = 100u32;
        let purchase_amount = 30u32;

        // Step 1: Setup permissions
        assert_ok!(XcavateWhitelist::add_admin(RuntimeOrigin::root(), [20; 32].into(),));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [1; 32].into(),
            pallet_xcavate_whitelist::Role::ModuleCreator
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [2; 32].into(),
            pallet_xcavate_whitelist::Role::ModuleSponsor
        ));
        new_region_helper();

        // Step 2: Creator creates a module
        assert_ok!(RealXEducation::create_module(
            RuntimeOrigin::signed([1; 32].into()),
            3,
            module_amount,
            bvec![1, 2, 3]
        ));

        let module = ModuleInfo::<Test>::get(0).unwrap();
        let asset_id = module.asset_id.into();

        // Pre-check: creator owns all 100 tokens
        assert_eq!(EducationAssets::balance(asset_id, &[1; 32].into()), 100);
        assert_eq!(EducationAssets::balance(asset_id, &[2; 32].into()), 0);
        assert_eq!(NextSponsorId::<Test>::get(), 0);

        // Step 3: Sponsor purchases 30 tokens
        assert_ok!(RealXEducation::sponsor_module(
            RuntimeOrigin::signed([2; 32].into()),
            0,
            purchase_amount,
            10
        ));

        // 1. Funds were held correctly
        let price_per_token = 1250;
        let multiplier =
            10u128.checked_pow(AssetsMetadataWrapper::get_decimals(10).unwrap().into()).unwrap(); // ttGBP has 6 decimals
        let expected_hold = price_per_token * purchase_amount as u128 * multiplier;

        let held = AssetsHolder::total_balance_on_hold(10u32.into(), &[2; 32].into());
        assert_eq!(held, expected_hold);

        // 2. Fractional tokens transferred
        let updated_module = ModuleInfo::<Test>::get(0).unwrap();
        assert_eq!(updated_module.sponsor_allocation, 70);
        assert_eq!(updated_module.school_allocation, 30);

        // 3. SponsoredModules tracking works
        assert_eq!(NextSponsorId::<Test>::get(), 1);
        assert_eq!(
            SponsoredModules::<Test>::get(0, 0).unwrap(),
            SponsoredModulesDetails {
                sponsor: [2; 32].into(),
                amount: purchase_amount,
                payment_asset: 10,
                sponsored_at: 61,
            }
        );

        // 4. Event emitted
        System::assert_last_event(
            Event::ModuleSponsored {
                module_id: 0,
                sponsor_id: 0,
                sponsor: [2; 32].into(),
                module_amount: purchase_amount,
                sponsored_at: 61,
            }
            .into(),
        );
    });
}

#[test]
fn sponsor_module_multiple_times_works() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);
        let module_amount = 100u32;
        let purchase_amount = 15u32;

        // Step 1: Setup permissions
        assert_ok!(XcavateWhitelist::add_admin(RuntimeOrigin::root(), [20; 32].into(),));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [1; 32].into(),
            pallet_xcavate_whitelist::Role::ModuleCreator
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [2; 32].into(),
            pallet_xcavate_whitelist::Role::ModuleSponsor
        ));
        new_region_helper();

        // Step 2: Creator creates a module
        assert_ok!(RealXEducation::create_module(
            RuntimeOrigin::signed([1; 32].into()),
            3,
            module_amount,
            bvec![1, 2, 3]
        ));

        let module = ModuleInfo::<Test>::get(0).unwrap();
        let asset_id = module.asset_id.into();

        // Pre-check: creator owns all 100 tokens
        assert_eq!(EducationAssets::balance(asset_id, &[1; 32].into()), 100);
        assert_eq!(EducationAssets::balance(asset_id, &[2; 32].into()), 0);
        assert_eq!(NextSponsorId::<Test>::get(), 0);

        // Step 3: Sponsor purchases 15 tokens
        assert_ok!(RealXEducation::sponsor_module(
            RuntimeOrigin::signed([2; 32].into()),
            0,
            purchase_amount,
            1984
        ));

        // Move forward
        let block_number = frame_system::Pallet::<Test>::block_number() + 10;
        run_to_block(block_number);

        // Step 4: Sponsor purchases 15 tokens again
        assert_ok!(RealXEducation::sponsor_module(
            RuntimeOrigin::signed([2; 32].into()),
            0,
            purchase_amount,
            1984
        ));

        // 1. Funds were held correctly
        let price_per_token = 1250;
        let multiplier = 10u128
            .checked_pow(AssetsMetadataWrapper::get_decimals(1984u32).unwrap().into())
            .unwrap();
        let expected_hold = price_per_token * purchase_amount as u128 * 2 * multiplier;

        let held = AssetsHolder::total_balance_on_hold(1984u32.into(), &[2; 32].into());
        assert_eq!(held, expected_hold);

        // 2. Fractional tokens transferred
        let updated_module = ModuleInfo::<Test>::get(0).unwrap();
        assert_eq!(updated_module.sponsor_allocation, 70);
        assert_eq!(updated_module.school_allocation, 30);

        // 3. SponsoredModules tracking works
        assert_eq!(NextSponsorId::<Test>::get(), 2);
        assert_eq!(
            SponsoredModules::<Test>::get(0, 0).unwrap(),
            SponsoredModulesDetails {
                sponsor: [2; 32].into(),
                amount: purchase_amount,
                payment_asset: 1984,
                sponsored_at: 61,
            }
        );
        assert_eq!(
            SponsoredModules::<Test>::get(0, 1).unwrap(),
            SponsoredModulesDetails {
                sponsor: [2; 32].into(),
                amount: purchase_amount,
                payment_asset: 1984,
                sponsored_at: 71,
            }
        );

        // 4. Event emitted
        System::assert_last_event(
            Event::ModuleSponsored {
                module_id: 0,
                sponsor_id: 1,
                sponsor: [2; 32].into(),
                module_amount: purchase_amount,
                sponsored_at: 71,
            }
            .into(),
        );
    });
}

#[test]
fn sponsor_module_fails() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);

        // No Permission fails
        assert_noop!(
            RealXEducation::sponsor_module(RuntimeOrigin::signed([2; 32].into()), 0, 0, 1984),
            BadOrigin
        );

        assert_ok!(XcavateWhitelist::add_admin(RuntimeOrigin::root(), [20; 32].into(),));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [1; 32].into(),
            pallet_xcavate_whitelist::Role::ModuleCreator
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [2; 32].into(),
            pallet_xcavate_whitelist::Role::ModuleSponsor
        ));
        new_region_helper();

        // Try to purchase from a non extisting module
        assert_noop!(
            RealXEducation::sponsor_module(RuntimeOrigin::signed([2; 32].into()), 0, 30, 1984),
            Error::<Test>::ModuleNotAvailable
        );

        // Creator creates a module
        assert_ok!(RealXEducation::create_module(
            RuntimeOrigin::signed([1; 32].into()),
            3,
            100,
            bvec![1, 2, 3]
        ));

        // Try to purchase 0 modules
        assert_noop!(
            RealXEducation::sponsor_module(RuntimeOrigin::signed([2; 32].into()), 0, 0, 1984),
            Error::<Test>::AmountCannotBeZero
        );

        // Try to purchase too many modules
        assert_noop!(
            RealXEducation::sponsor_module(RuntimeOrigin::signed([2; 32].into()), 0, 101, 1984),
            Error::<Test>::NotEnoughTokenAvailable
        );
    });
}

// book_module tests

#[test]
fn book_module_works() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);
        let module_amount = 100u32;
        let purchase_amount = 30u32;

        // Step 1: Setup permissions
        assert_ok!(XcavateWhitelist::add_admin(RuntimeOrigin::root(), [20; 32].into(),));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [1; 32].into(),
            pallet_xcavate_whitelist::Role::ModuleCreator
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [2; 32].into(),
            pallet_xcavate_whitelist::Role::ModuleSponsor
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [3; 32].into(),
            pallet_xcavate_whitelist::Role::ModuleBooker
        ));
        new_region_helper();

        // Step 2: Creator creates a module
        assert_ok!(RealXEducation::create_module(
            RuntimeOrigin::signed([1; 32].into()),
            3,
            module_amount,
            bvec![1, 2, 3]
        ));

        let module = ModuleInfo::<Test>::get(0).unwrap();
        let asset_id = module.asset_id.into();

        // Step 3: Sponsor purchases 30 tokens
        assert_ok!(RealXEducation::sponsor_module(
            RuntimeOrigin::signed([2; 32].into()),
            0,
            purchase_amount,
            1984
        ));

        // Pre-check: sponsor owns 30 tokens
        assert_eq!(EducationAssets::balance(asset_id, &[1; 32].into()), 70);
        assert_eq!(EducationAssets::balance(asset_id, &[2; 32].into()), 30);
        assert_eq!(EducationAssets::balance(asset_id, &[3; 32].into()), 0);
        assert_eq!(SponsoredModules::<Test>::get(0, 0).unwrap().amount, 30);

        // Step 4: School books a module
        assert_ok!(RealXEducation::book_module(
            RuntimeOrigin::signed([3; 32].into()),
            0,
            0,
            bvec![4, 5, 6]
        ));

        // 1. Booking was created with correct data
        let booking = Bookings::<Test>::get::<u32, u64>(0, 0).expect("Booking should exist");
        assert_eq!(booking.sponsor, [2; 32].into());
        assert_eq!(booking.school, [3; 32].into());
        assert_eq!(booking.lecturer, None);
        assert_eq!(booking.booked_at, 61);

        // 2. Fractional tokens transferred
        let updated_module = ModuleInfo::<Test>::get(0).unwrap();
        assert_eq!(updated_module.school_allocation, 29);
        assert_eq!(updated_module.university_student_allocation, 1);
        assert_eq!(EducationAssets::balance(asset_id, &[1; 32].into()), 70);
        assert_eq!(EducationAssets::balance(asset_id, &[2; 32].into()), 29);
        assert_eq!(EducationAssets::balance(asset_id, &[3; 32].into()), 1);

        // 3. Booking deposit held
        assert_eq!(Balances::free_balance(&([3; 32].into())), 4_990);
        assert_eq!(
            Balances::balance_on_hold(&HoldReason::BookingReserve.into(), &[3; 32].into()),
            10
        );

        // 4. SponsoredModules tracking works
        assert_eq!(SponsoredModules::<Test>::get(0, 0).unwrap().amount, purchase_amount - 1);

        // 5. Booking ID incremented
        assert_eq!(NextBookingId::<Test>::get(), 1);

        // 6. Event emitted
        System::assert_last_event(
            Event::ModuleBooked {
                module_id: 0,
                sponsor_id: 0,
                booking_id: 0,
                sponsor: [2; 32].into(),
                school: [3; 32].into(),
                booked_at: 61,
            }
            .into(),
        );
    });
}

#[test]
fn book_module_fails() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);
        // No Permission fails
        assert_noop!(
            RealXEducation::book_module(
                RuntimeOrigin::signed([3; 32].into()),
                0,
                0,
                bvec![4, 5, 6]
            ),
            BadOrigin
        );

        assert_ok!(XcavateWhitelist::add_admin(RuntimeOrigin::root(), [20; 32].into(),));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [1; 32].into(),
            pallet_xcavate_whitelist::Role::ModuleCreator
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [2; 32].into(),
            pallet_xcavate_whitelist::Role::ModuleSponsor
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [3; 32].into(),
            pallet_xcavate_whitelist::Role::ModuleBooker
        ));
        new_region_helper();

        assert_noop!(
            RealXEducation::book_module(
                RuntimeOrigin::signed([3; 32].into()),
                0,
                0,
                bvec![4, 5, 6]
            ),
            Error::<Test>::ModuleNotAvailable
        );

        assert_ok!(RealXEducation::create_module(
            RuntimeOrigin::signed([1; 32].into()),
            3,
            100u32,
            bvec![1, 2, 3]
        ));

        assert_noop!(
            RealXEducation::book_module(
                RuntimeOrigin::signed([3; 32].into()),
                0,
                0,
                bvec![4, 5, 6]
            ),
            Error::<Test>::NotEnoughTokenAvailable
        );
    });
}

// claim_booking tests

#[test]
fn claim_booking_works() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);
        let module_amount = 100u32;
        let purchase_amount = 30u32;

        // Step 1: Setup permissions
        assert_ok!(XcavateWhitelist::add_admin(RuntimeOrigin::root(), [20; 32].into(),));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [1; 32].into(),
            pallet_xcavate_whitelist::Role::ModuleCreator
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [2; 32].into(),
            pallet_xcavate_whitelist::Role::ModuleSponsor
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [3; 32].into(),
            pallet_xcavate_whitelist::Role::ModuleBooker
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [4; 32].into(),
            pallet_xcavate_whitelist::Role::ModuleDeliverer
        ));
        new_region_helper();
        assert_ok!(RealXEducation::register_module_deliverer(RuntimeOrigin::signed(
            [4; 32].into()
        )));

        // Step 2: Creator creates a module
        assert_ok!(RealXEducation::create_module(
            RuntimeOrigin::signed([1; 32].into()),
            3,
            module_amount,
            bvec![1, 2, 3]
        ));

        let module = ModuleInfo::<Test>::get(0).unwrap();
        let asset_id = module.asset_id.into();

        // Step 3: Sponsor purchases 30 tokens
        assert_ok!(RealXEducation::sponsor_module(
            RuntimeOrigin::signed([2; 32].into()),
            0,
            purchase_amount,
            1984
        ));

        // Step 4: School books a module
        assert_ok!(RealXEducation::book_module(
            RuntimeOrigin::signed([3; 32].into()),
            0,
            0,
            bvec![4, 5, 6]
        ));

        // Pre-checks
        assert_eq!(EducationAssets::balance(asset_id, &[3; 32].into()), 1);
        assert_eq!(EducationAssets::balance(asset_id, &[4; 32].into()), 0);
        assert_eq!(Bookings::<Test>::get::<u32, u64>(0, 0).unwrap().lecturer, None);

        // Step 5: University student claims a module
        assert_ok!(RealXEducation::claim_booking(RuntimeOrigin::signed([4; 32].into()), 0, 0,));

        // 1. Lecturer was set
        assert_eq!(Bookings::<Test>::get::<u32, u64>(0, 0).unwrap().lecturer, Some([4; 32].into()));

        // 2. Fractional tokens transferred
        let updated_module = ModuleInfo::<Test>::get(0).unwrap();
        assert_eq!(updated_module.university_student_allocation, 0);
        assert_eq!(EducationAssets::balance(asset_id, &[3; 32].into()), 1);
        assert_eq!(EducationAssets::balance(asset_id, &[4; 32].into()), 0);

        // 3. Event emitted
        System::assert_last_event(
            Event::BookingClaimed {
                module_id: 0,
                booking_id: 0,
                lecturer: [4; 32].into(),
                claimed_at: 61,
            }
            .into(),
        );
    });
}

#[test]
fn claim_booking_fails() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);
        // No Permission fails
        assert_noop!(
            RealXEducation::claim_booking(RuntimeOrigin::signed([4; 32].into()), 0, 0,),
            BadOrigin
        );

        assert_ok!(XcavateWhitelist::add_admin(RuntimeOrigin::root(), [20; 32].into(),));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [1; 32].into(),
            pallet_xcavate_whitelist::Role::ModuleCreator
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [2; 32].into(),
            pallet_xcavate_whitelist::Role::ModuleSponsor
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [3; 32].into(),
            pallet_xcavate_whitelist::Role::ModuleBooker
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [3; 32].into(),
            pallet_xcavate_whitelist::Role::ModuleDeliverer
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [4; 32].into(),
            pallet_xcavate_whitelist::Role::ModuleDeliverer
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [5; 32].into(),
            pallet_xcavate_whitelist::Role::ModuleDeliverer
        ));
        new_region_helper();

        assert_ok!(RealXEducation::create_module(
            RuntimeOrigin::signed([1; 32].into()),
            3,
            100,
            bvec![1, 2, 3]
        ));

        assert_ok!(RealXEducation::sponsor_module(
            RuntimeOrigin::signed([2; 32].into()),
            0,
            30,
            1984
        ));

        // Fails if there are no booked modules
        assert_noop!(
            RealXEducation::claim_booking(RuntimeOrigin::signed([4; 32].into()), 0, 0,),
            Error::<Test>::NoBookingAvailable
        );

        assert_ok!(RealXEducation::book_module(
            RuntimeOrigin::signed([3; 32].into()),
            0,
            0,
            bvec![4, 5, 6]
        ));

        // Fails if the module deliver is not registered
        assert_noop!(
            RealXEducation::claim_booking(RuntimeOrigin::signed([4; 32].into()), 0, 0,),
            Error::<Test>::ModuleDelivererNotRegistered
        );

        assert_ok!(RealXEducation::register_module_deliverer(RuntimeOrigin::signed(
            [3; 32].into()
        )));
        assert_ok!(RealXEducation::register_module_deliverer(RuntimeOrigin::signed(
            [4; 32].into()
        )));

        // Fails if the module deliverer is already the module booker of the booking
        assert_noop!(
            RealXEducation::claim_booking(RuntimeOrigin::signed([3; 32].into()), 0, 0,),
            Error::<Test>::SchoolCannotClaimOwnBooking
        );

        assert_ok!(RealXEducation::register_module_deliverer(RuntimeOrigin::signed(
            [5; 32].into()
        )));

        for _ in 0..12 {
            assert_ok!(RealXEducation::claim_booking(RuntimeOrigin::signed([5; 32].into()), 0, 0,));

            assert_ok!(RealXEducation::cancel_claim(RuntimeOrigin::signed([5; 32].into()), 0, 0,));
        }
        assert_eq!(ModuleDeliverer::<Test>::get::<&AccountId>(&[5; 32].into()).unwrap().deposit, 0);

        // Fails if the module deliverer does not have enough deposit to claim
        assert_noop!(
            RealXEducation::claim_booking(RuntimeOrigin::signed([5; 32].into()), 0, 0,),
            Error::<Test>::InsufficientDepositToClaim
        );

        assert_ok!(RealXEducation::claim_booking(RuntimeOrigin::signed([4; 32].into()), 0, 0,));

        // Fails if the booked module has already been claimed by another lecturer
        assert_noop!(
            RealXEducation::claim_booking(RuntimeOrigin::signed([5; 32].into()), 0, 0,),
            Error::<Test>::LecturerAlreadySet
        );
    });
}

#[test]
fn submit_impact_score_works() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);
        setup_all_roles();
        let asset_id: u32 = setup_and_claim_booking();

        let creator: AccountId = [1; 32].into();
        let sponsor: AccountId = [2; 32].into();
        let school: AccountId = [3; 32].into();
        let lecturer: AccountId = [4; 32].into();
        let ai_agent: AccountId = [5; 32].into();

        let multiplier = 10u128
            .checked_pow(AssetsMetadataWrapper::get_decimals(1984u32).unwrap().into())
            .unwrap();

        // Pre-checks
        assert_eq!(EducationAssets::balance(asset_id.into(), &school), 1);
        assert_eq!(EducationAssets::total_issuance(asset_id.into()), 100u128);
        assert_eq!(
            AssetsHolder::total_balance_on_hold(1984u32.into(), &sponsor),
            37_500 * multiplier
        );

        // Submit test results
        assert_ok!(RealXEducation::submit_impact_score(
            RuntimeOrigin::signed(ai_agent),
            0,
            0,
            Permill::from_percent(75),
            bvec![20, 20, 20],
            bvec![21, 21, 21],
            bvec![22, 22, 22]
        ));

        // 1. Token burned
        assert_eq!(EducationAssets::balance(asset_id.into(), &lecturer), 0);
        assert_eq!(EducationAssets::total_issuance(asset_id.into()), 99u128);

        // 2. Funds released and distributed
        let creator_pay = 6225u128;
        let protocol_pay = 3750u128;
        let regional_operator_pay = 6225u128;
        let dbs_pay = 2550u128;
        let lecturer_pay = 750u128 * multiplier;

        assert_eq!(ForeignAssets::balance(1984, &creator), creator_pay);
        assert_eq!(ForeignAssets::balance(1984, &lecturer), lecturer_pay + dbs_pay);
        assert_eq!(ForeignAssets::balance(1984, &[8; 32].into()), regional_operator_pay);
        assert_eq!(
            ForeignAssets::balance(1984, &RealXEducation::treasury_account_id()),
            protocol_pay
        );
        assert_eq!(
            AssetsHolder::total_balance_on_hold(1984u32.into(), &sponsor),
            36_250 * multiplier
        );

        // 3. NFTs minted
        assert_eq!(NextNftId::<Test>::get(0), 4);

        // 4. Event
        System::assert_last_event(
            Event::TestResultsSubmitted {
                module_id: 0,
                booking_id: 0,
                lecturer,
                score: Permill::from_percent(75),
                lecturer_pay: lecturer_pay + dbs_pay,
            }
            .into(),
        );
    });
}

#[test]
fn submit_impact_score_100_works() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);
        setup_all_roles();
        let asset_id: u32 = setup_and_claim_booking();

        let creator: AccountId = [1; 32].into();
        let sponsor: AccountId = [2; 32].into();
        let lecturer: AccountId = [4; 32].into();
        let ai_agent: AccountId = [5; 32].into();

        let multiplier = 10u128
            .checked_pow(AssetsMetadataWrapper::get_decimals(1984u32).unwrap().into())
            .unwrap();

        // Pre-checks
        assert_eq!(EducationAssets::total_issuance(asset_id.into()), 100u128);
        assert_eq!(
            AssetsHolder::total_balance_on_hold(1984u32.into(), &sponsor),
            37_500 * multiplier
        );

        // Submit test results at 100% score
        assert_ok!(RealXEducation::submit_impact_score(
            RuntimeOrigin::signed(ai_agent),
            0,
            0,
            Permill::from_percent(100),
            bvec![20, 20, 20],
            bvec![21, 21, 21],
            bvec![22, 22, 22]
        ));

        // 1. Token burned
        assert_eq!(EducationAssets::balance(asset_id.into(), &lecturer), 0);
        assert_eq!(EducationAssets::total_issuance(asset_id.into()), 99u128);

        // 2. Funds released and distributed
        let creator_pay = 83u128 * multiplier;
        let protocol_pay = 50u128 * multiplier;
        let regional_operator_pay = 83u128 * multiplier;
        let dbs_pay = 34u128 * multiplier;
        let lecturer_pay = 1000u128 * multiplier;

        assert_eq!(ForeignAssets::balance(1984, &creator), creator_pay);
        assert_eq!(ForeignAssets::balance(1984, &lecturer), lecturer_pay + dbs_pay);
        assert_eq!(ForeignAssets::balance(1984, &[8; 32].into()), regional_operator_pay);
        assert_eq!(
            ForeignAssets::balance(1984, &RealXEducation::treasury_account_id()),
            protocol_pay
        );
        assert_eq!(ForeignAssets::balance(1984, &sponsor), 22_500u128 * multiplier);
        assert_eq!(
            AssetsHolder::total_balance_on_hold(1984u32.into(), &sponsor),
            36_250 * multiplier
        );

        // 3. NFTs minted
        assert_eq!(NextNftId::<Test>::get(0), 4);

        // 4. Event
        System::assert_last_event(
            Event::TestResultsSubmitted {
                module_id: 0,
                booking_id: 0,
                lecturer,
                score: Permill::from_percent(100),
                lecturer_pay: lecturer_pay + dbs_pay,
            }
            .into(),
        );
    });
}

#[test]
fn submit_impact_score_below_50_works() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);
        setup_all_roles();
        let asset_id: u32 = setup_and_claim_booking();

        let creator: AccountId = [1; 32].into();
        let sponsor: AccountId = [2; 32].into();
        let lecturer: AccountId = [4; 32].into();
        let ai_agent: AccountId = [5; 32].into();

        let multiplier = 10u128
            .checked_pow(AssetsMetadataWrapper::get_decimals(1984u32).unwrap().into())
            .unwrap();

        // Pre-checks
        assert_eq!(EducationAssets::total_issuance(asset_id.into()), 100u128);
        assert_eq!(
            AssetsHolder::total_balance_on_hold(1984u32.into(), &sponsor),
            37_500 * multiplier
        );

        // Submit test results at 40% (below 50% threshold — no payments)
        assert_ok!(RealXEducation::submit_impact_score(
            RuntimeOrigin::signed(ai_agent),
            0,
            0,
            Permill::from_percent(40),
            bvec![20, 20, 20],
            bvec![21, 21, 21],
            bvec![22, 22, 22]
        ));

        // 1. Token burned
        assert_eq!(EducationAssets::balance(asset_id.into(), &lecturer), 0);
        assert_eq!(EducationAssets::total_issuance(asset_id.into()), 99u128);

        // 2. Funds released but not distributed (score below threshold)
        assert_eq!(ForeignAssets::balance(1984, &creator), 0);
        assert_eq!(ForeignAssets::balance(1984, &lecturer), 0);
        assert_eq!(ForeignAssets::balance(1984, &RealXEducation::treasury_account_id()), 0);
        assert_eq!(ForeignAssets::balance(1984, &sponsor), 23_750u128 * multiplier);
        assert_eq!(
            AssetsHolder::total_balance_on_hold(1984u32.into(), &sponsor),
            36_250 * multiplier
        );

        // 3. NFTs minted
        assert_eq!(NextNftId::<Test>::get(0), 4);

        // 4. Event
        System::assert_last_event(
            Event::TestResultsSubmitted {
                module_id: 0,
                booking_id: 0,
                lecturer,
                score: Permill::from_percent(40),
                lecturer_pay: 0,
            }
            .into(),
        );
    });
}

#[test]
fn submit_impact_score_reduce_strikes_works() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);

        let creator: AccountId = [1; 32].into();
        let sponsor: AccountId = [2; 32].into();
        let school: AccountId = [3; 32].into();
        let lecturer: AccountId = [4; 32].into();
        let ai_agent: AccountId = [5; 32].into();

        let module_amount = 100u32;
        let purchase_amount = 30u32;

        // Step 1: Setup permissions
        assert_ok!(XcavateWhitelist::add_admin(RuntimeOrigin::root(), [20; 32].into(),));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [1; 32].into(),
            pallet_xcavate_whitelist::Role::ModuleCreator
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [2; 32].into(),
            pallet_xcavate_whitelist::Role::ModuleSponsor
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [3; 32].into(),
            pallet_xcavate_whitelist::Role::ModuleBooker
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [4; 32].into(),
            pallet_xcavate_whitelist::Role::ModuleDeliverer
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [5; 32].into(),
            pallet_xcavate_whitelist::Role::ModuleAIAgent
        ));
        new_region_helper();
        assert_ok!(RealXEducation::register_module_deliverer(RuntimeOrigin::signed(
            [4; 32].into()
        )));

        // Step 2: Creator creates a module
        assert_ok!(RealXEducation::create_module(
            RuntimeOrigin::signed(creator.clone()),
            3,
            module_amount,
            bvec![1, 2, 3]
        ));

        let module = ModuleInfo::<Test>::get(0).unwrap();
        let asset_id = module.asset_id.into();

        // Step 3: Sponsor purchases 30 tokens
        assert_ok!(RealXEducation::sponsor_module(
            RuntimeOrigin::signed(sponsor.clone()),
            0,
            purchase_amount,
            1984
        ));

        // Step 4: Go through the booking cycle
        for i in 0..(<Test as Config>::SuccessfulDeliveriesForStrikeReduction::get() - 1) {
            assert_ok!(RealXEducation::book_module(
                RuntimeOrigin::signed(school.clone()),
                0,
                0,
                bvec![4, 5, 6]
            ));

            assert_ok!(RealXEducation::claim_booking(
                RuntimeOrigin::signed(lecturer.clone()),
                0,
                i.into(),
            ));

            assert_ok!(RealXEducation::cancel_claim(
                RuntimeOrigin::signed(lecturer.clone()),
                0,
                i.into(),
            ));

            assert_ok!(RealXEducation::claim_booking(
                RuntimeOrigin::signed(lecturer.clone()),
                0,
                i.into(),
            ));

            assert_ok!(RealXEducation::submit_impact_score(
                RuntimeOrigin::signed(ai_agent.clone()),
                0,
                i.into(),
                Permill::from_percent(70),
                bvec![20, 20, 20],
                bvec![21, 21, 21],
                bvec![22, 22, 22]
            ));
        }

        // Step 5: School books a module
        assert_ok!(RealXEducation::book_module(
            RuntimeOrigin::signed(school.clone()),
            0,
            0,
            bvec![4, 5, 6]
        ));

        // Step 6: University student claims a module
        assert_ok!(RealXEducation::claim_booking(RuntimeOrigin::signed(lecturer.clone()), 0, 4,));

        // Pre-checks
        assert_eq!(EducationAssets::balance(asset_id, &school), 1);
        let multiplier = 10u128
            .checked_pow(AssetsMetadataWrapper::get_decimals(1984u32).unwrap().into())
            .unwrap();
        let expected_hold = 1250 * 26 as u128 * multiplier;
        let held = AssetsHolder::total_balance_on_hold(1984u32.into(), &sponsor);
        assert_eq!(held, expected_hold);
        assert_eq!(EducationAssets::total_issuance(asset_id), (module_amount - 4).into());
        assert_eq!(ModuleDeliverer::<Test>::get(&lecturer).unwrap().successful_deliveries, 4);
        assert_eq!(ModuleDeliverer::<Test>::get(&lecturer).unwrap().active_strikes, 4);

        // Step 7: AI Agent submits test results
        assert_ok!(RealXEducation::submit_impact_score(
            RuntimeOrigin::signed(ai_agent),
            0,
            4,
            Permill::from_percent(70),
            bvec![20, 20, 20],
            bvec![21, 21, 21],
            bvec![22, 22, 22]
        ));

        // 1. Token burned
        assert_eq!(EducationAssets::balance(asset_id, &lecturer), 0);
        assert_eq!(EducationAssets::total_issuance(asset_id), (module_amount - 5).into());

        // 2. Funds released and distributed
        let creator_pay = 29050u128;
        let protocol_pay = 175u128 * multiplier;
        let regional_operator_pay = 29050u128;
        let dbs_pay = 11_900u128;
        let lecturer_pay = 350_000u128;

        assert_eq!(ForeignAssets::balance(1984, &creator), creator_pay);
        assert_eq!(ForeignAssets::balance(1984, &lecturer), lecturer_pay + dbs_pay);
        assert_eq!(ForeignAssets::balance(1984, &[8; 32].into()), regional_operator_pay);
        assert_eq!(
            ForeignAssets::balance(1984, &RealXEducation::treasury_account_id()),
            protocol_pay
        );

        assert_eq!(
            AssetsHolder::total_balance_on_hold(1984u32.into(), &sponsor),
            31_250 * multiplier
        );

        // 3. Module deliverer storage updated
        assert_eq!(ModuleDeliverer::<Test>::get(&lecturer).unwrap().successful_deliveries, 5);
        assert_eq!(ModuleDeliverer::<Test>::get(&lecturer).unwrap().active_strikes, 3);

        // 4. Event
        System::assert_last_event(
            Event::TestResultsSubmitted {
                module_id: 0,
                booking_id: 4,
                lecturer,
                score: Permill::from_percent(70),
                lecturer_pay: 72_380,
            }
            .into(),
        );
    });
}

#[test]
fn submit_impact_score_fails() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);
        // No Permission fails
        assert_noop!(
            RealXEducation::submit_impact_score(
                RuntimeOrigin::signed([5; 32].into()),
                0,
                0,
                Permill::from_percent(75),
                bvec![20, 20, 20],
                bvec![21, 21, 21],
                bvec![22, 22, 22]
            ),
            BadOrigin
        );

        assert_ok!(XcavateWhitelist::add_admin(RuntimeOrigin::root(), [20; 32].into(),));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [1; 32].into(),
            pallet_xcavate_whitelist::Role::ModuleCreator
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [2; 32].into(),
            pallet_xcavate_whitelist::Role::ModuleSponsor
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [3; 32].into(),
            pallet_xcavate_whitelist::Role::ModuleBooker
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [4; 32].into(),
            pallet_xcavate_whitelist::Role::ModuleDeliverer
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [5; 32].into(),
            pallet_xcavate_whitelist::Role::ModuleDeliverer
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [5; 32].into(),
            pallet_xcavate_whitelist::Role::ModuleAIAgent
        ));
        new_region_helper();
        assert_ok!(RealXEducation::register_module_deliverer(RuntimeOrigin::signed(
            [4; 32].into()
        )));

        // Fails if there is no booking available
        assert_noop!(
            RealXEducation::submit_impact_score(
                RuntimeOrigin::signed([5; 32].into()),
                0,
                0,
                Permill::from_percent(75),
                bvec![20, 20, 20],
                bvec![21, 21, 21],
                bvec![22, 22, 22]
            ),
            Error::<Test>::NoBookingAvailable
        );

        assert_ok!(RealXEducation::create_module(
            RuntimeOrigin::signed([1; 32].into()),
            3,
            100,
            bvec![1, 2, 3]
        ));

        assert_ok!(RealXEducation::sponsor_module(
            RuntimeOrigin::signed([2; 32].into()),
            0,
            30,
            1984
        ));

        assert_ok!(RealXEducation::book_module(
            RuntimeOrigin::signed([3; 32].into()),
            0,
            0,
            bvec![4, 5, 6]
        ));

        // Fails if there is no lecturer set
        assert_noop!(
            RealXEducation::submit_impact_score(
                RuntimeOrigin::signed([5; 32].into()),
                0,
                0,
                Permill::from_percent(75),
                bvec![20, 20, 20],
                bvec![21, 21, 21],
                bvec![22, 22, 22]
            ),
            Error::<Test>::NoLecturerSet
        );

        assert_ok!(RealXEducation::claim_booking(RuntimeOrigin::signed([4; 32].into()), 0, 0,));

        assert_ok!(RealXEducation::submit_impact_score(
            RuntimeOrigin::signed([5; 32].into()),
            0,
            0,
            Permill::from_percent(75),
            bvec![20, 20, 20],
            bvec![21, 21, 21],
            bvec![22, 22, 22]
        ));

        // Fails if the score has already been set
        assert_noop!(
            RealXEducation::submit_impact_score(
                RuntimeOrigin::signed([5; 32].into()),
                0,
                0,
                Permill::from_percent(75),
                bvec![20, 20, 20],
                bvec![21, 21, 21],
                bvec![22, 22, 22]
            ),
            Error::<Test>::ScoreAlreadySet
        );
    });
}

#[test]
fn mint_recipient_nft_works() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);

        let creator: AccountId = [1; 32].into();
        let sponsor: AccountId = [2; 32].into();
        let school: AccountId = [3; 32].into();
        let lecturer: AccountId = [4; 32].into();
        let ai_agent: AccountId = [5; 32].into();
        let student: AccountId = [6; 32].into();

        let module_amount = 100u32;
        let purchase_amount = 30u32;

        // Step 1: Setup permissions
        assert_ok!(XcavateWhitelist::add_admin(RuntimeOrigin::root(), [20; 32].into(),));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [1; 32].into(),
            pallet_xcavate_whitelist::Role::ModuleCreator
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [2; 32].into(),
            pallet_xcavate_whitelist::Role::ModuleSponsor
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [3; 32].into(),
            pallet_xcavate_whitelist::Role::ModuleBooker
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [4; 32].into(),
            pallet_xcavate_whitelist::Role::ModuleDeliverer
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [5; 32].into(),
            pallet_xcavate_whitelist::Role::ModuleAIAgent
        ));
        new_region_helper();
        assert_ok!(RealXEducation::register_module_deliverer(RuntimeOrigin::signed(
            [4; 32].into()
        )));

        // Step 2: Creator creates a module
        assert_ok!(RealXEducation::create_module(
            RuntimeOrigin::signed(creator.clone()),
            3,
            module_amount,
            bvec![1, 2, 3]
        ));

        // Step 3: Sponsor purchases 30 tokens
        assert_ok!(RealXEducation::sponsor_module(
            RuntimeOrigin::signed(sponsor.clone()),
            0,
            purchase_amount,
            1984
        ));

        // Step 4: School books a module
        assert_ok!(RealXEducation::book_module(
            RuntimeOrigin::signed(school.clone()),
            0,
            0,
            bvec![4, 5, 6]
        ));

        // Step 5: University student claims a module
        assert_ok!(RealXEducation::claim_booking(RuntimeOrigin::signed(lecturer.clone()), 0, 0,));

        // Step 6: AI Agent submits test results
        assert_ok!(RealXEducation::submit_impact_score(
            RuntimeOrigin::signed(ai_agent.clone()),
            0,
            0,
            Permill::from_percent(100),
            bvec![20, 20, 20],
            bvec![21, 21, 21],
            bvec![22, 22, 22]
        ));

        // Pre-state
        let collection_id = ModuleInfo::<Test>::get(0).unwrap().collection_id;
        let before_next_id = NextNftId::<Test>::get(collection_id);
        assert_eq!(before_next_id, 4);

        // Step 7: School mints NFT for student
        assert_ok!(RealXEducation::mint_recipient_nft(
            RuntimeOrigin::signed(ai_agent),
            0,
            0,
            student.clone(),
            bvec![1, 2, 3]
        ));

        // 1. NFT was minted to the correct student
        let expected_item_id = before_next_id;
        assert_eq!(EducationNfts::owner(collection_id, expected_item_id), Some(student.clone()));

        // 2. NextNftId incremented correctly
        assert_eq!(NextNftId::<Test>::get(collection_id), expected_item_id + 1);

        // 3. Event emitted
        System::assert_last_event(
            Event::StudentNftMinted { module_id: 0, booking_id: 0, student }.into(),
        );
    });
}

#[test]
fn mint_recipient_nft_fails() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);
        // No Permission fails
        assert_noop!(
            RealXEducation::mint_recipient_nft(
                RuntimeOrigin::signed([3; 32].into()),
                0,
                0,
                [6; 32].into(),
                bvec![1, 2, 3]
            ),
            BadOrigin
        );

        assert_ok!(XcavateWhitelist::add_admin(RuntimeOrigin::root(), [20; 32].into(),));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [1; 32].into(),
            pallet_xcavate_whitelist::Role::ModuleCreator
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [2; 32].into(),
            pallet_xcavate_whitelist::Role::ModuleSponsor
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [3; 32].into(),
            pallet_xcavate_whitelist::Role::ModuleBooker
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [4; 32].into(),
            pallet_xcavate_whitelist::Role::ModuleDeliverer
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [5; 32].into(),
            pallet_xcavate_whitelist::Role::ModuleDeliverer
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [5; 32].into(),
            pallet_xcavate_whitelist::Role::ModuleAIAgent
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [7; 32].into(),
            pallet_xcavate_whitelist::Role::ModuleBooker
        ));
        new_region_helper();
        assert_ok!(RealXEducation::register_module_deliverer(RuntimeOrigin::signed(
            [4; 32].into()
        )));

        // Fails if there is no booking available
        assert_noop!(
            RealXEducation::mint_recipient_nft(
                RuntimeOrigin::signed([5; 32].into()),
                0,
                0,
                [5; 32].into(),
                bvec![1, 2, 3]
            ),
            Error::<Test>::NoBookingAvailable
        );

        assert_ok!(RealXEducation::create_module(
            RuntimeOrigin::signed([1; 32].into()),
            3,
            100,
            bvec![1, 2, 3]
        ));

        assert_ok!(RealXEducation::sponsor_module(
            RuntimeOrigin::signed([2; 32].into()),
            0,
            30,
            1984
        ));

        assert_ok!(RealXEducation::book_module(
            RuntimeOrigin::signed([3; 32].into()),
            0,
            0,
            bvec![4, 5, 6]
        ));

        assert_ok!(RealXEducation::claim_booking(RuntimeOrigin::signed([4; 32].into()), 0, 0,));

        // Fails if the test results have not been submitted.
        assert_noop!(
            RealXEducation::mint_recipient_nft(
                RuntimeOrigin::signed([5; 32].into()),
                0,
                0,
                [5; 32].into(),
                bvec![1, 2, 3]
            ),
            Error::<Test>::NoTestResultsSubmitted
        );
    });
}

#[test]
fn finish_booking_process_works() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);

        let creator: AccountId = [1; 32].into();
        let sponsor: AccountId = [2; 32].into();
        let school: AccountId = [3; 32].into();
        let lecturer: AccountId = [4; 32].into();
        let ai_agent: AccountId = [5; 32].into();
        let student: AccountId = [6; 32].into();

        let module_amount = 100u32;
        let purchase_amount = 30u32;

        // Step 1: Setup permissions
        assert_ok!(XcavateWhitelist::add_admin(RuntimeOrigin::root(), [20; 32].into(),));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [1; 32].into(),
            pallet_xcavate_whitelist::Role::ModuleCreator
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [2; 32].into(),
            pallet_xcavate_whitelist::Role::ModuleSponsor
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [3; 32].into(),
            pallet_xcavate_whitelist::Role::ModuleBooker
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [4; 32].into(),
            pallet_xcavate_whitelist::Role::ModuleDeliverer
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [5; 32].into(),
            pallet_xcavate_whitelist::Role::ModuleAIAgent
        ));
        new_region_helper();
        assert_ok!(RealXEducation::register_module_deliverer(RuntimeOrigin::signed(
            [4; 32].into()
        )));

        // Step 2: Creator creates a module
        assert_ok!(RealXEducation::create_module(
            RuntimeOrigin::signed(creator.clone()),
            3,
            module_amount,
            bvec![1, 2, 3]
        ));

        // Step 3: Sponsor purchases 30 tokens
        assert_ok!(RealXEducation::sponsor_module(
            RuntimeOrigin::signed(sponsor.clone()),
            0,
            purchase_amount,
            1984
        ));

        // Step 4: School books a module
        assert_ok!(RealXEducation::book_module(
            RuntimeOrigin::signed(school.clone()),
            0,
            0,
            bvec![4, 5, 6]
        ));

        // Step 5: University student claims a module
        assert_ok!(RealXEducation::claim_booking(RuntimeOrigin::signed(lecturer.clone()), 0, 0,));

        // Step 6: AI Agent submits test results
        assert_ok!(RealXEducation::submit_impact_score(
            RuntimeOrigin::signed(ai_agent.clone()),
            0,
            0,
            Permill::from_percent(100),
            bvec![20, 20, 20],
            bvec![21, 21, 21],
            bvec![22, 22, 22]
        ));

        // Step 7: School mints NFT for student
        assert_ok!(RealXEducation::mint_recipient_nft(
            RuntimeOrigin::signed(ai_agent),
            0,
            0,
            student.clone(),
            bvec![1, 2, 3]
        ));

        // Pre-state
        assert_eq!(Balances::free_balance(&school), 4_990);
        assert_eq!(Balances::balance_on_hold(&HoldReason::BookingReserve.into(), &school), 10);
        assert!(Bookings::<Test>::get::<u32, u64>(0, 0).is_some());

        // Step 8: School finishes booking process
        assert_ok!(RealXEducation::finish_booking_process(
            RuntimeOrigin::signed(school.clone()),
            0,
            0,
        ));

        // 1. Locked token released
        assert_eq!(Balances::free_balance(&school), 5_000);
        assert_eq!(Balances::balance_on_hold(&HoldReason::BookingReserve.into(), &school), 0);

        // 2. Booking storage removed
        assert!(Bookings::<Test>::get::<u32, u64>(0, 0).is_none());

        // 3. Event emitted
        System::assert_last_event(
            Event::FinishBookingProcess { school, module_id: 0, booking_id: 0 }.into(),
        );
    });
}

#[test]
fn finish_booking_process_fails() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);
        // No Permission fails
        assert_noop!(
            RealXEducation::finish_booking_process(RuntimeOrigin::signed([3; 32].into()), 0, 0,),
            BadOrigin
        );

        assert_ok!(XcavateWhitelist::add_admin(RuntimeOrigin::root(), [20; 32].into(),));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [1; 32].into(),
            pallet_xcavate_whitelist::Role::ModuleCreator
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [2; 32].into(),
            pallet_xcavate_whitelist::Role::ModuleSponsor
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [3; 32].into(),
            pallet_xcavate_whitelist::Role::ModuleBooker
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [4; 32].into(),
            pallet_xcavate_whitelist::Role::ModuleDeliverer
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [5; 32].into(),
            pallet_xcavate_whitelist::Role::ModuleAIAgent
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [7; 32].into(),
            pallet_xcavate_whitelist::Role::ModuleBooker
        ));
        new_region_helper();
        assert_ok!(RealXEducation::register_module_deliverer(RuntimeOrigin::signed(
            [4; 32].into()
        )));

        // Fails if there is no booking available
        assert_noop!(
            RealXEducation::finish_booking_process(RuntimeOrigin::signed([3; 32].into()), 0, 0,),
            Error::<Test>::NoBookingAvailable
        );

        assert_ok!(RealXEducation::create_module(
            RuntimeOrigin::signed([1; 32].into()),
            3,
            100,
            bvec![1, 2, 3]
        ));

        assert_ok!(RealXEducation::sponsor_module(
            RuntimeOrigin::signed([2; 32].into()),
            0,
            30,
            1984
        ));

        assert_ok!(RealXEducation::book_module(
            RuntimeOrigin::signed([3; 32].into()),
            0,
            0,
            bvec![4, 5, 6]
        ));

        assert_ok!(RealXEducation::claim_booking(RuntimeOrigin::signed([4; 32].into()), 0, 0,));

        // Fails if the test results have not been submitted.
        assert_noop!(
            RealXEducation::finish_booking_process(RuntimeOrigin::signed([3; 32].into()), 0, 0,),
            Error::<Test>::NoTestResultsSubmitted
        );

        assert_ok!(RealXEducation::submit_impact_score(
            RuntimeOrigin::signed([5; 32].into()),
            0,
            0,
            Permill::from_percent(75),
            bvec![20, 20, 20],
            bvec![21, 21, 21],
            bvec![22, 22, 22]
        ));

        // Fails if the caller is not the school teacher who booked the module.
        assert_noop!(
            RealXEducation::finish_booking_process(RuntimeOrigin::signed([7; 32].into()), 0, 0,),
            Error::<Test>::NoPermission
        );
    });
}

#[test]
fn burn_unsponsored_token_works1() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);

        let creator: AccountId = [1; 32].into();
        let sponsor: AccountId = [2; 32].into();
        let school: AccountId = [3; 32].into();
        let lecturer: AccountId = [4; 32].into();
        let ai_agent: AccountId = [5; 32].into();

        let module_amount = 100u32;
        let purchase_amount = 30u32;
        let burn_amount = 20u32;

        // Step 1: Setup permissions
        assert_ok!(XcavateWhitelist::add_admin(RuntimeOrigin::root(), [20; 32].into(),));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [1; 32].into(),
            pallet_xcavate_whitelist::Role::ModuleCreator
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [2; 32].into(),
            pallet_xcavate_whitelist::Role::ModuleSponsor
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [3; 32].into(),
            pallet_xcavate_whitelist::Role::ModuleBooker
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [4; 32].into(),
            pallet_xcavate_whitelist::Role::ModuleDeliverer
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [5; 32].into(),
            pallet_xcavate_whitelist::Role::ModuleAIAgent
        ));
        new_region_helper();
        assert_ok!(RealXEducation::register_module_deliverer(RuntimeOrigin::signed(
            [4; 32].into()
        )));

        // Step 2: Creator creates a module
        assert_ok!(RealXEducation::create_module(
            RuntimeOrigin::signed(creator.clone()),
            3,
            module_amount,
            bvec![1, 2, 3]
        ));

        // Step 3: Sponsor purchases 30 tokens
        assert_ok!(RealXEducation::sponsor_module(
            RuntimeOrigin::signed(sponsor.clone()),
            0,
            purchase_amount,
            1984
        ));

        // Step 4: School books a module
        assert_ok!(RealXEducation::book_module(
            RuntimeOrigin::signed(school.clone()),
            0,
            0,
            bvec![4, 5, 6]
        ));

        // Step 5: University student claims a module
        assert_ok!(RealXEducation::claim_booking(RuntimeOrigin::signed(lecturer.clone()), 0, 0,));

        // Step 6: AI Agent submits test results
        assert_ok!(RealXEducation::submit_impact_score(
            RuntimeOrigin::signed(ai_agent.clone()),
            0,
            0,
            Permill::from_percent(100),
            bvec![20, 20, 20],
            bvec![21, 21, 21],
            bvec![22, 22, 22]
        ));

        // Pre-state
        assert_eq!(EducationAssets::balance(0, &creator), 70);
        assert_eq!(EducationAssets::total_issuance(0), 99);
        assert_eq!(ModuleInfo::<Test>::get(0).unwrap().sponsor_allocation, 70);

        // Step 7: Creator burns unsponsored tokens
        assert_ok!(RealXEducation::burn_unsponsored_token(
            RuntimeOrigin::signed(creator.clone()),
            0,
            burn_amount,
        ));

        // 1. Post burn balances
        assert_eq!(EducationAssets::balance(0, &creator), 50);
        assert_eq!(EducationAssets::total_issuance(0), 79);

        // 2. Module storage updated
        assert_eq!(ModuleInfo::<Test>::get(0).unwrap().sponsor_allocation, 50);

        // 3. Event emitted
        System::assert_last_event(
            Event::UnsponsoredTokensBurned {
                module_id: 0,
                creator,
                amount: burn_amount,
                remaining_allocation: 50,
            }
            .into(),
        );
    });
}

#[test]
fn burn_unsponsored_token_works2() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);

        let creator: AccountId = [1; 32].into();

        let module_amount = 100u32;
        let burn_amount = 80u32;

        // Step 1: Setup permissions
        assert_ok!(XcavateWhitelist::add_admin(RuntimeOrigin::root(), [20; 32].into(),));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [1; 32].into(),
            pallet_xcavate_whitelist::Role::ModuleCreator
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [2; 32].into(),
            pallet_xcavate_whitelist::Role::ModuleSponsor
        ));
        new_region_helper();

        // Step 2: Creator creates a module
        assert_ok!(RealXEducation::create_module(
            RuntimeOrigin::signed(creator.clone()),
            3,
            module_amount,
            bvec![1, 2, 3]
        ));

        // Pre-state
        assert_eq!(EducationAssets::balance(0, &creator), 100);
        assert_eq!(EducationAssets::total_issuance(0), 100);
        assert_eq!(ModuleInfo::<Test>::get(0).unwrap().sponsor_allocation, 100);

        // Step 3: Creator burns unsponsored tokens
        assert_ok!(RealXEducation::burn_unsponsored_token(
            RuntimeOrigin::signed(creator.clone()),
            0,
            burn_amount,
        ));

        // 1. Post burn balances
        assert_eq!(EducationAssets::balance(0, &creator), 20);
        assert_eq!(EducationAssets::total_issuance(0), 20);

        // 2. Module storage updated
        assert_eq!(ModuleInfo::<Test>::get(0).unwrap().sponsor_allocation, 20);

        // 3. Event emitted
        System::assert_last_event(
            Event::UnsponsoredTokensBurned {
                module_id: 0,
                creator,
                amount: burn_amount,
                remaining_allocation: 20,
            }
            .into(),
        );
    });
}

#[test]
fn burn_unsponsored_token_fails() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);
        // No Permission fails
        assert_noop!(
            RealXEducation::burn_unsponsored_token(RuntimeOrigin::signed([1; 32].into()), 0, 20),
            BadOrigin
        );

        assert_ok!(XcavateWhitelist::add_admin(RuntimeOrigin::root(), [20; 32].into(),));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [1; 32].into(),
            pallet_xcavate_whitelist::Role::ModuleCreator
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [10; 32].into(),
            pallet_xcavate_whitelist::Role::ModuleCreator
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [2; 32].into(),
            pallet_xcavate_whitelist::Role::ModuleSponsor
        ));
        new_region_helper();

        // Fails if there is no module
        assert_noop!(
            RealXEducation::burn_unsponsored_token(RuntimeOrigin::signed([1; 32].into()), 0, 20),
            Error::<Test>::ModuleNotAvailable
        );

        assert_ok!(RealXEducation::create_module(
            RuntimeOrigin::signed([1; 32].into()),
            3,
            30,
            bvec![1, 2, 3]
        ));

        // No Permission fails
        assert_noop!(
            RealXEducation::burn_unsponsored_token(RuntimeOrigin::signed([10; 32].into()), 0, 20),
            Error::<Test>::NoPermission
        );

        assert_ok!(RealXEducation::sponsor_module(
            RuntimeOrigin::signed([2; 32].into()),
            0,
            30,
            1984
        ));

        // Fails if insufficient balance
        assert_noop!(
            RealXEducation::burn_unsponsored_token(RuntimeOrigin::signed([1; 32].into()), 0, 1),
            Error::<Test>::InsufficientBalance
        );
    });
}

#[test]
fn remove_module_works1() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);

        let creator: AccountId = [1; 32].into();
        let sponsor: AccountId = [2; 32].into();
        let school: AccountId = [3; 32].into();
        let lecturer: AccountId = [4; 32].into();
        let ai_agent: AccountId = [5; 32].into();

        let module_amount = 100u32;
        let purchase_amount = 40u32;
        let burn_amount = 60u32;

        // Step 1: Setup permissions
        assert_ok!(XcavateWhitelist::add_admin(RuntimeOrigin::root(), [20; 32].into(),));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [1; 32].into(),
            pallet_xcavate_whitelist::Role::ModuleCreator
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [2; 32].into(),
            pallet_xcavate_whitelist::Role::ModuleSponsor
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [3; 32].into(),
            pallet_xcavate_whitelist::Role::ModuleBooker
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [4; 32].into(),
            pallet_xcavate_whitelist::Role::ModuleDeliverer
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [5; 32].into(),
            pallet_xcavate_whitelist::Role::ModuleAIAgent
        ));
        new_region_helper();
        assert_ok!(RealXEducation::register_module_deliverer(RuntimeOrigin::signed(
            [4; 32].into()
        )));

        // Step 2: Creator creates a module
        assert_ok!(RealXEducation::create_module(
            RuntimeOrigin::signed(creator.clone()),
            3,
            module_amount,
            bvec![1, 2, 3]
        ));

        assert_ok!(RealXEducation::sponsor_module(
            RuntimeOrigin::signed(sponsor.clone()),
            0,
            purchase_amount,
            1984
        ));

        // Step 3: Deliver ALL 40 sponsored lessons (40 bookings + submits)
        for i in 0..purchase_amount {
            assert_ok!(RealXEducation::book_module(
                RuntimeOrigin::signed(school.clone()),
                0,
                0,
                bvec![i as u8]
            ));

            assert_ok!(RealXEducation::claim_booking(
                RuntimeOrigin::signed(lecturer.clone()),
                0,
                i as u64
            ));

            assert_ok!(RealXEducation::submit_impact_score(
                RuntimeOrigin::signed(ai_agent.clone()),
                0,
                i as u64,
                Permill::from_percent(100),
                bvec![20],
                bvec![21],
                bvec![21]
            ));
        }

        // Step 4: Creator burns unsponsored tokens
        assert_ok!(RealXEducation::burn_unsponsored_token(
            RuntimeOrigin::signed(creator.clone()),
            0,
            burn_amount,
        ));

        // Pre-state
        let asset_id = ModuleInfo::<Test>::get(0).unwrap().asset_id;
        assert_eq!(EducationAssets::total_issuance(asset_id), 0);
        assert!(ModuleInfo::<Test>::get(0).is_some());

        // Step 5: Creator removes module
        assert_ok!(RealXEducation::remove_module(RuntimeOrigin::signed(creator.clone()), 0,));

        // 1. Module storage removed
        assert!(ModuleInfo::<Test>::get(0).is_none());
        assert!(!EducationAssets::asset_exists(asset_id));

        // 2. Event emitted
        System::assert_last_event(Event::ModuleRemoved { module_id: 0, creator }.into());
    });
}

#[test]
fn remove_module_works2() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);

        let creator: AccountId = [1; 32].into();

        let module_amount = 100u32;

        // Step 1: Setup permissions
        assert_ok!(XcavateWhitelist::add_admin(RuntimeOrigin::root(), [20; 32].into(),));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [1; 32].into(),
            pallet_xcavate_whitelist::Role::ModuleCreator
        ));
        new_region_helper();

        // Step 2: Creator creates a module
        assert_ok!(RealXEducation::create_module(
            RuntimeOrigin::signed(creator.clone()),
            3,
            module_amount,
            bvec![1, 2, 3]
        ));

        // Step 3: Creator burns unsponsored tokens
        assert_ok!(RealXEducation::burn_unsponsored_token(
            RuntimeOrigin::signed(creator.clone()),
            0,
            module_amount,
        ));

        // Pre-state
        let asset_id = ModuleInfo::<Test>::get(0).unwrap().asset_id;
        assert_eq!(EducationAssets::total_issuance(asset_id), 0);
        assert!(ModuleInfo::<Test>::get(0).is_some());

        // Step 4: Creator removes module
        assert_ok!(RealXEducation::remove_module(RuntimeOrigin::signed(creator.clone()), 0,));

        // 1. Module storage removed
        assert!(ModuleInfo::<Test>::get(0).is_none());
        assert!(!EducationAssets::asset_exists(asset_id));

        // 2. Event emitted
        System::assert_last_event(Event::ModuleRemoved { module_id: 0, creator }.into());
    });
}

#[test]
fn remove_module_fails() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);
        // No Permission fails
        assert_noop!(
            RealXEducation::remove_module(RuntimeOrigin::signed([1; 32].into()), 0),
            BadOrigin
        );

        assert_ok!(XcavateWhitelist::add_admin(RuntimeOrigin::root(), [20; 32].into(),));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [1; 32].into(),
            pallet_xcavate_whitelist::Role::ModuleCreator
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [10; 32].into(),
            pallet_xcavate_whitelist::Role::ModuleCreator
        ));
        new_region_helper();

        // Fails if there is no module
        assert_noop!(
            RealXEducation::remove_module(RuntimeOrigin::signed([1; 32].into()), 0),
            Error::<Test>::ModuleNotAvailable
        );

        assert_ok!(RealXEducation::create_module(
            RuntimeOrigin::signed([1; 32].into()),
            3,
            30,
            bvec![1, 2, 3]
        ));

        // No Permission fails
        assert_noop!(
            RealXEducation::remove_module(RuntimeOrigin::signed([10; 32].into()), 0),
            Error::<Test>::NoPermission
        );

        // Fails if insufficient balance
        assert_noop!(
            RealXEducation::remove_module(RuntimeOrigin::signed([1; 32].into()), 0),
            Error::<Test>::CannotRemoveModuleWithActiveTokens
        );
    });
}

#[test]
fn cancel_booking_works() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);

        let creator: AccountId = [1; 32].into();
        let sponsor: AccountId = [2; 32].into();
        let school: AccountId = [3; 32].into();

        let module_amount = 100u32;
        let purchase_amount = 40u32;

        // Step 1: Setup permissions
        assert_ok!(XcavateWhitelist::add_admin(RuntimeOrigin::root(), [20; 32].into(),));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [1; 32].into(),
            pallet_xcavate_whitelist::Role::ModuleCreator
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [2; 32].into(),
            pallet_xcavate_whitelist::Role::ModuleSponsor
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [3; 32].into(),
            pallet_xcavate_whitelist::Role::ModuleBooker
        ));
        new_region_helper();

        // Step 2: Creator creates a module
        assert_ok!(RealXEducation::create_module(
            RuntimeOrigin::signed(creator.clone()),
            3,
            module_amount,
            bvec![1, 2, 3]
        ));

        // Step 3: Sponsor purchases 40 tokens
        assert_ok!(RealXEducation::sponsor_module(
            RuntimeOrigin::signed(sponsor.clone()),
            0,
            purchase_amount,
            1984
        ));

        // Step 4: School books a module
        assert_ok!(RealXEducation::book_module(
            RuntimeOrigin::signed(school.clone()),
            0,
            0,
            bvec![4, 5, 6]
        ));

        // Pre-state
        assert_eq!(Balances::free_balance(&school), 4_990);
        assert_eq!(Balances::balance_on_hold(&HoldReason::BookingReserve.into(), &school), 10);
        assert!(Bookings::<Test>::get::<u32, u64>(0, 0).is_some());
        assert_eq!(EducationAssets::balance(0, &school), 1);
        assert_eq!(EducationAssets::balance(0, &sponsor), 39);
        let module = ModuleInfo::<Test>::get(0).unwrap();
        assert_eq!(module.school_allocation, 39);
        assert_eq!(module.university_student_allocation, 1);

        // Step 5: School cancels booking
        assert_ok!(RealXEducation::cancel_booking(RuntimeOrigin::signed(school.clone()), 0, 0,));

        // 1. Locked token released
        assert_eq!(Balances::free_balance(&school), 5_000);
        assert_eq!(Balances::balance_on_hold(&HoldReason::BookingReserve.into(), &school), 0);

        // 2. Booking storage removed
        assert!(Bookings::<Test>::get::<u32, u64>(0, 0).is_none());

        // 3. Cancellation storage updated
        assert_eq!(BookingCancellationCounter::<Test>::get(&school), 1);
        let current_block = System::block_number();
        assert!(SchoolCancellations::<Test>::contains_key(&school, (current_block, 0)));

        // 4. school balance updated
        assert_eq!(EducationAssets::balance(0, &school), 0);
        assert_eq!(EducationAssets::balance(0, &sponsor), 40);

        // 5, Module storage upadet
        let updated_module = ModuleInfo::<Test>::get(0).unwrap();
        assert_eq!(updated_module.school_allocation, 40);
        assert_eq!(updated_module.university_student_allocation, 0);
        assert_eq!(SponsoredModules::<Test>::get(0, 0).unwrap().amount, 40);

        // 6. Event emitted
        System::assert_last_event(
            Event::BookingCancelled { school, module_id: 0, booking_id: 0, cancellation_count: 1 }
                .into(),
        );
    });
}

#[test]
fn cancel_booking_set_lecturer_works() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);

        let creator: AccountId = [1; 32].into();
        let sponsor: AccountId = [2; 32].into();
        let school: AccountId = [3; 32].into();
        let lecturer: AccountId = [4; 32].into();

        let module_amount = 100u32;
        let purchase_amount = 40u32;

        // Step 1: Setup permissions
        assert_ok!(XcavateWhitelist::add_admin(RuntimeOrigin::root(), [20; 32].into(),));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [1; 32].into(),
            pallet_xcavate_whitelist::Role::ModuleCreator
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [2; 32].into(),
            pallet_xcavate_whitelist::Role::ModuleSponsor
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [3; 32].into(),
            pallet_xcavate_whitelist::Role::ModuleBooker
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [4; 32].into(),
            pallet_xcavate_whitelist::Role::ModuleDeliverer
        ));
        new_region_helper();
        assert_ok!(RealXEducation::register_module_deliverer(RuntimeOrigin::signed(
            [4; 32].into()
        )));

        // Step 2: Creator creates a module
        assert_ok!(RealXEducation::create_module(
            RuntimeOrigin::signed(creator.clone()),
            3,
            module_amount,
            bvec![1, 2, 3]
        ));

        // Step 3: Sponsor purchases 40 tokens
        assert_ok!(RealXEducation::sponsor_module(
            RuntimeOrigin::signed(sponsor.clone()),
            0,
            purchase_amount,
            1984
        ));

        // Step 4: School books a module
        assert_ok!(RealXEducation::book_module(
            RuntimeOrigin::signed(school.clone()),
            0,
            0,
            bvec![4, 5, 6]
        ));

        // Step 5: University student claims a module
        assert_ok!(RealXEducation::claim_booking(RuntimeOrigin::signed(lecturer.clone()), 0, 0,));

        // Pre-state
        assert_eq!(Balances::free_balance(&school), 4_990);
        assert_eq!(Balances::balance_on_hold(&HoldReason::BookingReserve.into(), &school), 10);
        assert!(Bookings::<Test>::get::<u32, u64>(0, 0).is_some());
        assert_eq!(EducationAssets::balance(0, &school), 1);
        assert_eq!(EducationAssets::balance(0, &sponsor), 39);
        let module = ModuleInfo::<Test>::get(0).unwrap();
        assert_eq!(module.school_allocation, 39);
        assert_eq!(module.university_student_allocation, 0);
        assert_eq!(ModuleDeliverer::<Test>::get(&lecturer).unwrap().active_claims, 1);

        // Step 6: School cancels booking
        assert_ok!(RealXEducation::cancel_booking(RuntimeOrigin::signed(school.clone()), 0, 0,));

        // 1. Locked token released
        assert_eq!(Balances::free_balance(&school), 5_000);
        assert_eq!(Balances::balance_on_hold(&HoldReason::BookingReserve.into(), &school), 0);

        // 2. Booking storage removed
        assert!(Bookings::<Test>::get::<u32, u64>(0, 0).is_none());

        // 3. Cancellation storage updated
        assert_eq!(BookingCancellationCounter::<Test>::get(&school), 1);
        let current_block = System::block_number();
        assert!(SchoolCancellations::<Test>::contains_key(&school, (current_block, 0)));
        assert_eq!(ModuleDeliverer::<Test>::get(&lecturer).unwrap().active_claims, 0);

        // 4. school balance updated
        assert_eq!(EducationAssets::balance(0, &school), 0);
        assert_eq!(EducationAssets::balance(0, &sponsor), 40);

        // 5, Module storage upadet
        let updated_module = ModuleInfo::<Test>::get(0).unwrap();
        assert_eq!(updated_module.school_allocation, 40);
        assert_eq!(updated_module.university_student_allocation, 0);
        assert_eq!(SponsoredModules::<Test>::get(0, 0).unwrap().amount, 40);

        // 6. Event emitted
        System::assert_last_event(
            Event::BookingCancelled { school, module_id: 0, booking_id: 0, cancellation_count: 1 }
                .into(),
        );
    });
}

#[test]
fn cancel_booking_works_slashing() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);

        let creator: AccountId = [1; 32].into();
        let sponsor: AccountId = [2; 32].into();
        let school: AccountId = [3; 32].into();

        let module_amount = 100u32;
        let purchase_amount = 40u32;

        // Step 1: Setup permissions
        assert_ok!(XcavateWhitelist::add_admin(RuntimeOrigin::root(), [20; 32].into(),));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [1; 32].into(),
            pallet_xcavate_whitelist::Role::ModuleCreator
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [2; 32].into(),
            pallet_xcavate_whitelist::Role::ModuleSponsor
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [3; 32].into(),
            pallet_xcavate_whitelist::Role::ModuleBooker
        ));
        new_region_helper();

        // Step 2: Creator creates a module
        assert_ok!(RealXEducation::create_module(
            RuntimeOrigin::signed(creator.clone()),
            3,
            module_amount,
            bvec![1, 2, 3]
        ));

        // Step 3: Sponsor purchases 40 tokens
        assert_ok!(RealXEducation::sponsor_module(
            RuntimeOrigin::signed(sponsor.clone()),
            0,
            purchase_amount,
            1984
        ));

        // Step 4: School cancels bookings up to the max allowed cancellations
        for i in 0..(<Test as Config>::MaxCancellations::get() - 1) {
            assert_ok!(RealXEducation::book_module(
                RuntimeOrigin::signed(school.clone()),
                0,
                0,
                bvec![4, 5, 6]
            ));

            assert_ok!(RealXEducation::cancel_booking(
                RuntimeOrigin::signed(school.clone()),
                0,
                i.into(),
            ));
        }

        // Step 5: School books a module
        assert_ok!(RealXEducation::book_module(
            RuntimeOrigin::signed(school.clone()),
            0,
            0,
            bvec![4, 5, 6]
        ));

        // Pre-state
        assert_eq!(Balances::free_balance(&school), 4_990);
        assert_eq!(Balances::balance_on_hold(&HoldReason::BookingReserve.into(), &school), 10);
        assert!(Bookings::<Test>::get::<u32, u64>(0, 2).is_some());
        assert_eq!(Balances::total_issuance(), 305_000);

        // Step 6: School cancels booking
        assert_ok!(RealXEducation::cancel_booking(RuntimeOrigin::signed(school.clone()), 0, 2,));

        // 1. Locked token released
        assert_eq!(Balances::free_balance(&school), 4_990);
        assert_eq!(Balances::balance_on_hold(&HoldReason::BookingReserve.into(), &school), 0);
        assert_eq!(Balances::total_issuance(), 304_990);

        // 2. Booking storage removed
        assert!(Bookings::<Test>::get::<u32, u64>(0, 0).is_none());

        // 3. Cancellation storage updated
        assert_eq!(BookingCancellationCounter::<Test>::get(&school), 3);
        let current_block = System::block_number();
        assert!(SchoolCancellations::<Test>::contains_key(&school, (current_block, 2)));

        // 4. Event emitted
        System::assert_last_event(
            Event::BookingCancelled { school, module_id: 0, booking_id: 2, cancellation_count: 3 }
                .into(),
        );
    });
}

#[test]
fn cancel_booking_fails() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);
        // No Permission fails
        assert_noop!(
            RealXEducation::cancel_booking(RuntimeOrigin::signed([3; 32].into()), 0, 0),
            BadOrigin
        );

        assert_ok!(XcavateWhitelist::add_admin(RuntimeOrigin::root(), [20; 32].into(),));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [1; 32].into(),
            pallet_xcavate_whitelist::Role::ModuleCreator
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [2; 32].into(),
            pallet_xcavate_whitelist::Role::ModuleSponsor
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [3; 32].into(),
            pallet_xcavate_whitelist::Role::ModuleBooker
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [10; 32].into(),
            pallet_xcavate_whitelist::Role::ModuleBooker
        ));
        new_region_helper();

        assert_ok!(RealXEducation::create_module(
            RuntimeOrigin::signed([1; 32].into()),
            3,
            100,
            bvec![1, 2, 3]
        ));

        assert_ok!(RealXEducation::sponsor_module(
            RuntimeOrigin::signed([2; 32].into()),
            0,
            30,
            1984
        ));

        // Fails if there is no booking
        assert_noop!(
            RealXEducation::cancel_booking(RuntimeOrigin::signed([3; 32].into()), 0, 0),
            Error::<Test>::NoBookingAvailable
        );

        assert_ok!(RealXEducation::book_module(
            RuntimeOrigin::signed([3; 32].into()),
            0,
            0,
            bvec![4, 5, 6]
        ));

        // No Permission fails
        assert_noop!(
            RealXEducation::cancel_booking(RuntimeOrigin::signed([10; 32].into()), 0, 0),
            Error::<Test>::NoPermission
        );
    });
}

#[test]
fn clear_old_cancellations_works1() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);

        let creator: AccountId = [1; 32].into();
        let sponsor: AccountId = [2; 32].into();
        let school: AccountId = [3; 32].into();

        let module_amount = 100u32;
        let purchase_amount = 40u32;

        // Step 1: Setup permissions
        assert_ok!(XcavateWhitelist::add_admin(RuntimeOrigin::root(), [20; 32].into(),));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [1; 32].into(),
            pallet_xcavate_whitelist::Role::ModuleCreator
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [2; 32].into(),
            pallet_xcavate_whitelist::Role::ModuleSponsor
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [3; 32].into(),
            pallet_xcavate_whitelist::Role::ModuleBooker
        ));
        new_region_helper();

        // Step 2: Creator creates a module
        assert_ok!(RealXEducation::create_module(
            RuntimeOrigin::signed(creator.clone()),
            3,
            module_amount,
            bvec![1, 2, 3]
        ));

        // Step 3: Sponsor purchases 40 tokens
        assert_ok!(RealXEducation::sponsor_module(
            RuntimeOrigin::signed(sponsor.clone()),
            0,
            purchase_amount,
            1984
        ));

        // Step 4: School cancels bookings up to the max allowed cancellations
        for i in 0..<Test as Config>::MaxCancellations::get() {
            assert_ok!(RealXEducation::book_module(
                RuntimeOrigin::signed(school.clone()),
                0,
                0,
                bvec![4, 5, 6]
            ));

            assert_ok!(RealXEducation::cancel_booking(
                RuntimeOrigin::signed(school.clone()),
                0,
                i.into(),
            ));
        }

        // Pre-state
        assert_eq!(BookingCancellationCounter::<Test>::get(&school), 3);
        let current_block = System::block_number();
        assert!(SchoolCancellations::<Test>::contains_key(&school, (current_block, 2)));

        // Move forward blocks beyond the cancellation window
        let block_number =
            frame_system::Pallet::<Test>::block_number() + CancellationWindow::get() + 1;
        run_to_block(block_number);

        // Step 5: Clear the cancellations older than the window
        assert_ok!(RealXEducation::clear_old_cancellations(RuntimeOrigin::signed(school.clone()),));

        // 1. Cancellation storage cleared
        assert_eq!(BookingCancellationCounter::<Test>::get(&school), 0);
        assert!(!SchoolCancellations::<Test>::contains_key(&school, (current_block, 2)));

        // 2. Event emitted
        System::assert_last_event(Event::OldCancellationsCleared { school, removed: 3 }.into());
    });
}

#[test]
fn clear_old_cancellations_works2() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);

        let creator: AccountId = [1; 32].into();
        let sponsor: AccountId = [2; 32].into();
        let school: AccountId = [3; 32].into();

        let module_amount = 100u32;
        let purchase_amount = 40u32;

        // Step 1: Setup permissions
        assert_ok!(XcavateWhitelist::add_admin(RuntimeOrigin::root(), [20; 32].into(),));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [1; 32].into(),
            pallet_xcavate_whitelist::Role::ModuleCreator
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [2; 32].into(),
            pallet_xcavate_whitelist::Role::ModuleSponsor
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [3; 32].into(),
            pallet_xcavate_whitelist::Role::ModuleBooker
        ));
        new_region_helper();

        // Step 2: Creator creates a module
        assert_ok!(RealXEducation::create_module(
            RuntimeOrigin::signed(creator.clone()),
            3,
            module_amount,
            bvec![1, 2, 3]
        ));

        // Step 3: Sponsor purchases 40 tokens
        assert_ok!(RealXEducation::sponsor_module(
            RuntimeOrigin::signed(sponsor.clone()),
            0,
            purchase_amount,
            1984
        ));

        // Step 4: School cancels bookings up to the max allowed cancellations
        for i in 0..<Test as Config>::MaxCancellations::get() {
            assert_ok!(RealXEducation::book_module(
                RuntimeOrigin::signed(school.clone()),
                0,
                0,
                bvec![4, 5, 6]
            ));

            assert_ok!(RealXEducation::cancel_booking(
                RuntimeOrigin::signed(school.clone()),
                0,
                i.into(),
            ));
        }

        // Move forward blocks 1 block
        let block_number = frame_system::Pallet::<Test>::block_number() + 1;
        run_to_block(block_number);

        for i in 0..<Test as Config>::MaxCancellations::get() {
            assert_ok!(RealXEducation::book_module(
                RuntimeOrigin::signed(school.clone()),
                0,
                0,
                bvec![4, 5, 6]
            ));

            assert_ok!(RealXEducation::cancel_booking(
                RuntimeOrigin::signed(school.clone()),
                0,
                (i + <Test as Config>::MaxCancellations::get()).into(),
            ));
        }

        // Pre-state
        assert_eq!(BookingCancellationCounter::<Test>::get(&school), 6);
        let current_block = System::block_number();
        assert!(SchoolCancellations::<Test>::contains_key(&school, ((current_block - 1), 2)));
        assert!(SchoolCancellations::<Test>::contains_key(&school, (current_block, 5)));

        // Move forward blocks beyond the cancellation window
        let block_number = frame_system::Pallet::<Test>::block_number() + CancellationWindow::get();
        run_to_block(block_number);

        // Step 5: Clear the cancellations older than the window
        assert_ok!(RealXEducation::clear_old_cancellations(RuntimeOrigin::signed(school.clone()),));

        // 1. Cancellation storage cleared
        assert_eq!(BookingCancellationCounter::<Test>::get(&school), 3);
        assert!(!SchoolCancellations::<Test>::contains_key(&school, ((current_block - 1), 2)));
        assert!(SchoolCancellations::<Test>::contains_key(&school, (current_block, 5)));

        // 2. Event emitted
        System::assert_last_event(Event::OldCancellationsCleared { school, removed: 3 }.into());
    });
}

#[test]
fn clear_old_cancellations_up_to_max_cleanup_works() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);

        let creator: AccountId = [1; 32].into();
        let sponsor: AccountId = [2; 32].into();
        let school: AccountId = [3; 32].into();

        let module_amount = 100u32;
        let purchase_amount = 40u32;

        // Step 1: Setup permissions
        assert_ok!(XcavateWhitelist::add_admin(RuntimeOrigin::root(), [20; 32].into(),));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [1; 32].into(),
            pallet_xcavate_whitelist::Role::ModuleCreator
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [2; 32].into(),
            pallet_xcavate_whitelist::Role::ModuleSponsor
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [3; 32].into(),
            pallet_xcavate_whitelist::Role::ModuleBooker
        ));
        new_region_helper();

        // Step 2: Creator creates a module
        assert_ok!(RealXEducation::create_module(
            RuntimeOrigin::signed(creator.clone()),
            3,
            module_amount,
            bvec![1, 2, 3]
        ));

        // Step 3: Sponsor purchases 40 tokens
        assert_ok!(RealXEducation::sponsor_module(
            RuntimeOrigin::signed(sponsor.clone()),
            0,
            purchase_amount,
            1984
        ));

        // Step 4: School cancels bookings up to the max allowed cancellations
        for i in 0..(<Test as Config>::MaxCleanupPerCall::get() + 10) {
            assert_ok!(RealXEducation::book_module(
                RuntimeOrigin::signed(school.clone()),
                0,
                0,
                bvec![4, 5, 6]
            ));

            assert_ok!(RealXEducation::cancel_booking(
                RuntimeOrigin::signed(school.clone()),
                0,
                i.into(),
            ));
        }

        // Pre-state
        assert_eq!(BookingCancellationCounter::<Test>::get(&school), 60);
        let current_block = System::block_number();
        assert!(SchoolCancellations::<Test>::contains_key(&school, (current_block, 50)));
        assert!(SchoolCancellations::<Test>::contains_key(&school, (current_block, 51)));

        // Move forward blocks beyond the cancellation window
        let block_number =
            frame_system::Pallet::<Test>::block_number() + CancellationWindow::get() + 1;
        run_to_block(block_number);

        // Step 5: Clear the cancellations older than the window
        assert_ok!(RealXEducation::clear_old_cancellations(RuntimeOrigin::signed(school.clone()),));

        // 1. Cancellation storage cleared
        assert_eq!(BookingCancellationCounter::<Test>::get(&school), 10);
        assert!(!SchoolCancellations::<Test>::contains_key(&school, (current_block, 49)));
        assert!(SchoolCancellations::<Test>::contains_key(&school, (current_block, 50)));

        // 2. Event emitted
        System::assert_last_event(
            Event::OldCancellationsCleared {
                school,
                removed: <Test as Config>::MaxCleanupPerCall::get(),
            }
            .into(),
        );
    });
}

// reclaim_unused_sponsorship tests

#[test]
fn reclaim_unused_sponsorship_works1() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);

        let creator: AccountId = [1; 32].into();
        let sponsor: AccountId = [2; 32].into();
        let school: AccountId = [3; 32].into();

        let module_amount = 100u32;
        let purchase_amount = 30u32;

        // Step 1: Setup permissions
        assert_ok!(XcavateWhitelist::add_admin(RuntimeOrigin::root(), [20; 32].into(),));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [1; 32].into(),
            pallet_xcavate_whitelist::Role::ModuleCreator
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [2; 32].into(),
            pallet_xcavate_whitelist::Role::ModuleSponsor
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [3; 32].into(),
            pallet_xcavate_whitelist::Role::ModuleBooker
        ));
        new_region_helper();

        // Step 2: Creator creates a module
        assert_ok!(RealXEducation::create_module(
            RuntimeOrigin::signed(creator.clone()),
            3,
            module_amount,
            bvec![1, 2, 3]
        ));

        let module = ModuleInfo::<Test>::get(0).unwrap();
        let asset_id = module.asset_id.into();

        // Step 3: Sponsor purchases 30 tokens
        assert_ok!(RealXEducation::sponsor_module(
            RuntimeOrigin::signed(sponsor.clone()),
            0,
            purchase_amount,
            1984
        ));

        // Step 4: School books a module
        assert_ok!(RealXEducation::book_module(
            RuntimeOrigin::signed(school),
            0,
            0,
            bvec![4, 5, 6]
        ));

        // Pre-checks
        let multiplier = 10u128
            .checked_pow(AssetsMetadataWrapper::get_decimals(1984u32).unwrap().into())
            .unwrap();
        assert_eq!(EducationAssets::balance(asset_id, &creator), 70);
        assert_eq!(EducationAssets::balance(asset_id, &sponsor), 29);
        assert_eq!(
            AssetsHolder::total_balance_on_hold(1984u32.into(), &sponsor),
            37_500 * multiplier
        );
        assert_eq!(ForeignAssets::balance(1984, &sponsor), 22_500 * multiplier);
        let module = ModuleInfo::<Test>::get(0).unwrap();
        assert_eq!(module.sponsor_allocation, 70);
        assert_eq!(module.school_allocation, 29);
        assert_eq!(SponsoredModules::<Test>::get(0, 0).unwrap().amount, 29);

        // Move forward blocks beyond the sponsorship window
        let block_number =
            frame_system::Pallet::<Test>::block_number() + SponsorshipWindow::get() + 1;
        run_to_block(block_number);

        // Step 5: Sponsor reclaims unused sponsorship
        assert_ok!(RealXEducation::reclaim_unused_sponsorship(
            RuntimeOrigin::signed(sponsor.clone()),
            0,
            0,
            10
        ));

        // 1. Sponsor balance updated
        assert_eq!(EducationAssets::balance(asset_id, &sponsor), 19);
        assert_eq!(
            AssetsHolder::total_balance_on_hold(1984u32.into(), &sponsor),
            25_000 * multiplier
        );
        assert_eq!(ForeignAssets::balance(1984, &sponsor), 35_000 * multiplier);

        // 2. Storage updated
        let updated_module = ModuleInfo::<Test>::get(0).unwrap();
        assert_eq!(updated_module.sponsor_allocation, 80);
        assert_eq!(updated_module.school_allocation, 19);
        assert_eq!(SponsoredModules::<Test>::get(0, 0).unwrap().amount, 19);

        // 3. Event emitted
        System::assert_last_event(
            Event::UnsponsoredTokensWithdrawn {
                module_id: 0,
                sponsor,
                amount: 10,
                payment_asset: 1984,
                refunded: 12_500 * multiplier,
            }
            .into(),
        );
    });
}

#[test]
fn reclaim_unused_sponsorship_works2() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);

        let creator: AccountId = [1; 32].into();
        let sponsor: AccountId = [2; 32].into();
        let school: AccountId = [3; 32].into();

        let module_amount = 100u32;
        let purchase_amount = 30u32;

        // Step 1: Setup permissions
        assert_ok!(XcavateWhitelist::add_admin(RuntimeOrigin::root(), [20; 32].into(),));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [1; 32].into(),
            pallet_xcavate_whitelist::Role::ModuleCreator
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [2; 32].into(),
            pallet_xcavate_whitelist::Role::ModuleSponsor
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [3; 32].into(),
            pallet_xcavate_whitelist::Role::ModuleBooker
        ));
        new_region_helper();

        // Step 2: Creator creates a module
        assert_ok!(RealXEducation::create_module(
            RuntimeOrigin::signed(creator.clone()),
            3,
            module_amount,
            bvec![1, 2, 3]
        ));

        let module = ModuleInfo::<Test>::get(0).unwrap();
        let asset_id = module.asset_id.into();

        // Step 3: Sponsor purchases 30 tokens
        assert_ok!(RealXEducation::sponsor_module(
            RuntimeOrigin::signed(sponsor.clone()),
            0,
            purchase_amount,
            1984
        ));

        // Step 4: School books a module
        assert_ok!(RealXEducation::book_module(
            RuntimeOrigin::signed(school),
            0,
            0,
            bvec![4, 5, 6]
        ));

        // Pre-checks
        let multiplier = 10u128
            .checked_pow(AssetsMetadataWrapper::get_decimals(1984u32).unwrap().into())
            .unwrap();
        assert_eq!(EducationAssets::balance(asset_id, &creator), 70);
        assert_eq!(EducationAssets::balance(asset_id, &sponsor), 29);
        assert_eq!(
            AssetsHolder::total_balance_on_hold(1984u32.into(), &sponsor),
            37_500 * multiplier
        );
        assert_eq!(ForeignAssets::balance(1984, &sponsor), 22_500 * multiplier);
        let module = ModuleInfo::<Test>::get(0).unwrap();
        assert_eq!(module.sponsor_allocation, 70);
        assert_eq!(module.school_allocation, 29);
        assert!(SponsoredModules::<Test>::get(0, 0).is_some());

        // Move forward blocks beyond the sponsorship window
        let block_number =
            frame_system::Pallet::<Test>::block_number() + SponsorshipWindow::get() + 1;
        run_to_block(block_number);

        // Step 5: Sponsor reclaims unused sponsorship
        assert_ok!(RealXEducation::reclaim_unused_sponsorship(
            RuntimeOrigin::signed(sponsor.clone()),
            0,
            0,
            29
        ));

        // 1. Sponsor balance updated
        assert_eq!(EducationAssets::balance(asset_id, &sponsor), 0);
        assert_eq!(
            AssetsHolder::total_balance_on_hold(1984u32.into(), &sponsor),
            1_250 * multiplier
        );
        assert_eq!(ForeignAssets::balance(1984, &sponsor), 58_750 * multiplier);

        // 2. Storage updated
        let updated_module = ModuleInfo::<Test>::get(0).unwrap();
        assert_eq!(updated_module.sponsor_allocation, 99);
        assert_eq!(updated_module.school_allocation, 0);
        assert!(SponsoredModules::<Test>::get(0, 0).is_none());

        // 3. Event emitted
        System::assert_last_event(
            Event::UnsponsoredTokensWithdrawn {
                module_id: 0,
                sponsor,
                amount: 29,
                payment_asset: 1984,
                refunded: 36_250 * multiplier,
            }
            .into(),
        );
    });
}

#[test]
fn reclaim_unused_sponsorship_fails() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);
        // No Permission fails
        assert_noop!(
            RealXEducation::reclaim_unused_sponsorship(
                RuntimeOrigin::signed([2; 32].into()),
                0,
                0,
                1
            ),
            BadOrigin
        );

        assert_ok!(XcavateWhitelist::add_admin(RuntimeOrigin::root(), [20; 32].into(),));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [1; 32].into(),
            pallet_xcavate_whitelist::Role::ModuleCreator
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [2; 32].into(),
            pallet_xcavate_whitelist::Role::ModuleSponsor
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [10; 32].into(),
            pallet_xcavate_whitelist::Role::ModuleSponsor
        ));
        new_region_helper();

        // Fails if there is no module
        assert_noop!(
            RealXEducation::reclaim_unused_sponsorship(
                RuntimeOrigin::signed([2; 32].into()),
                0,
                0,
                1
            ),
            Error::<Test>::ModuleNotAvailable
        );

        assert_ok!(RealXEducation::create_module(
            RuntimeOrigin::signed([1; 32].into()),
            3,
            100,
            bvec![1, 2, 3]
        ));

        assert_ok!(RealXEducation::sponsor_module(
            RuntimeOrigin::signed([2; 32].into()),
            0,
            30,
            1984
        ));

        // Fails if sponsor has not sponsored the module
        assert_noop!(
            RealXEducation::reclaim_unused_sponsorship(
                RuntimeOrigin::signed([2; 32].into()),
                0,
                1,
                30
            ),
            Error::<Test>::NoFundedModulesFromSponsor
        );

        // Fails if the caller is not the sponsor
        assert_noop!(
            RealXEducation::reclaim_unused_sponsorship(
                RuntimeOrigin::signed([10; 32].into()),
                0,
                0,
                30
            ),
            Error::<Test>::NoPermission
        );

        // Fails if sponsorship window not expired
        assert_noop!(
            RealXEducation::reclaim_unused_sponsorship(
                RuntimeOrigin::signed([2; 32].into()),
                0,
                0,
                30
            ),
            Error::<Test>::SponsorshipWindowNotExpired
        );

        let block_number =
            frame_system::Pallet::<Test>::block_number() + SponsorshipWindow::get() + 1;
        run_to_block(block_number);

        // Fails if amount is zero
        assert_noop!(
            RealXEducation::reclaim_unused_sponsorship(
                RuntimeOrigin::signed([2; 32].into()),
                0,
                0,
                0
            ),
            Error::<Test>::AmountCannotBeZero
        );

        // Fails if amount is higher than the available sponsored modules
        assert_noop!(
            RealXEducation::reclaim_unused_sponsorship(
                RuntimeOrigin::signed([2; 32].into()),
                0,
                0,
                50
            ),
            Error::<Test>::NotEnoughTokenAvailable
        );
    });
}

// cancel_claim tests

#[test]
fn cancel_claim_works() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);

        let creator: AccountId = [1; 32].into();
        let sponsor: AccountId = [2; 32].into();
        let school: AccountId = [3; 32].into();
        let lecturer: AccountId = [4; 32].into();

        let module_amount = 100u32;
        let purchase_amount = 40u32;

        // Step 1: Setup permissions
        assert_ok!(XcavateWhitelist::add_admin(RuntimeOrigin::root(), [20; 32].into(),));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [1; 32].into(),
            pallet_xcavate_whitelist::Role::ModuleCreator
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [2; 32].into(),
            pallet_xcavate_whitelist::Role::ModuleSponsor
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [3; 32].into(),
            pallet_xcavate_whitelist::Role::ModuleBooker
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [4; 32].into(),
            pallet_xcavate_whitelist::Role::ModuleDeliverer
        ));
        new_region_helper();
        assert_ok!(RealXEducation::register_module_deliverer(RuntimeOrigin::signed(
            [4; 32].into()
        )));

        // Step 2: Creator creates a module
        assert_ok!(RealXEducation::create_module(
            RuntimeOrigin::signed(creator.clone()),
            3,
            module_amount,
            bvec![1, 2, 3]
        ));

        // Step 3: Sponsor purchases 40 tokens
        assert_ok!(RealXEducation::sponsor_module(
            RuntimeOrigin::signed(sponsor.clone()),
            0,
            purchase_amount,
            1984
        ));

        // Step 4: School books a module
        assert_ok!(RealXEducation::book_module(
            RuntimeOrigin::signed(school.clone()),
            0,
            0,
            bvec![4, 5, 6]
        ));

        // Step 5: University student claims a module
        assert_ok!(RealXEducation::claim_booking(RuntimeOrigin::signed(lecturer.clone()), 0, 0,));

        // Pre-state
        assert_eq!(
            Bookings::<Test>::get::<u32, u64>(0, 0).unwrap().lecturer,
            Some(lecturer.clone())
        );
        assert_eq!(ModuleDeliverer::<Test>::get(&lecturer).unwrap().active_claims, 1);
        assert_eq!(ModuleDeliverer::<Test>::get(&lecturer).unwrap().active_strikes, 0);
        assert_eq!(ModuleInfo::<Test>::get(0).unwrap().university_student_allocation, 0);
        assert_eq!(EducationAssets::balance(0, &school), 1);
        assert_eq!(EducationAssets::balance(0, &lecturer), 0);

        // Step 6: Lecturer cancels claim
        assert_ok!(RealXEducation::cancel_claim(RuntimeOrigin::signed(lecturer.clone()), 0, 0,));

        // 1. Locked token released
        assert_eq!(Bookings::<Test>::get::<u32, u64>(0, 0).unwrap().lecturer, None);

        // 2. Booking storage removed
        assert_eq!(ModuleDeliverer::<Test>::get(&lecturer).unwrap().active_claims, 0);
        assert_eq!(ModuleDeliverer::<Test>::get(&lecturer).unwrap().active_strikes, 1);
        assert_eq!(ModuleInfo::<Test>::get(0).unwrap().university_student_allocation, 1);

        // 3. school balance updated
        assert_eq!(EducationAssets::balance(0, &school), 1);
        assert_eq!(EducationAssets::balance(0, &lecturer), 0);

        // 4. Event emitted
        System::assert_last_event(
            Event::ClaimCancelled { lecturer, module_id: 0, booking_id: 0, active_strikes: 1 }
                .into(),
        );
    });
}

#[test]
fn cancel_claim_slashing_works() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);

        let creator: AccountId = [1; 32].into();
        let sponsor: AccountId = [2; 32].into();
        let school: AccountId = [3; 32].into();
        let lecturer: AccountId = [4; 32].into();

        let module_amount = 100u32;
        let purchase_amount = 40u32;

        // Step 1: Setup permissions
        assert_ok!(XcavateWhitelist::add_admin(RuntimeOrigin::root(), [20; 32].into(),));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [1; 32].into(),
            pallet_xcavate_whitelist::Role::ModuleCreator
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [2; 32].into(),
            pallet_xcavate_whitelist::Role::ModuleSponsor
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [3; 32].into(),
            pallet_xcavate_whitelist::Role::ModuleBooker
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [4; 32].into(),
            pallet_xcavate_whitelist::Role::ModuleDeliverer
        ));
        new_region_helper();
        assert_ok!(RealXEducation::register_module_deliverer(RuntimeOrigin::signed(
            [4; 32].into()
        )));

        // Step 2: Creator creates a module
        assert_ok!(RealXEducation::create_module(
            RuntimeOrigin::signed(creator.clone()),
            3,
            module_amount,
            bvec![1, 2, 3]
        ));

        // Step 3: Sponsor purchases 40 tokens
        assert_ok!(RealXEducation::sponsor_module(
            RuntimeOrigin::signed(sponsor.clone()),
            0,
            purchase_amount,
            1984
        ));

        // Step 4: School books a module
        assert_ok!(RealXEducation::book_module(
            RuntimeOrigin::signed(school.clone()),
            0,
            0,
            bvec![4, 5, 6]
        ));

        // Step 5: Lecturer cancels claims up to the max allowed strikes
        for _ in 0..(<Test as Config>::MaxAllowedStrikes::get() - 1) {
            assert_ok!(RealXEducation::claim_booking(
                RuntimeOrigin::signed(lecturer.clone()),
                0,
                0,
            ));
            assert_ok!(
                RealXEducation::cancel_claim(RuntimeOrigin::signed(lecturer.clone()), 0, 0,)
            );
        }

        // Step 6: University student claims a module
        assert_ok!(RealXEducation::claim_booking(RuntimeOrigin::signed(lecturer.clone()), 0, 0,));

        // Pre-state
        assert_eq!(
            Bookings::<Test>::get::<u32, u64>(0, 0).unwrap().lecturer,
            Some(lecturer.clone())
        );
        assert_eq!(ModuleDeliverer::<Test>::get(&lecturer).unwrap().active_claims, 1);
        assert_eq!(ModuleDeliverer::<Test>::get(&lecturer).unwrap().active_strikes, 2);
        assert_eq!(ModuleDeliverer::<Test>::get(&lecturer).unwrap().deposit, 100);
        assert_eq!(Balances::free_balance(&lecturer), 4_900);
        assert_eq!(
            Balances::balance_on_hold(&HoldReason::ModuleDelivererReserve.into(), &lecturer),
            100
        );
        assert_eq!(Balances::total_issuance(), 305_000);

        // Step 7: Lecturer cancels claim
        assert_ok!(RealXEducation::cancel_claim(RuntimeOrigin::signed(lecturer.clone()), 0, 0,));

        // 1. Locked token released
        assert_eq!(Bookings::<Test>::get::<u32, u64>(0, 0).unwrap().lecturer, None);

        // 2. Booking storage removed
        assert_eq!(ModuleDeliverer::<Test>::get(&lecturer).unwrap().active_claims, 0);
        assert_eq!(ModuleDeliverer::<Test>::get(&lecturer).unwrap().active_strikes, 3);
        assert_eq!(ModuleDeliverer::<Test>::get(&lecturer).unwrap().deposit, 90);

        // 3. Module deliverer slashed balance
        assert_eq!(Balances::free_balance(&lecturer), 4_900);
        assert_eq!(
            Balances::balance_on_hold(&HoldReason::ModuleDelivererReserve.into(), &lecturer),
            90
        );
        assert_eq!(Balances::total_issuance(), 304_990);

        // 4. Event emitted
        System::assert_last_event(
            Event::ClaimCancelled { lecturer, module_id: 0, booking_id: 0, active_strikes: 3 }
                .into(),
        );
    });
}

#[test]
fn cancel_claim_fails() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);
        // No Permission fails
        assert_noop!(
            RealXEducation::cancel_claim(RuntimeOrigin::signed([4; 32].into()), 0, 0),
            BadOrigin
        );

        assert_ok!(XcavateWhitelist::add_admin(RuntimeOrigin::root(), [20; 32].into(),));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [1; 32].into(),
            pallet_xcavate_whitelist::Role::ModuleCreator
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [2; 32].into(),
            pallet_xcavate_whitelist::Role::ModuleSponsor
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [3; 32].into(),
            pallet_xcavate_whitelist::Role::ModuleBooker
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [4; 32].into(),
            pallet_xcavate_whitelist::Role::ModuleDeliverer
        ));
        new_region_helper();

        // Fails if there is no module
        assert_noop!(
            RealXEducation::cancel_claim(RuntimeOrigin::signed([4; 32].into()), 0, 0),
            Error::<Test>::ModuleNotAvailable
        );

        assert_ok!(RealXEducation::create_module(
            RuntimeOrigin::signed([1; 32].into()),
            3,
            100,
            bvec![1, 2, 3]
        ));

        assert_ok!(RealXEducation::sponsor_module(
            RuntimeOrigin::signed([2; 32].into()),
            0,
            30,
            1984
        ));

        // Fails if the booking does not exist
        assert_noop!(
            RealXEducation::cancel_claim(RuntimeOrigin::signed([4; 32].into()), 0, 0),
            Error::<Test>::NoBookingAvailable
        );

        assert_ok!(RealXEducation::book_module(
            RuntimeOrigin::signed([3; 32].into()),
            0,
            0,
            bvec![4, 5, 6]
        ));

        // Fails if the lecturer is not the assigned lecturer
        assert_noop!(
            RealXEducation::cancel_claim(RuntimeOrigin::signed([4; 32].into()), 0, 0),
            Error::<Test>::NoPermission
        );
    });
}

// register_module_deliverer tests

#[test]
fn register_module_deliverer_works() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);

        let lecturer: AccountId = [4; 32].into();

        // Step 1: Setup permissions
        assert_ok!(XcavateWhitelist::add_admin(RuntimeOrigin::root(), [20; 32].into(),));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [1; 32].into(),
            pallet_xcavate_whitelist::Role::ModuleCreator
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [4; 32].into(),
            pallet_xcavate_whitelist::Role::ModuleDeliverer
        ));

        // Pre-state
        assert!(ModuleDeliverer::<Test>::get(&lecturer).is_none());
        assert_eq!(Balances::free_balance(&lecturer), 5_000);
        assert_eq!(
            Balances::balance_on_hold(&HoldReason::ModuleDelivererReserve.into(), &lecturer),
            0
        );

        // Step 2: Register module deliverer
        assert_ok!(RealXEducation::register_module_deliverer(RuntimeOrigin::signed(
            lecturer.clone()
        )));

        // 1. Module deliverer storage set
        assert_eq!(ModuleDeliverer::<Test>::get(&lecturer).unwrap().deposit, 100);

        // 2. Locked token balance
        assert_eq!(Balances::free_balance(&lecturer), 4_900);
        assert_eq!(
            Balances::balance_on_hold(&HoldReason::ModuleDelivererReserve.into(), &lecturer),
            100
        );

        // 3. Event emitted
        System::assert_last_event(
            Event::ModuleDelivererRegistered { module_deliverer: lecturer, deposit: 100 }.into(),
        );
    });
}

#[test]
fn register_module_deliverer_increase_deposit_works() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);

        let creator: AccountId = [1; 32].into();
        let sponsor: AccountId = [2; 32].into();
        let school: AccountId = [3; 32].into();
        let lecturer: AccountId = [4; 32].into();

        let module_amount = 100u32;
        let purchase_amount = 30u32;

        // Step 1: Setup permissions
        assert_ok!(XcavateWhitelist::add_admin(RuntimeOrigin::root(), [20; 32].into(),));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [1; 32].into(),
            pallet_xcavate_whitelist::Role::ModuleCreator
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [2; 32].into(),
            pallet_xcavate_whitelist::Role::ModuleSponsor
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [3; 32].into(),
            pallet_xcavate_whitelist::Role::ModuleBooker
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [4; 32].into(),
            pallet_xcavate_whitelist::Role::ModuleDeliverer
        ));
        new_region_helper();
        assert_ok!(RealXEducation::register_module_deliverer(RuntimeOrigin::signed(
            [4; 32].into()
        )));

        // Step 2: Creator creates a module
        assert_ok!(RealXEducation::create_module(
            RuntimeOrigin::signed(creator),
            3,
            module_amount,
            bvec![1, 2, 3]
        ));

        // Step 3: Sponsor purchases 30 tokens
        assert_ok!(RealXEducation::sponsor_module(
            RuntimeOrigin::signed(sponsor),
            0,
            purchase_amount,
            1984
        ));

        // Step 4: School books a module
        assert_ok!(RealXEducation::book_module(
            RuntimeOrigin::signed(school),
            0,
            0,
            bvec![4, 5, 6]
        ));

        // Step 5: Claim booking and cancel claim multiple times to slash deposit
        for _ in 0..10 {
            assert_ok!(RealXEducation::claim_booking(
                RuntimeOrigin::signed(lecturer.clone()),
                0,
                0,
            ));

            assert_ok!(
                RealXEducation::cancel_claim(RuntimeOrigin::signed(lecturer.clone()), 0, 0,)
            );
        }

        // Pre-state
        assert_eq!(Balances::free_balance(&lecturer), 4_900);
        assert_eq!(
            Balances::balance_on_hold(&HoldReason::ModuleDelivererReserve.into(), &lecturer),
            20
        );

        // Step 6: Register module deliverer to deposit
        assert_ok!(RealXEducation::register_module_deliverer(RuntimeOrigin::signed(
            lecturer.clone()
        )));

        // 1. Module deliverer storage set
        assert_eq!(ModuleDeliverer::<Test>::get(&lecturer).unwrap().deposit, 100);

        // 2. Locked token balance
        assert_eq!(Balances::free_balance(&lecturer), 4_820);
        assert_eq!(
            Balances::balance_on_hold(&HoldReason::ModuleDelivererReserve.into(), &lecturer),
            100
        );

        // 3. Event emitted
        System::assert_last_event(
            Event::ModuleDelivererDepositIncreased {
                module_deliverer: lecturer,
                old_deposit: 20,
                new_deposit: 100,
            }
            .into(),
        );
    });
}

#[test]
fn register_module_deliverer_fails() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);
        // No Permission fails
        assert_noop!(
            RealXEducation::register_module_deliverer(RuntimeOrigin::signed([4; 32].into())),
            BadOrigin
        );

        assert_ok!(XcavateWhitelist::add_admin(RuntimeOrigin::root(), [20; 32].into(),));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [4; 32].into(),
            pallet_xcavate_whitelist::Role::ModuleDeliverer
        ));

        assert_ok!(RealXEducation::register_module_deliverer(RuntimeOrigin::signed(
            [4; 32].into()
        )));
    });
}

// unregister_module_deliverer tests

#[test]
fn unregister_module_deliverer_works() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);

        let lecturer: AccountId = [4; 32].into();

        // Step 1: Setup permissions
        assert_ok!(XcavateWhitelist::add_admin(RuntimeOrigin::root(), [20; 32].into(),));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [1; 32].into(),
            pallet_xcavate_whitelist::Role::ModuleCreator
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [4; 32].into(),
            pallet_xcavate_whitelist::Role::ModuleDeliverer
        ));

        // Step 2: Register module deliverer
        assert_ok!(RealXEducation::register_module_deliverer(RuntimeOrigin::signed(
            lecturer.clone()
        )));

        // Pre-state
        assert!(ModuleDeliverer::<Test>::get(&lecturer).is_some());
        assert_eq!(Balances::free_balance(&lecturer), 4_900);
        assert_eq!(
            Balances::balance_on_hold(&HoldReason::ModuleDelivererReserve.into(), &lecturer),
            100
        );

        // Step 2: Unregister module deliverer
        assert_ok!(RealXEducation::unregister_module_deliverer(RuntimeOrigin::signed(
            lecturer.clone()
        )));

        // 1. Module deliverer storage set
        assert!(ModuleDeliverer::<Test>::get(&lecturer).is_none());

        // 2. Released token balance
        assert_eq!(Balances::free_balance(&lecturer), 5_000);
        assert_eq!(
            Balances::balance_on_hold(&HoldReason::ModuleDelivererReserve.into(), &lecturer),
            0
        );

        // 3. ModuleDeliverer role got removed
        assert_eq!(XcavateWhitelist::has_role(&lecturer, Role::ModuleDeliverer), false);

        // 4. Event emitted
        System::assert_last_event(
            Event::ModuleDelivererUnregistered { module_deliverer: lecturer }.into(),
        );
    });
}

#[test]
fn unregister_module_deliverer_fails() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);
        // No Permission fails
        assert_noop!(
            RealXEducation::unregister_module_deliverer(RuntimeOrigin::signed([4; 32].into())),
            BadOrigin
        );

        assert_ok!(XcavateWhitelist::add_admin(RuntimeOrigin::root(), [20; 32].into(),));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [1; 32].into(),
            pallet_xcavate_whitelist::Role::ModuleCreator
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [2; 32].into(),
            pallet_xcavate_whitelist::Role::ModuleSponsor
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [3; 32].into(),
            pallet_xcavate_whitelist::Role::ModuleBooker
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [4; 32].into(),
            pallet_xcavate_whitelist::Role::ModuleDeliverer
        ));
        new_region_helper();

        assert_ok!(RealXEducation::create_module(
            RuntimeOrigin::signed([1; 32].into()),
            3,
            100,
            bvec![1, 2, 3]
        ));

        assert_ok!(RealXEducation::sponsor_module(
            RuntimeOrigin::signed([2; 32].into()),
            0,
            30,
            1984
        ));

        assert_ok!(RealXEducation::book_module(
            RuntimeOrigin::signed([3; 32].into()),
            0,
            0,
            bvec![4, 5, 6]
        ));

        // Fails if the module deliverer is not registered
        assert_noop!(
            RealXEducation::unregister_module_deliverer(RuntimeOrigin::signed([4; 32].into())),
            Error::<Test>::ModuleDelivererNotRegistered
        );

        assert_ok!(RealXEducation::register_module_deliverer(RuntimeOrigin::signed(
            [4; 32].into()
        )));

        assert_ok!(RealXEducation::claim_booking(RuntimeOrigin::signed([4; 32].into()), 0, 0));

        // Fails if the module deliverer has active claims
        assert_noop!(
            RealXEducation::unregister_module_deliverer(RuntimeOrigin::signed([4; 32].into())),
            Error::<Test>::ModuleDelivererStillActive
        );
    });
}
