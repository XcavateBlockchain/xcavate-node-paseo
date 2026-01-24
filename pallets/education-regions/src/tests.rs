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
use crate::{
    HoldReason, LastRegionProposalBlock, OngoingRegionOwnerProposalVotes,
    OngoingRegionProposalVotes, ProposalCounter, RegionAuctions, RegionDetails,
    RegionOwnerProposalId, RegionOwnerProposals, RegionProposalId, RegionProposals,
    RegionReplacementAuctions, UserRegionOwnerVote, UserRegionVote, VoteStats,
};
use frame_support::{
    assert_noop, assert_ok,
    traits::{fungible::Inspect, fungible::InspectHold, OnFinalize, OnInitialize},
};
use sp_runtime::{traits::BadOrigin, TokenError};

fn run_to_block(n: u64) {
    while System::block_number() < n {
        if System::block_number() > 0 {
            Regions::on_finalize(System::block_number());
            System::on_finalize(System::block_number());
        }
        System::reset_events();
        System::set_block_number(System::block_number() + 1);
        System::on_initialize(System::block_number());
        Regions::on_initialize(System::block_number());
    }
}

fn new_region_helper() {
    assert_ok!(Regions::propose_new_region(
        RuntimeOrigin::signed([8; 32].into()),
        crate::RegionIdentifier::Japan
    ));
    assert_ok!(Regions::vote_on_region_proposal(
        RuntimeOrigin::signed([8; 32].into()),
        3,
        crate::Vote::Yes,
        100_000
    ));
    run_to_block(31);
    assert_ok!(Regions::bid_on_region(RuntimeOrigin::signed([8; 32].into()), 3, 100_000));
    run_to_block(61);
    assert_ok!(Regions::create_new_region(RuntimeOrigin::signed([8; 32].into()), 3,));
    assert_ok!(Regions::unlock_region_voting_token(RuntimeOrigin::signed([8; 32].into()), 0,));
}

// propose_new_region tests

#[test]
fn propose_new_region_works() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);
        assert_ok!(XcavateWhitelist::add_admin(RuntimeOrigin::root(), [20; 32].into(),));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [0; 32].into(),
            pallet_xcavate_whitelist::Role::RegionalOperator
        ));
        assert_ok!(Regions::propose_new_region(
            RuntimeOrigin::signed([0; 32].into()),
            crate::RegionIdentifier::Japan
        ));
        assert_eq!(RegionProposals::<Test>::get(0).unwrap().proposal_expiry, 31);
        assert_eq!(
            OngoingRegionProposalVotes::<Test>::get(0).unwrap(),
            VoteStats { yes_voting_power: 0, no_voting_power: 0, abstain_voting_power: 0 }
        );

        assert_eq!(RegionProposalId::<Test>::get(3).is_some(), true);
        assert_eq!(Balances::free_balance(&([0; 32].into())), 195_000);
        assert_eq!(
            Balances::balance_on_hold(&HoldReason::RegionProposalReserve.into(), &([0; 32].into())),
            5_000
        );
        assert_eq!(ProposalCounter::<Test>::get(), 1);
    })
}

#[test]
fn propose_new_region_works_after_rejected() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);
        assert_ok!(XcavateWhitelist::add_admin(RuntimeOrigin::root(), [20; 32].into(),));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [0; 32].into(),
            pallet_xcavate_whitelist::Role::RegionalOperator
        ));
        assert_ok!(Regions::propose_new_region(
            RuntimeOrigin::signed([0; 32].into()),
            crate::RegionIdentifier::England
        ));
        assert_ok!(Regions::vote_on_region_proposal(
            RuntimeOrigin::signed([0; 32].into()),
            1,
            crate::Vote::No,
            10_000
        ));
        assert_eq!(RegionProposalId::<Test>::get(1).is_some(), true);
        run_to_block(31);
        assert_ok!(Regions::bid_on_region(RuntimeOrigin::signed([0; 32].into()), 1, 100_000));
        assert_eq!(RegionProposalId::<Test>::get(1).is_some(), false);
        // Use same region identifier after rejection
        assert_ok!(Regions::propose_new_region(
            RuntimeOrigin::signed([0; 32].into()),
            crate::RegionIdentifier::England
        ));
    })
}

#[test]
fn propose_new_region_slash_proposer() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);
        assert_ok!(XcavateWhitelist::add_admin(RuntimeOrigin::root(), [20; 32].into(),));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [0; 32].into(),
            pallet_xcavate_whitelist::Role::RegionalOperator
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [1; 32].into(),
            pallet_xcavate_whitelist::Role::RegionalOperator
        ));
        assert_ok!(Regions::propose_new_region(
            RuntimeOrigin::signed([0; 32].into()),
            crate::RegionIdentifier::Japan
        ));
        assert_ok!(Regions::vote_on_region_proposal(
            RuntimeOrigin::signed([0; 32].into()),
            3,
            crate::Vote::No,
            10_000
        ));
        run_to_block(31);
        assert_ok!(Balances::force_set_balance(
            RuntimeOrigin::root(),
            sp_runtime::MultiAddress::Id(Regions::treasury_account_id()),
            5_000
        ));
        assert_eq!(Balances::total_issuance(), 1_060_000);
        assert_ok!(Regions::bid_on_region(RuntimeOrigin::signed([1; 32].into()), 3, 10_000));
        // Check if proposer got slashed correctly.
        assert_eq!(Balances::free_balance(&([0; 32].into())), 185_000);
        assert_eq!(
            Balances::balance_on_hold(&HoldReason::RegionProposalReserve.into(), &([0; 32].into())),
            0
        );
        assert_eq!(Balances::total_issuance(), 1_055_000);
    })
}

#[test]
fn propose_new_region_no_treasury_rewards() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);
        assert_ok!(XcavateWhitelist::add_admin(RuntimeOrigin::root(), [20; 32].into(),));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [0; 32].into(),
            pallet_xcavate_whitelist::Role::RegionalOperator
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [1; 32].into(),
            pallet_xcavate_whitelist::Role::RegionalOperator
        ));
        assert_ok!(Regions::propose_new_region(
            RuntimeOrigin::signed([0; 32].into()),
            crate::RegionIdentifier::Japan
        ));
        assert_ok!(Regions::vote_on_region_proposal(
            RuntimeOrigin::signed([0; 32].into()),
            3,
            crate::Vote::Yes,
            100_000
        ));
        run_to_block(31);
        assert_ok!(Regions::bid_on_region(RuntimeOrigin::signed([1; 32].into()), 3, 10_000));
        // No treasury rewards because the treasury account does not have enough balance to pay the proposer.
        assert_eq!(Balances::free_balance(&([0; 32].into())), 100_000);
        assert_eq!(
            Balances::balance_on_hold(&HoldReason::RegionProposalReserve.into(), &([0; 32].into())),
            0
        );
    })
}

#[test]
fn propose_new_region_fails() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);
        assert_ok!(XcavateWhitelist::add_admin(RuntimeOrigin::root(), [20; 32].into(),));
        assert_noop!(
            Regions::propose_new_region(
                RuntimeOrigin::signed([0; 32].into()),
                crate::RegionIdentifier::Japan
            ),
            BadOrigin
        );
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [0; 32].into(),
            pallet_xcavate_whitelist::Role::RegionalOperator
        ));
        assert_eq!(LastRegionProposalBlock::<Test>::get(), None);
        assert_ok!(Regions::propose_new_region(
            RuntimeOrigin::signed([0; 32].into()),
            crate::RegionIdentifier::Japan
        ));
        assert_eq!(LastRegionProposalBlock::<Test>::get(), Some(1));
        run_to_block(10);
        assert_noop!(
            Regions::propose_new_region(
                RuntimeOrigin::signed([0; 32].into()),
                crate::RegionIdentifier::Japan
            ),
            Error::<Test>::RegionProposalAlreadyExists
        );
        assert_ok!(Regions::vote_on_region_proposal(
            RuntimeOrigin::signed([8; 32].into()),
            3,
            crate::Vote::Yes,
            100_000
        ));
        run_to_block(31);
        assert_ok!(Regions::bid_on_region(RuntimeOrigin::signed([0; 32].into()), 3, 120_000));
        assert_noop!(
            Regions::propose_new_region(
                RuntimeOrigin::signed([0; 32].into()),
                crate::RegionIdentifier::Japan
            ),
            Error::<Test>::RegionProposalAlreadyExists
        );
        run_to_block(61);
        assert_ok!(Regions::create_new_region(RuntimeOrigin::signed([0; 32].into()), 3,));
        assert_noop!(
            Regions::propose_new_region(
                RuntimeOrigin::signed([0; 32].into()),
                crate::RegionIdentifier::Japan
            ),
            Error::<Test>::RegionAlreadyCreated
        );
    })
}

// vote_on_region_proposal tests

#[test]
fn vote_on_region_proposal_works() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);
        assert_ok!(XcavateWhitelist::add_admin(RuntimeOrigin::root(), [20; 32].into(),));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [0; 32].into(),
            pallet_xcavate_whitelist::Role::RegionalOperator
        ));
        // First proposal
        assert_ok!(Regions::propose_new_region(
            RuntimeOrigin::signed([0; 32].into()),
            crate::RegionIdentifier::Japan
        ));
        assert_eq!(
            Balances::balance_on_hold(&HoldReason::RegionVotingReserve.into(), &([2; 32].into())),
            0
        );
        assert_ok!(Regions::vote_on_region_proposal(
            RuntimeOrigin::signed([2; 32].into()),
            3,
            crate::Vote::Yes,
            250_000
        ));
        assert_eq!(
            OngoingRegionProposalVotes::<Test>::get(0).unwrap(),
            VoteStats { yes_voting_power: 250_000, no_voting_power: 0, abstain_voting_power: 0 }
        );
        assert_eq!(
            UserRegionVote::<Test>::get::<u64, AccountId>(0, [2; 32].into()).unwrap(),
            crate::VoteRecord { vote: crate::Vote::Yes, region_id: 3, power: 250_000 }
        );
        assert_eq!(Balances::free_balance(&([2; 32].into())), 50_000);
        assert_eq!(
            Balances::balance_on_hold(&HoldReason::RegionVotingReserve.into(), &([2; 32].into())),
            250_000
        );
        assert_ok!(Regions::vote_on_region_proposal(
            RuntimeOrigin::signed([0; 32].into()),
            3,
            crate::Vote::Yes,
            150_000
        ));
        assert_ok!(Regions::vote_on_region_proposal(
            RuntimeOrigin::signed([1; 32].into()),
            3,
            crate::Vote::No,
            120_000
        ));
        assert_eq!(Balances::free_balance(&([1; 32].into())), 30_000);
        assert_eq!(
            Balances::balance_on_hold(&HoldReason::RegionVotingReserve.into(), &([1; 32].into())),
            120_000
        );
        assert_ok!(Regions::vote_on_region_proposal(
            RuntimeOrigin::signed([2; 32].into()),
            3,
            crate::Vote::No,
            200_000
        ));
        assert_eq!(Balances::free_balance(&([2; 32].into())), 100_000);
        assert_eq!(
            Balances::balance_on_hold(&HoldReason::RegionVotingReserve.into(), &([2; 32].into())),
            200_000
        );
        assert_eq!(
            OngoingRegionProposalVotes::<Test>::get(0).unwrap(),
            VoteStats {
                yes_voting_power: 150_000,
                no_voting_power: 320_000,
                abstain_voting_power: 0
            }
        );
        assert_ok!(Balances::force_set_balance(
            RuntimeOrigin::root(),
            sp_runtime::MultiAddress::Id([1; 32].into()),
            600_000
        ));
        assert_ok!(Regions::vote_on_region_proposal(
            RuntimeOrigin::signed([1; 32].into()),
            3,
            crate::Vote::Yes,
            510_000
        ));
        assert_eq!(Balances::free_balance(&([1; 32].into())), 210_000);
        assert_eq!(
            Balances::balance_on_hold(&HoldReason::RegionVotingReserve.into(), &([1; 32].into())),
            510_000
        );
        assert_eq!(
            OngoingRegionProposalVotes::<Test>::get(0).unwrap(),
            VoteStats {
                yes_voting_power: 660_000,
                no_voting_power: 200_000,
                abstain_voting_power: 0
            }
        );
        assert_eq!(
            UserRegionVote::<Test>::get::<u64, AccountId>(0, [2; 32].into()).unwrap(),
            crate::VoteRecord { vote: crate::Vote::No, region_id: 3, power: 200_000 }
        );
        assert_ok!(Regions::vote_on_region_proposal(
            RuntimeOrigin::signed([1; 32].into()),
            3,
            crate::Vote::No,
            400_000
        ));
        assert_eq!(
            OngoingRegionProposalVotes::<Test>::get(0).unwrap(),
            VoteStats {
                yes_voting_power: 150_000,
                no_voting_power: 600_000,
                abstain_voting_power: 0
            }
        );
        run_to_block(31);
        assert_ok!(Regions::unlock_region_voting_token(RuntimeOrigin::signed([1; 32].into()), 0,));
        assert_ok!(Regions::unlock_region_voting_token(RuntimeOrigin::signed([0; 32].into()), 0,));
        assert_ok!(Regions::bid_on_region(RuntimeOrigin::signed([0; 32].into()), 3, 100_000));
        // Second proposal
        assert_ok!(Regions::propose_new_region(
            RuntimeOrigin::signed([0; 32].into()),
            crate::RegionIdentifier::Japan
        ));
        assert_ok!(Regions::vote_on_region_proposal(
            RuntimeOrigin::signed([1; 32].into()),
            3,
            crate::Vote::Yes,
            400_000
        ));
        assert_eq!(
            OngoingRegionProposalVotes::<Test>::get(1).unwrap(),
            VoteStats { yes_voting_power: 400_000, no_voting_power: 0, abstain_voting_power: 0 }
        );
        assert_ok!(Regions::vote_on_region_proposal(
            RuntimeOrigin::signed([0; 32].into()),
            3,
            crate::Vote::No,
            150_000
        ));
        assert_eq!(
            OngoingRegionProposalVotes::<Test>::get(1).unwrap(),
            VoteStats {
                yes_voting_power: 400_000,
                no_voting_power: 150_000,
                abstain_voting_power: 0
            }
        );
    })
}

#[test]
fn vote_on_region_proposal_fails() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);
        assert_ok!(XcavateWhitelist::add_admin(RuntimeOrigin::root(), [20; 32].into(),));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [0; 32].into(),
            pallet_xcavate_whitelist::Role::RegionalOperator
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [8; 32].into(),
            pallet_xcavate_whitelist::Role::RegionalOperator
        ));
        assert_ok!(Balances::force_set_balance(
            RuntimeOrigin::root(),
            sp_runtime::MultiAddress::Id([9; 32].into()),
            900
        ));
        assert_noop!(
            Regions::vote_on_region_proposal(
                RuntimeOrigin::signed([2; 32].into()),
                3,
                crate::Vote::Yes,
                200_000
            ),
            Error::<Test>::NotOngoing
        );
        assert_ok!(Regions::propose_new_region(
            RuntimeOrigin::signed([0; 32].into()),
            crate::RegionIdentifier::Japan
        ));
        assert_noop!(
            Regions::vote_on_region_proposal(
                RuntimeOrigin::signed([9; 32].into()),
                3,
                crate::Vote::Yes,
                10_000
            ),
            Error::<Test>::NotEnoughTokenToVote
        );
        assert_noop!(
            Regions::vote_on_region_proposal(
                RuntimeOrigin::signed([9; 32].into()),
                3,
                crate::Vote::Yes,
                50
            ),
            Error::<Test>::BelowMinimumVotingAmount
        );
        run_to_block(40);
        assert_noop!(
            Regions::vote_on_region_proposal(
                RuntimeOrigin::signed([2; 32].into()),
                3,
                crate::Vote::Yes,
                200_000
            ),
            Error::<Test>::ProposalExpired
        );
    })
}

// unlock_region_voting_token tests

#[test]
fn unlock_region_voting_token_works() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);
        assert_ok!(XcavateWhitelist::add_admin(RuntimeOrigin::root(), [20; 32].into(),));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [0; 32].into(),
            pallet_xcavate_whitelist::Role::RegionalOperator
        ));
        assert_ok!(Regions::propose_new_region(
            RuntimeOrigin::signed([0; 32].into()),
            crate::RegionIdentifier::Japan
        ));
        assert_eq!(
            Balances::balance_on_hold(&HoldReason::RegionVotingReserve.into(), &([2; 32].into())),
            0
        );
        assert_ok!(Regions::vote_on_region_proposal(
            RuntimeOrigin::signed([2; 32].into()),
            3,
            crate::Vote::Yes,
            250_000
        ));
        assert_eq!(
            OngoingRegionProposalVotes::<Test>::get(0).unwrap(),
            VoteStats { yes_voting_power: 250_000, no_voting_power: 0, abstain_voting_power: 0 }
        );
        assert_eq!(
            UserRegionVote::<Test>::get::<u64, AccountId>(0, [2; 32].into()).unwrap(),
            crate::VoteRecord { vote: crate::Vote::Yes, region_id: 3, power: 250_000 }
        );
        assert_eq!(Balances::free_balance(&([2; 32].into())), 50_000);
        assert_eq!(
            Balances::balance_on_hold(&HoldReason::RegionVotingReserve.into(), &([2; 32].into())),
            250_000
        );
        assert_ok!(Regions::vote_on_region_proposal(
            RuntimeOrigin::signed([0; 32].into()),
            3,
            crate::Vote::Yes,
            150_000
        ));
        assert_ok!(Regions::vote_on_region_proposal(
            RuntimeOrigin::signed([1; 32].into()),
            3,
            crate::Vote::No,
            120_000
        ));
        assert_eq!(Balances::free_balance(&([1; 32].into())), 30_000);
        assert_eq!(
            Balances::balance_on_hold(&HoldReason::RegionVotingReserve.into(), &([1; 32].into())),
            120_000
        );
        run_to_block(31);
        assert_ok!(Regions::unlock_region_voting_token(RuntimeOrigin::signed([2; 32].into()), 0,));
        assert!(UserRegionVote::<Test>::get::<u64, AccountId>(0, [2; 32].into()).is_none());
        assert_eq!(Balances::free_balance(&([2; 32].into())), 300_000);
        assert_eq!(
            Balances::balance_on_hold(&HoldReason::RegionVotingReserve.into(), &([2; 32].into())),
            0
        );
        assert_ok!(Regions::unlock_region_voting_token(RuntimeOrigin::signed([1; 32].into()), 0,));
        assert!(UserRegionVote::<Test>::get::<u64, AccountId>(0, [1; 32].into()).is_none());
        assert_eq!(Balances::free_balance(&([1; 32].into())), 150_000);
        assert_eq!(
            Balances::balance_on_hold(&HoldReason::RegionVotingReserve.into(), &([1; 32].into())),
            0
        );
    })
}

#[test]
fn unlock_region_voting_token_fails() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);
        assert_ok!(XcavateWhitelist::add_admin(RuntimeOrigin::root(), [20; 32].into(),));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [0; 32].into(),
            pallet_xcavate_whitelist::Role::RegionalOperator
        ));
        assert_noop!(
            Regions::unlock_region_voting_token(RuntimeOrigin::signed([2; 32].into()), 0),
            Error::<Test>::NoFrozenAmount
        );
        assert_ok!(Regions::propose_new_region(
            RuntimeOrigin::signed([0; 32].into()),
            crate::RegionIdentifier::Japan
        ));
        assert_eq!(
            Balances::balance_on_hold(&HoldReason::RegionVotingReserve.into(), &([2; 32].into())),
            0
        );
        assert_noop!(
            Regions::unlock_region_voting_token(RuntimeOrigin::signed([2; 32].into()), 0),
            Error::<Test>::NoFrozenAmount
        );
        assert_ok!(Regions::vote_on_region_proposal(
            RuntimeOrigin::signed([2; 32].into()),
            3,
            crate::Vote::Yes,
            250_000
        ));
        assert_eq!(
            OngoingRegionProposalVotes::<Test>::get(0).unwrap(),
            VoteStats { yes_voting_power: 250_000, no_voting_power: 0, abstain_voting_power: 0 }
        );
        assert_eq!(
            UserRegionVote::<Test>::get::<u64, AccountId>(0, [2; 32].into()).unwrap(),
            crate::VoteRecord { vote: crate::Vote::Yes, region_id: 3, power: 250_000 }
        );
        assert_eq!(Balances::free_balance(&([2; 32].into())), 50_000);
        assert_eq!(
            Balances::balance_on_hold(&HoldReason::RegionVotingReserve.into(), &([2; 32].into())),
            250_000
        );
        assert_ok!(Regions::vote_on_region_proposal(
            RuntimeOrigin::signed([0; 32].into()),
            3,
            crate::Vote::Yes,
            150_000
        ));
        assert_ok!(Regions::vote_on_region_proposal(
            RuntimeOrigin::signed([1; 32].into()),
            3,
            crate::Vote::No,
            120_000
        ));
        assert_eq!(Balances::free_balance(&([1; 32].into())), 30_000);
        assert_eq!(
            Balances::balance_on_hold(&HoldReason::RegionVotingReserve.into(), &([1; 32].into())),
            120_000
        );
        assert_noop!(
            Regions::unlock_region_voting_token(RuntimeOrigin::signed([2; 32].into()), 0),
            Error::<Test>::VotingStillOngoing
        );
        run_to_block(31);
        assert_ok!(Regions::unlock_region_voting_token(RuntimeOrigin::signed([2; 32].into()), 0,));
        assert_noop!(
            Regions::unlock_region_voting_token(RuntimeOrigin::signed([2; 32].into()), 0),
            Error::<Test>::NoFrozenAmount
        );
    })
}

// bid_on_region tests

#[test]
fn region_rejected() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);
        assert_ok!(XcavateWhitelist::add_admin(RuntimeOrigin::root(), [20; 32].into(),));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [0; 32].into(),
            pallet_xcavate_whitelist::Role::RegionalOperator
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [1; 32].into(),
            pallet_xcavate_whitelist::Role::RegionalOperator
        ));
        // First proposal
        assert_ok!(Regions::propose_new_region(
            RuntimeOrigin::signed([0; 32].into()),
            crate::RegionIdentifier::Japan
        ));
        assert_eq!(
            Balances::balance_on_hold(&HoldReason::RegionProposalReserve.into(), &([0; 32].into())),
            5_000
        );
        assert_ok!(Regions::vote_on_region_proposal(
            RuntimeOrigin::signed([1; 32].into()),
            3,
            crate::Vote::Yes,
            1_000
        ));
        // Quorum not met
        assert_eq!(
            OngoingRegionProposalVotes::<Test>::get(0).unwrap(),
            VoteStats { yes_voting_power: 1_000, no_voting_power: 0, abstain_voting_power: 0 }
        );
        run_to_block(31);
        assert_ok!(Regions::unlock_region_voting_token(RuntimeOrigin::signed([1; 32].into()), 0));
        assert_ok!(Regions::bid_on_region(RuntimeOrigin::signed([1; 32].into()), 3, 10_000));
        assert_eq!(Balances::free_balance(&([0; 32].into())), 195_000);
        assert_eq!(
            Balances::balance_on_hold(&HoldReason::RegionProposalReserve.into(), &([0; 32].into())),
            0
        );
        System::assert_last_event(
            Event::RegionProposalRejected {
                region_id: 3,
                slashed_account: [0; 32].into(),
                amount: 5_000,
            }
            .into(),
        );
        // Second proposal
        assert_ok!(Regions::propose_new_region(
            RuntimeOrigin::signed([0; 32].into()),
            crate::RegionIdentifier::Japan
        ));
        assert_eq!(
            Balances::balance_on_hold(&HoldReason::RegionProposalReserve.into(), &([0; 32].into())),
            5_000
        );
        assert_ok!(Regions::vote_on_region_proposal(
            RuntimeOrigin::signed([0; 32].into()),
            3,
            crate::Vote::Yes,
            150_000
        ));
        assert_ok!(Regions::vote_on_region_proposal(
            RuntimeOrigin::signed([1; 32].into()),
            3,
            crate::Vote::Abstain,
            100_000
        ));
        assert_ok!(Regions::vote_on_region_proposal(
            RuntimeOrigin::signed([2; 32].into()),
            3,
            crate::Vote::No,
            100_000
        ));
        // Threshold of 75 % not met
        assert_eq!(
            OngoingRegionProposalVotes::<Test>::get(1).unwrap(),
            VoteStats {
                yes_voting_power: 150_000,
                no_voting_power: 100_000,
                abstain_voting_power: 100_000
            }
        );
        run_to_block(61);
        assert_ok!(Regions::unlock_region_voting_token(RuntimeOrigin::signed([0; 32].into()), 1));
        assert_ok!(Regions::bid_on_region(RuntimeOrigin::signed([1; 32].into()), 3, 10_000));
        assert_eq!(Balances::free_balance(&([0; 32].into())), 190_000);
        assert_eq!(
            Balances::balance_on_hold(&HoldReason::RegionProposalReserve.into(), &([0; 32].into())),
            0
        );
        System::assert_last_event(
            Event::RegionProposalRejected {
                region_id: 3,
                slashed_account: [0; 32].into(),
                amount: 5_000,
            }
            .into(),
        );
    })
}

#[test]
fn bid_on_region_works() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);
        assert_ok!(XcavateWhitelist::add_admin(RuntimeOrigin::root(), [20; 32].into(),));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [0; 32].into(),
            pallet_xcavate_whitelist::Role::RegionalOperator
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [1; 32].into(),
            pallet_xcavate_whitelist::Role::RegionalOperator
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [2; 32].into(),
            pallet_xcavate_whitelist::Role::RegionalOperator
        ));
        assert_ok!(Regions::propose_new_region(
            RuntimeOrigin::signed([0; 32].into()),
            crate::RegionIdentifier::Japan
        ));
        assert_ok!(Regions::vote_on_region_proposal(
            RuntimeOrigin::signed([0; 32].into()),
            3,
            crate::Vote::Yes,
            100_000
        ));
        run_to_block(31);
        assert_ok!(Balances::force_set_balance(
            RuntimeOrigin::root(),
            sp_runtime::MultiAddress::Id(Regions::treasury_account_id()),
            5_000
        ));
        assert_ok!(Regions::bid_on_region(RuntimeOrigin::signed([2; 32].into()), 3, 10_000));
        assert_ok!(Regions::unlock_region_voting_token(RuntimeOrigin::signed([0; 32].into()), 0,));
        // The account got 5_000 treasury rewards after the proposal passed.
        assert_eq!(Balances::free_balance(&([0; 32].into())), 205_000);
        assert_eq!(
            Balances::balance_on_hold(&HoldReason::RegionProposalReserve.into(), &([0; 32].into())),
            0
        );
        assert_eq!(Balances::free_balance(&([2; 32].into())), 290_000);
        assert_eq!(
            Balances::balance_on_hold(&HoldReason::RegionDepositReserve.into(), &([2; 32].into())),
            10_000
        );
        assert_eq!(RegionAuctions::<Test>::get(3).unwrap().collateral, 10_000);
        assert_eq!(RegionAuctions::<Test>::get(3).unwrap().highest_bidder.unwrap(), [2; 32].into());
        assert_ok!(Regions::bid_on_region(RuntimeOrigin::signed([1; 32].into()), 3, 25_001));
        assert_eq!(Balances::free_balance(&([2; 32].into())), 300_000);
        assert_eq!(
            Balances::balance_on_hold(&HoldReason::RegionDepositReserve.into(), &([2; 32].into())),
            0
        );
        assert_eq!(Balances::free_balance(&([1; 32].into())), 124_999);
        assert_eq!(
            Balances::balance_on_hold(&HoldReason::RegionDepositReserve.into(), &([1; 32].into())),
            25_001
        );
        assert_eq!(RegionAuctions::<Test>::get(3).unwrap().collateral, 25_001);
        assert_eq!(RegionAuctions::<Test>::get(3).unwrap().highest_bidder.unwrap(), [1; 32].into());
        assert_ok!(Regions::bid_on_region(RuntimeOrigin::signed([0; 32].into()), 3, 190_000));
        assert_eq!(RegionAuctions::<Test>::get(3).unwrap().collateral, 190_000);
        assert_eq!(RegionAuctions::<Test>::get(3).unwrap().highest_bidder.unwrap(), [0; 32].into());
        assert_ok!(Regions::bid_on_region(RuntimeOrigin::signed([0; 32].into()), 3, 195_000));
        assert_eq!(RegionAuctions::<Test>::get(3).unwrap().collateral, 195_000);
        assert_eq!(RegionAuctions::<Test>::get(3).unwrap().highest_bidder.unwrap(), [0; 32].into());
        assert_eq!(Balances::free_balance(&([0; 32].into())), 10_000);
        assert_eq!(
            Balances::balance_on_hold(&HoldReason::RegionDepositReserve.into(), &([0; 32].into())),
            195_000
        );
        assert_eq!(Balances::free_balance(&([1; 32].into())), 150_000);
        assert_eq!(
            Balances::balance_on_hold(&HoldReason::RegionDepositReserve.into(), &([1; 32].into())),
            0
        );
    })
}

#[test]
fn bid_on_region_fails() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);
        assert_ok!(XcavateWhitelist::add_admin(RuntimeOrigin::root(), [20; 32].into(),));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [0; 32].into(),
            pallet_xcavate_whitelist::Role::RegionalOperator
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [1; 32].into(),
            pallet_xcavate_whitelist::Role::RegionalOperator
        ));
        assert_ok!(Regions::propose_new_region(
            RuntimeOrigin::signed([0; 32].into()),
            crate::RegionIdentifier::Japan
        ));
        assert_ok!(Regions::vote_on_region_proposal(
            RuntimeOrigin::signed([0; 32].into()),
            3,
            crate::Vote::Yes,
            100_000
        ));
        assert_noop!(
            Regions::bid_on_region(RuntimeOrigin::signed([0; 32].into()), 3, 1_000),
            Error::<Test>::VotingStillOngoing,
        );
        run_to_block(31);
        assert_noop!(
            Regions::bid_on_region(RuntimeOrigin::signed([3; 32].into()), 3, 1_000),
            BadOrigin,
        );
        assert_noop!(
            Regions::bid_on_region(RuntimeOrigin::signed([1; 32].into()), 3, 0),
            Error::<Test>::BidCannotBeZero,
        );
        assert_noop!(
            Regions::bid_on_region(RuntimeOrigin::signed([1; 32].into()), 3, 5_000),
            Error::<Test>::BidBelowMinimum,
        );
        assert_ok!(Regions::bid_on_region(RuntimeOrigin::signed([0; 32].into()), 3, 10_000));
        assert_ok!(Regions::unlock_region_voting_token(RuntimeOrigin::signed([0; 32].into()), 0,));
        assert_noop!(
            Regions::bid_on_region(RuntimeOrigin::signed([1; 32].into()), 3, 5_000),
            Error::<Test>::BidTooLow,
        );
        assert_noop!(
            Regions::bid_on_region(RuntimeOrigin::signed([0; 32].into()), 3, 10_000),
            Error::<Test>::BidTooLow,
        );
        assert_noop!(
            Regions::bid_on_region(RuntimeOrigin::signed([1; 32].into()), 3, 160_000),
            TokenError::FundsUnavailable,
        );
        assert_ok!(Regions::propose_new_region(
            RuntimeOrigin::signed([0; 32].into()),
            crate::RegionIdentifier::France
        ));
        assert_eq!(Balances::free_balance(&([0; 32].into())), 185_000);
        assert_ok!(Regions::vote_on_region_proposal(
            RuntimeOrigin::signed([0; 32].into()),
            2,
            crate::Vote::No,
            120_000
        ));
        assert_eq!(
            OngoingRegionProposalVotes::<Test>::get(1).unwrap(),
            VoteStats { yes_voting_power: 0, no_voting_power: 120_000, abstain_voting_power: 0 }
        );
        run_to_block(61);
        assert_ok!(Regions::bid_on_region(RuntimeOrigin::signed([0; 32].into()), 2, 10_000));
        System::assert_last_event(
            Event::RegionProposalRejected {
                region_id: 2,
                slashed_account: [0; 32].into(),
                amount: 5_000,
            }
            .into(),
        );
        // Proposal got rejected therefore no ongoing auction
        assert_noop!(
            Regions::bid_on_region(RuntimeOrigin::signed([1; 32].into()), 2, 10_000),
            Error::<Test>::NotOngoing,
        );
    })
}

// create_new_region tests

#[test]
fn create_new_region_works() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);
        assert_ok!(XcavateWhitelist::add_admin(RuntimeOrigin::root(), [20; 32].into(),));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [0; 32].into(),
            pallet_xcavate_whitelist::Role::RegionalOperator
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [1; 32].into(),
            pallet_xcavate_whitelist::Role::RegionalOperator
        ));
        // First proposal
        assert_ok!(Regions::propose_new_region(
            RuntimeOrigin::signed([0; 32].into()),
            crate::RegionIdentifier::India
        ));
        assert_ok!(Regions::vote_on_region_proposal(
            RuntimeOrigin::signed([8; 32].into()),
            4,
            crate::Vote::Yes,
            100_000
        ));
        run_to_block(29);
        // Second proposal
        assert_ok!(Regions::propose_new_region(
            RuntimeOrigin::signed([0; 32].into()),
            crate::RegionIdentifier::France
        ));
        assert_ok!(Regions::vote_on_region_proposal(
            RuntimeOrigin::signed([1; 32].into()),
            2,
            crate::Vote::Yes,
            100_000
        ));
        assert_ok!(Regions::vote_on_region_proposal(
            RuntimeOrigin::signed([1; 32].into()),
            2,
            crate::Vote::Abstain,
            100_000
        ));
        run_to_block(31);
        assert_ok!(Regions::bid_on_region(RuntimeOrigin::signed([0; 32].into()), 4, 100_000));
        run_to_block(59);
        assert_ok!(Regions::bid_on_region(RuntimeOrigin::signed([0; 32].into()), 2, 70_000));
        assert_eq!(Balances::free_balance(&([0; 32].into())), 30_000);
        assert_eq!(Balances::free_balance(&([8; 32].into())), 300_000);
        assert_eq!(
            Balances::balance_on_hold(&HoldReason::RegionDepositReserve.into(), &([0; 32].into())),
            170_000
        );
        run_to_block(61);
        assert_ok!(Regions::create_new_region(RuntimeOrigin::signed([0; 32].into()), 4,));
        assert_eq!(RegionAuctions::<Test>::get(4).is_none(), true);
        run_to_block(89);
        assert_ok!(Regions::create_new_region(RuntimeOrigin::signed([0; 32].into()), 2,));
        assert_ok!(Regions::unlock_region_voting_token(RuntimeOrigin::signed([8; 32].into()), 0,));
        assert_eq!(Balances::free_balance(&([8; 32].into())), 400_000);
        assert_eq!(
            Balances::balance_on_hold(&HoldReason::RegionDepositReserve.into(), &([0; 32].into())),
            170_000
        );
        assert_eq!(RegionDetails::<Test>::get(4).unwrap().owner, [0; 32].into());
        assert_eq!(RegionProposalId::<Test>::get(2).is_none(), true);
    })
}

#[test]
fn create_new_region_does_not_work() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);
        assert_ok!(XcavateWhitelist::add_admin(RuntimeOrigin::root(), [20; 32].into(),));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [7; 32].into(),
            pallet_xcavate_whitelist::Role::RegionalOperator
        ));
        assert_noop!(
            Regions::create_new_region(RuntimeOrigin::signed([7; 32].into()), 0,),
            Error::<Test>::NoAuction
        );
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [8; 32].into(),
            pallet_xcavate_whitelist::Role::RegionalOperator
        ));
        assert_ok!(Regions::propose_new_region(
            RuntimeOrigin::signed([8; 32].into()),
            crate::RegionIdentifier::Japan
        ));
        assert_ok!(Regions::vote_on_region_proposal(
            RuntimeOrigin::signed([8; 32].into()),
            3,
            crate::Vote::Yes,
            20_000
        ));
        run_to_block(31);
        assert_ok!(Regions::bid_on_region(RuntimeOrigin::signed([8; 32].into()), 3, 100_000));
        assert_noop!(
            Regions::create_new_region(RuntimeOrigin::signed([7; 32].into()), 3,),
            Error::<Test>::AuctionNotFinished
        );
        assert_eq!(RegionAuctions::<Test>::get(3).is_some(), true);
        run_to_block(61);
        assert_noop!(
            Regions::create_new_region(RuntimeOrigin::signed([7; 32].into()), 3,),
            Error::<Test>::NotRegionOwner
        );
        assert_ok!(Regions::propose_new_region(
            RuntimeOrigin::signed([8; 32].into()),
            crate::RegionIdentifier::India
        ));
        assert_ok!(Regions::vote_on_region_proposal(
            RuntimeOrigin::signed([8; 32].into()),
            4,
            crate::Vote::Yes,
            10_000
        ));
        run_to_block(121);
        assert_noop!(
            Regions::create_new_region(RuntimeOrigin::signed([8; 32].into()), 4,),
            Error::<Test>::NoAuction
        );
    })
}

// propose_remove_regional_operator tests

#[test]
fn propose_remove_regional_operator_works() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);
        assert_ok!(XcavateWhitelist::add_admin(RuntimeOrigin::root(), [20; 32].into(),));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [8; 32].into(),
            pallet_xcavate_whitelist::Role::RegionalOperator
        ));
        new_region_helper();
        assert_ok!(Regions::propose_remove_regional_operator(
            RuntimeOrigin::signed([0; 32].into()),
            3
        ));
        assert_eq!(RegionOwnerProposalId::<Test>::get(3).unwrap(), 1);
        assert_eq!(RegionOwnerProposals::<Test>::get(1).is_some(), true);
        assert_eq!(
            OngoingRegionOwnerProposalVotes::<Test>::get(1).unwrap(),
            VoteStats { yes_voting_power: 0, no_voting_power: 0, abstain_voting_power: 0 }
        );
        assert_eq!(ProposalCounter::<Test>::get(), 2);
    })
}

#[test]
fn propose_remove_regional_operator_fails() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);
        assert_ok!(XcavateWhitelist::add_admin(RuntimeOrigin::root(), [20; 32].into(),));
        assert_noop!(
            Regions::propose_remove_regional_operator(RuntimeOrigin::signed([0; 32].into()), 0),
            Error::<Test>::RegionUnknown,
        );
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [8; 32].into(),
            pallet_xcavate_whitelist::Role::RegionalOperator
        ));
        new_region_helper();
        assert_ok!(Regions::propose_remove_regional_operator(
            RuntimeOrigin::signed([0; 32].into()),
            3
        ));
        assert_eq!(RegionOwnerProposals::<Test>::get(1).is_some(), true);
        assert_eq!(
            OngoingRegionOwnerProposalVotes::<Test>::get(1).unwrap(),
            VoteStats { yes_voting_power: 0, no_voting_power: 0, abstain_voting_power: 0 }
        );
        assert_noop!(
            Regions::propose_remove_regional_operator(RuntimeOrigin::signed([0; 32].into()), 3),
            Error::<Test>::ProposalAlreadyOngoing,
        );
    })
}

// vote_on_remove_owner_proposal tests

#[test]
fn vote_on_remove_owner_proposal_works() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);
        assert_ok!(XcavateWhitelist::add_admin(RuntimeOrigin::root(), [20; 32].into(),));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [8; 32].into(),
            pallet_xcavate_whitelist::Role::RegionalOperator
        ));
        new_region_helper();
        assert_ok!(Regions::propose_remove_regional_operator(
            RuntimeOrigin::signed([0; 32].into()),
            3
        ));
        assert_eq!(RegionOwnerProposals::<Test>::get(1).is_some(), true);
        assert_eq!(
            OngoingRegionOwnerProposalVotes::<Test>::get(1).unwrap(),
            VoteStats { yes_voting_power: 0, no_voting_power: 0, abstain_voting_power: 0 }
        );
        assert_ok!(Regions::vote_on_remove_owner_proposal(
            RuntimeOrigin::signed([0; 32].into()),
            3,
            crate::Vote::Yes,
            150_000
        ));
        assert_ok!(Regions::vote_on_remove_owner_proposal(
            RuntimeOrigin::signed([8; 32].into()),
            3,
            crate::Vote::No,
            250_000
        ));
        assert_eq!(
            OngoingRegionOwnerProposalVotes::<Test>::get(1).unwrap(),
            VoteStats {
                yes_voting_power: 150_000,
                no_voting_power: 250_000,
                abstain_voting_power: 0
            }
        );
        assert_eq!(
            UserRegionOwnerVote::<Test>::get::<u64, AccountId>(1, [0; 32].into()).unwrap(),
            crate::VoteRecord { vote: crate::Vote::Yes, region_id: 3, power: 150_000 }
        );
        assert_eq!(Balances::free_balance(&([0; 32].into())), 49_000);
        assert_eq!(
            Balances::balance_on_hold(
                &HoldReason::RegionOperatorRemovalVoting.into(),
                &([0; 32].into())
            ),
            150_000
        );
        assert_ok!(Regions::vote_on_remove_owner_proposal(
            RuntimeOrigin::signed([0; 32].into()),
            3,
            crate::Vote::Yes,
            110_000
        ));
        assert_eq!(Balances::free_balance(&([0; 32].into())), 89_000);
        assert_eq!(
            Balances::balance_on_hold(
                &HoldReason::RegionOperatorRemovalVoting.into(),
                &([0; 32].into())
            ),
            110_000
        );
    })
}

#[test]
fn vote_on_remove_owner_proposal_fails() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);
        assert_ok!(XcavateWhitelist::add_admin(RuntimeOrigin::root(), [20; 32].into(),));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [8; 32].into(),
            pallet_xcavate_whitelist::Role::RegionalOperator
        ));
        assert_noop!(
            Regions::vote_on_remove_owner_proposal(
                RuntimeOrigin::signed([8; 32].into()),
                3,
                crate::Vote::Yes,
                10_000
            ),
            Error::<Test>::NotOngoing,
        );
        new_region_helper();
        assert_noop!(
            Regions::vote_on_remove_owner_proposal(
                RuntimeOrigin::signed([8; 32].into()),
                3,
                crate::Vote::Yes,
                20_000
            ),
            Error::<Test>::NotOngoing,
        );
        assert_ok!(Regions::propose_remove_regional_operator(
            RuntimeOrigin::signed([0; 32].into()),
            3
        ));
        assert_ok!(Balances::force_set_balance(
            RuntimeOrigin::root(),
            sp_runtime::MultiAddress::Id([9; 32].into()),
            99
        ));
        assert_noop!(
            Regions::vote_on_remove_owner_proposal(
                RuntimeOrigin::signed([9; 32].into()),
                3,
                crate::Vote::Yes,
                20_000
            ),
            Error::<Test>::NotEnoughTokenToVote,
        );
        assert_noop!(
            Regions::vote_on_remove_owner_proposal(
                RuntimeOrigin::signed([9; 32].into()),
                3,
                crate::Vote::Yes,
                20
            ),
            Error::<Test>::BelowMinimumVotingAmount,
        );
        assert_ok!(Regions::vote_on_remove_owner_proposal(
            RuntimeOrigin::signed([0; 32].into()),
            3,
            crate::Vote::Yes,
            30_000
        ));
        run_to_block(91);
        assert_noop!(
            Regions::vote_on_remove_owner_proposal(
                RuntimeOrigin::signed([0; 32].into()),
                3,
                crate::Vote::Yes,
                30_000
            ),
            Error::<Test>::NotOngoing,
        );
    })
}

// unlock_region_owner_removal_voting_token tests

#[test]
fn unlock_region_owner_removal_voting_token_works() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);
        assert_ok!(XcavateWhitelist::add_admin(RuntimeOrigin::root(), [20; 32].into(),));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [8; 32].into(),
            pallet_xcavate_whitelist::Role::RegionalOperator
        ));
        new_region_helper();
        assert_ok!(Regions::propose_remove_regional_operator(
            RuntimeOrigin::signed([0; 32].into()),
            3
        ));
        assert_eq!(RegionOwnerProposals::<Test>::get(1).is_some(), true);
        assert_eq!(
            OngoingRegionOwnerProposalVotes::<Test>::get(1).unwrap(),
            VoteStats { yes_voting_power: 0, no_voting_power: 0, abstain_voting_power: 0 }
        );
        assert_ok!(Regions::vote_on_remove_owner_proposal(
            RuntimeOrigin::signed([0; 32].into()),
            3,
            crate::Vote::Yes,
            150_000
        ));
        assert_ok!(Regions::vote_on_remove_owner_proposal(
            RuntimeOrigin::signed([8; 32].into()),
            3,
            crate::Vote::No,
            250_000
        ));
        assert_eq!(
            OngoingRegionOwnerProposalVotes::<Test>::get(1).unwrap(),
            VoteStats {
                yes_voting_power: 150_000,
                no_voting_power: 250_000,
                abstain_voting_power: 0
            }
        );
        assert_eq!(
            UserRegionOwnerVote::<Test>::get::<u64, AccountId>(1, [0; 32].into()).unwrap(),
            crate::VoteRecord { vote: crate::Vote::Yes, region_id: 3, power: 150_000 }
        );
        assert_eq!(Balances::free_balance(&([0; 32].into())), 49_000);
        assert_eq!(
            Balances::balance_on_hold(
                &HoldReason::RegionOperatorRemovalVoting.into(),
                &([0; 32].into())
            ),
            150_000
        );
        let expiry = frame_system::Pallet::<Test>::block_number() + RegionOperatorVotingTime::get();
        run_to_block(expiry);
        assert_ok!(Regions::unlock_region_owner_removal_voting_token(
            RuntimeOrigin::signed([0; 32].into()),
            1,
        ));
        // Got slashed 1000 because the proposal got rejected
        assert_eq!(Balances::free_balance(&([0; 32].into())), 199_000);
        assert_eq!(
            Balances::balance_on_hold(
                &HoldReason::RegionOperatorRemovalVoting.into(),
                &([0; 32].into())
            ),
            0
        );
        assert!(UserRegionOwnerVote::<Test>::get::<u64, AccountId>(1, [0; 32].into()).is_none());
    })
}

#[test]
fn unlock_region_owner_removal_voting_token_fails() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);
        assert_ok!(XcavateWhitelist::add_admin(RuntimeOrigin::root(), [20; 32].into(),));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [8; 32].into(),
            pallet_xcavate_whitelist::Role::RegionalOperator
        ));
        new_region_helper();
        assert_noop!(
            Regions::unlock_region_owner_removal_voting_token(
                RuntimeOrigin::signed([0; 32].into()),
                1,
            ),
            Error::<Test>::NoFrozenAmount
        );
        assert_ok!(Regions::propose_remove_regional_operator(
            RuntimeOrigin::signed([0; 32].into()),
            3
        ));
        assert_noop!(
            Regions::unlock_region_owner_removal_voting_token(
                RuntimeOrigin::signed([0; 32].into()),
                1,
            ),
            Error::<Test>::NoFrozenAmount
        );
        assert_ok!(Regions::vote_on_remove_owner_proposal(
            RuntimeOrigin::signed([0; 32].into()),
            3,
            crate::Vote::Yes,
            150_000
        ));
        assert_ok!(Regions::vote_on_remove_owner_proposal(
            RuntimeOrigin::signed([8; 32].into()),
            3,
            crate::Vote::No,
            250_000
        ));
        assert_noop!(
            Regions::unlock_region_owner_removal_voting_token(
                RuntimeOrigin::signed([0; 32].into()),
                1,
            ),
            Error::<Test>::VotingStillOngoing
        );
        let expiry = frame_system::Pallet::<Test>::block_number() + RegionOperatorVotingTime::get();
        run_to_block(expiry);
        assert_ok!(Regions::unlock_region_owner_removal_voting_token(
            RuntimeOrigin::signed([0; 32].into()),
            1,
        ));
        assert_noop!(
            Regions::unlock_region_owner_removal_voting_token(
                RuntimeOrigin::signed([0; 32].into()),
                1,
            ),
            Error::<Test>::NoFrozenAmount
        );
    })
}

#[test]
fn remove_owner_proposal_passes() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);
        assert_ok!(XcavateWhitelist::add_admin(RuntimeOrigin::root(), [20; 32].into(),));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [8; 32].into(),
            pallet_xcavate_whitelist::Role::RegionalOperator
        ));
        new_region_helper();
        // First proposal
        assert_ok!(Regions::propose_remove_regional_operator(
            RuntimeOrigin::signed([0; 32].into()),
            3
        ));
        assert_eq!(
            Balances::balance_on_hold(
                &HoldReason::RegionalOperatorRemovalReserve.into(),
                &([0; 32].into())
            ),
            1_000
        );
        assert_ok!(Regions::vote_on_remove_owner_proposal(
            RuntimeOrigin::signed([0; 32].into()),
            3,
            crate::Vote::Yes,
            150_000
        ));
        assert_ok!(Regions::vote_on_remove_owner_proposal(
            RuntimeOrigin::signed([8; 32].into()),
            3,
            crate::Vote::Yes,
            250_000
        ));
        assert_ok!(Regions::vote_on_remove_owner_proposal(
            RuntimeOrigin::signed([2; 32].into()),
            3,
            crate::Vote::Abstain,
            290_000
        ));
        assert_ok!(Regions::vote_on_remove_owner_proposal(
            RuntimeOrigin::signed([1; 32].into()),
            3,
            crate::Vote::Abstain,
            100_000
        ));
        assert_eq!(
            OngoingRegionOwnerProposalVotes::<Test>::get(1).unwrap(),
            VoteStats {
                yes_voting_power: 400_000,
                no_voting_power: 0,
                abstain_voting_power: 390_000
            }
        );
        run_to_block(91);
        // First proposal passed and regional operator gets slashed
        assert_eq!(
            Balances::balance_on_hold(
                &HoldReason::RegionalOperatorRemovalReserve.into(),
                &([0; 32].into())
            ),
            0
        );
        assert_eq!(Balances::total_balance(&([0; 32].into())), 200_000);
        assert_eq!(
            Balances::balance_on_hold(&HoldReason::RegionDepositReserve.into(), &([8; 32].into())),
            90_000
        );
        assert_eq!(Balances::total_balance(&([8; 32].into())), 390_000);
        assert_eq!(Balances::total_issuance(), 1_045_000);
        System::assert_last_event(
            Event::RegionalOperatorSlashed {
                region_id: 3,
                slashed_account: [8; 32].into(),
                amount: 10_000,
                new_collateral_balance: 90_000,
                new_active_strikes: 1,
            }
            .into(),
        );
        assert_ok!(Regions::unlock_region_owner_removal_voting_token(
            RuntimeOrigin::signed([0; 32].into()),
            1,
        ));
        assert_eq!(RegionDetails::<Test>::get(3).unwrap().active_strikes, 1);
        // Second proposal
        assert_ok!(Regions::propose_remove_regional_operator(
            RuntimeOrigin::signed([0; 32].into()),
            3
        ));
        assert_ok!(Regions::vote_on_remove_owner_proposal(
            RuntimeOrigin::signed([0; 32].into()),
            3,
            crate::Vote::Yes,
            150_000
        ));
        run_to_block(121);
        // Second proposal passed and regional operator gets slashed again
        assert_ok!(Regions::unlock_region_owner_removal_voting_token(
            RuntimeOrigin::signed([0; 32].into()),
            2,
        ));
        // Third proposal
        assert_ok!(Regions::propose_remove_regional_operator(
            RuntimeOrigin::signed([0; 32].into()),
            3
        ));
        assert_ok!(Regions::vote_on_remove_owner_proposal(
            RuntimeOrigin::signed([0; 32].into()),
            3,
            crate::Vote::Yes,
            150_000
        ));
        run_to_block(151);
        // Third proposal passed and regional operator gets slashed again and owner change is enabled
        assert_eq!(RegionDetails::<Test>::get(3).unwrap().active_strikes, 3);
        assert_eq!(RegionDetails::<Test>::get(3).unwrap().next_owner_change, 151);
        System::assert_last_event(
            Event::RegionOwnerChangeEnabled { region_id: 3, next_change_allowed: 151 }.into(),
        );
    })
}

#[test]
fn remove_owner_proposal_doesnt_pass() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);
        assert_ok!(XcavateWhitelist::add_admin(RuntimeOrigin::root(), [20; 32].into(),));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [8; 32].into(),
            pallet_xcavate_whitelist::Role::RegionalOperator
        ));
        new_region_helper();
        // First proposal
        assert_ok!(Regions::propose_remove_regional_operator(
            RuntimeOrigin::signed([0; 32].into()),
            3
        ));
        assert_eq!(
            Balances::balance_on_hold(
                &HoldReason::RegionalOperatorRemovalReserve.into(),
                &([0; 32].into())
            ),
            1_000
        );
        assert_eq!(Balances::total_balance(&([0; 32].into())), 200_000);
        run_to_block(91);
        // Proposal fails because of no votes
        assert_eq!(
            Balances::balance_on_hold(
                &HoldReason::RegionalOperatorRemovalReserve.into(),
                &([0; 32].into())
            ),
            0
        );
        assert_eq!(Balances::total_balance(&([0; 32].into())), 199_000);
        assert_eq!(RegionOwnerProposals::<Test>::get(3).is_none(), true);
        assert_eq!(OngoingRegionOwnerProposalVotes::<Test>::get(3).is_none(), true);
        assert_eq!(
            UserRegionOwnerVote::<Test>::get::<u64, AccountId>(1, [0; 32].into()).is_none(),
            true
        );
        assert_eq!(RegionDetails::<Test>::get(3).unwrap().next_owner_change, 361);
        // Second proposal
        assert_ok!(Regions::propose_remove_regional_operator(
            RuntimeOrigin::signed([0; 32].into()),
            3
        ));
        assert_ok!(Regions::vote_on_remove_owner_proposal(
            RuntimeOrigin::signed([0; 32].into()),
            3,
            crate::Vote::Yes,
            150_000
        ));
        assert_ok!(Regions::vote_on_remove_owner_proposal(
            RuntimeOrigin::signed([8; 32].into()),
            3,
            crate::Vote::Yes,
            260_000
        ));
        assert_eq!(
            OngoingRegionOwnerProposalVotes::<Test>::get(2).unwrap(),
            VoteStats { yes_voting_power: 410_000, no_voting_power: 0, abstain_voting_power: 0 }
        );
        assert_ok!(Regions::vote_on_remove_owner_proposal(
            RuntimeOrigin::signed([0; 32].into()),
            3,
            crate::Vote::No,
            160_000
        ));
        assert_ok!(Regions::vote_on_remove_owner_proposal(
            RuntimeOrigin::signed([8; 32].into()),
            3,
            crate::Vote::Yes,
            270_000
        ));
        assert_eq!(
            OngoingRegionOwnerProposalVotes::<Test>::get(2).unwrap(),
            VoteStats {
                yes_voting_power: 270_000,
                no_voting_power: 160_000,
                abstain_voting_power: 0
            }
        );
        run_to_block(121);
        // Proposal fails because the threshold is not met
        assert_ok!(Regions::unlock_region_owner_removal_voting_token(
            RuntimeOrigin::signed([0; 32].into()),
            2,
        ));
        assert_eq!(
            Balances::balance_on_hold(
                &HoldReason::RegionalOperatorRemovalReserve.into(),
                &([0; 32].into())
            ),
            0
        );
        assert_eq!(Balances::total_balance(&([0; 32].into())), 198_000);
        assert_eq!(RegionOwnerProposals::<Test>::get(3).is_none(), true);
        assert_eq!(OngoingRegionOwnerProposalVotes::<Test>::get(3).is_none(), true);
        assert_eq!(
            UserRegionOwnerVote::<Test>::get::<u64, AccountId>(2, [0; 32].into()).is_none(),
            true
        );
        assert_eq!(
            Balances::balance_on_hold(&HoldReason::RegionDepositReserve.into(), &([8; 32].into())),
            100_000
        );
        assert_eq!(Balances::total_balance(&([8; 32].into())), 400_000);
        assert_eq!(Balances::total_issuance(), 1_053_000);
        assert_eq!(RegionOwnerProposals::<Test>::get(3).is_none(), true);
        assert_eq!(OngoingRegionOwnerProposalVotes::<Test>::get(3).is_none(), true);
        // Third proposal
        assert_ok!(Regions::propose_remove_regional_operator(
            RuntimeOrigin::signed([0; 32].into()),
            3
        ));
        assert_ok!(Regions::vote_on_remove_owner_proposal(
            RuntimeOrigin::signed([0; 32].into()),
            3,
            crate::Vote::Yes,
            150_000
        ));
        assert_ok!(Regions::unlock_region_owner_removal_voting_token(
            RuntimeOrigin::signed([8; 32].into()),
            2,
        ));
        run_to_block(151);
        assert_ok!(Regions::unlock_region_owner_removal_voting_token(
            RuntimeOrigin::signed([0; 32].into()),
            3,
        ));
        assert_eq!(
            Balances::balance_on_hold(
                &HoldReason::RegionalOperatorRemovalReserve.into(),
                &([0; 32].into())
            ),
            0
        );
        assert_eq!(Balances::total_balance(&([0; 32].into())), 198_000);
        // Fourth proposal
        assert_ok!(Regions::propose_remove_regional_operator(
            RuntimeOrigin::signed([0; 32].into()),
            3
        ));
        assert_ok!(Regions::vote_on_remove_owner_proposal(
            RuntimeOrigin::signed([0; 32].into()),
            3,
            crate::Vote::Yes,
            150_000
        ));
        assert_ok!(Regions::vote_on_remove_owner_proposal(
            RuntimeOrigin::signed([8; 32].into()),
            3,
            crate::Vote::No,
            260_000
        ));
        assert_eq!(
            OngoingRegionOwnerProposalVotes::<Test>::get(4).unwrap(),
            VoteStats {
                yes_voting_power: 150_000,
                no_voting_power: 260_000,
                abstain_voting_power: 0
            }
        );
        run_to_block(181);
        assert_ok!(Regions::unlock_region_owner_removal_voting_token(
            RuntimeOrigin::signed([0; 32].into()),
            4,
        ));
        assert_ok!(Regions::unlock_region_owner_removal_voting_token(
            RuntimeOrigin::signed([8; 32].into()),
            4,
        ));
        assert_eq!(
            Balances::balance_on_hold(
                &HoldReason::RegionalOperatorRemovalReserve.into(),
                &([0; 32].into())
            ),
            0
        );
        assert_eq!(Balances::total_balance(&([0; 32].into())), 197_000);
        assert_eq!(RegionOwnerProposals::<Test>::get(4).is_none(), true);
        assert_eq!(OngoingRegionOwnerProposalVotes::<Test>::get(4).is_none(), true);
        assert_eq!(
            UserRegionOwnerVote::<Test>::get::<u64, AccountId>(4, [0; 32].into()).is_none(),
            true
        );
        assert_eq!(RegionDetails::<Test>::get(3).unwrap().active_strikes, 1);
        assert_eq!(RegionDetails::<Test>::get(3).unwrap().next_owner_change, 361);
        // Fifth proposal
        assert_ok!(Regions::propose_remove_regional_operator(
            RuntimeOrigin::signed([0; 32].into()),
            3
        ));
        assert_ok!(Regions::vote_on_remove_owner_proposal(
            RuntimeOrigin::signed([0; 32].into()),
            3,
            crate::Vote::Yes,
            150_000
        ));
        assert_ok!(Regions::vote_on_remove_owner_proposal(
            RuntimeOrigin::signed([8; 32].into()),
            3,
            crate::Vote::Yes,
            260_000
        ));
        assert_eq!(
            OngoingRegionOwnerProposalVotes::<Test>::get(5).unwrap(),
            VoteStats { yes_voting_power: 410_000, no_voting_power: 0, abstain_voting_power: 0 }
        );
        assert_ok!(Regions::vote_on_remove_owner_proposal(
            RuntimeOrigin::signed([0; 32].into()),
            3,
            crate::Vote::Abstain,
            160_000
        ));
        assert_ok!(Regions::vote_on_remove_owner_proposal(
            RuntimeOrigin::signed([8; 32].into()),
            3,
            crate::Vote::Yes,
            270_000
        ));
        assert_ok!(Regions::vote_on_remove_owner_proposal(
            RuntimeOrigin::signed([2; 32].into()),
            3,
            crate::Vote::No,
            160_000
        ));
        assert_eq!(
            OngoingRegionOwnerProposalVotes::<Test>::get(5).unwrap(),
            VoteStats {
                yes_voting_power: 270_000,
                no_voting_power: 160_000,
                abstain_voting_power: 160_000
            }
        );
        run_to_block(211);
        // Proposal fails because the threshold is not met
        assert_eq!(
            Balances::balance_on_hold(
                &HoldReason::RegionalOperatorRemovalReserve.into(),
                &([0; 32].into())
            ),
            0
        );
        assert_eq!(Balances::total_balance(&([0; 32].into())), 196_000);
        assert_eq!(RegionOwnerProposals::<Test>::get(5).is_none(), true);
        assert_eq!(OngoingRegionOwnerProposalVotes::<Test>::get(5).is_none(), true);
        assert_eq!(
            UserRegionOwnerVote::<Test>::get::<u64, AccountId>(5, [0; 32].into()).is_none(),
            false
        );
        assert_eq!(RegionDetails::<Test>::get(3).unwrap().active_strikes, 1);
        assert_eq!(RegionDetails::<Test>::get(3).unwrap().next_owner_change, 361);
    })
}

#[test]
fn remove_owner_proposal_doesnt_pass_quorum() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);
        assert_ok!(XcavateWhitelist::add_admin(RuntimeOrigin::root(), [20; 32].into(),));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [8; 32].into(),
            pallet_xcavate_whitelist::Role::RegionalOperator
        ));
        new_region_helper();
        assert_ok!(Regions::propose_remove_regional_operator(
            RuntimeOrigin::signed([0; 32].into()),
            3
        ));
        assert_eq!(
            Balances::balance_on_hold(
                &HoldReason::RegionalOperatorRemovalReserve.into(),
                &([0; 32].into())
            ),
            1_000
        );
        assert_ok!(Regions::vote_on_remove_owner_proposal(
            RuntimeOrigin::signed([8; 32].into()),
            3,
            crate::Vote::Yes,
            1_000
        ));
        assert_eq!(
            OngoingRegionOwnerProposalVotes::<Test>::get(1).unwrap(),
            VoteStats { yes_voting_power: 1_000, no_voting_power: 0, abstain_voting_power: 0 }
        );
        assert_eq!(Balances::total_balance(&([0; 32].into())), 200_000);
        run_to_block(91);
        assert_eq!(
            Balances::balance_on_hold(
                &HoldReason::RegionalOperatorRemovalReserve.into(),
                &([0; 32].into())
            ),
            0
        );
        assert_eq!(Balances::total_balance(&([0; 32].into())), 199_000);
        assert_eq!(RegionOwnerProposals::<Test>::get(3).is_none(), true);
        assert_eq!(OngoingRegionOwnerProposalVotes::<Test>::get(3).is_none(), true);
        assert_eq!(
            UserRegionOwnerVote::<Test>::get::<u64, AccountId>(1, [0; 32].into()).is_none(),
            true
        );
        assert_eq!(RegionDetails::<Test>::get(3).unwrap().next_owner_change, 361);
    })
}

// bid_on_region_replacement tests

#[test]
fn bid_on_region_replacement_after_proposal_works() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);
        assert_ok!(XcavateWhitelist::add_admin(RuntimeOrigin::root(), [20; 32].into(),));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [8; 32].into(),
            pallet_xcavate_whitelist::Role::RegionalOperator
        ));
        new_region_helper();
        // First proposal
        assert_ok!(Regions::propose_remove_regional_operator(
            RuntimeOrigin::signed([0; 32].into()),
            3
        ));
        assert_ok!(Regions::vote_on_remove_owner_proposal(
            RuntimeOrigin::signed([0; 32].into()),
            3,
            crate::Vote::Yes,
            150_000
        ));
        assert_ok!(Regions::vote_on_remove_owner_proposal(
            RuntimeOrigin::signed([8; 32].into()),
            3,
            crate::Vote::Yes,
            260_000
        ));
        run_to_block(91);
        assert_ok!(Regions::unlock_region_owner_removal_voting_token(
            RuntimeOrigin::signed([0; 32].into()),
            1,
        ));
        assert_ok!(Regions::unlock_region_owner_removal_voting_token(
            RuntimeOrigin::signed([8; 32].into()),
            1,
        ));
        // Second proposal
        assert_ok!(Regions::propose_remove_regional_operator(
            RuntimeOrigin::signed([0; 32].into()),
            3
        ));
        assert_ok!(Regions::vote_on_remove_owner_proposal(
            RuntimeOrigin::signed([0; 32].into()),
            3,
            crate::Vote::Yes,
            150_000
        ));
        run_to_block(121);
        assert_ok!(Regions::unlock_region_owner_removal_voting_token(
            RuntimeOrigin::signed([0; 32].into()),
            2,
        ));
        assert_eq!(RegionDetails::<Test>::get(3).unwrap().active_strikes, 2);
        // Third proposal
        assert_ok!(Regions::propose_remove_regional_operator(
            RuntimeOrigin::signed([0; 32].into()),
            3
        ));
        assert_ok!(Regions::vote_on_remove_owner_proposal(
            RuntimeOrigin::signed([0; 32].into()),
            3,
            crate::Vote::Yes,
            150_000
        ));
        assert_eq!(RegionDetails::<Test>::get(3).unwrap().next_owner_change, 361);
        run_to_block(152);
        // After 3 strikes, owner change is enabled
        assert_eq!(RegionDetails::<Test>::get(3).unwrap().next_owner_change, 151);
        assert_eq!(Balances::total_balance(&([0; 32].into())), 200_000);
        assert_eq!(RegionReplacementAuctions::<Test>::get(3).is_none(), true);
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [0; 32].into(),
            pallet_xcavate_whitelist::Role::RegionalOperator
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [1; 32].into(),
            pallet_xcavate_whitelist::Role::RegionalOperator
        ));
        assert_ok!(Regions::bid_on_region_replacement(
            RuntimeOrigin::signed([0; 32].into()),
            3,
            12_000
        ));
        assert_eq!(
            Balances::balance_on_hold(&HoldReason::RegionDepositReserve.into(), &([8; 32].into())),
            70_000
        );
        assert_eq!(
            Balances::balance_on_hold(&HoldReason::RegionDepositReserve.into(), &([0; 32].into())),
            12_000
        );
        assert_eq!(
            RegionReplacementAuctions::<Test>::get(3).unwrap(),
            crate::RegionAuction {
                highest_bidder: Some([0; 32].into()),
                collateral: 12_000,
                auction_expiry: 182,
            }
        );
        run_to_block(170);
        assert_ok!(Regions::unlock_region_owner_removal_voting_token(
            RuntimeOrigin::signed([0; 32].into()),
            3,
        ));
        assert_ok!(Regions::bid_on_region_replacement(
            RuntimeOrigin::signed([1; 32].into()),
            3,
            35_000
        ));
        assert_eq!(
            RegionReplacementAuctions::<Test>::get(3).unwrap(),
            crate::RegionAuction {
                highest_bidder: Some([1; 32].into()),
                collateral: 35_000,
                auction_expiry: 182,
            }
        );
        assert_eq!(
            Balances::balance_on_hold(&HoldReason::RegionDepositReserve.into(), &([0; 32].into())),
            0
        );
        assert_eq!(
            Balances::balance_on_hold(&HoldReason::RegionDepositReserve.into(), &([1; 32].into())),
            35_000
        );
        assert_ok!(Regions::bid_on_region_replacement(
            RuntimeOrigin::signed([8; 32].into()),
            3,
            40_000
        ));
        assert_eq!(
            Balances::balance_on_hold(&HoldReason::RegionDepositReserve.into(), &([1; 32].into())),
            0
        );
        assert_eq!(
            Balances::balance_on_hold(&HoldReason::RegionDepositReserve.into(), &([8; 32].into())),
            110_000
        );
        assert_ok!(Regions::bid_on_region_replacement(
            RuntimeOrigin::signed([0; 32].into()),
            3,
            51_000
        ));
        run_to_block(213);
        assert_eq!(RegionDetails::<Test>::get(3).unwrap().owner, [0; 32].into());
        assert_eq!(RegionDetails::<Test>::get(3).unwrap().collateral, 51_000);
        assert_eq!(RegionDetails::<Test>::get(3).unwrap().next_owner_change, 482);
        assert_eq!(RegionDetails::<Test>::get(3).unwrap().active_strikes, 0);
        assert_eq!(
            Balances::balance_on_hold(&HoldReason::RegionDepositReserve.into(), &([8; 32].into())),
            0
        );
        assert_eq!(
            Balances::balance_on_hold(&HoldReason::RegionDepositReserve.into(), &([0; 32].into())),
            51_000
        );
        assert_eq!(RegionReplacementAuctions::<Test>::get(3).is_none(), true);
    })
}

#[test]
fn bid_on_region_replacement_after_time_works() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);
        assert_ok!(XcavateWhitelist::add_admin(RuntimeOrigin::root(), [20; 32].into(),));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [8; 32].into(),
            pallet_xcavate_whitelist::Role::RegionalOperator
        ));
        new_region_helper();
        assert_eq!(RegionDetails::<Test>::get(3).unwrap().next_owner_change, 361);
        run_to_block(362);
        assert_eq!(RegionReplacementAuctions::<Test>::get(3).is_none(), true);
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [0; 32].into(),
            pallet_xcavate_whitelist::Role::RegionalOperator
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [1; 32].into(),
            pallet_xcavate_whitelist::Role::RegionalOperator
        ));
        assert_ok!(Regions::bid_on_region_replacement(
            RuntimeOrigin::signed([1; 32].into()),
            3,
            80_000
        ));
        assert_eq!(
            Balances::balance_on_hold(&HoldReason::RegionDepositReserve.into(), &([8; 32].into())),
            100_000
        );
        assert_eq!(
            Balances::balance_on_hold(&HoldReason::RegionDepositReserve.into(), &([1; 32].into())),
            80_000
        );
        assert_eq!(
            RegionReplacementAuctions::<Test>::get(3).unwrap(),
            crate::RegionAuction {
                highest_bidder: Some([1; 32].into()),
                collateral: 80_000,
                auction_expiry: 392,
            }
        );
        run_to_block(370);
        assert_ok!(Regions::bid_on_region_replacement(
            RuntimeOrigin::signed([1; 32].into()),
            3,
            85_000
        ));
        assert_eq!(
            RegionReplacementAuctions::<Test>::get(3).unwrap(),
            crate::RegionAuction {
                highest_bidder: Some([1; 32].into()),
                collateral: 85_000,
                auction_expiry: 392,
            }
        );
        assert_eq!(
            Balances::balance_on_hold(&HoldReason::RegionDepositReserve.into(), &([1; 32].into())),
            85_000
        );
        assert_ok!(Regions::bid_on_region_replacement(
            RuntimeOrigin::signed([8; 32].into()),
            3,
            90_000
        ));
        assert_eq!(
            Balances::balance_on_hold(&HoldReason::RegionDepositReserve.into(), &([1; 32].into())),
            0
        );
        assert_eq!(
            Balances::balance_on_hold(&HoldReason::RegionDepositReserve.into(), &([8; 32].into())),
            190_000
        );
        assert_ok!(Regions::bid_on_region_replacement(
            RuntimeOrigin::signed([0; 32].into()),
            3,
            92_000
        ));
        run_to_block(394);
        assert_eq!(RegionDetails::<Test>::get(3).unwrap().owner, [0; 32].into());
        assert_eq!(RegionDetails::<Test>::get(3).unwrap().collateral, 92_000);
        assert_eq!(RegionDetails::<Test>::get(3).unwrap().next_owner_change, 692);
        assert_eq!(
            Balances::balance_on_hold(&HoldReason::RegionDepositReserve.into(), &([8; 32].into())),
            0
        );
        assert_eq!(
            Balances::balance_on_hold(&HoldReason::RegionDepositReserve.into(), &([0; 32].into())),
            92_000
        );
        assert_eq!(RegionReplacementAuctions::<Test>::get(3).is_none(), true);
    })
}

#[test]
fn bid_on_region_replacement_after_time_fails() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);
        assert_ok!(XcavateWhitelist::add_admin(RuntimeOrigin::root(), [20; 32].into(),));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [8; 32].into(),
            pallet_xcavate_whitelist::Role::RegionalOperator
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [0; 32].into(),
            pallet_xcavate_whitelist::Role::RegionalOperator
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [1; 32].into(),
            pallet_xcavate_whitelist::Role::RegionalOperator
        ));
        assert_noop!(
            Regions::bid_on_region_replacement(RuntimeOrigin::signed([1; 32].into()), 0, 10_000),
            Error::<Test>::RegionUnknown
        );
        new_region_helper();
        assert_eq!(RegionDetails::<Test>::get(3).unwrap().next_owner_change, 361);
        run_to_block(200);
        assert_noop!(
            Regions::bid_on_region_replacement(RuntimeOrigin::signed([1; 32].into()), 3, 10_000),
            Error::<Test>::RegionOwnerCantBeChanged
        );
        run_to_block(362);
        assert_noop!(
            Regions::bid_on_region_replacement(RuntimeOrigin::signed([2; 32].into()), 3, 10_000),
            BadOrigin
        );
        assert_noop!(
            Regions::bid_on_region_replacement(RuntimeOrigin::signed([1; 32].into()), 3, 9_500),
            Error::<Test>::BidBelowMinimum
        );
        assert_ok!(Regions::bid_on_region_replacement(
            RuntimeOrigin::signed([0; 32].into()),
            3,
            80_000
        ));
        assert_noop!(
            Regions::bid_on_region_replacement(RuntimeOrigin::signed([1; 32].into()), 3, 10_000),
            Error::<Test>::BidTooLow
        );
        assert_noop!(
            Regions::bid_on_region_replacement(RuntimeOrigin::signed([1; 32].into()), 3, 200_000),
            TokenError::FundsUnavailable,
        );
        assert_noop!(
            Regions::bid_on_region_replacement(RuntimeOrigin::signed([0; 32].into()), 3, 300_000),
            TokenError::FundsUnavailable,
        );
        assert_eq!(
            Balances::balance_on_hold(&HoldReason::RegionDepositReserve.into(), &([0; 32].into())),
            80_000
        );
    })
}

// initiate_region_owner_resignation tests

#[test]
fn initiate_region_owner_resignation_works() {
    new_test_ext().execute_with(|| {
        assert_ok!(XcavateWhitelist::add_admin(RuntimeOrigin::root(), [20; 32].into(),));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [8; 32].into(),
            pallet_xcavate_whitelist::Role::RegionalOperator
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [0; 32].into(),
            pallet_xcavate_whitelist::Role::RegionalOperator
        ));
        new_region_helper();
        assert_eq!(RegionDetails::<Test>::get(3).unwrap().next_owner_change, 361);
        run_to_block(100);
        assert_ok!(Regions::initiate_region_owner_resignation(
            RuntimeOrigin::signed([8; 32].into()),
            3
        ));
        assert_eq!(RegionDetails::<Test>::get(3).unwrap().next_owner_change, 200);
        System::assert_last_event(
            Event::RegionOwnerResignationInitiated {
                region_id: 3,
                region_owner: [8; 32].into(),
                next_owner_change: 200,
            }
            .into(),
        );
        run_to_block(201);
        assert_ok!(Regions::bid_on_region_replacement(
            RuntimeOrigin::signed([0; 32].into()),
            3,
            80_000
        ));
    })
}

#[test]
fn initiate_region_owner_resignation_fails() {
    new_test_ext().execute_with(|| {
        assert_ok!(XcavateWhitelist::add_admin(RuntimeOrigin::root(), [20; 32].into(),));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [8; 32].into(),
            pallet_xcavate_whitelist::Role::RegionalOperator
        ));
        assert_ok!(XcavateWhitelist::assign_role(
            RuntimeOrigin::signed([20; 32].into()),
            [0; 32].into(),
            pallet_xcavate_whitelist::Role::RegionalOperator
        ));
        assert_noop!(
            Regions::initiate_region_owner_resignation(RuntimeOrigin::signed([8; 32].into()), 3),
            Error::<Test>::RegionUnknown
        );
        new_region_helper();
        assert_noop!(
            Regions::initiate_region_owner_resignation(RuntimeOrigin::signed([0; 32].into()), 3),
            Error::<Test>::NotRegionOwner
        );
        assert_eq!(RegionDetails::<Test>::get(3).unwrap().next_owner_change, 361);
        run_to_block(300);
        assert_noop!(
            Regions::initiate_region_owner_resignation(RuntimeOrigin::signed([8; 32].into()), 3),
            Error::<Test>::OwnerChangeAlreadyScheduled
        );
        run_to_block(400);
        assert_noop!(
            Regions::initiate_region_owner_resignation(RuntimeOrigin::signed([8; 32].into()), 3),
            Error::<Test>::OwnerChangeAlreadyScheduled
        );
    })
}
