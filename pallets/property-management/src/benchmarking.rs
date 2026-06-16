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

//! Benchmarking setup for pallet-property-management
#![cfg(feature = "runtime-benchmarks")]
use super::*;

#[allow(unused)]
use crate::Pallet as PropertyManagement;
use frame_benchmarking::v2::*;
use frame_support::sp_runtime::{Permill, Saturating};
use frame_support::traits::fungible::{Inspect, Mutate};
use frame_support::traits::fungibles::InspectFreeze;
use frame_support::BoundedVec;
use frame_support::{assert_ok, traits::Get};
use frame_system::RawOrigin;
use pallet_marketplace::types::LegalProperty;
use pallet_marketplace::Pallet as Marketplace;
use pallet_regions::Pallet as Regions;
use pallet_regions::{RegionIdentifier, Vote};
use pallet_xcavate_whitelist::Pallet as Whitelist;
use pallet_xcavate_whitelist::Role;
use scale_info::prelude::vec;

pub trait Config:
    pallet_marketplace::Config
    + pallet_xcavate_whitelist::Config
    + pallet_regions::Config
    + crate::Config
{
}

impl<
        T: crate::Config
            + pallet_marketplace::Config
            + pallet_xcavate_whitelist::Config
            + pallet_regions::Config,
    > Config for T
{
}

fn create_whitelisted_user<T: Config>() -> (T::AccountId, T::AccountId) {
    let admin: T::AccountId = account("admin", 0, 0);
    let signer: T::AccountId = account("signer", 0, 0);
    assert_ok!(Whitelist::<T>::add_admin(RawOrigin::Root.into(), admin.clone()));
    assert_ok!(Whitelist::<T>::assign_role(
        RawOrigin::Signed(admin.clone()).into(),
        signer.clone(),
        Role::RealEstateDeveloper
    ));
    (signer, admin)
}

fn create_a_new_region<T: Config>(
    signer: T::AccountId,
    admin: T::AccountId,
) -> (u16, LocationId<T>) {
    let region = RegionIdentifier::France;
    let region_id = region.clone().into_u16();

    let deposit = T::RegionProposalDeposit::get();
    let auction_amount = T::MinimumRegionDeposit::get();
    let total_funds = deposit
        .saturating_mul(1000u32.into())
        .saturating_add(auction_amount.saturating_mul(100u32.into()));
    assert_ok!(<T as pallet_regions::Config>::NativeCurrency::mint_into(&signer, total_funds));

    assert_ok!(Whitelist::<T>::assign_role(
        RawOrigin::Signed(admin.clone()).into(),
        signer.clone(),
        Role::RegionalOperator
    ));
    assert_ok!(Whitelist::<T>::assign_role(
        RawOrigin::Signed(admin.clone()).into(),
        signer.clone(),
        Role::RealEstateInvestor
    ));
    assert_ok!(Regions::<T>::propose_new_region(
        RawOrigin::Signed(signer.clone()).into(),
        region.clone()
    ));
    assert_ok!(Regions::<T>::vote_on_region_proposal(
        RawOrigin::Signed(signer.clone()).into(),
        region_id,
        Vote::Yes,
        deposit.saturating_mul(800u32.into())
    ));

    let bid_amount = auction_amount.saturating_mul(10u32.into());

    let expiry = frame_system::Pallet::<T>::block_number() + T::RegionVotingTime::get();
    frame_system::Pallet::<T>::set_block_number(expiry);

    assert_ok!(Regions::<T>::bid_on_region(
        RawOrigin::Signed(signer.clone()).into(),
        region_id,
        bid_amount
    ));

    let auction_expiry = frame_system::Pallet::<T>::block_number() + T::RegionAuctionTime::get();
    frame_system::Pallet::<T>::set_block_number(auction_expiry);
    assert_ok!(Regions::<T>::create_new_region(
        RawOrigin::Signed(signer.clone()).into(),
        region_id,
        T::MaxListingDuration::get(),
        Permill::from_percent(5)
    ));

    let location = BoundedVec::try_from("SG23 5TH".as_bytes().to_vec()).unwrap();
    assert_ok!(Regions::<T>::create_new_location(
        RawOrigin::Signed(signer.clone()).into(),
        region_id,
        location.clone()
    ));

    // Verify region and location
    assert!(pallet_regions::RegionDetails::<T>::contains_key(region_id));
    assert!(pallet_regions::LocationRegistration::<T>::contains_key(region_id, &location));

    let location = BoundedVec::try_from("SG23 5TH".as_bytes().to_vec()).unwrap();

    (region_id, location)
}

fn list_and_sell_property<T: Config>(
    seller: T::AccountId,
    region_id: u16,
    admin: T::AccountId,
) -> T::AccountId {
    let pallet_account = Marketplace::<T>::account_id();
    let _ = <T as pallet_marketplace::Config>::NativeCurrency::mint_into(
        &pallet_account,
        1_000_000_000_000_000u128.into(),
    );
    let share_amount: u32 = <T as pallet_marketplace::Config>::MaxPropertyShares::get();
    let share_price: <T as pallet_marketplace::Config>::Balance = 1_000u32.into();
    let property_price = share_price.saturating_mul((share_amount as u128).into());
    let deposit_amount = property_price.saturating_mul(T::ListingDeposit::get()) / 100u128.into();
    assert_ok!(<T as pallet_marketplace::Config>::NativeCurrency::mint_into(
        &seller,
        deposit_amount.saturating_mul(20u32.into())
    ));

    let metadata: BoundedVec<u8, <T as pallet_marketplace::Config>::StringLimit> =
        BoundedVec::truncate_from(vec![
            42u8;
            <T as pallet_marketplace::Config>::StringLimit::get()
                as usize
        ]);

    let tax_paid_by_developer = true;
    let location = BoundedVec::try_from("SG23 5TH".as_bytes().to_vec()).unwrap();
    assert_ok!(Marketplace::<T>::list_property(
        RawOrigin::Signed(seller).into(),
        region_id,
        location,
        share_price,
        share_amount,
        metadata,
        tax_paid_by_developer,
    ));
    let listing_id = 0;
    let payment_asset = <T as pallet::Config>::AcceptedAssets::get()[0];
    let buyer: T::AccountId = account("buyer", 0, 0);
    assert_ok!(<T as pallet_marketplace::Config>::NativeCurrency::mint_into(
        &buyer,
        deposit_amount.saturating_mul(20u32.into())
    ));
    assert_ok!(<T as pallet_marketplace::Config>::ForeignCurrency::mint_into(
        payment_asset,
        &buyer,
        property_price.saturating_mul(100u32.into())
    ));
    assert_ok!(Whitelist::<T>::assign_role(
        RawOrigin::Signed(admin.clone()).into(),
        buyer.clone(),
        Role::RealEstateInvestor
    ));
    add_buyers_to_listing::<T>(share_amount - 1, payment_asset, property_price, admin.clone());

    assert_ok!(Marketplace::<T>::buy_property_shares(
        RawOrigin::Signed(buyer.clone()).into(),
        listing_id,
        1,
        payment_asset,
    ));
    let spv_admin: T::AccountId = account("spv_admin", 0, 0);
    assert_ok!(Whitelist::<T>::assign_role(
        RawOrigin::Signed(admin).into(),
        spv_admin.clone(),
        Role::SpvConfirmation
    ));
    assert_ok!(Marketplace::<T>::create_spv(RawOrigin::Signed(spv_admin).into(), listing_id,));
    claim_buyers_property_shares::<T>(share_amount - 1, listing_id);
    assert_ok!(Marketplace::<T>::claim_property_shares(
        RawOrigin::Signed(buyer.clone()).into(),
        listing_id,
    ));
    buyer
}

fn create_registered_property<T: Config>(
    seller: T::AccountId,
    region_id: u16,
    admin: T::AccountId,
) -> (T::AccountId, u32) {
    let share_owner = list_and_sell_property::<T>(seller.clone(), region_id, admin.clone());
    let lawyer_1: T::AccountId = account("lawyer1", 0, 0);
    let lawyer_2: T::AccountId = account("lawyer2", 0, 0);
    assert_ok!(Whitelist::<T>::assign_role(
        RawOrigin::Signed(admin.clone()).into(),
        lawyer_1.clone(),
        Role::Lawyer
    ));
    let lawyer_deposit = <T as pallet_regions::Config>::LawyerDeposit::get();
    let _ = <T as pallet_regions::Config>::NativeCurrency::mint_into(
        &lawyer_1,
        lawyer_deposit * 10u32.into(),
    );
    assert_ok!(Whitelist::<T>::assign_role(
        RawOrigin::Signed(admin).into(),
        lawyer_2.clone(),
        Role::Lawyer
    ));
    assert_ok!(Regions::<T>::register_lawyer(
        RawOrigin::Signed(lawyer_1.clone()).into(),
        region_id,
    ));
    let lawyer_deposit = <T as pallet_regions::Config>::LawyerDeposit::get();
    let _ = <T as pallet_regions::Config>::NativeCurrency::mint_into(
        &lawyer_2,
        lawyer_deposit * 10u32.into(),
    );
    assert_ok!(Regions::<T>::register_lawyer(
        RawOrigin::Signed(lawyer_2.clone()).into(),
        region_id,
    ));

    assert_ok!(Marketplace::<T>::lawyer_claim_property(
        RawOrigin::Signed(lawyer_1.clone()).into(),
        0,
        LegalProperty::RealEstateDeveloperSide,
        400_u32.into()
    ));
    assert_ok!(Marketplace::<T>::approve_developer_lawyer(
        RawOrigin::Signed(seller.clone()).into(),
        0,
        true
    ));
    assert_ok!(Marketplace::<T>::lawyer_claim_property(
        RawOrigin::Signed(lawyer_2.clone()).into(),
        0,
        LegalProperty::SpvSide,
        400_u32.into()
    ));
    let share_amount = <T as pallet::Config>::PropertyShares::get_share_balance(0, &share_owner);
    assert_ok!(Marketplace::<T>::vote_on_spv_lawyer(
        RawOrigin::Signed(share_owner.clone()).into(),
        0,
        pallet_marketplace::types::Vote::Yes,
        share_amount
    ));
    for i in 1..=<T as pallet_marketplace::Config>::MaxPropertyShares::get() - 1 {
        let buyer: T::AccountId = account("buyer", i, i);
        assert_ok!(Marketplace::<T>::vote_on_spv_lawyer(
            RawOrigin::Signed(buyer).into(),
            0,
            pallet_marketplace::types::Vote::Yes,
            1
        ));
    }
    let expiry = frame_system::Pallet::<T>::block_number() + T::LawyerVotingTime::get();
    frame_system::Pallet::<T>::set_block_number(expiry);
    assert_ok!(Marketplace::<T>::finalize_spv_lawyer(
        RawOrigin::Signed(share_owner.clone()).into(),
        0,
    ));

    assert_ok!(Marketplace::<T>::lawyer_confirm_documents(
        RawOrigin::Signed(lawyer_1).into(),
        0,
        true
    ));
    assert_ok!(Marketplace::<T>::lawyer_confirm_documents(
        RawOrigin::Signed(lawyer_2).into(),
        0,
        true
    ));
    let asset_id = 0u32;
    (share_owner, asset_id)
}

fn add_buyers_to_listing<T: Config + pallet_marketplace::Config>(
    buyers: u32,
    payment_asset: u32,
    property_price: <T as pallet_marketplace::Config>::Balance,
    admin: T::AccountId,
) {
    let deposit_amount = property_price
        .saturating_mul(<T as pallet_marketplace::Config>::ListingDeposit::get())
        / 100u128.into();

    for i in 1..=buyers {
        let buyer: T::AccountId = account("buyer", i, i);
        let payment_asset_buyers = <T as pallet::Config>::AcceptedAssets::get()[0];
        assert_ok!(<T as pallet_marketplace::Config>::NativeCurrency::mint_into(
            &buyer,
            deposit_amount.saturating_mul(20u32.into())
        ));
        assert_ok!(<T as pallet_marketplace::Config>::ForeignCurrency::mint_into(
            payment_asset,
            &buyer,
            property_price
        ));
        assert_ok!(Whitelist::<T>::assign_role(
            RawOrigin::Signed(admin.clone()).into(),
            buyer.clone(),
            Role::RealEstateInvestor
        ));
        assert_ok!(Marketplace::<T>::buy_property_shares(
            RawOrigin::Signed(buyer).into(),
            0,
            1,
            payment_asset_buyers
        ));
    }
}

fn claim_buyers_property_shares<T: Config>(buyers: u32, listing_id: pallet_marketplace::ListingId) {
    for i in 1..=buyers {
        let buyer: T::AccountId = account("buyer", i, i);
        assert_ok!(Marketplace::<T>::claim_property_shares(
            RawOrigin::Signed(buyer).into(),
            listing_id
        ));
    }
}

#[benchmarks]
mod benchmarks {
    use super::*;

    #[benchmark]
    fn add_letting_agent() {
        let (region_owner, admin): (T::AccountId, T::AccountId) = create_whitelisted_user::<T>();
        let (region_id, location) = create_a_new_region::<T>(region_owner.clone(), admin.clone());

        let letting_agent: T::AccountId = account("letting_agent", 0, 0);
        assert_ok!(Whitelist::<T>::assign_role(
            RawOrigin::Signed(admin.clone()).into(),
            letting_agent.clone(),
            Role::LettingAgent
        ));

        let deposit = T::LettingAgentDeposit::get().saturating_mul(20u32.into());
        assert_ok!(<T as pallet::Config>::NativeCurrency::mint_into(&letting_agent, deposit));
        let balance_before = <T as pallet::Config>::NativeCurrency::balance(&letting_agent);

        #[extrinsic_call]
        add_letting_agent(RawOrigin::Signed(letting_agent.clone()), region_id, location.clone());

        let letting_info = LettingInfo::<T>::get(&letting_agent).unwrap();
        assert_eq!(letting_info.region, region_id);
        assert!(letting_info.locations.contains_key(&location));
        assert_eq!(
            letting_info.locations.get(&location).unwrap().deposit,
            T::LettingAgentDeposit::get()
        );
        assert_eq!(letting_info.locations.get(&location).unwrap().assigned_properties, 0);
        assert_eq!(
            <T as pallet::Config>::NativeCurrency::balance(&letting_agent),
            balance_before - T::LettingAgentDeposit::get()
        );
    }

    #[benchmark]
    fn remove_letting_agent() {
        let (region_owner, admin): (T::AccountId, T::AccountId) = create_whitelisted_user::<T>();
        let (region_id, location) = create_a_new_region::<T>(region_owner.clone(), admin.clone());

        let letting_agent: T::AccountId = account("letting_agent", 0, 0);
        assert_ok!(Whitelist::<T>::assign_role(
            RawOrigin::Signed(admin.clone()).into(),
            letting_agent.clone(),
            Role::LettingAgent
        ));

        let deposit = T::LettingAgentDeposit::get().saturating_mul(20u32.into());
        assert_ok!(<T as pallet::Config>::NativeCurrency::mint_into(&letting_agent, deposit));
        let balance_before = <T as pallet::Config>::NativeCurrency::balance(&letting_agent);

        assert_ok!(PropertyManagement::<T>::add_letting_agent(
            RawOrigin::Signed(letting_agent.clone()).into(),
            region_id,
            location.clone(),
        ));
        assert_eq!(
            <T as pallet::Config>::NativeCurrency::balance(&letting_agent),
            balance_before - T::LettingAgentDeposit::get()
        );

        #[extrinsic_call]
        remove_letting_agent(RawOrigin::Signed(letting_agent.clone()), location.clone());

        assert!(LettingInfo::<T>::get(&letting_agent).is_none());
        assert_eq!(<T as pallet::Config>::NativeCurrency::balance(&letting_agent), balance_before);
    }

    #[benchmark]
    fn letting_agent_claim_property() {
        let (region_owner, admin): (T::AccountId, T::AccountId) = create_whitelisted_user::<T>();
        let (region_id, location) = create_a_new_region::<T>(region_owner.clone(), admin.clone());
        let (_, asset_id) =
            create_registered_property::<T>(region_owner.clone(), region_id, admin.clone());

        let letting_agent: T::AccountId = account("letting_agent", 0, 0);
        assert_ok!(Whitelist::<T>::assign_role(
            RawOrigin::Signed(admin).into(),
            letting_agent.clone(),
            Role::LettingAgent
        ));
        let deposit = T::LettingAgentDeposit::get().saturating_mul(20u32.into());
        assert_ok!(<T as pallet::Config>::NativeCurrency::mint_into(&letting_agent, deposit));

        assert_ok!(PropertyManagement::<T>::add_letting_agent(
            RawOrigin::Signed(letting_agent.clone()).into(),
            region_id,
            location,
        ));

        #[extrinsic_call]
        letting_agent_claim_property(RawOrigin::Signed(letting_agent.clone()), asset_id);

        let proposal_id = 0;
        assert_eq!(AssetLettingProposal::<T>::get(asset_id), Some(proposal_id));
        assert_eq!(
            LettingAgentProposal::<T>::get(proposal_id).unwrap().letting_agent,
            letting_agent
        );
        assert_eq!(ProposalCounter::<T>::get(), 1);
        assert_eq!(
            OngoingLettingAgentVoting::<T>::get(proposal_id).unwrap(),
            crate::VoteStats { yes_voting_power: 0, no_voting_power: 0, abstain_voting_power: 0 },
        );
    }

    #[benchmark]
    fn vote_on_letting_agent() {
        let (region_owner, admin): (T::AccountId, T::AccountId) = create_whitelisted_user::<T>();
        let (region_id, location) = create_a_new_region::<T>(region_owner.clone(), admin.clone());
        let (share_owner, asset_id) =
            create_registered_property::<T>(region_owner.clone(), region_id, admin.clone());

        let letting_agent: T::AccountId = account("letting_agent", 0, 0);
        assert_ok!(Whitelist::<T>::assign_role(
            RawOrigin::Signed(admin).into(),
            letting_agent.clone(),
            Role::LettingAgent
        ));
        let deposit = T::LettingAgentDeposit::get().saturating_mul(20u32.into());
        assert_ok!(<T as pallet::Config>::NativeCurrency::mint_into(&letting_agent, deposit));

        assert_ok!(PropertyManagement::<T>::add_letting_agent(
            RawOrigin::Signed(letting_agent.clone()).into(),
            region_id,
            location,
        ));

        assert_ok!(PropertyManagement::<T>::letting_agent_claim_property(
            RawOrigin::Signed(letting_agent.clone()).into(),
            asset_id
        ));

        let share_amount = <T as pallet::Config>::PropertyShares::get_share_balance(0, &share_owner);
        assert_eq!(
            <T as pallet::Config>::AssetsFreezer::balance_frozen(
                asset_id,
                &MarketplaceFreezeReason::LettingAgentVoting,
                &share_owner
            ),
            0u32.into()
        );
        assert_ok!(PropertyManagement::<T>::vote_on_letting_agent(
            RawOrigin::Signed(share_owner.clone()).into(),
            asset_id,
            crate::Vote::No,
            share_amount
        ));

        for i in 1..<T as pallet_marketplace::Config>::MaxPropertyShares::get() {
            let buyer: T::AccountId = account("buyer", i, i);
            let share_amount = <T as pallet::Config>::PropertyShares::get_share_balance(0, &buyer);
            assert_ok!(PropertyManagement::<T>::vote_on_letting_agent(
                RawOrigin::Signed(buyer).into(),
                asset_id,
                crate::Vote::Yes,
                share_amount
            ));
        }

        let share_amount = <T as pallet::Config>::PropertyShares::get_share_balance(0, &share_owner);

        #[extrinsic_call]
        vote_on_letting_agent(
            RawOrigin::Signed(share_owner.clone()),
            asset_id,
            crate::Vote::Yes,
            share_amount,
        );

        assert_eq!(UserLettingAgentVote::<T>::get(0, &share_owner).unwrap().vote, crate::Vote::Yes);
        assert_eq!(
            OngoingLettingAgentVoting::<T>::get(0).unwrap(),
            crate::VoteStats {
                yes_voting_power: <T as pallet_marketplace::Config>::MaxPropertyShares::get(),
                no_voting_power: 0,
                abstain_voting_power: 0,
            },
        );
        assert_eq!(
            <T as pallet::Config>::AssetsFreezer::balance_frozen(
                asset_id,
                &MarketplaceFreezeReason::LettingAgentVoting,
                &share_owner
            ),
            share_amount.into()
        );
    }

    #[benchmark]
    fn finalize_letting_agent() {
        let (region_owner, admin): (T::AccountId, T::AccountId) = create_whitelisted_user::<T>();
        let (region_id, location) = create_a_new_region::<T>(region_owner.clone(), admin.clone());
        let (share_owner, asset_id) =
            create_registered_property::<T>(region_owner.clone(), region_id, admin.clone());

        let letting_agent: T::AccountId = account("letting_agent", 0, 0);
        assert_ok!(Whitelist::<T>::assign_role(
            RawOrigin::Signed(admin).into(),
            letting_agent.clone(),
            Role::LettingAgent
        ));
        let deposit = T::LettingAgentDeposit::get().saturating_mul(20u32.into());
        assert_ok!(<T as pallet::Config>::NativeCurrency::mint_into(&letting_agent, deposit));

        assert_ok!(PropertyManagement::<T>::add_letting_agent(
            RawOrigin::Signed(letting_agent.clone()).into(),
            region_id,
            location,
        ));

        assert_ok!(PropertyManagement::<T>::letting_agent_claim_property(
            RawOrigin::Signed(letting_agent.clone()).into(),
            asset_id
        ));

        let share_amount = <T as pallet::Config>::PropertyShares::get_share_balance(0, &share_owner);
        assert_ok!(PropertyManagement::<T>::vote_on_letting_agent(
            RawOrigin::Signed(share_owner.clone()).into(),
            asset_id,
            crate::Vote::Yes,
            share_amount
        ));

        for i in 1..<T as pallet_marketplace::Config>::MaxPropertyShares::get() {
            let buyer: T::AccountId = account("buyer", i, i);
            let share_amount = <T as pallet::Config>::PropertyShares::get_share_balance(0, &buyer);
            assert_ok!(PropertyManagement::<T>::vote_on_letting_agent(
                RawOrigin::Signed(buyer).into(),
                asset_id,
                crate::Vote::Yes,
                share_amount
            ));
        }

        let expiry = frame_system::Pallet::<T>::block_number() + T::LettingAgentVotingTime::get();
        frame_system::Pallet::<T>::set_block_number(expiry);

        #[extrinsic_call]
        finalize_letting_agent(RawOrigin::Signed(share_owner.clone()), asset_id);

        assert!(LettingAgentProposal::<T>::get(0).is_none());
        assert!(OngoingLettingAgentVoting::<T>::get(0).is_none());
        assert!(AssetLettingProposal::<T>::get(asset_id).is_none());
        assert_eq!(LettingStorage::<T>::get(asset_id), Some(letting_agent.clone()));
    }

    #[benchmark]
    fn unfreeze_letting_voting_shares() {
        let (region_owner, admin): (T::AccountId, T::AccountId) = create_whitelisted_user::<T>();
        let (region_id, location) = create_a_new_region::<T>(region_owner.clone(), admin.clone());
        let (share_owner, asset_id) =
            create_registered_property::<T>(region_owner.clone(), region_id, admin.clone());

        let letting_agent: T::AccountId = account("letting_agent", 0, 0);
        assert_ok!(Whitelist::<T>::assign_role(
            RawOrigin::Signed(admin).into(),
            letting_agent.clone(),
            Role::LettingAgent
        ));
        let deposit = T::LettingAgentDeposit::get().saturating_mul(20u32.into());
        assert_ok!(<T as pallet::Config>::NativeCurrency::mint_into(&letting_agent, deposit));

        assert_ok!(PropertyManagement::<T>::add_letting_agent(
            RawOrigin::Signed(letting_agent.clone()).into(),
            region_id,
            location,
        ));

        assert_ok!(PropertyManagement::<T>::letting_agent_claim_property(
            RawOrigin::Signed(letting_agent.clone()).into(),
            asset_id
        ));

        let share_amount = <T as pallet::Config>::PropertyShares::get_share_balance(0, &share_owner);
        assert_ok!(PropertyManagement::<T>::vote_on_letting_agent(
            RawOrigin::Signed(share_owner.clone()).into(),
            asset_id,
            crate::Vote::Yes,
            share_amount
        ));

        for i in 1..<T as pallet_marketplace::Config>::MaxPropertyShares::get() {
            let buyer: T::AccountId = account("buyer", i, i);
            let share_amount = <T as pallet::Config>::PropertyShares::get_share_balance(0, &buyer);
            assert_ok!(PropertyManagement::<T>::vote_on_letting_agent(
                RawOrigin::Signed(buyer).into(),
                asset_id,
                crate::Vote::Yes,
                share_amount
            ));
        }

        let expiry = frame_system::Pallet::<T>::block_number() + T::LettingAgentVotingTime::get();
        frame_system::Pallet::<T>::set_block_number(expiry);

        assert_ok!(PropertyManagement::<T>::finalize_letting_agent(
            RawOrigin::Signed(share_owner.clone()).into(),
            asset_id,
        ));
        assert!(UserLettingAgentVote::<T>::get(0, &share_owner).is_some());
        assert_eq!(
            <T as pallet::Config>::AssetsFreezer::balance_frozen(
                asset_id,
                &MarketplaceFreezeReason::LettingAgentVoting,
                &share_owner
            ),
            share_amount.into()
        );

        #[extrinsic_call]
        unfreeze_letting_voting_shares(RawOrigin::Signed(share_owner.clone()), 0);

        assert!(UserLettingAgentVote::<T>::get(0, &share_owner).is_none());
        assert_eq!(
            <T as pallet::Config>::AssetsFreezer::balance_frozen(
                asset_id,
                &MarketplaceFreezeReason::LettingAgentVoting,
                &share_owner
            ),
            0u32.into()
        );
    }

    #[benchmark]
    fn distribute_income() {
        let (region_owner, admin): (T::AccountId, T::AccountId) = create_whitelisted_user::<T>();
        let (region_id, location) = create_a_new_region::<T>(region_owner.clone(), admin.clone());
        let (share_owner, asset_id) =
            create_registered_property::<T>(region_owner.clone(), region_id, admin.clone());

        let letting_agent: T::AccountId = account("letting_agent", 0, 0);
        assert_ok!(Whitelist::<T>::assign_role(
            RawOrigin::Signed(admin).into(),
            letting_agent.clone(),
            Role::LettingAgent
        ));
        let deposit = T::LettingAgentDeposit::get().saturating_mul(20u32.into());
        assert_ok!(<T as pallet::Config>::NativeCurrency::mint_into(&letting_agent, deposit));

        assert_ok!(PropertyManagement::<T>::add_letting_agent(
            RawOrigin::Signed(letting_agent.clone()).into(),
            region_id,
            location,
        ));
        assert_ok!(PropertyManagement::<T>::letting_agent_claim_property(
            RawOrigin::Signed(letting_agent.clone()).into(),
            0
        ));
        let share_amount = <T as pallet::Config>::PropertyShares::get_share_balance(0, &share_owner);
        assert_ok!(PropertyManagement::<T>::vote_on_letting_agent(
            RawOrigin::Signed(share_owner.clone()).into(),
            0,
            crate::Vote::Yes,
            share_amount
        ));
        for i in 1..=<T as pallet_marketplace::Config>::MaxPropertyShares::get() - 1 {
            let buyer: T::AccountId = account("buyer", i, i);
            let share_amount = <T as pallet::Config>::PropertyShares::get_share_balance(0, &buyer);
            assert_ok!(PropertyManagement::<T>::vote_on_letting_agent(
                RawOrigin::Signed(buyer).into(),
                0,
                crate::Vote::Yes,
                share_amount
            ));
        }
        let expiry = frame_system::Pallet::<T>::block_number() + T::LettingAgentVotingTime::get();
        frame_system::Pallet::<T>::set_block_number(expiry);
        assert_ok!(PropertyManagement::<T>::finalize_letting_agent(
            RawOrigin::Signed(share_owner.clone()).into(),
            0
        ));

        let payment_asset = <T as pallet::Config>::AcceptedAssets::get()[0];
        assert_ok!(<T as pallet::Config>::ForeignCurrency::mint_into(
            payment_asset,
            &letting_agent,
            200_000_000_000u128.into()
        ));

        let distribution_amount = 100_000u128.into();

        #[extrinsic_call]
        distribute_income(
            RawOrigin::Signed(letting_agent.clone()),
            asset_id,
            distribution_amount,
            payment_asset,
        );

        assert_eq!(
            PropertyIncome::<T>::get(asset_id),
            distribution_amount / <T as pallet_marketplace::Config>::MaxPropertyShares::get().into()
        );
    }

    #[benchmark]
    fn claim_income() {
        let (region_owner, admin): (T::AccountId, T::AccountId) = create_whitelisted_user::<T>();
        let (region_id, location) = create_a_new_region::<T>(region_owner.clone(), admin.clone());
        let (share_owner, asset_id) =
            create_registered_property::<T>(region_owner.clone(), region_id, admin.clone());

        let letting_agent: T::AccountId = account("letting_agent", 0, 0);
        assert_ok!(Whitelist::<T>::assign_role(
            RawOrigin::Signed(admin).into(),
            letting_agent.clone(),
            Role::LettingAgent
        ));
        let deposit = T::LettingAgentDeposit::get().saturating_mul(20u32.into());
        assert_ok!(<T as pallet::Config>::NativeCurrency::mint_into(&letting_agent, deposit));

        assert_ok!(PropertyManagement::<T>::add_letting_agent(
            RawOrigin::Signed(letting_agent.clone()).into(),
            region_id,
            location,
        ));
        assert_ok!(PropertyManagement::<T>::letting_agent_claim_property(
            RawOrigin::Signed(letting_agent.clone()).into(),
            0
        ));
        let share_amount = <T as pallet::Config>::PropertyShares::get_share_balance(0, &share_owner);
        assert_ok!(PropertyManagement::<T>::vote_on_letting_agent(
            RawOrigin::Signed(share_owner.clone()).into(),
            0,
            crate::Vote::Yes,
            share_amount
        ));
        for i in 1..=<T as pallet_marketplace::Config>::MaxPropertyShares::get() - 1 {
            let buyer: T::AccountId = account("buyer", i, i);
            let share_amount = <T as pallet::Config>::PropertyShares::get_share_balance(0, &buyer);
            assert_ok!(PropertyManagement::<T>::vote_on_letting_agent(
                RawOrigin::Signed(buyer).into(),
                0,
                crate::Vote::Yes,
                share_amount
            ));
        }
        let expiry = frame_system::Pallet::<T>::block_number() + T::LettingAgentVotingTime::get();
        frame_system::Pallet::<T>::set_block_number(expiry);
        assert_ok!(PropertyManagement::<T>::finalize_letting_agent(
            RawOrigin::Signed(share_owner.clone()).into(),
            0
        ));

        let payment_asset = <T as pallet::Config>::AcceptedAssets::get()[0];
        assert_ok!(<T as pallet::Config>::ForeignCurrency::mint_into(
            payment_asset,
            &letting_agent,
            200_000_000_000u128.into()
        ));

        let distribution_amount = 100_000u128.into();

        assert_ok!(PropertyManagement::<T>::distribute_income(
            RawOrigin::Signed(letting_agent.clone()).into(),
            asset_id,
            distribution_amount,
            payment_asset
        ));
        assert_eq!(
            PropertyIncome::<T>::get(asset_id),
            distribution_amount / <T as pallet_marketplace::Config>::MaxPropertyShares::get().into()
        );

        #[extrinsic_call]
        claim_income(RawOrigin::Signed(share_owner.clone()), asset_id);

        assert_eq!(
            OwnerCheckpoints::<T>::get(share_owner, asset_id),
            distribution_amount / <T as pallet_marketplace::Config>::MaxPropertyShares::get().into()
        );
    }

    #[benchmark]
    fn resign_from_property() {
        let (region_owner, admin): (T::AccountId, T::AccountId) = create_whitelisted_user::<T>();
        let (region_id, location) = create_a_new_region::<T>(region_owner.clone(), admin.clone());
        let (share_owner, asset_id) =
            create_registered_property::<T>(region_owner.clone(), region_id, admin.clone());

        let letting_agent: T::AccountId = account("letting_agent", 0, 0);
        assert_ok!(Whitelist::<T>::assign_role(
            RawOrigin::Signed(admin).into(),
            letting_agent.clone(),
            Role::LettingAgent
        ));
        let deposit = T::LettingAgentDeposit::get().saturating_mul(20u32.into());
        assert_ok!(<T as pallet::Config>::NativeCurrency::mint_into(&letting_agent, deposit));

        assert_ok!(PropertyManagement::<T>::add_letting_agent(
            RawOrigin::Signed(letting_agent.clone()).into(),
            region_id,
            location,
        ));
        assert_ok!(PropertyManagement::<T>::letting_agent_claim_property(
            RawOrigin::Signed(letting_agent.clone()).into(),
            0
        ));
        let share_amount = <T as pallet::Config>::PropertyShares::get_share_balance(0, &share_owner);
        assert_ok!(PropertyManagement::<T>::vote_on_letting_agent(
            RawOrigin::Signed(share_owner.clone()).into(),
            0,
            crate::Vote::Yes,
            share_amount
        ));
        for i in 1..=<T as pallet_marketplace::Config>::MaxPropertyShares::get() - 1 {
            let buyer: T::AccountId = account("buyer", i, i);
            let share_amount = <T as pallet::Config>::PropertyShares::get_share_balance(0, &buyer);
            assert_ok!(PropertyManagement::<T>::vote_on_letting_agent(
                RawOrigin::Signed(buyer).into(),
                0,
                crate::Vote::Yes,
                share_amount
            ));
        }
        let expiry = frame_system::Pallet::<T>::block_number() + T::LettingAgentVotingTime::get();
        frame_system::Pallet::<T>::set_block_number(expiry);
        assert_ok!(PropertyManagement::<T>::finalize_letting_agent(
            RawOrigin::Signed(share_owner.clone()).into(),
            0
        ));

        #[extrinsic_call]
        resign_from_property(RawOrigin::Signed(letting_agent.clone()), asset_id);
        let resignation_block =
            frame_system::Pallet::<T>::block_number() + T::LettingAgentNoticePeriod::get();
        let notice = ResignationNotices::<T>::get(asset_id).unwrap();
        assert_eq!(notice.letting_agent, letting_agent);
        assert_eq!(notice.resignation_block, resignation_block);
        assert_eq!(ResignationQueue::<T>::get(resignation_block)[0], asset_id);
    }

    impl_benchmark_test_suite!(PropertyManagement, crate::mock::new_test_ext(), crate::mock::Test);
}
