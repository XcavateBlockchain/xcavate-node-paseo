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

//! Benchmarking setup for pallet-regions
#![cfg(feature = "runtime-benchmarks")]
use super::*;

#[allow(unused)]
use crate::Pallet as Regions;
use frame_benchmarking::v2::*;
use frame_support::assert_ok;
use frame_system::RawOrigin;
use pallet_xcavate_whitelist::Pallet as Whitelist;
use pallet_xcavate_whitelist::Role;
use scale_info::prelude::vec;

pub trait Config: pallet_xcavate_whitelist::Config + crate::Config {}

impl<T: crate::Config + pallet_xcavate_whitelist::Config> Config for T {}

#[benchmarks]
mod benchmarks {
    use super::*;

    fn create_whitelisted_user<T: Config>() -> (T::AccountId, T::AccountId) {
        let admin: T::AccountId = account("admin", 0, 0);
        let signer: T::AccountId = account("signer", 0, 0);
        whitelist_account!(signer);
        assert_ok!(Whitelist::<T>::add_admin(RawOrigin::Root.into(), admin.clone()));
        assert_ok!(Whitelist::<T>::assign_role(
            RawOrigin::Signed(admin.clone()).into(),
            signer.clone(),
            Role::RealEstateInvestor
        ));
        (signer, admin)
    }

    fn create_a_new_region<T: Config>(signer: T::AccountId) -> u16 {
        let region = RegionIdentifier::France;
        let region_id = region.clone().into_u16();

        let deposit = T::RegionProposalDeposit::get();
        let _ = <T as pallet::Config>::NativeCurrency::mint_into(&signer, deposit * 1000u32.into());

        LastRegionProposalBlock::<T>::kill();
        let admin: T::AccountId = account("admin", 0, 0);
        assert_ok!(Whitelist::<T>::assign_role(
            RawOrigin::Signed(admin).into(),
            signer.clone(),
            Role::RegionalOperator
        ));
        assert_ok!(Regions::<T>::propose_new_region(
            RawOrigin::Signed(signer.clone()).into(),
            region.clone()
        ));
        assert_ok!(Regions::<T>::vote_on_region_proposal(
            RawOrigin::Signed(signer.clone()).into(),
            region_id,
            Vote::Yes,
            deposit * 80u32.into(),
        ));

        let auction_amount = T::MinimumRegionDeposit::get();
        let _ = <T as pallet::Config>::NativeCurrency::mint_into(&signer, auction_amount * 100u32.into());

        let bid_amount = auction_amount.saturating_mul(10u32.into());

        let expiry = frame_system::Pallet::<T>::block_number() + T::RegionVotingTime::get();
        frame_system::Pallet::<T>::set_block_number(expiry);

        assert_ok!(Regions::<T>::bid_on_region(
            RawOrigin::Signed(signer.clone()).into(),
            region_id,
            bid_amount
        ));

        let auction_expiry =
            frame_system::Pallet::<T>::block_number() + T::RegionAuctionTime::get();
        frame_system::Pallet::<T>::set_block_number(auction_expiry);
        assert_ok!(Regions::<T>::create_new_region(
            RawOrigin::Signed(signer.clone()).into(),
            region_id,
        ));
        region_id
    }

    #[benchmark]
    fn propose_new_region() {
        let (signer, admin): (T::AccountId, T::AccountId) = create_whitelisted_user::<T>();

        let region = RegionIdentifier::France;

        assert!(!RegionProposalId::<T>::contains_key(region.clone().into_u16()));
        assert!(!RegionDetails::<T>::contains_key(region.clone().into_u16()));

        let deposit = T::RegionProposalDeposit::get();
        let _ = <T as pallet::Config>::NativeCurrency::mint_into(&signer, deposit * 10u32.into());

        LastRegionProposalBlock::<T>::kill();
        assert_ok!(Whitelist::<T>::assign_role(
            RawOrigin::Signed(admin).into(),
            signer.clone(),
            Role::RegionalOperator
        ));

        #[extrinsic_call]
        propose_new_region(RawOrigin::Signed(signer.clone()), region.clone());

        let current_block = frame_system::Pallet::<T>::block_number();
        let expiry_block = current_block.saturating_add(T::RegionVotingTime::get());

        let proposal_id = RegionProposalId::<T>::get(region.into_u16()).unwrap();
        assert_eq!(RegionProposals::<T>::get(proposal_id).unwrap().proposal_expiry, expiry_block);
    }

    #[benchmark]
    fn vote_on_region_proposal() {
        let (signer, admin): (T::AccountId, T::AccountId) = create_whitelisted_user::<T>();

        let region = RegionIdentifier::France;
        let region_id = region.clone().into_u16();

        let deposit = T::RegionProposalDeposit::get();
        let _ = <T as pallet::Config>::NativeCurrency::mint_into(&signer, deposit * 10u32.into());

        LastRegionProposalBlock::<T>::kill();
        assert_ok!(Whitelist::<T>::assign_role(
            RawOrigin::Signed(admin).into(),
            signer.clone(),
            Role::RegionalOperator
        ));
        assert_ok!(Regions::<T>::propose_new_region(
            RawOrigin::Signed(signer.clone()).into(),
            region.clone()
        ));

        assert_ok!(Regions::<T>::vote_on_region_proposal(
            RawOrigin::Signed(signer.clone()).into(),
            region_id,
            Vote::Yes,
            deposit * 2u32.into()
        ));

        #[extrinsic_call]
        vote_on_region_proposal(RawOrigin::Signed(signer.clone()), region_id, Vote::Yes, deposit);

        let proposal_id = RegionProposalId::<T>::get(region_id).unwrap();

        assert_eq!(UserRegionVote::<T>::get(proposal_id, &signer).unwrap().power, deposit);
        assert_eq!(UserRegionVote::<T>::get(proposal_id, &signer).unwrap().vote, Vote::Yes);
    }

    #[benchmark]
    fn unlock_region_voting_token() {
        let (signer, admin): (T::AccountId, T::AccountId) = create_whitelisted_user::<T>();

        let region = RegionIdentifier::France;
        let region_id = region.clone().into_u16();

        let deposit = T::RegionProposalDeposit::get();
        let _ = <T as pallet::Config>::NativeCurrency::mint_into(&signer, deposit * 10u32.into());

        LastRegionProposalBlock::<T>::kill();
        assert_ok!(Whitelist::<T>::assign_role(
            RawOrigin::Signed(admin.clone()).into(),
            signer.clone(),
            Role::RegionalOperator
        ));
        assert_ok!(Regions::<T>::propose_new_region(
            RawOrigin::Signed(signer.clone()).into(),
            region.clone()
        ));

        let voter: T::AccountId = account("voter", 0, 0);
        let _ = <T as pallet::Config>::NativeCurrency::mint_into(&voter, deposit * 10u32.into());
        assert_ok!(Whitelist::<T>::assign_role(
            RawOrigin::Signed(admin).into(),
            voter.clone(),
            Role::RealEstateInvestor
        ));

        assert_ok!(Regions::<T>::vote_on_region_proposal(
            RawOrigin::Signed(voter.clone()).into(),
            region_id,
            Vote::Yes,
            deposit * 2u32.into()
        ));

        let proposal_id = RegionProposalId::<T>::get(region_id).unwrap();
        let expiry = frame_system::Pallet::<T>::block_number() + T::RegionVotingTime::get();
        frame_system::Pallet::<T>::set_block_number(expiry);
        let voter_balance_after_vote = <T as pallet::Config>::NativeCurrency::balance(&voter);

        #[extrinsic_call]
        unlock_region_voting_token(RawOrigin::Signed(voter.clone()), proposal_id);

        assert_eq!(<T as pallet::Config>::NativeCurrency::balance(&voter), voter_balance_after_vote + deposit * 2u32.into());
        assert!(UserRegionVote::<T>::get(proposal_id, &voter).is_none());
    }

    #[benchmark]
    fn bid_on_region() {
        let (signer, admin): (T::AccountId, T::AccountId) = create_whitelisted_user::<T>();

        let region = RegionIdentifier::France;
        let region_id = region.clone().into_u16();

        let deposit = T::RegionProposalDeposit::get();
        let _ = <T as pallet::Config>::NativeCurrency::mint_into(&signer, deposit * 1000u32.into());

        LastRegionProposalBlock::<T>::kill();
        assert_ok!(Whitelist::<T>::assign_role(
            RawOrigin::Signed(admin.clone()).into(),
            signer.clone(),
            Role::RegionalOperator
        ));
        assert_ok!(Regions::<T>::propose_new_region(
            RawOrigin::Signed(signer.clone()).into(),
            region.clone()
        ));
        assert_ok!(Regions::<T>::vote_on_region_proposal(
            RawOrigin::Signed(signer.clone()).into(),
            region_id,
            Vote::Yes,
            deposit * 80u32.into()
        ));

        for i in 1..250 {
            let voter: T::AccountId = account("voter", i, i);
            let _ = <T as pallet::Config>::NativeCurrency::mint_into(&voter, deposit * 1000u32.into());
            assert_ok!(Whitelist::<T>::assign_role(
                RawOrigin::Signed(admin.clone()).into(),
                voter.clone(),
                Role::RealEstateInvestor
            ));
            assert_ok!(Regions::<T>::vote_on_region_proposal(
                RawOrigin::Signed(voter.clone()).into(),
                region_id,
                Vote::Yes,
                deposit * 100u32.into()
            ));
        }

        let expiry = frame_system::Pallet::<T>::block_number() + T::RegionVotingTime::get();
        frame_system::Pallet::<T>::set_block_number(expiry);

        let auction_amount = T::MinimumRegionDeposit::get();
        let _ = <T as pallet::Config>::NativeCurrency::mint_into(&signer, auction_amount * 100u32.into());

        let bid_amount = auction_amount.saturating_mul(10u32.into());

        let first_bidder: T::AccountId = account("first_bidder", 0, 0);

        let _ = <T as pallet::Config>::NativeCurrency::mint_into(&first_bidder, auction_amount * 100u32.into());

        let first_bid_amount = auction_amount.saturating_mul(9u32.into());
        assert_ok!(Whitelist::<T>::assign_role(
            RawOrigin::Signed(admin).into(),
            first_bidder.clone(),
            Role::RegionalOperator
        ));
        assert_ok!(Regions::<T>::bid_on_region(
            RawOrigin::Signed(first_bidder).into(),
            region_id,
            first_bid_amount
        ));

        #[extrinsic_call]
        bid_on_region(RawOrigin::Signed(signer.clone()), region_id, bid_amount);

        assert_eq!(RegionAuctions::<T>::get(region_id).unwrap().highest_bidder, Some(signer));
    }

    #[benchmark]
    fn create_new_region() {
        let (signer, admin): (T::AccountId, T::AccountId) = create_whitelisted_user::<T>();

        let region = RegionIdentifier::France;
        let region_id = region.clone().into_u16();

        let deposit = T::RegionProposalDeposit::get();
        let _ = <T as pallet::Config>::NativeCurrency::mint_into(&signer, deposit * 1000u32.into());

        LastRegionProposalBlock::<T>::kill();
        assert_ok!(Whitelist::<T>::assign_role(
            RawOrigin::Signed(admin).into(),
            signer.clone(),
            Role::RegionalOperator
        ));
        assert_ok!(Regions::<T>::propose_new_region(
            RawOrigin::Signed(signer.clone()).into(),
            region.clone()
        ));
        assert_ok!(Regions::<T>::vote_on_region_proposal(
            RawOrigin::Signed(signer.clone()).into(),
            region_id,
            Vote::Yes,
            deposit * 80_u32.into()
        ));

        let auction_amount = T::MinimumRegionDeposit::get();
        let _ = <T as pallet::Config>::NativeCurrency::mint_into(&signer, auction_amount * 100u32.into());

        let bid_amount = auction_amount.saturating_mul(10u32.into());

        let expiry = frame_system::Pallet::<T>::block_number() + T::RegionVotingTime::get();
        frame_system::Pallet::<T>::set_block_number(expiry);

        assert_ok!(Regions::<T>::bid_on_region(
            RawOrigin::Signed(signer.clone()).into(),
            region_id,
            bid_amount
        ));

        let auction_expiry =
            frame_system::Pallet::<T>::block_number() + T::RegionAuctionTime::get();
        frame_system::Pallet::<T>::set_block_number(auction_expiry);

        #[extrinsic_call]
        create_new_region(RawOrigin::Signed(signer.clone()), region_id);

        assert_eq!(RegionDetails::<T>::get(region_id).unwrap().owner, signer);
    }

    #[benchmark]
    fn propose_remove_regional_operator() {
        let (signer, admin): (T::AccountId, T::AccountId) = create_whitelisted_user::<T>();
        let region_id = create_a_new_region::<T>(signer.clone());

        let proposer: T::AccountId = account("proposer", 0, 0);
        assert_ok!(Whitelist::<T>::assign_role(
            RawOrigin::Signed(admin).into(),
            proposer.clone(),
            Role::RealEstateInvestor
        ));

        let deposit = T::RegionProposalDeposit::get();
        let _ = <T as pallet::Config>::NativeCurrency::mint_into(&proposer, deposit * 10u32.into());

        let expiry_block =
            frame_system::Pallet::<T>::block_number() + T::RegionOperatorVotingTime::get();
        let dummy_region = u16::MAX;
        let max_proposals = T::MaxProposalsForBlock::get() as usize;
        RegionOwnerRoundsExpiring::<T>::insert(
            expiry_block,
            BoundedVec::truncate_from(vec![dummy_region; max_proposals - 1]),
        );

        #[extrinsic_call]
        propose_remove_regional_operator(RawOrigin::Signed(proposer.clone()), region_id);

        let proposal_id = RegionOwnerProposalId::<T>::get(region_id).unwrap();
        assert!(RegionOwnerProposals::<T>::contains_key(proposal_id));
        assert!(OngoingRegionOwnerProposalVotes::<T>::contains_key(proposal_id));
    }

    #[benchmark]
    fn vote_on_remove_owner_proposal() {
        let (signer, admin): (T::AccountId, T::AccountId) = create_whitelisted_user::<T>();
        let region_id = create_a_new_region::<T>(signer.clone());

        let proposer: T::AccountId = account("proposer", 0, 0);
        assert_ok!(Whitelist::<T>::assign_role(
            RawOrigin::Signed(admin.clone()).into(),
            proposer.clone(),
            Role::RealEstateInvestor
        ));

        let voter: T::AccountId = account("voter", 0, 0);
        assert_ok!(Whitelist::<T>::assign_role(
            RawOrigin::Signed(admin).into(),
            voter.clone(),
            Role::RealEstateInvestor
        ));

        let deposit = T::RegionProposalDeposit::get() * 100u32.into();
        let _ = <T as pallet::Config>::NativeCurrency::mint_into(&proposer, deposit);

        let vote_power = T::MinimumVotingAmount::get() * 100u32.into();
        let _ = <T as pallet::Config>::NativeCurrency::mint_into(&voter, vote_power);
        assert_ok!(Regions::<T>::propose_remove_regional_operator(
            RawOrigin::Signed(proposer.clone()).into(),
            region_id
        ));

        assert_ok!(Regions::<T>::vote_on_remove_owner_proposal(
            RawOrigin::Signed(voter.clone()).into(),
            region_id,
            Vote::No,
            vote_power / 10u32.into()
        ));

        #[extrinsic_call]
        vote_on_remove_owner_proposal(
            RawOrigin::Signed(voter.clone()),
            region_id,
            Vote::Yes,
            vote_power / 5u32.into(),
        );

        let proposal_id = RegionOwnerProposalId::<T>::get(region_id).unwrap();
        assert_eq!(
            OngoingRegionOwnerProposalVotes::<T>::get(proposal_id).unwrap().yes_voting_power,
            vote_power / 5u32.into()
        );
        assert!(UserRegionOwnerVote::<T>::get(proposal_id, &voter).is_some());
    }

    #[benchmark]
    fn unlock_region_owner_removal_voting_token() {
        let (signer, admin): (T::AccountId, T::AccountId) = create_whitelisted_user::<T>();
        let region_id = create_a_new_region::<T>(signer.clone());

        let proposer: T::AccountId = account("proposer", 0, 0);
        assert_ok!(Whitelist::<T>::assign_role(
            RawOrigin::Signed(admin.clone()).into(),
            proposer.clone(),
            Role::RealEstateInvestor
        ));

        let voter: T::AccountId = account("voter", 0, 0);
        assert_ok!(Whitelist::<T>::assign_role(
            RawOrigin::Signed(admin).into(),
            voter.clone(),
            Role::RealEstateInvestor
        ));

        let deposit = T::RegionProposalDeposit::get() * 100u32.into();
        let _ = <T as pallet::Config>::NativeCurrency::mint_into(&proposer, deposit);

        let vote_power = T::MinimumVotingAmount::get() * 100u32.into();
        let _ = <T as pallet::Config>::NativeCurrency::mint_into(&voter, vote_power);
        assert_ok!(Regions::<T>::propose_remove_regional_operator(
            RawOrigin::Signed(proposer.clone()).into(),
            region_id
        ));

        assert_ok!(Regions::<T>::vote_on_remove_owner_proposal(
            RawOrigin::Signed(voter.clone()).into(),
            region_id,
            Vote::No,
            vote_power / 10u32.into()
        ));

        let expiry = frame_system::Pallet::<T>::block_number() + T::RegionVotingTime::get();
        frame_system::Pallet::<T>::set_block_number(expiry);

        let voter_balance_after_vote = <T as pallet::Config>::NativeCurrency::balance(&voter);

        let proposal_id = RegionOwnerProposalId::<T>::get(region_id).unwrap();

        #[extrinsic_call]
        unlock_region_owner_removal_voting_token(RawOrigin::Signed(voter.clone()), proposal_id);

        let proposal_id = RegionOwnerProposalId::<T>::get(region_id).unwrap();
        assert!(UserRegionOwnerVote::<T>::get(proposal_id, &voter).is_none());
        assert_eq!(<T as pallet::Config>::NativeCurrency::balance(&voter), voter_balance_after_vote + vote_power / 10u32.into());
    }

    #[benchmark]
    fn bid_on_region_replacement() {
        let (signer, admin): (T::AccountId, T::AccountId) = create_whitelisted_user::<T>();
        let region_id = create_a_new_region::<T>(signer.clone());

        let bidder_1: T::AccountId = account("bidder1", 0, 0);
        let bidder_2: T::AccountId = account("bidder2", 0, 0);
        assert_ok!(Whitelist::<T>::assign_role(
            RawOrigin::Signed(admin.clone()).into(),
            bidder_1.clone(),
            Role::RegionalOperator
        ));
        assert_ok!(Whitelist::<T>::assign_role(
            RawOrigin::Signed(admin).into(),
            bidder_2.clone(),
            Role::RegionalOperator
        ));

        let expiry = frame_system::Pallet::<T>::block_number()
            + T::RegionOwnerChangePeriod::get()
            + 1u32.into();
        frame_system::Pallet::<T>::set_block_number(expiry);

        let base_bid = T::MinimumRegionDeposit::get() * 10u32.into();
        let _ = <T as pallet::Config>::NativeCurrency::mint_into(&bidder_1, base_bid);
        let higher_bid = T::MinimumRegionDeposit::get() * 20u32.into();
        let _ = <T as pallet::Config>::NativeCurrency::mint_into(&bidder_2, higher_bid);

        assert_ok!(Regions::<T>::bid_on_region_replacement(
            RawOrigin::Signed(bidder_1.clone()).into(),
            region_id,
            base_bid / 2u32.into()
        ));

        #[extrinsic_call]
        bid_on_region_replacement(
            RawOrigin::Signed(bidder_2.clone()),
            region_id,
            higher_bid / 2u32.into(),
        );

        let auction = RegionReplacementAuctions::<T>::get(region_id).unwrap();
        assert_eq!(auction.highest_bidder, Some(bidder_2));
        assert_eq!(auction.collateral, higher_bid / 2u32.into());
    }

    #[benchmark]
    fn initiate_region_owner_resignation() {
        let (signer, _): (T::AccountId, T::AccountId) = create_whitelisted_user::<T>();
        let region_id = create_a_new_region::<T>(signer.clone());

        let initial_block = frame_system::Pallet::<T>::block_number();
        let new_block = initial_block + 100u32.into();
        frame_system::Pallet::<T>::set_block_number(new_block);

        #[extrinsic_call]
        initiate_region_owner_resignation(RawOrigin::Signed(signer.clone()), region_id);

        let next_owner_change = new_block + T::RegionOwnerNoticePeriod::get();

        assert_eq!(
            RegionDetails::<T>::get(region_id).unwrap().next_owner_change,
            next_owner_change
        );
    }

    impl_benchmark_test_suite!(Regions, crate::mock::new_test_ext(), crate::mock::Test);
}
