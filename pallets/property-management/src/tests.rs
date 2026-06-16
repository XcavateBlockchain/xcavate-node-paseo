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
use frame_support::BoundedVec;
use frame_support::{
    assert_noop, assert_ok,
    traits::{fungible::InspectHold, fungibles::InspectFreeze, OnFinalize, OnInitialize},
};

use primitives::MarketplaceFreezeReason;

use crate::{
    AssetLettingProposal, HoldReason, LettingAgentProposal, LettingInfo, LettingStorage,
    OngoingLettingAgentVoting, OwnerCheckpoints, PropertyIncome, ProposalCounter,
    ResignationNotices, ResignationQueue, UserLettingAgentVote, VoteRecord,
};

use sp_runtime::{traits::BadOrigin, Permill, TokenError};

use pallet_marketplace::types::LegalProperty;

use pallet_regions::RegionIdentifier;

use pallet_real_world_asset::Error as RealWorldAssetError;

macro_rules! bvec {
	($( $x:tt )*) => {
		vec![$( $x )*].try_into().unwrap()
	}
}

fn run_to_block(n: u64) {
    while System::block_number() < n {
        if System::block_number() > 0 {
            PropertyManagement::on_finalize(System::block_number());
            System::on_finalize(System::block_number());
        }
        System::reset_events();
        System::set_block_number(System::block_number() + 1);
        System::on_initialize(System::block_number());
        PropertyManagement::on_initialize(System::block_number());
    }
}

fn new_region_helper() {
    assert_ok!(XcavateWhitelist::assign_role(
        RuntimeOrigin::signed([20; 32].into()),
        [6; 32].into(),
        pallet_xcavate_whitelist::Role::RegionalOperator
    ));
    assert_ok!(Regions::propose_new_region(
        RuntimeOrigin::signed([6; 32].into()),
        RegionIdentifier::Japan
    ));
    assert_ok!(Regions::vote_on_region_proposal(
        RuntimeOrigin::signed([6; 32].into()),
        3,
        pallet_regions::Vote::Yes,
        1_000_000
    ));
    run_to_block(31);
    assert_ok!(Regions::bid_on_region(RuntimeOrigin::signed([6; 32].into()), 3, 100_000));
    run_to_block(61);
    assert_ok!(Regions::create_new_region(
        RuntimeOrigin::signed([6; 32].into()),
        3,
        30,
        Permill::from_percent(3)
    ));
}

fn lawyer_process_helper(
    real_estate_developer: AccountId,
    listing_id: u32,
    accounts: Vec<(AccountId, u32)>,
) {
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
    finalize_property_helper(real_estate_developer, listing_id, accounts);
}

fn finalize_property_helper(
    real_estate_developer: AccountId,
    listing_id: u32,
    accounts: Vec<(AccountId, u32)>,
) {
    assert_ok!(Marketplace::lawyer_claim_property(
        RuntimeOrigin::signed([10; 32].into()),
        listing_id,
        LegalProperty::RealEstateDeveloperSide,
        400,
    ));
    assert_ok!(Marketplace::approve_developer_lawyer(
        RuntimeOrigin::signed(real_estate_developer),
        listing_id,
        true
    ));
    assert_ok!(Marketplace::lawyer_claim_property(
        RuntimeOrigin::signed([11; 32].into()),
        listing_id,
        LegalProperty::SpvSide,
        400,
    ));
    for account in &accounts {
        assert_ok!(Marketplace::vote_on_spv_lawyer(
            RuntimeOrigin::signed(account.0.clone()),
            listing_id,
            pallet_marketplace::types::Vote::Yes,
            account.1,
        ));
    }
    let expiry = frame_system::Pallet::<Test>::block_number() + LawyerVotingDuration::get();
    run_to_block(expiry);
    assert_ok!(Marketplace::finalize_spv_lawyer(
        RuntimeOrigin::signed(accounts[0].0.clone()),
        listing_id,
    ));
    assert_ok!(Marketplace::lawyer_confirm_documents(
        RuntimeOrigin::signed([10; 32].into()),
        listing_id,
        true,
    ));
    assert_ok!(Marketplace::lawyer_confirm_documents(
        RuntimeOrigin::signed([11; 32].into()),
        listing_id,
        true,
    ));
}

// add_letting_agent tests

#[test]
fn add_letting_agent_works() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);
        assert_ok!(XcavateWhitelist::add_admin(RuntimeOrigin::root(), [20; 32].into(),));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [8; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [6; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [0; 32].into(),
            pallet_xcavate_whitelist::Role::LettingAgent
        ));
        new_region_helper();
        assert_ok!(Regions::create_new_location(
            RuntimeOrigin::signed([6; 32].into()),
            3,
            bvec![10, 10]
        ));
        assert_ok!(PropertyManagement::add_letting_agent(
            RuntimeOrigin::signed([0; 32].into()),
            3,
            bvec![10, 10],
        ));
        assert_eq!(LettingInfo::<Test>::get::<AccountId>([0; 32].into()).is_some(), true);
        let location: BoundedVec<u8, Postcode> = bvec![10, 10];
        let letting_info = LettingInfo::<Test>::get::<AccountId>([0; 32].into()).unwrap();
        let location_info = letting_info.locations.get(&location).unwrap();
        assert_eq!(location_info.assigned_properties, 0);
        assert_eq!(location_info.deposit, 1_000);
        assert_eq!(Balances::free_balance(&([0; 32].into())), 19_999_000);
    });
}

#[test]
fn add_letting_agent_works2() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);
        // Set up roles and region
        assert_ok!(XcavateWhitelist::add_admin(RuntimeOrigin::root(), [20; 32].into(),));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [8; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [6; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [0; 32].into(),
            pallet_xcavate_whitelist::Role::LettingAgent
        ));
        new_region_helper();
        assert_ok!(Regions::create_new_location(
            RuntimeOrigin::signed([6; 32].into()),
            3,
            bvec![10, 10]
        ));
        assert_ok!(Regions::create_new_location(
            RuntimeOrigin::signed([6; 32].into()),
            3,
            bvec![11, 10]
        ));

        // Add letting agent to both locations
        assert_ok!(PropertyManagement::add_letting_agent(
            RuntimeOrigin::signed([0; 32].into()),
            3,
            bvec![10, 10],
        ));
        assert_eq!(Balances::free_balance(&([0; 32].into())), 19_999_000);
        assert_ok!(PropertyManagement::add_letting_agent(
            RuntimeOrigin::signed([0; 32].into()),
            3,
            bvec![11, 10],
        ));

        // Verify letting agent info
        assert_eq!(LettingInfo::<Test>::get::<AccountId>([0; 32].into()).is_some(), true);
        let location: BoundedVec<u8, Postcode> = bvec![10, 10];
        let letting_info = LettingInfo::<Test>::get::<AccountId>([0; 32].into()).unwrap();
        let location_info = letting_info.locations.get(&location).unwrap();
        assert_eq!(location_info.assigned_properties, 0);
        assert_eq!(location_info.deposit, 1_000);
        let location_info_2 = letting_info.locations.get(&bvec![11, 10]).unwrap();
        assert_eq!(location_info_2.assigned_properties, 0);
        assert_eq!(location_info_2.deposit, 1_000);
        assert_eq!(Balances::free_balance(&([0; 32].into())), 19_998_000);
        assert_eq!(
            Balances::balance_on_hold(&HoldReason::LettingAgent.into(), &([0; 32].into())),
            2_000
        );
    });
}

#[test]
fn add_letting_agent_fails() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);
        assert_ok!(XcavateWhitelist::add_admin(RuntimeOrigin::root(), [20; 32].into(),));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [8; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [6; 32].into(),
            pallet_xcavate_whitelist::Role::RegionalOperator
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [0; 32].into(),
            pallet_xcavate_whitelist::Role::LettingAgent
        ));
        assert_noop!(
            PropertyManagement::add_letting_agent(
                RuntimeOrigin::signed([0; 32].into()),
                0,
                bvec![10, 10],
            ),
            Error::<Test>::RegionUnknown
        );
        assert_ok!(XcavateWhitelist::remove_role(
            RuntimeOrigin::signed([20; 32].into()),
            [6; 32].into(),
            pallet_xcavate_whitelist::Role::RegionalOperator
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [6; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));
        new_region_helper();
        assert_noop!(
            PropertyManagement::add_letting_agent(
                RuntimeOrigin::signed([0; 32].into()),
                3,
                bvec![10, 10],
            ),
            Error::<Test>::LocationUnknown
        );
        assert_ok!(Regions::create_new_location(
            RuntimeOrigin::signed([6; 32].into()),
            3,
            bvec![10, 10]
        ));
        assert_ok!(PropertyManagement::add_letting_agent(
            RuntimeOrigin::signed([0; 32].into()),
            3,
            bvec![10, 10],
        ));
        assert_eq!(LettingInfo::<Test>::get::<AccountId>([0; 32].into()).is_some(), true);
        assert_noop!(
            PropertyManagement::add_letting_agent(
                RuntimeOrigin::signed([0; 32].into()),
                3,
                bvec![10, 10],
            ),
            Error::<Test>::LettingAgentInLocation
        );
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [5; 32].into(),
            pallet_xcavate_whitelist::Role::LettingAgent
        ));
        assert_noop!(
            PropertyManagement::add_letting_agent(
                RuntimeOrigin::signed([5; 32].into()),
                3,
                bvec![10, 10],
            ),
            TokenError::FundsUnavailable,
        );
    });
}

// remove_letting_agent tests

#[test]
fn remove_letting_agent_works() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);
        // Set up roles and region
        assert_ok!(XcavateWhitelist::add_admin(RuntimeOrigin::root(), [20; 32].into(),));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [8; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [6; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [0; 32].into(),
            pallet_xcavate_whitelist::Role::LettingAgent
        ));
        new_region_helper();
        assert_ok!(Regions::create_new_location(
            RuntimeOrigin::signed([6; 32].into()),
            3,
            bvec![10, 10]
        ));

        // Add letting agent
        assert_ok!(PropertyManagement::add_letting_agent(
            RuntimeOrigin::signed([0; 32].into()),
            3,
            bvec![10, 10],
        ));
        assert_eq!(LettingInfo::<Test>::get::<AccountId>([0; 32].into()).is_some(), true);
        let location = bvec![10, 10];
        assert_eq!(
            LettingInfo::<Test>::get::<AccountId>([0; 32].into())
                .unwrap()
                .locations
                .get(&location)
                .clone()
                .unwrap()
                .assigned_properties,
            0
        );
        let mut letting_info = LettingInfo::<Test>::get::<AccountId>([0; 32].into()).unwrap();
        if let Some(location_info) = letting_info.locations.get_mut(&location) {
            location_info.assigned_properties = 5;
        }
        let account: AccountId = [0; 32].into();
        LettingInfo::<Test>::insert(account.clone(), letting_info);
        assert_eq!(
            LettingInfo::<Test>::get::<AccountId>(account)
                .unwrap()
                .locations
                .get(&location)
                .clone()
                .unwrap()
                .assigned_properties,
            5
        );
        assert_noop!(
            PropertyManagement::remove_letting_agent(
                RuntimeOrigin::signed([0; 32].into()),
                bvec![10, 10],
            ),
            Error::<Test>::LettingAgentActive
        );
        let mut letting_info = LettingInfo::<Test>::get::<AccountId>([0; 32].into()).unwrap();
        if let Some(location_info) = letting_info.locations.get_mut(&location) {
            location_info.assigned_properties = 0;
        }
        let account: AccountId = [0; 32].into();
        LettingInfo::<Test>::insert(account.clone(), letting_info);
        assert_eq!(
            Balances::balance_on_hold(&HoldReason::LettingAgent.into(), &([0; 32].into())),
            1_000
        );

        // Remove letting agent
        assert_ok!(PropertyManagement::remove_letting_agent(
            RuntimeOrigin::signed([0; 32].into()),
            bvec![10, 10],
        ));
        assert!(LettingInfo::<Test>::get::<AccountId>(account).is_none());
        assert_eq!(
            Balances::balance_on_hold(&HoldReason::LettingAgent.into(), &([0; 32].into())),
            0
        );
    });
}

#[test]
fn remove_letting_agent_works_2() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);
        // Set up roles and region
        assert_ok!(XcavateWhitelist::add_admin(RuntimeOrigin::root(), [20; 32].into(),));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [8; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [6; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [0; 32].into(),
            pallet_xcavate_whitelist::Role::LettingAgent
        ));
        new_region_helper();
        assert_ok!(Regions::create_new_location(
            RuntimeOrigin::signed([6; 32].into()),
            3,
            bvec![10, 10]
        ));
        assert_ok!(Regions::create_new_location(
            RuntimeOrigin::signed([6; 32].into()),
            3,
            bvec![11, 10]
        ));

        // Add letting agent
        assert_ok!(PropertyManagement::add_letting_agent(
            RuntimeOrigin::signed([0; 32].into()),
            3,
            bvec![10, 10],
        ));
        assert_ok!(PropertyManagement::add_letting_agent(
            RuntimeOrigin::signed([0; 32].into()),
            3,
            bvec![11, 10],
        ));
        assert_eq!(LettingInfo::<Test>::get::<AccountId>([0; 32].into()).is_some(), true);
        let location = bvec![10, 10];
        assert_eq!(
            LettingInfo::<Test>::get::<AccountId>([0; 32].into())
                .unwrap()
                .locations
                .get(&location)
                .clone()
                .unwrap()
                .assigned_properties,
            0
        );
        assert_eq!(
            Balances::balance_on_hold(&HoldReason::LettingAgent.into(), &([0; 32].into())),
            2_000
        );

        // Remove letting agent
        assert_ok!(PropertyManagement::remove_letting_agent(
            RuntimeOrigin::signed([0; 32].into()),
            bvec![10, 10],
        ));
        assert!(LettingInfo::<Test>::get::<AccountId>([0; 32].into())
            .unwrap()
            .locations
            .get(&location)
            .is_none());
        assert!(LettingInfo::<Test>::get::<AccountId>([0; 32].into())
            .unwrap()
            .locations
            .get(&bvec![11, 10])
            .is_some());
        assert_eq!(
            Balances::balance_on_hold(&HoldReason::LettingAgent.into(), &([0; 32].into())),
            1_000
        );
    });
}

#[test]
fn remove_letting_agent_fails() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);
        // Set up roles and region
        assert_ok!(XcavateWhitelist::add_admin(RuntimeOrigin::root(), [20; 32].into(),));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [8; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [6; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [0; 32].into(),
            pallet_xcavate_whitelist::Role::LettingAgent
        ));
        new_region_helper();
        assert_ok!(Regions::create_new_location(
            RuntimeOrigin::signed([6; 32].into()),
            3,
            bvec![10, 10]
        ));
        assert_ok!(Regions::create_new_location(
            RuntimeOrigin::signed([6; 32].into()),
            3,
            bvec![11, 10]
        ));

        // Letting agent process
        assert_noop!(
            PropertyManagement::remove_letting_agent(
                RuntimeOrigin::signed([0; 32].into()),
                bvec![10, 10],
            ),
            Error::<Test>::AgentNotFound
        );
        assert_ok!(PropertyManagement::add_letting_agent(
            RuntimeOrigin::signed([0; 32].into()),
            3,
            bvec![10, 10],
        ));
        assert_noop!(
            PropertyManagement::remove_letting_agent(
                RuntimeOrigin::signed([1; 32].into()),
                bvec![10, 10],
            ),
            BadOrigin
        );
        assert_noop!(
            PropertyManagement::remove_letting_agent(
                RuntimeOrigin::signed([0; 32].into()),
                bvec![11, 10],
            ),
            Error::<Test>::LettingAgentNotActiveInLocation
        );
        assert_eq!(LettingInfo::<Test>::get::<AccountId>([0; 32].into()).is_some(), true);
        let location = bvec![10, 10];
        assert_eq!(
            LettingInfo::<Test>::get::<AccountId>([0; 32].into())
                .unwrap()
                .locations
                .get(&location)
                .clone()
                .unwrap()
                .assigned_properties,
            0
        );
        let mut letting_info = LettingInfo::<Test>::get::<AccountId>([0; 32].into()).unwrap();
        if let Some(location_info) = letting_info.locations.get_mut(&location) {
            location_info.assigned_properties = 5;
        }
        let account: AccountId = [0; 32].into();
        LettingInfo::<Test>::insert(account.clone(), letting_info);
        assert_noop!(
            PropertyManagement::remove_letting_agent(
                RuntimeOrigin::signed([0; 32].into()),
                bvec![10, 10],
            ),
            Error::<Test>::LettingAgentActive
        );
    });
}

// letting_agent_claim_property tests

#[test]
fn letting_agent_claim_property_works() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);
        // Set up roles and region
        assert_ok!(XcavateWhitelist::add_admin(RuntimeOrigin::root(), [20; 32].into(),));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [8; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [6; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));
        new_region_helper();
        assert_ok!(Regions::create_new_location(
            RuntimeOrigin::signed([6; 32].into()),
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
            [4; 32].into(),
            pallet_xcavate_whitelist::Role::LettingAgent
        ));

        // Property listing process
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
        lawyer_process_helper([0; 32].into(), 0, vec![([1; 32].into(), 40), ([30; 32].into(), 30)]);

        // Letting agent process
        assert_ok!(PropertyManagement::add_letting_agent(
            RuntimeOrigin::signed([4; 32].into()),
            3,
            bvec![10, 10],
        ));
        assert_eq!(ProposalCounter::<Test>::get(), 0);
        assert_ok!(PropertyManagement::letting_agent_claim_property(
            RuntimeOrigin::signed([4; 32].into()),
            0
        ));
        assert_eq!(LettingAgentProposal::<Test>::get(0).unwrap().letting_agent, [4; 32].into());
        assert_eq!(
            OngoingLettingAgentVoting::<Test>::get(0).unwrap(),
            crate::VoteStats { yes_voting_power: 0, no_voting_power: 0, abstain_voting_power: 0 },
        );
        assert_eq!(AssetLettingProposal::<Test>::get(0).unwrap(), 0);
        assert_eq!(ProposalCounter::<Test>::get(), 1);
    });
}

#[test]
fn letting_agent_claim_property_fails() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);
        // Set up roles and region
        assert_ok!(XcavateWhitelist::add_admin(RuntimeOrigin::root(), [20; 32].into(),));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [8; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [6; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));
        new_region_helper();
        assert_ok!(Regions::create_new_location(
            RuntimeOrigin::signed([6; 32].into()),
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
            [4; 32].into(),
            pallet_xcavate_whitelist::Role::LettingAgent
        ));
        assert_noop!(
            PropertyManagement::letting_agent_claim_property(
                RuntimeOrigin::signed([4; 32].into()),
                0
            ),
            Error::<Test>::NoObjectFound
        );

        // Property listing process
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
        assert_ok!(PropertyManagement::add_letting_agent(
            RuntimeOrigin::signed([4; 32].into()),
            3,
            bvec![10, 10],
        ));
        assert_noop!(
            PropertyManagement::letting_agent_claim_property(
                RuntimeOrigin::signed([4; 32].into()),
                0
            ),
            RealWorldAssetError::<Test>::PropertyNotFinalized
        );
        lawyer_process_helper([0; 32].into(), 0, vec![([1; 32].into(), 40), ([30; 32].into(), 30)]);

        // Letting agent process
        assert_noop!(
            PropertyManagement::letting_agent_claim_property(
                RuntimeOrigin::signed([2; 32].into()),
                0
            ),
            BadOrigin
        );
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [2; 32].into(),
            pallet_xcavate_whitelist::Role::LettingAgent
        ));
        assert_noop!(
            PropertyManagement::letting_agent_claim_property(
                RuntimeOrigin::signed([2; 32].into()),
                0
            ),
            Error::<Test>::AgentNotFound
        );
        assert_ok!(Regions::create_new_location(
            RuntimeOrigin::signed([6; 32].into()),
            3,
            bvec![20, 10]
        ));
        assert_ok!(PropertyManagement::add_letting_agent(
            RuntimeOrigin::signed([2; 32].into()),
            3,
            bvec![20, 10],
        ));
        assert_noop!(
            PropertyManagement::letting_agent_claim_property(
                RuntimeOrigin::signed([2; 32].into()),
                0
            ),
            Error::<Test>::NoPermission
        );
        assert_ok!(PropertyManagement::letting_agent_claim_property(
            RuntimeOrigin::signed([4; 32].into()),
            0
        ));
        assert_noop!(
            PropertyManagement::letting_agent_claim_property(
                RuntimeOrigin::signed([4; 32].into()),
                0
            ),
            Error::<Test>::LettingAgentProposalOngoing
        );
        assert_ok!(PropertyManagement::vote_on_letting_agent(
            RuntimeOrigin::signed([1; 32].into()),
            0,
            crate::Vote::Yes,
            40
        ));
        assert_ok!(PropertyManagement::vote_on_letting_agent(
            RuntimeOrigin::signed([30; 32].into()),
            0,
            crate::Vote::Yes,
            30
        ));
        let expiry =
            frame_system::Pallet::<Test>::block_number() + LettingAgentVotingDuration::get();
        run_to_block(expiry);
        assert_ok!(PropertyManagement::finalize_letting_agent(
            RuntimeOrigin::signed([1; 32].into()),
            0,
        ));
        assert_noop!(
            PropertyManagement::letting_agent_claim_property(
                RuntimeOrigin::signed([4; 32].into()),
                0
            ),
            Error::<Test>::LettingAgentAlreadySet
        );
    });
}

// vote_on_letting_agent tests

#[test]
fn vote_on_letting_agent_works() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);
        // Set up roles and region
        assert_ok!(XcavateWhitelist::add_admin(RuntimeOrigin::root(), [20; 32].into(),));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [8; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [6; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));
        new_region_helper();
        assert_ok!(Regions::create_new_location(
            RuntimeOrigin::signed([6; 32].into()),
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
            pallet_xcavate_whitelist::Role::LettingAgent
        ));

        // Property listing process
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
        lawyer_process_helper([0; 32].into(), 0, vec![([1; 32].into(), 40), ([2; 32].into(), 30)]);

        // Letting agent process
        assert_ok!(PropertyManagement::add_letting_agent(
            RuntimeOrigin::signed([4; 32].into()),
            3,
            bvec![10, 10],
        ));
        assert_ok!(PropertyManagement::letting_agent_claim_property(
            RuntimeOrigin::signed([4; 32].into()),
            0
        ));
        assert_eq!(
            OngoingLettingAgentVoting::<Test>::get(0).unwrap(),
            crate::VoteStats { yes_voting_power: 0, no_voting_power: 0, abstain_voting_power: 0 },
        );
        assert_ok!(PropertyManagement::vote_on_letting_agent(
            RuntimeOrigin::signed([1; 32].into()),
            0,
            crate::Vote::Yes,
            40,
        ));
        assert_eq!(
            OngoingLettingAgentVoting::<Test>::get(0).unwrap(),
            crate::VoteStats { yes_voting_power: 40, no_voting_power: 0, abstain_voting_power: 0 },
        );
        assert_eq!(
            UserLettingAgentVote::<Test>::get::<u64, AccountId>(0, [1; 32].into()).unwrap(),
            VoteRecord { vote: crate::Vote::Yes, asset_id: 0, power: 40 }
        );
        assert_eq!(
            AssetsFreezer::balance_frozen(
                0,
                &MarketplaceFreezeReason::LettingAgentVoting,
                &[1; 32].into()
            ),
            40
        );
        assert_ok!(PropertyManagement::vote_on_letting_agent(
            RuntimeOrigin::signed([1; 32].into()),
            0,
            crate::Vote::No,
            25,
        ));
        assert_eq!(
            OngoingLettingAgentVoting::<Test>::get(0).unwrap(),
            crate::VoteStats { yes_voting_power: 0, no_voting_power: 25, abstain_voting_power: 0 },
        );
        assert_ok!(PropertyManagement::vote_on_letting_agent(
            RuntimeOrigin::signed([2; 32].into()),
            0,
            crate::Vote::Yes,
            20,
        ));
        assert_eq!(
            OngoingLettingAgentVoting::<Test>::get(0).unwrap(),
            crate::VoteStats { yes_voting_power: 20, no_voting_power: 25, abstain_voting_power: 0 },
        );
        assert_eq!(
            UserLettingAgentVote::<Test>::get::<u64, AccountId>(0, [1; 32].into()).unwrap(),
            VoteRecord { vote: crate::Vote::No, asset_id: 0, power: 25 }
        );
        assert_eq!(
            AssetsFreezer::balance_frozen(
                0,
                &MarketplaceFreezeReason::LettingAgentVoting,
                &[1; 32].into()
            ),
            25
        );
    });
}

#[test]
fn vote_on_letting_agent_fails() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);
        // Set up roles and region
        assert_ok!(XcavateWhitelist::add_admin(RuntimeOrigin::root(), [20; 32].into(),));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [8; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [6; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));
        new_region_helper();
        assert_ok!(Regions::create_new_location(
            RuntimeOrigin::signed([6; 32].into()),
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
            [4; 32].into(),
            pallet_xcavate_whitelist::Role::LettingAgent
        ));
        assert_noop!(
            PropertyManagement::vote_on_letting_agent(
                RuntimeOrigin::signed([1; 32].into()),
                0,
                crate::Vote::Yes,
                100
            ),
            Error::<Test>::NoLettingAgentProposed
        );

        // Property listing process
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
        lawyer_process_helper([0; 32].into(), 0, vec![([1; 32].into(), 40), ([30; 32].into(), 30)]);

        // Letting agent process
        assert_ok!(PropertyManagement::add_letting_agent(
            RuntimeOrigin::signed([4; 32].into()),
            3,
            bvec![10, 10],
        ));
        assert_noop!(
            PropertyManagement::vote_on_letting_agent(
                RuntimeOrigin::signed([1; 32].into()),
                0,
                crate::Vote::Yes,
                100
            ),
            Error::<Test>::NoLettingAgentProposed
        );
        assert_ok!(PropertyManagement::letting_agent_claim_property(
            RuntimeOrigin::signed([4; 32].into()),
            0
        ));
        assert_noop!(
            PropertyManagement::vote_on_letting_agent(
                RuntimeOrigin::signed([2; 32].into()),
                0,
                crate::Vote::Yes,
                100
            ),
            Error::<Test>::NoPermission
        );
        assert_noop!(
            PropertyManagement::vote_on_letting_agent(
                RuntimeOrigin::signed([1; 32].into()),
                0,
                crate::Vote::Yes,
                0
            ),
            Error::<Test>::ZeroVoteAmount
        );
        let expiry =
            frame_system::Pallet::<Test>::block_number() + LettingAgentVotingDuration::get();
        run_to_block(expiry);
        assert_noop!(
            PropertyManagement::vote_on_letting_agent(
                RuntimeOrigin::signed([1; 32].into()),
                0,
                crate::Vote::Yes,
                100
            ),
            Error::<Test>::VotingExpired
        );
    });
}

// finalize_letting_agent tests

#[test]
fn finalize_letting_agent_works() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);
        // Set up roles and region
        assert_ok!(XcavateWhitelist::add_admin(RuntimeOrigin::root(), [20; 32].into(),));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [8; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [6; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));
        new_region_helper();
        assert_ok!(Regions::create_new_location(
            RuntimeOrigin::signed([6; 32].into()),
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
            pallet_xcavate_whitelist::Role::LettingAgent
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [4; 32].into(),
            pallet_xcavate_whitelist::Role::LettingAgent
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

        // Property listing process
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
            3,
            bvec![10, 10],
            1_000,
            100,
            bvec![22, 22],
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
        assert_ok!(Marketplace::buy_property_shares(
            RuntimeOrigin::signed([1; 32].into()),
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
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [5; 32].into(),
            pallet_xcavate_whitelist::Role::SpvConfirmation
        ));
        assert_ok!(Marketplace::create_spv(RuntimeOrigin::signed([5; 32].into()), 0,));
        assert_ok!(Marketplace::create_spv(RuntimeOrigin::signed([5; 32].into()), 1,));
        assert_ok!(Marketplace::create_spv(RuntimeOrigin::signed([5; 32].into()), 2,));
        assert_ok!(Marketplace::claim_property_shares(RuntimeOrigin::signed([1; 32].into()), 0));
        assert_ok!(Marketplace::claim_property_shares(RuntimeOrigin::signed([2; 32].into()), 0));
        assert_ok!(Marketplace::claim_property_shares(RuntimeOrigin::signed([30; 32].into()), 0));
        assert_ok!(Marketplace::claim_property_shares(RuntimeOrigin::signed([1; 32].into()), 1));
        assert_ok!(Marketplace::claim_property_shares(RuntimeOrigin::signed([30; 32].into()), 1));
        assert_ok!(Marketplace::claim_property_shares(RuntimeOrigin::signed([31; 32].into()), 1));
        assert_ok!(Marketplace::claim_property_shares(RuntimeOrigin::signed([1; 32].into()), 2));
        assert_ok!(Marketplace::claim_property_shares(RuntimeOrigin::signed([30; 32].into()), 2));
        assert_ok!(Marketplace::claim_property_shares(RuntimeOrigin::signed([31; 32].into()), 2));
        assert_ok!(PropertyManagement::add_letting_agent(
            RuntimeOrigin::signed([3; 32].into()),
            3,
            bvec![10, 10],
        ));
        assert_ok!(PropertyManagement::add_letting_agent(
            RuntimeOrigin::signed([4; 32].into()),
            3,
            bvec![10, 10],
        ));
        lawyer_process_helper([0; 32].into(), 0, vec![([1; 32].into(), 40), ([2; 32].into(), 30)]);

        // Add letting agent to the first property
        assert_ok!(PropertyManagement::letting_agent_claim_property(
            RuntimeOrigin::signed([4; 32].into()),
            0
        ));
        assert_eq!(AssetLettingProposal::<Test>::get(0).unwrap(), 0);
        assert_ok!(PropertyManagement::vote_on_letting_agent(
            RuntimeOrigin::signed([1; 32].into()),
            0,
            crate::Vote::Yes,
            40
        ));
        assert_ok!(PropertyManagement::vote_on_letting_agent(
            RuntimeOrigin::signed([30; 32].into()),
            0,
            crate::Vote::Yes,
            30
        ));
        let expiry =
            frame_system::Pallet::<Test>::block_number() + LettingAgentVotingDuration::get();
        run_to_block(expiry);
        assert_ok!(PropertyManagement::finalize_letting_agent(
            RuntimeOrigin::signed([7; 32].into()),
            0,
        ));
        assert_ok!(PropertyManagement::unfreeze_letting_voting_shares(
            RuntimeOrigin::signed([1; 32].into()),
            0,
        ));
        assert_eq!(LettingStorage::<Test>::get(0).unwrap(), [4; 32].into());
        assert_eq!(
            LettingInfo::<Test>::get::<AccountId>([4; 32].into())
                .unwrap()
                .locations
                .get(&bvec![10, 10])
                .clone()
                .unwrap()
                .assigned_properties,
            1
        );
        finalize_property_helper(
            [0; 32].into(),
            1,
            vec![([1; 32].into(), 40), ([30; 32].into(), 30)],
        );

        // Add letting agent to the second property
        assert_ok!(PropertyManagement::letting_agent_claim_property(
            RuntimeOrigin::signed([4; 32].into()),
            1
        ));
        assert_eq!(AssetLettingProposal::<Test>::get(1).unwrap(), 1);
        assert_ok!(PropertyManagement::vote_on_letting_agent(
            RuntimeOrigin::signed([1; 32].into()),
            1,
            crate::Vote::Yes,
            40
        ));
        assert_ok!(PropertyManagement::vote_on_letting_agent(
            RuntimeOrigin::signed([30; 32].into()),
            1,
            crate::Vote::Yes,
            30
        ));
        let expiry =
            frame_system::Pallet::<Test>::block_number() + LettingAgentVotingDuration::get();
        run_to_block(expiry);
        assert_ok!(PropertyManagement::finalize_letting_agent(
            RuntimeOrigin::signed([2; 32].into()),
            1,
        ));
        assert_ok!(PropertyManagement::unfreeze_letting_voting_shares(
            RuntimeOrigin::signed([1; 32].into()),
            1,
        ));
        finalize_property_helper(
            [0; 32].into(),
            2,
            vec![([1; 32].into(), 40), ([30; 32].into(), 30)],
        );

        // Add letting agent to the third property
        assert_ok!(PropertyManagement::letting_agent_claim_property(
            RuntimeOrigin::signed([3; 32].into()),
            2
        ));
        assert_eq!(AssetLettingProposal::<Test>::get(2).unwrap(), 2);
        assert_ok!(PropertyManagement::vote_on_letting_agent(
            RuntimeOrigin::signed([1; 32].into()),
            2,
            crate::Vote::Yes,
            40
        ));
        assert_ok!(PropertyManagement::vote_on_letting_agent(
            RuntimeOrigin::signed([30; 32].into()),
            2,
            crate::Vote::Abstain,
            30
        ));
        let expiry =
            frame_system::Pallet::<Test>::block_number() + LettingAgentVotingDuration::get();
        run_to_block(expiry);
        assert_ok!(PropertyManagement::finalize_letting_agent(
            RuntimeOrigin::signed([2; 32].into()),
            2,
        ));
        assert_ok!(PropertyManagement::unfreeze_letting_voting_shares(
            RuntimeOrigin::signed([1; 32].into()),
            2,
        ));

        // Final assertions
        assert_eq!(LettingStorage::<Test>::get(0).unwrap(), [4; 32].into());
        assert_eq!(LettingStorage::<Test>::get(1).unwrap(), [4; 32].into());
        assert_eq!(LettingStorage::<Test>::get(2).unwrap(), [3; 32].into());
        assert_eq!(
            LettingInfo::<Test>::get::<AccountId>([4; 32].into())
                .unwrap()
                .locations
                .get(&bvec![10, 10])
                .clone()
                .unwrap()
                .assigned_properties,
            2
        );
        assert_eq!(
            LettingInfo::<Test>::get::<AccountId>([3; 32].into())
                .unwrap()
                .locations
                .get(&bvec![10, 10])
                .clone()
                .unwrap()
                .assigned_properties,
            1
        );
        assert!(LettingAgentProposal::<Test>::get(0).is_none());
        assert_eq!(OngoingLettingAgentVoting::<Test>::get(0), None);
        assert_eq!(AssetLettingProposal::<Test>::get(0), None);
        assert_eq!(UserLettingAgentVote::<Test>::get::<u64, AccountId>(0, [1; 32].into()), None);
        assert_eq!(UserLettingAgentVote::<Test>::get::<u64, AccountId>(1, [1; 32].into()), None);
        assert_eq!(UserLettingAgentVote::<Test>::get::<u64, AccountId>(2, [1; 32].into()), None);
    });
}

#[test]
fn finalize_letting_agent_works_2() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);
        // Set up roles and region
        assert_ok!(XcavateWhitelist::add_admin(RuntimeOrigin::root(), [20; 32].into(),));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [8; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [6; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));
        new_region_helper();
        assert_ok!(Regions::create_new_location(
            RuntimeOrigin::signed([6; 32].into()),
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
            pallet_xcavate_whitelist::Role::LettingAgent
        ));

        // Property listing process
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
        lawyer_process_helper([0; 32].into(), 0, vec![([1; 32].into(), 40), ([2; 32].into(), 30)]);

        // Letting agent process
        assert_ok!(PropertyManagement::add_letting_agent(
            RuntimeOrigin::signed([4; 32].into()),
            3,
            bvec![10, 10],
        ));

        // First voting round
        assert_ok!(PropertyManagement::letting_agent_claim_property(
            RuntimeOrigin::signed([4; 32].into()),
            0
        ));
        assert_eq!(AssetLettingProposal::<Test>::get(0).unwrap(), 0);
        assert_ok!(PropertyManagement::vote_on_letting_agent(
            RuntimeOrigin::signed([1; 32].into()),
            0,
            crate::Vote::Yes,
            20
        ));
        assert_ok!(PropertyManagement::vote_on_letting_agent(
            RuntimeOrigin::signed([2; 32].into()),
            0,
            crate::Vote::Yes,
            20
        ));
        let expiry =
            frame_system::Pallet::<Test>::block_number() + LettingAgentVotingDuration::get();
        run_to_block(expiry);
        // First round rejected because quorum not met
        assert_ok!(PropertyManagement::finalize_letting_agent(
            RuntimeOrigin::signed([2; 32].into()),
            0,
        ));
        assert_ok!(PropertyManagement::unfreeze_letting_voting_shares(
            RuntimeOrigin::signed([1; 32].into()),
            0,
        ));
        assert_ok!(PropertyManagement::unfreeze_letting_voting_shares(
            RuntimeOrigin::signed([2; 32].into()),
            0,
        ));
        assert!(LettingStorage::<Test>::get(0).is_none());
        assert_eq!(
            LettingInfo::<Test>::get::<AccountId>([4; 32].into())
                .unwrap()
                .locations
                .get(&bvec![10, 10])
                .clone()
                .unwrap()
                .assigned_properties,
            0
        );

        // Second voting round
        assert_ok!(PropertyManagement::letting_agent_claim_property(
            RuntimeOrigin::signed([4; 32].into()),
            0
        ));
        assert_ok!(PropertyManagement::vote_on_letting_agent(
            RuntimeOrigin::signed([1; 32].into()),
            0,
            crate::Vote::Yes,
            20
        ));
        assert_ok!(PropertyManagement::vote_on_letting_agent(
            RuntimeOrigin::signed([2; 32].into()),
            0,
            crate::Vote::Yes,
            30
        ));
        let expiry =
            frame_system::Pallet::<Test>::block_number() + LettingAgentVotingDuration::get();
        run_to_block(expiry);
        // Secound round rejected because quorum not met
        assert_ok!(PropertyManagement::finalize_letting_agent(
            RuntimeOrigin::signed([2; 32].into()),
            0,
        ));
        assert_ok!(PropertyManagement::unfreeze_letting_voting_shares(
            RuntimeOrigin::signed([1; 32].into()),
            1,
        ));
        assert_ok!(PropertyManagement::unfreeze_letting_voting_shares(
            RuntimeOrigin::signed([2; 32].into()),
            1,
        ));
        assert!(LettingStorage::<Test>::get(0).is_none());

        // Third voting round
        assert_ok!(PropertyManagement::letting_agent_claim_property(
            RuntimeOrigin::signed([4; 32].into()),
            0
        ));
        assert_ok!(PropertyManagement::vote_on_letting_agent(
            RuntimeOrigin::signed([1; 32].into()),
            0,
            crate::Vote::Yes,
            40
        ));
        assert_ok!(PropertyManagement::vote_on_letting_agent(
            RuntimeOrigin::signed([2; 32].into()),
            0,
            crate::Vote::Yes,
            25
        ));
        let expiry =
            frame_system::Pallet::<Test>::block_number() + LettingAgentVotingDuration::get();
        run_to_block(expiry);
        // Third round passed
        assert_ok!(PropertyManagement::finalize_letting_agent(
            RuntimeOrigin::signed([2; 32].into()),
            0,
        ));
        assert_eq!(LettingStorage::<Test>::get(0).unwrap(), [4; 32].into());
    });
}

#[test]
fn finalize_letting_agent_fails() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);
        // Set up roles and region
        assert_ok!(XcavateWhitelist::add_admin(RuntimeOrigin::root(), [20; 32].into(),));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [8; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [6; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));
        new_region_helper();
        assert_ok!(Regions::create_new_location(
            RuntimeOrigin::signed([6; 32].into()),
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
            [15; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [16; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [4; 32].into(),
            pallet_xcavate_whitelist::Role::LettingAgent
        ));

        // First property listing process
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
        assert_noop!(
            PropertyManagement::finalize_letting_agent(RuntimeOrigin::signed([2; 32].into()), 0,),
            Error::<Test>::NoLettingAgentProposed
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
        lawyer_process_helper([0; 32].into(), 0, vec![([1; 32].into(), 40), ([2; 32].into(), 30)]);

        // Letting agent process
        assert_ok!(PropertyManagement::add_letting_agent(
            RuntimeOrigin::signed([4; 32].into()),
            3,
            bvec![10, 10],
        ));
        assert_noop!(
            PropertyManagement::finalize_letting_agent(RuntimeOrigin::signed([2; 32].into()), 0,),
            Error::<Test>::NoLettingAgentProposed
        );
        assert_ok!(PropertyManagement::letting_agent_claim_property(
            RuntimeOrigin::signed([4; 32].into()),
            0
        ));
        assert_eq!(AssetLettingProposal::<Test>::get(0).unwrap(), 0);
        assert_ok!(PropertyManagement::vote_on_letting_agent(
            RuntimeOrigin::signed([1; 32].into()),
            0,
            crate::Vote::Yes,
            40
        ));
        assert_ok!(PropertyManagement::vote_on_letting_agent(
            RuntimeOrigin::signed([2; 32].into()),
            0,
            crate::Vote::Yes,
            30
        ));
        assert_noop!(
            PropertyManagement::finalize_letting_agent(RuntimeOrigin::signed([2; 32].into()), 0,),
            Error::<Test>::VotingStillOngoing
        );
        let expiry =
            frame_system::Pallet::<Test>::block_number() + LettingAgentVotingDuration::get();
        run_to_block(expiry);
        // Assign maximum amount of properties to a letting agent
        for x in 1..=MaxProperty::get() {
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
                RuntimeOrigin::signed([0; 32].into()),
                x,
                40,
                1984
            ));
            assert_ok!(Marketplace::buy_property_shares(
                RuntimeOrigin::signed([15; 32].into()),
                x,
                30,
                1984
            ));
            assert_ok!(Marketplace::buy_property_shares(
                RuntimeOrigin::signed([16; 32].into()),
                x,
                30,
                1984
            ));
            assert_ok!(Marketplace::create_spv(RuntimeOrigin::signed([5; 32].into()), x,));
            assert_ok!(Marketplace::claim_property_shares(RuntimeOrigin::signed([0; 32].into()), x));
            assert_ok!(Marketplace::claim_property_shares(
                RuntimeOrigin::signed([15; 32].into()),
                x
            ));
            assert_ok!(Marketplace::claim_property_shares(
                RuntimeOrigin::signed([16; 32].into()),
                x
            ));
            finalize_property_helper(
                [0; 32].into(),
                x,
                vec![([0; 32].into(), 40), ([15; 32].into(), 30)],
            );
            assert_ok!(PropertyManagement::letting_agent_claim_property(
                RuntimeOrigin::signed([4; 32].into()),
                x
            ));
            assert_ok!(PropertyManagement::vote_on_letting_agent(
                RuntimeOrigin::signed([0; 32].into()),
                x,
                crate::Vote::Yes,
                40
            ));
            assert_ok!(PropertyManagement::vote_on_letting_agent(
                RuntimeOrigin::signed([15; 32].into()),
                x,
                crate::Vote::Yes,
                30
            ));
            let expiry =
                frame_system::Pallet::<Test>::block_number() + LettingAgentVotingDuration::get();
            frame_system::Pallet::<Test>::set_block_number(expiry);
            assert_ok!(PropertyManagement::finalize_letting_agent(
                RuntimeOrigin::signed([0; 32].into()),
                x
            ));
        }
        assert_eq!(
            LettingInfo::<Test>::get::<AccountId>([4; 32].into())
                .unwrap()
                .locations
                .get(&bvec![10, 10])
                .clone()
                .unwrap()
                .assigned_properties,
            MaxProperty::get()
        );
        assert_ok!(PropertyManagement::finalize_letting_agent(
            RuntimeOrigin::signed([1; 32].into()),
            0
        ),);
        assert_ok!(PropertyManagement::unfreeze_letting_voting_shares(
            RuntimeOrigin::signed([1; 32].into()),
            0,
        ));
        assert!(LettingStorage::<Test>::get(0).is_some());
        assert_eq!(
            LettingInfo::<Test>::get::<AccountId>([4; 32].into())
                .unwrap()
                .locations
                .get(&bvec![10, 10])
                .clone()
                .unwrap()
                .assigned_properties,
            MaxProperty::get() + 1
        );
        assert!(LettingAgentProposal::<Test>::get(0).is_none());
        assert_eq!(OngoingLettingAgentVoting::<Test>::get(0), None);
        assert_eq!(UserLettingAgentVote::<Test>::get::<u64, AccountId>(0, [1; 32].into()), None);
    });
}

// finalize_letting_agent tests

#[test]
fn unfreeze_letting_voting_shares_works() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);
        // Set up roles and region
        assert_ok!(XcavateWhitelist::add_admin(RuntimeOrigin::root(), [20; 32].into(),));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [8; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [6; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));
        new_region_helper();
        assert_ok!(Regions::create_new_location(
            RuntimeOrigin::signed([6; 32].into()),
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
            pallet_xcavate_whitelist::Role::LettingAgent
        ));

        // Property listing process
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
        lawyer_process_helper([0; 32].into(), 0, vec![([1; 32].into(), 40), ([2; 32].into(), 30)]);

        // Letting agent process
        assert_ok!(PropertyManagement::add_letting_agent(
            RuntimeOrigin::signed([4; 32].into()),
            3,
            bvec![10, 10],
        ));
        assert_ok!(PropertyManagement::letting_agent_claim_property(
            RuntimeOrigin::signed([4; 32].into()),
            0
        ));
        assert_ok!(PropertyManagement::vote_on_letting_agent(
            RuntimeOrigin::signed([1; 32].into()),
            0,
            crate::Vote::Yes,
            40
        ));
        assert_ok!(PropertyManagement::vote_on_letting_agent(
            RuntimeOrigin::signed([2; 32].into()),
            0,
            crate::Vote::No,
            20
        ));
        let expiry =
            frame_system::Pallet::<Test>::block_number() + LettingAgentVotingDuration::get();
        run_to_block(expiry);
        assert_ok!(PropertyManagement::finalize_letting_agent(
            RuntimeOrigin::signed([2; 32].into()),
            0,
        ));
        assert_eq!(
            AssetsFreezer::balance_frozen(
                0,
                &MarketplaceFreezeReason::LettingAgentVoting,
                &[1; 32].into()
            ),
            40
        );
        assert_eq!(
            AssetsFreezer::balance_frozen(
                0,
                &MarketplaceFreezeReason::LettingAgentVoting,
                &[2; 32].into()
            ),
            20
        );
        assert_ok!(PropertyManagement::unfreeze_letting_voting_shares(
            RuntimeOrigin::signed([1; 32].into()),
            0,
        ));
        assert_ok!(PropertyManagement::unfreeze_letting_voting_shares(
            RuntimeOrigin::signed([2; 32].into()),
            0,
        ));
        assert_eq!(
            AssetsFreezer::balance_frozen(
                0,
                &MarketplaceFreezeReason::LettingAgentVoting,
                &[1; 32].into()
            ),
            0
        );
        assert_eq!(
            AssetsFreezer::balance_frozen(
                0,
                &MarketplaceFreezeReason::LettingAgentVoting,
                &[2; 32].into()
            ),
            0
        );
        assert_eq!(UserLettingAgentVote::<Test>::get::<u64, AccountId>(0, [1; 32].into()), None);
        assert_eq!(UserLettingAgentVote::<Test>::get::<u64, AccountId>(0, [2; 32].into()), None);
    });
}

#[test]
fn unfreeze_letting_voting_shares_fails() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);
        // Set up roles and region
        assert_ok!(XcavateWhitelist::add_admin(RuntimeOrigin::root(), [20; 32].into(),));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [8; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [6; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));
        new_region_helper();
        assert_ok!(Regions::create_new_location(
            RuntimeOrigin::signed([6; 32].into()),
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
            [4; 32].into(),
            pallet_xcavate_whitelist::Role::LettingAgent
        ));

        // Property listing process
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
        lawyer_process_helper([0; 32].into(), 0, vec![([1; 32].into(), 40), ([30; 32].into(), 30)]);

        // Letting agent process
        assert_ok!(PropertyManagement::add_letting_agent(
            RuntimeOrigin::signed([4; 32].into()),
            3,
            bvec![10, 10],
        ));
        assert_ok!(PropertyManagement::letting_agent_claim_property(
            RuntimeOrigin::signed([4; 32].into()),
            0
        ));
        assert_noop!(
            PropertyManagement::unfreeze_letting_voting_shares(
                RuntimeOrigin::signed([1; 32].into()),
                0,
            ),
            Error::<Test>::NoFrozenAmount
        );
        assert_ok!(PropertyManagement::vote_on_letting_agent(
            RuntimeOrigin::signed([1; 32].into()),
            0,
            crate::Vote::Yes,
            40
        ));
        assert_ok!(PropertyManagement::vote_on_letting_agent(
            RuntimeOrigin::signed([30; 32].into()),
            0,
            crate::Vote::Yes,
            30
        ));
        assert_noop!(
            PropertyManagement::unfreeze_letting_voting_shares(
                RuntimeOrigin::signed([1; 32].into()),
                0,
            ),
            Error::<Test>::VotingStillOngoing
        );
    });
}

// distribute_income tests

#[test]
fn distribute_income_works() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);
        // Set up roles and region
        assert_ok!(XcavateWhitelist::add_admin(RuntimeOrigin::root(), [20; 32].into(),));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [8; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [6; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));
        new_region_helper();
        assert_ok!(Regions::create_new_location(
            RuntimeOrigin::signed([6; 32].into()),
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

        // Property listing process
        assert_ok!(Marketplace::list_property(
            RuntimeOrigin::signed([0; 32].into()),
            3,
            bvec![10, 10],
            9_000,
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
            RuntimeOrigin::signed([2; 32].into()),
            0,
            30,
            1984
        ));
        assert_ok!(Marketplace::buy_property_shares(
            RuntimeOrigin::signed([3; 32].into()),
            0,
            45,
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
        assert_ok!(Marketplace::claim_property_shares(RuntimeOrigin::signed([3; 32].into()), 0));

        // Legal process
        assert_ok!(Marketplace::lawyer_claim_property(
            RuntimeOrigin::signed([10; 32].into()),
            0,
            LegalProperty::RealEstateDeveloperSide,
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
            LegalProperty::SpvSide,
            4_000,
        ));
        assert_ok!(Marketplace::vote_on_spv_lawyer(
            RuntimeOrigin::signed([1; 32].into()),
            0,
            pallet_marketplace::types::Vote::Yes,
            20,
        ));
        assert_ok!(Marketplace::vote_on_spv_lawyer(
            RuntimeOrigin::signed([3; 32].into()),
            0,
            pallet_marketplace::types::Vote::Yes,
            40,
        ));
        let expiry =
            frame_system::Pallet::<Test>::block_number() + LettingAgentVotingDuration::get();
        run_to_block(expiry);
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

        // Letting agent process
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [4; 32].into(),
            pallet_xcavate_whitelist::Role::LettingAgent
        ));
        assert_ok!(PropertyManagement::add_letting_agent(
            RuntimeOrigin::signed([4; 32].into()),
            3,
            bvec![10, 10],
        ));
        assert_ok!(PropertyManagement::letting_agent_claim_property(
            RuntimeOrigin::signed([4; 32].into()),
            0
        ));
        assert_ok!(PropertyManagement::vote_on_letting_agent(
            RuntimeOrigin::signed([1; 32].into()),
            0,
            crate::Vote::Yes,
            25
        ));
        assert_ok!(PropertyManagement::vote_on_letting_agent(
            RuntimeOrigin::signed([3; 32].into()),
            0,
            crate::Vote::Abstain,
            45
        ));
        let expiry =
            frame_system::Pallet::<Test>::block_number() + LettingAgentVotingDuration::get();
        run_to_block(expiry);
        assert_ok!(PropertyManagement::finalize_letting_agent(
            RuntimeOrigin::signed([3; 32].into()),
            0,
        ));

        // Income distribution
        assert_ok!(PropertyManagement::distribute_income(
            RuntimeOrigin::signed([4; 32].into()),
            0,
            2200,
            1984,
        ));
        assert_eq!(PropertyIncome::<Test>::get(0), 22);
        assert_eq!(ForeignAssets::balance(1984, &[4; 32].into()), 2800);
        assert_ok!(PropertyManagement::distribute_income(
            RuntimeOrigin::signed([4; 32].into()),
            0,
            2000,
            1337,
        ));
        assert_ok!(PropertyManagement::distribute_income(
            RuntimeOrigin::signed([4; 32].into()),
            0,
            100,
            1984,
        ));
        assert_eq!(PropertyIncome::<Test>::get(0), 43);
    });
}

#[test]
fn distribute_income_fails() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);
        // Set up roles and region
        assert_ok!(XcavateWhitelist::add_admin(RuntimeOrigin::root(), [20; 32].into(),));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [8; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [6; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));
        new_region_helper();
        assert_ok!(Regions::create_new_location(
            RuntimeOrigin::signed([6; 32].into()),
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
            [4; 32].into(),
            pallet_xcavate_whitelist::Role::LettingAgent
        ));

        // Property listing process
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
        lawyer_process_helper([0; 32].into(), 0, vec![([1; 32].into(), 40), ([30; 32].into(), 30)]);

        // Letting agent process
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [5; 32].into(),
            pallet_xcavate_whitelist::Role::LettingAgent
        ));
        assert_noop!(
            PropertyManagement::distribute_income(
                RuntimeOrigin::signed([5; 32].into()),
                0,
                200,
                1984
            ),
            Error::<Test>::NoLettingAgentFound
        );
        assert_eq!(PropertyIncome::<Test>::get(0), 0);
        assert_ok!(PropertyManagement::add_letting_agent(
            RuntimeOrigin::signed([4; 32].into()),
            3,
            bvec![10, 10],
        ));
        assert_ok!(PropertyManagement::letting_agent_claim_property(
            RuntimeOrigin::signed([4; 32].into()),
            0
        ));
        assert_ok!(PropertyManagement::vote_on_letting_agent(
            RuntimeOrigin::signed([1; 32].into()),
            0,
            crate::Vote::Yes,
            40
        ));
        assert_ok!(PropertyManagement::vote_on_letting_agent(
            RuntimeOrigin::signed([30; 32].into()),
            0,
            crate::Vote::Yes,
            30
        ));
        let expiry =
            frame_system::Pallet::<Test>::block_number() + LettingAgentVotingDuration::get();
        run_to_block(expiry);
        assert_ok!(PropertyManagement::finalize_letting_agent(
            RuntimeOrigin::signed([1; 32].into()),
            0,
        ));

        // Income distribution failure cases
        assert_noop!(
            PropertyManagement::distribute_income(
                RuntimeOrigin::signed([5; 32].into()),
                0,
                200,
                1984
            ),
            Error::<Test>::NoPermission
        );
        assert_noop!(
            PropertyManagement::distribute_income(
                RuntimeOrigin::signed([4; 32].into()),
                0,
                20000,
                1984
            ),
            Error::<Test>::NotEnoughFunds
        );
        assert_noop!(
            PropertyManagement::distribute_income(
                RuntimeOrigin::signed([4; 32].into()),
                0,
                2000,
                1
            ),
            Error::<Test>::PaymentAssetNotSupported
        );
    });
}

// claim_income tests

#[test]
fn claim_income_works() {
    new_test_ext().execute_with(|| {
        // Set up roles and region
        System::set_block_number(1);
        assert_ok!(XcavateWhitelist::add_admin(RuntimeOrigin::root(), [20; 32].into(),));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [8; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [6; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));
        new_region_helper();
        assert_ok!(Regions::create_new_location(
            RuntimeOrigin::signed([6; 32].into()),
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
            [4; 32].into(),
            pallet_xcavate_whitelist::Role::LettingAgent
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

        // Property listing process
        assert_ok!(Marketplace::list_property(
            RuntimeOrigin::signed([0; 32].into()),
            3,
            bvec![10, 10],
            9_000,
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
            LegalProperty::RealEstateDeveloperSide,
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
            LegalProperty::SpvSide,
            4_000,
        ));
        assert_ok!(Marketplace::vote_on_spv_lawyer(
            RuntimeOrigin::signed([1; 32].into()),
            0,
            pallet_marketplace::types::Vote::Yes,
            40
        ));
        assert_ok!(Marketplace::vote_on_spv_lawyer(
            RuntimeOrigin::signed([30; 32].into()),
            0,
            pallet_marketplace::types::Vote::Yes,
            30
        ));
        let expiry = frame_system::Pallet::<Test>::block_number() + LawyerVotingDuration::get();
        run_to_block(expiry);
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

        // Letting agent process
        assert_ok!(PropertyManagement::add_letting_agent(
            RuntimeOrigin::signed([4; 32].into()),
            3,
            bvec![10, 10],
        ));
        assert_ok!(PropertyManagement::letting_agent_claim_property(
            RuntimeOrigin::signed([4; 32].into()),
            0
        ));
        assert_ok!(PropertyManagement::vote_on_letting_agent(
            RuntimeOrigin::signed([1; 32].into()),
            0,
            crate::Vote::Yes,
            40
        ));
        assert_ok!(PropertyManagement::vote_on_letting_agent(
            RuntimeOrigin::signed([30; 32].into()),
            0,
            crate::Vote::Yes,
            30
        ));
        let expiry =
            frame_system::Pallet::<Test>::block_number() + LettingAgentVotingDuration::get();
        run_to_block(expiry);
        assert_ok!(PropertyManagement::finalize_letting_agent(
            RuntimeOrigin::signed([1; 32].into()),
            0,
        ));

        // Income distribution and claiming
        assert_ok!(PropertyManagement::distribute_income(
            RuntimeOrigin::signed([4; 32].into()),
            0,
            2200,
            1984,
        ));
        assert_ok!(PropertyManagement::distribute_income(
            RuntimeOrigin::signed([4; 32].into()),
            0,
            1000,
            1337,
        ));
        assert_eq!(PropertyIncome::<Test>::get(0), 32);
        assert_eq!(OwnerCheckpoints::<Test>::get::<AccountId, u32>([1; 32].into(), 0), 0);
        assert_eq!(ForeignAssets::balance(1984, &[4; 32].into()), 2800);
        assert_eq!(Balances::free_balance(&([4; 32].into())), 4000);
        assert_eq!(Balances::free_balance(&PropertyManagement::property_account_id(0)), 5085);
        assert_eq!(ForeignAssets::balance(1984, &PropertyManagement::property_account_id(0)), 2200);
        assert_eq!(ForeignAssets::balance(1337, &PropertyManagement::property_account_id(0)), 1000);
        assert_ok!(PropertyManagement::claim_income(RuntimeOrigin::signed([1; 32].into()), 0,));
        // Claiming increases the owner checkpoint to the current income checkpoint
        assert_eq!(OwnerCheckpoints::<Test>::get::<AccountId, u32>([1; 32].into(), 0), 32);
        assert_eq!(ForeignAssets::balance(1984, &PropertyManagement::property_account_id(0)), 1920);
        assert_eq!(ForeignAssets::balance(1337, &PropertyManagement::property_account_id(0)), 0);
        assert_eq!(ForeignAssets::balance(1984, &[1; 32].into()), 1_125_880);
        assert_eq!(ForeignAssets::balance(1337, &[1; 32].into()), 1_000);
        assert_ok!(PropertyManagement::claim_income(RuntimeOrigin::signed([30; 32].into()), 0,));
        assert_ok!(PropertyManagement::claim_income(RuntimeOrigin::signed([31; 32].into()), 0,));
        assert_eq!(ForeignAssets::balance(1984, &PropertyManagement::property_account_id(0)), 0);
        assert_eq!(OwnerCheckpoints::<Test>::get::<AccountId, u32>([30; 32].into(), 0), 32);
        assert_eq!(OwnerCheckpoints::<Test>::get::<AccountId, u32>([31; 32].into(), 0), 32);
    });
}

#[test]
fn claim_income_works_2() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);
        // Set up roles and region
        assert_ok!(XcavateWhitelist::add_admin(RuntimeOrigin::root(), [20; 32].into(),));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [8; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [6; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));
        new_region_helper();
        assert_ok!(Regions::create_new_location(
            RuntimeOrigin::signed([6; 32].into()),
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
            [15; 32].into(),
            pallet_xcavate_whitelist::Role::LettingAgent
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

        // Property listing process
        assert_ok!(Marketplace::list_property(
            RuntimeOrigin::signed([0; 32].into()),
            3,
            bvec![10, 10],
            4_500,
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
            LegalProperty::RealEstateDeveloperSide,
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
            LegalProperty::SpvSide,
            4_000,
        ));
        assert_ok!(Marketplace::vote_on_spv_lawyer(
            RuntimeOrigin::signed([1; 32].into()),
            0,
            pallet_marketplace::types::Vote::Yes,
            80
        ));
        assert_ok!(Marketplace::vote_on_spv_lawyer(
            RuntimeOrigin::signed([30; 32].into()),
            0,
            pallet_marketplace::types::Vote::Yes,
            30
        ));
        let expiry = frame_system::Pallet::<Test>::block_number() + LawyerVotingDuration::get();
        run_to_block(expiry);
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

        // Letting agent process
        assert_ok!(PropertyManagement::add_letting_agent(
            RuntimeOrigin::signed([15; 32].into()),
            3,
            bvec![10, 10],
        ));
        assert_ok!(PropertyManagement::letting_agent_claim_property(
            RuntimeOrigin::signed([15; 32].into()),
            0
        ));
        assert_ok!(PropertyManagement::vote_on_letting_agent(
            RuntimeOrigin::signed([1; 32].into()),
            0,
            crate::Vote::Yes,
            80
        ));
        assert_ok!(PropertyManagement::vote_on_letting_agent(
            RuntimeOrigin::signed([30; 32].into()),
            0,
            crate::Vote::Yes,
            30
        ));
        let expiry =
            frame_system::Pallet::<Test>::block_number() + LettingAgentVotingDuration::get();
        run_to_block(expiry);
        assert_ok!(PropertyManagement::finalize_letting_agent(
            RuntimeOrigin::signed([1; 32].into()),
            0,
        ));

        // Income distribution and claiming
        assert_ok!(PropertyManagement::distribute_income(
            RuntimeOrigin::signed([15; 32].into()),
            0,
            4000,
            1984,
        ));
        assert_eq!(PropertyIncome::<Test>::get(0), 20);
        assert_eq!(OwnerCheckpoints::<Test>::get::<AccountId, u32>([1; 32].into(), 0), 0);
        assert_eq!(Balances::free_balance(&([15; 32].into())), 4000);
        assert_eq!(Balances::free_balance(&PropertyManagement::property_account_id(0)), 5085);
        assert_eq!(ForeignAssets::balance(1984, &PropertyManagement::property_account_id(0)), 4000);
        assert_eq!(ForeignAssets::balance(1984, &[1; 32].into()), 1_125_600);
        assert_ok!(PropertyManagement::claim_income(RuntimeOrigin::signed([1; 32].into()), 0,));
        // Claiming increases the owner checkpoint to the current income checkpoint
        assert_eq!(OwnerCheckpoints::<Test>::get::<AccountId, u32>([1; 32].into(), 0), 20);
        assert_eq!(ForeignAssets::balance(1984, &PropertyManagement::property_account_id(0)), 2400);
        assert_eq!(ForeignAssets::balance(1984, &[1; 32].into()), 1_127_200);
        assert_ok!(PropertyManagement::distribute_income(
            RuntimeOrigin::signed([15; 32].into()),
            0,
            6000,
            1984,
        ));
        assert_eq!(PropertyIncome::<Test>::get(0), 50);
        // Check that the owner checkpoint is still the same as before income distribution.
        assert_eq!(OwnerCheckpoints::<Test>::get::<AccountId, u32>([1; 32].into(), 0), 20);
        assert_eq!(ForeignAssets::balance(1984, &PropertyManagement::property_account_id(0)), 8400);
        assert_ok!(PropertyManagement::claim_income(RuntimeOrigin::signed([1; 32].into()), 0,));
        assert_eq!(OwnerCheckpoints::<Test>::get::<AccountId, u32>([1; 32].into(), 0), 50);
        assert_eq!(ForeignAssets::balance(1984, &PropertyManagement::property_account_id(0)), 6000);
        assert_eq!(ForeignAssets::balance(1984, &[1; 32].into()), 1_129_600);
        assert_ok!(PropertyManagement::claim_income(RuntimeOrigin::signed([30; 32].into()), 0,));
        assert_eq!(OwnerCheckpoints::<Test>::get::<AccountId, u32>([30; 32].into(), 0), 50);
    });
}

#[test]
fn claim_income_when_sending_shares_works() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);
        // Set up roles and region
        assert_ok!(XcavateWhitelist::add_admin(RuntimeOrigin::root(), [20; 32].into(),));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [8; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [6; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));
        new_region_helper();
        assert_ok!(Regions::create_new_location(
            RuntimeOrigin::signed([6; 32].into()),
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
            [3; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [15; 32].into(),
            pallet_xcavate_whitelist::Role::LettingAgent
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

        // Property listing process
        assert_ok!(Marketplace::list_property(
            RuntimeOrigin::signed([0; 32].into()),
            3,
            bvec![10, 10],
            4_500,
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
            LegalProperty::RealEstateDeveloperSide,
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
            LegalProperty::SpvSide,
            4_000,
        ));
        assert_ok!(Marketplace::vote_on_spv_lawyer(
            RuntimeOrigin::signed([1; 32].into()),
            0,
            pallet_marketplace::types::Vote::Yes,
            80
        ));
        assert_ok!(Marketplace::vote_on_spv_lawyer(
            RuntimeOrigin::signed([30; 32].into()),
            0,
            pallet_marketplace::types::Vote::Yes,
            30
        ));
        let expiry = frame_system::Pallet::<Test>::block_number() + LawyerVotingDuration::get();
        run_to_block(expiry);
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

        // Letting agent process
        assert_ok!(PropertyManagement::add_letting_agent(
            RuntimeOrigin::signed([15; 32].into()),
            3,
            bvec![10, 10],
        ));
        assert_ok!(PropertyManagement::letting_agent_claim_property(
            RuntimeOrigin::signed([15; 32].into()),
            0
        ));
        assert_ok!(PropertyManagement::vote_on_letting_agent(
            RuntimeOrigin::signed([1; 32].into()),
            0,
            crate::Vote::Yes,
            80
        ));
        assert_ok!(PropertyManagement::vote_on_letting_agent(
            RuntimeOrigin::signed([30; 32].into()),
            0,
            crate::Vote::Yes,
            30
        ));
        let expiry =
            frame_system::Pallet::<Test>::block_number() + LettingAgentVotingDuration::get();
        run_to_block(expiry);
        assert_ok!(PropertyManagement::finalize_letting_agent(
            RuntimeOrigin::signed([1; 32].into()),
            0,
        ));
        assert_ok!(PropertyManagement::unfreeze_letting_voting_shares(
            RuntimeOrigin::signed([1; 32].into()),
            0,
        ));

        // Income distribution and claiming
        assert_ok!(Marketplace::send_property_shares(
            RuntimeOrigin::signed([31; 32].into()),
            0,
            [3; 32].into(),
            2
        ));
        assert_ok!(PropertyManagement::distribute_income(
            RuntimeOrigin::signed([15; 32].into()),
            0,
            4000,
            1984,
        ));
        assert_eq!(PropertyIncome::<Test>::get(0), 20);
        assert_eq!(OwnerCheckpoints::<Test>::get::<AccountId, u32>([1; 32].into(), 0), 0);
        assert_eq!(Balances::free_balance(&([15; 32].into())), 4000);
        assert_eq!(Balances::free_balance(&PropertyManagement::property_account_id(0)), 5085);
        assert_eq!(ForeignAssets::balance(1984, &PropertyManagement::property_account_id(0)), 4000);
        assert_eq!(ForeignAssets::balance(1984, &[1; 32].into()), 1_125_600);
        assert_eq!(ForeignAssets::balance(1984, &[2; 32].into()), 1_150_000);
        assert_ok!(Marketplace::send_property_shares(
            RuntimeOrigin::signed([1; 32].into()),
            0,
            [2; 32].into(),
            40
        ));
        // Sending property shares sets the owner checkpoint of both parties to the income checkpoint
        assert_eq!(OwnerCheckpoints::<Test>::get::<AccountId, u32>([1; 32].into(), 0), 20);
        assert_eq!(OwnerCheckpoints::<Test>::get::<AccountId, u32>([2; 32].into(), 0), 20);
        assert_eq!(ForeignAssets::balance(1984, &PropertyManagement::property_account_id(0)), 2400);
        assert_eq!(ForeignAssets::balance(1984, &[1; 32].into()), 1_127_200);
        assert_eq!(ForeignAssets::balance(1984, &[2; 32].into()), 1_150_000);
        assert_ok!(PropertyManagement::distribute_income(
            RuntimeOrigin::signed([15; 32].into()),
            0,
            10000,
            1984,
        ));
        assert_eq!(ForeignAssets::balance(1984, &[30; 32].into()), 919_200);
        assert_ok!(Marketplace::send_property_shares(
            RuntimeOrigin::signed([2; 32].into()),
            0,
            [30; 32].into(),
            20
        ));
        assert_eq!(ForeignAssets::balance(1984, &[2; 32].into()), 1_152_000);
        assert_eq!(ForeignAssets::balance(1984, &[30; 32].into()), 923_400);
        assert_eq!(PropertyIncome::<Test>::get(0), 70);
        assert_eq!(OwnerCheckpoints::<Test>::get::<AccountId, u32>([2; 32].into(), 0), 70);
        assert_eq!(OwnerCheckpoints::<Test>::get::<AccountId, u32>([30; 32].into(), 0), 70);
    });
}

#[test]
fn claim_income_when_buying_shares_works() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);
        // Set up roles and region
        assert_ok!(XcavateWhitelist::add_admin(RuntimeOrigin::root(), [20; 32].into(),));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [8; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [6; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));
        new_region_helper();
        assert_ok!(Regions::create_new_location(
            RuntimeOrigin::signed([6; 32].into()),
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
            [15; 32].into(),
            pallet_xcavate_whitelist::Role::LettingAgent
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
        // Property listing process
        assert_ok!(Marketplace::list_property(
            RuntimeOrigin::signed([0; 32].into()),
            3,
            bvec![10, 10],
            4_500,
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
            LegalProperty::RealEstateDeveloperSide,
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
            LegalProperty::SpvSide,
            4_000,
        ));
        assert_ok!(Marketplace::vote_on_spv_lawyer(
            RuntimeOrigin::signed([1; 32].into()),
            0,
            pallet_marketplace::types::Vote::Yes,
            80
        ));
        assert_ok!(Marketplace::vote_on_spv_lawyer(
            RuntimeOrigin::signed([30; 32].into()),
            0,
            pallet_marketplace::types::Vote::Yes,
            30
        ));
        let expiry = frame_system::Pallet::<Test>::block_number() + LawyerVotingDuration::get();
        run_to_block(expiry);
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

        // Letting agent process
        assert_ok!(PropertyManagement::add_letting_agent(
            RuntimeOrigin::signed([15; 32].into()),
            3,
            bvec![10, 10],
        ));
        assert_ok!(PropertyManagement::letting_agent_claim_property(
            RuntimeOrigin::signed([15; 32].into()),
            0
        ));
        assert_ok!(PropertyManagement::vote_on_letting_agent(
            RuntimeOrigin::signed([1; 32].into()),
            0,
            crate::Vote::Yes,
            80
        ));
        assert_ok!(PropertyManagement::vote_on_letting_agent(
            RuntimeOrigin::signed([30; 32].into()),
            0,
            crate::Vote::Yes,
            30
        ));
        let expiry =
            frame_system::Pallet::<Test>::block_number() + LettingAgentVotingDuration::get();
        run_to_block(expiry);
        assert_ok!(PropertyManagement::finalize_letting_agent(
            RuntimeOrigin::signed([1; 32].into()),
            0,
        ));
        assert_ok!(PropertyManagement::unfreeze_letting_voting_shares(
            RuntimeOrigin::signed([1; 32].into()),
            0,
        ));

        // Income distribution and claiming
        assert_ok!(PropertyManagement::distribute_income(
            RuntimeOrigin::signed([15; 32].into()),
            0,
            4000,
            1984,
        ));
        assert_eq!(PropertyIncome::<Test>::get(0), 20);
        assert_eq!(OwnerCheckpoints::<Test>::get::<AccountId, u32>([1; 32].into(), 0), 0);
        assert_eq!(Balances::free_balance(&([15; 32].into())), 4000);
        assert_eq!(Balances::free_balance(&PropertyManagement::property_account_id(0)), 5085);
        assert_eq!(ForeignAssets::balance(1984, &PropertyManagement::property_account_id(0)), 4000);
        assert_eq!(ForeignAssets::balance(1984, &[1; 32].into()), 1_125_600);
        assert_eq!(ForeignAssets::balance(1984, &[2; 32].into()), 1_150_000);
        assert_ok!(Marketplace::relist_shares(RuntimeOrigin::signed([1; 32].into()), 0, 100, 40));
        assert_ok!(Marketplace::buy_relisted_shares(
            RuntimeOrigin::signed([2; 32].into()),
            1,
            40,
            1984
        ));
        // buying relisted shares sets the owner checkpoint of both parties to the income checkpoint
        assert_eq!(OwnerCheckpoints::<Test>::get::<AccountId, u32>([1; 32].into(), 0), 20);
        assert_eq!(OwnerCheckpoints::<Test>::get::<AccountId, u32>([2; 32].into(), 0), 20);
        assert_eq!(ForeignAssets::balance(1984, &PropertyManagement::property_account_id(0)), 2400);
        assert_eq!(ForeignAssets::balance(1984, &[1; 32].into()), 1_131_160);
        assert_eq!(ForeignAssets::balance(1984, &[2; 32].into()), 1_146_000);
        assert_ok!(PropertyManagement::distribute_income(
            RuntimeOrigin::signed([15; 32].into()),
            0,
            10000,
            1984,
        ));
        assert_eq!(ForeignAssets::balance(1984, &[30; 32].into()), 919_200);
        assert_ok!(Marketplace::relist_shares(RuntimeOrigin::signed([2; 32].into()), 0, 100, 20));
        assert_ok!(Marketplace::buy_relisted_shares(
            RuntimeOrigin::signed([30; 32].into()),
            2,
            20,
            1984
        ));
        assert_eq!(ForeignAssets::balance(1984, &[2; 32].into()), 1_149_980);
        assert_eq!(ForeignAssets::balance(1984, &[30; 32].into()), 921_400);
        assert_eq!(PropertyIncome::<Test>::get(0), 70);
        assert_eq!(OwnerCheckpoints::<Test>::get::<AccountId, u32>([2; 32].into(), 0), 70);
        assert_eq!(OwnerCheckpoints::<Test>::get::<AccountId, u32>([30; 32].into(), 0), 70);
    });
}

#[test]
fn claim_income_fails() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);
        // Set up roles and region
        assert_ok!(XcavateWhitelist::add_admin(RuntimeOrigin::root(), [20; 32].into(),));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [8; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [6; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));
        new_region_helper();
        assert_ok!(Regions::create_new_location(
            RuntimeOrigin::signed([6; 32].into()),
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
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [4; 32].into(),
            pallet_xcavate_whitelist::Role::LettingAgent
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
        // Property listing process
        assert_ok!(Marketplace::list_property(
            RuntimeOrigin::signed([0; 32].into()),
            3,
            bvec![10, 10],
            900,
            1000,
            bvec![22, 22],
            false
        ));
        assert_ok!(Marketplace::buy_property_shares(
            RuntimeOrigin::signed([1; 32].into()),
            0,
            400,
            1984
        ));
        assert_ok!(Marketplace::buy_property_shares(
            RuntimeOrigin::signed([30; 32].into()),
            0,
            300,
            1984
        ));
        assert_ok!(Marketplace::buy_property_shares(
            RuntimeOrigin::signed([31; 32].into()),
            0,
            300,
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
            LegalProperty::RealEstateDeveloperSide,
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
            LegalProperty::SpvSide,
            4_000,
        ));
        assert_ok!(Marketplace::vote_on_spv_lawyer(
            RuntimeOrigin::signed([1; 32].into()),
            0,
            pallet_marketplace::types::Vote::Yes,
            400
        ));
        assert_ok!(Marketplace::vote_on_spv_lawyer(
            RuntimeOrigin::signed([30; 32].into()),
            0,
            pallet_marketplace::types::Vote::Yes,
            300
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
        assert_eq!(LocalAssets::total_supply(0), 1000);

        // Letting agent process
        assert_ok!(PropertyManagement::add_letting_agent(
            RuntimeOrigin::signed([4; 32].into()),
            3,
            bvec![10, 10],
        ));
        assert_ok!(PropertyManagement::letting_agent_claim_property(
            RuntimeOrigin::signed([4; 32].into()),
            0
        ));
        assert_ok!(PropertyManagement::vote_on_letting_agent(
            RuntimeOrigin::signed([1; 32].into()),
            0,
            crate::Vote::Yes,
            400
        ));
        assert_ok!(PropertyManagement::vote_on_letting_agent(
            RuntimeOrigin::signed([30; 32].into()),
            0,
            crate::Vote::Yes,
            300
        ));
        let expiry =
            frame_system::Pallet::<Test>::block_number() + LettingAgentVotingDuration::get();
        run_to_block(expiry);
        assert_ok!(PropertyManagement::finalize_letting_agent(
            RuntimeOrigin::signed([1; 32].into()),
            0,
        ));

        // Income distribution and claiming
        assert_ok!(PropertyManagement::distribute_income(
            RuntimeOrigin::signed([4; 32].into()),
            0,
            3200,
            1984,
        ));
        assert_eq!(PropertyIncome::<Test>::get(0), 3);
        assert_noop!(
            PropertyManagement::claim_income(RuntimeOrigin::signed([2; 32].into()), 0),
            Error::<Test>::UserHasNoFundsStored
        );
    });
}

// resign_from_property tests

#[test]
fn resign_from_property_works() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);
        // Set up roles and region
        assert_ok!(XcavateWhitelist::add_admin(RuntimeOrigin::root(), [20; 32].into(),));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [8; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [6; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));
        new_region_helper();
        assert_ok!(Regions::create_new_location(
            RuntimeOrigin::signed([6; 32].into()),
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
            [4; 32].into(),
            pallet_xcavate_whitelist::Role::LettingAgent
        ));

        // Property listing process
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
        lawyer_process_helper([0; 32].into(), 0, vec![([1; 32].into(), 40), ([30; 32].into(), 30)]);

        // Letting agent process
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [5; 32].into(),
            pallet_xcavate_whitelist::Role::LettingAgent
        ));
        assert_noop!(
            PropertyManagement::distribute_income(
                RuntimeOrigin::signed([5; 32].into()),
                0,
                200,
                1984
            ),
            Error::<Test>::NoLettingAgentFound
        );
        assert_eq!(PropertyIncome::<Test>::get(0), 0);
        assert_eq!(OwnerCheckpoints::<Test>::get::<AccountId, u32>([1; 32].into(), 0), 0);
        assert_ok!(PropertyManagement::add_letting_agent(
            RuntimeOrigin::signed([4; 32].into()),
            3,
            bvec![10, 10],
        ));
        assert_ok!(PropertyManagement::letting_agent_claim_property(
            RuntimeOrigin::signed([4; 32].into()),
            0
        ));
        assert_ok!(PropertyManagement::vote_on_letting_agent(
            RuntimeOrigin::signed([1; 32].into()),
            0,
            crate::Vote::Yes,
            40
        ));
        assert_ok!(PropertyManagement::vote_on_letting_agent(
            RuntimeOrigin::signed([30; 32].into()),
            0,
            crate::Vote::Yes,
            30
        ));
        let expiry =
            frame_system::Pallet::<Test>::block_number() + LettingAgentVotingDuration::get();
        run_to_block(expiry);
        assert_ok!(PropertyManagement::finalize_letting_agent(
            RuntimeOrigin::signed([1; 32].into()),
            0,
        ));
        assert_ok!(PropertyManagement::distribute_income(
            RuntimeOrigin::signed([4; 32].into()),
            0,
            200,
            1984
        ));

        // Letting agent resignation
        assert_ok!(PropertyManagement::resign_from_property(
            RuntimeOrigin::signed([4; 32].into()),
            0,
        ));
        let resignation_block =
            frame_system::Pallet::<Test>::block_number() + LettingAgentNoticeTime::get();
        assert_eq!(ResignationNotices::<Test>::get(0).unwrap().letting_agent, [4; 32].into(),);
        assert_eq!(
            ResignationNotices::<Test>::get(0).unwrap().resignation_block,
            resignation_block,
        );
        assert_eq!(ResignationQueue::<Test>::get(resignation_block)[0], 0);
        assert_eq!(
            LettingInfo::<Test>::get::<AccountId>([4; 32].into())
                .unwrap()
                .locations
                .get(&bvec![10, 10])
                .clone()
                .unwrap()
                .assigned_properties,
            1
        );
        assert_eq!(LettingStorage::<Test>::get(0).unwrap(), [4; 32].into());
        run_to_block(resignation_block);
        assert!(ResignationNotices::<Test>::get(0).is_none());
        assert_eq!(
            LettingInfo::<Test>::get::<AccountId>([4; 32].into())
                .unwrap()
                .locations
                .get(&bvec![10, 10])
                .clone()
                .unwrap()
                .assigned_properties,
            0
        );
        assert!(LettingStorage::<Test>::get(0).is_none());
    });
}

#[test]
fn resign_from_property_fails() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);
        // Set up roles and region
        assert_ok!(XcavateWhitelist::add_admin(RuntimeOrigin::root(), [20; 32].into(),));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [8; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [6; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));
        new_region_helper();
        assert_ok!(Regions::create_new_location(
            RuntimeOrigin::signed([6; 32].into()),
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
            [4; 32].into(),
            pallet_xcavate_whitelist::Role::LettingAgent
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [2; 32].into(),
            pallet_xcavate_whitelist::Role::LettingAgent
        ));

        // Property listing process
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
        lawyer_process_helper([0; 32].into(), 0, vec![([1; 32].into(), 40), ([30; 32].into(), 30)]);

        // Letting agent process
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [5; 32].into(),
            pallet_xcavate_whitelist::Role::LettingAgent
        ));
        assert_eq!(PropertyIncome::<Test>::get(0), 0);
        assert_eq!(OwnerCheckpoints::<Test>::get::<AccountId, u32>([1; 32].into(), 0), 0);
        assert_ok!(PropertyManagement::add_letting_agent(
            RuntimeOrigin::signed([4; 32].into()),
            3,
            bvec![10, 10],
        ));
        assert_ok!(PropertyManagement::letting_agent_claim_property(
            RuntimeOrigin::signed([4; 32].into()),
            0
        ));
        assert_ok!(PropertyManagement::vote_on_letting_agent(
            RuntimeOrigin::signed([1; 32].into()),
            0,
            crate::Vote::Yes,
            40
        ));
        assert_ok!(PropertyManagement::vote_on_letting_agent(
            RuntimeOrigin::signed([30; 32].into()),
            0,
            crate::Vote::Yes,
            30
        ));
        let expiry =
            frame_system::Pallet::<Test>::block_number() + LettingAgentVotingDuration::get();
        assert_noop!(
            PropertyManagement::resign_from_property(RuntimeOrigin::signed([4; 32].into()), 0,),
            Error::<Test>::NoLettingAgentFound
        );
        run_to_block(expiry);
        assert_ok!(PropertyManagement::finalize_letting_agent(
            RuntimeOrigin::signed([1; 32].into()),
            0,
        ));
        assert_ok!(PropertyManagement::distribute_income(
            RuntimeOrigin::signed([4; 32].into()),
            0,
            200,
            1984
        ));

        // Letting agent resignation failure cases
        assert_noop!(
            PropertyManagement::resign_from_property(RuntimeOrigin::signed([1; 32].into()), 0,),
            BadOrigin
        );
        assert_noop!(
            PropertyManagement::resign_from_property(RuntimeOrigin::signed([2; 32].into()), 0,),
            Error::<Test>::NoPermission
        );
        assert_ok!(PropertyManagement::resign_from_property(
            RuntimeOrigin::signed([4; 32].into()),
            0,
        ));
        assert_noop!(
            PropertyManagement::resign_from_property(RuntimeOrigin::signed([4; 32].into()), 0,),
            Error::<Test>::ResignationAlreadyInitiated
        );
    });
}
