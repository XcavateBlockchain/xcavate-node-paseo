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

use crate::{mock::*, Error, Event};
use frame_support::{
    assert_noop, assert_ok,
    sp_runtime::{Percent, Permill},
    traits::{fungible::InspectHold, fungibles::InspectFreeze, OnFinalize, OnInitialize},
};

use crate::{
    AssetLettingChallenge, AssetProposal, ChallengeRoundsExpiring, Challenges,
    LastAutoExecutionBlock, OngoingChallengeVotes, OngoingProposalVotes, Proposals,
    UserChallengeVote, UserProposalVote, VoteRecord,
};

use pallet_property_management::{LettingInfo, LettingStorage, OwnerCheckpoints, PropertyIncome};

use pallet_marketplace::types::LegalProperty;

use pallet_regions::RegionIdentifier;

use primitives::MarketplaceFreezeReason;

macro_rules! bvec {
	($( $x:tt )*) => {
		vec![$( $x )*].try_into().unwrap()
	}
}

fn run_to_block(n: u64) {
    while System::block_number() < n {
        if System::block_number() > 0 {
            PropertyGovernance::on_finalize(System::block_number());
            System::on_finalize(System::block_number());
        }
        System::reset_events();
        System::set_block_number(System::block_number() + 1);
        System::on_initialize(System::block_number());
        PropertyGovernance::on_initialize(System::block_number());
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

fn listing_process() {
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
}

fn setting_letting_agent(agent: AccountId, voters: Vec<(AccountId, u32)>) {
    assert_ok!(PropertyManagement::add_letting_agent(
        RuntimeOrigin::signed(agent.clone()),
        3,
        bvec![10, 10],
    ));
    assert_ok!(PropertyManagement::letting_agent_claim_property(
        RuntimeOrigin::signed(agent.clone()),
        0
    ));
    for voter in &voters {
        assert_ok!(PropertyManagement::vote_on_letting_agent(
            RuntimeOrigin::signed(voter.0.clone()),
            0,
            pallet_property_management::Vote::Yes,
            voter.1
        ));
    }
    let expiry = frame_system::Pallet::<Test>::block_number() + LettingAgentVotingDuration::get();
    frame_system::Pallet::<Test>::set_block_number(expiry);
    assert_ok!(PropertyManagement::finalize_letting_agent(
        RuntimeOrigin::signed(voters[0].0.clone()),
        0,
    ));
    assert_eq!(LettingStorage::<Test>::get(0).unwrap(), agent);
}

fn lawyer_process(accounts: Vec<(AccountId, u32)>) {
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
    for account in accounts {
        assert_ok!(Marketplace::vote_on_spv_lawyer(
            RuntimeOrigin::signed(account.0),
            0,
            pallet_marketplace::types::Vote::Yes,
            account.1,
        ));
    }
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
}

// propose tests

#[test]
fn propose_works() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);
        // Listing setup and role assignments
        assert_ok!(XcavateWhitelist::add_admin(RuntimeOrigin::root(), [20; 32].into(),));
        listing_process();
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
        lawyer_process(vec![([1; 32].into(), 40), ([30; 32].into(), 30)]);

        // Letting agent process
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [2; 32].into(),
            pallet_xcavate_whitelist::Role::LettingAgent
        ));
        setting_letting_agent([2; 32].into(), vec![([1; 32].into(), 40), ([30; 32].into(), 30)]);
        assert_eq!(LettingStorage::<Test>::get(0).unwrap(), [2; 32].into());

        // Proposal process
        assert_ok!(PropertyGovernance::propose(
            RuntimeOrigin::signed([2; 32].into()),
            0,
            1000,
            bvec![10, 10]
        ));
        assert_eq!(Proposals::<Test>::get(0).is_some(), true);
        assert_eq!(OngoingProposalVotes::<Test>::get(0).is_some(), true);
        assert_eq!(AssetProposal::<Test>::get(0).unwrap(), 0);
    });
}

#[test]
fn proposal_with_low_amount_works() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);
        // Listing setup and role assignments
        assert_ok!(XcavateWhitelist::add_admin(RuntimeOrigin::root(), [20; 32].into(),));
        listing_process();
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
        lawyer_process(vec![([1; 32].into(), 40), ([30; 32].into(), 30)]);

        // Letting agent process
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [4; 32].into(),
            pallet_xcavate_whitelist::Role::LettingAgent
        ));
        setting_letting_agent([4; 32].into(), vec![([1; 32].into(), 40), ([30; 32].into(), 30)]);
        assert_eq!(LettingStorage::<Test>::get(0).unwrap(), [4; 32].into());
        assert_ok!(PropertyManagement::distribute_income(
            RuntimeOrigin::signed([4; 32].into()),
            0,
            1000,
            1984,
        ));

        // Proposal process
        assert_ok!(PropertyGovernance::propose(
            RuntimeOrigin::signed([4; 32].into()),
            0,
            500,
            bvec![10, 10]
        ));
        System::assert_last_event(Event::ProposalExecuted { asset_id: 0, amount: 500 }.into());
        assert_eq!(Balances::free_balance(&([4; 32].into())), 4000);
        assert_eq!(ForeignAssets::balance(1984, &[4; 32].into()), 4000);
        assert_eq!(OngoingProposalVotes::<Test>::get(0).is_some(), false);
        assert_eq!(
            LastAutoExecutionBlock::<Test>::get(0).unwrap(),
            frame_system::Pallet::<Test>::block_number()
        );

        let expiry =
            frame_system::Pallet::<Test>::block_number() + AutoExecutionCooldown::get() + 1;
        run_to_block(expiry);

        assert_ok!(PropertyGovernance::propose(
            RuntimeOrigin::signed([4; 32].into()),
            0,
            400,
            bvec![10, 10]
        ));
        System::assert_last_event(Event::ProposalExecuted { asset_id: 0, amount: 400 }.into());
        assert_eq!(
            LastAutoExecutionBlock::<Test>::get(0).unwrap(),
            frame_system::Pallet::<Test>::block_number()
        );
    });
}

#[test]
fn propose_fails() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);
        // Listing setup and role assignments
        assert_ok!(XcavateWhitelist::add_admin(RuntimeOrigin::root(), [20; 32].into(),));
        listing_process();
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
            pallet_xcavate_whitelist::Role::LettingAgent
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
        assert_noop!(
            PropertyGovernance::propose(
                RuntimeOrigin::signed([2; 32].into()),
                0,
                1000,
                bvec![10, 10]
            ),
            Error::<Test>::NoLettingAgentFound
        );
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [5; 32].into(),
            pallet_xcavate_whitelist::Role::SpvConfirmation
        ));
        assert_ok!(Marketplace::create_spv(RuntimeOrigin::signed([5; 32].into()), 0,));
        assert_ok!(Marketplace::claim_property_shares(RuntimeOrigin::signed([1; 32].into()), 0,));
        assert_ok!(Marketplace::claim_property_shares(RuntimeOrigin::signed([30; 32].into()), 0,));
        assert_ok!(Marketplace::claim_property_shares(RuntimeOrigin::signed([31; 32].into()), 0,));
        lawyer_process(vec![([1; 32].into(), 40), ([30; 32].into(), 30)]);

        // Letting agent process
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [0; 32].into(),
            pallet_xcavate_whitelist::Role::LettingAgent
        ));
        setting_letting_agent([0; 32].into(), vec![([1; 32].into(), 40), ([30; 32].into(), 30)]);
        assert_eq!(LettingStorage::<Test>::get(0).unwrap(), [0; 32].into());

        // Proposal process
        assert_noop!(
            PropertyGovernance::propose(
                RuntimeOrigin::signed([2; 32].into()),
                0,
                1000,
                bvec![10, 10]
            ),
            Error::<Test>::NoPermission
        );
        assert_ok!(PropertyGovernance::propose(
            RuntimeOrigin::signed([0; 32].into()),
            0,
            1000,
            bvec![10, 10]
        ));
        assert_noop!(
            PropertyGovernance::propose(
                RuntimeOrigin::signed([0; 32].into()),
                0,
                1000,
                bvec![10, 10]
            ),
            Error::<Test>::ProposalOngoing
        );
    });
}

#[test]
fn propose_multiple_auto_execution_fails() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);
        // Listing setup and role assignments
        assert_ok!(XcavateWhitelist::add_admin(RuntimeOrigin::root(), [20; 32].into(),));
        listing_process();
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
            pallet_xcavate_whitelist::Role::LettingAgent
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
        lawyer_process(vec![([1; 32].into(), 40), ([30; 32].into(), 30)]);

        // Letting agent process
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [0; 32].into(),
            pallet_xcavate_whitelist::Role::LettingAgent
        ));
        setting_letting_agent([0; 32].into(), vec![([1; 32].into(), 40), ([30; 32].into(), 30)]);
        assert_eq!(LettingStorage::<Test>::get(0).unwrap(), [0; 32].into());

        // Proposal process
        assert_ok!(PropertyGovernance::propose(
            RuntimeOrigin::signed([0; 32].into()),
            0,
            500,
            bvec![10, 10]
        ));
        assert_noop!(
            PropertyGovernance::propose(
                RuntimeOrigin::signed([0; 32].into()),
                0,
                500,
                bvec![10, 10]
            ),
            Error::<Test>::AutoExecutionTooSoon
        );
    });
}

// challenge_against_letting_agent tests

#[test]
fn challenge_against_letting_agent_works() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);
        // Listing setup and role assignments
        assert_ok!(XcavateWhitelist::add_admin(RuntimeOrigin::root(), [20; 32].into(),));
        listing_process();
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
        lawyer_process(vec![([1; 32].into(), 40), ([30; 32].into(), 30)]);

        // Letting agent process
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [0; 32].into(),
            pallet_xcavate_whitelist::Role::LettingAgent
        ));
        setting_letting_agent([0; 32].into(), vec![([1; 32].into(), 40), ([30; 32].into(), 30)]);

        // Challenge process
        assert_ok!(PropertyGovernance::challenge_against_letting_agent(
            RuntimeOrigin::signed([1; 32].into()),
            0
        ));
        assert_eq!(AssetLettingChallenge::<Test>::get(0).is_some(), true);
        assert_eq!(OngoingChallengeVotes::<Test>::get(0).is_some(), true);
        assert_eq!(Challenges::<Test>::get(0).is_some(), true);
    });
}

#[test]
fn challenge_against_letting_agent_fails() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);
        // Listing setup and role assignments
        assert_ok!(XcavateWhitelist::add_admin(RuntimeOrigin::root(), [20; 32].into(),));
        listing_process();
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
        lawyer_process(vec![([1; 32].into(), 40), ([30; 32].into(), 30)]);

        // Letting agent process
        assert_noop!(
            PropertyGovernance::challenge_against_letting_agent(
                RuntimeOrigin::signed([1; 32].into()),
                0
            ),
            Error::<Test>::NoLettingAgentFound
        );
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [0; 32].into(),
            pallet_xcavate_whitelist::Role::LettingAgent
        ));
        setting_letting_agent([0; 32].into(), vec![([1; 32].into(), 40), ([30; 32].into(), 30)]);

        // Challenge process
        assert_noop!(
            PropertyGovernance::challenge_against_letting_agent(
                RuntimeOrigin::signed([2; 32].into()),
                0
            ),
            Error::<Test>::NoPermission
        );
        assert_eq!(Challenges::<Test>::get(0).is_some(), false);
        assert_ok!(PropertyGovernance::challenge_against_letting_agent(
            RuntimeOrigin::signed([1; 32].into()),
            0
        ));
        assert_noop!(
            PropertyGovernance::challenge_against_letting_agent(
                RuntimeOrigin::signed([1; 32].into()),
                0
            ),
            Error::<Test>::ChallengeAlreadyOngoing
        );
    });
}

// vote_on_proposal tests

#[test]
fn vote_on_proposal_works() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);
        // Listing setup and role assignments
        assert_ok!(XcavateWhitelist::add_admin(RuntimeOrigin::root(), [20; 32].into(),));
        listing_process();
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
        assert_ok!(Marketplace::buy_property_shares(
            RuntimeOrigin::signed([1; 32].into()),
            0,
            30,
            1984
        ));
        assert_ok!(Marketplace::buy_property_shares(
            RuntimeOrigin::signed([1; 32].into()),
            0,
            15,
            1984
        ));
        assert_ok!(Marketplace::buy_property_shares(
            RuntimeOrigin::signed([2; 32].into()),
            0,
            15,
            1984
        ));
        assert_ok!(Marketplace::buy_property_shares(
            RuntimeOrigin::signed([3; 32].into()),
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
        assert_ok!(Marketplace::claim_property_shares(RuntimeOrigin::signed([3; 32].into()), 0));
        lawyer_process(vec![([1; 32].into(), 20), ([3; 32].into(), 40)]);

        // Letting agent process
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [0; 32].into(),
            pallet_xcavate_whitelist::Role::LettingAgent
        ));
        setting_letting_agent([0; 32].into(), vec![([1; 32].into(), 20), ([3; 32].into(), 40)]);
        assert_eq!(LettingStorage::<Test>::get(0).unwrap(), [0; 32].into());
        assert_ok!(PropertyManagement::distribute_income(
            RuntimeOrigin::signed([0; 32].into()),
            0,
            1000,
            1984,
        ));

        // Proposal process
        assert_ok!(PropertyGovernance::propose(
            RuntimeOrigin::signed([0; 32].into()),
            0,
            1000,
            bvec![10, 10]
        ));
        assert_ok!(PropertyGovernance::vote_on_proposal(
            RuntimeOrigin::signed([1; 32].into()),
            0,
            crate::Vote::Yes,
            45
        ));
        assert_eq!(
            AssetsFreezer::balance_frozen(
                0,
                &MarketplaceFreezeReason::ProposalVoting,
                &[1; 32].into()
            ),
            45
        );
        assert_eq!(
            UserProposalVote::<Test>::get::<u64, AccountId>(0, [1; 32].into()).unwrap(),
            VoteRecord { vote: crate::Vote::Yes, asset_id: 0, power: 45 }
        );
        assert_ok!(PropertyGovernance::vote_on_proposal(
            RuntimeOrigin::signed([2; 32].into()),
            0,
            crate::Vote::Yes,
            10
        ));
        assert_ok!(PropertyGovernance::vote_on_proposal(
            RuntimeOrigin::signed([3; 32].into()),
            0,
            crate::Vote::No,
            40
        ));
        assert_ok!(PropertyGovernance::vote_on_proposal(
            RuntimeOrigin::signed([1; 32].into()),
            0,
            crate::Vote::No,
            30
        ));
        assert_eq!(OngoingProposalVotes::<Test>::get(0).unwrap().yes_voting_power, 10);
        assert_eq!(OngoingProposalVotes::<Test>::get(0).unwrap().no_voting_power, 70);
        assert_eq!(
            AssetsFreezer::balance_frozen(
                0,
                &MarketplaceFreezeReason::ProposalVoting,
                &[1; 32].into()
            ),
            30
        );
        assert_eq!(
            UserProposalVote::<Test>::get::<u64, AccountId>(0, [1; 32].into()).unwrap(),
            VoteRecord { vote: crate::Vote::No, asset_id: 0, power: 30 }
        );
    });
}

#[test]
fn proposal_pass() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);
        // Listing setup and role assignments
        assert_ok!(XcavateWhitelist::add_admin(RuntimeOrigin::root(), [20; 32].into(),));
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
        assert_ok!(Marketplace::list_property(
            RuntimeOrigin::signed([2; 32].into()),
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
        assert_ok!(Regions::register_lawyer(RuntimeOrigin::signed([10; 32].into()), 3,));
        assert_ok!(Regions::register_lawyer(RuntimeOrigin::signed([11; 32].into()), 3,));
        assert_ok!(Marketplace::lawyer_claim_property(
            RuntimeOrigin::signed([10; 32].into()),
            0,
            LegalProperty::RealEstateDeveloperSide,
            4_000,
        ));
        assert_ok!(Marketplace::approve_developer_lawyer(
            RuntimeOrigin::signed([2; 32].into()),
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
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [0; 32].into(),
            pallet_xcavate_whitelist::Role::LettingAgent
        ));
        setting_letting_agent([0; 32].into(), vec![([1; 32].into(), 40), ([30; 32].into(), 30)]);
        assert_eq!(LettingStorage::<Test>::get(0).unwrap(), [0; 32].into());
        assert_eq!(ForeignAssets::balance(1984, &[0; 32].into()), 20_000_000);
        assert_ok!(PropertyManagement::distribute_income(
            RuntimeOrigin::signed([0; 32].into()),
            0,
            1000,
            1984,
        ));

        // Proposal process
        assert_ok!(PropertyGovernance::propose(
            RuntimeOrigin::signed([0; 32].into()),
            0,
            1000,
            bvec![10, 10]
        ));
        assert_ok!(PropertyGovernance::vote_on_proposal(
            RuntimeOrigin::signed([1; 32].into()),
            0,
            crate::Vote::Yes,
            40
        ));
        assert_ok!(PropertyGovernance::vote_on_proposal(
            RuntimeOrigin::signed([30; 32].into()),
            0,
            crate::Vote::Yes,
            30
        ));
        assert_eq!(Proposals::<Test>::get(0).is_some(), true);
        assert_eq!(Balances::free_balance(&([0; 32].into())), 19_999_000);
        assert_eq!(ForeignAssets::balance(1984, &[0; 32].into()), 19_999_000);
        assert_eq!(
            ForeignAssets::balance(1984, &PropertyGovernance::property_account_id(0)),
            1_000
        );
        let expiry = frame_system::Pallet::<Test>::block_number() + PropertyVotingTime::get();
        run_to_block(expiry);
        System::assert_last_event(Event::ProposalExecuted { asset_id: 0, amount: 1000 }.into());
        assert_eq!(Proposals::<Test>::get(0).is_none(), true);
        assert_eq!(OngoingProposalVotes::<Test>::get(0).is_none(), true);
    });
}

#[test]
fn proposal_pass_2() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);
        // Listing setup and role assignments
        assert_ok!(XcavateWhitelist::add_admin(RuntimeOrigin::root(), [20; 32].into(),));
        listing_process();
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
        lawyer_process(vec![([1; 32].into(), 40), ([30; 32].into(), 30)]);

        // Letting agent process
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [4; 32].into(),
            pallet_xcavate_whitelist::Role::LettingAgent
        ));
        setting_letting_agent([4; 32].into(), vec![([1; 32].into(), 40), ([30; 32].into(), 30)]);
        assert_eq!(LettingStorage::<Test>::get(0).unwrap(), [4; 32].into());

        // Proposal process
        assert_ok!(PropertyGovernance::propose(
            RuntimeOrigin::signed([4; 32].into()),
            0,
            10000,
            bvec![10, 10]
        ));
        assert_ok!(PropertyGovernance::vote_on_proposal(
            RuntimeOrigin::signed([1; 32].into()),
            0,
            crate::Vote::No,
            40
        ));
        assert_ok!(PropertyGovernance::vote_on_proposal(
            RuntimeOrigin::signed([1; 32].into()),
            0,
            crate::Vote::Yes,
            40
        ));
        assert_ok!(PropertyGovernance::vote_on_proposal(
            RuntimeOrigin::signed([30; 32].into()),
            0,
            crate::Vote::Yes,
            30
        ));
        assert_eq!(Proposals::<Test>::get(0).is_some(), true);
        let expiry = frame_system::Pallet::<Test>::block_number() + PropertyVotingTime::get();
        run_to_block(expiry);
        System::assert_last_event(Event::ProposalExecuted { asset_id: 0, amount: 10000 }.into());
        assert_eq!(Proposals::<Test>::get(0).is_none(), true);
        assert_eq!(PropertyIncome::<Test>::get(0), 0);
        assert_eq!(OwnerCheckpoints::<Test>::get::<AccountId, u32>([1; 32].into(), 0), 0);
    });
}

#[test]
fn proposal_not_pass() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);
        // Listing setup and role assignments
        assert_ok!(XcavateWhitelist::add_admin(RuntimeOrigin::root(), [20; 32].into(),));
        listing_process();
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
        lawyer_process(vec![([1; 32].into(), 40), ([2; 32].into(), 30)]);

        // Letting agent process
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [4; 32].into(),
            pallet_xcavate_whitelist::Role::LettingAgent
        ));
        setting_letting_agent([4; 32].into(), vec![([1; 32].into(), 40), ([2; 32].into(), 30)]);
        assert_eq!(LettingStorage::<Test>::get(0).unwrap(), [4; 32].into());
        assert_ok!(PropertyManagement::distribute_income(
            RuntimeOrigin::signed([4; 32].into()),
            0,
            1000,
            1984,
        ));

        // Proposal process
        assert_ok!(PropertyGovernance::propose(
            RuntimeOrigin::signed([4; 32].into()),
            0,
            1000,
            bvec![10, 10]
        ));
        assert_ok!(PropertyGovernance::vote_on_proposal(
            RuntimeOrigin::signed([1; 32].into()),
            0,
            crate::Vote::No,
            40
        ));
        assert_ok!(PropertyGovernance::vote_on_proposal(
            RuntimeOrigin::signed([2; 32].into()),
            0,
            crate::Vote::No,
            30
        ));
        assert_eq!(Proposals::<Test>::get(0).is_some(), true);
        assert_eq!(ForeignAssets::balance(1984, &[4; 32].into()), 4000);
        assert_eq!(Balances::free_balance(&([0; 32].into())), 19_999_900);
        assert_eq!(ForeignAssets::balance(1984, &PropertyGovernance::property_account_id(0)), 1000);
        let expiry = frame_system::Pallet::<Test>::block_number() + PropertyVotingTime::get();
        run_to_block(expiry);
        assert_eq!(ForeignAssets::balance(1984, &[4; 32].into()), 4000);
        assert_eq!(ForeignAssets::balance(1984, &PropertyGovernance::property_account_id(0)), 1000);
        assert_eq!(Proposals::<Test>::get(0).is_none(), true);
        System::assert_last_event(Event::ProposalRejected { proposal_id: 0 }.into());
    });
}

#[test]
fn proposal_not_pass_2() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);
        // Listing setup and role assignments
        assert_ok!(XcavateWhitelist::add_admin(RuntimeOrigin::root(), [20; 32].into(),));
        listing_process();
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
            1984
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
        assert_ok!(Marketplace::claim_property_shares(RuntimeOrigin::signed([1; 32].into()), 0));
        assert_ok!(Marketplace::claim_property_shares(RuntimeOrigin::signed([2; 32].into()), 0));
        assert_ok!(Marketplace::claim_property_shares(RuntimeOrigin::signed([30; 32].into()), 0));
        lawyer_process(vec![([1; 32].into(), 40), ([2; 32].into(), 40)]);

        // Letting agent process
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [4; 32].into(),
            pallet_xcavate_whitelist::Role::LettingAgent
        ));
        setting_letting_agent([4; 32].into(), vec![([1; 32].into(), 40), ([2; 32].into(), 40)]);
        assert_eq!(LettingStorage::<Test>::get(0).unwrap(), [4; 32].into());
        assert_ok!(PropertyManagement::distribute_income(
            RuntimeOrigin::signed([4; 32].into()),
            0,
            1000,
            1984,
        ));

        // Proposal process
        assert_ok!(PropertyGovernance::propose(
            RuntimeOrigin::signed([4; 32].into()),
            0,
            10000,
            bvec![10, 10]
        ));
        assert_ok!(PropertyGovernance::vote_on_proposal(
            RuntimeOrigin::signed([1; 32].into()),
            0,
            crate::Vote::Yes,
            45
        ));
        assert_ok!(PropertyGovernance::vote_on_proposal(
            RuntimeOrigin::signed([2; 32].into()),
            0,
            crate::Vote::No,
            25
        ));
        assert_ok!(PropertyGovernance::vote_on_proposal(
            RuntimeOrigin::signed([30; 32].into()),
            0,
            crate::Vote::Abstain,
            15
        ));
        assert_eq!(Proposals::<Test>::get(0).is_some(), true);
        assert_eq!(Proposals::<Test>::get(0).unwrap().amount, 10000);
        assert_eq!(ForeignAssets::balance(1984, &[4; 32].into()), 4000);
        assert_eq!(ForeignAssets::balance(1984, &PropertyGovernance::property_account_id(0)), 1000);
        let expiry = frame_system::Pallet::<Test>::block_number() + PropertyVotingTime::get();
        run_to_block(expiry);
        System::assert_last_event(
            Event::ProposalThresHoldNotReached {
                proposal_id: 0,
                required_threshold: Percent::from_percent(67),
            }
            .into(),
        );
        assert_eq!(Proposals::<Test>::get(0).is_none(), true);
        assert_eq!(ForeignAssets::balance(1984, &[4; 32].into()), 4000);
        assert_eq!(ForeignAssets::balance(1984, &PropertyGovernance::property_account_id(0)), 1000);
    });
}

#[test]
fn proposal_not_pass_3() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);
        // Listing setup and role assignments
        assert_ok!(XcavateWhitelist::add_admin(RuntimeOrigin::root(), [20; 32].into(),));
        listing_process();
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
            1984
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
        assert_ok!(Marketplace::claim_property_shares(RuntimeOrigin::signed([1; 32].into()), 0));
        assert_ok!(Marketplace::claim_property_shares(RuntimeOrigin::signed([2; 32].into()), 0));
        assert_ok!(Marketplace::claim_property_shares(RuntimeOrigin::signed([30; 32].into()), 0));
        lawyer_process(vec![([1; 32].into(), 40), ([2; 32].into(), 35)]);

        // Letting agent process
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [4; 32].into(),
            pallet_xcavate_whitelist::Role::LettingAgent
        ));
        setting_letting_agent([4; 32].into(), vec![([1; 32].into(), 45), ([2; 32].into(), 35)]);
        assert_eq!(LettingStorage::<Test>::get(0).unwrap(), [4; 32].into());
        assert_ok!(PropertyManagement::distribute_income(
            RuntimeOrigin::signed([4; 32].into()),
            0,
            1000,
            1984,
        ));

        // Proposal process
        assert_ok!(PropertyGovernance::propose(
            RuntimeOrigin::signed([4; 32].into()),
            0,
            1000,
            bvec![10, 10]
        ));
        assert_ok!(PropertyGovernance::vote_on_proposal(
            RuntimeOrigin::signed([1; 32].into()),
            0,
            crate::Vote::Yes,
            40
        ));
        assert_eq!(Proposals::<Test>::get(0).is_some(), true);
        assert_eq!(Proposals::<Test>::get(0).unwrap().amount, 1000);
        assert_eq!(ForeignAssets::balance(1984, &[4; 32].into()), 4000);
        assert_eq!(ForeignAssets::balance(1984, &PropertyGovernance::property_account_id(0)), 1000);
        let expiry = frame_system::Pallet::<Test>::block_number() + PropertyVotingTime::get();
        run_to_block(expiry);
        System::assert_last_event(Event::ProposalRejected { proposal_id: 0 }.into());
        assert_eq!(Proposals::<Test>::get(0).is_none(), true);
        assert_eq!(ForeignAssets::balance(1984, &[4; 32].into()), 4000);
        assert_eq!(ForeignAssets::balance(1984, &PropertyGovernance::property_account_id(0)), 1000);
    });
}

#[test]
fn vote_on_proposal_fails() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);
        // Listing setup and role assignments
        assert_ok!(XcavateWhitelist::add_admin(RuntimeOrigin::root(), [20; 32].into(),));
        listing_process();
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
        lawyer_process(vec![([1; 32].into(), 40), ([30; 32].into(), 30)]);
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [0; 32].into(),
            pallet_xcavate_whitelist::Role::LettingAgent
        ));

        // Letting agent process
        setting_letting_agent([0; 32].into(), vec![([1; 32].into(), 40), ([30; 32].into(), 30)]);
        assert_eq!(LettingStorage::<Test>::get(0).unwrap(), [0; 32].into());
        assert_noop!(
            PropertyGovernance::vote_on_proposal(
                RuntimeOrigin::signed([1; 32].into()),
                0,
                crate::Vote::Yes,
                40
            ),
            Error::<Test>::NotOngoing
        );
        assert_ok!(PropertyManagement::distribute_income(
            RuntimeOrigin::signed([0; 32].into()),
            0,
            1000,
            1984,
        ));

        // Proposal process
        assert_ok!(PropertyGovernance::propose(
            RuntimeOrigin::signed([0; 32].into()),
            0,
            1000,
            bvec![10, 10]
        ));
        assert_noop!(
            PropertyGovernance::vote_on_proposal(
                RuntimeOrigin::signed([1; 32].into()),
                0,
                crate::Vote::Yes,
                0
            ),
            Error::<Test>::ZeroVoteAmount
        );
        assert_ok!(PropertyGovernance::vote_on_proposal(
            RuntimeOrigin::signed([1; 32].into()),
            0,
            crate::Vote::Yes,
            40
        ));
        assert_ok!(PropertyGovernance::vote_on_proposal(
            RuntimeOrigin::signed([30; 32].into()),
            0,
            crate::Vote::Yes,
            30
        ));
        assert_noop!(
            PropertyGovernance::vote_on_proposal(
                RuntimeOrigin::signed([2; 32].into()),
                0,
                crate::Vote::Yes,
                100
            ),
            Error::<Test>::NoPermission
        );
        let expiry = frame_system::Pallet::<Test>::block_number() + PropertyVotingTime::get();
        run_to_block(expiry);
        assert_noop!(
            PropertyGovernance::vote_on_proposal(
                RuntimeOrigin::signed([1; 32].into()),
                0,
                crate::Vote::Yes,
                40
            ),
            Error::<Test>::NotOngoing
        );
    });
}

// unfreeze_proposal_shares tests

#[test]
fn unfreeze_proposal_shares_works() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);

        // Listing setup and role assignments
        assert_ok!(XcavateWhitelist::add_admin(RuntimeOrigin::root(), [20; 32].into(),));
        listing_process();
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
        assert_ok!(Marketplace::buy_property_shares(
            RuntimeOrigin::signed([1; 32].into()),
            0,
            30,
            1984
        ));
        assert_ok!(Marketplace::buy_property_shares(
            RuntimeOrigin::signed([1; 32].into()),
            0,
            15,
            1984
        ));
        assert_ok!(Marketplace::buy_property_shares(
            RuntimeOrigin::signed([2; 32].into()),
            0,
            15,
            1984
        ));
        assert_ok!(Marketplace::buy_property_shares(
            RuntimeOrigin::signed([3; 32].into()),
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
        assert_ok!(Marketplace::claim_property_shares(RuntimeOrigin::signed([3; 32].into()), 0));
        lawyer_process(vec![([1; 32].into(), 20), ([3; 32].into(), 40)]);

        // Letting agent process
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [0; 32].into(),
            pallet_xcavate_whitelist::Role::LettingAgent
        ));
        setting_letting_agent([0; 32].into(), vec![([1; 32].into(), 20), ([3; 32].into(), 40)]);
        assert_ok!(PropertyManagement::distribute_income(
            RuntimeOrigin::signed([0; 32].into()),
            0,
            1000,
            1984,
        ));

        // Proposal process
        assert_ok!(PropertyGovernance::propose(
            RuntimeOrigin::signed([0; 32].into()),
            0,
            1000,
            bvec![10, 10]
        ));
        assert_ok!(PropertyGovernance::vote_on_proposal(
            RuntimeOrigin::signed([1; 32].into()),
            0,
            crate::Vote::Yes,
            45
        ));
        assert_eq!(
            AssetsFreezer::balance_frozen(
                0,
                &MarketplaceFreezeReason::ProposalVoting,
                &[1; 32].into()
            ),
            45
        );
        assert_ok!(PropertyGovernance::vote_on_proposal(
            RuntimeOrigin::signed([2; 32].into()),
            0,
            crate::Vote::Yes,
            10
        ));
        assert_ok!(PropertyGovernance::vote_on_proposal(
            RuntimeOrigin::signed([3; 32].into()),
            0,
            crate::Vote::No,
            40
        ));
        assert_ok!(PropertyGovernance::vote_on_proposal(
            RuntimeOrigin::signed([1; 32].into()),
            0,
            crate::Vote::Abstain,
            30
        ));
        assert_eq!(
            AssetsFreezer::balance_frozen(
                0,
                &MarketplaceFreezeReason::ProposalVoting,
                &[1; 32].into()
            ),
            30
        );
        assert_eq!(
            UserProposalVote::<Test>::get::<u64, AccountId>(0, [1; 32].into()).unwrap(),
            VoteRecord { vote: crate::Vote::Abstain, asset_id: 0, power: 30 }
        );
        let expiry = frame_system::Pallet::<Test>::block_number() + PropertyVotingTime::get();
        run_to_block(expiry);
        assert_ok!(PropertyGovernance::unfreeze_proposal_shares(
            RuntimeOrigin::signed([1; 32].into()),
            0,
        ));
        assert!(UserProposalVote::<Test>::get::<u64, AccountId>(0, [1; 32].into()).is_none());
        assert_eq!(
            AssetsFreezer::balance_frozen(
                0,
                &MarketplaceFreezeReason::ProposalVoting,
                &[1; 32].into()
            ),
            0
        );
    });
}

#[test]
fn unfreeze_proposal_shares_fails() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);
        // Listing setup and role assignments
        assert_ok!(XcavateWhitelist::add_admin(RuntimeOrigin::root(), [20; 32].into(),));
        listing_process();
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
        assert_ok!(Marketplace::buy_property_shares(
            RuntimeOrigin::signed([1; 32].into()),
            0,
            30,
            1984
        ));
        assert_ok!(Marketplace::buy_property_shares(
            RuntimeOrigin::signed([1; 32].into()),
            0,
            15,
            1984
        ));
        assert_ok!(Marketplace::buy_property_shares(
            RuntimeOrigin::signed([2; 32].into()),
            0,
            15,
            1984
        ));
        assert_ok!(Marketplace::buy_property_shares(
            RuntimeOrigin::signed([3; 32].into()),
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
        assert_ok!(Marketplace::claim_property_shares(RuntimeOrigin::signed([3; 32].into()), 0));
        lawyer_process(vec![([1; 32].into(), 20), ([3; 32].into(), 40)]);

        // Letting agent process
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [0; 32].into(),
            pallet_xcavate_whitelist::Role::LettingAgent
        ));
        setting_letting_agent([0; 32].into(), vec![([1; 32].into(), 20), ([3; 32].into(), 40)]);
        assert_ok!(PropertyManagement::distribute_income(
            RuntimeOrigin::signed([0; 32].into()),
            0,
            1000,
            1984,
        ));

        // Proposal process
        assert_ok!(PropertyGovernance::propose(
            RuntimeOrigin::signed([0; 32].into()),
            0,
            1000,
            bvec![10, 10]
        ));
        assert_noop!(
            PropertyGovernance::unfreeze_proposal_shares(RuntimeOrigin::signed([1; 32].into()), 0,),
            Error::<Test>::NoFrozenAmount
        );
        assert_ok!(PropertyGovernance::vote_on_proposal(
            RuntimeOrigin::signed([1; 32].into()),
            0,
            crate::Vote::Yes,
            45
        ));
        assert_eq!(
            AssetsFreezer::balance_frozen(
                0,
                &MarketplaceFreezeReason::ProposalVoting,
                &[1; 32].into()
            ),
            45
        );
        assert_ok!(PropertyGovernance::vote_on_proposal(
            RuntimeOrigin::signed([2; 32].into()),
            0,
            crate::Vote::Yes,
            10
        ));
        assert_ok!(PropertyGovernance::vote_on_proposal(
            RuntimeOrigin::signed([1; 32].into()),
            0,
            crate::Vote::No,
            30
        ));
        assert_eq!(
            AssetsFreezer::balance_frozen(
                0,
                &MarketplaceFreezeReason::ProposalVoting,
                &[1; 32].into()
            ),
            30
        );
        assert_eq!(
            UserProposalVote::<Test>::get::<u64, AccountId>(0, [1; 32].into()).unwrap(),
            VoteRecord { vote: crate::Vote::No, asset_id: 0, power: 30 }
        );
        assert_noop!(
            PropertyGovernance::unfreeze_proposal_shares(RuntimeOrigin::signed([1; 32].into()), 0,),
            Error::<Test>::VotingStillOngoing
        );
        let expiry = frame_system::Pallet::<Test>::block_number() + PropertyVotingTime::get();
        run_to_block(expiry);
        assert_noop!(
            PropertyGovernance::unfreeze_proposal_shares(RuntimeOrigin::signed([3; 32].into()), 0,),
            Error::<Test>::NoFrozenAmount
        );
        assert_ok!(PropertyGovernance::unfreeze_proposal_shares(
            RuntimeOrigin::signed([1; 32].into()),
            0,
        ));
        assert_noop!(
            PropertyGovernance::unfreeze_proposal_shares(RuntimeOrigin::signed([1; 32].into()), 0,),
            Error::<Test>::NoFrozenAmount
        );
    });
}

// vote_on_letting_agent_challenge tests

#[test]
fn vote_on_challenge_works() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);
        // Listing setup and role assignments
        assert_ok!(XcavateWhitelist::add_admin(RuntimeOrigin::root(), [20; 32].into(),));
        listing_process();
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
        assert_ok!(Marketplace::buy_property_shares(
            RuntimeOrigin::signed([1; 32].into()),
            0,
            20,
            1984
        ));
        assert_ok!(Marketplace::buy_property_shares(
            RuntimeOrigin::signed([2; 32].into()),
            0,
            30,
            1984
        ));
        assert_ok!(Marketplace::buy_property_shares(
            RuntimeOrigin::signed([2; 32].into()),
            0,
            10,
            1984
        ));
        assert_ok!(Marketplace::buy_property_shares(
            RuntimeOrigin::signed([3; 32].into()),
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
        assert_ok!(Marketplace::claim_property_shares(RuntimeOrigin::signed([3; 32].into()), 0));
        lawyer_process(vec![([1; 32].into(), 20), ([3; 32].into(), 40)]);

        // Letting agent process
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [0; 32].into(),
            pallet_xcavate_whitelist::Role::LettingAgent
        ));
        setting_letting_agent([0; 32].into(), vec![([1; 32].into(), 20), ([3; 32].into(), 40)]);

        // Challenge process
        assert_ok!(PropertyGovernance::challenge_against_letting_agent(
            RuntimeOrigin::signed([1; 32].into()),
            0
        ));
        assert_ok!(PropertyGovernance::vote_on_letting_agent_challenge(
            RuntimeOrigin::signed([2; 32].into()),
            0,
            crate::Vote::Yes,
            30
        ));
        assert_eq!(
            AssetsFreezer::balance_frozen(
                0,
                &MarketplaceFreezeReason::ChallengeVoting,
                &[2; 32].into()
            ),
            30
        );
        assert_eq!(
            UserChallengeVote::<Test>::get::<u64, AccountId>(0, [2; 32].into()).unwrap(),
            VoteRecord { vote: crate::Vote::Yes, asset_id: 0, power: 30 }
        );
        assert_ok!(PropertyGovernance::vote_on_letting_agent_challenge(
            RuntimeOrigin::signed([1; 32].into()),
            0,
            crate::Vote::Yes,
            20
        ));
        assert_ok!(PropertyGovernance::vote_on_letting_agent_challenge(
            RuntimeOrigin::signed([3; 32].into()),
            0,
            crate::Vote::Yes,
            10
        ));
        assert_ok!(PropertyGovernance::vote_on_letting_agent_challenge(
            RuntimeOrigin::signed([2; 32].into()),
            0,
            crate::Vote::No,
            40
        ));
        assert_eq!(
            AssetsFreezer::balance_frozen(
                0,
                &MarketplaceFreezeReason::ChallengeVoting,
                &[2; 32].into()
            ),
            40
        );
        assert_eq!(
            UserChallengeVote::<Test>::get::<u64, AccountId>(0, [2; 32].into()).unwrap(),
            VoteRecord { vote: crate::Vote::No, asset_id: 0, power: 40 }
        );
        assert_eq!(OngoingChallengeVotes::<Test>::get(0).unwrap().yes_voting_power, 30);
        assert_eq!(OngoingChallengeVotes::<Test>::get(0).unwrap().no_voting_power, 40);
    });
}

#[test]
fn challenge_pass() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);
        // Listing setup and role assignments
        assert_ok!(XcavateWhitelist::add_admin(RuntimeOrigin::root(), [20; 32].into(),));
        listing_process();
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [0; 32].into(),
            pallet_xcavate_whitelist::Role::LettingAgent
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [1; 32].into(),
            pallet_xcavate_whitelist::Role::LettingAgent
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
        assert_ok!(PropertyManagement::add_letting_agent(
            RuntimeOrigin::signed([0; 32].into()),
            3,
            bvec![10, 10],
        ));
        assert_ok!(PropertyManagement::add_letting_agent(
            RuntimeOrigin::signed([1; 32].into()),
            3,
            bvec![10, 10],
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
        lawyer_process(vec![([1; 32].into(), 30), ([2; 32].into(), 40)]);

        // Letting agent process
        assert_ok!(PropertyManagement::letting_agent_claim_property(
            RuntimeOrigin::signed([0; 32].into()),
            0
        ));
        assert_ok!(PropertyManagement::vote_on_letting_agent(
            RuntimeOrigin::signed([1; 32].into()),
            0,
            pallet_property_management::Vote::Yes,
            30
        ));
        assert_ok!(PropertyManagement::vote_on_letting_agent(
            RuntimeOrigin::signed([2; 32].into()),
            0,
            pallet_property_management::Vote::Yes,
            40
        ));
        let expiry =
            frame_system::Pallet::<Test>::block_number() + LettingAgentVotingDuration::get();
        frame_system::Pallet::<Test>::set_block_number(expiry);
        assert_ok!(PropertyManagement::finalize_letting_agent(
            RuntimeOrigin::signed([1; 32].into()),
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
        assert_eq!(LettingStorage::<Test>::get(0).unwrap(), [0; 32].into());

        // Challenge process
        // First challenge
        assert_eq!(Balances::total_balance_on_hold(&[1; 32].into()), 1000);
        assert_ok!(PropertyGovernance::challenge_against_letting_agent(
            RuntimeOrigin::signed([1; 32].into()),
            0
        ));
        assert_eq!(Balances::total_balance_on_hold(&[1; 32].into()), 1500);
        assert_eq!(Balances::free_balance(&([1; 32].into())), 14_998_500);
        assert_eq!(Challenges::<Test>::get(0).is_some(), true);
        assert_ok!(PropertyGovernance::vote_on_letting_agent_challenge(
            RuntimeOrigin::signed([1; 32].into()),
            0,
            crate::Vote::No,
            30
        ));
        assert_ok!(PropertyGovernance::vote_on_letting_agent_challenge(
            RuntimeOrigin::signed([1; 32].into()),
            0,
            crate::Vote::Yes,
            30
        ));
        assert_ok!(PropertyGovernance::vote_on_letting_agent_challenge(
            RuntimeOrigin::signed([2; 32].into()),
            0,
            crate::Vote::Yes,
            40
        ));
        assert_ok!(PropertyGovernance::vote_on_letting_agent_challenge(
            RuntimeOrigin::signed([30; 32].into()),
            0,
            crate::Vote::Yes,
            30
        ));
        assert_eq!(AssetLettingChallenge::<Test>::get(0), Some(0));
        let expiry =
            frame_system::Pallet::<Test>::block_number() + LettingAgentVotingDuration::get();
        assert_eq!(ChallengeRoundsExpiring::<Test>::get(expiry).len(), 1);
        run_to_block(expiry);
        assert_eq!(Balances::total_balance_on_hold(&[1; 32].into()), 1000);
        assert_eq!(Balances::free_balance(&([1; 32].into())), 14_999_000);
        assert_eq!(AssetLettingChallenge::<Test>::get(0), None);
        assert_ok!(PropertyGovernance::unfreeze_challenge_shares(
            RuntimeOrigin::signed([1; 32].into()),
            0,
        ));
        assert_ok!(PropertyGovernance::unfreeze_challenge_shares(
            RuntimeOrigin::signed([2; 32].into()),
            0,
        ));
        assert_ok!(PropertyGovernance::unfreeze_challenge_shares(
            RuntimeOrigin::signed([30; 32].into()),
            0,
        ));
        assert_eq!(
            LettingInfo::<Test>::get::<AccountId>([0; 32].into())
                .unwrap()
                .active_strikes
                .get(&0u32)
                .unwrap(),
            &1u8
        );

        // Second challenge
        assert_ok!(PropertyGovernance::challenge_against_letting_agent(
            RuntimeOrigin::signed([1; 32].into()),
            0
        ));
        assert_eq!(LettingStorage::<Test>::get(0).unwrap(), [0; 32].into());
        assert_ok!(PropertyGovernance::vote_on_letting_agent_challenge(
            RuntimeOrigin::signed([1; 32].into()),
            0,
            crate::Vote::Yes,
            30
        ));
        assert_ok!(PropertyGovernance::vote_on_letting_agent_challenge(
            RuntimeOrigin::signed([2; 32].into()),
            0,
            crate::Vote::Yes,
            40
        ));
        assert_ok!(PropertyGovernance::vote_on_letting_agent_challenge(
            RuntimeOrigin::signed([30; 32].into()),
            0,
            crate::Vote::Abstain,
            30
        ));
        assert_eq!(Balances::total_balance_on_hold(&[0; 32].into()), 900);
        assert_eq!(Balances::total_issuance(), 59_604_901);
        let expiry =
            frame_system::Pallet::<Test>::block_number() + LettingAgentVotingDuration::get();
        run_to_block(expiry);
        assert_eq!(
            LettingInfo::<Test>::get::<AccountId>([0; 32].into())
                .unwrap()
                .active_strikes
                .get(&0u32)
                .unwrap(),
            &2u8
        );
        assert_ok!(PropertyGovernance::unfreeze_challenge_shares(
            RuntimeOrigin::signed([1; 32].into()),
            1,
        ));
        assert_ok!(PropertyGovernance::unfreeze_challenge_shares(
            RuntimeOrigin::signed([2; 32].into()),
            1,
        ));
        assert_ok!(PropertyGovernance::unfreeze_challenge_shares(
            RuntimeOrigin::signed([30; 32].into()),
            1,
        ));

        // Third challenge
        assert_ok!(PropertyGovernance::challenge_against_letting_agent(
            RuntimeOrigin::signed([1; 32].into()),
            0
        ));
        assert_eq!(Balances::total_balance_on_hold(&[0; 32].into()), 800);
        assert_eq!(Balances::total_issuance(), 59_604_801);
        assert_ok!(PropertyGovernance::vote_on_letting_agent_challenge(
            RuntimeOrigin::signed([1; 32].into()),
            0,
            crate::Vote::Yes,
            30
        ));
        assert_ok!(PropertyGovernance::vote_on_letting_agent_challenge(
            RuntimeOrigin::signed([2; 32].into()),
            0,
            crate::Vote::Yes,
            40
        ));
        assert_ok!(PropertyGovernance::vote_on_letting_agent_challenge(
            RuntimeOrigin::signed([30; 32].into()),
            0,
            crate::Vote::Yes,
            30
        ));
        assert_eq!(LettingStorage::<Test>::get(0).unwrap(), [0; 32].into());
        assert_eq!(
            LettingInfo::<Test>::get::<AccountId>([0; 32].into()).unwrap().locations.len(),
            1
        );
        assert_eq!(
            LettingInfo::<Test>::get::<AccountId>([0; 32].into())
                .unwrap()
                .active_strikes
                .get(&0u32)
                .unwrap(),
            &2u8
        );
        assert_eq!(
            LettingInfo::<Test>::get::<AccountId>([0; 32].into())
                .unwrap()
                .locations
                .get(&bvec![10, 10])
                .clone()
                .unwrap()
                .assigned_properties,
            1
        );
        run_to_block(211);
        // Letting agent got removed from property after 3 strikes
        assert_eq!(
            LettingInfo::<Test>::get::<AccountId>([0; 32].into())
                .unwrap()
                .active_strikes
                .get(&0u32)
                .is_none(),
            true
        );
        assert_eq!(
            LettingInfo::<Test>::get::<AccountId>([0; 32].into())
                .unwrap()
                .locations
                .get(&bvec![10, 10])
                .clone()
                .unwrap()
                .assigned_properties,
            1
        );
        assert_eq!(LettingStorage::<Test>::get(0).is_none(), true);
        assert_eq!(
            LettingInfo::<Test>::get::<AccountId>([0; 32].into()).unwrap().locations.len(),
            1
        );
        assert_eq!(Challenges::<Test>::get(0).is_none(), true);

        // New letting agent claims property
        assert_ok!(PropertyManagement::letting_agent_claim_property(
            RuntimeOrigin::signed([1; 32].into()),
            0
        ));
        assert_ok!(PropertyManagement::vote_on_letting_agent(
            RuntimeOrigin::signed([1; 32].into()),
            0,
            pallet_property_management::Vote::Yes,
            30
        ));
        assert_ok!(PropertyManagement::vote_on_letting_agent(
            RuntimeOrigin::signed([2; 32].into()),
            0,
            pallet_property_management::Vote::Yes,
            30
        ));
        let expiry =
            frame_system::Pallet::<Test>::block_number() + LettingAgentVotingDuration::get();
        run_to_block(expiry);
        assert_ok!(PropertyManagement::finalize_letting_agent(
            RuntimeOrigin::signed([1; 32].into()),
            0,
        ));
        assert_eq!(LettingStorage::<Test>::get(0).unwrap(), [1; 32].into());
    });
}

#[test]
fn challenge_does_not_pass() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);
        // Listing setup and role assignments
        assert_ok!(XcavateWhitelist::add_admin(RuntimeOrigin::root(), [20; 32].into(),));
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
            [0; 32].into(),
            pallet_xcavate_whitelist::Role::LettingAgent
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [1; 32].into(),
            pallet_xcavate_whitelist::Role::LettingAgent
        ));
        assert_ok!(PropertyManagement::add_letting_agent(
            RuntimeOrigin::signed([0; 32].into()),
            3,
            bvec![10, 10],
        ));
        assert_ok!(PropertyManagement::add_letting_agent(
            RuntimeOrigin::signed([1; 32].into()),
            3,
            bvec![10, 10],
        ));
        assert_ok!(Marketplace::list_property(
            RuntimeOrigin::signed([0; 32].into()),
            3,
            bvec![10, 10],
            4_000,
            250,
            bvec![22, 22],
            false
        ));
        assert_ok!(Marketplace::buy_property_shares(
            RuntimeOrigin::signed([1; 32].into()),
            0,
            75,
            1984
        ));
        assert_ok!(Marketplace::buy_property_shares(
            RuntimeOrigin::signed([2; 32].into()),
            0,
            100,
            1984
        ));
        assert_ok!(Marketplace::buy_property_shares(
            RuntimeOrigin::signed([30; 32].into()),
            0,
            75,
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
        lawyer_process(vec![([1; 32].into(), 75), ([2; 32].into(), 75)]);

        // Letting agent process
        assert_ok!(PropertyManagement::letting_agent_claim_property(
            RuntimeOrigin::signed([0; 32].into()),
            0
        ));
        assert_ok!(PropertyManagement::vote_on_letting_agent(
            RuntimeOrigin::signed([1; 32].into()),
            0,
            pallet_property_management::Vote::Yes,
            75
        ));
        assert_ok!(PropertyManagement::vote_on_letting_agent(
            RuntimeOrigin::signed([2; 32].into()),
            0,
            pallet_property_management::Vote::Yes,
            75
        ));
        let expiry =
            frame_system::Pallet::<Test>::block_number() + LettingAgentVotingDuration::get();
        frame_system::Pallet::<Test>::set_block_number(expiry);
        assert_ok!(PropertyManagement::finalize_letting_agent(
            RuntimeOrigin::signed([1; 32].into()),
            0,
        ));
        assert_eq!(LettingStorage::<Test>::get(0).unwrap(), [0; 32].into());

        // Challenge process
        // First challenge
        assert_ok!(PropertyGovernance::challenge_against_letting_agent(
            RuntimeOrigin::signed([1; 32].into()),
            0
        ));
        assert_eq!(Challenges::<Test>::get(0).is_some(), true);
        assert_ok!(PropertyGovernance::vote_on_letting_agent_challenge(
            RuntimeOrigin::signed([1; 32].into()),
            0,
            crate::Vote::Yes,
            75
        ));
        assert_ok!(PropertyGovernance::vote_on_letting_agent_challenge(
            RuntimeOrigin::signed([2; 32].into()),
            0,
            crate::Vote::Yes,
            100
        ));
        assert_ok!(PropertyGovernance::vote_on_letting_agent_challenge(
            RuntimeOrigin::signed([30; 32].into()),
            0,
            crate::Vote::Yes,
            75
        ));
        let expiry =
            frame_system::Pallet::<Test>::block_number() + LettingAgentVotingDuration::get();
        assert_eq!(ChallengeRoundsExpiring::<Test>::get(expiry).len(), 1);
        run_to_block(expiry);
        assert_ok!(PropertyGovernance::unfreeze_challenge_shares(
            RuntimeOrigin::signed([1; 32].into()),
            0,
        ));

        // Second challenge
        assert_ok!(PropertyGovernance::challenge_against_letting_agent(
            RuntimeOrigin::signed([1; 32].into()),
            0
        ));
        assert_eq!(Balances::total_balance_on_hold(&[1; 32].into()), 1500);
        assert_eq!(Balances::free_balance(&([1; 32].into())), 14_998_500);
        assert_eq!(LettingStorage::<Test>::get(0).unwrap(), [0; 32].into());
        assert_ok!(PropertyGovernance::vote_on_letting_agent_challenge(
            RuntimeOrigin::signed([1; 32].into()),
            0,
            crate::Vote::Yes,
            75
        ));
        assert_eq!(Balances::total_issuance(), 59_604_901);
        let expiry =
            frame_system::Pallet::<Test>::block_number() + LettingAgentVotingDuration::get();
        run_to_block(expiry);
        System::assert_last_event(Event::ChallengeRejected { asset_id: 0 }.into());
        assert_eq!(Challenges::<Test>::get(0).is_none(), true);
        assert_eq!(Balances::total_balance_on_hold(&[1; 32].into()), 1000);
        assert_eq!(Balances::free_balance(&([1; 32].into())), 14_998_500);
        assert_eq!(Balances::total_issuance(), 59_604_401);
    });
}

#[test]
fn challenge_pass_only_one_agent() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);
        // Listing setup and role assignments
        assert_ok!(XcavateWhitelist::add_admin(RuntimeOrigin::root(), [20; 32].into(),));
        listing_process();
        assert_ok!(Regions::create_new_location(
            RuntimeOrigin::signed([6; 32].into()),
            3,
            bvec![9, 10]
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [0; 32].into(),
            pallet_xcavate_whitelist::Role::LettingAgent
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [1; 32].into(),
            pallet_xcavate_whitelist::Role::LettingAgent
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
        assert_ok!(PropertyManagement::add_letting_agent(
            RuntimeOrigin::signed([0; 32].into()),
            3,
            bvec![10, 10],
        ));
        assert_ok!(PropertyManagement::add_letting_agent(
            RuntimeOrigin::signed([1; 32].into()),
            3,
            bvec![9, 10],
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
        lawyer_process(vec![([1; 32].into(), 30), ([2; 32].into(), 40)]);

        // Letting agent process
        assert_ok!(PropertyManagement::letting_agent_claim_property(
            RuntimeOrigin::signed([0; 32].into()),
            0
        ));
        assert_ok!(PropertyManagement::vote_on_letting_agent(
            RuntimeOrigin::signed([1; 32].into()),
            0,
            pallet_property_management::Vote::Yes,
            30
        ));
        assert_ok!(PropertyManagement::vote_on_letting_agent(
            RuntimeOrigin::signed([2; 32].into()),
            0,
            pallet_property_management::Vote::Yes,
            40
        ));
        let expiry =
            frame_system::Pallet::<Test>::block_number() + LettingAgentVotingDuration::get();
        frame_system::Pallet::<Test>::set_block_number(expiry);
        assert_ok!(PropertyManagement::finalize_letting_agent(
            RuntimeOrigin::signed([1; 32].into()),
            0,
        ));
        assert_eq!(LettingStorage::<Test>::get(0).unwrap(), [0; 32].into());

        // Challenge process
        // First challenge
        assert_ok!(PropertyGovernance::challenge_against_letting_agent(
            RuntimeOrigin::signed([1; 32].into()),
            0
        ));
        assert_eq!(Challenges::<Test>::get(0).is_some(), true);
        assert_ok!(PropertyGovernance::vote_on_letting_agent_challenge(
            RuntimeOrigin::signed([1; 32].into()),
            0,
            crate::Vote::Yes,
            30
        ));
        assert_ok!(PropertyGovernance::vote_on_letting_agent_challenge(
            RuntimeOrigin::signed([2; 32].into()),
            0,
            crate::Vote::Yes,
            40
        ));
        let expiry =
            frame_system::Pallet::<Test>::block_number() + LettingAgentVotingDuration::get();
        assert_eq!(ChallengeRoundsExpiring::<Test>::get(expiry).len(), 1);
        run_to_block(expiry);
        assert_ok!(PropertyGovernance::unfreeze_challenge_shares(
            RuntimeOrigin::signed([1; 32].into()),
            0,
        ));
        assert_ok!(PropertyGovernance::unfreeze_challenge_shares(
            RuntimeOrigin::signed([2; 32].into()),
            0,
        ));

        // Second process
        assert_ok!(PropertyGovernance::challenge_against_letting_agent(
            RuntimeOrigin::signed([1; 32].into()),
            0
        ));
        assert_eq!(LettingStorage::<Test>::get(0).unwrap(), [0; 32].into());
        assert_ok!(PropertyGovernance::vote_on_letting_agent_challenge(
            RuntimeOrigin::signed([1; 32].into()),
            0,
            crate::Vote::Yes,
            30
        ));
        assert_ok!(PropertyGovernance::vote_on_letting_agent_challenge(
            RuntimeOrigin::signed([2; 32].into()),
            0,
            crate::Vote::Abstain,
            40
        ));
        let expiry =
            frame_system::Pallet::<Test>::block_number() + LettingAgentVotingDuration::get();
        run_to_block(expiry);
        assert_ok!(PropertyGovernance::unfreeze_challenge_shares(
            RuntimeOrigin::signed([1; 32].into()),
            1,
        ));
        assert_ok!(PropertyGovernance::unfreeze_challenge_shares(
            RuntimeOrigin::signed([2; 32].into()),
            1,
        ));

        // Third process
        assert_ok!(PropertyGovernance::challenge_against_letting_agent(
            RuntimeOrigin::signed([1; 32].into()),
            0
        ));
        assert_ok!(PropertyGovernance::vote_on_letting_agent_challenge(
            RuntimeOrigin::signed([1; 32].into()),
            0,
            crate::Vote::Yes,
            30
        ));
        assert_ok!(PropertyGovernance::vote_on_letting_agent_challenge(
            RuntimeOrigin::signed([2; 32].into()),
            0,
            crate::Vote::Yes,
            40
        ));
        assert_eq!(LettingStorage::<Test>::get(0).unwrap(), [0; 32].into());
        run_to_block(211);
        // Letting agent got removed after 3 strikes
        assert_eq!(LettingStorage::<Test>::get(0).is_none(), true);
        assert_eq!(Challenges::<Test>::get(0).is_none(), true);
    });
}

#[test]
fn challenge_not_pass() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);
        // Listing setup and role assignments
        assert_ok!(XcavateWhitelist::add_admin(RuntimeOrigin::root(), [20; 32].into(),));
        listing_process();
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
        lawyer_process(vec![([1; 32].into(), 40), ([2; 32].into(), 30)]);

        // Letting agent process
        assert_noop!(
            PropertyGovernance::challenge_against_letting_agent(
                RuntimeOrigin::signed([1; 32].into()),
                0
            ),
            Error::<Test>::NoLettingAgentFound
        );
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [0; 32].into(),
            pallet_xcavate_whitelist::Role::LettingAgent
        ));
        setting_letting_agent([0; 32].into(), vec![([1; 32].into(), 40), ([2; 32].into(), 30)]);

        // Challenge process
        assert_ok!(PropertyGovernance::challenge_against_letting_agent(
            RuntimeOrigin::signed([1; 32].into()),
            0
        ));
        assert_ok!(PropertyGovernance::vote_on_letting_agent_challenge(
            RuntimeOrigin::signed([1; 32].into()),
            0,
            crate::Vote::Abstain,
            40
        ));
        assert_ok!(PropertyGovernance::vote_on_letting_agent_challenge(
            RuntimeOrigin::signed([2; 32].into()),
            0,
            crate::Vote::No,
            30
        ));
        assert_eq!(Challenges::<Test>::get(0).is_some(), true);
        let expiry =
            frame_system::Pallet::<Test>::block_number() + LettingAgentVotingDuration::get();
        run_to_block(expiry);
        System::assert_last_event(Event::ChallengeRejected { asset_id: 0 }.into());
        assert_eq!(Challenges::<Test>::get(0).is_none(), true);
    });
}

#[test]
fn challenge_not_pass2() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);
        // Listing setup and role assignments
        assert_ok!(XcavateWhitelist::add_admin(RuntimeOrigin::root(), [20; 32].into(),));
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
            pallet_xcavate_whitelist::Role::RealEstateDeveloper
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
            5_000,
            200,
            bvec![22, 22],
            false
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [1; 32].into(),
            pallet_xcavate_whitelist::Role::RealEstateInvestor
        ));
        assert_ok!(Marketplace::buy_property_shares(
            RuntimeOrigin::signed([1; 32].into()),
            0,
            80,
            1984
        ));
        assert_ok!(Marketplace::buy_property_shares(
            RuntimeOrigin::signed([2; 32].into()),
            0,
            60,
            1984
        ));
        assert_ok!(Marketplace::buy_property_shares(
            RuntimeOrigin::signed([30; 32].into()),
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
        assert_ok!(Marketplace::claim_property_shares(RuntimeOrigin::signed([2; 32].into()), 0));
        assert_ok!(Marketplace::claim_property_shares(RuntimeOrigin::signed([30; 32].into()), 0));
        lawyer_process(vec![([1; 32].into(), 80), ([2; 32].into(), 60)]);

        // Letting agent process
        assert_noop!(
            PropertyGovernance::challenge_against_letting_agent(
                RuntimeOrigin::signed([1; 32].into()),
                0
            ),
            Error::<Test>::NoLettingAgentFound
        );
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [0; 32].into(),
            pallet_xcavate_whitelist::Role::LettingAgent
        ));
        setting_letting_agent([0; 32].into(), vec![([1; 32].into(), 80), ([2; 32].into(), 60)]);

        // Challenge process
        assert_ok!(PropertyGovernance::challenge_against_letting_agent(
            RuntimeOrigin::signed([1; 32].into()),
            0
        ));
        assert_ok!(PropertyGovernance::vote_on_letting_agent_challenge(
            RuntimeOrigin::signed([1; 32].into()),
            0,
            crate::Vote::Yes,
            80
        ));
        assert_ok!(PropertyGovernance::vote_on_letting_agent_challenge(
            RuntimeOrigin::signed([2; 32].into()),
            0,
            crate::Vote::Yes,
            20
        ));
        assert_eq!(Challenges::<Test>::get(0).is_some(), true);
        let expiry =
            frame_system::Pallet::<Test>::block_number() + LettingAgentVotingDuration::get();
        run_to_block(expiry);
        System::assert_last_event(Event::ChallengeRejected { asset_id: 0 }.into());
        assert_eq!(Challenges::<Test>::get(0).is_none(), true);
    });
}

#[test]
fn vote_on_challenge_fails() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);
        // Listing setup and role assignments
        assert_ok!(XcavateWhitelist::add_admin(RuntimeOrigin::root(), [20; 32].into(),));
        listing_process();
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
        lawyer_process(vec![([1; 32].into(), 40), ([30; 32].into(), 30)]);

        // Letting agent process
        assert_noop!(
            PropertyGovernance::vote_on_letting_agent_challenge(
                RuntimeOrigin::signed([1; 32].into()),
                0,
                crate::Vote::Yes,
                40
            ),
            Error::<Test>::NotOngoing
        );
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [0; 32].into(),
            pallet_xcavate_whitelist::Role::LettingAgent
        ));
        setting_letting_agent([0; 32].into(), vec![([1; 32].into(), 40), ([30; 32].into(), 30)]);

        // Challenge process
        assert_ok!(PropertyGovernance::challenge_against_letting_agent(
            RuntimeOrigin::signed([1; 32].into()),
            0
        ));
        assert_noop!(
            PropertyGovernance::vote_on_letting_agent_challenge(
                RuntimeOrigin::signed([1; 32].into()),
                0,
                crate::Vote::Yes,
                0
            ),
            Error::<Test>::ZeroVoteAmount
        );
        assert_ok!(PropertyGovernance::vote_on_letting_agent_challenge(
            RuntimeOrigin::signed([1; 32].into()),
            0,
            crate::Vote::Yes,
            40
        ));
        assert_ok!(PropertyGovernance::vote_on_letting_agent_challenge(
            RuntimeOrigin::signed([30; 32].into()),
            0,
            crate::Vote::Yes,
            30
        ));
        assert_noop!(
            PropertyGovernance::vote_on_letting_agent_challenge(
                RuntimeOrigin::signed([2; 32].into()),
                0,
                crate::Vote::Yes,
                10
            ),
            Error::<Test>::NoPermission
        );
    });
}

// unfreeze_challenge_shares tests

#[test]
fn unfreeze_challenge_shares_works() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);
        // Listing setup and role assignments
        assert_ok!(XcavateWhitelist::add_admin(RuntimeOrigin::root(), [20; 32].into(),));
        listing_process();
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
        assert_ok!(Marketplace::buy_property_shares(
            RuntimeOrigin::signed([1; 32].into()),
            0,
            20,
            1984
        ));
        assert_ok!(Marketplace::buy_property_shares(
            RuntimeOrigin::signed([2; 32].into()),
            0,
            30,
            1984
        ));
        assert_ok!(Marketplace::buy_property_shares(
            RuntimeOrigin::signed([2; 32].into()),
            0,
            10,
            1984
        ));
        assert_ok!(Marketplace::buy_property_shares(
            RuntimeOrigin::signed([3; 32].into()),
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
        assert_ok!(Marketplace::claim_property_shares(RuntimeOrigin::signed([3; 32].into()), 0));
        lawyer_process(vec![([1; 32].into(), 20), ([3; 32].into(), 40)]);

        // Letting agent process
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [0; 32].into(),
            pallet_xcavate_whitelist::Role::LettingAgent
        ));
        setting_letting_agent([0; 32].into(), vec![([1; 32].into(), 20), ([3; 32].into(), 40)]);

        // Challenge process
        assert_ok!(PropertyGovernance::challenge_against_letting_agent(
            RuntimeOrigin::signed([1; 32].into()),
            0
        ));
        assert_ok!(PropertyGovernance::vote_on_letting_agent_challenge(
            RuntimeOrigin::signed([2; 32].into()),
            0,
            crate::Vote::Yes,
            30
        ));
        assert_ok!(PropertyGovernance::vote_on_letting_agent_challenge(
            RuntimeOrigin::signed([1; 32].into()),
            0,
            crate::Vote::Yes,
            20
        ));
        assert_ok!(PropertyGovernance::vote_on_letting_agent_challenge(
            RuntimeOrigin::signed([3; 32].into()),
            0,
            crate::Vote::Yes,
            10
        ));
        assert_eq!(
            AssetsFreezer::balance_frozen(
                0,
                &MarketplaceFreezeReason::ChallengeVoting,
                &[2; 32].into()
            ),
            30
        );
        assert_eq!(
            UserChallengeVote::<Test>::get::<u64, AccountId>(0, [2; 32].into()).unwrap(),
            VoteRecord { vote: crate::Vote::Yes, asset_id: 0, power: 30 }
        );
        let expiry = frame_system::Pallet::<Test>::block_number() + PropertyVotingTime::get();
        run_to_block(expiry);
        assert_ok!(PropertyGovernance::unfreeze_challenge_shares(
            RuntimeOrigin::signed([2; 32].into()),
            0,
        ));
        assert!(UserChallengeVote::<Test>::get::<u64, AccountId>(0, [2; 32].into()).is_none());
        assert_eq!(
            AssetsFreezer::balance_frozen(
                0,
                &MarketplaceFreezeReason::ChallengeVoting,
                &[2; 32].into()
            ),
            0
        );
    });
}

#[test]
fn unfreeze_challenge_shares_fails() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);
        // Listing setup and role assignments
        assert_ok!(XcavateWhitelist::add_admin(RuntimeOrigin::root(), [20; 32].into(),));
        listing_process();
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
        assert_ok!(Marketplace::buy_property_shares(
            RuntimeOrigin::signed([1; 32].into()),
            0,
            20,
            1984
        ));
        assert_ok!(Marketplace::buy_property_shares(
            RuntimeOrigin::signed([2; 32].into()),
            0,
            30,
            1984
        ));
        assert_ok!(Marketplace::buy_property_shares(
            RuntimeOrigin::signed([2; 32].into()),
            0,
            10,
            1984
        ));
        assert_ok!(Marketplace::buy_property_shares(
            RuntimeOrigin::signed([3; 32].into()),
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
        assert_ok!(Marketplace::claim_property_shares(RuntimeOrigin::signed([3; 32].into()), 0));
        lawyer_process(vec![([1; 32].into(), 20), ([3; 32].into(), 40)]);

        // Letting agent process
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [0; 32].into(),
            pallet_xcavate_whitelist::Role::LettingAgent
        ));
        setting_letting_agent([0; 32].into(), vec![([1; 32].into(), 20), ([3; 32].into(), 40)]);

        // Challenge process
        assert_noop!(
            PropertyGovernance::unfreeze_challenge_shares(RuntimeOrigin::signed([2; 32].into()), 0,),
            Error::<Test>::NoFrozenAmount
        );
        assert_ok!(PropertyGovernance::challenge_against_letting_agent(
            RuntimeOrigin::signed([1; 32].into()),
            0
        ));
        assert_ok!(PropertyGovernance::vote_on_letting_agent_challenge(
            RuntimeOrigin::signed([2; 32].into()),
            0,
            crate::Vote::Yes,
            30
        ));
        assert_ok!(PropertyGovernance::vote_on_letting_agent_challenge(
            RuntimeOrigin::signed([1; 32].into()),
            0,
            crate::Vote::Yes,
            20
        ));
        assert_eq!(
            AssetsFreezer::balance_frozen(
                0,
                &MarketplaceFreezeReason::ChallengeVoting,
                &[2; 32].into()
            ),
            30
        );
        assert_eq!(
            UserChallengeVote::<Test>::get::<u64, AccountId>(0, [2; 32].into()).unwrap(),
            VoteRecord { vote: crate::Vote::Yes, asset_id: 0, power: 30 }
        );
        assert_noop!(
            PropertyGovernance::unfreeze_challenge_shares(RuntimeOrigin::signed([2; 32].into()), 0,),
            Error::<Test>::VotingStillOngoing
        );
        let expiry = frame_system::Pallet::<Test>::block_number() + PropertyVotingTime::get();
        run_to_block(expiry);
        assert_noop!(
            PropertyGovernance::unfreeze_challenge_shares(RuntimeOrigin::signed([3; 32].into()), 0,),
            Error::<Test>::NoFrozenAmount
        );
        assert_ok!(PropertyGovernance::unfreeze_challenge_shares(
            RuntimeOrigin::signed([2; 32].into()),
            0,
        ));
        assert_noop!(
            PropertyGovernance::unfreeze_challenge_shares(RuntimeOrigin::signed([2; 32].into()), 0,),
            Error::<Test>::NoFrozenAmount
        );
    });
}

// Test with different proposals.
#[test]
fn different_proposals() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);
        // Listing setup and role assignments
        assert_ok!(XcavateWhitelist::add_admin(RuntimeOrigin::root(), [20; 32].into(),));
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
        // Set up property
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
            60,
            1984
        ));
        assert_ok!(Marketplace::buy_property_shares(
            RuntimeOrigin::signed([2; 32].into()),
            0,
            60,
            1984
        ));
        assert_ok!(Marketplace::buy_property_shares(
            RuntimeOrigin::signed([3; 32].into()),
            0,
            80,
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
        lawyer_process(vec![([1; 32].into(), 60), ([2; 32].into(), 60)]);
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [4; 32].into(),
            pallet_xcavate_whitelist::Role::LettingAgent
        ));
        setting_letting_agent([4; 32].into(), vec![([1; 32].into(), 60), ([2; 32].into(), 60)]);
        assert_eq!(LettingStorage::<Test>::get(0).unwrap(), [4; 32].into());
        assert_ok!(PropertyManagement::distribute_income(
            RuntimeOrigin::signed([4; 32].into()),
            0,
            3000,
            1984,
        ));
        // First proposal
        assert_ok!(PropertyGovernance::propose(
            RuntimeOrigin::signed([4; 32].into()),
            0,
            1000,
            bvec![10, 10]
        ));
        assert_ok!(PropertyGovernance::vote_on_proposal(
            RuntimeOrigin::signed([1; 32].into()),
            0,
            crate::Vote::Yes,
            60
        ));
        assert_eq!(Proposals::<Test>::get(0).is_some(), true);
        assert_eq!(ForeignAssets::balance(1984, &[4; 32].into()), 2000);
        assert_eq!(ForeignAssets::balance(1984, &PropertyGovernance::property_account_id(0)), 3000);
        assert_eq!(
            UserProposalVote::<Test>::get::<u64, AccountId>(0, [1; 32].into()).is_some(),
            true
        );
        let expiry =
            frame_system::Pallet::<Test>::block_number() + LettingAgentVotingDuration::get();
        run_to_block(expiry);
        assert_eq!(
            UserProposalVote::<Test>::get::<u64, AccountId>(0, [1; 32].into()).is_some(),
            true
        );
        assert_eq!(ForeignAssets::balance(1984, &[4; 32].into()), 2000);
        assert_eq!(ForeignAssets::balance(1984, &PropertyGovernance::property_account_id(0)), 3000);
        assert_eq!(Proposals::<Test>::get(0).is_none(), true);
        assert_ok!(PropertyGovernance::unfreeze_proposal_shares(
            RuntimeOrigin::signed([1; 32].into()),
            0,
        ));
        // Second proposal
        assert_ok!(PropertyGovernance::propose(
            RuntimeOrigin::signed([4; 32].into()),
            0,
            3000,
            bvec![10, 10]
        ));
        assert_eq!(Proposals::<Test>::get(1).is_some(), true);
        assert_ok!(PropertyGovernance::vote_on_proposal(
            RuntimeOrigin::signed([1; 32].into()),
            0,
            crate::Vote::Yes,
            60
        ));
        assert_ok!(PropertyGovernance::vote_on_proposal(
            RuntimeOrigin::signed([2; 32].into()),
            0,
            crate::Vote::Yes,
            60
        ));
        let expiry =
            frame_system::Pallet::<Test>::block_number() + LettingAgentVotingDuration::get();
        run_to_block(expiry);
        assert_eq!(ForeignAssets::balance(1984, &[4; 32].into()), 2000);
        assert_eq!(ForeignAssets::balance(1984, &PropertyGovernance::property_account_id(0)), 3000);
        assert_ok!(PropertyGovernance::unfreeze_proposal_shares(
            RuntimeOrigin::signed([1; 32].into()),
            1,
        ));
        assert_ok!(PropertyGovernance::unfreeze_proposal_shares(
            RuntimeOrigin::signed([2; 32].into()),
            1,
        ));
        // Third proposal
        assert_ok!(PropertyGovernance::propose(
            RuntimeOrigin::signed([4; 32].into()),
            0,
            3000,
            bvec![10, 10]
        ));
        assert_eq!(Proposals::<Test>::get(2).is_some(), true);
        assert_ok!(PropertyGovernance::vote_on_proposal(
            RuntimeOrigin::signed([1; 32].into()),
            0,
            crate::Vote::Abstain,
            60
        ));
        assert_ok!(PropertyGovernance::vote_on_proposal(
            RuntimeOrigin::signed([2; 32].into()),
            0,
            crate::Vote::No,
            60
        ));
        assert_ok!(PropertyGovernance::vote_on_proposal(
            RuntimeOrigin::signed([3; 32].into()),
            0,
            crate::Vote::Yes,
            80
        ));
        let expiry =
            frame_system::Pallet::<Test>::block_number() + LettingAgentVotingDuration::get();
        run_to_block(expiry);
        assert_eq!(ForeignAssets::balance(1984, &[4; 32].into()), 2000);
        assert_eq!(ForeignAssets::balance(1984, &PropertyGovernance::property_account_id(0)), 3000);
        assert_ok!(PropertyManagement::distribute_income(
            RuntimeOrigin::signed([4; 32].into()),
            0,
            1700,
            1984,
        ));
        assert_ok!(PropertyManagement::distribute_income(
            RuntimeOrigin::signed([4; 32].into()),
            0,
            300,
            1337,
        ));
        // Fourth proposal
        assert_ok!(PropertyGovernance::propose(
            RuntimeOrigin::signed([4; 32].into()),
            0,
            1500,
            bvec![10, 10]
        ));
        assert_eq!(Proposals::<Test>::get(3).is_some(), true);
        assert_ok!(PropertyGovernance::unfreeze_proposal_shares(
            RuntimeOrigin::signed([1; 32].into()),
            2,
        ));
        assert_ok!(PropertyGovernance::unfreeze_proposal_shares(
            RuntimeOrigin::signed([2; 32].into()),
            2,
        ));
        assert_ok!(PropertyGovernance::unfreeze_proposal_shares(
            RuntimeOrigin::signed([3; 32].into()),
            2,
        ));
        assert_ok!(PropertyGovernance::vote_on_proposal(
            RuntimeOrigin::signed([1; 32].into()),
            0,
            crate::Vote::Yes,
            60
        ));
        assert_ok!(PropertyGovernance::vote_on_proposal(
            RuntimeOrigin::signed([2; 32].into()),
            0,
            crate::Vote::Yes,
            60
        ));
        assert_ok!(PropertyGovernance::vote_on_proposal(
            RuntimeOrigin::signed([3; 32].into()),
            0,
            crate::Vote::No,
            80
        ));
        let expiry =
            frame_system::Pallet::<Test>::block_number() + LettingAgentVotingDuration::get();
        run_to_block(expiry);
        assert_eq!(ForeignAssets::balance(1984, &[4; 32].into()), 300);
        assert_eq!(ForeignAssets::balance(1984, &PropertyGovernance::property_account_id(0)), 4700);
        assert_eq!(ForeignAssets::balance(1337, &[4; 32].into()), 4700);
        assert_eq!(ForeignAssets::balance(1337, &PropertyGovernance::property_account_id(0)), 300);
    });
}
